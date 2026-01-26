//! Legacy types for Xenoblade Chronicles X.
use binrw::{BinRead, binread};
use xc3_write::{Xc3Write, Xc3WriteOffsets};

use crate::{
    map::Texture,
    mxmd::{
        PackedTextures,
        legacy::{Materials, Models, Unk1, VertexData},
    },
    parse_count32_offset32, parse_offset32_count32, parse_opt_ptr32, parse_ptr32,
    spco::Spco,
};

// TODO: How many of these types are shared with the switch formats?
// TODO: make this work with wii u and not just xcx de.
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
    pub textures: Vec<Texture>,

    pub unk4: u32,
    pub unk5: u32,

    #[br(parse_with = parse_ptr32)]
    #[xc3(offset(u32), align(4096))]
    pub spco: Spco,

    // TODO: offset count?
    // TODO: Texture ids?
    #[br(parse_with = parse_offset32_count32)]
    #[xc3(offset_count(u32, u32))]
    pub low_texture_indices: Vec<u16>,

    #[br(parse_with = parse_ptr32)]
    #[xc3(offset(u32), align(4096))]
    pub unk8: TerrainModelDataUnk8,

    // TODO: padding?
    pub unks: [u32; 7],
}

// TODO: identical to map model groups?
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
    pub items2: Vec<u16>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct TerrainModelDataUnk8Item1 {
    pub max_xyz: [f32; 3],
    pub min_xyz: [f32; 3],
    pub unk1: [u32; 2], // TODO: vertex data index for each lod?
    pub unk3: u32,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct TerrainModelDataUnk8Item2 {
    pub unk1: u16, // TODO: counts up?
    pub unk2: u16,
    pub unk3: u16,
    pub unk4: u16,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct ObjectModelData {
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

    pub unk1: u32, // TODO: Offset?

    #[br(parse_with = parse_ptr32)]
    #[xc3(offset(u32))]
    pub unk2: ObjectModelDataUnk2,

    pub unk3: u32,

    #[br(parse_with = parse_offset32_count32)]
    #[xc3(offset_count(u32, u32))]
    pub unk4: Vec<[u16; 4]>,

    pub unk5: [u32; 2], // TODO: offset count?

    pub unk6: [u32; 4],

    pub unk7: u32,

    #[br(parse_with = parse_ptr32)]
    #[xc3(offset(u32))]
    pub spco: Spco,

    #[br(parse_with = parse_offset32_count32)]
    #[xc3(offset_count(u32, u32))]
    pub unk8: Vec<u16>,

    // TODO: padding?
    pub unks: [u32; 6],
}

#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct ObjectModelDataUnk2 {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    pub unk1: u32,

    #[br(parse_with = parse_count32_offset32, offset = base_offset)]
    #[xc3(count_offset(u32, u32))]
    pub items1: Vec<[u32; 2]>,

    #[br(parse_with = parse_count32_offset32, offset = base_offset)]
    #[xc3(count_offset(u32, u32))]
    pub items2: Vec<ObjectModelDataUnk2Item2>,

    #[br(parse_with = parse_count32_offset32, offset = base_offset)]
    #[xc3(count_offset(u32, u32))]
    pub items3: Vec<[f32; 27]>,

    pub unk2: [u32; 2],

    #[br(parse_with = parse_count32_offset32, offset = base_offset)]
    #[xc3(count_offset(u32, u32))]
    pub items4: Vec<[f32; 8]>,

    pub unk3: [u32; 3],

    pub unk4: u32, // TODO: offset?

    pub unk5: [u32; 4],

    pub unk6: u32, // TODO: offset?

    pub unk7: [u32; 2],

    #[br(parse_with = parse_ptr32)]
    #[br(args { offset: base_offset, inner: base_offset})]
    #[xc3(offset(u32))]
    pub unk8: ObjectModelDataUnk2Unk8,

    // TODO: padding?
    pub unks: [u32; 6],
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct ObjectModelDataUnk2Item2 {
    pub unk1: [f32; 8],
    pub unk2: u32,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct ObjectModelDataUnk2Unk8 {
    #[br(parse_with = parse_offset32_count32)]
    #[br(args { offset: base_offset, inner: base_offset})]
    #[xc3(offset_count(u32, u32))]
    pub unk1: Vec<ObjectModelDataUnk2Unk8Unk1>,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub unk2: Vec<[u32; 4]>, // TODO: type?

    // TODO: padding?
    pub unks: [u32; 4],
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct ObjectModelDataUnk2Unk8Unk1 {
    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub unk1: Vec<[u16; 4]>,
}

// TODO: same as Unk9ModelData?
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct SkyModelData {
    #[br(parse_with = parse_ptr32)]
    #[xc3(offset(u32))]
    pub models: Models,

    #[br(parse_with = parse_ptr32)]
    #[xc3(offset(u32))]
    pub materials: Materials,

    #[br(parse_with = parse_opt_ptr32)]
    #[xc3(offset(u32))]
    pub unk1: Option<Unk1>,

    #[br(parse_with = parse_ptr32)]
    #[xc3(offset(u32), align(4096))]
    pub vertex: VertexData,

    #[br(parse_with = parse_ptr32)]
    #[xc3(offset(u32))]
    pub packed_textures: PackedTextures,

    pub unk3: u32,

    #[br(parse_with = parse_ptr32)]
    #[xc3(offset(u32), align(4096))]
    pub spco: Spco,

    // TODO: padding?
    pub unks: [u32; 9],
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct ObjectStreamData {
    pub unk1: u32,
}
