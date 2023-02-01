use std::collections::{hash_map::DefaultHasher, HashMap};
use std::error::Error;
use std::fmt::Debug;
use std::hash::{Hash, Hasher};
use std::io::{Read, Seek, SeekFrom};
use std::ops::DerefMut;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::db;
use async_channel;
use db::queries::{get_sources_from_id, get_sources_from_ids};
use db::Source;
use futures::{future::BoxFuture, Future, StreamExt};
use tokio::sync::mpsc::error::SendError;
use vp::{
    self,
    types::{VPEntry, VPFile},
};

use super::DataPath;
use bytes::Bytes;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncSeek, AsyncSeekExt, AsyncWrite, AsyncWriteExt};
use tokio::sync::{mpsc, oneshot, Mutex, RwLock};
use tokio::task::{AbortHandle, JoinHandle, JoinSet};
use tokio_util::io::ReaderStream;

pub type StreamItem = Result<Bytes, std::io::Error>;

pub type SendChannel = mpsc::Sender<StreamItem>;
pub type RecieveChannel = mpsc::Receiver<StreamItem>;

pub enum ReaderRequest {
    Read(String, SendChannel),
    Exit(),
}

impl Debug for ReaderRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Read(string, _stream) => f.debug_tuple("Read").field(string).finish(),
            Self::Exit() => f.debug_tuple("Exit").finish(),
        }
    }
}

pub type RequestSender = mpsc::Sender<ReaderRequest>;
pub type RequestReciver = mpsc::Receiver<ReaderRequest>;
#[derive(Debug, Clone)]
pub struct ReaderPool {
    // Maintain a map of open connections to each of the tasks,
    vp_channels: Arc<RwLock<HashMap<PathBuf, RequestSender>>>,
    sz_channels: Arc<RwLock<HashMap<PathBuf, RequestSender>>>,
    // Need to keep task handles open to them too.
    task_pool: Arc<Mutex<JoinSet<()>>>,
}

type TODO = ();

impl ReaderPool {
    pub fn new() -> ReaderPool {
        ReaderPool {
            vp_channels: Arc::new(RwLock::new(HashMap::new())),
            sz_channels: Arc::new(RwLock::new(HashMap::new())),
            task_pool: Arc::new(Mutex::new(JoinSet::new())),
        }
    }

    pub async fn get(&mut self, entry: DataPath) -> RecieveChannel {
        // We'll find or create a reader, we need a way of reciving the data back.
        // Buffered with a size of 32 as like, we probably want some buffering right?
        // The standard read-chunk size is 4kB, so per handle we're expecting at worse 128kB.
        // but at the same time it's not like buffering LOADS is going to speed up writes much,
        // it's just more memory usage.
        let (tx, rx) = mpsc::channel(32);

        match entry {
            DataPath::Raw(filepath) => {
                // Spawn a task to just stream out the file.
                self.task_pool
                    .lock()
                    .await
                    .build_task()
                    .name(format!("raw - {}", filepath.display()).as_str())
                    .spawn(async move { get_file(filepath, tx).await });
            }
            DataPath::VPEntry(filepath, path) => {
                let vps = self.vp_channels.read().await;

                // Check if we haven't already got this source listed.
                if !vps.contains_key(&filepath) {
                    drop(vps); // Can't find it, so we need write access.
                    let mut vps_write = self.vp_channels.write().await;
                    // build our channels and tasks.
                    let (rs_tx, rs_rx) = mpsc::channel(100);
                    vps_write.insert(filepath.clone(), rs_tx);
                    let fp_clone = filepath.clone();
                    self.task_pool
                        .lock()
                        .await
                        .build_task()
                        .name(format!("vp - {}", filepath.display()).as_str())
                        .spawn(async move { vp_reader(fp_clone, rs_rx).await });
                }
                // now we definitely have a reader, send the request.

                let vps = self.vp_channels.read().await;
                let rs_tx = vps.get(&filepath).unwrap();
                let req = ReaderRequest::Read(path, tx);
                rs_tx.send(req).await.unwrap();
            } 
            DataPath::SZEntry(filepath, path) => { todo!()
                //   let szs = self.sz_channels.read().await;

                //   if !szs.contains_key(&filepath) {
                //       drop(szs); // Can't find it, so we need write access.
                //       let mut szs_write = self.vp_channels.write().await;
                //       // build our channels and tasks.
                //       let (rs_tx, rs_rx) = mpsc::channel(100);
                //       szs_write.insert(filepath.clone(), rs_tx);
                //       let fp_clone = filepath.clone();
                //       let temp_dir = self.temp_dir.clone();
                //       self.task_pool
                //           .build_task()
                //           .name(format!("7z - {}", filepath.display()).as_str())
                //           .spawn(async move {
                //               tokio::task::spawn_blocking(|| sevenz_reader(fp_clone, temp_dir, rs_rx))
                //                   .await
                //                   .unwrap()
                //           });
                //   }
                //   // now we definitely have a reader, send the request.
                //   let szs = self.sz_channels.read().await;
                //   let rs_tx = szs.get(&filepath).unwrap();
                //   let req = ReaderRequest::Read(path, tx);
                //   rs_tx.send(req).await.unwrap();
              }
        };

        rx
    }
}

// As we're going to be streaming the entire file once,
// and we are then unlikely to need it again,
// it doesn't make sense to hold a file handle open.
// Instead we just spawn a task to get the file
// and then let the function clean itself up once complete.
pub async fn get_file(filepath: PathBuf, tx: SendChannel) -> () {
    let file = tokio::fs::File::open(filepath).await.unwrap(); // TODO: send an error down the tx stream.
    let mut stream = ReaderStream::new(file);
    while let Some(chunk) = stream.next().await {
        tx.send(chunk).await.unwrap();
    }
}

pub async fn vp_reader(
    file_path: impl AsRef<Path>,
    mut rx: mpsc::Receiver<ReaderRequest>,
    // decompress_queue: Option<async_channel::Sender<TODO>>, // we'll implement decompression later
) -> () {
    let mut vp = tokio::fs::File::open(file_path).await.unwrap();
    let index = vp::fs::async_index(&mut vp).await.unwrap();

    let path_map = HashMap::<String, VPFile>::from_iter(
        index
            .flatten()
            .into_iter()
            .map(|vpf| (vpf.name.clone(), vpf)),
    );

    while let Some(request) = rx.recv().await {
        match request {
            ReaderRequest::Exit() => break, // We've been told to quit, so do so.
            ReaderRequest::Read(path, tx) => {
                let vpfile = path_map.get(&path).unwrap();
                vp.seek(SeekFrom::Start(vpfile.fileoffset)).await.unwrap();
                // TODO: Check for compression
                // We're basically manually doing a ReaderStream thing here
                // as it means we don't have ownership issues over vp
                // Also we can bound how big our read is too.
                const BUFSIZE: usize = 4096; // same as ReaderStream.
                let size: usize = vpfile.size.try_into().unwrap();
                let mut buf = Box::new([0u8; BUFSIZE]);
                let mut readlen: usize = 0;
                // Read BUFSIZE, or if there's less than that remaining, read that many bytes.
                let mut readsize = std::cmp::min(size - readlen, BUFSIZE);
                while let read_result = vp.read(&mut (buf[..readsize])).await {
                    match read_result {
                        Ok(len) => {
                            tx.send(Ok(Bytes::copy_from_slice(&buf[..len])))
                                .await
                                .unwrap();
                            readlen += len;
                            // Read BUFSIZE, or if there's less than that remaining, read that.
                            readsize = std::cmp::min(size - readlen, BUFSIZE);
                            if readsize == 0 {
                                break;
                            }
                        }
                        Err(e) => tx.send(Err(e)).await.unwrap(),
                    };
                }
            }
        }
    }
    ()
}
