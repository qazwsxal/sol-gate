use std::{
    ops::{Deref, DerefMut},
    path::PathBuf,
    sync::Arc,
};

use axum::{
    body::{boxed, Empty, Full},
    extract::{FromRef, State},
    http::{header, HeaderValue, StatusCode, Uri},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use tokio::sync::RwLock;

use crate::{
    config::{self, Config, FSNPaths},
    db, files, fsnebula, router, SolGateState,
};
use include_dir::{include_dir, Dir};

static FRONTEND_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/frontend/build");

pub(crate) async fn make_api(sol_state: SolGateState) -> Router {
    let appdir = Config::default_dir();
    // let (tx, rx) = tokio::sync::oneshot::channel::<()>();
    // let txsend = Arc::new(async move {|| tx.send(());});

    let fsn_router = fsnebula::api::router(&appdir).await.unwrap();
    let mods_router = router::router(appdir.clone()).await.unwrap();
    let files_router = files::api::router(sol_state.config.read().await.clone())
        .await
        .unwrap();

    let api_router = Router::new()
        .nest("/fsn", fsn_router)
        .nest("/mods", mods_router)
        .nest("/files", files_router)
        .route(
            "/config",
            get(config::api::get_config).put(config::api::put_config),
        );

    //TODO: Actually add API endpoints
    Router::new()
        .fallback(frontend)
        .nest("/api", api_router)
        .with_state(sol_state)
}

async fn frontend(uri: Uri) -> impl IntoResponse {
    // Ugly "no path means index.html" hack
    let path = match uri.path().trim_start_matches("/") {
        "" => "index.html",
        x => x,
    };

    let mime_type = mime_guess::from_path(path).first_or_text_plain();
    let file = FRONTEND_DIR.get_file(path);
    match file {
        None => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(boxed(Empty::new()))
            .unwrap(),
        Some(file) => Response::builder()
            .status(StatusCode::OK)
            .header(
                header::CONTENT_TYPE,
                HeaderValue::from_str(mime_type.as_ref()).unwrap(),
            )
            .body(boxed(Full::from(file.contents())))
            .unwrap(),
    }
}
