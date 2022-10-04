use nom::number::complete::le_u32;
use nom::IResult;

use lz4::block::{compress, decompress};

pub struct LZ4Info {
    pub offsets: u32,
    pub filesize: u32,
    pub blocksize: u32,
}

impl LZ4Info {
    pub fn parse(i: &[u8]) -> IResult<&[u8], Self> {
        let (i, offsets) = le_u32(i)?;
        let (i, filesize) = le_u32(i)?;
        let (i, blocksize) = le_u32(i)?;
        Ok((
            i,
            Self {
                offsets,
                filesize,
                blocksize,
            },
        ))
    }
}

// Pass Vec<u8> in so we can either consume it or return it.
pub fn maybe_decompress(buf: Vec<u8>) -> Vec<u8> {
    if &buf[..4] == "LZ41".as_bytes() {
        real_decompress(&buf[4..])
    } else {
        buf
    }
}

fn real_decompress(buf: &[u8]) -> Vec<u8> {
    let end: usize = buf.len() - 12;
    let info = LZ4Info::parse(&buf[end..]).unwrap().1;
    let len = info.filesize.try_into().unwrap();
    decompress(buf, Some(len)).unwrap()
}

pub fn maybe_compress(buf: Vec<u8>) -> Vec<u8> {
    let _raw_compressed = compress(&buf, None, false).unwrap();
    // Add extra stuff to make full compressed vec.
    // if compressed.len() < buf.len() {
    //     compressed;
    // } else {
    //     buf;
    // }
    buf // TODO
}
