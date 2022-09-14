use std::{
    mem::replace,
    ops::{Deref, DerefMut},
    path::PathBuf,
    sync::Arc,
};

use axum::{
    self,
    body::{boxed, Body, Empty, Full},
    http::{header, HeaderValue, StatusCode, Uri},
    response::{IntoResponse, Response},
    routing::get,
    Extension, Json, Router,
};
use tokio::sync::RwLock;
use tower::ServiceBuilder;

use crate::{
    config::{self, Config},
    fsnebula,
};
use include_dir::{include_dir, Dir};

static FRONTEND_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/frontend/build");

pub(crate) async fn make_api(config: Config, appdir: PathBuf) -> Router {
    // let (tx, rx) = tokio::sync::oneshot::channel::<()>();
    // let txsend = Arc::new(async move {|| tx.send(());});

    let fsn_router = fsnebula::router::router(config.fsnebula.clone(), appdir)
        .await
        .unwrap();
    let mut config = config.clone();
    let api_router = axum::Router::new()
        .nest("/fsn", fsn_router)
        .route("/config", get(get_config).put(put_config))
        .layer(
            ServiceBuilder::new()
                .layer(Extension(Arc::new(RwLock::new(config))))
                .into_inner(),
        );
    //TODO: Actually add API endpoints
    axum::Router::new()
        .fallback(get(frontend))
        .nest("/api", api_router)
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

async fn get_config(Extension(config): Extension<Arc<RwLock<Config>>>) -> Json<Config> {
    let x = config.read().await.deref().clone();
    Json(x)
}

async fn put_config(
    Json(new_config): Json<Config>,
    Extension(config): Extension<Arc<RwLock<Config>>>,
) -> impl IntoResponse {
    let mut x = config.write().await;
    replace(x.deref_mut(), new_config);
    (*x).save().map_err(|x| x.to_string())
}
