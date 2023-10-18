//! Compressed container used to store data in other formats.
use std::io::Read;

use binrw::{BinRead, BinResult, BinWrite, NullString};
use flate2::{bufread::ZlibEncoder, Compression};
use zune_inflate::{errors::InflateDecodeErrors, DeflateDecoder, DeflateOptions};

use xc3_write::Xc3Write;

#[derive(BinRead, BinWrite, Debug)]
#[brw(magic(b"xbc1"))]
pub struct Xbc1 {
    pub unk1: u32,
    pub decomp_size: u32,
    pub comp_size: u32,

    pub unk2: u32, // TODO: hash of string data?

    #[br(map = |x: NullString| x.to_string())]
    #[bw(map = |x: &String| NullString::from(x.as_str()))]
    #[brw(pad_size_to = 28)]
    pub text: String,

    /// A zlib encoded compressed stream.
    /// The decompressed or "inflated" stream will have size [decomp_size](#structfield.decomp_size).
    #[br(count = comp_size)]
    #[brw(align_after = 16)]
    pub deflate_stream: Vec<u8>,
}

impl Xbc1 {
    pub fn from_decompressed(name: String, decompressed: &[u8]) -> Self {
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
