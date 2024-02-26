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
    dhal::{Textures, Unk1, Unk2, Unk3, Unk4, Unk5, Unk6},
    parse_offset32_count32, parse_opt_ptr32, parse_ptr32, parse_string_ptr32,
};
use binrw::{args, binread, BinRead};
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
    #[xc3(skip)]
    offset: u32,

    #[br(parse_with = parse_ptr32)]
    #[xc3(offset(u32))]
    pub unk1: Unk1,

    #[br(parse_with = parse_ptr32)]
    #[xc3(offset(u32))]
    pub unk2: Unk2,

    #[br(parse_with = parse_opt_ptr32)]
    #[xc3(offset(u32), align(4))]
    pub unk3: Option<Unk3>,

    #[br(parse_with = parse_opt_ptr32)]
    #[br(args { inner: args! { offset, version } })]
    #[xc3(offset(u32))]
    pub unk4: Option<Unk4>,

    #[br(parse_with = parse_opt_ptr32)]
    #[xc3(offset(u32))]
    pub unk5: Option<Unk5>,

    #[br(parse_with = parse_opt_ptr32)]
    #[xc3(offset(u32))]
    pub unk6: Option<Unk6>,

    #[br(parse_with = parse_opt_ptr32)]
    #[xc3(offset(u32))]
    pub textures: Option<Textures>,

    pub unk8: u32,
    pub unk9: u32,
    pub unk10: u32,
    pub unk11: u32,

    pub unk12: u32,

    #[br(parse_with = parse_opt_ptr32)]
    #[xc3(offset(u32))]
    pub unk13: Option<Unk13>,

    // TODO: padding?
    #[br(assert(unk.iter().all(|u| *u == 0)))]
    pub unk: [u32; 11],
}

// TODO: more strings?
#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct Unk13 {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    #[br(parse_with = parse_offset32_count32)]
    #[br(args { offset: base_offset, inner: base_offset })]
    #[xc3(offset_count(u32, u32))]
    pub unk1: Vec<Unk13Unk1>,

    // TODO: type?
    #[br(parse_with = parse_opt_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub unk2: Option<[u32; 40]>,

    // TODO: type?
    #[br(parse_with = parse_opt_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub unk3: Option<[u16; 4]>,

    // TODO: padding?
    pub unk: [u32; 4],
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct Unk13Unk1 {
    pub unk1: u32,
    pub unk2: u32,
    pub unk3: u32,
    pub unk4: u32,
    pub unk5: u32,
    pub unk6: u32,
    #[br(parse_with = parse_string_ptr32, offset = base_offset)]
    pub unk7: String,
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
