//! Map data stored in compressed sections in `.wismda` files.
//!
//! Many of these sections use the same formats as character models.

use binrw::binread;

use crate::{
    mxmd::{Materials, Models},
    parse_count_offset, parse_offset_count, parse_ptr32, parse_string_ptr32,
    spch::Spch,
    vertex::VertexData,
};

// TODO: Improve docs.
// TODO: Link to appropriate stream field with doc links.
/// The data for a [PropModel](crate::msmd::PropModel).
#[binread]
#[derive(Debug)]
pub struct PropModelData {
    pub unk1: [u32; 3],

    /// Each model has a corresponding element in [vertex_data_indices](#structfield.vertex_data_indices).
    #[br(parse_with = parse_ptr32)]
    pub models: Models,

    #[br(parse_with = parse_ptr32)]
    pub materials: Materials,

    unk2: u32,

    // Is this the actual props in the scene?
    #[br(parse_with = parse_ptr32)]
    pub lods: PropLods,

    unk3: u32,

    /// The textures referenced by [materials](#structfield.materials).
    #[br(parse_with = parse_offset_count)]
    pub textures: Vec<Texture>,

    /// The index of the [VertexData](crate::vertex::VertexData)
    /// in [prop_vertex_data](../msmd/struct.Msmd.html#structfield.prop_vertex_data)
    /// for each of the models in [models](#structfield.models).
    #[br(parse_with = parse_offset_count)]
    pub model_vertex_data_indices: Vec<u32>,

    unk4: [u32; 5],

    #[br(parse_with = parse_ptr32)]
    pub spch: Spch,

    unk5: u32,
    unk6: u32,
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

    unk1: u32,

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
    count2: u32,
    offset2: u32,

    // render tree nodes?
    #[br(parse_with = parse_count_offset, offset = base_offset)]
    pub unk3: Vec<PropUnk3>,

    unk2: u32,

    unks: [u32; 10],

    // TODO: indices into animated map parts in msmd that then index into props?
    pub animated_parts_start_index: u32,
    pub animated_parts_count: u32,
    // TODO: indices into map parts in msmd?
    pub static_parts_start_index: u32,
    pub static_parts_count: u32,
}

#[binread]
#[derive(Debug)]
#[br(stream = r)]
pub struct PropLod {
    // TODO: Do these actually index into the PropModelLod?
    /// The index of the base LOD (highest quality) [Model](crate::mxmd::Model)
    /// in [models](struct.PropModelData.html#structfield.models).
    pub base_lod_index: u32,
    /// The number of LOD models with higher indices having lower quality.
    pub lod_count: u32,
}

#[binread]
#[derive(Debug)]
pub struct PropModelLod {
    radius: f32,
    distance: f32,
    // TODO: Index into PropModelData.models?
    index: u32,
}

#[binread]
#[derive(Debug)]
pub struct PropInstance {
    /// The transform of the instance as a 4x4 column-major matrix.
    pub transform: [[f32; 4]; 4],
    position: [f32; 3],
    radius: f32,
    center: [f32; 3],

    // TODO: fix this doc link
    /// The index into [props](struct.PropLods.html#structfield.props).
    pub prop_index: u32,

    unk1: u16,

    // TODO: part_id of MapPart?
    // TODO: Does a value of 0 indicate no parent MapPart?
    pub part_id: u16,

    unk3: u16,
    unk4: u16,
    // TODO: padding?
    unks: [u32; 2],
}

#[binread]
#[derive(Debug)]
pub struct PropUnk3 {
    unk1: [f32; 5],
    unk2: [u32; 3],
}

// TODO: Link to appropriate stream field with doc links.
/// The data for a [MapModel](crate::msmd::MapModel).
#[binread]
#[derive(Debug)]
pub struct MapModelData {
    unk1: [u32; 3],

    #[br(parse_with = parse_ptr32)]
    pub models: Models,

    #[br(parse_with = parse_ptr32)]
    pub materials: Materials,

    m_unk2: [u32; 2],

    /// The textures referenced by [materials](#structfield.materials).
    #[br(parse_with = parse_offset_count)]
    pub textures: Vec<Texture>,

    m_unk3: [u32; 2],

    #[br(parse_with = parse_ptr32)]
    pub spch: Spch,

    // TODO: What does this do?
    low_res_offset: u32,
    low_res_count: u32,

    #[br(parse_with = parse_ptr32)]
    pub groups: MapModelGroups,
    // padding?
}

// TODO: Shared with other formats?
#[binread]
#[derive(Debug)]
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

    /// The index of the [VertexData](crate::vertex::VertexData)
    /// in [map_vertex_data](../msmd/struct.Msmd.html#structfield.map_vertex_data)
    /// for each of the models in [models](struct.MapModelData.html#structfield.models).
    #[br(parse_with = parse_offset_count, offset = base_offset)]
    pub model_vertex_data_indices: Vec<u16>,
}

// Groups?
#[binread]
#[derive(Debug)]
pub struct MapModelGroup {
    max: [f32; 3],
    min: [f32; 3],

    /// The index of the [VertexData](crate::vertex::VertexData)
    /// in [map_vertex_data](../msmd/struct.Msmd.html#structfield.map_vertex_data).
    pub vertex_data_index: u32,
    // TODO: lod vertex data index?
    // TODO: This is also used in indices?
    pub unk_vertex_data_index: u32,
    unk3: u32,
}

// TODO: Link to appropriate fields with doc links.
/// The data for a [EnvModel](crate::msmd::EnvModel).
#[binread]
#[derive(Debug)]
pub struct EnvModelData {
    #[br(parse_with = parse_ptr32)]
    pub models: Models,

    #[br(parse_with = parse_ptr32)]
    pub materials: Materials,

    // TODO: Pointers to MIBL files?
    unk_offset1: u32,

    #[br(parse_with = parse_ptr32)]
    pub vertex_data: VertexData,

    #[br(parse_with = parse_ptr32)]
    pub textures: PackedTextures,

    // TODO: always 0?
    unk6: u32,

    #[br(parse_with = parse_ptr32)]
    pub spch: Spch,
    // padding?
}

// TODO: Shared with Mxmd?
#[binread]
#[derive(Debug)]
#[br(stream = r)]
pub struct PackedTextures {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    #[br(parse_with = parse_count_offset, args { offset: base_offset, inner: base_offset })]
    pub textures: Vec<TextureItem>,

    unk2: u32,
    strings_offset: u32,
}

#[binread]
#[derive(Debug)]
#[br(import_raw(base_offset: u64))]
pub struct TextureItem {
    unk1: u32,

    // TODO: Optimized function for reading bytes?
    #[br(parse_with = parse_count_offset, offset = base_offset)]
    pub mibl_data: Vec<u8>,

    #[br(parse_with = parse_string_ptr32, offset = base_offset)]
    pub name: String,
}

// TODO: Link to appropriate fields with doc links.
/// The data for a [FoliageModel](crate::msmd::FoliageModel).
#[binread]
#[derive(Debug)]
pub struct FoliageModelData {
    #[br(parse_with = parse_ptr32)]
    pub models: Models,

    #[br(parse_with = parse_ptr32)]
    pub materials: FoliageMaterials,

    unk1: u32,

    #[br(parse_with = parse_ptr32)]
    pub vertex_data: VertexData,

    #[br(parse_with = parse_ptr32)]
    pub textures: PackedTextures,

    unk4: [u32; 11], // padding?
}

#[binread]
#[derive(Debug)]
#[br(stream = r)]
pub struct FoliageMaterials {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    #[br(parse_with = parse_offset_count, args { offset: base_offset, inner: base_offset })]
    pub materials: Vec<FoliageMaterial>,

    unk1: u32,
    unk2: u32,
    unk3: u32,
    unk4: u32,
    unk5: u32,
}

#[binread]
#[derive(Debug)]
#[br(import_raw(base_offset: u64))]
pub struct FoliageMaterial {
    #[br(parse_with = parse_string_ptr32, offset = base_offset)]
    pub name: String,

    unk1: u16,
    unk2: u16,
    unk3: u16,
    unk4: u16,
    unk5: u16,
    unk6: u16,
    unk7: u16,
    unk8: u16,
    unk9: u16,
    unk10: u16,
    unk11: u16,
    unk12: u16,
    unk13: u16,
    unk14: u16,
}

#[binread]
#[derive(Debug)]
pub struct FoliageVertexData {
    #[br(parse_with = parse_count_offset)]
    unk1: Vec<FoliageVertex1>,
    #[br(parse_with = parse_count_offset)]
    unk2: Vec<FoliageVertex2>,
    unk3: u32,
    // TODO: padding?
    unks: [u32; 7],
}

#[binread]
#[derive(Debug)]
pub struct FoliageVertex1 {
    unk1: (f32, f32, f32),
    unk2: [u8; 4],
}

#[binread]
#[derive(Debug)]
pub struct FoliageVertex2 {
    unk1: (f32, f32, f32, f32),
    unk2: u32, // offset?
    unk3: u32, // offset?
    unk4: u32,
    unk5: u32,
}

#[binread]
#[derive(Debug)]
pub struct FoliageUnkData {
    unk1: [u32; 9], // length of the file repeated?
    unk2: [f32; 4],
    // TODO: padding?
    unk3: [u32; 8],
}

/// The data for a [MapLowModel](crate::msmd::MapLowModel).
#[binread]
#[derive(Debug)]
pub struct MapLowModelData {
    unk1: u32,
    unk2: u32,

    #[br(parse_with = parse_ptr32)]
    pub models: Models,

    #[br(parse_with = parse_ptr32)]
    pub materials: Materials,

    unk5: u32,
    unk6: u32,

    #[br(parse_with = parse_ptr32)]
    pub vertex_data: VertexData,

    unk8: u32,

    #[br(parse_with = parse_ptr32)]
    pub spch: Spch,
    // TODO: more fields?
}

// TODO: Is this documented correctly?
// TODO: https://github.com/atnavon/xc2f/wiki/map-instance-chunk#extrainstancepack
#[binread]
#[derive(Debug)]
pub struct PropPositions {
    #[br(parse_with = parse_count_offset)]
    pub instances: Vec<PropInstance>,
    unk1: u32,
    unk2: u32,
    #[br(parse_with = parse_count_offset)]
    nodes: Vec<RenderNode>,
    unk3: u32,
    unk4: u32,
    unk5: u32,
    animated_parts_start_index: u32,
    animated_parts_count: u32,
    tree_offset: u32,
    unk6: u32,
    // TODO: more fields?
}

#[binread]
#[derive(Debug)]
pub struct RenderNode {
    center: [f32; 3],
    radius: f32,
    unk1: f32,
    unk2: u32,
    unk3: u32,
    unk4: u32,
}
