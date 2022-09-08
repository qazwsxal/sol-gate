use std::error::Error;
use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;

use axum::{
    self,
    extract::{Extension, Path},
    http::StatusCode,
    routing::get,
    Router,
};

use itertools::Itertools;
use reqwest::header::{ETAG, IF_NONE_MATCH};
use serde_json;
use sqlx::{
    migrate::{MigrateError, Migrator},
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
    SqlitePool, Transaction,
};
use tokio::task::JoinError;
use tower::ServiceBuilder;

use super::{
    db,
    structs::{FSNChecksum, FSNMod, Repo},
    FSNPaths, FSNebula,
};
use crate::common::{
    router::{internal_error, internal_error_dyn},
    ModType, Stability,
};

pub async fn router(urls: FSNPaths, appdir: PathBuf) -> Result<Router, Box<dyn Error>> {
    let cache = appdir.join("fsnebula");
    tokio::fs::create_dir_all(&cache).await?;
    let fsn_pool = db::init(cache.join("mods.db")).await?;

    let app = Router::new()
        .route("/mods/list", get(mod_list_wrapper))
        .route("/mods/update", get(mod_update_wrapper))
        .route("/mod/:id", get(mod_info_wrapper))
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

async fn mod_list_wrapper(
    Extension(fsn_pool): Extension<SqlitePool>,
) -> Result<String, (StatusCode, String)> {
    match mod_list(fsn_pool).await {
        Ok(str) => Ok(str),
        Err(e) => Err(internal_error_dyn(e)),
    }
}

async fn mod_list(fsn_pool: SqlitePool) -> Result<String, Box<dyn Error>> {
    let mut tx = fsn_pool.begin().await?;
    let mods = sqlx::query_as!(SimpleMod, "SELECT id, title, `version`, logo FROM mods")
        .fetch_all(&mut tx)
        .await?;
    serde_json::to_string(&mods).map_err(|e| e.into())
}

async fn mod_info_wrapper(
    Path(id): Path<String>,
    Extension(fsn_pool): Extension<SqlitePool>,
) -> Result<String, (StatusCode, String)> {
    match mod_info(id, fsn_pool).await {
        Ok(str) => Ok(str),
        Err(e) => Err(internal_error_dyn(e)),
    }
}

async fn mod_info(id: String, fsn_pool: SqlitePool) -> Result<String, Box<dyn Error>> {
    let mut tx = fsn_pool.begin().await?;
    let mods = sqlx::query_as!(
        SimpleMod,
        "SELECT id, title, `version`, logo FROM mods \
        where mods.id = ?1",
        id
    )
    .fetch_all(&mut tx)
    .await?;
    serde_json::to_string(&mods).map_err(|e| e.into())
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

async fn mod_update_wrapper(
    Extension(fsn_pool): Extension<SqlitePool>,
    Extension(urls): Extension<FSNPaths>,
    Extension(cache): Extension<PathBuf>,
) -> Result<String, (StatusCode, String)> {
    match mod_update(fsn_pool, urls, cache).await {
        Ok(str) => Ok(str),
        Err(e) => Err(internal_error_dyn(e)),
    }
}

async fn mod_update(
    fsn_pool: SqlitePool,
    urls: FSNPaths,
    cache: PathBuf,
) -> Result<String, Box<dyn Error>> {
    let repo = get_fsnmods(cache, urls).await?;
    // Early exit, we don't have to do anything.
    if let UpdateStatus::Unchanged() = repo {
        return Ok("unchanged".to_string());
    }
    if let UpdateStatus::Changed(rep) = repo {
        for fsnmod in rep.mods {
            let mut tx = fsn_pool.begin().await?;
            let exists = sqlx::query!(
                "SELECT mods.id, mods.version FROM mods WHERE (mods.id, mods.version) = (?1, ?2);",
                fsnmod.id,
                fsnmod.version,
            )
            .fetch_optional(&mut tx)
            .await?;

            // Skip if we already have that mod/version combo in the database.
            if exists.is_some() {
                continue;
            }

            db::update_mods(&fsnmod, &mut tx).await?;
            if let Some(stab) = &fsnmod.stability {
                sqlx::query!(
                    "INSERT OR IGNORE INTO mods_stab (`stab`, `id`, `version`)
                    VALUES (?1, ?2, ?3)",
                    stab,
                    fsnmod.id,
                    fsnmod.version
                )
                .execute(&mut tx)
                .await?;
            }
            for screen in fsnmod.screenshots.iter() {
                db::update_link(&fsnmod, "screenshot", screen, &mut tx).await?;
            }

            for attach in fsnmod.attachments.iter() {
                db::update_link(&fsnmod, "attachment", attach, &mut tx).await?;
            }

            if let Some(thread) = &fsnmod.release_thread {
                db::update_link(&fsnmod, "thread", thread, &mut tx).await?;
            }

            for vid in fsnmod.videos.iter() {
                db::update_link(&fsnmod, "videos", vid, &mut tx).await?;
            }

            for dep in fsnmod.mod_flag.iter() {
                db::update_mod_flags(&fsnmod, dep, &mut tx).await?;
            }

            for package in fsnmod.packages {
                let p_id = sqlx::query_file!(
                    "src/fsnebula/queries/update/packages.sql",
                    fsnmod.id,
                    fsnmod.version,
                    package.name,
                    package.notes,
                    package.status,
                    package.environment,
                    package.folder,
                    package.is_vp
                )
                .fetch_one(&mut tx)
                .await?
                .p_id;

                for zipfile in package.files {
                    sqlx::query_file!(
                        "src/fsnebula/queries/update/zipfiles.sql",
                        p_id,
                        zipfile.filename,
                        zipfile.dest,
                        zipfile.filesize,
                    )
                    .execute(&mut tx)
                    .await?;
                }
                for dep in package.dependencies {
                    let dep_id = sqlx::query!(
                        "INSERT INTO pak_dep (p_id, version, dep_mod_id)
                        VALUES (?1, ?2, ?3)
                        RETURNING id
                        ",
                        p_id,
                        dep.version,
                        dep.id
                    )
                    .fetch_one(&mut tx)
                    .await?
                    .id;

                    for dep_pak in dep.packages {
                        sqlx::query!(
                            "INSERT into dep_pak (dep_id, name) \
                            VALUES (?1, ?2);",
                            dep_id,
                            dep_pak
                        )
                        .execute(&mut tx)
                        .await?;
                    }
                }

                for modfile in package.filelist {
                    let FSNChecksum::SHA256(hash) = modfile.checksum.clone();

                    let zip_id: i64 = sqlx::query!(
                        "SELECT id FROM zipfiles WHERE (p_id, filename) == (?1, ?2)",
                        p_id,
                        modfile.archive
                    )
                    .fetch_one(&mut tx)
                    .await?
                    .id;

                    sqlx::query!(
                        "INSERT OR IGNORE INTO files (f_path, zip_id, h_val) \
                        VALUES (?1, ?2, ?3)",
                        modfile.filename,
                        zip_id,
                        hash,
                    )
                    .execute(&mut tx)
                    .await?;
                }
            }

            tx.commit().await?;
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
