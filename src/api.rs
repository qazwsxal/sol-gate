use std::{
    ops::{Deref, DerefMut},
    path::PathBuf,
    sync::Arc,
};

use axum::{
    body::{boxed, Empty, Full},
    extract::State,
    http::{header, HeaderValue, StatusCode, Uri},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use tokio::sync::RwLock;

use crate::{config, db, fsnebula, router, files};
use include_dir::{include_dir, Dir};

static FRONTEND_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/frontend/build");

struct Readers {
    raw_local: async_channel::Sender<files::PathOneshot>,
    
}





pub(crate) async fn make_api(config: config::Config, appdir: PathBuf) -> Router {
    // let (tx, rx) = tokio::sync::oneshot::channel::<()>();
    // let txsend = Arc::new(async move {|| tx.send(());});
    
    let sql_pool = db::init(appdir.clone().join("mods.db"))
        .await
        .expect("Could not init sql connection");
    let (fsn_router, fsn_state) =
        fsnebula::api::router(config.fsnebula.clone(), appdir.clone(), sql_pool.clone())
            .await
            .unwrap();

    let arc_config = Arc::new(RwLock::new(config.clone()));
    let sql_router = router::router(appdir.clone()).await.unwrap();
    let api_router = Router::new()
        .nest("/fsn", fsn_router.with_state(fsn_state))
        .nest("/mods", sql_router.with_state(sql_pool))
        .route("/config", get(config::api::get_config).put(config::api::put_config));

    //TODO: Actually add API endpoints
    Router::new()
        .fallback(frontend)
        .nest("/api", api_router.with_state(arc_config))
        .with_state(()) //Keep state clean, and fallback doesn't need it.
                        //        .route("/shutdown", get(txsend))
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