use vp;

use async_channel;

use tokio::sync::oneshot;
use tokio::task::JoinSet;

type VecPlusOneshot = (Vec<u8>, oneshot::Sender<Vec<u8>>);

pub async fn spawn_vp_decompressors(
    queue_len: usize,
    dc_count: Option<usize>,
) -> (async_channel::Sender<VecPlusOneshot>, JoinSet<()>) {
    let (tx_uc, rx_uc) = async_channel::bounded::<VecPlusOneshot>(queue_len);
    let threads = dc_count.unwrap_or_else(num_cpus::get);
    let mut decompress_tasks = JoinSet::new();
    for _ in 0..threads {
        let rx = rx_uc.clone();
        decompress_tasks.spawn(async move {
            // decompression cannot fail, it can only be failed
            while let Ok((entry, os_tx)) = rx.recv().await {
                // Recieve a vector to be decompressed, and the oneshot for it too.
                let contents = vp::compression::maybe_decompress(entry);
                os_tx.send(contents).unwrap() // Yeah, again, don't really know what to do if the oneshot sender fails here.
            }
        });
    }
    (tx_uc, decompress_tasks)
}

pub async fn spawn_vp_compressors(
    queue_len: usize,
    dc_count: Option<usize>,
) -> (async_channel::Sender<VecPlusOneshot>, JoinSet<()>) {
    let (tx_uc, rx_uc) = async_channel::bounded::<VecPlusOneshot>(queue_len);
    let threads = dc_count.unwrap_or_else(num_cpus::get);
    let mut compress_tasks = JoinSet::new();
    for _ in 0..threads {
        let rx = rx_uc.clone();
        compress_tasks.spawn(async move {
            // decompression cannot fail, it can only be failed
            while let Ok((entry, os_tx)) = rx.recv().await {
                // Recieve a vector to be decompressed, and the oneshot for it too.
                let contents = vp::compression::maybe_decompress(entry);
                os_tx.send(contents).unwrap() // Yeah, again, don't really know what to do if the oneshot sender fails here.
            }
        });
    }
    (tx_uc, compress_tasks)
}
