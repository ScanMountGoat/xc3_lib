//! Textures in `.catex` or `.calut` files or embedded in `.wismt` files and other formats.
//!
//! # File Paths
//!
//! | Game | File Patterns |
//! | --- | --- |
//! | Xenoblade Chronicles X | `chrfc_tex/*.catex`, `chrfc_eye/*.catex`, `menu/tex/*.catex`,  `monolib/shader/*.{calut, catex}` |
//! | Xenoblade Chronicles 1 DE |  |
//! | Xenoblade Chronicles 2 |  |
//! | Xenoblade Chronicles 3 |  |
use std::io::SeekFrom;

use binrw::{binrw, BinRead, BinWrite};
use image_dds::{ddsfile::Dds, Surface};

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
    pub aa_mode: u32,
    pub tile_mode: TileMode,
    pub unk1: u32,
    pub alignment: u32,
    pub pitch: u32,
    pub unk: [u16; 26], // TODO: not always 0?
    pub version: u32,   // 10001 or 10002?

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
        // Assume the Mtxt is the only item in the reader.
        reader.seek(SeekFrom::End(-(FOOTER_SIZE as i64)))?;
        let image_size = reader.stream_position()?;
        let footer = MtxtFooter::read_options(reader, endian, args)?;

        reader.seek(SeekFrom::Start(0))?;

        // TODO: How much of this is just alignment padding?
        let mut image_data = vec![0u8; image_size as usize];
        reader.read_exact(&mut image_data)?;

        Ok(Mtxt { image_data, footer })
    }
}

impl Mtxt {
    /// Deswizzles all layers and mipmaps to a standard row-major memory layout.
    pub fn deswizzled_image_data(&self) -> Vec<u8> {
        let (block_width, block_height) = self.footer.surface_format.block_dim();

        let div_round_up = |x, d| (x + d - 1) / d;

        // TODO: out of bounds for small textures?
        wiiu_swizzle::deswizzle_surface(
            div_round_up(self.footer.width, block_width),
            div_round_up(self.footer.height, block_height),
            self.footer.depth_or_array_layers,
            &self.image_data,
            self.footer.swizzle,
            self.footer.pitch,
            self.footer.tile_mode.into(),
            self.footer.surface_format.bytes_per_pixel(),
        )
    }

    /// Deswizzles all layers and mipmaps to a compatible surface for easier conversions.
    pub fn to_surface(&self) -> Surface<Vec<u8>> {
        Surface {
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
            data: self.deswizzled_image_data(),
        }
    }

    /// Deswizzles all layers and mipmaps to a Direct Draw Surface (DDS).
    pub fn to_dds(&self) -> Result<Dds, crate::dds::CreateDdsError> {
        self.to_surface().to_dds().map_err(Into::into)
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

impl From<TileMode> for wiiu_swizzle::AddrTileMode {
    fn from(value: TileMode) -> Self {
        match value {
            TileMode::D1TiledThin1 => wiiu_swizzle::AddrTileMode::ADDR_TM_1D_TILED_THIN1,
            TileMode::D2TiledThin1 => wiiu_swizzle::AddrTileMode::ADDR_TM_2D_TILED_THIN1,
            TileMode::D2TiledThick => wiiu_swizzle::AddrTileMode::ADDR_TM_2D_TILED_THICK,
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
