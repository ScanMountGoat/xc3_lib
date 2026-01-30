//! `.wismhd` files for map data that points to data in a corresponding `.wismda` files
//!
//! # File Paths
//! | Game | Version | File Patterns |
//! | --- | --- | --- |
//! | Xenoblade 1 DE | 10112 | `map/*.wismhd` |
//! | Xenoblade 2 | 10112 |  `map/*.wismhd` |
//! | Xenoblade 3 | 10112 |  `map/*.wismhd` |
//! | Xenoblade X DE | 10011 | `map/*.wismhd` |
use std::{
    io::{Cursor, Read, Seek, SeekFrom},
    marker::PhantomData,
};

use binrw::{BinRead, BinResult, BinWrite, Endian, args, binread};
use xc3_write::{Xc3Write, Xc3WriteOffsets};

use crate::{
    error::DecompressStreamError,
    map::{
        EnvModelData, FoliageModelData, FoliageUnkData, FoliageVertexData, MapLowModelData,
        MapModelData, PropInstance, PropModelData, PropPositions,
    },
    mibl::Mibl,
    msmd::legacy::MsmdV11,
    mxmd::{Mxmd, TextureUsage},
    parse_count32_offset32, parse_offset32_count32, parse_opt_ptr32, parse_ptr32,
    parse_string_opt_ptr32, parse_string_ptr32, parse_vec,
    vertex::VertexData,
    xbc1::Xbc1,
    xc3_write_binwrite_impl,
};

pub mod legacy;

/// The main map data for a `.wismhd` file.
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(magic(b"DMSM"))]
#[xc3(magic(b"DMSM"))]
pub struct Msmd {
    pub version: u32,
    // TODO: always 0?
    pub unk1: [u32; 4],

    #[br(args_raw(version))]
    pub inner: MsmdInner,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(version: u32))]
pub enum MsmdInner {
    #[br(pre_assert(version == 10011))]
    V11(MsmdV11),

    #[br(pre_assert(version == 10112))]
    V112(MsmdV112),
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct MsmdV112 {
    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub map_models: Vec<MapModel>,

    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub prop_models: Vec<PropModel>,

    pub unk1_1: [u32; 2],

    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub env_models: Vec<EnvModel>,

    #[br(parse_with = parse_ptr32)]
    #[xc3(offset(u32))]
    pub wismda_info: WismdaInfo,

    // TODO: streaming file name offset relative to strings_offset?
    pub unk2_1: u32,

    #[br(parse_with = parse_opt_ptr32)]
    #[xc3(offset(u32))]
    pub effects: Option<Effects>,

    pub unk2: [u32; 3],

    /// `.wismda` data with names like `/seamwork/inst/mdl/00003.te`.
    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub prop_vertex_data: Vec<StreamEntry<VertexData>>,

    /// High resolution textures.
    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub textures: Vec<Texture>,

    // TODO: Offset for string table?
    #[br(parse_with = parse_string_opt_ptr32)]
    #[xc3(offset(u32))]
    pub strings_offset: Option<String>,

    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub foliage_models: Vec<FoliageModel>,

    /// `.wismda` data with names like `/seamwork/inst/pos/00000.et`.
    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub prop_positions: Vec<StreamEntry<PropPositions>>,

    /// `.wismda` data with names like `/seamwork/mpfmap/poli//0022`.
    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub foliage_data: Vec<StreamEntry<FoliageVertexData>>,

    #[br(parse_with = parse_opt_ptr32)]
    #[xc3(offset(u32))]
    pub child_models: Option<MapChildModels>,

    pub unk3_2: u32, // TODO: cover data

    #[br(parse_with = parse_ptr32)]
    #[xc3(offset(u32))]
    pub dlgt: Dlgt,

    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub unk_lights: Vec<UnkLight>,

    // low resolution packed textures?
    /// `.wismda` data with names like `/seamwork/texture/00000_wi`.
    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub low_textures: Vec<StreamEntry<LowTextures>>,

    // TODO: Document more of these fields.
    pub unk4: [u32; 6],

    #[br(parse_with = parse_opt_ptr32)]
    #[xc3(offset(u32))]
    pub parts: Option<MapParts>,

    pub unk4_2: u32,

    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub low_models: Vec<MapLowModel>,

    pub env_flags: u32,

    /// `.wismda` data with names like `/seamwork/mpfmap/poli//0000`.
    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub unk_foliage_data: Vec<StreamEntry<FoliageUnkData>>,

    /// `.wismda` data with names like `/seamwork/basemap/poli//000`
    /// or `/seamwork/basemap/poli//001`.
    // TODO: Are all of these referenced by map models?
    // TODO: What references "poli/001"?
    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub map_vertex_data: Vec<StreamEntry<VertexData>>,

    #[br(parse_with = parse_ptr32)]
    #[xc3(offset(u32))]
    #[br(args { inner: env_flags })]
    pub nerd: EnvironmentData,

    pub unk6: [u32; 3],

    #[br(parse_with = parse_ptr32)]
    #[xc3(offset(u32))]
    pub ibl: Ibl,

    #[br(parse_with = parse_opt_ptr32)]
    #[xc3(offset(u32))]
    pub cmld: Option<Cmld>,

    pub unk5_2: u32,
    pub unk5_3: u32,

    #[br(parse_with = parse_opt_ptr32)]
    #[xc3(offset(u32))]
    pub unk5_4: Option<Doce>,

    pub unk5_5: u32,
    pub unk5_6: u32,

    // padding?
    pub unk7: [u32; 8],
}

/// References to medium and high resolution [Mibl] textures.
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct Texture {
    pub mid: StreamEntry<Mibl>,
    // TODO: This isn't always used?
    pub base_mip: StreamEntry<Vec<u8>>,
    pub flags: u32, // TODO: What do these do?
}

// TODO: Better name for this?
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct MapModel {
    pub bounds: BoundingBox,
    // bounding sphere?
    pub unk2: [f32; 4],
    /// `.wismda` data with names like `bina_basefix.temp_wi`.
    pub entry: StreamEntry<MapModelData>,
    pub unk3: [f32; 4],
}

// TODO: Better name for this?
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct PropModel {
    pub bounds: BoundingBox,
    // bounding sphere?
    pub unk2: [f32; 4],
    /// `.wismda` data with names like `/seamwork/inst/out/00000.te`.
    pub entry: StreamEntry<PropModelData>,
    pub unk3: u32,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct EnvModel {
    pub bounds: BoundingBox,
    // bounding sphere?
    pub unk2: [f32; 4],
    /// `.wismda` data with names like `/seamwork/envmap/ma00a/bina`.
    pub entry: StreamEntry<EnvModelData>,
}

// TODO: also in mxmd but without the center?

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct BoundingBox {
    pub max: [f32; 3],
    pub min: [f32; 3],
    pub center: [f32; 3],
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct MapLowModel {
    pub bounds: BoundingBox,
    pub unk1: f32,
    /// `.wismda` data with names like `/seamwork/lowmap/ma11a/bina`.
    pub entry: StreamEntry<MapLowModelData>,
    pub unk2: u16,
    pub unk3: u16,
    // TODO: padding?
    pub unk: [u32; 5],
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct FoliageModel {
    pub unk1: [f32; 9],
    pub unk: [u32; 3],
    pub unk2: f32,
    /// `.wismda` data with names like `/seamwork/mpfmap/ma11a/bina`.
    pub entry: StreamEntry<FoliageModelData>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(flags: u32))]
pub enum EnvironmentData {
    #[br(pre_assert(flags == 0))]
    Cems(Cems),
    #[br(pre_assert(flags == 2))]
    Nerd(Nerd),
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(magic(b"DREN"))]
#[xc3(magic(b"DREN"))]
pub struct Nerd {
    pub version: u32,
    pub unk1: u32,
    pub unk2: u32,
    pub unk3: u32,
    pub unk4: u32,
    pub unk5: u32,
    // padding?
    pub unk6: [u32; 6],
}

// TODO: This contains a Nerd?

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(magic(b"SMEC"))]
#[xc3(magic(b"SMEC"))]
pub struct Cems {
    pub unk1: [u32; 10],
    pub offset: u32,
}

// TODO: cloud data?
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(magic(b"CMLD"))]
#[xc3(magic(b"CMLD"))]
pub struct Cmld {
    pub version: u32,
}

// TODO: Lighting data?
// TODO: .wilgt files?
#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(magic(b"DLGT"))]
#[xc3(magic(b"DLGT"))]
#[xc3(base_offset)]
#[br(stream = r)]
pub struct Dlgt {
    // Subtract the magic size.
    #[br(temp, try_calc = r.stream_position().map(|p| p - 4))]
    base_offset: u64,

    pub version: u32, // 10008

    #[br(parse_with = parse_ptr32, args { offset: base_offset, inner: base_offset})]
    #[xc3(offset(u32))]
    pub light_data: LightData,

    #[br(parse_with = parse_ptr32, args { offset: base_offset, inner: base_offset})]
    #[xc3(offset(u32))]
    pub light_instance_data: LightInstanceData,

    pub unk3: u32,

    #[br(parse_with = parse_ptr32, args { offset: base_offset, inner: base_offset})]
    #[xc3(offset(u32))]
    pub fog_data: LightFogData,

    pub unk4: u32,

    #[br(parse_with = parse_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub time_data: LightTimeData,

    pub animation_data: u32,

    #[br(parse_with = parse_ptr32, args { offset: base_offset, inner: base_offset})]
    #[xc3(offset(u32))]
    pub zone_data: LightZoneData,

    pub flags: u32, // TODO: Bit flags
    // TODO: Check flags
    // #[br(parse_with = parse_ptr32, args { offset: base_offset, inner: base_offset})]
    // #[xc3(offset(u32))]
    pub clip_group_data: u32,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct LightData {
    // TODO: why does count need to be multiplied by 4?
    #[br(parse_with = parse_count32_offset32, args { offset: base_offset, inner: base_offset })]
    #[xc3(count_offset(u32, u32))]
    pub lights: Vec<[Light; 4]>,

    pub textures_offset: u32,
    pub textures_count: u32,
    pub ambient_light_count: u32,
    pub directional_light_count: u32,
    pub point_light_count: u32,
    pub spot_light_count: u32,
    pub unk1: u32,
    pub unk2: u32,
    pub animation_data: u32, // TODO: offset
    pub local_ibl_light_count: u32,
}

#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct Light {
    #[br(temp, restore_position)]
    offset_ty: [u32; 2],

    // TODO: data type depends on ty?
    // #[br(parse_with = parse_ptr32, args { offset: base_offset, inner: offset_ty[1] })]
    // #[xc3(offset(u32))]
    // pub params: LightParam,
    pub params_offset: u32,

    pub ty: u32,
    pub flags: u32,
    pub color: [f32; 3],
    pub intensity: f32,
    pub unk1: f32,
    pub unk2: u16,
    pub group_id: u16,
    pub falloff: f32,
    pub animation_index: u16,
    pub animation_constant_start_index: u16,
    pub time_group_index: u16,
    pub zone_group_index: u16,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(ty: u32))]
pub enum LightParam {
    #[br(pre_assert(ty == 0))]
    Unk0([u32; 11]),

    #[br(pre_assert(ty == 1))]
    Unk1(LightParamUnk1),
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct LightParamUnk1 {
    pub unk1: u32,
    pub unk2: [f32; 4],
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct LightInstanceData {
    #[br(parse_with = parse_count32_offset32, offset = base_offset)]
    #[xc3(count_offset(u32, u32))]
    pub instances: Vec<LightInstance>,

    #[br(parse_with = parse_count32_offset32, args { offset: base_offset, inner: base_offset })]
    #[xc3(count_offset(u32, u32))]
    pub tree_nodes: Vec<LightTreeNode>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct LightInstance {
    pub world_matrix: [[f32; 4]; 4],
    pub bounds_center: [f32; 3],
    pub bounds_radius: f32,
    pub fade_position: [f32; 3],
    pub fade_distance: f32,
    pub clip_group_index_plus_one: u16,
    pub map_part_id: u16,
    // TODO: padding?
    pub unks: [u32; 3],
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct LightTreeNode {
    pub bounds_max: [f32; 3],
    pub bounds_min: [f32; 3],
    pub child_node_indices_offset: u32,
    pub light_indices_offset: u32,
    pub child_node_indices_count: u16,
    pub light_indices_count: u16,

    // TODO: better handling of offset + count
    #[br(parse_with = parse_offset_count(base_offset + child_node_indices_offset as u64, child_node_indices_count as usize))]
    child_node_indices: Vec<u32>,

    #[br(parse_with = parse_offset_count(base_offset + light_indices_offset as u64, light_indices_count as usize))]
    light_indices: Vec<u32>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct LightFogData {
    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub fogs: Vec<Fog>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct Fog {
    pub color: [f32; 3],
    pub horizon_color: [f32; 3],
    pub sun_color: [f32; 3],
    pub near: f32,
    pub far: f32,
    pub density: f32,
    pub falloff: f32,
    pub horizon_falloff: f32,
    pub sun_falloff: f32,
    pub god_ray_strength: f32,
    pub god_ray_falloff: f32,
    pub animation_index: u16,
    pub time_group_index: u16,
    pub flags: u16,
    pub ty: u16,
    pub zone_group_index: u16,
    pub sky_intensity: f32,
    pub unk1: u32,
    // TODO: padding?
    pub unk2: u32,
}

#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct LightTimeData {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    #[br(parse_with = parse_count32_offset32, args { offset: base_offset, inner: base_offset })]
    #[xc3(count_offset(u32, u32))]
    pub groups: Vec<TimeGroup>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct TimeGroup {
    pub weather_bitmap: u32,

    #[br(parse_with = parse_count32_offset32, args { offset: base_offset, inner: base_offset })]
    #[xc3(count_offset(u32, u32))]
    pub weathers: Vec<TimeWeather>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct TimeWeather {
    pub weather_bitmap: u32,

    // TODO: how to select light or fog interval?
    #[br(parse_with = parse_count32_offset32, offset = base_offset)]
    #[xc3(count_offset(u32, u32))]
    pub intervals: Vec<TimeWeatherIntervalLight>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct TimeWeatherIntervalLight {
    pub color: [f32; 3],
    pub intensity: f32,
    pub shadow_intensity: f32,
    pub blend_time: f32,
    pub start_time: f32,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct TimeWeatherIntervalFog {
    pub color: [f32; 3],
    pub horizon_color: [f32; 3],
    pub sun_color: [f32; 3],
    pub near: f32,
    pub far: f32,
    pub horizon_falloff: f32,
    pub sun_falloff: f32,
    pub god_ray_strength: f32,
    pub god_ray_falloff: f32,
    pub blend_time: f32,
    pub start_time: f32,
    pub density: f32,
    pub sky_density: f32,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct LightZoneData {
    #[br(parse_with = parse_count32_offset32, args { offset: base_offset, inner: base_offset })]
    #[xc3(count_offset(u32, u32))]
    pub groups: Vec<ZoneGroup>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct ZoneGroup {
    #[br(parse_with = parse_count32_offset32, offset = base_offset)]
    #[xc3(count_offset(u32, u32))]
    pub light_alternatives: Vec<ZoneLightAlternative>,

    pub zone_id: u32,
    // TODO: padding?
    pub unks: [u32; 2],
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct ZoneLightAlternative {
    pub zone_id: u16,
    pub light_index: u16,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct ClipGroupData {
    #[br(parse_with = parse_offset32_count32, args { offset: base_offset, inner: base_offset })]
    #[xc3(offset_count(u32, u32))]
    pub groups: Vec<ClipGroup>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct ClipGroup {
    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub items: Vec<ClipGroupItem>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct ClipGroupItem {
    pub clip_volume_group_index: u16,
    pub ty: u16,
    pub priority: u16,
    pub unk: u16,
}

#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct Ibl {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    #[br(parse_with = parse_count32_offset32, args { offset: base_offset, inner: base_offset })]
    #[xc3(count_offset(u32, u32))]
    pub unk1: Vec<IblInner>,

    pub unk3: u32,
    pub unk4: u32,
    pub unk5: u32,
    pub unk6: u32,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct IblInner {
    pub unk1: u32, // 0?

    #[br(parse_with = parse_string_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub map_name: String,

    #[br(parse_with = parse_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub gibl: Gibl,

    pub unk4: u32, // gibl section length?
    // padding?
    pub unk5: [u32; 6],
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(magic(b"GIBL"))]
#[xc3(magic(b"GIBL"))]
pub struct Gibl {
    pub unk1: u32,
    pub unk2: u32,
    pub unk3: u32,
    pub unk4: u32, // offset to mibl?
    pub unk5: u32,
    // TODO: padding?
    pub unk6: [u32; 6],
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct WismdaInfo {
    pub compressed_length: u32,
    pub unk1: u32,
    pub decompressed_length: u32,
    pub streaming_buffer_length: u32,
    pub unks: [u32; 50],
}

#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct Effects {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    #[br(parse_with = parse_count32_offset32, offset = base_offset)]
    #[xc3(count_offset(u32, u32))]
    pub unk1: Vec<Effect>,

    pub unk3: u32,
}

#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct Effect {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    #[br(parse_with = parse_string_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub unk1: String,

    // TODO: xc2 has a string here instead?
    pub transform_count: u32,
    pub transform_offset: u32,

    pub unk4: u32,
    pub unk5: u32,
    pub unk6: f32,
    pub unk7: f32,
    pub unk8: f32,
    pub unk9: f32,
    pub unk10: u32,
    pub unk11: u32,
    pub unk12: u32,
    pub unk13: u32,
    pub unk14: u32,
    pub unk15: u32,
    pub unk16: u32,
}

// TODO: What does this do?
// 116 bytes including magic?
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(magic(b"DOCE"))]
#[xc3(magic(b"DOCE"))]
pub struct Doce {
    pub version: u32,
    pub offset: u32,
    pub count: u32,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct LowTextures {
    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub textures: Vec<LowTexture>,
    // TODO: Padding?
    pub unk: [u32; 5],
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct LowTexture {
    pub usage: TextureUsage,
    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub mibl_data: Vec<u8>,
    pub unk2: i32, // TODO: always -1?
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct UnkLight {
    pub max: [f32; 3],
    pub min: [f32; 3],
    /// `.wismda` data with names like `/seamwork/lgt/bina/00000.wi`.
    pub entry: StreamEntry<Dlgt>,
    pub unk3: u32,
    // TODO: padding?
    pub unk4: [u32; 5],
}

// TODO: How to get writing working?
#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct MapParts {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    // TODO: Where do static parts index?
    #[br(parse_with = parse_offset32_count32, args { offset: base_offset, inner: base_offset })]
    #[xc3(offset_count(u32, u32))]
    pub parts: Vec<MapPart>,

    pub unk_count: u32,

    // TODO: How to handle this for writing?
    #[br(temp)]
    animated_parts_offset: u32,

    #[br(temp)]
    instance_animations_offset: u32,

    pub unk2: u32,

    #[br(temp)]
    instance_animations_count: u32,

    // TODO: Find a cleaner way of handling these offsets.
    #[br(seek_before = std::io::SeekFrom::Start(base_offset + animated_parts_offset as u64))]
    #[br(args { count: instance_animations_count as usize })]
    #[br(restore_position)]
    pub animated_instances: Vec<PropInstance>,

    #[br(seek_before = std::io::SeekFrom::Start(base_offset + instance_animations_offset as u64))]
    #[br(args { count: instance_animations_count as usize, inner: base_offset })]
    #[br(restore_position)]
    pub instance_animations: Vec<MapPartInstanceAnimation>,

    pub unk4: u32,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub unk5: Vec<[u32; 3]>, // TODO: Offset after map parts?

    pub unk6: u32,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub transforms: Vec<[[f32; 4]; 4]>,

    pub unks: [u32; 3],
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct MapPartInstanceAnimation {
    pub translation: [f32; 3],
    pub rotation: [f32; 3],
    pub scale: [f32; 3],
    pub unk1: u32,
    pub unk2: u32,
    pub unk3: u32,
    pub flags: u32,

    #[br(parse_with = parse_offset32_count32, args { offset: base_offset, inner: base_offset })]
    #[xc3(offset_count(u32, u32))]
    pub channels: Vec<MapPartInstanceAnimationChannel>,

    pub time_min: u16,
    pub time_max: u16,
    // TODO: padding?
    pub unks: [u32; 5],
}

// TODO: Derive xc3write?
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct MapPartInstanceAnimationChannel {
    // TODO: Group this together into a single type?
    pub keyframes_offset: u32,
    pub channel_type: ChannelType,
    pub keyframe_count: u16,

    pub time_min: u16,
    pub time_max: u16,

    // TODO: Write offset?
    #[br(seek_before = std::io::SeekFrom::Start(base_offset + keyframes_offset as u64))]
    #[br(count = keyframe_count as usize)]
    #[br(restore_position)]
    pub keyframes: Vec<MapPartInstanceAnimationKeyframe>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, BinWrite, PartialEq, Clone)]
#[brw(repr(u16))]
pub enum ChannelType {
    TranslationX = 0,
    TranslationY = 1,
    TranslationZ = 2,
    RotationX = 3,
    RotationY = 4,
    RotationZ = 5,
    ScaleX = 6,
    ScaleY = 7,
    ScaleZ = 8,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct MapPartInstanceAnimationKeyframe {
    pub slope_out: f32,
    pub slope_in: f32,
    pub value: f32,
    pub time: u16,
    pub flags: u16,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct MapPart {
    #[br(parse_with = parse_string_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub name: String,

    // TODO: The index of the instance in PropLods.instances?
    pub instance_index: u32,

    // TODO: matches with PropInstance part id?
    // TODO: Multiple MapPart can have the same ID?
    pub part_id: u16,

    pub flags: u16,
    pub animation_start: u8,
    pub animation_speed: u8,

    /// The transform from [transforms](struct.MapParts.html#structfield.transforms).
    pub transform_index: u16,

    pub node_animation_index: u16,
    pub instance_animation_index: u16,
    pub switch_group_index: u16,
    pub unk: u16,
}

#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct MapChildModels {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    #[br(parse_with = parse_offset32_count32, args { offset: base_offset, inner: base_offset })]
    #[xc3(offset_count(u32, u32))]
    pub models: Vec<MapChildModel>,

    pub instance_count: u32,

    #[br(parse_with = parse_ptr32)]
    #[br(args { offset: base_offset, inner: args! { count: instance_count as usize } })]
    #[xc3(offset(u32))]
    pub instance_transforms: Vec<[[f32; 4]; 4]>,

    #[br(parse_with = parse_ptr32)]
    #[br(args { offset: base_offset, inner: args! { count: instance_count as usize } })]
    #[xc3(offset(u32))]
    pub instances: Vec<MapChildModelInstance>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct MapChildModel {
    #[br(parse_with = parse_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub mxmd: Mxmd,

    #[br(parse_with = parse_string_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub streaming_file_name: String,

    pub instances_start_index: u32,
    pub instances_count: u32,
    pub cull_distance: f32,
    // TODO: padding?
    pub unk: [u32; 2],
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct MapChildModelInstance {
    pub flags: u32,
    pub map_part_id1: u32,
    pub map_part_id2: u32,
}

/// A reference to an [Xbc1] in the `.wismda` file.
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, BinWrite, PartialEq, Clone)]
pub struct StreamEntry<T> {
    /// The offset of the [Xbc1] in the `.wismda` file.
    pub offset: u32,
    pub decompressed_size: u32,
    #[bw(ignore)]
    phantom: PhantomData<T>,
}

impl<T> StreamEntry<T> {
    /// Decompress the data from a reader for a `.wismda` file.
    pub fn decompress<R: Read + Seek>(
        &self,
        wismda: &mut R,
        is_compressed: bool,
    ) -> Result<Vec<u8>, DecompressStreamError> {
        // Not all wismda files use XBC1 archives to store data.
        wismda.seek(SeekFrom::Start(self.offset as u64))?;
        if is_compressed {
            let bytes = Xbc1::read(wismda)?.decompress()?;
            Ok(bytes)
        } else {
            let mut bytes = vec![0u8; self.decompressed_size as usize];
            wismda.read_exact(&mut bytes)?;
            Ok(bytes)
        }
    }
}

impl<T> StreamEntry<T>
where
    for<'a> T: BinRead<Args<'a> = ()>,
{
    /// Decompress and read the data from a reader for a `.wismda` file.
    #[tracing::instrument(skip_all)]
    pub fn extract<R: Read + Seek>(
        &self,
        wismda: &mut R,
        is_compressed: bool,
    ) -> Result<T, DecompressStreamError> {
        let bytes = self.decompress(wismda, is_compressed)?;
        T::read_le(&mut Cursor::new(bytes)).map_err(Into::into)
    }
}

// TODO: Find a way to derive this?
impl<T> Xc3Write for StreamEntry<T> {
    type Offsets<'a>
        = ()
    where
        T: 'a;

    fn xc3_write<W: std::io::Write + Seek>(
        &self,
        writer: &mut W,
        endian: xc3_write::Endian,
    ) -> xc3_write::Xc3Result<Self::Offsets<'_>> {
        let endian = match endian {
            xc3_write::Endian::Little => binrw::Endian::Little,
            xc3_write::Endian::Big => binrw::Endian::Big,
        };
        self.write_options(writer, endian, ())
            .map_err(std::io::Error::other)?;
        Ok(())
    }
}

xc3_write_binwrite_impl!(ChannelType);

fn parse_offset_count<R, T>(
    offset: u64,
    count: usize,
) -> impl Fn(&mut R, Endian, ()) -> BinResult<Vec<T>>
where
    T: 'static,
    for<'a> T: BinRead<Args<'a> = ()>,
    R: Read + Seek,
{
    // Needed for offset tracking.
    move |reader, endian, _args| parse_vec(reader, endian, Default::default(), offset, count)
}
