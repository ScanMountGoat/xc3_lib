//! Map data header in `.nvhe` or `.winvhe` files.
//!
//! # File Paths
//! | Game | Versions | File Patterns |
//! | --- | --- | --- |
//! | Xenoblade 1 DE | | `map/*.winvhe` |
//! | Xenoblade 2 |  | `map/*.winvhe`|
//! | Xenoblade 3 |  | `map/*.nvhe` |
//! | Xenoblade X DE | | `map/*.winvhe` |
use crate::parse_offset32_count32;
use binrw::BinRead;
use xc3_write::{Xc3Write, Xc3WriteOffsets};

// TODO: xcx wii u format is similar
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(magic(b"NVMS"))]
#[xc3(magic(b"NVMS"))]
pub struct Nvms {
    pub unk1: u32, // 4115 (xcxde)

    #[br(parse_with = parse_offset32_count32)]
    #[xc3(offset_count(u32, u32))]
    pub items1: Vec<NvmsEntry>,

    #[br(parse_with = parse_offset32_count32)]
    #[xc3(offset_count(u32, u32))]
    pub items2: Vec<NvmsItem2>,

    #[br(parse_with = parse_offset32_count32)]
    #[xc3(offset_count(u32, u32))]
    pub items3: Vec<NvmsItem3>,

    // TODO: padding?
    pub unks: [u32; 8],
}

// TODO: why does each entry refer to 2 NVPT?
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct NvmsEntry {
    pub unk1: u32,
    /// The offset in bytes for the data in the [Nvda](crate::nvda::Nvda).
    pub offset1: u32,
    /// The total length in bytes for both data items in the [Nvda](crate::nvda::Nvda).
    pub length: u32,
    /// The offset in bytes for the data in the [Nvda](crate::nvda::Nvda).
    pub offset2: u32,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct NvmsItem2 {
    pub unk1: u32,
    pub unk2: u32,
    pub unk3: [f32; 7],

    #[br(parse_with = parse_offset32_count32)]
    #[xc3(offset_count(u32, u32))]
    pub unk4: Vec<u16>,

    pub unk6: u32,
    pub unk7: u32, // offset?

    // TODO: padding?
    pub unks: [u32; 3],
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct NvmsItem3 {
    pub unk1: u32,
    pub unk2: u32,

    // TODO: padding?
    pub unks: [u32; 4],
}
