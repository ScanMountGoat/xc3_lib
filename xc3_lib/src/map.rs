//! Map data stored in compressed sections in `.wismda` files.
//!
//! Many of these sections use the same formats as character models.

use binrw::{binread, BinRead};

use crate::{
    mxmd::{Materials, Models, PackedTextures},
    parse_count_offset, parse_offset_count, parse_ptr32, parse_string_ptr32,
    spch::Spch,
    vertex::VertexData,
};

// TODO: Improve docs.
// TODO: Link to appropriate stream field with doc links.
/// The data for a [PropModel](crate::msmd::PropModel).

#[derive(BinRead, Debug)]
pub struct PropModelData {
    pub unk1: [u32; 3],

    /// Each model has a corresponding element in [vertex_data_indices](#structfield.vertex_data_indices).
    #[br(parse_with = parse_ptr32)]
    pub models: Models,

    #[br(parse_with = parse_ptr32)]
    pub materials: Materials,

    pub unk2: u32,

    // Is this the actual props in the scene?
    #[br(parse_with = parse_ptr32)]
    pub lods: PropLods,

    pub unk3: u32,

    /// The textures referenced by [materials](#structfield.materials).
    #[br(parse_with = parse_offset_count)]
    pub textures: Vec<Texture>,

    /// The index of the [VertexData](crate::vertex::VertexData)
    /// in [prop_vertex_data](../msmd/struct.Msmd.html#structfield.prop_vertex_data)
    /// for each of the models in [models](#structfield.models).
    #[br(parse_with = parse_offset_count)]
    pub model_vertex_data_indices: Vec<u32>,

    pub unk4: [u32; 5],

    #[br(parse_with = parse_ptr32)]
    pub spch: Spch,

    pub unk5: u32,
    pub unk6: u32,
    // 16 bytes of padding?
}

// Similar to LOD data in mxmd?
// TODO: Better names for these types
#[binread]
#[derive(Debug)]
#[br(stream = r)]
pub struct PropLods {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    pub unk1: u32,

    // model groups?
    // Each of these is a single prop with all of its lods?
    #[br(parse_with = parse_count_offset, offset = base_offset)]
    pub props: Vec<PropLod>,

    #[br(parse_with = parse_count_offset, offset = base_offset)]
    pub lods: Vec<PropModelLod>,

    /// Instance information for [props](#structfield.props).
    #[br(parse_with = parse_count_offset, offset = base_offset)]
    pub instances: Vec<PropInstance>,

    // render tree node indices?
    pub count2: u32,
    pub offset2: u32,

    // render tree nodes?
    #[br(parse_with = parse_count_offset, offset = base_offset)]
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

#[derive(BinRead, Debug)]
#[br(stream = r)]
pub struct PropLod {
    // TODO: Do these actually index into the PropModelLod?
    /// The index of the base LOD (highest quality) [Model](crate::mxmd::Model)
    /// in [models](struct.PropModelData.html#structfield.models).
    pub base_lod_index: u32,
    /// The number of LOD models with higher indices having lower quality.
    pub lod_count: u32,
}

#[derive(BinRead, Debug)]
pub struct PropModelLod {
    pub radius: f32,
    pub distance: f32,
    // TODO: Index into PropModelData.models?
    pub index: u32,
}

#[derive(BinRead, Debug)]
pub struct PropInstance {
    /// The transform of the instance as a 4x4 column-major matrix.
    pub transform: [[f32; 4]; 4],
    pub position: [f32; 3],
    pub radius: f32,
    pub center: [f32; 3],

    // TODO: fix this doc link
    /// The index into [props](struct.PropLods.html#structfield.props).
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

#[derive(BinRead, Debug)]
pub struct PropUnk3 {
    pub unk1: [f32; 5],
    pub unk2: [u32; 3],
}

// TODO: Link to appropriate stream field with doc links.
/// The data for a [MapModel](crate::msmd::MapModel).

#[derive(BinRead, Debug)]
pub struct MapModelData {
    pub unk1: [u32; 3],

    #[br(parse_with = parse_ptr32)]
    pub models: Models,

    #[br(parse_with = parse_ptr32)]
    pub materials: Materials,

    pub m_unk2: [u32; 2],

    /// The textures referenced by [materials](#structfield.materials).
    #[br(parse_with = parse_offset_count)]
    pub textures: Vec<Texture>,

    pub m_unk3: [u32; 2],

    #[br(parse_with = parse_ptr32)]
    pub spch: Spch,

    // TODO: What does this do?
    pub low_res_offset: u32,
    pub low_res_count: u32,

    #[br(parse_with = parse_ptr32)]
    pub groups: MapModelGroups,
    // padding?
}

// TODO: Shared with other formats?

#[derive(BinRead, Debug)]
pub struct Texture {
    // TODO: What do these index into?
    pub low_texture_index: i16,
    pub low_texture_container_index: i16,
    pub texture_index: i16, // index into texture list in msmd?
    pub texture_type: u16,
}

// TODO: What to call this?
#[binread]
#[derive(Debug)]
#[br(stream = r)]
pub struct MapModelGroups {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    #[br(parse_with = parse_offset_count, offset = base_offset)]
    pub groups: Vec<MapModelGroup>,

    /// The index of the [MapModelGroup] in [groups](#structfield.groups)
    /// for each of the models in [models](struct.MapModelData.html#structfield.models).
    #[br(parse_with = parse_offset_count, offset = base_offset)]
    pub model_group_index: Vec<u16>,
}

// Groups?

#[derive(BinRead, Debug)]
pub struct MapModelGroup {
    pub max: [f32; 3],
    pub min: [f32; 3],

    /// The index of the [VertexData](crate::vertex::VertexData)
    /// in [map_vertex_data](../msmd/struct.Msmd.html#structfield.map_vertex_data).
    pub vertex_data_index: u32,
    // TODO: lod vertex data index?
    // TODO: This is also used in indices?
    pub unk_vertex_data_index: u32,
    pub unk3: u32,
}

// TODO: Link to appropriate fields with doc links.
/// The data for a [EnvModel](crate::msmd::EnvModel).

#[derive(BinRead, Debug)]
pub struct EnvModelData {
    #[br(parse_with = parse_ptr32)]
    pub models: Models,

    #[br(parse_with = parse_ptr32)]
    pub materials: Materials,

    // TODO: Pointers to MIBL files?
    pub unk_offset1: u32,

    #[br(parse_with = parse_ptr32)]
    pub vertex_data: VertexData,

    #[br(parse_with = parse_ptr32)]
    pub textures: PackedTextures,

    // TODO: always 0?
    pub unk6: u32,

    #[br(parse_with = parse_ptr32)]
    pub spch: Spch,
    // padding?
}

// TODO: Link to appropriate fields with doc links.
/// The data for a [FoliageModel](crate::msmd::FoliageModel).

#[derive(BinRead, Debug)]
pub struct FoliageModelData {
    #[br(parse_with = parse_ptr32)]
    pub models: Models,

    #[br(parse_with = parse_ptr32)]
    pub materials: FoliageMaterials,

    pub unk1: u32,

    #[br(parse_with = parse_ptr32)]
    pub vertex_data: VertexData,

    #[br(parse_with = parse_ptr32)]
    pub textures: PackedTextures,

    pub unk4: [u32; 11], // padding?
}

#[binread]
#[derive(Debug)]
#[br(stream = r)]
pub struct FoliageMaterials {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    #[br(parse_with = parse_offset_count, args { offset: base_offset, inner: base_offset })]
    pub materials: Vec<FoliageMaterial>,

    pub unk1: u32,
    pub unk2: u32,
    pub unk3: u32,
    pub unk4: u32,
    pub unk5: u32,
}

#[derive(BinRead, Debug)]
#[br(import_raw(base_offset: u64))]
pub struct FoliageMaterial {
    #[br(parse_with = parse_string_ptr32, offset = base_offset)]
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

#[derive(BinRead, Debug)]
pub struct FoliageVertexData {
    #[br(parse_with = parse_count_offset)]
    pub unk1: Vec<FoliageVertex1>,
    #[br(parse_with = parse_count_offset)]
    pub unk2: Vec<FoliageVertex2>,
    pub unk3: u32,
    // TODO: padding?
    pub unks: [u32; 7],
}

#[derive(BinRead, Debug)]
pub struct FoliageVertex1 {
    pub unk1: (f32, f32, f32),
    pub unk2: [u8; 4],
}

#[derive(BinRead, Debug)]
pub struct FoliageVertex2 {
    pub unk1: (f32, f32, f32, f32),
    pub unk2: u32, // offset?
    pub unk3: u32, // offset?
    pub unk4: u32,
    pub unk5: u32,
}

#[derive(BinRead, Debug)]
pub struct FoliageUnkData {
    pub unk1: [u32; 9], // length of the file repeated?
    pub unk2: [f32; 4],
    // TODO: padding?
    pub unk3: [u32; 8],
}

/// The data for a [MapLowModel](crate::msmd::MapLowModel).

#[derive(BinRead, Debug)]
pub struct MapLowModelData {
    pub unk1: u32,
    pub unk2: u32,

    #[br(parse_with = parse_ptr32)]
    pub models: Models,

    #[br(parse_with = parse_ptr32)]
    pub materials: Materials,

    pub unk5: u32,
    pub unk6: u32,

    #[br(parse_with = parse_ptr32)]
    pub vertex_data: VertexData,

    pub unk8: u32,

    #[br(parse_with = parse_ptr32)]
    pub spch: Spch,
    // TODO: more fields?
}

// TODO: Is this documented correctly?
// TODO: https://github.com/atnavon/xc2f/wiki/map-instance-chunk#extrainstancepack

#[derive(BinRead, Debug)]
pub struct PropPositions {
    #[br(parse_with = parse_count_offset)]
    pub instances: Vec<PropInstance>,
    pub unk1: u32,
    pub unk2: u32,
    #[br(parse_with = parse_count_offset)]
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

#[derive(BinRead, Debug)]
pub struct RenderNode {
    pub center: [f32; 3],
    pub radius: f32,
    pub unk1: f32,
    pub unk2: u32,
    pub unk3: u32,
    pub unk4: u32,
}
