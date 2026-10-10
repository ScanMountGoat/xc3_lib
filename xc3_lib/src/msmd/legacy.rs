//! Legacy types for Xenoblade Chronicles X.
use binrw::{BinRead, binread};
use xc3_write::{Xc3Write, Xc3WriteOffsets};

use crate::{
    StringOffset32,
    map::legacy::{MapModelData, PropModelData, PropPositions, SkyModelData},
    msmd::LowTextures,
    mtxt::Mtxt,
    mxmd::legacy::VertexData,
    parse_count32_offset32, parse_offset32_count32, parse_opt_ptr32, parse_ptr32,
    parse_string_ptr32,
    spco::Spco,
};

use super::{BoundingBox, Cems, Dlgt, StreamEntry, Texture};

// TODO: make the magic endian dependent to avoid having a legacy type?
/// The main map data for a `.wismhd` file.
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(magic(b"MSMD"))]
#[xc3(magic(b"MSMD"))]
pub struct MsmdLegacy {
    #[br(assert(version == 10011))]
    pub version: u32,
    // TODO: always 0?
    pub unk1: [u32; 4],

    pub inner: MsmdV11Legacy,
}

// TODO: make this generic over mibl vs mtxt?
// TODO: use the same naming conventions as the switch format.
/// The main map data for a `.wismhd` file.
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct MsmdV11 {
    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub map_models: Vec<MapModel<Spco>>,

    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub prop_models: Vec<PropModel<Spco>>,

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

    // TODO: string section for map regions like ma00a?
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

    // TODO: mtxt and different texture usage
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
    pub map_vertex_data: Vec<StreamEntry<VertexData>>,

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

// TODO: share code with xcx and xcx de formats.
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct MsmdV11Legacy {
    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub map_models: Vec<MapModel<crate::mxmd::legacy::Shaders>>,

    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub prop_models: Vec<PropModel<crate::mxmd::legacy::Shaders>>,

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
    pub textures: Vec<TextureLegacy>,

    // TODO: string section for map regions like ma00a?
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
    pub dlgt: u32, // TODO: dlgt with BE magic

    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub dlgts: Vec<DlgtEntry>,

    // TODO: mtxt and different texture usage
    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub low_textures: Vec<StreamEntry<LowTexturesLegacy>>,

    // TODO: referenced by low textures?
    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub terrain_streaming_textures: Vec<StreamEntry<Mtxt>>,

    #[br(parse_with = parse_ptr32)]
    #[xc3(offset(u32))]
    pub unk10: CsvbBlock,

    pub unk11: u32, // TODO: offset?

    #[br(parse_with = parse_opt_ptr32)]
    #[xc3(offset(u32))]
    pub dmcl: Option<u32>, // TODO: Dmcl with BE magic

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
    pub map_vertex_data: Vec<StreamEntry<VertexData>>,

    #[br(parse_with = parse_ptr32)]
    #[xc3(offset(u32))]
    pub unk15: u32, // TODO: Cems with BE magic

    #[br(parse_with = parse_offset32_count32)]
    #[xc3(offset_count(u32, u32))]
    pub unk16: Vec<[f32; 8]>,

    #[br(parse_with = parse_offset32_count32)]
    #[xc3(offset_count(u32, u32))]
    pub unk17: Vec<[u32; 4]>,

    // TODO: padding?
    pub unks: [u32; 8],
}

/// References to medium and high resolution [Mtxt] textures.
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct TextureLegacy {
    pub mid: StreamEntry<Mtxt>,
    // TODO: This isn't always used?
    pub base_mip: StreamEntry<Vec<u8>>,
    pub flags: u32, // TODO: What do these do?
}

// TODO: make this generic over the legacy vs modern instead of separate types?
#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct LowTexturesLegacy {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    #[br(parse_with = parse_count32_offset32, args { offset: base_offset, inner: base_offset })]
    #[xc3(count_offset(u32, u32))]
    pub textures: Vec<LowTextureLegacy>,

    pub unk1: u32, // TODO: some sort of count?
    // TODO: Padding?
    pub unk: [u32; 4],
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct LowTextureLegacy {
    pub usage: u32, //crate::mxmd::legacy::TextureUsage,

    #[br(parse_with = parse_count32_offset32, offset = base_offset)]
    #[xc3(count_offset(u32, u32))]
    pub mtxt_data: Vec<u8>,

    // TODO: determines if a texture is cached or not?
    pub unk2: i32, // TODO: 0 or -1 for xcx maps?
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
pub struct MapModel<S>
where
    S: Xc3Write + 'static,
    for<'a> S: BinRead<Args<'a> = ()>,
    for<'a> S::Offsets<'a>: Xc3WriteOffsets<Args = ()>,
{
    pub bounds: BoundingBox,
    pub unk1: u16,
    pub map_part_id: u16,
    pub unk2: u32,
    pub unk3: u32,
    pub bounds_radius: f32,
    pub entry: StreamEntry<MapModelData<S>>,
    // TODO: padding?
    pub unk4: [u32; 4],
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct Collision {
    pub bounds: BoundingBox,
    pub unk1: [f32; 4],                         // TODO: bounding sphere?
    pub entry: StreamEntry<MapModelData<Spco>>, // TODO: is this really the same type?
    pub unk2: [u32; 3],
    pub unk3: u32,
    pub unk4: [u32; 3],
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct PropModel<S>
where
    S: Xc3Write + 'static,
    for<'a> S: BinRead<Args<'a> = ()>,
    for<'a> S::Offsets<'a>: Xc3WriteOffsets<Args = ()>,
{
    pub bounds: BoundingBox,
    pub unk1: u16,
    pub map_part_id: u16,
    pub unk2: [f32; 3],
    pub entry: StreamEntry<PropModelData<S>>,
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

// TODO: magic depends on endianness
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(magic(b"DMCL"))]
#[xc3(magic(b"DMCL"))]
pub struct Dmcl {
    pub version: u32,
}
