use hex::encode;
use std::path::{Path, PathBuf};

use crate::{
    common::{SHA256Checksum, SourceLocation},
    config::Config,
};

#[derive(Debug, thiserror::Error)]
pub enum UrlError {
    #[error("Location Error {0}")]
    LocationError(String),
    #[error("Source Error {0}")]
    SourceError(String),
}

pub fn get_cs_path(temp_dir: impl AsRef<Path>, checksum: &SHA256Checksum) -> PathBuf {
    let cs_filename = split_hash(checksum);
    temp_dir.as_ref().clone().join(cs_filename)
}

pub fn split_hash(checksum: &SHA256Checksum) -> String {
    let data = checksum.0.clone();
    let cs_filename = format!("{:x}/{:x}/{}", data[0], data[1], encode(&data[2..]));
    cs_filename
}
// Fix the error type on this
pub fn get_urls(
    config: &Config,
    source_location: &SourceLocation,
    checksum: &SHA256Checksum,
) -> Result<Vec<String>, UrlError> {
    match source_location {
        SourceLocation::Local => Err(UrlError::LocationError(
            "Local sources don't need URLs!".to_string(),
        )),
        SourceLocation::Temp => Err(UrlError::LocationError(
            "Local sources don't need URLs!".to_string(),
        )),
        SourceLocation::Unmanaged => Err(UrlError::LocationError(
            "Local sources don't need URLs!".to_string(),
        )),
        SourceLocation::FSN => {
            let hashpath = split_hash(checksum);
            let repos = &config.fsnebula.repos;
            if repos.len() == 0 {
                Err(UrlError::SourceError(
                    "No FSNebula repositories specified.".to_string(),
                ))
            } else {
                let urls = repos.iter().map(|r| r.clone() + &hashpath).collect();
                Ok(urls)
            }
        }
        SourceLocation::SolGate => {
            let hashpath = split_hash(checksum);
            let gates = &config.gates;
            if gates.len() == 0 {
                Err(UrlError::SourceError(
                    "No sol-gate repositories specified.".to_string(),
                ))
            } else {
                let urls = gates.iter().map(|r| r.url.clone() + &hashpath).collect();
                Ok(urls)
            }
        }
    }
}
