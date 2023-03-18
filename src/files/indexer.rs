use std::{ffi::OsStr, path::Path};

use bytes::Bytes;
use futures::stream::{self, StreamExt};
use hash_hasher::HashedMap;
use tokio::sync::mpsc;
use vp::fs::async_index;
use walkdir::WalkDir;

use super::{
    hash::hash_channel,
    readers::{Get, GetRequest, ReaderError},
    DataPath,
};
use crate::common::{Archive, ArchiveEntry, Source, SourceFormat};
use crate::{
    db::{
        queries::{add_archive_entries, add_hashes, add_sources, get_hash_ids},
        SourceLocation,
    },
    SolGateState,
};

#[derive(Debug, thiserror::Error)]
pub enum IndexError {
    #[error("IO Error")]
    IOError(std::io::Error),
    #[error("SQL Error: {0}")]
    SqlxError(sqlx::Error),
}

impl From<sqlx::Error> for IndexError {
    fn from(err: sqlx::Error) -> Self {
        IndexError::SqlxError(err)
    }
}

impl From<std::io::Error> for IndexError {
    fn from(err: std::io::Error) -> Self {
        IndexError::IOError(err)
    }
}

pub async fn index_dir(
    dir: impl AsRef<Path>,
    state: SolGateState,
    location: SourceLocation,
) -> Result<(), IndexError> {
    let files = WalkDir::new(dir)
        .into_iter()
        .filter_map(|file| file.ok())
        .filter(|f| f.metadata().unwrap().is_file());
    let mut tasks = tokio::task::JoinSet::new();

    for file in files {
        let file2 = file.clone();
        let state2 = state.clone();
        // hash and index the file itself.
        tasks
            .build_task()
            .name(format!("index - {}", file.path().to_string_lossy()).as_str())
            .spawn(async move { index_file(file2.path(), state2, location).await });
    }
    while let Some(result) = tasks.join_next().await {
        result.unwrap()?;
    }
    Ok(())
}

pub async fn index_file(
    filepath: impl AsRef<Path>,
    state: SolGateState,
    location: SourceLocation,
) -> Result<(), IndexError> {
    let path = filepath.as_ref().to_path_buf();
    let dp = DataPath::Raw(path.clone());
    let (hash_tx, hash_rx) = mpsc::channel::<Result<Bytes, ReaderError>>(5);
    state
        .reader_pool
        .tx
        .send(GetRequest {
            contents: Get::Path(dp),
            channel: hash_tx,
            queue: true,
        })
        .await
        .expect("Send failed, but it's infallible???");
    // We're reading from disk here, so spawn as a seperate task
    let hash_jh = tokio::spawn(async move { hash_channel(hash_rx).await });
    // TODO: Find hash_id and commit.
    let mut sql_tx = state.sql_pool.begin().await?;
    let hash = hash_jh.await.unwrap(); // hash_channel can't panic so unwrap() is ok here
    let hashvec = vec![hash.clone()];

    add_hashes(&hashvec, &mut sql_tx).await?;
    let file_hid = get_hash_ids(&hashvec, &mut sql_tx)
        .await?
        .get(0)
        .unwrap() // It's fetching the hash of the one we just added.
        .1
        .clone();
    let file_format: SourceFormat;
    let mut vp_jh = None;
    if path.extension() == Some(&OsStr::new("vp")) {
        // Index the VP contents too.
        let path2 = path.clone();
        vp_jh = Some(tokio::spawn(async move {
            index_vp(&path2, state.clone(), file_hid).await
        }));
        file_format = SourceFormat::VP;
    } else if path.extension() == Some(&OsStr::new("vpc")) {
        todo!("Compressed VP handling not implemented yet.")
    } else {
        file_format = SourceFormat::Raw;
    }
    let size = tokio::fs::metadata(filepath).await?.len();
    let file_source = Source {
        path: path.to_string_lossy().to_string(),
        h_id: file_hid,
        size: size.try_into().unwrap(), // We're not expecting 9.2 Exabyte files so we'll be OK.
        format: file_format,
        location,
    };

    let sources = vec![file_source];
    add_sources(&sources, &mut sql_tx).await;
    Ok(())
}

pub async fn index_vp(
    path: &std::path::PathBuf,
    state: SolGateState,
    vp_hash_id: i64,
) -> Result<(), IndexError> {
    // It's a VP, so we need to index everything in it.
    let mut file = tokio::fs::File::open(path.clone()).await?;
    let idx = async_index(&mut file).await.unwrap(); //REALLY hope we're indexing a VP.
    drop(file);
    let vp_entries = idx.flatten();
    let names: Vec<String> = vp_entries
        .iter()
        .map(|vp_path| vp_path.name.clone())
        .collect();
    // No point making this concurrent as the current reader_pool implementation
    // is sequential for files in the same .vp
    let mut hashes = Vec::with_capacity(vp_entries.len());
    for vp_entry in vp_entries {
        let dp = DataPath::VPEntry(path.clone(), vp_entry.name);
        let (hash_tx, hash_rx) = mpsc::channel::<Result<Bytes, ReaderError>>(5);
        state
            .reader_pool
            .tx
            .send(GetRequest {
                contents: Get::Path(dp),
                channel: hash_tx,
                queue: true,
            })
            .await
            .expect("Send failed, but it's infallible???");
        let hash = hash_channel(hash_rx).await;
        hashes.push(hash)
    }
    let mut sql_tx = state.sql_pool.begin().await.unwrap();
    add_hashes(&hashes, &mut sql_tx).await?;
    let hids = HashedMap::from_iter(get_hash_ids(&hashes, &mut sql_tx).await?.into_iter());

    let archive_entries = hashes
        .into_iter()
        .zip(names)
        .map(|(hash, file_path)| ArchiveEntry {
            file_id: hids.get(&hash).unwrap().clone(),
            file_path,
            archive_id: vp_hash_id,
            archive_type: Archive::VP,
        })
        .collect::<Vec<ArchiveEntry>>();
    add_archive_entries(&archive_entries, &mut sql_tx).await?;
    Ok(())
}
