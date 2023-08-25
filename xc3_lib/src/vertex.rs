use crate::{
    parse_count_offset, parse_offset_count, parse_opt_ptr32, parse_ptr32,
    write::{round_up, xc3_write_binwrite_impl, Xc3Write},
};
use binrw::{args, binread, BinRead, BinResult, BinWrite};

/// Vertex and vertex index buffer data used by a [Model](crate::mxmd::Model).
#[binread]
#[derive(Debug, Xc3Write)]
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
    pub unk0: u32,
    pub unk1: u32,
    pub unk2: u32,

    // TODO: Extra data for every buffer except the single weights buffer?
    #[br(parse_with = parse_ptr32)]
    #[br(args { offset: base_offset, inner: args! { count: vertex_buffers.len() - 1 }})]
    #[xc3(offset)]
    pub vertex_buffer_info: Vec<VertexBufferInfo>,

    // 332 bytes of data?
    #[br(parse_with = parse_offset_count, offset = base_offset)]
    #[xc3(offset_count)]
    pub outline_buffers: Vec<OutlineBuffer>,

    #[br(parse_with = parse_opt_ptr32, offset = base_offset)]
    #[xc3(offset)]
    pub vertex_animation: Option<VertexAnimation>,

    /// The data buffer containing all the geometry data.
    // TODO: Optimized function for reading bytes?
    #[br(parse_with = parse_count_offset, offset = base_offset)]
    #[xc3(count_offset, align(4096))]
    pub buffer: Vec<u8>,

    // TODO: particles?
    #[br(parse_with = parse_opt_ptr32, offset = base_offset)]
    #[xc3(offset)]
    pub unk_data: Option<UnkData>,

    #[br(parse_with = parse_ptr32, offset = base_offset)]
    #[xc3(offset)]
    pub weights: Weights,

    #[br(parse_with = parse_ptr32, offset = base_offset)]
    #[xc3(offset)]
    pub unk7: Unk,

    // TODO: padding?
    pub unks: [u32; 5],
}

#[derive(BinRead, Xc3Write, Debug)]
#[br(import_raw(base_offset: u64))]
pub struct VertexBufferDescriptor {
    /// The offset into [buffer](struct.VertexData.html#structfield.buffer).
    pub data_offset: u32,
    pub vertex_count: u32,
    /// The size or stride of the vertex in bytes.
    pub vertex_size: u32,

    /// A tightly packed list of attributes for the data for this buffer.
    #[br(parse_with = parse_offset_count, offset = base_offset)]
    #[xc3(offset_count)]
    pub attributes: Vec<VertexAttribute>,

    pub unk1: u32,
    pub unk2: u32,
    pub unk3: u32,
}

/// A single attribute in a [VertexBufferDescriptor] like positions or normals.
///
/// Attributes are tightly packed, so the relative offset is
/// the sum of previous attribute sizes.
#[derive(BinRead, BinWrite, Debug)]
pub struct VertexAttribute {
    pub data_type: DataType,
    /// The size in bytes of [data_type](#structfield.data_type).
    pub data_size: u16,
}

/// The data type, usage, and component count for a [VertexAttribute].
#[derive(BinRead, BinWrite, Debug)]
#[brw(repr(u16))]
pub enum DataType {
    /// Float32x3 position.
    Position = 0,
    Unk1 = 1,
    Unk2 = 2,
    /// u32 index for the element in the weights buffer
    /// at index [vertex_buffer_index](struct.Weights.html#structfield.vertex_buffer_index)
    /// containing the [DataType::SkinWeights] and [DataType::BoneIndices] attributes.
    WeightIndex = 3,
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
    /// Unorm16x4 skin weights for up to 4 bone influences.
    SkinWeights = 41,
    /// u8x4 bone indices for up to 4 bone infuences in the [Skeleton](crate::mxmd::Skeleton) in the [Mxmd](crate::mxmd::Mxmd).
    BoneIndices = 42,
    Unk52 = 52,
}

// TODO: Is this data always u16?
#[derive(BinRead, BinWrite, Debug)]
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

#[derive(BinRead, BinWrite, Debug)]
#[brw(repr(u16))]
pub enum Unk1 {
    Unk0 = 0,
    Unk3 = 3,
}

#[derive(BinRead, BinWrite, Debug)]
#[brw(repr(u16))]
pub enum Unk2 {
    Unk0 = 0,
}

/// Vertex animation data often called "vertex morphs", "shape keys", or "blend shapes".
#[derive(BinRead, Xc3Write, Debug)]
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

#[derive(BinRead, Xc3Write, Debug)]
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
#[derive(BinRead, BinWrite, Debug)]
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
#[derive(BinRead, Xc3Write, Debug)]
pub struct Weights {
    #[br(parse_with = parse_count_offset)]
    #[xc3(count_offset)]
    pub groups: Vec<WeightGroup>,

    /// The descriptor in [vertex_buffers](struct.VertexData.html#structfield.vertex_buffer) containing the weight data.
    /// This is typically the last element.
    pub vertex_buffer_index: u16,

    // TODO: parse_count16_offset32?
    lod_count: u16,
    #[br(parse_with = parse_ptr32)]
    #[br(args { inner: args! { count: lod_count as usize }})]
    #[xc3(offset)]
    pub weight_lods: Vec<WeightLod>,

    pub unk4: u32,
    pub unks5: [u32; 4], // padding?
}

// TODO: Counts up to the total number of "vertices" in the skin weights buffer?
// TODO: How to select the weight group for each mesh in the model?
#[derive(BinRead, BinWrite, Debug)]
pub struct WeightGroup {
    pub output_start_index: u32,
    /// Start of the items in the weights buffer at [vertex_buffer_index](struct.Weights.html#structfield.vertex_buffer_index).
    // TODO: Why does this sometimes offset the starting index but not always?
    pub input_start_index: u32,
    /// Number of items in the weights buffer.
    pub count: u32,
    pub unks: [u32; 4],
    /// Index into [group_indices_plus_one](struct.WeightLod.html#structfield.group_indices_plus_one)
    /// pointing back to this group.
    pub lod_group_index: u8,
    /// Index into [weight_lods](struct.Weights.html#structfield.weight_lods).
    pub lod_index: u8,
    /// The max number of non-zero bone influences per vertex
    /// for the range of items in the weights buffer.
    pub max_influences: u8,
    pub unk4: u8,
    pub unks2: [u32; 2],
}

#[derive(BinRead, BinWrite, Debug)]
pub struct WeightLod {
    /// One plus the indices pointing back to [groups](struct.Weights.html#structfield.groups).
    /// Unused entries use the value `0`.
    pub group_indices_plus_one: [u16; 9],
}

#[binread]
#[derive(Debug, Xc3Write)]
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

#[derive(BinRead, BinWrite, Debug)]
pub struct UnkInner {
    unk1: u16,
    unk2: u16,
    unk3: u32,
    unk4: u32,
    unk5: u32,
    unk6: u32,
}

#[derive(BinRead, BinWrite, Debug)]
pub struct VertexBufferInfo {
    flags: u16,
    outline_buffer_index: u16,
    vertex_animation_target_start_index: u16,
    vertex_animation_target_count: u16,
    // TODO: padding?
    unk: u32,
}

#[derive(BinRead, BinWrite, Debug)]
pub struct OutlineBuffer {
    /// The offset into [buffer](struct.VertexData.html#structfield.buffer).
    pub data_offset: u32,
    pub vertex_count: u32,
    /// The size or stride of the vertex in bytes.
    pub vertex_size: u32,
    // TODO: padding?
    unk: u32,
}

#[derive(BinRead, BinWrite, Debug)]
pub struct UnkData {
    pub unk: [u32; 17],
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
    WeightLod,
    UnkData
);

// TODO: Generate this with a macro rules macro?
// TODO: Include this in some sort of trait?
pub fn write_vertex_data<W: std::io::Write + std::io::Seek>(
    vertex_data: &VertexData,
    writer: &mut W,
) -> BinResult<()> {
    let mut data_ptr = 0;

    let root = vertex_data.write(writer, &mut data_ptr)?;

    let vertex_buffers = root.vertex_buffers.write_offset(writer, 0, &mut data_ptr)?;
    root.index_buffers.write_offset(writer, 0, &mut data_ptr)?;
    root.vertex_buffer_info
        .write_offset(writer, 0, &mut data_ptr)?;

    // TODO: Do all empty lists use offset 0?
    if !vertex_data.outline_buffers.is_empty() {
        root.outline_buffers
            .write_offset(writer, 0, &mut data_ptr)?;
    }

    for vertex_buffer in vertex_buffers {
        vertex_buffer
            .attributes
            .write_offset(writer, 0, &mut data_ptr)?;
    }

    let weights = root.weights.write_offset(writer, 0, &mut data_ptr)?;
    weights.groups.write_offset(writer, 0, &mut data_ptr)?;
    weights.weight_lods.write_offset(writer, 0, &mut data_ptr)?;

    root.unk_data.write_offset(writer, 0, &mut data_ptr)?;

    // TODO: Add alignment customization to derive?
    data_ptr = round_up(data_ptr, 4);
    if let Some(vertex_animation) = root
        .vertex_animation
        .write_offset(writer, 0, &mut data_ptr)?
    {
        let descriptors = vertex_animation
            .descriptors
            .write_offset(writer, 0, &mut data_ptr)?;
        vertex_animation
            .targets
            .write_offset(writer, 0, &mut data_ptr)?;

        for descriptor in descriptors {
            descriptor.unk1.write_offset(writer, 0, &mut data_ptr)?;
        }
    }

    // TODO: Add alignment customization to derive?
    data_ptr = round_up(data_ptr, 4);
    let unk7 = root.unk7.write_offset(writer, 0, &mut data_ptr)?;
    unk7.unk1
        .write_offset(writer, unk7.base_offset, &mut data_ptr)?;

    root.buffer.write_offset(writer, 0, &mut data_ptr)?;

    Ok(())
}
