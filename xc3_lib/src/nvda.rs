//! Map data in `.nvda` or `.winvda` files.
//!
//! # File Paths
//! | Game | Versions | File Patterns |
//! | --- | --- | --- |
//! | Xenoblade 1 DE | | `map/*.winvda` |
//! | Xenoblade 2 |  | `map/*.winvda`|
//! | Xenoblade 3 |  | `map/*.nvda` |
//! | Xenoblade X DE | | `map/*.winvda` |
use crate::parse_offset32_count32;
use binrw::{BinRead, binread};
use xc3_write::{Xc3Write, Xc3WriteOffsets};

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(magic(b"NVDA"))]
#[xc3(magic(b"NVDA"))]
pub struct Nvda {
    pub unk1: u32,       // 4115 (xc3), 4160 (xc1, xc2, xcxde)
    pub unk2: [u32; 14], // TODO: padding?

    // TODO: parse entries until end of file
    // TODO: nvpt for xcx de and xbc1 for other games?
    pub entry: Nvpt,
}

#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(magic(b"NVPT"))]
#[xc3(magic(b"NVPT"))]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct Nvpt {
    // Subtract the magic size.
    #[br(temp, try_calc = r.stream_position().map(|p| p - 4))]
    base_offset: u64,

    pub unk1: u32,        // TODO: same number as nvda?
    pub unk2: (u16, u16), // TODO: numbers match the xbc1 names like 9, 9 for 009009?
    pub unk3: [f32; 7],

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub unk4: Vec<[u32; 2]>,
    pub unk5: u32,
    pub unk6: u32,
    pub unk7: u32,
    pub unk8: u32,
    pub unk9: u32,
    pub unk10: u32,
    pub unk11: u32,
    pub unk12: u32,
    pub unk13: u32,
    pub unk14: u32,
    pub unk15: u32,
    pub unk16: [f32; 4],
    // TODO: padding?
}
