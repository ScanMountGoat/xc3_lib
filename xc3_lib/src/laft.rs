use std::io::SeekFrom;

use crate::mibl::Mibl;
use crate::{parse_offset32_count32, parse_opt_ptr32, parse_ptr32, parse_vec};
use binrw::file_ptr::FilePtrArgs;
use binrw::{BinRead, BinResult};
use xc3_write::{Xc3Write, Xc3WriteOffsets};

const VERSION: u32 = 0x2711;

#[derive(BinRead, Xc3Write, Xc3WriteOffsets, Clone)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[br(magic = b"LAFT")]
#[xc3(magic(b"LAFT"))]
pub struct Laft {
    #[br(assert(version == VERSION), pad_size_to(8))]
    #[xc3(pad_size_to(8))]
    version: u32,

    #[br(parse_with = parse_offset32_glyph_count)]
    #[xc3(offset(u32))]
    pub font_info: Vec<GlyphFontInfo>,

    #[br(parse_with = parse_offset32_count32)]
    #[xc3(offset_count(u32, u32))]
    pub offsets: Vec<u16>,

    #[br(parse_with = parse_offset32_count32)]
    #[xc3(offset_count(u32, u32))]
    pub mappings: Vec<GlyphClass>,

    glyph_class_mask: u32,

    #[br(parse_with = parse_opt_ptr32)]
    #[xc3(offset_size(u32, u32))]
    pub texture: Option<Mibl>,

    #[br(parse_with = parse_ptr32)]
    #[xc3(offset(u32))]
    pub settings: FontSettings,

    /// Horizontal space reduction between glyphs.
    ///
    /// This is subtracted from [`GlyphFontInfo::width`].
    ///
    /// XC3 uses 4, but for new files I think it's best to keep it at 0 and adjust space manually
    /// on each glyph. It might have some other purpose I'm not aware of, though.
    pub global_width_reduction: u32,
    /// Used to align text vertically and control line breaks.
    ///
    /// Only used in DE/3. In those games, this value needs to be non-zero for text to display
    /// properly.
    pub line_height: u32,
}

#[derive(BinRead, Xc3Write, Xc3WriteOffsets, Clone, Copy)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub struct FontSettings {
    pub texture_width: u32,
    pub texture_height: u32,
    /// Dimensions of the area of each glyph in the texture, reduced by 1.
    /// For example, if each glyph has a 30x20 area in the texture, these two fields have values
    /// 29 and 19.
    pub glyph_area_width: u32,
    pub glyph_area_height: u32,
    pub glyphs_per_row: u32,
    /// Number of occupied rows, i.e. `ceil(number of glyphs / glyphs_per_row)`
    pub num_rows: u32,
}

/// A class of glyph IDs modulo `mappings_count`.
///
/// In the offset list, `size` consecutive entries can be found for this class, ordered by the
/// codepoint of the glyph they point to.
#[derive(BinRead, Xc3Write, Xc3WriteOffsets, Clone, Copy, Default)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub struct GlyphClass {
    /// Points to [`offsets`]. If `size > 1`, there will be consecutive entries for this class,
    /// ordered by the codepoint of the glyph they point to.
    ///
    /// [`offsets`]: Wifnt::offsets
    pub representative_offset: u16,
    pub size: u16,
}

#[derive(BinRead, Xc3Write, Xc3WriteOffsets, Clone, Copy)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub struct GlyphFontInfo {
    /// The glyph's UTF-16 code point (max `U+FFFF`)
    pub codepoint: u16,
    /// The leftmost x coordinate of the glyph's area, usually one or two pixels left to the
    /// leftmost non-empty pixel
    pub left_x: u8,
    /// The glyph's area width, usually enough to cover one or two pixels after the rightmost
    /// non-empty pixel
    pub width: u8,
}

impl Laft {
    pub fn new(settings: FontSettings, max_mappings: usize) -> Self {
        assert!(max_mappings != 0 && max_mappings.is_power_of_two());
        Self {
            version: VERSION,
            glyph_class_mask: (max_mappings - 1).try_into().unwrap(),
            global_width_reduction: 0,
            line_height: settings.glyph_area_height / 2,
            mappings: vec![Default::default(); max_mappings],
            offsets: Vec::new(),
            font_info: Vec::new(),
            settings,
            texture: None,
        }
    }

    /// For the given UTF-16 code point, returns the glyph's position in the texture grid, and
    /// its associated font info.
    ///
    /// The returned position is the cell number, to get which row/column that corresponds to, use
    /// ```text
    /// row = pos / glyphs_per_row
    /// col = pos % glyphs_per_row
    /// ```
    pub fn get_glyph(&self, codepoint: u16) -> Option<(usize, GlyphFontInfo)> {
        let mapping = self.mappings[(codepoint as u32 & self.glyph_class_mask) as usize];
        if mapping.size == 0 {
            return None;
        }
        let offset = mapping.representative_offset as usize;
        let offset = offset
            + self.offsets[offset..offset + mapping.size as usize]
                .binary_search_by_key(&codepoint, |ofs| self.font_info[*ofs as usize].codepoint)
                .ok()?;
        let grid_pos = self.offsets[offset] as usize;
        Some((grid_pos, self.font_info.get(grid_pos).copied()?))
    }

    /// Registers a glyph.
    ///
    /// **Note**: Glyphs must be registered in the same order as they appear in the texture.
    pub fn register_glyph(&mut self, font_info: GlyphFontInfo) {
        let mapping =
            &mut self.mappings[(font_info.codepoint as u32 & self.glyph_class_mask) as usize];

        // This is the offset in `font_info`, but also the position in the texture grid.
        let font_offset: u16 = self.font_info.len().try_into().unwrap();
        self.font_info.push(font_info);

        mapping.size += 1;

        if mapping.size > 1 {
            // Collision, add offset next to old one, respecting codepoint order
            let old_offset = mapping.representative_offset as usize;
            let next_idx = old_offset
                + self.offsets[old_offset..old_offset + (mapping.size - 1) as usize]
                    .binary_search_by_key(&font_info.codepoint, |ofs| {
                        self.font_info[*ofs as usize].codepoint
                    })
                    .expect_err("glyph already registered");
            self.offsets.insert(next_idx, font_offset);

            // Because we've added an entry in the offsets table, we need to shift all mappings
            // that point to something after it
            for mapping in &mut self.mappings {
                if mapping.representative_offset as usize > old_offset {
                    mapping.representative_offset += 1;
                }
            }
        } else {
            mapping.representative_offset = self.offsets.len().try_into().unwrap();
            self.offsets.push(font_offset);
        }
    }
}

fn parse_offset32_glyph_count<T, R, Args>(
    reader: &mut R,
    endian: binrw::Endian,
    args: FilePtrArgs<Args>,
) -> BinResult<Vec<T>>
where
    for<'a> T: BinRead<Args<'a> = Args> + 'static,
    R: std::io::Read + std::io::Seek,
    Args: Clone,
{
    let pos = reader.stream_position()?;
    let offset = u32::read_options(reader, endian, ())?;
    reader.seek(SeekFrom::Current(4))?;
    let count = u32::read_options(reader, endian, ())?;
    reader.seek(SeekFrom::Current(-4))?;

    if offset == 0 && count != 0 {
        return Err(binrw::Error::AssertFail {
            pos,
            message: format!("unexpected null offset for count {count}"),
        });
    }

    parse_vec(reader, endian, args, offset as u64, count as usize)
}

#[cfg(test)]
mod tests {
    use super::{FontSettings, GlyphFontInfo, Laft};

    const MAX_CODE: u16 = u16::MAX;
    // Spice up the order a bit
    const KEY: u16 = 0xCAFE;

    #[test]
    fn glyph_register() {
        let mut wifnt = Laft::new(
            FontSettings {
                texture_width: 0,
                texture_height: 0,
                glyph_area_width: 0,
                glyph_area_height: 0,
                glyphs_per_row: 0,
                num_rows: 0,
            },
            512,
        );

        for code in (0..MAX_CODE).map(|c| c ^ KEY) {
            wifnt.register_glyph(GlyphFontInfo {
                codepoint: code,
                left_x: 0,
                width: 0,
            });
        }

        for (i, code) in (0..MAX_CODE).map(|c| c ^ KEY).enumerate() {
            let (pos, font) = wifnt.get_glyph(code).unwrap();
            assert_eq!(pos, i);
            assert_eq!(font.codepoint, code);
        }
    }
}
