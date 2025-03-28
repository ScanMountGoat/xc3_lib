//! User interface [Mibl](crate::mibl::Mibl) images in `.wilay` files.
//!
//! # File Paths
//! Xenoblade 1 and some Xenoblade 3 `.wilay` [Lagp] are in [Xbc1](crate::xbc1::Xbc1) archives.
//!
//! | Game | Versions | File Patterns |
//! | --- | --- | --- |
//! | Xenoblade 1 DE | 10002, 10003 | `menu/image/*.wilay` |
//! | Xenoblade 2 |  | |
//! | Xenoblade 3 | 10003 | `menu/image/*.wilay` |
//! | Xenoblade X DE | 10003 | `ui/image/*.wilay` |
use crate::{
    dhal::{Textures, Unk1, Unk2, Unk3, Unk4, Unk5, Unk6},
    parse_count32_offset32, parse_offset32_count32, parse_opt_ptr32, parse_ptr32,
    parse_string_ptr32,
};
use binrw::{args, binread, BinRead, NullString};
use xc3_write::{
    strings::{StringSectionUnique, WriteOptions},
    Xc3Write, Xc3WriteOffsets,
};

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
    #[br(args { inner: args! { offset: offsets[0], version } })]
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

    #[br(temp, restore_position)]
    offsets: [u32; 2],

    // TODO: type and count?
    #[br(parse_with = parse_opt_ptr32)]
    #[br(args { offset: base_offset, inner: args! { count: (offsets[1] - offsets[0]) as usize / 4 }})]
    #[xc3(offset(u32), align(1))]
    pub unk2: Option<Vec<i32>>,

    // TODO: can be string or vec<u16>?
    #[br(parse_with = parse_opt_ptr32)]
    #[br(args { offset: base_offset, inner: args! { count: 0 }})]
    #[xc3(offset(u32), align(1))]
    pub unk3: Option<Vec<u16>>,

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
    #[br(args { offset: base_offset, inner: args! { count: (offsets[2] - offsets[0]) as usize / 2 }})]
    #[xc3(offset(u32), align(1))]
    pub unk2: Vec<i16>,

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

// TODO: data is similar to Unk8ItemInner but with offsets?
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

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
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
    pub unk1: Vec<Unk13Unk1Unk4Unk1>,

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

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct Unk13Unk1Unk4Unk1 {
    pub unk1: u32,
    pub unk2: u16, // TODO: index?
    pub unk3: u16, // TODO: index?
    pub unk4: u32, // TODO: index?
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
impl Xc3WriteOffsets for LagpOffsets<'_> {
    type Args = ();

    fn write_offsets<W: std::io::prelude::Write + std::io::prelude::Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
        endian: xc3_write::Endian,
        _args: Self::Args,
    ) -> xc3_write::Xc3Result<()> {
        // Different order than field order.
        self.unk1
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.unk3
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.unk4
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.unk13
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.unk2
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.unk5
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.unk6
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.textures
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        Ok(())
    }
}

impl Xc3WriteOffsets for Unk13Offsets<'_> {
    type Args = ();

    fn write_offsets<W: std::io::prelude::Write + std::io::prelude::Seek>(
        &self,
        writer: &mut W,
        _base_offset: u64,
        data_ptr: &mut u64,
        endian: xc3_write::Endian,
        _args: Self::Args,
    ) -> xc3_write::Xc3Result<()> {
        let base_offset = self.base_offset;

        // Some strings are grouped at the end.
        // Strings should use insertion order instead of alphabetical.
        let mut string_section = StringSectionUnique::default();

        let unk1 = self.unk1.write(writer, base_offset, data_ptr, endian)?;
        for u in &unk1.0 {
            u.unk6
                .write_full(writer, base_offset, data_ptr, endian, ())?;
            u.unk2
                .write_full(writer, base_offset, data_ptr, endian, ())?;
            u.unk4
                .write_full(writer, base_offset, data_ptr, endian, ())?;
            string_section.insert_offset32(&u.unk7);
            if let Some(unk5) = u.unk5.write(writer, base_offset, data_ptr, endian)? {
                let base_offset = unk5.base_offset;
                let items = unk5.items.write(writer, base_offset, data_ptr, endian)?;
                for item in items.0 {
                    string_section.insert_offset32(&item.unk1);
                }
            }
            u.unk3
                .write_full(writer, base_offset, data_ptr, endian, ())?;
        }

        self.unk2
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.unk3
            .write_full(writer, base_offset, data_ptr, endian, ())?;

        string_section.write(
            writer,
            base_offset,
            data_ptr,
            &WriteOptions::default(),
            endian,
        )?;
        Ok(())
    }
}

impl Xc3WriteOffsets for Unk13Unk1Unk3Unk2Offsets<'_> {
    type Args = ();

    fn write_offsets<W: std::io::prelude::Write + std::io::prelude::Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
        endian: xc3_write::Endian,
        _args: Self::Args,
    ) -> xc3_write::Xc3Result<()> {
        // The string offset is relative to the start of unk4 data.
        let string_start = *data_ptr;
        self.unk4
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.unk6
            .write_full(writer, string_start, data_ptr, endian, ())?;
        Ok(())
    }
}

impl Xc3WriteOffsets for Unk13Unk1Unk4Offsets<'_> {
    type Args = ();

    fn write_offsets<W: std::io::prelude::Write + std::io::prelude::Seek>(
        &self,
        writer: &mut W,
        _base_offset: u64,
        data_ptr: &mut u64,
        endian: xc3_write::Endian,
        _args: Self::Args,
    ) -> xc3_write::Xc3Result<()> {
        let base_offset = self.base_offset;
        self.unk1
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        if !self.unk2.data.is_empty() {
            self.unk2
                .write_full(writer, base_offset, data_ptr, endian, ())?;
        }
        if !self.unk3.data.is_empty() {
            self.unk3
                .write_full(writer, base_offset, data_ptr, endian, ())?;
        }
        if !self.unk4.data.is_empty() {
            self.unk4
                .write_full(writer, base_offset, data_ptr, endian, ())?;
        }
        Ok(())
    }
}

impl Xc3Write for UnkString {
    type Offsets<'a> = ();

    fn xc3_write<W: std::io::prelude::Write + std::io::prelude::Seek>(
        &self,
        writer: &mut W,
        endian: xc3_write::Endian,
    ) -> xc3_write::Xc3Result<Self::Offsets<'_>> {
        // TODO: Add align_size_to attribute to xc3_write_derive?
        // TODO: Just use binwrite for this?
        let start = writer.stream_position()?;
        self.0.xc3_write(writer, endian)?;
        let end = writer.stream_position()?;

        let size = end - start;
        let aligned_size = size.next_multiple_of(4);
        vec![0u8; (aligned_size - size) as usize].xc3_write(writer, endian)?;
        Ok(())
    }
}
