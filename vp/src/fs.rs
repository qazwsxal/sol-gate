use crate::parser;
use crate::types::{VPDir, VPEntry, VPFile};
use std::path::Path;
use std::{
    fs::File,
    io::{self, Read, Seek, SeekFrom},
    path::PathBuf,
};

pub fn index<T: Read + Seek>(handle: &mut T) -> io::Result<VPDir> {
    handle.seek(SeekFrom::Start(0))?;
    let mut headbuf = vec![0u8; 16];
    handle.read_exact(&mut headbuf)?;
    // Using unwrap here, we can't pass error upwards with ?
    // as headbuf would need to be passed up
    // and would outlive the local variable.
    // Should handle errors better, but it's a first try for now.
    let (_, head) = parser::header(&headbuf).unwrap();

    handle.seek(SeekFrom::Start(head.offset.into()))?;

    let mut indexbuf = vec![0u8; 0];
    handle.read_to_end(&mut indexbuf)?;

    // As before, .unwrap() because we can't pass indexbuf up.
    let (_, vp_index) = parser::indicies(&indexbuf).unwrap();
    Ok(VPDir::from(vp_index))
}

pub fn read_entry<P: Into<PathBuf>>(path: P) -> io::Result<VPFile> {
    let path: PathBuf = path.into();
    let (vp_filepath, mut folders) = split_path(&path)?;

    let mut vpcontents: Vec<VPEntry> = index(&mut File::open(&vp_filepath)?)?.contents;
    let file = folders
        .pop()
        .ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))?;
    for folder in folders.into_iter() {
        vpcontents = vpcontents
            .into_iter()
            .filter_map(|x| match x {
                VPEntry::Dir(d) => Some(d),
                _ => None,
            })
            .find(|x| x.name == folder)
            .ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))?
            .contents
    }
    vpcontents
        .into_iter()
        .filter_map(|x| match x {
            VPEntry::File(f) => Some(f),
            _ => None,
        })
        .find(|x| x.name == file)
        .ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))
}

pub fn split_path(path: &Path) -> Result<(PathBuf, Vec<String>), std::io::Error> {
    let mut vp_filepath: PathBuf = path.to_path_buf();
    let mut folders: Vec<String> = Vec::new();
    while !vp_filepath.is_file() {
        folders.push(
            vp_filepath
                .file_name()
                .ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))?
                .to_os_string()
                .into_string()
                .map_err(|_| io::Error::from(io::ErrorKind::NotFound))?,
        );
        vp_filepath = vp_filepath
            .parent()
            .ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))?
            .to_path_buf();
    }
    folders.reverse();
    Ok((vp_filepath, folders))
}
