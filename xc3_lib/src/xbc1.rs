//! Compressed container used to store data in other formats.
//!
//! [Xbc1] are often used to compress an entire file like `.wismt` texture files for Xenoblade 3
//! or some `.wilay` or `.mot` files for Xenoblade 1 DE.
//! Files may also contain multiple [Xbc1] like model `.wismt` files or map `wismda` files.
//!
//! Decompress the data using [Xbc1::decompress].
//! If the format for the data is known,
//! the decompression and reading can be done in a single call using [Xbc1::extract].
use std::{
    io::{Cursor, Read, Seek},
    path::Path,
};

use binrw::{BinRead, BinReaderExt, BinWrite, NullString};
use flate2::{bufread::ZlibEncoder, Compression};
use thiserror::Error;
use zune_inflate::{DeflateDecoder, DeflateOptions};

use xc3_write::{write_full, Xc3Write, Xc3WriteOffsets};

use crate::{error::DecompressStreamError, hash::hash_crc};

/// A compressed container for a single file or stream.
#[derive(Debug, BinRead, BinWrite, PartialEq, Clone)]
#[brw(magic(b"xbc1"))]
pub struct Xbc1 {
    // TODO: Not always zlib?
    #[br(assert(unk1 == 1))]
    pub unk1: u32,
    /// The number of bytes in [Self::decompress].
    pub decompressed_size: u32,
    /// The number of bytes in [compressed_stream](#structfield.compressed_stream).
    pub compressed_size: u32,

    /// Hash of the original decompressed bytes
    /// for [compressed_stream](#structfield.compressed_stream) using [hash_crc].
    pub decompressed_hash: u32,

    /// The name for this archive.
    /// This is often the name of the original file like `3d4f4c6_middle.witx`.
    #[br(map = |x: NullString| x.to_string())]
    #[bw(map = |x: &String| NullString::from(x.as_str()))]
    #[brw(pad_size_to = 28)]
    pub name: String,

    /// A zlib encoded compressed stream.
    /// The decompressed or "inflated" stream will have size [decompressed_size](#structfield.decompressed_size).
    #[br(count = compressed_size)]
    #[brw(align_after = 16)]
    pub compressed_stream: Vec<u8>,
}

#[derive(Debug, Error)]
pub enum CreateXbc1Error {
    #[error("error reading data: {0}")]
    Io(#[from] std::io::Error),

    #[error("error writing data: {0}")]
    Xc3Write(#[from] Box<dyn std::error::Error>),
}

impl Xbc1 {
    /// Write and compress `data` using ZLIB.
    pub fn new<'a, T>(name: String, data: &'a T) -> Result<Self, CreateXbc1Error>
    where
        T: Xc3Write + 'static,
        T::Offsets<'a>: Xc3WriteOffsets,
    {
        let mut writer = Cursor::new(Vec::new());
        write_full(data, &mut writer, 0, &mut 0)?;
        let decompressed = writer.into_inner();

        Self::from_decompressed(name, &decompressed)
    }

    /// Compress `decompressed` using ZLIB.
    pub fn from_decompressed(name: String, decompressed: &[u8]) -> Result<Self, CreateXbc1Error> {
        let mut encoder = ZlibEncoder::new(decompressed, Compression::best());
        let mut compressed_stream = Vec::new();
        encoder.read_to_end(&mut compressed_stream)?;

        Ok(Self {
            unk1: 1,
            decompressed_size: decompressed.len() as u32,
            compressed_size: compressed_stream.len() as u32,
            decompressed_hash: hash_crc(decompressed),
            name,
            compressed_stream,
        })
    }

    /// Decompresses the data by assuming ZLIB compression.
    pub fn decompress(&self) -> Result<Vec<u8>, DecompressStreamError> {
        let mut decoder = DeflateDecoder::new_with_options(
            &self.compressed_stream,
            DeflateOptions::default().set_size_hint(self.decompressed_size as usize),
        );
        decoder.decode_zlib().map_err(Into::into)
    }

    /// Decompress and read the data by assuming ZLIB compression.
    pub fn extract<T>(&self) -> Result<T, DecompressStreamError>
    where
        for<'a> T: BinRead<Args<'a> = ()>,
    {
        let bytes = self.decompress()?;
        T::read_le(&mut Cursor::new(bytes)).map_err(Into::into)
    }
}

// TODO: Derive this?
impl Xc3Write for Xbc1 {
    type Offsets<'a> = ();

    fn xc3_write<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        data_ptr: &mut u64,
    ) -> xc3_write::Xc3Result<Self::Offsets<'_>> {
        self.write_le(writer)?;
        *data_ptr = (*data_ptr).max(writer.stream_position()?);
        Ok(())
    }

    const ALIGNMENT: u64 = 16;
}

/// Helper type for reading data that may be compressed in an [Xbc1] archive.
#[derive(BinRead)]
pub enum MaybeXbc1<T>
where
    for<'a> T: BinRead<Args<'a> = ()>,
{
    Uncompressed(T),
    Xbc1(Xbc1),
}

impl<T> MaybeXbc1<T>
where
    for<'a> T: BinRead<Args<'a> = ()>,
{
    pub fn read<R: Read + Seek>(reader: &mut R) -> binrw::BinResult<Self> {
        reader.read_le().map_err(Into::into)
    }

    /// Read from `path` using a fully buffered reader for performance.
    pub fn from_file<P: AsRef<Path>>(path: P) -> binrw::BinResult<Self> {
        let mut reader = Cursor::new(std::fs::read(path)?);
        reader.read_le().map_err(Into::into)
    }

    /// Read from `bytes` using a fully buffered reader for performance.
    pub fn from_bytes<B: AsRef<[u8]>>(bytes: B) -> binrw::BinResult<Self> {
        Self::read(&mut Cursor::new(bytes))
    }
}
