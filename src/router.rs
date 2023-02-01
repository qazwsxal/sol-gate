use std::error::Error;
use std::path::PathBuf;

use axum::{
    self,
    extract::{FromRef, Path, State},
    routing::get,
    Json, Router,
};

use sqlx::SqlitePool;

use crate::SolGateState;

// grab pool out of global state.
impl FromRef<SolGateState> for SqlitePool {
    fn from_ref(sg_state: &SolGateState) -> SqlitePool {
        sg_state.sql_pool.clone()
    }
}

pub async fn router(appdir: PathBuf) -> Result<Router<SolGateState>, Box<dyn Error>> {
    let app = Router::new()
        .route("/list", get(mod_list))
        .route("/info/:id", get(mod_info));

    Ok(app)
}

#[derive(serde::Serialize)]
struct SimpleMod {
    id: String,
    title: String,
    version: String,
    tile: Option<String>,
}

// async fn mod_list_wrapper(State(fsn_pool): State<SqlitePool>) -> (StatusCode, String) {
//     internal_error_dyn(mod_list(fsn_pool).await)
// }

async fn mod_list(State(pool): State<SqlitePool>) -> Result<Json<Vec<SimpleMod>>, String> {
    let mut tx = pool.begin().await.map_err(|x| x.to_string())?;
    let mods: Vec<SimpleMod> = sqlx::query_as!(SimpleMod, "SELECT releases.name as id, coalesce(max(releases.`version`),'0.1.0') as version, mods.`title`, mods.tile FROM releases INNER JOIN mods on releases.rel_id=mods.rel_id GROUP BY releases.name;")
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
