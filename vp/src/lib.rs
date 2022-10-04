pub mod compression;
pub mod fs;
pub mod parser;
pub mod types;
#[cfg(test)]
mod tests {
    use std::{
        fs::{read, File},
        io::{Read, Seek},
        path::PathBuf,
    };

    use crate::types::VPFile;

    use super::*;

    #[test]
    fn read_vp() {
        let mut vp_file = File::open("./test_files/mv_radaricons.vp").unwrap();
        let result = fs::index(&mut vp_file);
        println!("{:?}", result);
        assert!(result.is_ok());
    }

    #[test]
    fn read_entry() {
        let entry: VPFile =
            fs::read_entry("./test_files/mv_radaricons.vp/data/hud/radar-asteroid.dds").unwrap();
        dbg!(entry);
    }

    #[test]
    fn read_file() {
        let raw_data = read("./test_files/radar-asteroid.dds").unwrap();
        let vp_filepath: PathBuf =
            "./test_files/mv_radaricons.vp/data/hud/radar-asteroid.dds".into();
        // Split into filesystem + VP path
        let (fs_path, vp_path) = fs::split_path(&vp_filepath).unwrap();
        // locate where in VP file the file we want is by reading the index
        let contents = fs::index(&mut File::open(&fs_path).unwrap()).unwrap();
        let vp_entry = contents.locate(&vp_path).unwrap();
        // Now we know where the file is, allocate a buffer to read it into
        let filesize = vp_entry.size.try_into().unwrap();
        let mut vp_data: Vec<u8> = vec![0; filesize];
        // Actually do the business of opening, seeking and reading the data
        let mut vp_file = File::open(&fs_path).unwrap();
        vp_file
            .seek(std::io::SeekFrom::Start(vp_entry.fileoffset))
            .unwrap();
        vp_file.read_exact(&mut vp_data).unwrap();
        assert_eq!(raw_data, vp_data)
    }
}
