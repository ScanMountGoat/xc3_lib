//! Compressed container used to store data in other formats.
use std::io::Read;

use binrw::{BinRead, BinResult, BinWrite, NullString};
use flate2::{bufread::ZlibEncoder, Compression};
use zune_inflate::{errors::InflateDecodeErrors, DeflateDecoder, DeflateOptions};

use xc3_write::Xc3Write;

use crate::hash::hash_crc;

#[derive(BinRead, BinWrite, Debug)]
#[brw(magic(b"xbc1"))]
pub struct Xbc1 {
    pub unk1: u32,
    pub decomp_size: u32,
    pub comp_size: u32,

    /// Hash of the original decompressed bytes
    /// for [compressed_stream](#structfield.compressed_stream) using [hash_crc](crate::hash::hash_crc).
    pub decompressed_hash: u32,

    #[br(map = |x: NullString| x.to_string())]
    #[bw(map = |x: &String| NullString::from(x.as_str()))]
    #[brw(pad_size_to = 28)]
    pub text: String,

    /// A zlib encoded compressed stream.
    /// The decompressed or "inflated" stream will have size [decomp_size](#structfield.decomp_size).
    #[br(count = comp_size)]
    #[brw(align_after = 16)]
    pub compressed_stream: Vec<u8>,
}

impl Xbc1 {
    /// Compresses the data in `decompressed` using ZLIB.
    pub fn from_decompressed(name: String, decompressed: &[u8]) -> Self {
        let mut encoder = ZlibEncoder::new(decompressed, Compression::best());
        let mut compressed_stream = Vec::new();
        encoder.read_to_end(&mut compressed_stream).unwrap();

        Self {
            unk1: 1,
            decomp_size: decompressed.len() as u32,
            comp_size: compressed_stream.len() as u32,
            decompressed_hash: hash_crc(decompressed),
            text: name,
            compressed_stream,
        }
    }

    /// Decompresses the data by assuming ZLIB compression.
    pub fn decompress(&self) -> Result<Vec<u8>, InflateDecodeErrors> {
        let mut decoder = DeflateDecoder::new_with_options(
            &self.compressed_stream,
            DeflateOptions::default().set_size_hint(self.decomp_size as usize),
        );
        decoder.decode_zlib()
    }
}

impl Xc3Write for Xbc1 {
    type Offsets<'a> = ();

    fn xc3_write<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        data_ptr: &mut u64,
    ) -> BinResult<Self::Offsets<'_>> {
        let result = self.write_le(writer);
        *data_ptr = (*data_ptr).max(writer.stream_position()?);
        result
    }

    const ALIGNMENT: u64 = 16;
}
