use crate::{parse_count_offset, parse_offset_count, parse_offset_count2, parse_ptr32};
use binrw::{args, binread, FilePtr32};
use serde::Serialize;

/// Vertex and vertex index buffer data used by a [Model](crate::mxmd::Model).
#[binread]
#[derive(Debug, Serialize)]
#[br(stream = r)]
pub struct VertexData {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    #[br(parse_with = parse_offset_count2, args_raw(base_offset))]
    pub vertex_buffers: Vec<VertexBufferDescriptor>,

    #[br(parse_with = parse_offset_count, args_raw(base_offset))]
    pub index_buffers: Vec<IndexBufferDescriptor>,

    // padding?
    unk0: u32,
    unk1: u32,
    unk2: u32,

    // 144 bytes of data?
    unk_offset0: u32,

    // 332 bytes of data?
    unk_offset1: u32,
    unk4: u32,

    #[br(parse_with = parse_ptr32, args_raw(base_offset))]
    pub vertex_animation: Option<VertexAnimation>,

    #[br(temp)]
    buffer_length: u32,

    /// The data buffer containing all the geometry data.
    #[br(parse_with = FilePtr32::parse, args {
        offset: base_offset,
        inner: args! { count: buffer_length as usize }
    })]
    pub buffer: Vec<u8>,

    unk6: u32,

    #[br(parse_with = FilePtr32::parse, offset = base_offset)]
    pub weights: Weights,

    unk7: u32,
    // padding?
}

#[binread]
#[derive(Debug, Serialize)]
#[br(import_raw(base_offset: u64))]
pub struct VertexBufferDescriptor {
    /// The offset into [buffer](struct.VertexData.html#structfield.buffer).
    pub data_offset: u32,
    pub vertex_count: u32,
    /// The size or stride of the vertex in bytes.
    pub vertex_size: u32,

    // Corresponds to attributes in vertex shader?
    #[br(parse_with = parse_offset_count, args_raw(base_offset))]
    pub attributes: Vec<VertexAttribute>,

    // TODO: padding?
    pub unk1: u32,
    pub unk2: u32,
    pub unk3: u32,
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
    Unk1 = 1,
    Unk2 = 2,
    WeightIndex = 3, // bone indices?
    Unk4 = 4,
    /// Float32x2 UV coordinates.
    Uv1 = 5,
    Uv2 = 6,
    Uv3 = 7,
    Uv4 = 8,
    Unk14 = 14,
    /// Unorm8x4 vertex RGBA color.
    VertexColor = 17,
    /// Snorm8x4 normal vector.
    Normal = 28,
    /// Snorm8x4 tangent vector with bitangent sign in the fourth component.
    Tangent = 29,
    /// Snorm8x4 normal vector.
    Normal2 = 32,
    Unk33 = 33,
    WeightShort = 41,
    // 4 bytes with each byte being used separately by vertex shader?
    // one of the bytes selects some sort of group and
    // one byte selects bones within a group?
    // only the least significant byte matters?
    BoneId2 = 42,
    Unk52 = 52,
}

// TODO: Is this data always u16?
#[binread]
#[derive(Debug, Serialize)]
pub struct IndexBufferDescriptor {
    /// The offset into [buffer](struct.VertexData.html#structfield.buffer).
    pub data_offset: u32,
    pub index_count: u32,
    pub unk1: Unk1, // TODO: primitive type?
    pub unk2: Unk2, // TODO: index format?
    // TODO: padding?
    unk3: u32,
    unk4: u32,
}

#[binread]
#[derive(Debug, Serialize)]
#[br(repr(u16))]
pub enum Unk1 {
    Unk0 = 0,
    Unk3 = 3,
}

#[binread]
#[derive(Debug, Serialize)]
#[br(repr(u16))]
pub enum Unk2 {
    Unk0 = 0,
}

/// Vertex animation data often called "vertex morphs", "shape keys", or "blend shapes".
#[binread]
#[derive(Debug, Serialize)]
pub struct VertexAnimation {
    #[br(parse_with = parse_count_offset)]
    pub descriptors: Vec<VertexAnimationDescriptor>,
    #[br(parse_with = parse_count_offset)]
    pub targets: Vec<VertexAnimationTarget>,
}

#[binread]
#[derive(Debug, Serialize)]
pub struct VertexAnimationDescriptor {
    pub vertex_buffer_index: u32,
    pub target_start_index: u32,
    pub target_count: u32,
    // pointer to u16 indices 0,1,2,...?
    // start and ending frame for each target?
    #[br(parse_with = FilePtr32::parse)]
    #[br(args { inner: args! { count: target_count as usize * 2 }})]
    pub unk1: Vec<u16>,

    pub unk2: u32,
}

// TODO: vertex attributes for vertex animation data?
/// A set of target vertex values similar to a keyframe in traditional animations.
#[binread]
#[derive(Debug, Serialize)]
pub struct VertexAnimationTarget {
    /// Relative to [data_base_offset](struct.ModelData.html#structfield.data_base_offset)
    pub data_offset: u32,
    pub vertex_count: u32,
    pub vertex_size: u32,
    pub unk1: u32,
}

// TODO: How are weights assigned to vertices?
// TODO: Skinning happens in the vertex shader?
// TODO: Where are the skin weights in the vertex shader?
#[binread]
#[derive(Debug, Serialize)]
pub struct Weights {
    #[br(parse_with = parse_count_offset)]
    pub weights: Vec<Weight>,

    /// The descriptor in [vertex_buffers](struct.VertexData.html#structfield.vertex_buffer) containing the weight data.
    /// This is typically the last element.
    pub vertex_buffer_index: u16,

    count: u16,
    offset: u32, // offset to something?
    unk4: u32,
    unks5: [u32; 4], // padding?
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
