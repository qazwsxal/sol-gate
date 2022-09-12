use std::{error::Error};
use std::fmt;
use std::path::PathBuf;


use axum::{
    self,
    extract::{Extension, Path},
    http::StatusCode,
    routing::get,
    Router,
    Json,

};

use reqwest::header::{ETAG, IF_NONE_MATCH};
use serde_json;
use sqlx::{
    SqlitePool,
};
use tokio::task::JoinError;
use tower::ServiceBuilder;

use super::{
    db,
    structs::{FSNChecksum, FSNMod, Repo},
    FSNPaths,
};

pub async fn router(urls: FSNPaths, appdir: PathBuf) -> Result<Router, Box<dyn Error>> {
    let cache = appdir.join("fsnebula");
    tokio::fs::create_dir_all(&cache).await?;
    let fsn_pool = db::init(cache.join("mods.db")).await?;

    let app = Router::new()
        .route("/mods/list", get(mod_list))
        .route("/mods/update", get(mod_update))
        .route("/mod/:id", get(mod_info))
        .layer(
            ServiceBuilder::new()
                .layer(Extension(fsn_pool))
                .layer(Extension(urls.clone()))
                .layer(Extension(cache.clone()))
                .into_inner(),
        );
    Ok(app)
}

#[derive(serde::Serialize)]
struct SimpleMod {
    id: String,
    title: String,
    version: String,
    logo: Option<String>,
}

impl From<FSNMod> for SimpleMod {
    fn from(m: FSNMod) -> Self {
        Self {
            id: m.id,
            title: m.title,
            version: m.version,
            logo: m.logo,
        }
    }
}

// async fn mod_list_wrapper(Extension(fsn_pool): Extension<SqlitePool>) -> (StatusCode, String) {
//     internal_error_dyn(mod_list(fsn_pool).await)
// }

async fn mod_list(Extension(fsn_pool): Extension<SqlitePool>) -> Result<Json<Vec<SimpleMod>>, String> {
    let mut tx = fsn_pool.begin().await.map_err(|x| x.to_string())?;
    let mods: Vec<SimpleMod> = sqlx::query_as!(SimpleMod, "select id, title, coalesce(max(`version`),'0.1.0') as version, logo from mods group by id;")
        .fetch_all(&mut tx)
        .await.map_err(|x| x.to_string())?;
    Ok(Json(mods))
}

async fn mod_info(Path(id): Path<String>, Extension(fsn_pool): Extension<SqlitePool>) -> Result<Json<Vec<SimpleMod>>, String> {
    let mut tx = fsn_pool.begin().await.map_err(|x| x.to_string())?;
    let mods = sqlx::query_as!(
        SimpleMod,
        "SELECT id, title, `version`, logo FROM mods \
        where mods.id = ?1",
        id
    )
    .fetch_all(&mut tx)
    .await.map_err(|x| x.to_string())?;
    Ok(Json(mods))
}

#[derive(PartialEq)]
enum UpdateStatus {
    Changed(Repo),
    Unchanged(),
}
#[derive(Debug)]
enum UpdateError {
    NotFound(),
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

async fn mod_update(
    Extension(fsn_pool): Extension<SqlitePool>,
    Extension(urls): Extension<FSNPaths>,
    Extension(cache): Extension<PathBuf>,
) -> Result<String, String> {
    let repo = get_fsnmods(cache, urls).await.map_err(|x| x.to_string())?;
    // Early exit, we don't have to do anything.
    if let UpdateStatus::Unchanged() = repo {
        return Ok("unchanged".to_string());
    }
    if let UpdateStatus::Changed(rep) = repo {
        // Select most recent mod already in DB:
        let mut tx = fsn_pool.begin().await.map_err(|x| x.to_string())?;
        let newest_mod  = sqlx::query!(
            "SELECT coalesce(max(last_update),'0000-00-00') as val from mods;")
            .fetch_optional(&mut tx)
            .await
            .map_err(|x| x.to_string())?
            .map_or(
                "0000-00-00".to_string(), // Handle no row returned
                //sqlx thinks the row can be null, so handle that option too
                |x| x.val.unwrap_or("0000-00-00".to_string())
            ); 
            tx.commit().await.map_err(|x| x.to_string())?;

        let new_mods = rep.mods.into_iter().filter(|m| m.last_update >= newest_mod);
        
        for fsnmod in new_mods {
            db::commit_mod(&fsn_pool, fsnmod).await.map_err(|x| x.to_string())?;
        }
    }
    Ok("updated".to_string())
}



async fn get_fsnmods(cache: PathBuf, urls: FSNPaths) -> Result<UpdateStatus, Box<dyn Error>> {
    tokio::fs::create_dir_all(&cache).await?;
    let client = reqwest::Client::new();
    let etag_path = cache.join("mods.json.etag");
    let etag: String = tokio::fs::read_to_string(&etag_path)
        .await
        .unwrap_or_else(|_| String::default());
    let mut status: Option<UpdateStatus> = None;
    for repo_url in urls.repos.iter() {
        let req_result = client
            .get(repo_url)
            .header(IF_NONE_MATCH, etag.clone())
            .send()
            .await;
        match req_result {
            Err(_) => continue, // Try next repo.json url
            Ok(response) => match response.status() {
                // No changes, so no need to update
                StatusCode::NOT_MODIFIED => {
                    status = Some(UpdateStatus::Unchanged());
                    break;
                }
                // mods.json has changed, so update etag and parse new modlist file.
                StatusCode::OK => {
                    match response.headers().get(ETAG) {
                        Some(tag) => tokio::fs::write(&etag_path, tag.to_str().unwrap()).await?,
                        None => (),
                    }
                    let resp = response.text().await?;
                    tokio::fs::write(&cache.join("mods.json"), &resp).await?;
                    status = Some(UpdateStatus::Changed(serde_json::from_str(&resp)?));
                    break;
                }
                _ => continue,
            },
        }
    }
    status.ok_or(Box::new(UpdateError::NotFound()))
}
