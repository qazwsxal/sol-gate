use std::{
    mem::replace,
    ops::{Deref, DerefMut},
    sync::Arc,
};

use axum::{
    extract::State,
    response::{IntoResponse, },
    Json,
};
use tokio::sync::RwLock;


use super::Config;

pub(crate) async fn get_config(State(config): State<Arc<RwLock<Config>>>) -> Json<Config> {
    let x = config.read().await.deref().clone();
    Json(x)
}

pub(crate) async fn put_config(
    State(config): State<Arc<RwLock<Config>>>,
    Json(new_config): Json<Config>,
) -> impl IntoResponse {
    let mut x = config.write().await;
    let _old = replace(x.deref_mut(), new_config);
    (*x).save().map_err(|x| x.to_string())
}
