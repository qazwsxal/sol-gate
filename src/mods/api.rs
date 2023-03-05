use std::error::Error;

use axum::{
    self,
    extract::{Form, FromRef, Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};

use sqlx::SqlitePool;

use crate::{
    common::queries::{get_mod_details, get_mod_packages, get_package_files},
    db::{queries, Package},
    files::install_files,
    SolGateState,
};

// grab pool out of global state.
impl FromRef<SolGateState> for SqlitePool {
    fn from_ref(sg_state: &SolGateState) -> SqlitePool {
        sg_state.sql_pool.clone()
    }
}

pub async fn router() -> Result<Router<SolGateState>, Box<dyn Error>> {
    let app = Router::new()
        .route("/avaliable", get(mod_list))
        .route("/installed", get(installed_list))
        .route("/info/:id", get(mod_info))
        .route("/install", post(install_mod));

    Ok(app)
}

#[derive(serde::Serialize, serde::Deserialize)]
struct SimpleMod {
    id: String,
    title: String,
    version: String,
    tile: Option<String>,
}

enum ModError {
    SqlxError(sqlx::Error),
    InstallError,
}

impl From<sqlx::Error> for ModError {
    fn from(err: sqlx::Error) -> Self {
        ModError::SqlxError(err)
    }
}

impl IntoResponse for ModError {
    fn into_response(self) -> axum::response::Response {
        let body = match self {
            ModError::InstallError => String::from("Installation Error"),
            ModError::SqlxError(sql_err) => sql_err.to_string(),
        };

        (StatusCode::INTERNAL_SERVER_ERROR, body).into_response()
    }
}

async fn mod_list(State(pool): State<SqlitePool>) -> Result<Json<Vec<SimpleMod>>, String> {
    let mut tx = pool.begin().await.map_err(|x| x.to_string())?;
    let mods: Vec<SimpleMod> = sqlx::query_as!(SimpleMod, "SELECT releases.name as id, coalesce(max(releases.`version`),'0.1.0') as version, mods.`title`, mods.tile FROM releases INNER JOIN mods on releases.rel_id=mods.rel_id GROUP BY releases.name;")
        .fetch_all(&mut tx)
        .await.map_err(|x| x.to_string())?;
    Ok(Json(mods))
}

async fn installed_list(State(pool): State<SqlitePool>) -> Result<Json<Vec<SimpleMod>>, String> {
    let mut tx = pool.begin().await.map_err(|x| x.to_string())?;
    let mods: Vec<SimpleMod> = sqlx::query_as!(SimpleMod, "SELECT releases.name as id, coalesce(max(releases.`version`),'0.1.0') as version, mods.`title`, mods.tile FROM releases INNER JOIN mods on releases.rel_id=mods.rel_id WHERE mods.installed = 1 GROUP BY releases.name;")
        .fetch_all(&mut tx)
        .await.map_err(|x| x.to_string())?;
    Ok(Json(mods))
}

async fn mod_info(
    Path(id): Path<String>,
    State(pool): State<SqlitePool>,
) -> Result<Json<Vec<SimpleMod>>, String> {
    let mut tx = pool.begin().await.map_err(|x| x.to_string())?;
    let mods = sqlx::query_as!(
        SimpleMod,
        "SELECT releases.name as \"id!\", coalesce(max(releases.`version`),'0.1.0') as \"version!\", mods.title as \"title!\", mods.tile \
        FROM releases \
        INNER JOIN mods on releases.rel_id=mods.rel_id \
        WHERE releases.name = ? \
        ",
        id
    )
    .fetch_all(&mut tx)
    .await
    .map_err(|x| x.to_string())?;
    Ok(Json(mods))
}

async fn install_mod(
    State(sol_state): State<SolGateState>,
    mod_info: Form<SimpleMod>,
) -> Result<(), ModError> {
    let mut tx = sol_state.sql_pool.begin().await?;
    let mod_details = get_mod_details(&mod_info.id, &mod_info.version, &mut tx)
        .await?
        .ok_or(ModError::InstallError)?;
    let packages = get_mod_packages(&mod_info.id, &mod_info.version, &mut tx).await?;
    let mut package_details: Vec<(Package, Vec<crate::db::File>)> = Vec::new();
    for package in packages.into_iter() {
        let files = get_package_files(&package.p_id, &mut tx).await?;
        package_details.push((package, files))
    }
    let parent_dir = mod_details.parent; // None means it's a TC, Some means it's a mod for a TC (or FS2)
    todo!()
}
