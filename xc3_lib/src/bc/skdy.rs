use crate::{parse_ptr64, parse_string_ptr64};
use binrw::{binread, BinRead};
use xc3_write::{Xc3Write, Xc3WriteOffsets};

use super::BcList;

// skeleton dynamics?
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(magic(b"SKDY"))]
#[xc3(magic(b"SKDY"))]
pub struct Skdy {
    #[br(parse_with = parse_ptr64)]
    #[xc3(offset(u64))]
    pub dynamics: Dynamics,
}

// TODO: All names should be written at the end.
#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct Dynamics {
    pub unk1: BcList<()>,
    pub unk2: u64,

    #[br(temp, restore_position)]
    offset: u32,

    #[br(parse_with = parse_ptr64)]
    #[xc3(offset(u64))]
    pub unk3: DynamicsUnk1,

    // TODO: not always present?
    #[br(parse_with = parse_ptr64)]
    #[br(if(offset >= 96))]
    #[xc3(offset(u64))]
    pub unk4: Option<DynamicsUnk2>,

    #[br(parse_with = parse_ptr64)]
    #[br(if(offset >= 104))]
    #[xc3(offset(u64))]
    pub unk5: Option<DynamicsUnk3>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct DynamicsUnk1 {
    pub unk1: BcList<DynamicsUnk1Item>,
    // TODO: type?
    pub unk2: BcList<u8>,
    pub unk3: BcList<u8>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct DynamicsUnk1Item {
    pub unk1: u32,
    pub unk2: i32,

    #[br(parse_with = parse_string_ptr64)]
    #[xc3(offset(u64))]
    pub name1: String,

    // TODO: Shared offset to string + 0xFF?
    #[br(parse_with = parse_string_ptr64)]
    #[xc3(offset(u64))]
    pub name2: String,
    pub unk4: u32,
    pub unk5: i32,

    pub unk6: [f32; 9],
    pub unk7: [i32; 3],
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct DynamicsUnk2 {
    pub unk1: BcList<DynamicsUnk2Item>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct DynamicsUnk2Item {
    #[br(parse_with = parse_string_ptr64)]
    #[xc3(offset(u64))]
    pub name: String,

    pub unk1: BcList<DynamicsUnk2ItemUnk1>,
    pub unk2: BcList<[f32; 4]>,
    pub unk3: BcList<DynamicsUnk2ItemUnk3>,
    pub unk4: BcList<()>,
    pub unk5: BcList<()>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct DynamicsUnk2ItemUnk1 {
    #[br(parse_with = parse_string_ptr64)]
    #[xc3(offset(u64))]
    pub name1: String,

    #[br(parse_with = parse_string_ptr64)]
    #[xc3(offset(u64))]
    pub name2: String,

    pub unk1: [f32; 7],
    pub unk2: u32,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct DynamicsUnk2ItemUnk3 {
    #[br(parse_with = parse_string_ptr64)]
    #[xc3(offset(u64))]
    pub name: String,

    pub unk1: [f32; 7],
    pub unk2: u32,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct DynamicsUnk3 {
    // TODO: points to string section?
    pub unk1: BcList<()>,
}
