//! User interface [Mibl](crate::mibl::Mibl) images in `.wilay` files.
//!
//! # File Paths
//! Xenoblade 1 and some Xenoblade 3 `.wilay` [Lagp] are in [Xbc1](crate::xbc1::Xbc1) archives.
//!
//! | Game | Versions | File Patterns |
//! | --- | --- | --- |
//! | Xenoblade Chronicles 1 DE | 10002, 10003 | `menu/image/*.wilay` |
//! | Xenoblade Chronicles 2 |  | |
//! | Xenoblade Chronicles 3 | 10003 | `menu/image/*.wilay` |
use crate::{
    dhal::{next_offset, Textures, Unk1, Unk2, Unk3, Unk4, Unk5, Unk6},
    parse_count32_offset32, parse_offset32_count32, parse_opt_ptr32, parse_ptr32,
    parse_string_ptr32,
};
use binrw::{args, binread, BinRead, NullString};
use indexmap::IndexMap;
use xc3_write::{Xc3Write, Xc3WriteOffsets};

// TODO: How much of this is shared with LAHD?
#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, PartialEq, Clone)]
#[br(magic(b"LAGP"))]
#[xc3(magic(b"LAGP"))]
pub struct Lagp {
    // TODO: enum?
    pub version: u32,
    // TODO: Different values than dhal?
    pub unk0: u32, // 0, 64, 256, 320?

    #[br(temp, restore_position)]
    offsets: [u32; 13],

    #[br(parse_with = parse_ptr32)]
    #[xc3(offset(u32), align(16))]
    pub unk1: Unk1,

    #[br(parse_with = parse_ptr32)]
    #[xc3(offset(u32), align(1))]
    pub unk2: Unk2,

    #[br(parse_with = parse_opt_ptr32)]
    #[xc3(offset(u32), align(4))]
    pub unk3: Option<Unk3>,

    #[br(parse_with = parse_opt_ptr32)]
    #[br(args { inner: args! { offset: offsets[0], next_unk_offset: next_offset(&offsets, offsets[3]), version } })]
    #[xc3(offset(u32), align(16))]
    pub unk4: Option<Unk4>,

    #[br(parse_with = parse_opt_ptr32)]
    #[xc3(offset(u32))]
    pub unk5: Option<Unk5>,

    #[br(parse_with = parse_opt_ptr32)]
    #[xc3(offset(u32))]
    pub unk6: Option<Unk6>,

    #[br(parse_with = parse_opt_ptr32)]
    #[xc3(offset(u32), align(1))]
    pub textures: Option<Textures>,

    pub unk8: u32,
    pub unk9: u32,
    pub unk10: u32,
    pub unk11: u32,

    pub unk12: u32,

    // TODO: This type is slightly different in 10002 for xc1.
    #[br(parse_with = parse_opt_ptr32)]
    #[br(if(version > 10002))]
    #[xc3(offset(u32), align(1))]
    pub unk13: Option<Unk13>,

    // TODO: padding?
    pub unk: [u32; 11],
}

// TODO: fix writing.
#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, PartialEq, Clone)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct Unk13 {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    #[br(parse_with = parse_offset32_count32)]
    #[br(args { offset: base_offset, inner: base_offset })]
    #[xc3(offset_count(u32, u32), align(1))]
    pub unk1: Vec<Unk13Unk1>,

    // TODO: type?
    #[br(parse_with = parse_opt_ptr32, offset = base_offset)]
    #[xc3(offset(u32), align(1))]
    pub unk2: Option<[i32; 48]>,

    // TODO: type?
    #[br(parse_with = parse_opt_ptr32, offset = base_offset)]
    #[xc3(offset(u32), align(1))]
    pub unk3: Option<[u16; 5]>,

    // TODO: padding?
    pub unk: [u32; 4],
}

#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct Unk13Unk1 {
    pub unk1: u32,

    #[br(temp, restore_position)]
    offsets: [u32; 3],

    #[br(parse_with = parse_ptr32)]
    #[br(args { offset: base_offset, inner: args! { count: (offsets[2] - offsets[0]) as usize / 4 }})]
    #[xc3(offset(u32), align(1))]
    pub unk2: Vec<u32>,

    #[br(parse_with = parse_opt_ptr32, offset = base_offset)]
    #[xc3(offset(u32), align(1))]
    pub unk3: Option<Unk13Unk1Unk3>,

    #[br(parse_with = parse_ptr32, offset = base_offset)]
    #[xc3(offset(u32), align(1))]
    pub unk4: Unk13Unk1Unk4,

    #[br(parse_with = parse_opt_ptr32)]
    #[br(args { offset: base_offset, inner: base_offset })]
    #[xc3(offset(u32), align(1))]
    pub unk5: Option<Unk13Unk1Unk5>,

    #[br(parse_with = parse_opt_ptr32, offset = base_offset)]
    #[xc3(offset(u32), align(1))]
    pub unk6: Option<Unk13Unk1Unk6>,

    #[br(parse_with = parse_string_ptr32, offset = base_offset)]
    #[xc3(offset(u32), align(1))]
    pub unk7: String,
}

#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct Unk13Unk1Unk3 {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    pub unk1: u32,

    #[br(parse_with = parse_offset32_count32)]
    #[br(args { offset: base_offset, inner: base_offset })]
    #[xc3(offset_count(u32, u32), align(1))]
    pub unk2: Vec<Unk13Unk1Unk3Unk2>,

    // TODO: padding?
    pub unk: [u32; 12],
}

#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct Unk13Unk1Unk3Unk2 {
    pub unk1: u32,
    pub unk2: u32,
    pub unk3: u32,

    #[br(temp, restore_position)]
    offsets: [u32; 3],

    // TODO: Does this have a count field?
    #[br(parse_with = parse_ptr32)]
    #[br(args { offset: base_offset, inner: args! { count: (offsets[2] / 4) as usize }})]
    #[xc3(offset(u32), align(1))]
    pub unk4: Vec<u32>,

    pub unk5: u32,

    // Relative to the start of unk4 data.
    #[br(parse_with = parse_ptr32, offset = base_offset + offsets[0] as u64)]
    #[xc3(offset(u32), align(1))]
    pub unk6: UnkString,

    pub unk7: f32,

    // TODO: padding?
    pub unk: [u32; 4],
}

#[derive(Debug, BinRead, PartialEq, Clone)]
pub struct UnkString(#[br(map(|x: NullString| x.to_string()))] pub String);

// TODO: padding after some of the arrays?
#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, PartialEq, Clone)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct Unk13Unk1Unk4 {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    #[br(parse_with = parse_count32_offset32, offset = base_offset)]
    #[xc3(count_offset(u32, u32), align(1))]
    pub unk1: Vec<[u32; 3]>, // [???, ???, index]

    #[br(parse_with = parse_count32_offset32, offset = base_offset)]
    #[xc3(count_offset(u32, u32), align(1))]
    pub unk2: Vec<u16>,

    #[br(parse_with = parse_count32_offset32, offset = base_offset)]
    #[xc3(count_offset(u32, u32), align(1))]
    pub unk3: Vec<u16>,

    #[br(parse_with = parse_count32_offset32, offset = base_offset)]
    #[xc3(count_offset(u32, u32), align(1))]
    pub unk4: Vec<u16>,

    // TODO: padding?
    pub unk: [u32; 8],
}

#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(stream = r)]
#[br(import_raw(parent_base_offset: u64))]
#[xc3(base_offset)]
pub struct Unk13Unk1Unk5 {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    #[br(parse_with = parse_count32_offset32)]
    #[br(args { offset: base_offset, inner: parent_base_offset })]
    #[xc3(count_offset(u32, u32), align(1))]
    pub items: Vec<Unk13Unk1Unk5Item>,
}

// TODO: This doesn't work for version 10002?
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct Unk13Unk1Unk5Item {
    #[br(parse_with = parse_string_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub unk1: String,
    pub unk2: u32, // index?
    pub unk3: f32,
    pub unk4: f32,
    pub unk5: f32,
    pub unk6: f32,
    pub unk7: u32,
    // TODO: not in version 10002?
    pub unk8: u32,
}

#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct Unk13Unk1Unk6 {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32), align(1))]
    pub items: Vec<[i32; 5]>,

    // TODO: Padding?
    pub unk: [u32; 4],
}

// TODO: identical to dhal?
impl<'a> Xc3WriteOffsets for LagpOffsets<'a> {
    fn write_offsets<W: std::io::prelude::Write + std::io::prelude::Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
    ) -> xc3_write::Xc3Result<()> {
        // Different order than field order.
        self.unk1.write_full(writer, base_offset, data_ptr)?;
        self.unk3.write_full(writer, base_offset, data_ptr)?;
        self.unk4.write_full(writer, base_offset, data_ptr)?;
        self.unk13.write_full(writer, base_offset, data_ptr)?;
        self.unk2.write_full(writer, base_offset, data_ptr)?;
        self.unk5.write_full(writer, base_offset, data_ptr)?;
        self.unk6.write_full(writer, base_offset, data_ptr)?;
        self.textures.write_full(writer, base_offset, data_ptr)?;
        Ok(())
    }
}

impl<'a> Xc3WriteOffsets for Unk13Offsets<'a> {
    fn write_offsets<W: std::io::prelude::Write + std::io::prelude::Seek>(
        &self,
        writer: &mut W,
        _base_offset: u64,
        data_ptr: &mut u64,
    ) -> xc3_write::Xc3Result<()> {
        let base_offset = self.base_offset;

        // Some strings are grouped at the end.
        // Strings should use insertion order instead of alphabetical.
        let mut string_section = StringSection::default();

        let unk1 = self.unk1.write(writer, base_offset, data_ptr)?;
        for u in &unk1.0 {
            u.unk6.write_full(writer, base_offset, data_ptr)?;
            u.unk2.write_full(writer, base_offset, data_ptr)?;
            u.unk4.write_full(writer, base_offset, data_ptr)?;
            string_section.insert_offset(&u.unk7);
            if let Some(unk5) = u.unk5.write(writer, base_offset, data_ptr)? {
                let base_offset = unk5.base_offset;
                let items = unk5.items.write(writer, base_offset, data_ptr)?;
                for item in items.0 {
                    string_section.insert_offset(&item.unk1);
                }
            }
            u.unk3.write_full(writer, base_offset, data_ptr)?;
        }

        self.unk2.write_full(writer, base_offset, data_ptr)?;
        self.unk3.write_full(writer, base_offset, data_ptr)?;

        string_section.write(writer, base_offset, data_ptr, 1)?;
        Ok(())
    }
}

impl<'a> Xc3WriteOffsets for Unk13Unk1Unk3Unk2Offsets<'a> {
    fn write_offsets<W: std::io::prelude::Write + std::io::prelude::Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
    ) -> xc3_write::Xc3Result<()> {
        // The string offset is relative to the start of unk4 data.
        let string_start = *data_ptr;
        self.unk4.write_full(writer, base_offset, data_ptr)?;
        self.unk6.write_full(writer, string_start, data_ptr)?;
        Ok(())
    }
}

impl<'a> Xc3WriteOffsets for Unk13Unk1Unk4Offsets<'a> {
    fn write_offsets<W: std::io::prelude::Write + std::io::prelude::Seek>(
        &self,
        writer: &mut W,
        _base_offset: u64,
        data_ptr: &mut u64,
    ) -> xc3_write::Xc3Result<()> {
        let base_offset = self.base_offset;
        self.unk1.write_full(writer, base_offset, data_ptr)?;
        if !self.unk2.data.is_empty() {
            self.unk2.write_full(writer, base_offset, data_ptr)?;
        }
        if !self.unk3.data.is_empty() {
            self.unk3.write_full(writer, base_offset, data_ptr)?;
        }
        if !self.unk4.data.is_empty() {
            self.unk4.write_full(writer, base_offset, data_ptr)?;
        }
        Ok(())
    }
}

impl Xc3Write for UnkString {
    type Offsets<'a> = ();

    fn xc3_write<W: std::io::prelude::Write + std::io::prelude::Seek>(
        &self,
        writer: &mut W,
    ) -> xc3_write::Xc3Result<Self::Offsets<'_>> {
        // TODO: Add align_size_to attribute to xc3_write_derive?
        // TODO: Just use binwrite for this?
        let start = writer.stream_position()?;
        self.0.xc3_write(writer)?;
        let end = writer.stream_position()?;

        let size = end - start;
        let aligned_size = size.next_multiple_of(4);
        vec![0u8; (aligned_size - size) as usize].xc3_write(writer)?;
        Ok(())
    }
}

// TODO: Create a shared type that handles pointer width and sorting.
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Default)]
struct StringSection {
    name_to_offsets: IndexMap<String, Vec<u64>>,
}

impl StringSection {
    fn insert_offset(&mut self, offset: &xc3_write::Offset<'_, u32, String>) {
        self.name_to_offsets
            .entry(offset.data.clone())
            .or_default()
            .push(offset.position);
    }

    fn write<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
        alignment: u64,
    ) -> xc3_write::Xc3Result<()> {
        // Write the string data.
        // TODO: Cleaner way to handle alignment?
        let mut name_to_position = IndexMap::new();
        writer.seek(std::io::SeekFrom::Start(*data_ptr))?;
        let aligned = data_ptr.next_multiple_of(alignment);
        writer.write_all(&vec![0xff; (aligned - *data_ptr) as usize])?;

        for name in self.name_to_offsets.keys() {
            let offset = writer.stream_position()?;
            writer.write_all(name.as_bytes())?;
            writer.write_all(&[0u8])?;
            name_to_position.insert(name, offset);
        }
        *data_ptr = (*data_ptr).max(writer.stream_position()?);

        // Update offsets.
        for (name, offsets) in &self.name_to_offsets {
            for offset in offsets {
                let position = name_to_position[name];
                // Assume all string pointers are 4 bytes.
                writer.seek(std::io::SeekFrom::Start(*offset))?;
                let final_offset = position - base_offset;
                (final_offset as u32).xc3_write(writer)?;
            }
        }

        Ok(())
    }
}
