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
    pub image_format: ImageFormat,
    pub size: u32,
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
pub enum ImageFormat {
    R8G8B8A8Unorm = 26,
    BC1Unorm = 49,
    BC2Unorm = 50,
    BC3Unorm = 51,
}

/// GX2TileMode variants used by Xenoblade X.
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, BinWrite, Debug, Clone, Copy, PartialEq, Eq)]
#[brw(repr(u32))]
pub enum TileMode {
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
