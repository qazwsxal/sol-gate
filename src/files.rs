use std::collections::{HashMap, hash_map::DefaultHasher};
use std::hash::{Hash, Hasher};
use std::io::{SeekFrom, Seek, Read};
use std::path::{Path, PathBuf};

use sevenz_rust::{SevenZReader, SevenZArchiveEntry, default_entry_extract_fn};
use tokio::task::{JoinHandle, AbortHandle};
use vp;
use vp::types::{VPEntry, VPFile};
use async_channel;
use crate::db;

use tokio::io::{AsyncRead, AsyncSeek, AsyncReadExt, AsyncSeekExt, AsyncWrite, AsyncWriteExt};
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinSet;

pub type PathOneshot = (String, oneshot::Sender<Vec<u8>>);

type VecPlusOneshot = (Vec<u8>, oneshot::Sender<Vec<u8>>);


pub async fn vp_reader<T: AsyncRead + AsyncSeek + std::marker::Unpin>(
    file_like: &mut T,
    mut rx: mpsc::Receiver<PathOneshot>,
    decompress_queue: Option<async_channel::Sender<VecPlusOneshot>>
) -> Result<(), Box<dyn std::error::Error>> {
    let index = vp::fs::async_index(file_like).await?;

    let path_map = HashMap::<String, VPFile>::from_iter(
        index
            .flatten()
            .into_iter()
            .map(|vpf| (vpf.name.clone(), vpf)),
    );

    while let Some((path, os_tx)) = rx.recv().await {
        let vpfile = path_map.get(&path).unwrap();
        file_like.seek(SeekFrom::Start(vpfile.fileoffset)).await?;
        let mut buf = vec![0u8; vpfile.size.try_into().unwrap()];
        file_like.read_exact(&mut buf).await?;
        
        match decompress_queue {
            Some(ref dc_tx) => dc_tx.send((buf, os_tx)).await?, // Send the vector (to be decompressed) and the oneshot too.
            None => os_tx.send(buf).unwrap() // Yeah don't really know what to do if the oneshot sender fails here.
        };
    };
    Ok(())
}

// Unfortunately due to the library implementation, this relies on blocking read operations. 
// reccommend spawning on tokio::task::spawn_blocking as read operations over network *will* block for a long time.
pub async fn sevenz_reader<T: Read + Seek>(
    file_like: &mut T,
    temp_dir: impl AsRef<Path>,
    mut rx: mpsc::Receiver<PathOneshot>,
) -> Result<(), Box<dyn std::error::Error>> {
    let pos = file_like.stream_position()?;
    let len = file_like.seek(SeekFrom::End(0))?;
    file_like.seek(SeekFrom::Start(pos))?;
    let seven = SevenZReader::new(&mut *file_like, len, "".into())?;

    let path_map = HashMap::<String, SevenZArchiveEntry>::from_iter(
        seven.archive.files.iter().map(|f| (f.name.clone(), f.clone()))
    );

    let dest = PathBuf::from(temp_dir.as_ref());

    let mut hasher =  DefaultHasher::new();
    while let Some((path, os_tx)) = rx.recv().await {
        if let Some(entry) = path_map.get(&path) {
            path.hash(&mut hasher);
            let temp_name = format!("{:x}", hasher.finish());
            let mut outpath = dest.clone();
            outpath.push(temp_name);
            default_entry_extract_fn(entry, file_like, &outpath)?;

            let buf = tokio::fs::read(path).await?;
            os_tx.send(buf).unwrap();
        }
    };
    Ok(())
}


pub async fn raw_reader(
    rx: async_channel::Receiver<PathOneshot>,
) -> Result<(), Box<dyn std::error::Error>> {
    while let Ok((path, os_tx)) = rx.recv().await {
        let buf =  tokio::fs::read(path).await?;
        os_tx.send(buf).unwrap(); // Yeah don't really know what to do if the oneshot sender fails here.
    };
    Ok(())
}

pub async fn vp_writer<T: AsyncWrite + std::marker::Unpin>(
    file_like: &mut T,
    mut rx: mpsc::Receiver<(VPEntry, Option<oneshot::Receiver<Vec<u8>>>)>
) -> Result<(), Box<dyn std::error::Error>> {
    todo!(); // Nowhere near complete lol.
    // let vpstuff = 6; // TODO
    // while let Some((info, data)) = rx.recv().await {
    //     let d_len = data.len();
    //     file_like.write_all(&data).await?;
    //     // Uuh
    // }
    Ok(())
}

pub async fn spawn_decompressors(
    queue_len: usize, dc_count: Option<usize>
) -> (async_channel::Sender<VecPlusOneshot>, JoinSet<()>) {
    
    let (tx_uc, rx_uc) = async_channel::bounded::<VecPlusOneshot>(queue_len);
    let threads = dc_count.unwrap_or_else(num_cpus::get);
    let mut decompress_tasks = JoinSet::new();
    for _ in 0..threads {
        let rx = rx_uc.clone();
        decompress_tasks.spawn(
            async move { // decompression cannot fail, it can only be failed
            while let Ok((entry, os_tx)) = rx.recv().await { // Recieve a vector to be decompressed, and the oneshot for it too.
                let contents = vp::compression::maybe_decompress(entry);
                os_tx.send(contents).unwrap() // Yeah, again, don't really know what to do if the oneshot sender fails here.
            };
        }
    );
    }
    (tx_uc, decompress_tasks)
}


pub async fn spawn_compressors(
    queue_len: usize, dc_count: Option<usize>
) -> (async_channel::Sender<VecPlusOneshot>, JoinSet<()>) {
    
    let (tx_uc, rx_uc) = async_channel::bounded::<VecPlusOneshot>(queue_len);
    let threads = dc_count.unwrap_or_else(num_cpus::get);
    let mut compress_tasks = JoinSet::new();
    for _ in 0..threads {
        let rx = rx_uc.clone();
        compress_tasks.spawn(
            async move { // decompression cannot fail, it can only be failed
            while let Ok((entry, os_tx)) = rx.recv().await { // Recieve a vector to be decompressed, and the oneshot for it too.
                let contents = vp::compression::maybe_decompress(entry);
                os_tx.send(contents).unwrap() // Yeah, again, don't really know what to do if the oneshot sender fails here.
            };
        }
    );
    }
    (tx_uc, compress_tasks)
}