use std::io::Read;

use binrw::binread;
use flate2::bufread::ZlibDecoder;
use serde::Serialize;

#[binread]
#[derive(Debug, Serialize)]
#[br(magic(b"xbc1"))]
pub struct Xbc1 {
    unk1: u32,
    pub decomp_size: u32,
    comp_size: u32,
    unk2: u32,
    #[br(pad_after = 24)]
    unk3: u32,
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
