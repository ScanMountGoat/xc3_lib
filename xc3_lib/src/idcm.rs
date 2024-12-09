//! Map collision geometry in `.idcm` files or embedded in other files.
//!
//! # File Paths
//! | Game | Versions | File Patterns |
//! | --- | --- | --- |
//! | Xenoblade Chronicles 1 DE | 10003 | `map/*.wiidcm` |
//! | Xenoblade Chronicles 2 | 10003 | `map/*.wiidcm` |
//! | Xenoblade Chronicles 3 | 10003 | `map/*.idcm` |
use crate::{
    parse_offset32_count16, parse_offset32_count32, parse_ptr32, parse_string_ptr32, StringOffset32,
};
use binrw::{binread, BinRead};
use xc3_write::{Xc3Write, Xc3WriteOffsets};

#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, PartialEq, Clone)]
#[br(stream = r)]
#[br(magic(b"IDCM"))]
#[xc3(base_offset)]
#[xc3(magic(b"IDCM"))]
pub struct Idcm {
    // Subtract the magic size.
    #[br(temp, try_calc = r.stream_position().map(|p| p - 4))]
    base_offset: u64,

    pub version: u32,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub unk1: Vec<[u32; 15]>,

    #[br(parse_with = parse_offset32_count32)]
    #[br(args { offset: base_offset, inner: base_offset })]
    #[xc3(offset_count(u32, u32))]
    pub unk2: Vec<Unk2>,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub unk3: Vec<u64>,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub unk4: Vec<[u32; 2]>,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub unk5: Vec<u32>,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub unk6: Vec<u32>,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub unk7: Vec<[u32; 3]>,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub unk8: Vec<[f32; 4]>,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub unk9: Vec<Unk9>,

    pub unk10: u64,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub unk11: Vec<u32>, // TODO: type?

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub unk13: Vec<[f32; 8]>,

    #[br(parse_with = parse_offset32_count32)]
    #[br(args { offset: base_offset, inner: base_offset })]
    #[xc3(offset_count(u32, u32))]
    pub unk19: Vec<Unk19>,

    pub unks1_1: u32,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub unk18: Vec<[u32; 10]>,

    pub unks1_3: [u32; 2],

    #[br(parse_with = parse_ptr32)]
    #[br(args { offset: base_offset, inner: base_offset })]
    #[xc3(offset(u32))]
    pub unk20: StringOffset32,

    pub unk21: u32,

    #[br(parse_with = parse_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub unk15: [f32; 10],

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub unks1_2: Vec<u32>, // TODO: type?

    // TODO: string pointers?
    #[br(parse_with = parse_offset32_count32)]
    #[br(args{ offset: base_offset, inner: base_offset})]
    #[xc3(offset_count(u32, u32))]
    pub unk16: Vec<Unk16>,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub unk17: Vec<[u32; 4]>,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub unks1_4: Vec<u32>, // TODO: type?

    pub unks: [u32; 12], // TODO: padding?
}

// TODO: face data?
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct Unk2 {
    // TODO: These offsets aren't in any particular order?
    #[br(parse_with = parse_offset32_count16, offset = base_offset)]
    #[xc3(offset_count(u32, u16))]
    pub unk1: Vec<[u16; 3]>,

    pub unk2: u16,
    pub unk3: u32,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct Unk9 {
    // TODO: half float?
    pub unk1: u16,
    pub unk2: u16,
    pub unk3: u16,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct Unk16 {
    #[br(parse_with = parse_string_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub unk1: String,
    pub unk2: u32,
    pub unk3: u32,
    pub unk4: u32,
    pub unk5: u32,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct Unk19 {
    #[br(parse_with = parse_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub unk1: u32,
    pub unk2: u32, // TODO: offset into floats?
    pub unk3: u32,
}

impl<'a> Xc3WriteOffsets for IdcmOffsets<'a> {
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
        // Different order than field order.
        self.unk15
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.unk1
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.unk17
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        let unk2 = self.unk2.write(writer, base_offset, data_ptr, endian)?;
        self.unk3
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.unk4
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.unk18
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        let unk16 = self.unk16.write(writer, base_offset, data_ptr, endian)?;
        self.unk5
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.unk6
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.unk7
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.unk8
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.unk9
            .write_full(writer, base_offset, data_ptr, endian, ())?;

        // TODO: A lot of empty lists go here?
        *data_ptr += 12;

        self.unk11
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.unks1_2
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.unks1_4
            .write_full(writer, base_offset, data_ptr, endian, ())?;

        self.unk13
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        let unk19 = self.unk19.write(writer, base_offset, data_ptr, endian)?;

        for u in unk2.0 {
            u.write_offsets(writer, base_offset, data_ptr, endian, ())?;
        }

        for u in unk19.0 {
            u.write_offsets(writer, base_offset, data_ptr, endian, ())?;
        }

        self.unk20
            .write_full(writer, base_offset, data_ptr, endian, ())?;

        for u in unk16.0 {
            u.write_offsets(writer, base_offset, data_ptr, endian, ())?;
        }

        Ok(())
    }
}
