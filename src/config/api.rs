use std::{
    mem::replace,
    ops::{Deref, DerefMut},
    sync::Arc,
};

use axum::{
    extract::{FromRef, State},
    response::IntoResponse,
    Json,
};
use tokio::sync::RwLock;

use crate::SolGateState;

use super::Config;

// support converting an `Arc<RwLock<config::Config>>` from a `SolGateState`
impl FromRef<SolGateState> for Arc<RwLock<Config>> {
    fn from_ref(sg_state: &SolGateState) -> Arc<RwLock<Config>> {
        sg_state.config.clone()
    }
}

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
