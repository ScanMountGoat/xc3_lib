//! Legacy types for Xenoblade Chronicles X.
use binrw::BinRead;
use xc3_write::{Xc3Write, Xc3WriteOffsets};

use crate::{
    StringOffset32,
    map::legacy::{MapModelData, PropModelData, PropPositions, SkyModelData},
    msmd::LowTextures,
    mxmd::legacy::VertexData,
    parse_count32_offset32, parse_offset32_count32, parse_opt_ptr32, parse_ptr32,
    parse_string_ptr32,
};

use super::{BoundingBox, Cems, Dlgt, StreamEntry, Texture};

// TODO: make this generic over mibl vs mtxt?
// TODO: use the same naming conventions as the switch format.
/// The main map data for a `.wismhd` file.
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct MsmdV11 {
    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub map_models: Vec<MapModel>,

    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub prop_models: Vec<PropModel>,

    // TODO: collisions?
    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub unk4: Vec<Collision>,

    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub env_models: Vec<EnvModel>,

    pub unk6: [u32; 6],

    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub prop_vertex_data: Vec<StreamEntry<VertexData>>,

    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub textures: Vec<Texture>,

    #[br(parse_with = parse_string_ptr32)]
    #[xc3(offset(u32))]
    pub unk_name: String,

    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub grass: Vec<u32>, // TODO: type?

    // TODO: prop positions?
    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub prop_positions: Vec<StreamEntry<PropPositions>>, // TODO: type?

    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub unk8: Vec<StreamEntry<()>>, // TODO: type?

    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub unk_names: Vec<StringOffset32>,

    #[br(parse_with = parse_ptr32)]
    #[xc3(offset(u32))]
    pub dlgt: Dlgt,

    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub dlgts: Vec<DlgtEntry>,

    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub low_textures: Vec<StreamEntry<LowTextures>>,

    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub terrain_streaming_textures: Vec<StreamEntry<()>>, // TODO: type?

    #[br(parse_with = parse_ptr32)]
    #[xc3(offset(u32))]
    pub unk10: CsvbBlock,

    pub unk11: u32, // TODO: offset?

    #[br(parse_with = parse_opt_ptr32)]
    #[xc3(offset(u32))]
    pub dmcl: Option<Dmcl>,

    pub unk12: u32, // TODO: size of dmcl in bytes?

    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub effects: Vec<StreamEntry<()>>, // TODO: type?

    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub terrain_lod_models: Vec<TerrainLodModel>,

    pub unk12_1: u32,

    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub unk13: Vec<StreamEntry<()>>, // TODO: type?

    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub map_terrain_buffers: Vec<StreamEntry<VertexData>>,

    #[br(parse_with = parse_ptr32)]
    #[xc3(offset(u32))]
    pub unk15: Cems,

    #[br(parse_with = parse_offset32_count32)]
    #[xc3(offset_count(u32, u32))]
    pub unk16: Vec<[f32; 8]>,

    #[br(parse_with = parse_offset32_count32)]
    #[xc3(offset_count(u32, u32))]
    pub unk17: Vec<[u32; 4]>,

    // TODO: padding?
    pub unks: [u32; 8],
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
pub struct MapModel {
    pub bounds: BoundingBox,
    pub unk1: [f32; 4],
    pub entry: StreamEntry<MapModelData>,
    // TODO: padding?
    pub unk2: [u32; 4],
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct Collision {
    pub bounds: BoundingBox,
    pub unk1: [f32; 4],                   // TODO: bounding sphere?
    pub entry: StreamEntry<MapModelData>, // TODO: is this really the same type?
    pub unk2: [u32; 3],
    pub unk3: u32,
    pub unk4: [u32; 3],
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct PropModel {
    pub bounds: BoundingBox,
    pub unk1: u32,
    pub unk2: [f32; 3],
    pub entry: StreamEntry<PropModelData>,
    pub unk3: u32,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct DlgtEntry {
    pub max: [f32; 3],
    pub min: [f32; 3],
    pub entry: StreamEntry<Dlgt>,
    pub unk1: [u32; 6],
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct EnvModel {
    pub bounds: BoundingBox,
    pub unk1: [f32; 4],
    pub entry: StreamEntry<SkyModelData>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct TerrainLodModel {
    pub bounds: BoundingBox,
    pub unk1: f32,
    pub entry: StreamEntry<()>, // TODO: type?
    pub unk2: [u32; 2],
    pub unk3: [f32; 4],
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(magic(b"DMCL"))]
#[xc3(magic(b"DMCL"))]
pub struct Dmcl {
    pub version: u32,
}
