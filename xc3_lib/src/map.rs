//! Map streaming data in `.wismda` files.
//!
//! Many of these sections use the same formats as character models.
//! `.wismda` files serve a similar purpose as `.wismt` files used for characters.
//! Unlike the [Msrd](crate::msrd::Msrd) in most model `.wismt` files, `.wismda` files do not contain a header
//! and can only be parsed by looking at the [StreamEntry](crate::msmd::StreamEntry)
//! for the [Msmd](crate::msmd::Msmd) in the corresponding `.wismhd` file.
//!
//! # File Paths
//! | Game | File Patterns |
//! | --- | --- |
//! | Xenoblade 1 DE | `map/*.wismda` |
//! | Xenoblade 2 | `map/*.wismda` |
//! | Xenoblade 3 | `map/*.wismda` |
//! | Xenoblade X DE | `map/*.wismda` |
use bilge::prelude::*;
use binrw::{binread, BinRead, BinWrite};
use xc3_write::{Xc3Write, Xc3WriteOffsets};

use crate::{
    mxmd::{Materials, Models, PackedTextures},
    parse_count32_offset32, parse_offset32_count32, parse_ptr32, parse_string_ptr32,
    spch::Spch,
    vertex::VertexData,
    xc3_write_binwrite_impl,
};

// TODO: Improve docs.
// TODO: Link to appropriate stream field with doc links.
/// The data for a [PropModel](crate::msmd::PropModel).
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct PropModelData {
    pub unk1: [u32; 3],

    /// Each model has a corresponding element in [vertex_data_indices](#structfield.vertex_data_indices).
    #[br(parse_with = parse_ptr32)]
    #[xc3(offset(u32))]
    pub models: Models,

    #[br(parse_with = parse_ptr32)]
    #[xc3(offset(u32))]
    pub materials: Materials,

    pub unk2: u32,

    // Is this the actual props in the scene?
    #[br(parse_with = parse_ptr32)]
    #[xc3(offset(u32))]
    pub lods: PropLods,

    pub unk3: u32,

    /// The textures referenced by [materials](#structfield.materials).
    #[br(parse_with = parse_offset32_count32)]
    #[xc3(offset_count(u32, u32))]
    pub textures: Vec<Texture>,

    /// The index of the [VertexData]
    /// in [prop_vertex_data](../msmd/struct.Msmd.html#structfield.prop_vertex_data)
    /// for each of the models in [models](#structfield.models).
    #[br(parse_with = parse_offset32_count32)]
    #[xc3(offset_count(u32, u32))]
    pub model_vertex_data_indices: Vec<u32>,

    pub unk4_1: u32,
    pub unk4_2: u32,

    #[br(parse_with = parse_offset32_count32)]
    #[xc3(offset_count(u32, u32))]
    pub prop_info: Vec<PropPositionInfo>,

    pub unk4_5: u32,

    #[br(parse_with = parse_ptr32)]
    #[xc3(offset(u32))]
    pub spch: Spch,

    /// Indices into [low_textures](../msmd/struct.Msmd.html#structfield.low_textures)
    /// for the entry indices in [textures](#structfield.textures).
    #[br(parse_with = parse_offset32_count32)]
    #[xc3(offset_count(u32, u32))]
    pub low_texture_entry_indices: Vec<u16>,
    // TODO: 16 bytes of padding?
}

// Similar to LOD data in mxmd?
// TODO: Better names for these types
#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct PropLods {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    pub unk1: u32,

    // model groups?
    // Each of these is a single prop with all of its lods?
    #[br(parse_with = parse_count32_offset32, offset = base_offset)]
    #[xc3(count_offset(u32, u32))]
    pub props: Vec<PropLod>,

    #[br(parse_with = parse_count32_offset32, offset = base_offset)]
    #[xc3(count_offset(u32, u32))]
    pub lods: Vec<PropModelLod>,

    /// Instance information for [props](#structfield.props).
    #[br(parse_with = parse_count32_offset32, offset = base_offset)]
    #[xc3(count_offset(u32, u32))]
    pub instances: Vec<PropInstance>,

    // render tree node indices?
    pub count2: u32,
    pub offset2: u32,

    // render tree nodes?
    #[br(parse_with = parse_count32_offset32, offset = base_offset)]
    #[xc3(count_offset(u32, u32))]
    pub unk3: Vec<PropUnk3>,

    pub unk2: u32,

    pub unks: [u32; 10],

    // TODO: indices into animated map parts in msmd that then index into props?
    pub animated_parts_start_index: u32,
    pub animated_parts_count: u32,
    // TODO: indices into map parts in msmd?
    pub static_parts_start_index: u32,
    pub static_parts_count: u32,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct PropLod {
    // TODO: Do these actually index into the PropModelLod?
    /// The index of the base LOD (highest quality) [Model](crate::mxmd::Model)
    /// in [models](struct.PropModelData.html#structfield.models).
    /// Only the first 28 bits (0xFFFFFFF) contain the actual index.
    pub base_lod_index: u32,
    /// The number of LOD models with higher indices having lower quality.
    pub lod_count: u32,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct PropModelLod {
    pub radius: f32,
    pub distance: f32,
    // TODO: Index into PropModelData.models?
    pub index: u32,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct PropInstance {
    /// The transform of the instance as a 4x4 column-major matrix.
    pub transform: [[f32; 4]; 4],
    pub position: [f32; 3],
    pub radius: f32,
    pub center: [f32; 3],

    /// Index into [props](struct.PropLods.html#structfield.props).
    pub prop_index: u32,

    pub unk1: u16,

    // TODO: part_id of MapPart?
    // TODO: Does a value of 0 indicate no parent MapPart?
    pub part_id: u16,

    pub unk3: u16,
    pub unk4: u16,
    // TODO: padding?
    pub unks: [u32; 2],
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct PropUnk3 {
    pub unk1: [f32; 5],
    pub unk2: [u32; 3],
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct PropPositionInfo {
    /// Index into [prop_positions](../msmd/struct.Msmd.html#structfield.prop_positions).
    pub prop_position_entry_index: u32,
    pub instance_start_index: u32,
    pub max_xyz: [f32; 3],
    pub min_xyz: [f32; 3],
}

// TODO: Link to appropriate stream field with doc links.
/// The data for a [MapModel](crate::msmd::MapModel).
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct MapModelData {
    pub unk1: [u32; 3],

    #[br(parse_with = parse_ptr32)]
    #[xc3(offset(u32))]
    pub models: Models,

    #[br(parse_with = parse_ptr32)]
    #[xc3(offset(u32))]
    pub materials: Materials,

    pub m_unk2: [u32; 2],

    /// The textures referenced by [materials](#structfield.materials).
    #[br(parse_with = parse_offset32_count32)]
    #[xc3(offset_count(u32, u32))]
    pub textures: Vec<Texture>,

    pub m_unk3: [u32; 2],

    #[br(parse_with = parse_ptr32)]
    #[xc3(offset(u32))]
    pub spch: Spch,

    /// Indices into [low_textures](../msmd/struct.Msmd.html#structfield.low_textures)
    /// for the entry indices in [textures](#structfield.textures).
    #[br(parse_with = parse_offset32_count32)]
    #[xc3(offset_count(u32, u32))]
    pub low_texture_entry_indices: Vec<u16>,

    #[br(parse_with = parse_ptr32)]
    #[xc3(offset(u32))]
    pub groups: MapModelGroups,
    // padding?
}

// TODO: Shared with other formats?
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct Texture {
    /// Index into [textures](../msmd/struct.LowTextures.html#structfield.textures)
    /// for the corresponding [LowTextures](crate::msmd::LowTextures).
    pub low_texture_index: i16,
    /// Index into an additional index list in the model data
    /// that indexes into [low_textures](../msmd/struct.Msmd.html#structfield.low_textures).
    pub low_textures_entry_index: i16,
    /// Index into [textures](../msmd/struct.Msmd.html#structfield.textures).
    pub texture_index: i16,
    pub flags: TextureFlags,
}

#[bitsize(16)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(DebugBits, FromBits, BinRead, BinWrite, PartialEq, Clone, Copy)]
#[br(map = u16::into)]
#[bw(map = |&x| u16::from(x))]
pub struct TextureFlags {
    pub has_low_texture: bool,
    pub has_high_texture: bool,
    pub unk: u14,
}

// TODO: What to call this?
#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct MapModelGroups {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub groups: Vec<MapModelGroup>,

    /// The index of the [MapModelGroup] in [groups](#structfield.groups)
    /// for each of the models in [models](struct.MapModelData.html#structfield.models).
    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub model_group_index: Vec<u16>,
}

// Groups?
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct MapModelGroup {
    pub max_xyz: [f32; 3],
    pub min_xyz: [f32; 3],

    /// The index of the [VertexData] in [map_vertex_data](../msmd/struct.Msmd.html#structfield.map_vertex_data).
    pub vertex_data_index: u32,
    // TODO: lod vertex data index?
    // TODO: This is also used in indices?
    pub unk_vertex_data_index: u32,
    pub unk3: u32,
}

// TODO: Link to appropriate fields with doc links.
/// The data for a [EnvModel](crate::msmd::EnvModel).
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct EnvModelData {
    #[br(parse_with = parse_ptr32)]
    #[xc3(offset(u32))]
    pub models: Models,

    #[br(parse_with = parse_ptr32)]
    #[xc3(offset(u32))]
    pub materials: Materials,

    // TODO: Pointers to MIBL files?
    pub unk_offset1: u32,

    #[br(parse_with = parse_ptr32)]
    #[xc3(offset(u32))]
    pub vertex_data: VertexData,

    #[br(parse_with = parse_ptr32)]
    #[xc3(offset(u32))]
    pub textures: PackedTextures,

    // TODO: always 0?
    pub unk6: u32,

    #[br(parse_with = parse_ptr32)]
    #[xc3(offset(u32))]
    pub spch: Spch,
    // padding?
}

// TODO: Link to appropriate fields with doc links.
/// The data for a [FoliageModel](crate::msmd::FoliageModel).
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct FoliageModelData {
    #[br(parse_with = parse_ptr32)]
    #[xc3(offset(u32))]
    pub models: Models,

    #[br(parse_with = parse_ptr32)]
    #[xc3(offset(u32))]
    pub materials: FoliageMaterials,

    pub unk1: u32,

    #[br(parse_with = parse_ptr32)]
    #[xc3(offset(u32))]
    pub vertex_data: VertexData,

    #[br(parse_with = parse_ptr32)]
    #[xc3(offset(u32))]
    pub textures: PackedTextures,

    pub unk4: [u32; 11], // padding?
}

#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct FoliageMaterials {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    #[br(parse_with = parse_offset32_count32, args { offset: base_offset, inner: base_offset })]
    #[xc3(offset_count(u32, u32))]
    pub materials: Vec<FoliageMaterial>,

    pub unk1: u32,
    pub unk2: u32,
    pub unk3: u32,
    pub unk4: u32,
    pub unk5: u32,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct FoliageMaterial {
    #[br(parse_with = parse_string_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub name: String,

    pub unk1: u16,
    pub unk2: u16,
    pub unk3: u16,
    pub unk4: u16,
    pub unk5: u16,
    pub unk6: u16,
    pub unk7: u16,
    pub unk8: u16,
    pub unk9: u16,
    pub unk10: u16,
    pub unk11: u16,
    pub unk12: u16,
    pub unk13: u16,
    pub unk14: u16,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct FoliageVertexData {
    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub unk1: Vec<FoliageVertex1>,

    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub unk2: Vec<FoliageVertex2>,

    pub unk3: u32,
    // TODO: padding?
    pub unks: [u32; 7],
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct FoliageVertex1 {
    pub unk1: [f32; 3],
    pub unk2: [u8; 4],
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct FoliageVertex2 {
    pub unk1: [f32; 4],
    pub unk2: u32, // offset?
    pub unk3: u32, // offset?
    pub unk4: u32,
    pub unk5: u32,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct FoliageUnkData {
    pub unk1: [u32; 9], // length of the file repeated?
    pub unk2: [f32; 4],
    // TODO: padding?
    pub unk3: [u32; 8],
}

/// The data for a [MapLowModel](crate::msmd::MapLowModel).

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct MapLowModelData {
    pub unk1: u32,
    pub unk2: u32,

    #[br(parse_with = parse_ptr32)]
    #[xc3(offset(u32))]
    pub models: Models,

    #[br(parse_with = parse_ptr32)]
    #[xc3(offset(u32))]
    pub materials: Materials,

    pub unk5: u32,
    pub unk6: u32,

    #[br(parse_with = parse_ptr32)]
    #[xc3(offset(u32))]
    pub vertex_data: VertexData,

    pub unk8: u32,

    #[br(parse_with = parse_ptr32)]
    #[xc3(offset(u32))]
    pub spch: Spch,
    // TODO: more fields?
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct PropPositions {
    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub instances: Vec<PropInstance>,

    pub unk1: u32,
    pub unk2: u32,

    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub nodes: Vec<RenderNode>,

    pub unk3: u32,
    pub unk4: u32,
    pub unk5: u32,
    pub animated_parts_start_index: u32,
    pub animated_parts_count: u32,
    pub tree_offset: u32,
    pub unk6: u32,
    // TODO: more fields?
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct RenderNode {
    pub center: [f32; 3],
    pub radius: f32,
    pub unk1: f32,
    pub unk2: u32,
    pub unk3: u32,
    pub unk4: u32,
}

xc3_write_binwrite_impl!(TextureFlags);
