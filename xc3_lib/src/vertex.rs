use crate::{
    parse_count_offset, parse_offset_count, parse_opt_ptr32, parse_ptr32,
    write::{round_up, xc3_write_binwrite_impl, Xc3Write},
};
use binrw::{args, binread, BinRead, BinResult, BinWrite};
use serde::Serialize;

/// Vertex and vertex index buffer data used by a [Model](crate::mxmd::Model).
#[binread]
#[derive(Debug, Xc3Write, Serialize)]
#[br(stream = r)]
pub struct VertexData {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    #[br(parse_with = parse_offset_count, args { offset: base_offset, inner: base_offset })]
    #[xc3(offset_count)]
    pub vertex_buffers: Vec<VertexBufferDescriptor>,

    #[br(parse_with = parse_offset_count, offset = base_offset)]
    #[xc3(offset_count)]
    pub index_buffers: Vec<IndexBufferDescriptor>,

    // padding?
    unk0: u32,
    unk1: u32,
    unk2: u32,

    // TODO: Extra data for every buffer except the single weights buffer?
    #[br(parse_with = parse_ptr32)]
    #[br(args { offset: base_offset, inner: args! { count: vertex_buffers.len() - 1 }})]
    #[xc3(offset)]
    vertex_buffer_info: Vec<VertexBufferInfo>,

    // 332 bytes of data?
    #[br(parse_with = parse_offset_count, offset = base_offset)]
    #[xc3(offset_count)]
    outline_buffers: Vec<OutlineBuffer>,

    #[br(parse_with = parse_opt_ptr32, offset = base_offset)]
    #[xc3(offset)]
    pub vertex_animation: Option<VertexAnimation>,

    /// The data buffer containing all the geometry data.
    // TODO: Optimized function for reading bytes?
    #[br(parse_with = parse_count_offset, offset = base_offset)]
    #[xc3(count_offset)]
    pub buffer: Vec<u8>,

    // TODO: particles?
    unk6: u32,

    #[br(parse_with = parse_ptr32, offset = base_offset)]
    #[xc3(offset)]
    pub weights: Weights,

    #[br(parse_with = parse_ptr32, offset = base_offset)]
    #[xc3(offset)]
    unk7: Unk,

    // TODO: padding?
    unks: [u32; 5],
}

#[derive(BinRead, Xc3Write, Debug, Serialize)]
#[br(import_raw(base_offset: u64))]
pub struct VertexBufferDescriptor {
    /// The offset into [buffer](struct.VertexData.html#structfield.buffer).
    pub data_offset: u32,
    pub vertex_count: u32,
    /// The size or stride of the vertex in bytes.
    pub vertex_size: u32,

    // Corresponds to attributes in vertex shader?
    #[br(parse_with = parse_offset_count, offset = base_offset)]
    #[xc3(offset_count)]
    pub attributes: Vec<VertexAttribute>,

    pub unk1: u32,
    pub unk2: u32,
    pub unk3: u32,
}

#[derive(BinRead, BinWrite, Debug, Serialize)]
pub struct VertexAttribute {
    pub data_type: DataType,
    pub data_size: u16,
}

#[derive(BinRead, BinWrite, Debug, Serialize)]
#[brw(repr(u16))]
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
    Unk15 = 15,
    Unk16 = 16,
    /// Unorm8x4 vertex RGBA color.
    VertexColor = 17,
    Unk18 = 18,
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
#[derive(BinRead, BinWrite, Debug, Serialize)]
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

#[derive(BinRead, BinWrite, Debug, Serialize)]
#[brw(repr(u16))]
pub enum Unk1 {
    Unk0 = 0,
    Unk3 = 3,
}

#[derive(BinRead, BinWrite, Debug, Serialize)]
#[brw(repr(u16))]
pub enum Unk2 {
    Unk0 = 0,
}

/// Vertex animation data often called "vertex morphs", "shape keys", or "blend shapes".
#[derive(BinRead, Xc3Write, Debug, Serialize)]
pub struct VertexAnimation {
    #[br(parse_with = parse_count_offset)]
    #[xc3(count_offset)]
    pub descriptors: Vec<VertexAnimationDescriptor>,

    #[br(parse_with = parse_count_offset)]
    #[xc3(count_offset)]
    pub targets: Vec<VertexAnimationTarget>,

    // TODO: padding?
    pub unks: [u32; 4],
}

#[derive(BinRead, Xc3Write, Debug, Serialize)]
pub struct VertexAnimationDescriptor {
    pub vertex_buffer_index: u32,
    pub target_start_index: u32,
    pub target_count: u32,

    // TODO: count_offset?
    // pointer to u16 indices 0,1,2,...?
    // start and ending frame for each target?
    #[br(parse_with = parse_ptr32)]
    #[br(args { inner: args! { count: target_count as usize }})]
    #[xc3(offset)]
    pub unk1: Vec<u16>,

    pub unk2: u32,
}

// TODO: vertex attributes for vertex animation data?
/// A set of target vertex values similar to a keyframe in traditional animations.
#[derive(BinRead, BinWrite, Debug, Serialize)]
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
#[derive(BinRead, Xc3Write, Debug, Serialize)]
pub struct Weights {
    #[br(parse_with = parse_count_offset)]
    #[xc3(count_offset)]
    pub groups: Vec<WeightGroup>,

    /// The descriptor in [vertex_buffers](struct.VertexData.html#structfield.vertex_buffer) containing the weight data.
    /// This is typically the last element.
    pub vertex_buffer_index: u16,

    lod_count: u16,
    #[br(parse_with = parse_ptr32)]
    #[br(args { inner: args! { count: lod_count as usize }})]
    #[xc3(offset)]
    weight_lods: Vec<WeightLod>,

    unk4: u32,
    unks5: [u32; 4], // padding?
}

#[derive(BinRead, BinWrite, Debug, Serialize)]
pub struct WeightGroup {
    // offsets are just the sum of the previous counts?
    unk1: u32, // offset?
    unk2: u32, // offset?
    unk3: u32, // count?
    unks: [u32; 7],
}

#[derive(BinRead, BinWrite, Debug, Serialize)]
pub struct WeightLod {
    unks: [u16; 9],
}

#[binread]
#[derive(Debug, Xc3Write, Serialize)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct Unk {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    #[br(parse_with = parse_count_offset, offset = base_offset)]
    #[xc3(count_offset)]
    pub unk1: Vec<UnkInner>,

    // The length of the data in bytes.
    pub data_count: u32,

    /// The offset into [buffer](struct.VertexData.html#structfield.buffer).
    pub data_offset: u32,

    // TODO: Padding?
    unks: [u32; 8],
}

#[derive(BinRead, BinWrite, Debug, Serialize)]
pub struct UnkInner {
    unk1: u16,
    unk2: u16,
    unk3: u32,
    unk4: u32,
    unk5: u32,
    unk6: u32,
}

#[derive(BinRead, BinWrite, Debug, Serialize)]
pub struct VertexBufferInfo {
    flags: u16,
    outline_buffer_index: u16,
    vertex_animation_target_start_index: u16,
    vertex_animation_target_count: u16,
    // TODO: padding?
    unk: u32,
}

#[derive(BinRead, BinWrite, Debug, Serialize)]
pub struct OutlineBuffer {
    /// The offset into [buffer](struct.VertexData.html#structfield.buffer).
    pub data_offset: u32,
    pub vertex_count: u32,
    /// The size or stride of the vertex in bytes.
    pub vertex_size: u32,
    // TODO: padding?
    unk: u32,
}

xc3_write_binwrite_impl!(
    VertexAttribute,
    DataType,
    IndexBufferDescriptor,
    Unk1,
    Unk2,
    VertexAnimationTarget,
    WeightGroup,
    UnkInner,
    VertexBufferInfo,
    OutlineBuffer,
    WeightLod
);

// TODO: Generate this with a macro rules macro?
// TODO: Include this in some sort of trait?
pub fn write_vertex_data<W: std::io::Write + std::io::Seek>(
    root: &VertexData,
    writer: &mut W,
) -> BinResult<()> {
    let mut data_ptr = 0;

    let root_offsets = root.write(writer, &mut data_ptr)?;

    let vertex_buffers_offsets =
        root_offsets
            .vertex_buffers
            .write_offset(writer, 0, &mut data_ptr)?;
    root_offsets
        .index_buffers
        .write_offset(writer, 0, &mut data_ptr)?;
    root_offsets
        .vertex_buffer_info
        .write_offset(writer, 0, &mut data_ptr)?;

    // TODO: Do all empty lists use offset 0?
    if !root.outline_buffers.is_empty() {
        root_offsets
            .outline_buffers
            .write_offset(writer, 0, &mut data_ptr)?;
    }

    for offsets in vertex_buffers_offsets {
        offsets.attributes.write_offset(writer, 0, &mut data_ptr)?;
    }

    let weights_offsets = root_offsets
        .weights
        .write_offset(writer, 0, &mut data_ptr)?;
    weights_offsets
        .groups
        .write_offset(writer, 0, &mut data_ptr)?;
    weights_offsets
        .weight_lods
        .write_offset(writer, 0, &mut data_ptr)?;

    // TODO: Prevent writing the offset for the null case?
    // TODO: Add alignment customization to derive?
    data_ptr = round_up(data_ptr, 4);
    if root.vertex_animation.is_some() {
        if let Some(vertex_animation_offsets) =
            root_offsets
                .vertex_animation
                .write_offset(writer, 0, &mut data_ptr)?
        {
            let descriptors_offsets =
                vertex_animation_offsets
                    .descriptors
                    .write_offset(writer, 0, &mut data_ptr)?;
            vertex_animation_offsets
                .targets
                .write_offset(writer, 0, &mut data_ptr)?;

            for offsets in descriptors_offsets {
                offsets.unk1.write_offset(writer, 0, &mut data_ptr)?;
            }
        }
    }

    // TODO: Add alignment customization to derive?
    data_ptr = round_up(data_ptr, 4);
    let unk_offsets = root_offsets.unk7.write_offset(writer, 0, &mut data_ptr)?;
    unk_offsets
        .unk1
        .write_offset(writer, unk_offsets.base_offset, &mut data_ptr)?;

    // TODO: Special type with 4096 byte alignment?
    data_ptr = round_up(data_ptr, 4096);
    root_offsets.buffer.write_offset(writer, 0, &mut data_ptr)?;

    Ok(())
}
