//! Fonts in `.fnt` files.
//!
//! # File Paths
//!
//! | Game | Versions | File Patterns |
//! | --- | --- | --- |
//! | Xenoblade Chronicles X | | `menu/font/**/*.fnt` |

use std::io::{Cursor, SeekFrom};

use crate::{mtxt::Mtxt, parse_ptr32};
use binrw::{BinRead, BinResult};
use xc3_write::{Xc3Write, Xc3WriteOffsets};

const VERSION: u32 = 2;

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, PartialEq, Clone)]
pub struct Fnt {
    #[br(assert(version == VERSION))]
    version: u32,
    #[xc3(shared_offset)]
    file_size: u32,

    #[br(parse_with = parse_ptr32)]
    #[xc3(offset(u32))]
    pub font: XcxFont,

    // No problem with sub-cursors here, XCX expects the MTXT footer to end the file
    // (i.e. mtxt size = file_size - textures_offset)
    #[br(parse_with = parse_ptr32_mtxt)]
    #[xc3(offset(u32), align(4096))]
    pub textures: Mtxt,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct XcxFont {
    pub grid_width: u32,
    pub grid_height: u32,
    glyph_count: u32,

    /// Some sort of threshold, setting it too high makes some of the text not render.  
    /// Setting it to zero is a safe bet.
    pub unk_1: u32,
    /// Final width property for glyph rendering.
    ///
    /// Glyph indexes use the regular grid dimensions ([`grid_width`] x [`grid_height`]). To render
    /// the glyph, the game then clips/extends the sprite to this width.
    ///
    /// Unlike [`Laft`], glyph-specific x/width does not shift the sprite box (potentially overlapping
    /// other glyphs), instead it just controls how much whitespace is rendered before and after the glyph.
    ///
    /// [`Laft`]: crate::laft::Laft
    pub subgrid_width: u32,
    /// No visual differences when changed
    pub unk_2: u32,

    pub font_height: u32,
    pub glyphs_per_row: u32,
    pub num_rows: u32,

    #[br(count = glyph_count)]
    glyphs: Vec<XcxGlyph>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone, Copy)]
pub struct XcxGlyph {
    /// UTF-16 code point, acting as the primary key for this glyph.
    ///
    /// Both [`XcxFont::glyphs`] and the grid in the texture sheet must be ordered by this key.
    pub code_utf16: u16,
    /// Shift-JIS code point, allows duplicates (e.g. unsupported characters share it with a similar
    /// character) and needs not be a sort key.
    pub code_shift_jis: u16,

    /// X offset relative to the next glyph (higher values result in the glyph getting shifted
    /// farther to the left)
    pub x_offset: u8,
    /// Additional whitespace after the glyph
    pub width: u8,
}

impl XcxFont {
    /// Returns the registered glyphs, in UTF-16 code point order
    pub fn glyphs(&self) -> &[XcxGlyph] {
        &self.glyphs
    }

    pub fn get_glyph_by_utf16(&self, code_utf16: u16) -> Option<&XcxGlyph> {
        self.glyphs
            .binary_search_by_key(&code_utf16, |g| g.code_utf16)
            .ok()
            .map(|i| &self.glyphs[i])
    }

    pub fn get_glyph_by_shift_jis(&self, code_shift_jis: u16) -> Option<&XcxGlyph> {
        self.glyphs
            .iter()
            .find(|g| g.code_shift_jis == code_shift_jis)
    }

    /// Registers a new glyph.
    ///
    /// Duplicate Shift-JIS code points are allowed, while duplicate UTF-16 codes are not. The
    /// function panics if a glyph with the same UTF-16 code point is already registered.
    pub fn register_glyph(&mut self, glyph: XcxGlyph) {
        let idx = self
            .glyphs
            .binary_search_by_key(&glyph.code_utf16, |g| g.code_utf16)
            .expect_err("glyph already registered");
        self.glyphs.insert(idx, glyph);
        self.glyph_count += 1;
    }
}

impl<'a> Xc3WriteOffsets for FntOffsets<'a> {
    fn write_offsets<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
        endian: xc3_write::Endian,
    ) -> xc3_write::Xc3Result<()> {
        self.font
            .write_full(writer, base_offset, data_ptr, endian)?;
        self.textures
            .write_full(writer, base_offset, data_ptr, endian)?;
        // Update file size field with current position after writing the entire file structure
        self.file_size
            .write_full(writer, base_offset, data_ptr, endian)?;
        Ok(())
    }
}

fn parse_ptr32_mtxt<T, R, Args>(reader: &mut R, endian: binrw::Endian, args: Args) -> BinResult<T>
where
    for<'a> T: BinRead<Args<'a> = Args> + 'static,
    R: std::io::Read + std::io::Seek,
    Args: Clone,
{
    // Mtxt uses SeekFrom::Start(0), we need to create a sub-cursor
    let offset = u32::read_options(reader, endian, ())?;
    let pos = reader.stream_position()?;
    let mut buf = Vec::new();

    reader.seek(SeekFrom::Start(offset.into()))?;
    reader.read_to_end(&mut buf)?;
    reader.seek(SeekFrom::Start(pos))?;

    T::read_options(&mut Cursor::new(buf), endian, args)
}
