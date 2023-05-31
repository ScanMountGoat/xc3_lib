use std::io::Read;

use binrw::{binread, NullString};
use flate2::bufread::ZlibDecoder;
use serde::Serialize;

// TODO: binwrite as well?
// TODO: test read + write
#[binread]
#[derive(Debug, Serialize)]
#[br(magic(b"xbc1"))]
pub struct Xbc1 {
    unk1: u32,
    pub decomp_size: u32,
    // temp + calc?
    comp_size: u32,

    unk2: u32, // hash of string data?

    #[br(map = |x: NullString| x.to_string(), pad_size_to = 28)]
    text: String,

    #[br(count = comp_size)]
    #[serde(skip)]
    pub deflate_stream: Vec<u8>,
}

impl Xbc1 {
    pub fn decompress(&self) -> Result<Vec<u8>, std::io::Error> {
        let mut decoder = ZlibDecoder::new(&self.deflate_stream[..]);
        let mut decompressed = vec![0u8; self.decomp_size as usize];
        decoder.read_exact(&mut decompressed)?;

        Ok(decompressed)
    }
}
