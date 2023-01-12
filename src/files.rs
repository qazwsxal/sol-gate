use std::collections::{HashMap, HashSet, VecDeque};
use std::error::Error;
use std::fmt::Display;
use std::hash::{self, Hash, Hasher};
use std::io::{Read, Seek, SeekFrom};
use std::num::NonZeroUsize;
use std::ops::Deref;
use std::path::{Path, PathBuf};

use crate::{db, SolGateState};
use db::{queries::*, EntryType};
use db::{Mod, SHA256Checksum, Source, SourceLocation, SourceWithID};
use hash_hasher::HashedMap;
use itertools::Itertools;
use reqwest::Client;
use sha2::digest::typenum::NonZero;
use sqlx::{pool, Pool, Sqlite, Transaction};
use tokio::task::{AbortHandle, JoinError, JoinHandle};
use tower::util::error::optional::None;

use futures_util::StreamExt;
use tokio::fs::{DirBuilder, File};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncSeek, AsyncSeekExt, AsyncWrite, AsyncWriteExt};
use tokio::sync::{mpsc, oneshot};

use self::util::UrlError;

type PathOneshot = (String, oneshot::Sender<Vec<u8>>);

type VecPlusOneshot = (Vec<u8>, oneshot::Sender<Vec<u8>>);

pub mod api;
pub mod compression;
mod dag;
pub mod readers;
mod solver;
mod util;

type Manifest = Vec<Entry>;
#[derive(Clone)]
pub struct Entry {
    pub path: PathBuf,
    pub ident: Ident,
}

// It's either a raw file, or a VP.
#[derive(Clone)]
pub enum Ident {
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
    Raw,
    VPEntry(PathBuf),
    SZEntry(PathBuf),
}

pub struct ArchiveEntry {
    pub h_id: i64,
    pub path: Vec<DataPath>,
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

pub fn build_paths(
    start: &i64,
    ends: &HashSet<i64>,
    hid_hierarchy: &dag::HashDAG<i64, DataPath>,
) -> Vec<ArchiveEntry> {
    // Create a empty vector of paths.
    let mut paths = Vec::<ArchiveEntry>::new();
    // Early out here, we might have a direct source.
    if ends.contains(start) {
        return vec![ArchiveEntry {
            h_id: start.clone(),
            path: vec![DataPath::Raw],
        }];
    }

    // If we have any children of the start, iterate over them.
    if let Some(start_children) = hid_hierarchy.children(&start) {
        for child in start_children {
            let edge_data = hid_hierarchy.get_edge_data(&start, &child).unwrap();
            let path = vec![edge_data.clone()];
            if ends.contains(&child) {
                paths.push(ArchiveEntry {
                    h_id: child.clone(),
                    path: path.clone(),
                });
            }
            // We need to handle the possibility of nested Archives, so let's do it via a depth-first search.
            let child_paths = build_paths(&child, &ends, hid_hierarchy);
            let indirect_paths = child_paths
                .iter()
                .map(|entry| ArchiveEntry {
                    h_id: entry.h_id,
                    path: [path.clone(), entry.path.clone()].concat(), // The entry is the same, we just stick the path to the child on in front.
                });
            paths.extend(indirect_paths);
        }
    }
    paths
}

pub async fn acquire_files(
    state: SolGateState,
    manifest: &Manifest,
) -> Result<(), FileAcquisitionError> {
    let mut tx = state.sql_pool.begin().await?;

    let hashes = manifest
        .iter()
        .flat_map(|f| match &f.ident {
            Ident::Raw(hash) => vec![hash.clone()],
            Ident::VP(vp) => match &vp {
                VPContents::Hash(hash) => vec![hash.clone()],
                VPContents::Contents(entries) => entries
                    .iter()
                    .map(|ve| ve.hash.clone())
                    .collect::<Vec<SHA256Checksum>>(),
            },
        })
        .collect::<Vec<SHA256Checksum>>();
    let hids = get_hash_ids(&hashes, &mut tx)
        .await?
        .values()
        .cloned()
        .collect::<Vec<_>>();
    tx.commit().await?;
    
    // DAG of hash IDs, nodes without children are items from the manifest.
    let mut hid_hierarchy = generate_dag(&state, &hids).await?;

    let missing = calculate_missing(&state, &hids, &hid_hierarchy).await?;

    let sources = calculate_fetches(&state, &missing, &hid_hierarchy).await?;

    // We now know what fetches we want, so let's make a map of what we want from each source.

    let missing_set = HashSet::from_iter(missing);
    let mut fetch_vec =
        Vec::from_iter(sources.iter().map(|source| {
            (
                source.clone(),
                build_paths(&source.h_id, &missing_set, &hid_hierarchy),
            )
        }));

    let mut tasks = Vec::new();
    for (source, paths) in fetch_vec.into_iter() {
        // Make task-local clones of the system state, as they're cheap to copy and we're spawning tasks here.
        let stateclone = state.clone();
        tasks.push(tokio::spawn(async move {
            fetch_files(source, paths, stateclone).await
        }));
    }
    // need to handle these tasks.
    todo!()
}

pub async fn generate_dag(
    state: &SolGateState,
    ids: &Vec<i64>,
) -> Result<dag::HashDAG<i64, DataPath>, FileAcquisitionError> {
    let mut hid_hierarchy = dag::HashDAG::<i64, DataPath>::new();
    let mut self_ids = ids.clone();
    let mut tx = state.sql_pool.begin().await?;
    let mut parents = get_parents_from_ids(&self_ids, &mut tx).await?;
    while !parents.is_empty() {
        for (parent, children) in parents.iter() {
            let child_ids = children.iter().map(|(id, _, _)| id).cloned().collect();
            let edge_data = children
                .iter()
                .map(|(_, path, p_type)| match p_type {
                    EntryType::Raw => DataPath::Raw,
                    EntryType::SevenZipEntry => DataPath::SZEntry(path.into()),
                    EntryType::VPEntry => DataPath::VPEntry(path.into()),
                })
                .collect();
            hid_hierarchy.add_children(&child_ids, parent, &edge_data);
        }
        self_ids = parents.keys().cloned().collect();
        parents = get_parents_from_ids(&self_ids, &mut tx).await?;
    }
    Ok(hid_hierarchy)
}

pub async fn calculate_missing(
    state: &SolGateState,
    hids: &Vec<i64>,
    hid_hierarchy: &dag::HashDAG<i64, DataPath>,
) -> Result<Vec<i64>, FileAcquisitionError> {
    let mut missing_hids = HashSet::<i64>::from_iter(hids.iter().cloned());

    let mut hidpool = missing_hids.clone();
    for hid in missing_hids.iter() {
        let ancestors = hid_hierarchy.ancestors(&hid).unwrap();
        hidpool.extend(ancestors);
    }
    let mut tx = state.sql_pool.begin().await?;

    let sources = get_sources_from_ids(&hidpool.iter().cloned().collect(), &mut tx).await?;
    for source in sources {
        if source.location.is_local() {
            let source_hid = source.h_id;
            let descendants = hid_hierarchy.descendants(&source_hid).unwrap();
            for descendant in descendants {
                missing_hids.remove(&descendant);
            }
        }
    }

    Ok(missing_hids.into_iter().collect())
}

pub async fn calculate_fetches(
    state: &SolGateState,
    missing: &Vec<i64>,
    hid_hierarchy: &dag::HashDAG<i64, (DataPath)>,
) -> Result<Vec<Source>, FileAcquisitionError> {
    let mut tx = state.sql_pool.begin().await?;
    let missing_source_hids: Vec<i64> = missing
        .iter()
        .flat_map(|m| hid_hierarchy.ancestors(m).unwrap())
        .unique()
        .collect();

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

    // This is pretty CPU intensive and might block for a while, so run seperately
    let minimized_sources =
        tokio::task::spawn_blocking(move || solver::solve_files(sourcemap)).await?;

    Ok(minimized_sources)
}

pub async fn install_files(
    manifest: Manifest,
    mod_info: Mod,
    state: &SolGateState,
) -> Result<(), Box<dyn Error>> {
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
pub async fn fetch_files(
    source: Source,
    content: Vec<ArchiveEntry>,
    state: SolGateState,
) -> Result<(), FileAcquisitionError> {
    // Step one, download file.
    let mut tx = state.sql_pool.begin().await?;
    let hash_vec = get_hashes_from_ids(&vec![source.h_id], &mut tx).await?;
    let hash = hash_vec.get(0).unwrap();
    let hash_str = util::split_hash(&hash.val);
    let save_loc = state.config.read().await.local_settings.temp_dir.join(hash_str);
    get_http_source(state.http_client, &source.path, save_loc).await?;
    // Step Two, extract the stuff we want into tempdirs.
    todo!();

    // Step Three, add the stuff we've just extracted into the database.
    todo!();

}


pub async fn get_http_source(
    client: Client,
    url: &str,
    save_loc: impl AsRef<Path>,
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
