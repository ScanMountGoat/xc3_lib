use std::io::Read;

use binrw::{binrw, NullString};
use flate2::{
    bufread::{ZlibDecoder, ZlibEncoder},
    Compression,
};
use serde::Serialize;

// TODO: binwrite as well?
// TODO: test read + write
#[binrw]
#[derive(Debug, Serialize)]
#[brw(magic(b"xbc1"))]
pub struct Xbc1 {
    unk1: u32,
    pub decomp_size: u32,
    // temp + calc?
    comp_size: u32,

    unk2: u32, // hash of string data?

    #[br(map = |x: NullString| x.to_string())]
    #[bw(map = |x: &String| NullString::from(x.as_str()))]
    #[brw(pad_size_to = 28)]
    text: String,

    #[br(count = comp_size)]
    #[serde(skip)]
    pub deflate_stream: Vec<u8>,
}

impl Xbc1 {
    pub fn from_decompressed(name: String, decompressed: &[u8]) -> Self {
        let mut encoder = ZlibEncoder::new(&decompressed[..], Compression::best());
        let mut deflate_stream = Vec::new();
        encoder.read_to_end(&mut deflate_stream).unwrap();

        Self {
            unk1: 1,
            decomp_size: decompressed.len() as u32,
            comp_size: deflate_stream.len() as u32,
            // TODO: get from name and hash of name?
            // TODO: Is the "hash" important?
            unk2: 0xEFE353FC,
            text: name,
            deflate_stream,
        }
    }

    pub fn decompress(&self) -> Result<Vec<u8>, std::io::Error> {
        let mut decoder = ZlibDecoder::new(&self.deflate_stream[..]);
        let mut decompressed = vec![0u8; self.decomp_size as usize];
        decoder.read_exact(&mut decompressed)?;

        Ok(decompressed)
    }
}
