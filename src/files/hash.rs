use sha2::{self, Digest};
use tokio::sync::mpsc::channel;

use crate::common::SHA256Checksum;

use super::readers::RecieveChannel;

pub async fn hash_channel(mut rx: RecieveChannel) -> SHA256Checksum {
    let mut hasher = sha2::Sha256::new();
    while let Some(chunk) = rx.recv().await {
        hasher.update(chunk.unwrap());
    }
    SHA256Checksum(hasher.finalize().into_iter().collect())
}
