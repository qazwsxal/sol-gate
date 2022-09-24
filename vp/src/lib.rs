use std::fs::File;
use std::io::{Read, Seek, SeekFrom};

pub mod fs;
pub mod parser;
pub mod types;

#[cfg(test)]
mod tests {
    use std::fs::read;

    use crate::types::VPEntry;

    use super::*;

    #[test]
    fn read_vp() {
        let mut vp_file = File::open("./test_files/mv_radaricons.vp").unwrap();
        let result = fs::index(&mut vp_file);
        println!("{:?}", result);
        assert!(result.is_ok());
    }

    #[test]
    fn ls() {
        let vp_dir = fs::read_dir("./test_files/mv_radaricons.vp/data/").unwrap();
        println!("{:?}", vp_dir);
    }
    #[test]
    fn read_entry() {
        let entry: VPEntry = fs::read_entry("./test_files/mv_radaricons.vp/data/hud/radar-asteroid.dds").unwrap();
        dbg!(entry);
    }

    #[test]
    fn read_file() {
        let mut raw_file = read("./test_files/radar-asteroid.dds").unwrap();

        let mut vp_entry = fs::read_entry("./test_files/mv_radaricons.vp/data/hud/radar-asteroid.dds").unwrap();
        let vp_file = fs::read_file(vp_entry);
    }

}
