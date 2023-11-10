//! Vertex and geometry data for model formats.
use crate::{
    parse_count16_offset32, parse_count32_offset32, parse_offset32_count32, parse_opt_ptr32,
    parse_ptr32, xc3_write_binwrite_impl,
};
use bilge::prelude::*;
use binrw::{args, binread, BinRead, BinWrite};
use xc3_write::{Xc3Write, Xc3WriteOffsets};

/// Vertex and vertex index buffer data used by a [Model](crate::mxmd::Model).
#[binread]
#[derive(Debug, Xc3Write)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct VertexData {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    #[br(parse_with = parse_offset32_count32, args { offset: base_offset, inner: base_offset })]
    #[xc3(offset_count(u32, u32))]
    pub vertex_buffers: Vec<VertexBufferDescriptor>,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub index_buffers: Vec<IndexBufferDescriptor>,

    // padding?
    pub unk0: u32,
    pub unk1: u32,
    pub unk2: u32,

    // TODO: Extra data for every buffer except the single weights buffer?
    #[br(parse_with = parse_ptr32)]
    #[br(args { offset: base_offset, inner: args! { count: vertex_buffers.len() - 1 }})]
    #[xc3(offset(u32))]
    pub vertex_buffer_info: Vec<VertexBufferInfo>,

    // 332 bytes of data?
    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub outline_buffers: Vec<OutlineBuffer>,

    #[br(parse_with = parse_opt_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub vertex_morphs: Option<VertexMorphs>,

    /// The data buffer containing all the geometry data.
    // TODO: Optimized function for reading bytes?
    #[br(parse_with = parse_count32_offset32, offset = base_offset)]
    #[xc3(count_offset(u32, u32), align(4096))]
    pub buffer: Vec<u8>,

    // TODO: particles?
    #[br(parse_with = parse_opt_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub unk_data: Option<UnkData>,

    #[br(parse_with = parse_opt_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub weights: Option<Weights>,

    #[br(parse_with = parse_opt_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub unk7: Option<Unk>,

    // TODO: padding?
    pub unks: [u32; 5],
}

#[derive(Debug, BinRead, Xc3Write)]
#[br(import_raw(base_offset: u64))]
pub struct VertexBufferDescriptor {
    /// The offset into [buffer](struct.VertexData.html#structfield.buffer).
    pub data_offset: u32,
    pub vertex_count: u32,
    /// The size or stride of the vertex in bytes.
    pub vertex_size: u32,

    /// A tightly packed list of attributes for the data for this buffer.
    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub attributes: Vec<VertexAttribute>,

    pub unk1: u32,
    pub unk2: u32,
    pub unk3: u32,
}

/// A single attribute in a [VertexBufferDescriptor] like positions or normals.
///
/// Attributes are tightly packed, so the relative offset is
/// the sum of previous attribute sizes.
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct VertexAttribute {
    pub data_type: DataType,
    /// The size in bytes of [data_type](#structfield.data_type).
    pub data_size: u16,
}

/// The data type, usage, and component count for a [VertexAttribute].
#[derive(Debug, BinRead, BinWrite, PartialEq, Eq, Clone, Copy)]
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
    // TODO: Are these actually UV coordinates?
    Uv2 = 6,
    Uv3 = 7,
    Uv4 = 8,
    Uv5 = 9,
    Uv6 = 10,
    Uv7 = 11,
    Uv8 = 12,
    Uv9 = 13,
    /// Unorm8x4 vertex RGBA color.
    VertexColorUnk14 = 14,
    Unk15 = 15,
    Unk16 = 16,
    /// Unorm8x4 vertex RGBA color.
    VertexColorUnk17 = 17,
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
    /// u8x4 bone indices for up to 4 bone infuences in the [Skinning](crate::mxmd::Skinning) in the [Mxmd](crate::mxmd::Mxmd).
    BoneIndices = 42,
    Unk52 = 52,
}

// TODO: Is this data always u16?
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
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

#[derive(Debug, BinRead, BinWrite)]
#[brw(repr(u16))]
pub enum Unk1 {
    Unk0 = 0,
    Unk3 = 3,
}

#[derive(Debug, BinRead, BinWrite)]
#[brw(repr(u16))]
pub enum Unk2 {
    Unk0 = 0,
}

/// Vertex animation data often called "vertex morphs", "shape keys", or "blend shapes".
#[derive(Debug, BinRead, Xc3Write)]
pub struct VertexMorphs {
    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub descriptors: Vec<MorphDescriptor>,

    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub targets: Vec<MorphTarget>,

    // TODO: padding?
    pub unks: [u32; 4],
}

#[derive(Debug, BinRead, Xc3Write)]
pub struct MorphDescriptor {
    pub vertex_buffer_index: u32,
    pub target_start_index: u32,
    pub target_count: u32,

    // TODO: count_offset?
    // pointer to u16 indices 0,1,2,...?
    // start and ending frame for each target?
    #[br(parse_with = parse_ptr32)]
    #[br(args { inner: args! { count: target_count as usize }})]
    #[xc3(offset(u32))]
    pub unk1: Vec<u16>,

    // flags?
    pub unk2: u32,
}

// TODO: vertex attributes for vertex animation data?
/// A set of target vertex values similar to a keyframe in traditional animations.
#[derive(Debug, BinRead, BinWrite)]
pub struct MorphTarget {
    /// Relative to [data_base_offset](struct.ModelData.html#structfield.data_base_offset)
    pub data_offset: u32,
    pub vertex_count: u32,
    pub vertex_size: u32,

    pub flags: MorphTargetFlags,
}

#[bitsize(32)]
#[derive(DebugBits, FromBits, BinRead, BinWrite, Clone, Copy)]
#[br(map = u32::into)]
#[bw(map = |&x| u32::from(x))]
pub struct MorphTargetFlags {
    pub unk1: u16,                 // always 0?
    pub blend_target_buffer: bool, // once per descriptor?
    pub default_buffer: bool,      // once per descriptor?
    pub param_buffer: bool,
    pub unk5: u13, // always 0?
}

// TODO: How are weights assigned to vertices?
// TODO: Skinning happens in the vertex shader?
// TODO: Where are the skin weights in the vertex shader?
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct Weights {
    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub groups: Vec<WeightGroup>,

    /// The descriptor in [vertex_buffers](struct.VertexData.html#structfield.vertex_buffer) containing the weight data.
    /// This is typically the last element.
    pub vertex_buffer_index: u16,

    // TODO: same count as WeightLod in mxmd?
    #[br(parse_with = parse_count16_offset32)]
    #[xc3(count_offset(u16, u32))]
    pub weight_lods: Vec<WeightLod>,

    pub unk4: u32,
    pub unks5: [u32; 4], // padding?
}

// TODO: Counts up to the total number of "vertices" in the skin weights buffer?
// TODO: How to select the weight group for each mesh in the model?
#[derive(Debug, Clone, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct WeightGroup {
    /// Index into the skinning buffer used in the shader that combines weights and transforms.
    pub output_start_index: u32,
    /// Start of the items in the weights buffer at [vertex_buffer_index](struct.Weights.html#structfield.vertex_buffer_index).
    // TODO: Why does this sometimes offset the starting index but not always?
    pub input_start_index: u32,
    /// Number of items in the weights buffer.
    pub count: u32,
    pub unks: [u32; 4], // TODO: always 0?
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

// TODO: The material's pass index indexes into this?
// [unk0, ???, ???, unk7, ???, ???, ???, ???, ???]

// group_index = weights.weight_lods[mesh.lod].group_indices_plus_one[material.program.pass_index] - 1
// group = weights.groups[group_index]

// TODO: What indexes into this?
// TODO: something related to render pass?
#[derive(Debug, Clone, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct WeightLod {
    /// One plus the indices pointing back to [groups](struct.Weights.html#structfield.groups).
    /// Unused entries use the value `0`.
    pub group_indices_plus_one: [u16; 9],
}

#[binread]
#[derive(Debug, Xc3Write, Xc3WriteOffsets)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct Unk {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    #[br(parse_with = parse_count32_offset32, offset = base_offset)]
    #[xc3(count_offset(u32, u32))]
    pub unk1: Vec<UnkInner>,

    // The length of the data in bytes.
    pub data_count: u32,

    /// The offset into [buffer](struct.VertexData.html#structfield.buffer).
    pub data_offset: u32,

    // TODO: Padding?
    unks: [u32; 8],
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct UnkInner {
    unk1: u16,
    unk2: u16,
    unk3: u32,
    unk4: u32,
    unk5: u32,
    unk6: u32,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct VertexBufferInfo {
    flags: u16,
    outline_buffer_index: u16,
    vertex_animation_target_start_index: u16,
    vertex_animation_target_count: u16,
    // TODO: padding?
    unk: u32,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct OutlineBuffer {
    /// The offset into [buffer](struct.VertexData.html#structfield.buffer).
    pub data_offset: u32,
    pub vertex_count: u32,
    /// The size or stride of the vertex in bytes.
    pub vertex_size: u32,
    // TODO: padding?
    unk: u32,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct UnkData {
    pub unk: [u32; 17],
}

xc3_write_binwrite_impl!(DataType, Unk1, Unk2, MorphTarget);

impl<'a> Xc3WriteOffsets for VertexDataOffsets<'a> {
    fn write_offsets<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
    ) -> xc3_write::Xc3Result<()> {
        let vertex_buffers = self
            .vertex_buffers
            .write_offset(writer, base_offset, data_ptr)?;
        self.index_buffers
            .write_offset(writer, base_offset, data_ptr)?;
        self.vertex_buffer_info
            .write_offset(writer, base_offset, data_ptr)?;

        // TODO: Do all empty lists use offset 0?
        if !self.outline_buffers.data.is_empty() {
            self.outline_buffers
                .write_offset(writer, base_offset, data_ptr)?;
        }

        // The first attribute is aligned to 16.
        // TODO: This doesn't always happen?
        // *data_ptr = round_up(*data_ptr, 16);
        for vertex_buffer in vertex_buffers.0 {
            vertex_buffer
                .attributes
                .write_offset(writer, base_offset, data_ptr)?;
        }

        self.weights.write_full(writer, base_offset, data_ptr)?;

        self.unk_data.write_offset(writer, base_offset, data_ptr)?;

        if let Some(vertex_animation) =
            self.vertex_morphs
                .write_offset(writer, base_offset, data_ptr)?
        {
            let descriptors =
                vertex_animation
                    .descriptors
                    .write_offset(writer, base_offset, data_ptr)?;
            vertex_animation
                .targets
                .write_offset(writer, base_offset, data_ptr)?;

            for descriptor in descriptors.0 {
                descriptor
                    .unk1
                    .write_offset(writer, base_offset, data_ptr)?;
            }
        }

        self.unk7.write_full(writer, base_offset, data_ptr)?;

        self.buffer.write_offset(writer, base_offset, data_ptr)?;

        Ok(())
    }
}
