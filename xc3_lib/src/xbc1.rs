use std::io::Read;

use binrw::{BinRead, BinWrite, NullString};
use flate2::{bufread::ZlibEncoder, Compression};
use serde::Serialize;
use zune_inflate::{errors::InflateDecodeErrors, DeflateDecoder, DeflateOptions};

// TODO: test read + write
#[derive(BinRead, BinWrite, Debug, Serialize)]
#[brw(magic(b"xbc1"))]
pub struct Xbc1 {
    unk1: u32,
    pub decomp_size: u32,
    // temp + calc?
    comp_size: u32,

    unk2: u32, // TODO: hash of string data?

    #[br(map = |x: NullString| x.to_string())]
    #[bw(map = |x: &String| NullString::from(x.as_str()))]
    #[brw(pad_size_to = 28)]
    pub text: String,

    /// A zlib encoded compressed stream.
    /// The decompressed or "inflated" stream will have size [decomp_size](#structfield.decomp_size).
    #[br(count = comp_size)]
    #[br(align_after = 16)]
    #[serde(skip)]
    pub deflate_stream: Vec<u8>,
}

impl Xbc1 {
    pub fn from_decompressed(name: String, decompressed: &[u8]) -> Self {
        // TODO: Benchmark this?
        let mut encoder = ZlibEncoder::new(decompressed, Compression::best());
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

    pub fn decompress(&self) -> Result<Vec<u8>, InflateDecodeErrors> {
        let mut decoder = DeflateDecoder::new_with_options(
            &self.deflate_stream,
            DeflateOptions::default().set_size_hint(self.decomp_size as usize),
        );
        decoder.decode_zlib()
    }
}
