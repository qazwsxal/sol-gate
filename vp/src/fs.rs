use crate::parser;
use crate::types::{VPDir, VPEntry, VPHeader, VPIndex};
use std::{
    fs::File,
    io::{self, Read, Seek, SeekFrom},
    path::{Path, PathBuf},
};

pub fn index<T: Read + Seek>(handle: &mut T) -> io::Result<VPDir> {
    handle.seek(SeekFrom::Start(0))?;
    let mut headbuf = vec![0u8; 16];
    let headreadresult = handle.read(&mut headbuf)?;
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

#[derive(Debug)]
pub struct ReadDir {
    path: PathBuf,
    vpcontents: Vec<VPEntry>,
}

pub fn read_dir<P: Into<PathBuf>>(path: P) -> io::Result<ReadDir> {
    let path = path.into();
    let mut vp_filepath: PathBuf = path.clone();
    let mut folders: Vec<String> = Vec::new();
    while vp_filepath.is_file() == false {
        folders.push(
            vp_filepath
                .file_name()
                .ok_or(io::Error::from(io::ErrorKind::NotFound))?
                .to_os_string()
                .into_string()
                .map_err(|_| io::Error::from(io::ErrorKind::NotFound))?,
        );
        vp_filepath = vp_filepath
            .parent()
            .ok_or(io::Error::from(io::ErrorKind::NotFound))?.to_path_buf();
    }

    let mut vpcontents: Vec<VPEntry> = index(&mut File::open(&vp_filepath)?)?.contents;
    folders.reverse();
    for folder in folders.into_iter() {
        vpcontents = vpcontents
            .into_iter()
            .filter_map(|x| match x {
                VPEntry::Dir(d) => Some(d),
                _ => None,
            })
            .find(|x| x.name == folder)
            .ok_or(io::Error::from(io::ErrorKind::NotFound))?
            .contents
    }
    Ok(ReadDir {
        path,
        vpcontents,
    })
}
