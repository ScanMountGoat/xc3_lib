//! Font styles in `.wisty` files.
//!
//! # File Paths
//! Xenoblade DE `.wisty` [Last] are in [Xbc1](crate::xbc1::Xbc1) archives.
//!
//! | Game | Versions | File Patterns |
//! | --- | --- | --- |
//! | Xenoblade Chronicles 1 DE | 10001 | `menu/font/*.wisty` |
//! | Xenoblade Chronicles 2 | 10001 | `menu/font/*.wisty` |
//! | Xenoblade Chronicles 3 | 10001 | `menu/font/*.wisty` |

use crate::{
    parse_offset32_count32, parse_string_opt_ptr32, parse_string_ptr32, xc3_write_binwrite_impl,
};
use bilge::{arbitrary_int::u30, bitsize, prelude::Number, Bitsized, DebugBits, FromBits};
use binrw::{binread, BinRead, BinWrite};
use xc3_write::{Xc3Write, Xc3WriteOffsets};

const VERSION: u32 = 10001;

#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(magic = b"LAST")]
#[xc3(magic(b"LAST"))]
#[xc3(align_after(16))]
pub struct Last {
    #[br(assert(version == VERSION))]
    pub version: u32,

    #[br(temp, restore_position)]
    offset: u32,

    #[br(parse_with = parse_offset32_count32)]
    #[xc3(offset_count(u32, u32))]
    pub styles: Vec<FontStyle>,

    #[br(count = (offset - 16) / 4)]
    pub unks: Vec<u32>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct FontStyle {
    pub flags: StyleFlags,

    /// The name of this style
    #[br(parse_with = parse_string_ptr32)]
    #[xc3(offset(u32))]
    pub style_name: String,

    /// The name of the font to use
    #[br(parse_with = parse_string_opt_ptr32)]
    #[xc3(offset(u32))]
    pub font_name: Option<String>,

    pub scale_x: f32,
    pub scale_y: f32,

    /// Always 100.0, no visible changes when edited
    #[br(assert(unk1 == 100.0))]
    pub unk1: f32,

    /// Maximum width of a single line.
    ///
    /// In XC2, characters that would make the text go past the limit are not displayed.  
    /// In XCDE and XC3, instead, the text area is stretched to fit the maximum width.
    pub max_width: u16,
    /// Maximum lines to display
    pub max_lines: u16,

    /// Affects space between lines.
    ///
    /// For horizontal text, this is added to the glyph height. For vertical text,
    /// this is added to the glyph width.
    pub add_line_space: u16,
    /// Affects space between characters.
    ///
    /// For horizontal text, this is added to the glyph width. For vertical text,
    /// this is added to the glyph height.
    #[xc3(pad_size_to(6))]
    #[br(pad_size_to = 6)]
    pub add_char_space: u16,

    /// Always 4, no visible changes when edited
    #[br(assert(unk2 == 4))]
    pub unk2: u32,
}

#[bitsize(32)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(DebugBits, FromBits, BinRead, BinWrite, PartialEq, Clone, Copy)]
#[br(map = u32::into)]
#[bw(map = |&x| u32::from(x))]
pub struct StyleFlags {
    /// Most likely. This is unset for some unused styles, but unsetting it does not prevent
    /// styles from being loaded.
    pub enabled: bool,
    /// Prevents glyphs from using their own space data, effectively making the font monospace.
    pub monospace: bool,
    pub unk: u30,
}

xc3_write_binwrite_impl!(StyleFlags);
