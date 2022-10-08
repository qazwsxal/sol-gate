use std::{
    error::Error,
    fmt::Display,
    io::SeekFrom,
    path::{Path, PathBuf},
};

use clap::{Parser, Subcommand};
use tokio::{
    self,
    fs::File,
    io::{AsyncReadExt, AsyncSeekExt, BufReader},
    task::{JoinHandle, JoinSet},
};

use async_channel;
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
    #[cfg(debug_assertions)]
    console_subscriber::init();

    let cli = Cli::parse();
    match cli.mode {
        Mode::Decompress(opts) => decompress(opts).await?,
        Mode::Compress(opts) => compress(opts).await?,
    }
    Ok(())
}
#[derive(Debug)]
pub enum VPReaderError<T: std::fmt::Debug> {
    IOError(tokio::io::Error),
    ChannelError(async_channel::SendError<T>),
}

impl<T: std::fmt::Debug> Display for VPReaderError<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl<T: std::fmt::Debug> Error for VPReaderError<T> {}

impl<T: std::fmt::Debug> From<tokio::io::Error> for VPReaderError<T> {
    fn from(e: tokio::io::Error) -> Self {
        VPReaderError::<T>::IOError(e)
    }
}

impl<T: std::fmt::Debug> From<async_channel::SendError<T>> for VPReaderError<T> {
    fn from(e: async_channel::SendError<T>) -> Self {
        VPReaderError::<T>::ChannelError(e)
    }
}

async fn decompress(opts: DCopts) -> Result<(), Box<dyn std::error::Error>> {
    let mut index = fs::index(&mut std::fs::File::open(&opts.input_vp)?)?;
    // Set index of root directory to extraction directory
    index.name = opts.output_dir.to_string_lossy().to_string();
    // index.flatten turns it into a Vec of VP files with full paths.
    let mut files = index.flatten();
    files.sort_by_key(|f| f.fileoffset); // Order by file offset so we're not seeking back and forth.
    let (tx_vp, rx_vp) = async_channel::bounded::<FileContents>(32);
    let vp_task: JoinHandle<Result<(), VPReaderError<FileContents>>> =
        tokio::task::spawn(async move {
            let mut vp = BufReader::new(File::open(opts.input_vp).await?);
            let mut currpos = 0;
            for vpfile in files {
                // files in a VP are *usually* contigious, but there's no actual garuantee.
                // We keep track of the current offset to prevent an unnecessary seek operation
                // if everything lines up.
                if currpos != vpfile.fileoffset {
                    // We'll hit this on first iteration as the header is 16 bytes long.
                    vp.seek(SeekFrom::Start(vpfile.fileoffset)).await?;
                    currpos = vpfile.fileoffset;
                }
                let mut buf = vec![0u8; vpfile.size.try_into().unwrap()];
                vp.read_exact(&mut buf).await?;
                currpos += vpfile.size as u64;
                tx_vp
                    .send(FileContents {
                        path: vpfile.name.into(),
                        contents: buf,
                    })
                    .await?;
            }
            Ok(())
        });
    let mut decompress_tasks = JoinSet::new();
    let mut save_tasks = JoinSet::new();

    let (tx_uc, rx_uc) = async_channel::bounded::<FileContents>(32);
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
            let dir: PathBuf = entry
                .path
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .to_path_buf();
            //dbg!(&entry.path);
            tokio::fs::DirBuilder::new()
                .recursive(true)
                .create(dir)
                .await
                .unwrap();
            tokio::fs::write(entry.path, entry.contents).await
        });
    }

    while let Some(task) = save_tasks.join_next().await {
        task??;
    }
    tokio::join!(vp_task).0??;
    Ok(())
}

async fn compress(opts: Copts) -> Result<(), Box<dyn std::error::Error>> {
    todo!()
}
