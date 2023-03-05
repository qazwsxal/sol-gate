use std::collections::{HashMap};
use std::fmt::Debug;
use std::io::SeekFrom;
use std::path::{Path, PathBuf};

use crate::db;
use db::queries;
use db::{Archive, SHA256Checksum};
use futures::StreamExt;
use sqlx::pool::PoolConnection;
use sqlx::Acquire;
use vp::{
    self,
    types::VPFile,
};

use super::DataPath;
use bytes::Bytes;
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use tokio::sync::mpsc;
use tokio_util::io::ReaderStream;

use rand::seq::SliceRandom;
#[derive(Debug)]
pub enum Get {
    CS(SHA256Checksum),
    Path(DataPath),
}

#[derive(Debug)]
pub struct GetRequest {
    pub contents: Get,
    pub channel: mpsc::Sender<Result<Bytes, ReaderError>>,
    pub queue: bool, // If resource handler should prefer queueing requests or grabbing immediately.
}
pub enum VPRequestMsg {
    Read(String, mpsc::Sender<Result<Bytes, ReaderError>>),
    Exit(),
}

impl Debug for VPRequestMsg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Read(string, _stream) => f.debug_tuple("Read").field(string).finish(),
            Self::Exit() => f.debug_tuple("Exit").finish(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ReaderError {
    #[error("IO Error")]
    IOError(std::io::Error),
    #[error("SQL Error: {0}")]
    SqlxError(sqlx::Error),
    #[error("Could not locate checksum {0:?}")]
    LocateError(SHA256Checksum),
    #[error("Could not locate file {1} in {0}")]
    VPError(PathBuf, String),
    #[error("Tokio Runtime Error: {0}")]
    JoinError(tokio::task::JoinError),
}

impl From<std::io::Error> for ReaderError {
    fn from(err: std::io::Error) -> Self {
        ReaderError::IOError(err)
    }
}
impl From<sqlx::Error> for ReaderError {
    fn from(err: sqlx::Error) -> Self {
        ReaderError::SqlxError(err)
    }
}
impl From<tokio::task::JoinError> for ReaderError {
    fn from(err: tokio::task::JoinError) -> Self {
        ReaderError::JoinError(err)
    }
}

pub struct ReaderPoolActor {
    receiver: mpsc::Receiver<GetRequest>,
    sql_conn: PoolConnection<sqlx::Sqlite>,
    vp_channels: HashMap<PathBuf, Vec<VPReadHandle>>,
}

impl ReaderPoolActor {
    pub fn new(
        sql_conn: PoolConnection<sqlx::Sqlite>,
        receiver: mpsc::Receiver<GetRequest>,
    ) -> Self {
        // Maintain a map of open connections to each of the tasks,
        let vp_channels = HashMap::<PathBuf, Vec<VPReadHandle>>::new();

        ReaderPoolActor {
            receiver,
            sql_conn,
            vp_channels,
        }
    }
    async fn handle_msg(&mut self, get_request: GetRequest) {
        let path_res = match get_request.contents {
            // If we're getting something by checksum, locate where we can find it.
            Get::CS(checksum) => self.get_path(&checksum).await,
            Get::Path(path) => Ok(path),
        };

        match path_res {
            Err(patherr) => {
                get_request.channel.send(Err(patherr)).await.unwrap();
                return;
            }
            Ok(path) => {
                match path {
                    DataPath::Raw(fp) => {
                        tokio::spawn(get_file(fp, get_request.channel));
                    }
                    DataPath::VPEntry(fp, entry) => {
                        let channels = self.vp_channels.get_mut(&fp).unwrap();
                        // Clean up closed channels.
                        channels.retain(|c| !c.tx.is_closed());

                        if get_request.queue && !channels.is_empty() {
                            // Queue on random existing vec.
                            let handle = channels.choose(&mut rand::thread_rng()).unwrap();
                            handle
                                .tx
                                .send(VPRequestMsg::Read(entry, get_request.channel))
                                .await
                                .unwrap();
                        } else {
                            // Otherwise make an new readhandle.
                            let handle = VPReadHandle::new(fp).await;
                            handle
                                .tx
                                .send(VPRequestMsg::Read(entry, get_request.channel))
                                .await
                                .unwrap();
                            channels.push(handle)
                        }
                    }
                    DataPath::SZEntry(_fp, _entry) => unimplemented!(),
                }
            }
        }
    }

    async fn get_path(&mut self, checksum: &SHA256Checksum) -> Result<DataPath, ReaderError> {
        let sql_res = self.sql_conn.begin().await;
        // Really hope this doesn't fail lol
        let mut sql_tx = sql_res?;
        // We've got a transaction!
        // Get a set of valid DataPaths for the checksum we want.
        let direct_sources = queries::get_sources_from_hash(&checksum, &mut sql_tx).await?;

        if direct_sources.is_empty() {
            // We need to find a VP containing our file.
            let h_id = queries::get_id_from_hash(&checksum, &mut sql_tx).await?;
            let hid_vec = vec![h_id];
            let parents = queries::get_parents_from_ids(&hid_vec, &mut sql_tx).await?;
            let parent_map = parents
                .into_iter()
                .filter(|ae| ae.archive_type == Archive::VP) // Don't know how to extract from 7z yet.
                .map(|p| (p.archive_id, p.file_path))
                .collect::<HashMap<i64, String>>();
            let parent_ids = parent_map.keys().cloned().collect();
            let sources = queries::get_sources_from_ids(&parent_ids, &mut sql_tx).await?;
            let best_source = sources.iter().min().unwrap();
            let inner_path = parent_map.get(&best_source.h_id).unwrap();

            Ok(DataPath::VPEntry(
                best_source.path.clone().into(),
                inner_path.clone(),
            ))
        } else {
            // If a direct source exists, just use that.
            // Sort first so we prefer reading from tempdir. it's more likely to be stored on an SSD
            Ok(DataPath::Raw(
                direct_sources.iter().min().unwrap().path.clone().into(),
            ))
        }
    }
}

// We keep this as a seperate function - it could be a method on ReaderPoolActor,
// but the ownership model can trip new coders up as it needs (mut self) not (&mut self).
// see Alice Rhyl's material on Actors in Tokio.
async fn run_reader_pool(mut pool: ReaderPoolActor) {
    while let Some(msg) = pool.receiver.recv().await {
        pool.handle_msg(msg).await
    }
}

#[derive(Debug, Clone)]
pub struct ReaderPoolHandle {
    pub tx: mpsc::Sender<GetRequest>,
}

impl ReaderPoolHandle {
    pub fn new(sql_conn: PoolConnection<sqlx::Sqlite>) -> Self {
        let (send, rec) = mpsc::channel(8);
        let actor = ReaderPoolActor::new(sql_conn, rec);
        tokio::spawn(run_reader_pool(actor));

        ReaderPoolHandle { tx: send }
    }
}

// As we're going to be streaming the entire file once,
// and we are then unlikely to need it again,
// it doesn't make sense to hold a file handle open.
// Instead we just spawn a task to get the file
// and then let the function clean itself up once complete.
pub async fn get_file(filepath: PathBuf, tx: mpsc::Sender<Result<Bytes, ReaderError>>) -> () {
    match tokio::fs::File::open(filepath).await {
        Ok(file) => {
            // TODO: send an error down the tx stream.
            let mut stream = ReaderStream::new(file);
            while let Some(chunk) = stream.next().await {
                tx.send(chunk.map_err(|e| e.into())).await.unwrap();
            }
        }
        Err(io_err) => tx.send(Err(io_err.into())).await.unwrap(),
    }
}

struct VPReadActor {
    vp: tokio::fs::File,
    path_map: HashMap<String, VPFile>,
    receiver: mpsc::Receiver<VPRequestMsg>,
}

impl VPReadActor {
    const BUFSIZE: usize = 4096; // same as ReaderStream.
    async fn new(file_path: impl AsRef<Path>, rx: mpsc::Receiver<VPRequestMsg>) -> Self {
        let mut vp = tokio::fs::File::open(file_path).await.unwrap();
        let index = vp::fs::async_index(&mut vp).await.unwrap();

        let path_map = HashMap::<String, VPFile>::from_iter(
            index
                .flatten()
                .into_iter()
                .map(|vpf| (vpf.name.clone(), vpf)),
        );

        VPReadActor {
            vp,
            path_map,
            receiver: rx,
        }
    }

    async fn handle_msg(&mut self, request: VPRequestMsg) {
        match request {
            VPRequestMsg::Exit() => return, // We've been told to quit, so do so.
            VPRequestMsg::Read(path, tx) => {
                let vpfile = self.path_map.get(&path).unwrap();
                self.vp
                    .seek(SeekFrom::Start(vpfile.fileoffset))
                    .await
                    .unwrap();
                // TODO: Check for compression
                // We're basically manually doing a ReaderStream thing here
                // as it means we don't have ownership issues over vp
                // Also we can bound how big our read is too.
                let size: usize = vpfile.size.try_into().unwrap();
                let mut buf = Box::new([0u8; Self::BUFSIZE]);
                let mut readlen: usize = 0;
                // Read BUFSIZE, or if there's less than that remaining, read that many bytes.
                let mut readsize = std::cmp::min(size - readlen, Self::BUFSIZE);
                loop {
                    let read_result = self.vp.read(&mut (buf[..readsize])).await;
                    match read_result {
                        Ok(len) => {
                            tx.send(Ok(Bytes::copy_from_slice(&buf[..len])))
                                .await
                                .unwrap();
                            readlen += len;
                            // Read BUFSIZE, or if there's less than that remaining, read that.
                            readsize = std::cmp::min(size - readlen, Self::BUFSIZE);
                            if readsize == 0 {
                                return;
                            }
                        }
                        Err(e) => {
                            tx.send(Err(e.into())).await.unwrap();
                            return;
                        }
                    };
                }
            }
        }
    }
}

// We keep this as a seperate function - it could be a method on ReaderPoolActor,
// but the ownership model can trip new coders up as it needs (mut self) not (&mut self). 
// see Alice Rhyl's material on Actors in Tokio.
async fn run_vp_reader(mut vp_reader: VPReadActor) {
    use tokio::time::{timeout, Duration};
    let duration = Duration::from_millis(500); // Hang around for 500ms
                                               // If we don't get a new request after 500ms, close the
    while let Ok(Some(msg)) = timeout(duration, vp_reader.receiver.recv()).await {
        match msg {
            VPRequestMsg::Exit() => break, // We've been told to quit, so do so.
            _ => (),
        } 
        vp_reader.handle_msg(msg).await
    }
}
#[derive(Debug, Clone)]
pub struct VPReadHandle {
    pub tx: mpsc::Sender<VPRequestMsg>,
}

impl VPReadHandle {
    async fn new(file_path: impl AsRef<Path>) -> Self {
        let (send, rec) = mpsc::channel(8);
        let actor = VPReadActor::new(file_path, rec).await;
        tokio::spawn(run_vp_reader(actor));

        VPReadHandle { tx: send }
    }
}
