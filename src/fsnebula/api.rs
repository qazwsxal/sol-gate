use std::collections::HashSet;
use std::fmt;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::{error::Error, fs};

use axum::{
    self,
    extract::{ConnectInfo, State},
    routing::get,
    Json, Router,
};

use axum_macros::FromRef;
use serde::Serialize;
use serde_json;
use sqlx::SqlitePool;
use tokio::task::JoinError;

use super::{db, FSNebula};
use crate::config::FSNPaths;
use crate::fsnebula::structs::FSNMod;
use crate::SolGateState;
#[derive(Clone, FromRef)]
pub struct FSNState {
    pool: SqlitePool,
    urls: FSNPaths,
    cache: PathBuf,
}

pub async fn router(appdir: &Path) -> Result<Router<SolGateState>, Box<dyn Error>> {
    let cache = appdir.join("fsnebula");
    tokio::fs::create_dir_all(&cache).await?;
    let app = Router::new().route("/update", get(mod_update));

    Ok(app)
}

#[derive(Debug)]
enum UpdateError {
    IOError(std::io::Error),
    ParseError(serde_json::Error),
    RequestError(reqwest::Error),
    JoinError(JoinError),
}
impl std::error::Error for UpdateError {}

impl fmt::Display for UpdateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", &self)
    }
}

impl From<serde_json::Error> for UpdateError {
    fn from(err: serde_json::Error) -> Self {
        UpdateError::ParseError(err)
    }
}

impl From<std::io::Error> for UpdateError {
    fn from(err: std::io::Error) -> Self {
        UpdateError::IOError(err)
    }
}

impl From<reqwest::Error> for UpdateError {
    fn from(err: reqwest::Error) -> Self {
        UpdateError::RequestError(err)
    }
}

impl From<JoinError> for UpdateError {
    fn from(err: JoinError) -> Self {
        UpdateError::JoinError(err)
    }
}

#[derive(Debug, Serialize, Default)]
struct UpdateInfo {
    status: String,
    get_time: u128,
    commit_time: u128,
}

async fn mod_update(
    State(state): State<SolGateState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> Result<Json<UpdateInfo>, String> {
    println!("{:?}", addr);
    let mut tx = state.sql_pool.begin().await.map_err(|x| x.to_string())?;
    let config_guard = state.config.read().await;
    let config = config_guard.clone();
    let urls = config.fsnebula;
    let cache = urls.cache.clone();

    let start_time = std::time::Instant::now();
    let neb = FSNebula::init(urls, cache)
        .await
        .map_err(|x| x.to_string())?;
    let get_time = start_time.elapsed().as_millis();
    let rep = neb.repo;

    // Select most recent mod already in DB:
    let existing_releases = crate::db::queries::get_releases(&mut tx)
        .await
        .map_err(|x| x.to_string())?;

    let new_mods = rep
        .mods
        .into_iter()
        .filter(|m| {
            !existing_releases.contains(&crate::db::Rel {
                name: m.id.clone(),
                version: m.version.clone(),
            })
        })
        .collect::<Vec<FSNMod>>();
    db::commit_mods(tx, new_mods)
        .await
        .map_err(|x| x.to_string())?;
    let commit_time = start_time.elapsed().as_millis() - get_time;
    Ok(Json(UpdateInfo {
        status: "updated".to_string(),
        get_time,
        commit_time,
    }))
}
