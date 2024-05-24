//! Cutscene data in .beh files.
//!
//! # File Paths
//!
//! | Game | Versions | File Patterns |
//! | --- | --- | --- |
//! | Xenoblade Chronicles 1 DE | |  |
//! | Xenoblade Chronicles 2 |  | |
//! | Xenoblade Chronicles 3 |  | `event/**/*.beh` |
use crate::{parse_ptr32, xc3_write_binwrite_impl};
use binrw::{binread, BinRead, BinWrite};
use xc3_write::{Xc3Write, Xc3WriteOffsets};

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(magic(b"hdev"))]
#[xc3(magic(b"hdev"))]
pub struct Beh {
    pub unk1: u32, // version?
    pub unk2: u32, // TODO: version 1, 2, 3?

    #[br(if(unk2 >= 2))]
    pub unk3: Option<u32>, // string section ptr?

    // TODO: what is the correct check for this?
    #[br(if(unk1 != 0))]
    pub inner: Option<BehInner>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct BehInner {
    #[br(parse_with = parse_ptr32)]
    #[xc3(offset(u32))]
    pub unk4: Unk4, // ptr?
    // TODO: padding?
    pub unks: [u32; 3],
}

#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(stream = r)]
#[br(magic(b"test"))]
#[xc3(magic(b"test"))]
pub struct Unk4 {
    // Subtract the magic size.
    #[br(temp, try_calc = r.stream_position().map(|p| p - 4))]
    base_offset: u64,

    pub unk4_count: u32,
    pub unk2: u32,

    #[br(parse_with = parse_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub unk3: [[f32; 4]; 4],

    #[br(count = unk4_count)]
    pub unk4: Vec<Unk4Unk4>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct Unk4Unk4 {
    pub unk1: u32, // data type?
    pub unk2: u32, // flags?
    pub unk3: u32, // offset?
}

// TODO: is this actually an enum?
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, BinWrite, Clone, Copy, PartialEq, Eq, Hash)]
#[brw(repr(u32))]
pub enum Unk4DataType {
    Unk0 = 0,   // 0 bytes (offset shared with previous)
    Unk1 = 1,   // 16 bytes?
    Unk2 = 2,   // 16 bytes?
    Unk3 = 3,   // 16 bytes?
    Unk4 = 4,   // 16 bytes?
    Unk5 = 5,   // 32 bytes?
    Unk6 = 6,   // 16 bytes?
    Unk8 = 8,   // ???
    Unk10 = 10, // 32 bytes?
    Unk16 = 16, // 16 bytes?
}

xc3_write_binwrite_impl!(Unk4DataType);
