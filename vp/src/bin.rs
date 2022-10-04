use std::{io::SeekFrom, path::{PathBuf, Path}};

use async_channel;
use clap::{Parser, Subcommand};
use num_cpus;
use tokio::{
    self,
    fs::File,
    io::{AsyncReadExt, AsyncSeekExt},
    sync::Mutex,
    task::{JoinHandle, JoinSet},
};
use console_subscriber;

use vp::{compression::maybe_decompress, fs};

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    #[clap(subcommand)]
    mode: Mode,
}

#[derive(Parser, Debug)]
struct DCopts {
    #[clap(value_parser)]
    input_vp: PathBuf,
    #[clap(value_parser, default_value = ".")]
    output_dir: PathBuf,
}

#[derive(Parser, Debug)]
struct Copts {
    #[clap(value_parser)]
    input_dir: PathBuf,
    #[clap(value_parser)]
    output_vp: PathBuf,
    #[clap(short, default_value_t = false)]
    z: bool,
}

#[derive(Subcommand, Debug)]
enum Mode {
    Decompress(DCopts),
    Compress(Copts),
}

#[derive(Debug)]
struct FileContents {
    path: PathBuf,
    contents: Vec<u8>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    console_subscriber::init();
    let cli = Cli::parse();
    match cli.mode {
        Mode::Decompress(opts) => decompress(opts).await?,
        Mode::Compress(opts) => compress(opts).await?,
    }
    Ok(())
}

async fn decompress(opts: DCopts) -> Result<(), Box<dyn std::error::Error>> {
    let index = fs::index(&mut std::fs::File::open(&opts.input_vp)?)?;
    let files = index.flatten();
    let (mut tx_vp, mut rx_vp) = async_channel::bounded::<FileContents>(4);
    let vp_task: JoinHandle<tokio::io::Result<()>> = tokio::spawn(async move {
        let mut vp = File::open(opts.input_vp).await?;
        for vpfile in files {
            let mut buf = vec![0u8; vpfile.size.try_into().unwrap()];
            vp.seek(SeekFrom::Start(vpfile.fileoffset)).await?;
            vp.read(&mut buf).await?;
            tx_vp.send(FileContents {
                path: vpfile.name.into(),
                contents: buf,
            }).await;
        }
        Ok(())
    });
    let mut decompress_tasks= JoinSet::new();
    let mut save_tasks = JoinSet::new();

    let (tx_uc, rx_uc) = async_channel::bounded::<FileContents>(4);    
    for _ in 0..num_cpus::get() {
            let rx = rx_vp.clone();
            let tx = tx_uc.clone();
            decompress_tasks.spawn(async move {
                while let Ok(entry) = rx.recv().await {
                    let path = entry.path;
                    let contents = maybe_decompress(entry.contents);
                    tx.send(FileContents { path, contents }).await;
                }
            });
        }
        drop(tx_uc);

        while let Ok(entry) = rx_uc.recv().await {
                    save_tasks.spawn(async move {
                    let dir: PathBuf = entry.path.parent().unwrap_or(&Path::new(".")).to_path_buf();
                    dbg!(&entry.path);
                    tokio::fs::DirBuilder::new().recursive(true).create(dir).await.unwrap();
                    tokio::fs::write(entry.path, entry.contents).await.unwrap();
                }
            );
        }
    
    while let Some(task) = save_tasks.join_next().await {
    }    
    Ok(())
}

async fn compress(opts: Copts) -> Result<(), Box<dyn std::error::Error>> {
    todo!()
}
