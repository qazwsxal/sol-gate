use axum::{self};
use clap::Parser;
use config::Config;
use files::readers::ReaderPool;
use open;
use reqwest::Client;
use std::collections::HashMap;
use std::io::ErrorKind;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::exit;
use std::sync::Arc;
use tokio::signal;
use tokio::sync::RwLock;
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;

mod api;
mod cli;
mod common;
mod config;
mod db;
mod files;
mod fsnebula;
mod router;

#[derive(Debug, Clone)]
pub struct ReaderEntry {
    pub send: mpsc::Sender<(String, oneshot::Sender<Vec<u8>>)>,
    pub handle: Arc<JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>>>,
}

#[derive(Debug, Clone)]
pub struct SolGateState {
    pub sql_pool: sqlx::sqlite::SqlitePool,
    pub config: Arc<RwLock<config::Config>>,
    pub reader_pool: Arc<RwLock<ReaderPool>>,
    pub http_client: Client,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    console_subscriber::init();
    let args = cli::Cli::parse();
    let config_path: Option<PathBuf> = args.config.map(|v| PathBuf::from(v));
    let config_result = config::Config::read(config_path);
    let config: config::Config = match config_result {
        Ok(config) => config,
        Err(conf_err) => match conf_err {
            config::ConfigReadError::ParseError(e) => {
                eprintln!("Error parsing config: \n{error}", error = e);
                exit(1)
            }
            config::ConfigReadError::IOError(e) => match e.kind() {
                ErrorKind::NotFound => config::Config::default(),
                _ => panic!("{error:#?}", error = e),
            },
        },
    };
    let appdir = Config::default_dir();

    let mut sol_state = init_state(config).await.unwrap();

    let app = api::make_api(sol_state).await;

    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 4000)); // User configurable?

    let server = tokio::spawn(async move {
        axum::Server::bind(&addr)
            .serve(app.into_make_service_with_connect_info::<SocketAddr>())
            .with_graceful_shutdown(shutdown_signal())
            .await
    });
    // open::that("http://127.0.0.1:4000/")?;
    open::that("http://127.0.0.1:4000/api/fsn/update")?; // Testing FSN update mechanism
    let (_result,) = tokio::join!(server);
    Ok(())
}

async fn init_state(config: Config) -> Result<SolGateState, Box<dyn std::error::Error>> {
    let appdir = Config::default_dir();

    let sql_pool = db::init(appdir.clone().join("mods.db"))
        .await
        .expect("Could not init sql connection");

    let rwl_config = Arc::new(RwLock::new(config));

    let reader_pool = Arc::new(RwLock::new(ReaderPool::new()));
    let http_client = Client::new();
    Ok(SolGateState {
        sql_pool,
        config: rwl_config,
        reader_pool,
        http_client,
    })
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    println!("signal received, starting graceful shutdown");
}
