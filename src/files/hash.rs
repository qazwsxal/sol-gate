use bytes::Bytes;
use sha2::{self, Digest};
use tokio::sync::mpsc;

use crate::common::SHA256Checksum;

use super::readers::ReaderError;



pub async fn hash_channel(mut rx: mpsc::Receiver<Result<Bytes, ReaderError>>) -> SHA256Checksum {
    let mut hasher = sha2::Sha256::new();
    while let Some(chunk) = rx.recv().await {
        hasher.update(chunk.unwrap());
    }
    SHA256Checksum(hasher.finalize().into_iter().collect())
}
