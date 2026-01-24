//! Legacy types for Xenoblade Chronicles X.
use binrw::BinRead;
use xc3_write::{Xc3Write, Xc3WriteOffsets};

use crate::{
    map::legacy::{ObjectModelData, TerrainModelData, Unk9ModelData},
    mibl::Mibl,
    mxmd::legacy::VertexData,
    parse_count32_offset32, parse_ptr32, parse_string_ptr32,
};

use super::{BoundingBox, Cems, Dlgt, StreamEntry, Texture};

// TODO: make this generic over mibl vs mtxt?
// TODO: use the same naming conventions as the switch format.
/// The main map data for a `.wismhd` file.
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(magic(b"DMSM"))]
#[xc3(magic(b"DMSM"))]
pub struct Msmd {
    /// 10111
    pub version: u32,
    // TODO: always 0?
    pub unk1: [u32; 4],

    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub unk2: Vec<TerrainModel>,

    // TODO: objects?
    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub unk3: Vec<ObjectModel>,

    // TODO: collisions?
    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub unk4: Vec<Collision>,

    // TODO: sky models?
    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub unk5: Vec<u32>,

    pub unk6: [u32; 6],

    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub object_streams: Vec<StreamEntry<()>>, // TODO: type?

    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub object_textures: Vec<Texture>, // TODO: type?

    #[br(parse_with = parse_string_ptr32)]
    #[xc3(offset(u32))]
    pub unk_name: String,

    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub grass: Vec<u32>, // TODO: type?

    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub unk7: Vec<StreamEntry<()>>, // TODO: type?

    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub unk8: Vec<StreamEntry<()>>, // TODO: type?

    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub unk_names: Vec<u32>, // TODO: type?

    #[br(parse_with = parse_ptr32)]
    #[xc3(offset(u32))]
    pub dlgt: Dlgt,

    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub unk9: Vec<Unk9Model>, // TODO: type?

    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub terrain_cached_textures: Vec<StreamEntry<Mibl>>,

    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub terrain_streaming_textures: Vec<StreamEntry<()>>, // TODO: type?

    #[br(parse_with = parse_ptr32)]
    #[xc3(offset(u32))]
    pub unk10: CsvbBlock,

    pub unk11: u32,

    pub unk12: [u32; 7],

    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub unk13: Vec<StreamEntry<()>>, // TODO: type?

    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub map_terrain_buffers: Vec<StreamEntry<VertexData>>,

    #[br(parse_with = parse_ptr32)]
    #[xc3(offset(u32))]
    pub unk15: Cems,

    // TODO: padding?
    pub unks: [u32; 12],
}

// TODO: BVSC to consistently use BE for name?
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct CsvbBlock {
    pub hash: u32,

    // #[br(parse_with = parse_offset32_count32)]
    // #[xc3(offset_count(u32, u32))]
    // pub items: [Vec<CsvbBlockItem>],
    pub items: [u32; 2],

    pub unk2: u32,
    pub unk3: u32,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct CsvbBlockItem {
    pub unk1: u32,
    pub unk2: u32,
    // TODO: padding?
    pub unks: [u32; 4],
}

// TODO: how many of these structs are shared with switch?
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct TerrainModel {
    pub bounds: BoundingBox,
    pub unk1: [f32; 4],
    pub entry: StreamEntry<TerrainModelData>, // TODO: type?
    pub unk2: u32,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct Collision {
    pub bounds: BoundingBox,
    pub unk1: [f32; 4],                       // TODO: bounding sphere?
    pub entry: StreamEntry<TerrainModelData>, // TODO: is this really the same type?
    pub unk2: [u32; 3],
    pub unk3: u32,
    pub unk4: [u32; 3],
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct ObjectModel {
    pub bounds: BoundingBox,
    pub unk1: u32,
    pub unk2: [f32; 3],
    pub entry: StreamEntry<ObjectModelData>, // TODO: type?
    pub unk3: u32,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct Unk9Model {
    pub bounds: BoundingBox,
    pub entry: StreamEntry<Unk9ModelData>, // TODO: type?
    pub unk1: [u32; 6],
}
