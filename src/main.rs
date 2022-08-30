use clap::Parser;
use config::Config;
use fsnebula::mods::Repo;
use itertools::Itertools;
use std::io::{self, ErrorKind};
use std::path::PathBuf;
use std::process::exit;

mod cli;
mod common;
mod config;
mod db;
mod fsnebula;

#[tokio::main]
async fn main() {
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

    let fsn = fsnebula::FSNebula::init(config.fsnebula, config::default_dir())
        .await
        .unwrap();
    let repo = fsn.repo;
    let files: Vec<fsnebula::mods::FSNModFile> = repo
        .mods
        .iter()
        .flat_map(|f: &fsnebula::mods::FSNMod| &(f.packages))
        .flat_map(|f: &fsnebula::mods::FSNPackage| f.filelist.clone())
        .collect::<Vec<fsnebula::mods::FSNModFile>>();
    let checksums: Vec<fsnebula::mods::FSNChecksum> = files
        .iter()
        .map(|f| f.checksum.clone())
        .unique()
        .collect::<Vec<fsnebula::mods::FSNChecksum>>();
    println!(
        "{num_files} files present\n{num_unique} unique.",
        num_files = files.len(),
        num_unique = checksums.len()
    );
}
