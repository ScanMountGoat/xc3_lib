//! Textures in `.witx`, `.witex`, or `.wismt` files or embedded in other formats.
//! 
//! XC3: `chr/tex/nx/*/*.wismt`, `monolib/shader/*.{witex,witx}`
use std::io::SeekFrom;

use anyhow::Result;
use binrw::{binrw, BinRead, BinWrite};
use ddsfile::Dds;
use tegra_swizzle::{surface::BlockDim, SwizzleError};

use crate::dds::create_dds;

/// Image texture data.
/// Used in `.witex` or `.witx` files or embedded in `.wismt` files.
// TODO: also .wiltp and .wilay?
#[derive(Debug, PartialEq, Eq)]
pub struct Mibl {
    pub image_data: Vec<u8>,
    pub footer: MiblFooter,
}

const MIBL_FOOTER_SIZE: usize = 40;

#[binrw]
#[derive(Debug, PartialEq, Eq)]
pub struct MiblFooter {
    /// Swizzled image size for the entire surface aligned to 4096 (0x1000).
    pub image_size: u32,
    pub unk: u32, // TODO: is this actually 0x1000 for swizzled like with nutexb?
    pub width: u32,
    pub height: u32,
    pub depth: u32,
    pub view_dimension: ViewDimension,
    pub image_format: ImageFormat,
    pub mipmap_count: u32,
    pub version: u32,

    #[brw(magic(b"LBIM"))]
    #[br(temp)]
    #[bw(ignore)]
    magic: (),
}

#[derive(BinRead, BinWrite, Debug, Clone, Copy, PartialEq, Eq)]
#[brw(repr(u32))]
pub enum ViewDimension {
    D2 = 1,
    D3 = 2,
    Cube = 8,
}

#[derive(BinRead, BinWrite, Debug, Clone, Copy, PartialEq, Eq)]
#[brw(repr(u32))]
pub enum ImageFormat {
    R8Unorm = 1,
    R8G8B8A8Unorm = 37,
    R16G16B16A16Float = 41,
    BC1Unorm = 66,
    BC2Unorm = 67,
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
            ImageFormat::BC2Unorm => BlockDim::block_4x4(),
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
            ImageFormat::BC2Unorm => 16,
            ImageFormat::BC3Unorm => 16,
            ImageFormat::BC4Unorm => 8,
            ImageFormat::BC5Unorm => 16,
            ImageFormat::BC7Unorm => 16,
            ImageFormat::B8G8R8A8Unorm => 4,
        }
    }
}

impl BinRead for Mibl {
    type Args<'a> = ();

    fn read_options<R: std::io::Read + std::io::Seek>(
        reader: &mut R,
        endian: binrw::Endian,
        args: Self::Args<'_>,
    ) -> binrw::BinResult<Self> {
        // Assume the MIBL is the only item in the reader.
        reader.seek(SeekFrom::End(-(MIBL_FOOTER_SIZE as i64)))?;
        let footer = MiblFooter::read_options(reader, endian, args)?;

        reader.seek(SeekFrom::Start(0))?;

        let mut image_data = vec![0u8; footer.image_size as usize];
        reader.read_exact(&mut image_data)?;

        Ok(Mibl { image_data, footer })
    }
}

impl BinWrite for Mibl {
    type Args<'a> = ();

    fn write_options<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        endian: binrw::Endian,
        _args: Self::Args<'_>,
    ) -> binrw::BinResult<()> {
        let unaligned_size = tegra_swizzle::surface::swizzled_surface_size(
            self.footer.width as usize,
            self.footer.height as usize,
            self.footer.depth as usize,
            self.footer.image_format.block_dim(),
            None,
            self.footer.image_format.bytes_per_pixel(),
            self.footer.mipmap_count as usize,
            if self.footer.view_dimension == ViewDimension::Cube {
                6
            } else {
                1
            },
        );

        // Assume the data is already aligned to 4096.
        // TODO: Better to just store unpadded data?
        let aligned_size = self.image_data.len();

        self.image_data.write_options(writer, endian, ())?;

        // Fit the footer within the padding if possible.
        // Otherwise, create another 4096 bytes for the footer.
        if (aligned_size - unaligned_size) < MIBL_FOOTER_SIZE {
            writer.write_all(&[0u8; 4096])?;
        }

        writer.seek(SeekFrom::End(-(MIBL_FOOTER_SIZE as i64)))?;
        self.footer.write_options(writer, endian, ())?;

        Ok(())
    }
}

impl Mibl {
    pub fn deswizzled_image_data(&self) -> Result<Vec<u8>, SwizzleError> {
        tegra_swizzle::surface::deswizzle_surface(
            self.footer.width as usize,
            self.footer.height as usize,
            self.footer.depth as usize,
            &self.image_data,
            self.footer.image_format.block_dim(),
            None,
            self.footer.image_format.bytes_per_pixel(),
            self.footer.mipmap_count as usize,
            if self.footer.view_dimension == ViewDimension::Cube {
                6
            } else {
                1
            },
        )
    }

    pub fn to_dds(&self) -> Result<Dds> {
        create_dds(self)
    }
}
