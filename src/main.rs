use axum::{
    self,
    body::{boxed, Empty, Full},
    extract::Path,
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
};

use clap::Parser;

use include_dir::{include_dir, Dir};
use itertools::Itertools;
use open;
use std::io::{self, ErrorKind};
use std::path::PathBuf;
use std::process::exit;

mod cli;
mod common;
mod config;
mod fsnebula;
static FRONTEND_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/frontend/build");

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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
                ErrorKind::NotFound => config::setup(),
                _ => panic!("{error:#?}", error = e),
            },
        },
    };
    let api_router = axum::Router::new().nest(
        "/fsn",
        fsnebula::router::router(config.fsnebula, config::default_dir()).await?,
    );
    //TODO: Actually add API endpoints
    let app = axum::Router::new()
        .route("/ui/*path", get(static_path))
        .nest("/api", api_router);

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], 3000));

    let server = tokio::spawn(async move {
        axum::Server::bind(&addr)
            .serve(app.into_make_service())
            .await
    });
    open::that("http://127.0.0.1:3000/ui/")?;
    let (_result,) = tokio::join!(server);
    Ok(())
}

async fn static_path(Path(path): Path<String>) -> impl IntoResponse {
    let path = match path.trim_start_matches('/') {
        "" => "index.html",
        x => x
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
//     let fsn = fsnebula::FSNebula::init(config.fsnebula, config::default_dir())
//         .await
//         .unwrap();
//     let repo = fsn.repo;
//     let files: Vec<fsnebula::mods::FSNModFile> = repo
//         .mods
//         .iter()
//         .flat_map(|f: &fsnebula::mods::FSNMod| &(f.packages))
//         .flat_map(|f: &fsnebula::mods::FSNPackage| f.filelist.clone())
//         .collect::<Vec<fsnebula::mods::FSNModFile>>();
//     let checksums: Vec<fsnebula::mods::FSNChecksum> = files
//         .iter()
//         .map(|f| f.checksum.clone())
//         .unique()
//         .collect::<Vec<fsnebula::mods::FSNChecksum>>();
//     println!(
//         "{num_files} files present\n{num_unique} unique.",
//         num_files = files.len(),
//         num_unique = checksums.len()
//     );
// }
