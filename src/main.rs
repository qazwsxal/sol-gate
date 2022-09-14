use axum::{self, Router};
use clap::Parser;
use open;
use std::path::PathBuf;
use std::process::exit;
use std::{io::ErrorKind, sync::Arc};

mod api;
mod cli;
mod common;
mod config;
mod fsnebula;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    console_subscriber::init();

    let args = cli::Cli::parse();
    let config_path: Option<PathBuf> = args.config.map(|v| PathBuf::from(v));
    let config_result = config::Config::read(config_path);
    let config: config::Config = match config_result {
        Ok(config) => config,
        Err(conf_err) => match conf_err {
            config::ReadError::ParseError(e) => {
                eprintln!("Error parsing config: \n{error}", error = e);
                exit(1)
            }
            config::ReadError::IOError(e) => match e.kind() {
                ErrorKind::NotFound => config::Config::default(),
                _ => panic!("{error:#?}", error = e),
            },
        },
    };
    let app: Router = api::make_api(config, config::default_dir()).await;

    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 4000)); // User configurable?

    let server = tokio::spawn(async move {
        axum::Server::bind(&addr)
            .serve(app.into_make_service())
            .await
    });
    open::that("http://127.0.0.1:4000/")?;
    let (_result,) = tokio::join!(server);
    Ok(())
}
