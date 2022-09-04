use super::{mods::FSNMod, FSNPaths, FSNebula};
use axum::{
    self,
    body::{boxed, Empty, Full},
    extract::{Extension, Path},
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use itertools::Itertools;
use serde_json;
use std::path::PathBuf;
use std::sync::Arc;
use tower::{BoxError, ServiceBuilder};
use tower_http::{
    add_extension::AddExtensionLayer, auth::RequireAuthorizationLayer,
    compression::CompressionLayer, trace::TraceLayer,
};

pub async fn router(urls: FSNPaths, appdir: PathBuf) -> Result<Router, Box<dyn std::error::Error>> {
    let fsnebula: Arc<FSNebula> = Arc::new(FSNebula::init(urls, appdir).await?);

    let app = Router::new()
        .route("/mods", get(modlist))
        .route("/mod/:id", get(modinfo))
        .layer(
            ServiceBuilder::new()
                .layer(Extension(fsnebula))
                .into_inner(),
        );
    Ok(app)
}

type ShortMod = (String, String, String, Option<String>);

impl From<FSNMod> for ShortMod {
    fn from(m: FSNMod) -> Self {
        (m.id, m.title, m.version, m.logo)
    }
}
async fn modlist(Extension(fsnebula): Extension<Arc<FSNebula>>) -> String {
    let mods: Vec<ShortMod> = fsnebula
        .repo
        .mods
        .iter()
        .cloned()
        .map(|m: FSNMod| ShortMod::from(m))
        .collect::<Vec<ShortMod>>();
    serde_json::to_string(&mods).unwrap()
}

async fn modinfo(Path(id): Path<String>, Extension(fsnebula): Extension<Arc<FSNebula>>) -> String {
    let mods: Option<FSNMod> = fsnebula
        .repo
        .mods
        .iter()
        .filter(|m| m.id == id)
        .sorted_by(|a, b| Ord::cmp(&a.last_update, &b.last_update))
        .last()
        .cloned();
    serde_json::to_string(&mods).unwrap()
}
