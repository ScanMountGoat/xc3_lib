use std::io::SeekFrom;

use binrw::binread;
use tegra_swizzle::surface::BlockDim;

#[binread]
#[derive(Debug)]
#[br(import(length: usize))]
pub struct Libm {
    // TODO: Does the footer actually overlap the image data?
    // TODO: Is the actual image data size stored somewhere?
    #[br(count = length)]
    pub image_data: Vec<u8>,
    #[br(seek_before = SeekFrom::Current(-MIBL_FOOTER_SIZE))]
    pub footer: LibmFooter,
}

const MIBL_FOOTER_SIZE: i64 = 40;

#[binread]
#[derive(Debug)]
pub struct LibmFooter {
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
    // TODO: magic b"LBIM"?
    // TODO: use () and br(magic)?
    pub magic: u32,
}

#[binread]
#[br(repr(u32))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewDimension {
    D2 = 1,
    D3 = 2,
    Cube = 8,
}

#[binread]
#[br(repr(u32))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    R8Unorm = 1, // TODO: srgb or unorm?
    R8G8B8A8Unorm = 37,
    R16G16B16A16Unorm = 41,
    Bc1Unorm = 66,
    Bc3Unorm = 68,
    Bc4Unorm = 73,
    Bc5Unorm = 75,
    Bc7Unorm = 77,
}

impl ImageFormat {
    pub fn block_dim(&self) -> BlockDim {
        match self {
            ImageFormat::R8Unorm => BlockDim::uncompressed(),
            ImageFormat::R8G8B8A8Unorm => BlockDim::uncompressed(),
            ImageFormat::R16G16B16A16Unorm => BlockDim::uncompressed(),
            ImageFormat::Bc1Unorm => BlockDim::block_4x4(),
            ImageFormat::Bc3Unorm => BlockDim::block_4x4(),
            ImageFormat::Bc4Unorm => BlockDim::block_4x4(),
            ImageFormat::Bc5Unorm => BlockDim::block_4x4(),
            ImageFormat::Bc7Unorm => BlockDim::block_4x4(),
        }
    }

    pub fn bytes_per_pixel(&self) -> usize {
        match self {
            ImageFormat::R8Unorm => 1,
            ImageFormat::R8G8B8A8Unorm => 4,
            ImageFormat::R16G16B16A16Unorm => 8,
            ImageFormat::Bc1Unorm => 8,
            ImageFormat::Bc3Unorm => 16,
            ImageFormat::Bc4Unorm => 8,
            ImageFormat::Bc5Unorm => 16,
            ImageFormat::Bc7Unorm => 16,
        }
    }
}
