use std::collections::HashSet;
use std::fmt;
use std::net::SocketAddr;
use std::path::PathBuf;
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

use super::{db, structs::Repo, FSNPaths, FSNebula};
use crate::fsnebula::structs::FSNMod;
#[derive(Clone, FromRef)]
pub struct FSNState {
    pool: SqlitePool,
    urls: FSNPaths,
    cache: PathBuf,
}

pub async fn router(
    urls: FSNPaths,
    appdir: PathBuf,
    pool: SqlitePool,
) -> Result<(Router<FSNState>, FSNState), Box<dyn Error>> {
    let cache = appdir.join("fsnebula");
    tokio::fs::create_dir_all(&cache).await?;
    let fsn_state = FSNState {
        pool,
        urls: urls.clone(),
        cache: cache.clone(),
    };

    let app = Router::new().route("/update", get(mod_update));

    Ok((app, fsn_state))
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
    State(fsn_state): State<FSNState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> Result<Json<UpdateInfo>, String> {
    println!("{:?}", addr);
    let pool = fsn_state.pool;
    let urls = fsn_state.urls;
    let cache = fsn_state.cache;

    let start_time = std::time::Instant::now();
    let neb = FSNebula::init(urls, cache)
        .await
        .map_err(|x| x.to_string())?;
    let get_time = start_time.elapsed().as_millis();
    let rep = neb.repo;

    #[derive(Hash, PartialEq, Eq)]
    struct Rel {
        name: String,
        version: String,
    }
    // Select most recent mod already in DB:
    let mut tx = pool.begin().await.map_err(|x| x.to_string())?;
    let existing_releases = sqlx::query_as!(Rel, "SELECT `name`, `version` from releases;")
        .fetch_all(&mut tx)
        .await
        .map_err(|x| x.to_string())?;
    tx.commit().await.map_err(|x| x.to_string())?;

    let mut em_map = HashSet::<Rel>::new();
    for rel in existing_releases {
        em_map.insert(rel);
    }

    let new_mods = rep
        .mods
        .into_iter()
        .filter(|m| {
            !em_map.contains(&Rel {
                name: m.id.clone(),
                version: m.version.clone(),
            })
        })
        .collect::<Vec<FSNMod>>();
    db::commit_mods(&pool, new_mods)
        .await
        .map_err(|x| x.to_string())?;
    let commit_time = start_time.elapsed().as_millis() - get_time;
    Ok(Json(UpdateInfo {
        status: "updated".to_string(),
        get_time,
        commit_time,
    }))
}
