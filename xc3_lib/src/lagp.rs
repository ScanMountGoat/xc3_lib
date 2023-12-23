//! User interface [Mibl](crate::mibl::Mibl) images in `.wilay` files.
//!
//! # File Paths
//! Xenoblade 1 and some Xenoblade 3 `.wilay` [Lagp] are in [Xbc1](crate::xbc1::Xbc1) archives.
//!
//! | Game | Versions | File Patterns |
//! | --- | --- |
//! | Xenoblade Chronicles 1 DE | 10002, 10003 | `menu/image/*.wilay` |
//! | Xenoblade Chronicles 2 |  | |
//! | Xenoblade Chronicles 3 | 10003 | `menu/image/*.wilay` |
use crate::{
    dhal::{Textures, Unk1, Unk3, Unk4},
    parse_offset32_count32, parse_opt_ptr32, parse_ptr32, parse_string_ptr32,
};
use binrw::{binread, BinRead};
use xc3_write::{Xc3Write, Xc3WriteOffsets};

// TODO: How much of this is shared with LAHD?
// TODO: Is this used for xc2?
#[derive(Debug, BinRead, Xc3Write)]
#[br(magic(b"LAGP"))]
#[xc3(magic(b"LAGP"))]
pub struct Lagp {
    // TODO: enum?
    pub version: u32,
    // TODO: Different values than dhal?
    pub unk0: u32, // 0, 64, 256, 320?

    #[br(parse_with = parse_ptr32)]
    #[xc3(offset(u32))]
    pub unk1: Unk1,

    // TODO: Only field not present with dhal?
    #[br(parse_with = parse_ptr32)]
    #[xc3(offset(u32))]
    pub unk2: Unk2,

    #[br(parse_with = parse_opt_ptr32)]
    #[xc3(offset(u32), align(4))]
    pub unk3: Option<Unk3>,

    #[br(parse_with = parse_opt_ptr32)]
    #[xc3(offset(u32))]
    pub unk4: Option<Unk4>,

    #[br(parse_with = parse_opt_ptr32)]
    #[xc3(offset(u32))]
    pub unk5: Option<[u32; 4]>,

    #[br(parse_with = parse_opt_ptr32)]
    #[xc3(offset(u32))]
    pub unk6: Option<[u32; 3]>,

    #[br(parse_with = parse_opt_ptr32)]
    #[xc3(offset(u32))]
    pub textures: Option<Textures>,

    pub unk8: u32,
    pub unk9: u32,
    pub unk10: u32,
    pub unk11: u32,

    pub unk12: u32,
    // offset?
    #[br(parse_with = parse_opt_ptr32)]
    #[xc3(offset(u32))]
    pub unk13: Option<Unk13>,

    // TODO: padding?
    #[br(assert(unk.iter().all(|u| *u == 0)))]
    pub unk: [u32; 11],
}

#[binread]
#[derive(Debug, Xc3Write, Xc3WriteOffsets)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct Unk2 {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub unk1: Vec<[u32; 3]>,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub unk2: Vec<[u32; 2]>,

    // TODO: type?
    // TODO: params with f32, f32, ..., 0xffffffff?
    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub unk3: Vec<u8>,

    // TODO: padding?
    pub unk: [u32; 4],
}

#[binread]
#[derive(Debug, Xc3Write, Xc3WriteOffsets)]
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
    #[br(parse_with = parse_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub unk2: [u32; 40],

    // TODO: type?
    #[br(parse_with = parse_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub unk3: [u16; 4],

    // TODO: padding?
    pub unk: [u32; 4],
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
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
        self.unk2.write_full(writer, base_offset, data_ptr)?;
        self.unk5.write_full(writer, base_offset, data_ptr)?;
        self.unk6.write_full(writer, base_offset, data_ptr)?;
        self.textures.write_full(writer, base_offset, data_ptr)?;

        Ok(())
    }
}
