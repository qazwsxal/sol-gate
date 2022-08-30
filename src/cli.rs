use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
pub struct Cli {
    /// Sets a custom config file
    #[clap(short, long, value_parser, value_name = "FILE")]
    pub config: Option<String>,
    /// Turn debugging information on
    #[clap(short, long, action = clap::ArgAction::Count)]
    pub debug: u8,

    #[clap(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    Launch {
        #[clap(value_parser, value_name = "MOD NAME")]
        name: String,
        #[clap(short, long, value_name = "MOD VERSION")]
        version: Option<String>,
    },
    Fetch {
        #[clap(value_parser, value_name = "MOD NAME")]
        name: String,
        #[clap(short, long, value_name = "MOD VERSION")]
        version: Option<String>,
    },
}
