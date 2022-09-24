use std::fs::File;
use std::io::{Read, Seek, SeekFrom};

pub mod fs;
pub mod parser;
pub mod types;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_vp() {
        let mut vp_file = File::open("./test_files/mv_radaricons.vp").unwrap();
        let result = fs::index(&mut vp_file);
        println!("{:?}", result);
        assert!(result.is_ok());
    }

    #[test]
    fn list() {
        let vp_dir = fs::read_dir("./test_files/mv_radaricons.vp/data/").unwrap();
        println!("{:?}", vp_dir);
    }

}
