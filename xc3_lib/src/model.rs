use crate::parse_array;
use binrw::{args, binread, FilePtr32};
use serde::Serialize;

// wismt model data
// are these buffers referenced by wimdo?
// TODO: what to call this?
#[binread]
#[derive(Debug, Serialize)]
pub struct ModelData {
    #[br(parse_with = parse_array)]
    pub vertex_buffers: Vec<VertexBuffer>,

    #[br(parse_with = parse_array)]
    pub index_buffers: Vec<IndexBuffer>,

    // padding?
    unk0: u32,
    unk1: u32,
    unk2: u32,

    // 144 bytes of data?
    unk_offset0: u32,

    // 332 bytes of data?
    unk_offset1: u32,
    unk4: u32,

    morph_offset: u32,

    unk5: u32,

    pub data_base_offset: u32,

    unk6: u32,

    #[br(parse_with = FilePtr32::parse)]
    weights: Weights,

    unk7: u32,
    // padding?
}

// vertex buffer?
#[binread]
#[derive(Debug, Serialize)]
pub struct VertexBuffer {
    pub data_offset: u32,
    pub vertex_count: u32,
    pub vertex_size: u32,

    // Corresponds to attributes in vertex shader?
    #[br(parse_with = parse_array)]
    pub attributes: Vec<VertexAttribute>,

    // padding?
    unk1: u32,
    unk2: u32,
    unk3: u32,
}

#[binread]
#[derive(Debug, Serialize)]
pub struct VertexAttribute {
    pub data_type: DataType,
    pub data_size: u16,
}

#[binread]
#[derive(Debug, Serialize)]
#[br(repr(u16))]
pub enum DataType {
    /// Float32x3 position.
    Position = 0,
    WeightIndex = 3, // bone indices?
    Unk4 = 4,
    Uv1 = 5, // f32x2
    Uv2 = 6,
    Uv3 = 7,
    Unk14 = 14,
    /// Unorm8x4 vertex RGBA color.
    VertexColor = 17,
    /// Snorm8x4 normal vector.
    Normal = 28,
    /// Snorm8x4 tangent vector with bitangent sign in the fourth component.
    Tangent = 29,
    Unk32 = 32,
    Unk33 = 33,
    WeightShort = 41,
    // 4 bytes with each byte being used separately by vertex shader?
    // one of the bytes selects some sort of group and
    // one byte selects bones within a group?
    // only the least significant byte matters?
    BoneId2 = 42,
}

#[binread]
#[derive(Debug, Serialize)]
pub struct IndexBuffer {
    // TODO: Is this data always u16?
    pub data_offset: u32,
    pub index_count: u32,
    // padding?
    unk1: u32,
    unk2: u32,
    unk3: u32,
}

// TODO: How are weights assigned to vertices?
// TODO: Skinning happens in the vertex shader?
// TODO: Where are the skin weights in the vertex shader?
#[binread]
#[derive(Debug, Serialize)]
pub struct Weights {
    #[br(temp)]
    count: u32,

    // TODO: Find an easier way to write this?
    #[br(parse_with = FilePtr32::parse)]
    #[br(args { inner: args! { count: count as usize } })]
    weights: Vec<Weight>,

    unk1: u32,
    unk2: u32, // offset to something?
    unk3: u32,
    unks4: [u32; 4], // padding?
}

// 40 bytes?
#[binread]
#[derive(Debug, Serialize)]
pub struct Weight {
    // offsets are just the sum of the previous counts?
    unk1: u32, // offset?
    unk2: u32, // offset?
    unk3: u32, // count?
    unks: [u32; 7],
}

// TODO: functions for accessing data.
