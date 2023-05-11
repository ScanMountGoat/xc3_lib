use std::{
    error::Error,
    io::{Cursor, SeekFrom},
    path::Path,
};

use binrw::{binread, BinReaderExt};
use serde::Serialize;
use tegra_swizzle::surface::BlockDim;

// .witex, .witx, embedded in .wismt files
// TODO: also .wiltp and .wilay?
#[binread]
#[derive(Debug, Serialize)]
#[br(import(length: usize))]
pub struct Mibl {
    // TODO: Does the footer actually overlap the image data?
    // TODO: Is the actual image data size stored somewhere?
    #[br(count = length)]
    pub image_data: Vec<u8>,
    #[br(seek_before = SeekFrom::Current(-MIBL_FOOTER_SIZE))]
    pub footer: MiblFooter,
}

impl Mibl {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn Error>> {
        let bytes = std::fs::read(path)?;
        let length = bytes.len();
        let mut reader = Cursor::new(bytes);
        reader.read_le_args((length,)).map_err(Into::into)
    }
}

const MIBL_FOOTER_SIZE: i64 = 40;

#[binread]
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct MiblFooter {
    /// Swizzled image size for the entire surface aligned to 4096 (0x1000).
    pub image_size: u32,
    pub unk: u32, // is this actually 0x1000 for swizzled like with nutexb?
    pub width: u32,
    pub height: u32,
    pub depth: u32,
    pub view_dimension: ViewDimension,
    pub image_format: ImageFormat,
    pub mipmap_count: u32,
    pub version: u32,

    #[br(temp, magic(b"LBIM"))]
    magic: (),
}

#[binread]
#[br(repr(u32))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ViewDimension {
    D2 = 1,
    D3 = 2,
    Cube = 8,
}

#[binread]
#[br(repr(u32))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ImageFormat {
    R8Unorm = 1, // TODO: srgb or unorm?
    R8G8B8A8Unorm = 37,
    R16G16B16A16Float = 41,
    BC1Unorm = 66,
    BC3Unorm = 68,
    BC4Unorm = 73,
    BC5Unorm = 75,
    BC7Unorm = 77,
    B8G8R8A8Unorm = 109,
}

impl ImageFormat {
    pub fn block_dim(&self) -> BlockDim {
        match self {
            ImageFormat::R8Unorm => BlockDim::uncompressed(),
            ImageFormat::R8G8B8A8Unorm => BlockDim::uncompressed(),
            ImageFormat::R16G16B16A16Float => BlockDim::uncompressed(),
            ImageFormat::BC1Unorm => BlockDim::block_4x4(),
            ImageFormat::BC3Unorm => BlockDim::block_4x4(),
            ImageFormat::BC4Unorm => BlockDim::block_4x4(),
            ImageFormat::BC5Unorm => BlockDim::block_4x4(),
            ImageFormat::BC7Unorm => BlockDim::block_4x4(),
            ImageFormat::B8G8R8A8Unorm => BlockDim::uncompressed(),
        }
    }

    pub fn bytes_per_pixel(&self) -> usize {
        match self {
            ImageFormat::R8Unorm => 1,
            ImageFormat::R8G8B8A8Unorm => 4,
            ImageFormat::R16G16B16A16Float => 8,
            ImageFormat::BC1Unorm => 8,
            ImageFormat::BC3Unorm => 16,
            ImageFormat::BC4Unorm => 8,
            ImageFormat::BC5Unorm => 16,
            ImageFormat::BC7Unorm => 16,
            ImageFormat::B8G8R8A8Unorm => 4,
        }
    }
}
