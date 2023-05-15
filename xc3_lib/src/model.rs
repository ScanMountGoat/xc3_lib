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
    vertex_buffers: Vec<VertexBuffer>,

    #[br(parse_with = parse_array)]
    index_buffers: Vec<IndexBuffer>,

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

    data_base_offset: u32,

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
    data_offset: u32,
    vertex_count: u32,
    vertex_size: u32,

    // Corresponds to attributes in vertex shader?
    #[br(parse_with = parse_array)]
    attributes: Vec<VertexAttribute>,

    // padding?
    unk1: u32,
    unk2: u32,
    unk3: u32,
}

#[binread]
#[derive(Debug, Serialize)]
pub struct VertexAttribute {
    data_type: DataType,
    data_size: u16,
}

#[binread]
#[derive(Debug, Serialize)]
#[br(repr(u16))]
pub enum DataType {
    Position = 0,    // f32x3
    WeightIndex = 3, // u32
    Unk4 = 4,
    Uv1 = 5, // f32x2
    Uv2 = 6,
    Uv3 = 7,
    Unk14 = 14,
    VertexColor = 17, // u8x4
    Normal = 28,      // i8x4?
    Tangent = 29,     // i8x4? tangent?
    Unk32 = 32,
    Unk33 = 33,
    WeightShort = 41,
    BoneId2 = 42,
}

#[binread]
#[derive(Debug, Serialize)]
pub struct IndexBuffer {
    // is this data u16?
    data_offset: u32,
    data_length: u32,
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
