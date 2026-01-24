//! Legacy types for Xenoblade Chronicles X.
use binrw::{BinRead, binread};
use xc3_write::{Xc3Write, Xc3WriteOffsets};

use crate::{
    mxmd::legacy::{Materials, Models},
    parse_offset32_count32, parse_ptr32,
    spco::Spco,
};

// TODO: How many of these types are shared with the switch formats?
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct TerrainModelData {
    // TODO: flags?
    pub unks_1: u32,
    pub unks_2: u32,
    pub unks_3: u32,

    #[br(parse_with = parse_ptr32)]
    #[xc3(offset(u32))]
    pub models: Models,

    #[br(parse_with = parse_ptr32)]
    #[xc3(offset(u32))]
    pub materials: Materials,

    pub unk1: u32,

    // TODO: offset?
    pub unk2: u32,

    #[br(parse_with = parse_offset32_count32)]
    #[xc3(offset_count(u32, u32))]
    pub unk3: Vec<[u16; 4]>,

    pub unk4: u32,
    pub unk5: u32,

    #[br(parse_with = parse_ptr32)]
    #[xc3(offset(u32), align(4096))]
    pub spco: Spco,

    // TODO: offset count?
    pub unk6: u32,
    pub unk7: u32,

    #[br(parse_with = parse_ptr32)]
    #[xc3(offset(u32), align(4096))]
    pub unk8: TerrainModelDataUnk8,

    // TODO: padding?
    pub unks: [u32; 7],
}

#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct TerrainModelDataUnk8 {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub items1: Vec<TerrainModelDataUnk8Item1>,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub items2: Vec<[u16; 4]>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct TerrainModelDataUnk8Item1 {
    pub max: [f32; 3],
    pub min: [f32; 3],
    pub unk1: u32,
    pub unk2: u32,
    pub unk3: u32,
}
