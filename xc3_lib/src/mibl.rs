//! Textures in `.witx` or `.witex` files or embedded in `.wismt` files and other formats.
//!
//! # Overview
//! [Mibl] image textures consists of an image data section containing all array layers and mipmaps
//! and a footer describing the surface dimensions and format.
//! The image data is ordered by layer and mipmap like
//! "Layer0 Mip 0 Layer 0 Mip 1 ... Layer L-1 Mip M-1" for L layers and M mipmaps.
//! This is the same ordering expected by DDS and modern graphics APIs.
//!
//! The image data uses a "swizzled" memory layout optimized for the Tegra X1
//! and must be decoded to a standard row-major layout using [Mibl::deswizzled_image_data]
//! for use on other hardware.
//!
//! All of the image formats used in game are supported by DDS, enabling cheap conversions using [Mibl::to_dds]
//! and [Mibl::from_dds]. For converting to and from uncompressed formats like PNG or TIFF,
//! use the encoding and decoding provided by [image_dds].
//!
//! # File Paths
//! Xenoblade 3 `.wismt` [Mibl] are in [Xbc1](crate::xbc1::Xbc1) archives.
//!
//! | Game | File Patterns |
//! | --- | --- |
//! | Xenoblade Chronicles 1 DE | `monolib/shader/*.{witex,witx}` |
//! | Xenoblade Chronicles 2 | `monolib/shader/*.{witex,witx}` |
//! | Xenoblade Chronicles 3 | `chr/tex/nx/{h,m}/*.wismt`, `monolib/shader/*.{witex,witx}` |
use std::io::SeekFrom;

use binrw::{binrw, BinRead, BinWrite};
use image_dds::{ddsfile::Dds, Surface};
use tegra_swizzle::surface::BlockDim;
use thiserror::Error;
use xc3_write::Xc3Write;

pub use tegra_swizzle::SwizzleError;

use crate::xc3_write_binwrite_impl;

/// A swizzled image texture surface.
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Eq, Clone)]
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
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Eq, Clone)]
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
    _magic: (),
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, BinWrite, Debug, Clone, Copy, PartialEq, Eq)]
#[brw(repr(u32))]
pub enum ViewDimension {
    D2 = 1,
    D3 = 2,
    Cube = 8,
}

/// nvn image format types used by Xenoblade 1 DE, Xenoblade 2, and Xenoblade 3.
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, BinWrite, Debug, Clone, Copy, PartialEq, Eq)]
#[brw(repr(u32))]
pub enum ImageFormat {
    R8Unorm = 1,
    R8G8B8A8Unorm = 37,
    R16G16B16A16Float = 41,
    R4G4B4A4Unorm = 57, // TODO: try using this format in xc3 in renderdoc to check channels?
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
            ImageFormat::R4G4B4A4Unorm => BlockDim::uncompressed(),
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
            ImageFormat::R4G4B4A4Unorm => 2,
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
        let aligned_size = unaligned_size.next_multiple_of(4096);

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

xc3_write_binwrite_impl!(Mibl);

#[derive(Debug, Error)]
pub enum CreateMiblError {
    #[error("error swizzling surface")]
    SwizzleError(#[from] tegra_swizzle::SwizzleError),

    #[error("error creating surface from DDS")]
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

    /// Add the swizzled `base_mip_level` with the existing mipmaps.
    /// The base mip should have twice current width and height.
    pub fn with_base_mip(&self, base_mip_level: &[u8]) -> Self {
        // TODO: Will this always have the appropriate mipmap alignment?
        // TODO: How does this work for 3D or array layers?
        let mut image_data = base_mip_level.to_vec();
        image_data.extend_from_slice(&self.image_data);

        let image_size = image_data.len().next_multiple_of(4096) as u32;

        Self {
            image_data,
            footer: MiblFooter {
                image_size,
                width: self.footer.width * 2,
                height: self.footer.height * 2,
                mipmap_count: self.footer.mipmap_count + 1,
                ..self.footer
            },
        }
    }

    // TODO: Tests for this?
    /// Split the texture into a texture with half resolution and a separate base mip level.
    /// The inverse operation of [Self::with_base_mip].
    pub fn split_base_mip(&self) -> (Self, Vec<u8>) {
        // TODO: Does this correctly handle alignment?
        let base_mip_size = self.footer.swizzled_base_mip_size();
        let (base_mip, image_data) = self.image_data.split_at(base_mip_size);

        (
            Self {
                image_data: image_data.to_vec(),
                footer: MiblFooter {
                    image_size: image_data.len().next_multiple_of(4096) as u32,
                    width: self.footer.width / 2,
                    height: self.footer.height / 2,
                    mipmap_count: self.footer.mipmap_count - 1,
                    ..self.footer
                },
            },
            base_mip.to_vec(),
        )
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

        let image_size = image_data.len().next_multiple_of(4096) as u32;

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

    /// Deswizzles all layers and mipmaps to a Direct Draw Surface (DDS).
    pub fn to_dds(&self) -> Result<Dds, crate::dds::CreateDdsError> {
        self.to_surface()?.to_dds().map_err(Into::into)
    }

    /// Swizzles all layers and mipmaps in `dds` to an equivalent [Mibl].
    ///
    /// Returns an error if the conversion fails or the image format is not supported.
    pub fn from_dds(dds: &Dds) -> Result<Self, CreateMiblError> {
        let surface = image_dds::Surface::from_dds(dds)?;
        Self::from_surface(surface)
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

    fn swizzled_base_mip_size(&self) -> usize {
        tegra_swizzle::surface::swizzled_surface_size(
            self.width as usize,
            self.height as usize,
            self.depth as usize,
            self.image_format.block_dim(),
            None,
            self.image_format.bytes_per_pixel(),
            1,
            if self.view_dimension == ViewDimension::Cube {
                6
            } else {
                1
            },
        )
    }
}

impl From<ImageFormat> for image_dds::ImageFormat {
    fn from(value: ImageFormat) -> Self {
        match value {
            ImageFormat::R8Unorm => image_dds::ImageFormat::R8Unorm,
            ImageFormat::R8G8B8A8Unorm => image_dds::ImageFormat::Rgba8Unorm,
            ImageFormat::R16G16B16A16Float => image_dds::ImageFormat::Rgba16Float,
            ImageFormat::R4G4B4A4Unorm => image_dds::ImageFormat::Bgra4Unorm,
            ImageFormat::BC1Unorm => image_dds::ImageFormat::BC1RgbaUnorm,
            ImageFormat::BC2Unorm => image_dds::ImageFormat::BC2RgbaUnorm,
            ImageFormat::BC3Unorm => image_dds::ImageFormat::BC3RgbaUnorm,
            ImageFormat::BC4Unorm => image_dds::ImageFormat::BC4RUnorm,
            ImageFormat::BC5Unorm => image_dds::ImageFormat::BC5RgUnorm,
            ImageFormat::BC7Unorm => image_dds::ImageFormat::BC7RgbaUnorm,
            ImageFormat::BC6UFloat => image_dds::ImageFormat::BC6hRgbUfloat,
            ImageFormat::B8G8R8A8Unorm => image_dds::ImageFormat::Bgra8Unorm,
        }
    }
}

impl TryFrom<image_dds::ImageFormat> for ImageFormat {
    type Error = CreateMiblError;

    fn try_from(value: image_dds::ImageFormat) -> Result<Self, Self::Error> {
        match value {
            image_dds::ImageFormat::R8Unorm => Ok(ImageFormat::R8Unorm),
            image_dds::ImageFormat::Rgba8Unorm => Ok(ImageFormat::R8G8B8A8Unorm),
            image_dds::ImageFormat::Rgba16Float => Ok(ImageFormat::R16G16B16A16Float),
            image_dds::ImageFormat::Bgra8Unorm => Ok(ImageFormat::B8G8R8A8Unorm),
            image_dds::ImageFormat::BC1RgbaUnorm => Ok(ImageFormat::BC1Unorm),
            image_dds::ImageFormat::BC2RgbaUnorm => Ok(ImageFormat::BC2Unorm),
            image_dds::ImageFormat::BC3RgbaUnorm => Ok(ImageFormat::BC3Unorm),
            image_dds::ImageFormat::BC4RUnorm => Ok(ImageFormat::BC4Unorm),
            image_dds::ImageFormat::BC5RgUnorm => Ok(ImageFormat::BC5Unorm),
            image_dds::ImageFormat::BC6hRgbUfloat => Ok(ImageFormat::BC6UFloat),
            image_dds::ImageFormat::BC7RgbaUnorm => Ok(ImageFormat::BC7Unorm),
            image_dds::ImageFormat::Bgra4Unorm => Ok(ImageFormat::R4G4B4A4Unorm),
            _ => Err(CreateMiblError::UnsupportedImageFormat(value)),
        }
    }
}
