use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::path::{Path, PathBuf};

use crate::{db, SolGateState};
use db::{queries::*, Archive, SourceFormat};
use db::{Mod, SHA256Checksum, Source};
use itertools::Itertools;
use reqwest::Client;
use tokio::task::JoinError;

use futures::stream::{self, FuturesUnordered, Stream, StreamExt};
use tokio::fs::{DirBuilder, File};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncSeek, AsyncSeekExt, AsyncWrite, AsyncWriteExt};
use tokio::sync::{mpsc, oneshot};

type PathOneshot = (String, oneshot::Sender<Vec<u8>>);

type VecPlusOneshot = (Vec<u8>, oneshot::Sender<Vec<u8>>);

pub mod api;
pub mod compression;
mod dag;
mod hash;
mod indexer;
pub mod readers;
mod sevenz;
mod solver;
mod util;

use self::indexer::{index_dir, index_file, IndexError};
use self::sevenz::sevenz_extract;
use self::util::UrlError;

type Manifest = Vec<ManifEntry>;
#[derive(Clone)]
pub struct ManifEntry {
    pub path: PathBuf,
    pub ident: ManifIdent,
}

// It's either a raw file, or a VP.
#[derive(Clone)]
pub enum ManifIdent {
    Raw(SHA256Checksum),
    VP(VPContents),
}

// If it's a VP, we should know what its hash is,
// or what its contents are we can build it locally.
#[derive(Clone)]
pub enum VPContents {
    Hash(SHA256Checksum),
    Contents(Vec<VPEntry>),
}

#[derive(Clone)]
pub struct VPEntry {
    pub path: PathBuf,
    pub hash: SHA256Checksum,
}

#[derive(Clone)]
pub enum DataPath {
    Raw(PathBuf),
    VPEntry(PathBuf, String),
    SZEntry(PathBuf, String), // Not currently supported.
}

#[derive(Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum DagEdge {
    VP(String),
    SZ(String),
}

pub async fn install_files(
    manifest: Manifest,
    mod_info: Mod,
    state: &SolGateState,
) -> Result<(), Box<dyn Error>> {
    // first we need to make sure we actually have a local copy of the files we need.
    acquire_files(state.clone(), &manifest).await?;
    // Construct full installation path.
    let mut install_path =
        PathBuf::from(state.config.read().await.local_settings.install_dir.clone());
    // optional mod_parent member, populated by mods but not TCs.
    if let Some(mod_parent) = &mod_info.parent {
        install_path.push(mod_parent);
    }
    install_path.push(&mod_info.name);
    let modver = format!("{}-{}", &mod_info.name, &mod_info.version);
    install_path.push(modver);

    Ok(())
}

pub async fn acquire_files(
    state: SolGateState,
    manifest: &Manifest,
) -> Result<(), FileAcquisitionError> {
    // We need to first make sure we know exactly what files we want to fetch.
    // This involves figuring out what we already have so that we minimise the amount we download.

    let hashes = manifest
        .iter()
        .flat_map(|f| match &f.ident {
            ManifIdent::Raw(hash) => vec![hash.clone()],
            ManifIdent::VP(vp) => match &vp {
                VPContents::Hash(hash) => vec![hash.clone()],
                VPContents::Contents(entries) => entries
                    .iter()
                    .map(|ve| ve.hash.clone())
                    .collect::<Vec<SHA256Checksum>>(),
            },
        })
        .collect::<Vec<SHA256Checksum>>();

    let mut tx = state.sql_pool.begin().await?;
    let hids = get_hash_ids(&hashes, &mut tx)
        .await?
        .into_iter()
        .map(|(_, h_id)| h_id)
        .collect::<Vec<_>>();
    tx.commit().await?;

    // Files can be inside other ones, i.e. inside VPs and inside 7z archives.
    // We construct a DAG of what archives contain what files.
    // This can be multi layered, as VPs are contained within 7z archives on FSNebula.
    // The DAG's nodes are hash IDs.
    // DAG edges describe what sort of archive and what the path is inside.
    let hid_hierarchy = generate_dag(&state, &hids).await?;
    // Once we have a full DAG, we calculate which items in the manifest are missing a local source
    let missing = calculate_missing(&state, &hids, &hid_hierarchy).await?;

    // We find a set of sources that minimize the total required download.
    let sources = calculate_fetches(&state, &missing, &hid_hierarchy).await?;

    // We'll set up a stream of http fetch tasks
    let mut tasks = stream::iter(sources.into_iter().map(|source| {
        let s = state.clone();
        async move { fetch_files(&source, &s).await.unwrap() }
    }))
    .buffer_unordered(4) // TODO Replace 4 with parallel download count.
    .then(|loc| async {
        match loc {
            FetchResult::Directory(dir) => {
                index_dir(&dir, state.clone(), db::SourceLocation::Temp).await
            }
            FetchResult::File(file) => {
                index_file(&file, state.clone(), db::SourceLocation::Temp).await
            }
        }
    })
    .boxed();
    // need to handle these tasks.
    while let Some(result) = tasks.next().await {
        result?
    }
    
    todo!()
}

pub async fn generate_dag(
    state: &SolGateState,
    ids: &Vec<i64>,
) -> Result<dag::HashDAG<i64, DagEdge>, FileAcquisitionError> {
    let mut hid_hierarchy = dag::HashDAG::<i64, DagEdge>::new();
    let mut sql_tx = state.sql_pool.begin().await?;
    // We're going to loop over the parents, parent's parents etc. until there's none left.
    // This will fill the DAG with every possible container of our needed files.
    let mut parents = get_parents_from_ids(&ids, &mut sql_tx).await?;
    while !parents.is_empty() {
        // Loop terminates once there's no parents.
        for archive_entry in parents.iter() {
            // Make sure we add the file and archive it's in
            hid_hierarchy.add(&archive_entry.file_id);
            hid_hierarchy.add(&archive_entry.archive_id);
            // Need to create a special struct for holding this edge info.
            let edge = match archive_entry.archive_type {
                Archive::SevenZip => DagEdge::SZ(archive_entry.file_path.clone()),
                Archive::VP => DagEdge::VP(archive_entry.file_path.clone()),
            };
            hid_hierarchy
                .add_relationship(&archive_entry.file_id, &archive_entry.archive_id, edge)
                .unwrap();
        }
        let parent_ids = parents
            .into_iter()
            .map(|ae| ae.archive_id)
            .unique()
            .collect();
        parents = get_parents_from_ids(&parent_ids, &mut sql_tx).await?;
    }
    sql_tx.commit().await?;
    Ok(hid_hierarchy)
}

pub async fn calculate_missing(
    state: &SolGateState,
    hids: &Vec<i64>,
    hid_hierarchy: &dag::HashDAG<i64, DagEdge>,
) -> Result<Vec<i64>, FileAcquisitionError> {
    let mut missing_hids = HashSet::<i64>::from_iter(hids.iter().cloned());

    // At this point, we generate a list of all sources by querying for all h_ids.
    let all_hids = hid_hierarchy
        .get_nodes()
        .iter()
        .map(|n| n.data())
        .cloned()
        .collect_vec();
    let mut sql_tx = state.sql_pool.begin().await?;
    let sources = get_sources_from_ids(&all_hids, &mut sql_tx).await?;
    sql_tx.commit().await?;
    // We then find sources we have local copies of,
    // and remove all the hash ids that particular source contains.
    for source in sources {
        if source.location.is_local() {
            let source_hid = source.h_id;
            let descendants = hid_hierarchy.descendants(&source_hid).unwrap();
            for descendant in descendants {
                missing_hids.remove(&descendant);
            }
        }
    }
    // This gives us a set of the h_ids we do not have any local of.
    Ok(missing_hids.into_iter().collect())
}

pub async fn calculate_fetches(
    state: &SolGateState,
    missing: &Vec<i64>,
    hid_hierarchy: &dag::HashDAG<i64, DagEdge>,
) -> Result<Vec<Source>, FileAcquisitionError> {
    // We now need to solve our fetch problem.
    // How do we get the missing files using the smallest amount of downloading?
    // We construct a map of Source to Vec<hash_id>,
    // so we know which of our missing files each source contains.
    let missing_source_hids: Vec<i64> = missing
        .iter()
        .flat_map(|m| hid_hierarchy.ancestors(m).unwrap())
        .unique()
        .collect();
    let mut tx = state.sql_pool.begin().await?;
    let missing_sources = get_sources_from_ids(&missing_source_hids, &mut tx).await?;
    let remote_sources = missing_sources
        .into_iter()
        .filter(|s| !s.location.is_local())
        .collect::<Vec<Source>>();

    let sourcemap = HashMap::<Source, Vec<i64>>::from_iter(remote_sources.into_iter().map(|s| {
        (
            s.clone(),
            hid_hierarchy
                .descendants(&(s.h_id))
                .unwrap()
                .into_iter()
                .filter(|d| missing.contains(&d))
                .collect(),
        )
    }));

    // At this point we offload calculation of this weighted set coverage problem to the fetch solver.
    // This is pretty CPU intensive and might block for a while, so run seperately.
    let minimized_sources =
        tokio::task::spawn_blocking(move || solver::solve_files(sourcemap)).await?;

    Ok(minimized_sources)
}

enum FetchResult {
    Directory(PathBuf),
    File(PathBuf),
}

async fn fetch_files(
    source: &Source,
    state: &SolGateState,
) -> Result<FetchResult, FileAcquisitionError> {
    // Step one, download file.
    let mut tx = state.sql_pool.begin().await?;
    let hash_vec = get_hashes_from_ids(&vec![source.h_id], &mut tx).await?;
    let hash = hash_vec.get(0).unwrap();
    tx.commit().await?;
    let hash_str = util::split_hash(&hash.val);
    let save_loc = state
        .config
        .read()
        .await
        .local_settings
        .temp_dir
        .join(hash_str);
    let client = state.http_client.clone();
    get_http_source(client, &source.path, &save_loc).await?;
    let location_string = save_loc.clone().to_string_lossy().to_string();
    // Step two, index it.
    if source.format == SourceFormat::SevenZip {
        // Extract and index our contents.
        let extract_dir = save_loc.join("/extract");
        sevenz_extract(&save_loc, &extract_dir)
            .await
            .expect("7z extraction failed");
        Ok(FetchResult::Directory(extract_dir.clone()))
    } else {
        Ok(FetchResult::File(save_loc.clone()))
    }
}

pub async fn get_http_source(
    client: Client,
    url: &str,
    save_loc: &impl AsRef<Path>,
) -> Result<(), FileAcquisitionError> {
    let res = client.get(url).send().await?;
    let mut stream = res.bytes_stream();

    let dir = save_loc.as_ref().clone().parent().unwrap();
    tokio::fs::DirBuilder::new()
        .recursive(true)
        .create(dir)
        .await?;
    let mut outfile = File::create(save_loc).await?;

    while let Some(item) = stream.next().await {
        let chunk = item?;
        outfile.write_all(&chunk).await?;
    }
    Ok(())
}

pub async fn vp_writer<T: AsyncWrite + std::marker::Unpin>(
    file_like: &mut T,
    mut rx: mpsc::Receiver<(VPEntry, Option<oneshot::Receiver<Vec<u8>>>)>,
) -> Result<(), Box<dyn std::error::Error>> {
    todo!(); // Nowhere near complete lol.
             // let vpstuff = 6; // TODO
             // while let Some((info, data)) = rx.recv().await {
             //     let d_len = data.len();
             //     file_like.write_all(&data).await?;
             //     // Uuh
             // }
}
#[derive(Debug, thiserror::Error)]
pub enum FileAcquisitionError {
    #[error("Network Error")]
    NetworkError(reqwest::Error),
    #[error("IO Error")]
    IOError(std::io::Error),
    #[error("SQL Error: {0}")]
    SqlxError(sqlx::Error),
    #[error("Tokio Runtime Error: {0}")]
    JoinError(JoinError),
    #[error("Program Logic Error: {0}")]
    LogicError(String),
}

impl From<sqlx::Error> for FileAcquisitionError {
    fn from(err: sqlx::Error) -> Self {
        FileAcquisitionError::SqlxError(err)
    }
}
impl From<JoinError> for FileAcquisitionError {
    fn from(err: JoinError) -> Self {
        FileAcquisitionError::JoinError(err)
    }
}
impl From<reqwest::Error> for FileAcquisitionError {
    fn from(err: reqwest::Error) -> Self {
        FileAcquisitionError::NetworkError(err)
    }
}
impl From<std::io::Error> for FileAcquisitionError {
    fn from(err: std::io::Error) -> Self {
        FileAcquisitionError::IOError(err)
    }
}
impl From<UrlError> for FileAcquisitionError {
    fn from(err: UrlError) -> Self {
        Self::LogicError(err.to_string())
    }
}

impl From<IndexError> for FileAcquisitionError {
    fn from(value: IndexError) -> Self {
        match value {
            IndexError::IOError(ioerr) => FileAcquisitionError::IOError(ioerr),
            IndexError::SqlxError(sqlerr) => FileAcquisitionError::SqlxError(sqlerr),
        }
    }
}
