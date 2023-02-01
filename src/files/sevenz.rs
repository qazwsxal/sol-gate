use std::path::Path;

use sevenz_rust::decompress_file;

// TODO: Figure out how to extract a single file from the archive.
// Try to use spawn_blocking for this.
pub async fn sevenz_extract(
    file_path: impl AsRef<Path>,
    temp_dir: impl AsRef<Path>,
) -> Result<(), sevenz_rust::Error> {
    // Unfortunately due to the library implementation, this relies on blocking read operations.
    let fp = file_path.as_ref().to_path_buf();
    let td = temp_dir.as_ref().to_path_buf();
    let decompress = tokio::task::spawn_blocking(|| decompress_file(fp, td));
    decompress.await.expect("join failed")
}
