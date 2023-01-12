use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::{error::Error, fs};
use std::{fmt, io};

use axum::{
    self,
    body::StreamBody,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};

use futures_util::Stream;
use tokio::fs::File;
use tokio::io::AsyncRead;
use tokio_util::io::ReaderStream;

use crate::common::queries::get_sources_from_hash;
use crate::common::{EntryType, SHA256Checksum, SourceLocation};
use crate::config::Config;
use crate::SolGateState;
pub async fn router(config: Config) -> Result<(Router<SolGateState>), Box<dyn Error>> {
    // let app = Router::new().route("/:b1/:b2/:bx", get(get_file));

    //    Ok(app);
    todo!();
}

pub enum FileError {
    PathNotSpecified,
    HexDecodeError(hex::FromHexError),
    HexLengthError((usize, usize)),
    FileNotFound(Vec<u8>),
    SqlxError(sqlx::Error),
}

impl From<sqlx::Error> for FileError {
    fn from(err: sqlx::Error) -> Self {
        FileError::SqlxError(err)
    }
}

impl From<hex::FromHexError> for FileError {
    fn from(err: hex::FromHexError) -> Self {
        FileError::HexDecodeError(err)
    }
}

impl IntoResponse for FileError {
    fn into_response(self) -> axum::response::Response {
        match self {
            FileError::PathNotSpecified => {
                (StatusCode::BAD_REQUEST, "I don't know how you managed this").into_response()
            }
            FileError::HexDecodeError(part) => (
                StatusCode::BAD_REQUEST,
                format!("path part {} not parseable as hex", part),
            )
                .into_response(),
            FileError::HexLengthError((len, expect)) => (
                StatusCode::BAD_REQUEST,
                format!("expected {} chars, got {}", expect * 2, len * 2),
            )
                .into_response(),
            FileError::FileNotFound(hash) => (
                StatusCode::NOT_FOUND,
                format!("could not find {}", hex::encode(hash)),
            )
                .into_response(),
            FileError::SqlxError(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("sql failure: {}", err),
            )
                .into_response(),
        }
    }
}

pub(crate) async fn get_file(
    Path(params): Path<HashMap<String, String>>,
    State(state): State<SolGateState>,
) -> Result<StreamBody<Box<dyn Stream<Item = io::Result<&'static u8>>>>, FileError> {
    let mut tx = state.sql_pool.begin().await?;
    let b1 = params.get("b1").ok_or(FileError::PathNotSpecified)?;
    let b2 = params.get("b2").ok_or(FileError::PathNotSpecified)?;
    let bx = params.get("bx").ok_or(FileError::PathNotSpecified)?;
    let h1 = hex::decode(b1)?;
    let h2 = hex::decode(b2)?;
    let hx = hex::decode(bx)?;
    if h1.len() != 1 {
        return Err(FileError::HexLengthError((h1.len(), 1)));
    }
    if h2.len() != 1 {
        return Err(FileError::HexLengthError((h2.len(), 1)));
    }
    if hx.len() != 30 {
        return Err(FileError::HexLengthError((hx.len(), 30)));
    }
    let mut cs: Vec<u8> = Vec::with_capacity(32);
    cs.extend(h1);
    cs.extend(h2);
    cs.extend(hx);
    let hash = SHA256Checksum(cs);

    // Use hash to find where file is, get and transmit it.
    let sources = get_sources_from_hash(&hash, &mut tx).await?;
    todo!();
    // for source in sources.iter() {
    //     if source.location.is_local() {
    //         if source.s_type == EntryType::Raw {
    //             let path = state
    //                 .config
    //                 .read()
    //                 .await
    //                 .local_settings
    //                 .install_dir
    //                 .clone()
    //                 .join(&source.path);
    //             match File::open(path).await {
    //                 Ok(file) => {
    //                     // convert the `AsyncRead` into a `Stream`
    //                     let stream = ReaderStream::new(file);
    //                     // convert the `Stream` into an `axum::body::HttpBody`
    //                     let body = StreamBody::new(stream);
    //                     return Ok(body);
    //                 }
    //                 Err(_) => continue,
    //             };
    //         } else if source.s_type == EntryType::VPEntry {
    //             todo!()
    //         }
    //     }
    // }
    // return Err(FileError::FileNotFound(hash.0));
}
