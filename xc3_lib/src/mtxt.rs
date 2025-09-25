//! Textures in `.catex`, `.calut`, or `.caavp` files or embedded in `.casmt` files and other formats.
//!
//! # File Paths
//!
//! | Game | Versions | File Patterns |
//! | --- | --- | --- |
//! | Xenoblade X | 10001, 10002 | `chrfc_tex/*.catex`, `chrfc_eye/*.catex`, `menu/tex/*.catex`, `menu/tex/avatar/*.caavp`, `monolib/shader/*.{calut, catex}` |
use std::io::SeekFrom;

use binrw::{BinRead, BinWrite, binrw};
use image_dds::{Surface, ddsfile::Dds};
use xc3_write::Xc3Write;

pub use wiiu_swizzle::SwizzleError;

use crate::{error::CreateMtxtError, xc3_write_binwrite_impl};

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinWrite, PartialEq, Eq, Clone)]
pub struct Mtxt {
    /// The combined image surface data.
    pub image_data: Vec<u8>,
    /// A description of the surface in [image_data](#structfield.image_data).
    pub footer: MtxtFooter,
}

const FOOTER_SIZE: u64 = 112;

// TODO: consistent naming with mibl fields?
// TODO: Fix up these fields and docs.
/// A description of the image surface.
#[binrw]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct MtxtFooter {
    pub swizzle: u32,
    pub surface_dim: SurfaceDim,

    /// The width of the base mip level in pixels.
    pub width: u32,

    /// The height of the base mip level in pixels.
    pub height: u32,

    /// The depth of the base mip level in pixels for 3D textures
    /// or the number of array layers for 2D textures.
    pub depth_or_array_layers: u32,

    /// The number of mip levels or 1 if there are no mipmaps.
    pub mipmap_count: u32,

    pub surface_format: SurfaceFormat,
    pub size: u32, // TODO: linear or row-major size?

    // TODO: 0 if and only if no mipmaps?
    // TODO: points past the start of the last mipmap?
    pub unk_mip_offset: u32, // TODO: what does this do?

    pub tile_mode: TileMode,

    pub unk1: u32, // TODO: usually identical to swizzle?

    /// Usually set to `512 * bytes_per_pixel`.
    pub alignment: u32, // TODO: Is it better to read this as bytes per pixel?

    pub pitch: u32,

    /// Offset into [image_data](struct.Mtxt.html#structfield.image_data)
    /// for each mipmap past the base level starting with mip 1.
    /// Mipmap offsets after mip 1 are relative to the mip 1 offset.
    pub mipmap_offsets: [u32; 13],

    pub version: u32, // TODO: 10001 or 10002?

    #[brw(magic(b"MTXT"))]
    #[br(temp)]
    #[bw(ignore)]
    _magic: (),
}

/// GX2SurfaceDim variants used by Xenoblade X.
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, BinWrite, Debug, Clone, Copy, PartialEq, Eq)]
#[brw(repr(u32))]
pub enum SurfaceDim {
    D2 = 1,
    D3 = 2,
    Cube = 3,
}

/// GX2SurfaceFormat variants used by Xenoblade X.
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, BinWrite, Debug, Clone, Copy, PartialEq, Eq)]
#[brw(repr(u32))]
pub enum SurfaceFormat {
    R8G8B8A8Unorm = 26,
    BC1Unorm = 49,
    BC2Unorm = 50,
    BC3Unorm = 51,
    BC4Unorm = 52,
    BC5Unorm = 53,
}

/// GX2TileMode variants used by Xenoblade X.
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, BinWrite, Debug, Clone, Copy, PartialEq, Eq)]
#[brw(repr(u32))]
pub enum TileMode {
    D1TiledThin1 = 2,
    D2TiledThin1 = 4,
    D2TiledThick = 7,
}

impl BinRead for Mtxt {
    type Args<'a> = ();

    fn read_options<R: std::io::Read + std::io::Seek>(
        reader: &mut R,
        endian: binrw::Endian,
        args: Self::Args<'_>,
    ) -> binrw::BinResult<Self> {
        let saved_pos = reader.stream_position()?;

        // Assume the Mtxt is the last item in the reader.
        reader.seek(SeekFrom::End(-(FOOTER_SIZE as i64)))?;
        let image_size = reader.stream_position()?.saturating_sub(saved_pos);
        let footer = MtxtFooter::read_options(reader, endian, args)?;

        reader.seek(SeekFrom::Start(saved_pos))?;

        // TODO: How much of this is just alignment padding?
        let mut image_data = vec![0u8; image_size as usize];
        reader.read_exact(&mut image_data)?;

        Ok(Mtxt { image_data, footer })
    }
}

impl Mtxt {
    /// Deswizzles all layers and mipmaps to a standard row-major memory layout.
    pub fn deswizzled_image_data(&self) -> Result<Vec<u8>, SwizzleError> {
        // TODO: Investigate swizzling for chr_dl/dl029100.camdo chr_oj/oj300031.camdo.
        if self.footer.width == 2 && self.image_data.len() == 144 {
            return Ok(self.image_data.clone());
        }

        wiiu_swizzle::Gx2Surface {
            dim: self.footer.surface_dim.into(),
            width: self.footer.width,
            height: self.footer.height,
            depth_or_array_layers: self.footer.depth_or_array_layers,
            mipmap_count: self.footer.mipmap_count,
            format: self.footer.surface_format.into(),
            aa: wiiu_swizzle::AaMode::X1,
            usage: 0,
            image_data: &self.image_data,
            mipmap_data: if self.footer.mipmap_count > 1 {
                // TODO: Fix potential panic.
                &self.image_data[self.footer.size as usize..]
            } else {
                &[]
            },
            tile_mode: self.footer.tile_mode.into(),
            swizzle: self.footer.swizzle,
            alignment: self.footer.swizzle,
            pitch: self.footer.pitch,
            mipmap_offsets: self.footer.mipmap_offsets,
        }
        .deswizzle()
    }

    /// Deswizzles all layers and mipmaps to a compatible surface for easier conversions.
    pub fn to_surface(&self) -> Result<Surface<Vec<u8>>, SwizzleError> {
        Ok(Surface {
            width: self.footer.width,
            height: self.footer.height,
            depth: if self.footer.surface_dim == SurfaceDim::D3 {
                self.footer.depth_or_array_layers
            } else {
                1
            },
            layers: if self.footer.surface_dim != SurfaceDim::D3 {
                self.footer.depth_or_array_layers
            } else {
                1
            },
            mipmaps: self.footer.mipmap_count,
            image_format: self.footer.surface_format.into(),
            data: self.deswizzled_image_data()?,
        })
    }

    /// Swizzles all layers and mipmaps in `surface` to an equivalent [Mtxt].
    ///
    /// Returns an error if the conversion fails or the image format is not supported.
    pub fn from_surface<T: AsRef<[u8]>>(surface: Surface<T>) -> Result<Self, CreateMtxtError> {
        let surface_format = surface.image_format.try_into()?;

        // TODO: How to set these values?
        // Assume either depth or layers are used but not both.
        Ok(Self {
            image_data: Vec::new(),
            footer: MtxtFooter {
                swizzle: 0,
                surface_dim: if surface.layers == 6 {
                    SurfaceDim::Cube
                } else if surface.depth > 1 {
                    SurfaceDim::D3
                } else {
                    SurfaceDim::D2
                },
                width: surface.width,
                height: surface.height,
                depth_or_array_layers: surface.depth.max(surface.layers),
                mipmap_count: surface.mipmaps,
                surface_format,
                size: 0,
                unk_mip_offset: 0,
                tile_mode: TileMode::D2TiledThin1,
                unk1: 0,
                alignment: surface_format.bytes_per_pixel() * 512,
                pitch: 0,
                mipmap_offsets: [0; 13],
                version: 10002,
            },
        })
    }

    /// Deswizzles all layers and mipmaps to a Direct Draw Surface (DDS).
    pub fn to_dds(&self) -> Result<Dds, crate::dds::CreateDdsError> {
        self.to_surface()?.to_dds().map_err(Into::into)
    }
}

impl From<SurfaceFormat> for image_dds::ImageFormat {
    fn from(value: SurfaceFormat) -> Self {
        match value {
            SurfaceFormat::R8G8B8A8Unorm => Self::Rgba8Unorm,
            SurfaceFormat::BC1Unorm => Self::BC1RgbaUnorm,
            SurfaceFormat::BC2Unorm => Self::BC2RgbaUnorm,
            SurfaceFormat::BC3Unorm => Self::BC3RgbaUnorm,
            SurfaceFormat::BC4Unorm => Self::BC4RUnorm,
            SurfaceFormat::BC5Unorm => Self::BC5RgUnorm,
        }
    }
}

impl From<SurfaceFormat> for wiiu_swizzle::SurfaceFormat {
    fn from(value: SurfaceFormat) -> Self {
        // This is a subset of the wiiu_swizzle GX2 enum.
        Self::from_repr(value as u32).unwrap()
    }
}

impl From<SurfaceDim> for wiiu_swizzle::SurfaceDim {
    fn from(value: SurfaceDim) -> Self {
        // This is a subset of the wiiu_swizzle GX2 enum.
        Self::from_repr(value as u32).unwrap()
    }
}

impl From<TileMode> for wiiu_swizzle::TileMode {
    fn from(value: TileMode) -> Self {
        // This is a subset of the wiiu_swizzle GX2 enum.
        Self::from_repr(value as u32).unwrap()
    }
}

impl TryFrom<image_dds::ImageFormat> for SurfaceFormat {
    type Error = CreateMtxtError;

    fn try_from(value: image_dds::ImageFormat) -> Result<Self, Self::Error> {
        match value {
            image_dds::ImageFormat::Rgba8Unorm => Ok(Self::R8G8B8A8Unorm),
            image_dds::ImageFormat::BC1RgbaUnorm => Ok(Self::BC1Unorm),
            image_dds::ImageFormat::BC2RgbaUnorm => Ok(Self::BC2Unorm),
            image_dds::ImageFormat::BC3RgbaUnorm => Ok(Self::BC3Unorm),
            image_dds::ImageFormat::BC4RUnorm => Ok(Self::BC4Unorm),
            image_dds::ImageFormat::BC5RgUnorm => Ok(Self::BC5Unorm),
            _ => Err(CreateMtxtError::UnsupportedImageFormat(value)),
        }
    }
}

impl SurfaceFormat {
    pub fn block_dim(&self) -> (u32, u32) {
        match self {
            SurfaceFormat::R8G8B8A8Unorm => (1, 1),
            SurfaceFormat::BC1Unorm => (4, 4),
            SurfaceFormat::BC2Unorm => (4, 4),
            SurfaceFormat::BC3Unorm => (4, 4),
            SurfaceFormat::BC4Unorm => (4, 4),
            SurfaceFormat::BC5Unorm => (4, 4),
        }
    }

    pub fn bytes_per_pixel(&self) -> u32 {
        match self {
            SurfaceFormat::R8G8B8A8Unorm => 4,
            SurfaceFormat::BC1Unorm => 8,
            SurfaceFormat::BC2Unorm => 16,
            SurfaceFormat::BC3Unorm => 16,
            SurfaceFormat::BC4Unorm => 8,
            SurfaceFormat::BC5Unorm => 16,
        }
    }
}

xc3_write_binwrite_impl!(Mtxt);
