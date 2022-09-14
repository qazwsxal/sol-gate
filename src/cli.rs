use clap::Parser;

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
pub struct Cli {
    /// Sets a custom config file
    #[clap(short, long, value_parser, value_name = "FILE")]
    pub config: Option<String>,
    /// Turn debugging information on
    #[clap(short, long, action = clap::ArgAction::Count)]
    pub debug: u8,

    #[clap(short, long)]
    pub ip: Option<String>,
    #[clap(short, long)]
    pub port: Option<u16>,
}
