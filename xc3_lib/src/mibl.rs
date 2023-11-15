//! Textures in `.witx`, `.witex`, or `.wismt` files or embedded in other formats.
//!
//! XC3: `chr/tex/nx/*/*.wismt`, `monolib/shader/*.{witex,witx}`
use std::io::SeekFrom;

use binrw::{binrw, BinRead, BinWrite};
use image_dds::Surface;
use tegra_swizzle::surface::BlockDim;
use thiserror::Error;
use xc3_write::{round_up, Xc3Write};

pub use tegra_swizzle::SwizzleError;

use crate::xc3_write_binwrite_impl;

/// Data for an image texture surface.
#[derive(Debug, PartialEq, Eq)]
pub struct Mibl {
    /// The combined swizzled image surface data.
    /// Ordered as `Layer 0 Mip 0, Layer 0 Mip 1, ... Layer L-1 Mip M-1`
    /// for L layers and M mipmaps similar to DDS files.
    pub image_data: Vec<u8>,
    /// A description of the surface in [image_data](#structfield.image_data).
    pub footer: MiblFooter,
}

const MIBL_FOOTER_SIZE: u64 = 40;

/// A description of the image surface.
#[binrw]
#[derive(Debug, PartialEq, Eq)]
pub struct MiblFooter {
    /// The size of [image_data](struct.Mibl.html#structfield.image_data)
    /// aligned to the page size of 4096 (0x1000) bytes.
    /// This may include the bytes for the footer for some files
    /// and will not always equal the file size.
    pub image_size: u32,
    pub unk: u32, // TODO: is this actually 0x1000 for swizzled like with nutexb?
    /// The width of the base mip level in pixels.
    pub width: u32,
    /// The height of the base mip level in pixels.
    pub height: u32,
    /// The depth of the base mip level in pixels.
    pub depth: u32,
    pub view_dimension: ViewDimension,
    pub image_format: ImageFormat,
    /// The number of mip levels or 1 if there are no mipmaps.
    pub mipmap_count: u32,
    pub version: u32, // 10001?

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

/// nvn image format types used by Xenoblade 1 DE, Xenoblade 2, and Xenoblade 3.
#[derive(BinRead, BinWrite, Debug, Clone, Copy, PartialEq, Eq)]
#[brw(repr(u32))]
pub enum ImageFormat {
    R8Unorm = 1,
    R8G8B8A8Unorm = 37,
    R16G16B16A16Float = 41,
    R4G4B4A4 = 57, // TODO: try using this format in xc3 in renderdoc to check channels?
    BC1Unorm = 66,
    BC2Unorm = 67,
    BC3Unorm = 68,
    BC4Unorm = 73,
    BC5Unorm = 75,
    BC7Unorm = 77,
    BC6UFloat = 80,
    B8G8R8A8Unorm = 109,
}

impl ImageFormat {
    pub fn block_dim(&self) -> BlockDim {
        match self {
            ImageFormat::R8Unorm => BlockDim::uncompressed(),
            ImageFormat::R8G8B8A8Unorm => BlockDim::uncompressed(),
            ImageFormat::R16G16B16A16Float => BlockDim::uncompressed(),
            ImageFormat::R4G4B4A4 => BlockDim::uncompressed(),
            ImageFormat::BC1Unorm => BlockDim::block_4x4(),
            ImageFormat::BC2Unorm => BlockDim::block_4x4(),
            ImageFormat::BC3Unorm => BlockDim::block_4x4(),
            ImageFormat::BC4Unorm => BlockDim::block_4x4(),
            ImageFormat::BC5Unorm => BlockDim::block_4x4(),
            ImageFormat::BC7Unorm => BlockDim::block_4x4(),
            ImageFormat::BC6UFloat => BlockDim::block_4x4(),
            ImageFormat::B8G8R8A8Unorm => BlockDim::uncompressed(),
        }
    }

    pub fn bytes_per_pixel(&self) -> usize {
        match self {
            ImageFormat::R8Unorm => 1,
            ImageFormat::R8G8B8A8Unorm => 4,
            ImageFormat::R16G16B16A16Float => 8,
            ImageFormat::R4G4B4A4 => 2,
            ImageFormat::BC1Unorm => 8,
            ImageFormat::BC2Unorm => 16,
            ImageFormat::BC3Unorm => 16,
            ImageFormat::BC4Unorm => 8,
            ImageFormat::BC5Unorm => 16,
            ImageFormat::BC7Unorm => 16,
            ImageFormat::BC6UFloat => 16,
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

        // Avoid potentially storing the footer in the image data.
        // Alignment will be applied when writing.
        let unaligned_size = footer.swizzled_surface_size();
        let mut image_data = vec![0u8; unaligned_size];
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
        // Assume the image data isn't aligned to the page size.
        let unaligned_size = self.image_data.len() as u64;
        let aligned_size = round_up(unaligned_size, 4096);

        self.image_data.write_options(writer, endian, ())?;

        // Fit the footer within the padding if possible.
        // Otherwise, create another 4096 bytes for the footer.
        let padding_size = aligned_size - unaligned_size;
        writer.write_all(&vec![0u8; padding_size as usize])?;

        if padding_size < MIBL_FOOTER_SIZE {
            writer.write_all(&[0u8; 4096])?;
        }

        writer.seek(SeekFrom::End(-(MIBL_FOOTER_SIZE as i64)))?;
        self.footer.write_options(writer, endian, ())?;

        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum CreateMiblError {
    #[error("error swizzling surface: {0}")]
    SwizzleError(#[from] tegra_swizzle::SwizzleError),

    #[error("error creating surface from DDS: {0}")]
    DdsError(#[from] image_dds::error::SurfaceError),

    #[error("image format {0:?} is not supported by Mibl")]
    UnsupportedImageFormat(image_dds::ImageFormat),
}

impl Mibl {
    /// Deswizzles all layers and mipmaps to a standard row-major memory layout.
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

    /// Deswizzle the combined data from `self` and `base_mip_level` to a standard row-major memory layout.
    pub fn deswizzle_image_data_base_mip(
        &self,
        base_mip_level: Vec<u8>,
    ) -> Result<Vec<u8>, SwizzleError> {
        // The high resolution texture is only a single mip level.
        // TODO: double depth?
        let mut image_data = tegra_swizzle::surface::deswizzle_surface(
            (self.footer.width * 2) as usize,
            (self.footer.height * 2) as usize,
            self.footer.depth as usize,
            &base_mip_level,
            self.footer.image_format.block_dim(),
            None,
            self.footer.image_format.bytes_per_pixel(),
            1,
            if self.footer.view_dimension == ViewDimension::Cube {
                6
            } else {
                1
            },
        )?;

        // Non swizzled data has no alignment requirements.
        // We can just combine the two surfaces.
        image_data.extend_from_slice(&self.deswizzled_image_data()?);

        Ok(image_data)
    }

    /// Deswizzles all layers and mipmaps to a compatible surface for easier conversions.
    pub fn to_surface(&self) -> Result<Surface<Vec<u8>>, SwizzleError> {
        Ok(Surface {
            width: self.footer.width,
            height: self.footer.height,
            depth: self.footer.depth,
            layers: if self.footer.view_dimension == ViewDimension::Cube {
                6
            } else {
                1
            },
            mipmaps: self.footer.mipmap_count,
            image_format: self.footer.image_format.into(),
            data: self.deswizzled_image_data()?,
        })
    }

    /// Swizzles all layers and mipmaps in `dds` to an equivalent [Mibl].
    ///
    /// Returns an error if the conversion fails or the image format is not supported.
    pub fn from_surface<T: AsRef<[u8]>>(surface: Surface<T>) -> Result<Self, CreateMiblError> {
        let Surface {
            width,
            height,
            depth,
            layers,
            mipmaps,
            image_format,
            data,
        } = surface;
        let image_format = ImageFormat::try_from(image_format)?;

        let image_data = tegra_swizzle::surface::swizzle_surface(
            width as usize,
            height as usize,
            depth as usize,
            data.as_ref(),
            image_format.block_dim(),
            None,
            image_format.bytes_per_pixel(),
            mipmaps as usize,
            layers as usize,
        )?;

        let image_size = round_up(image_data.len() as u64, 4096) as u32;

        Ok(Self {
            image_data,
            footer: MiblFooter {
                image_size,
                unk: 4096,
                width,
                height,
                depth,
                view_dimension: if depth > 1 {
                    ViewDimension::D3
                } else if layers == 6 {
                    ViewDimension::Cube
                } else {
                    ViewDimension::D2
                },
                image_format,
                mipmap_count: mipmaps,
                version: 10001,
            },
        })
    }
}

impl MiblFooter {
    fn swizzled_surface_size(&self) -> usize {
        tegra_swizzle::surface::swizzled_surface_size(
            self.width as usize,
            self.height as usize,
            self.depth as usize,
            self.image_format.block_dim(),
            None,
            self.image_format.bytes_per_pixel(),
            self.mipmap_count as usize,
            if self.view_dimension == ViewDimension::Cube {
                6
            } else {
                1
            },
        )
    }
}

xc3_write_binwrite_impl!(Mibl);
