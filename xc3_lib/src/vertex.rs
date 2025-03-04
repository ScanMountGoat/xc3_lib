//! Vertex and geometry data for models and map models.
//!
//! # Overview
//! A [VertexData] file stores model geometry in a combined [buffer](struct.VertexData.html#structfield.buffer).
//! The remaining fields describe the data stored in the buffer like vertices or morph targets.
//!
//! Each [Mesh](crate::mxmd::Mesh) draw call references a [VertexBufferDescriptor] and [IndexBufferDescriptor].
//! Vertex buffers except the weights buffer have an associated [VertexBufferExtInfo]
//! for assigning additional data like outline buffers or morph targets.
//!
//! The weights buffer just contains [DataType::SkinWeights] and [DataType::BoneIndices].
//! This buffer is shared between all vertex buffers with
//! each buffer selecting weight buffer "vertices" using [DataType::WeightIndex]
//! and additional indexing information defined in [Weights].
//! See [xc3_model](https://docs.rs/xc3_model) for the complete indexing implementation.
//!
//! Some vertex buffers have optional morph targets assigned in [VertexMorphs].
//! Morph targets define a default target for the neutral pose as well as additional
//! targets applied on top of the default target.
//! Morph targets define attributes not present in the vertex buffer and have a
//! final attribute value defined as `default + target_delta * weight`
//! where `target_delta` is defined sparsely using a list of deltas and vertex indices.
//!
//! # Attribute Layout
//! The sections of the byte buffer for each descriptor contain each attribute for each vertex in order.
//! This interleaved or "array of structs" layout is cache friendly when accessing each attribute for each vertex
//! like in the vertex shaders in game.
//! ```text
//! position 0
//! normal 0
//! position 1
//! normal 1
//! ...
//! ```
//! Applications tend to work better with a "struct of arrays" approach where
//! dedicated arrays store the values for a single attribute for all items.
//! This approach is cache friendly when accessing the same attribute for all vertices
//! and allows for easily adding and removing attributes.
//! This is the approach used by [xc3_model](https://docs.rs/xc3_model).
//! ```text
//! position 0
//! position 1
//! ...
//! ```
//! ```text
//! normal 0
//! normal 1
//! ...
//! ```
use crate::{
    parse_count16_offset32, parse_count32_offset32, parse_offset32_count32, parse_opt_ptr32,
    parse_ptr32, xc3_write_binwrite_impl,
};
use bilge::prelude::*;
use binrw::{args, binread, BinRead, BinWrite};
use xc3_write::{Xc3Write, Xc3WriteOffsets};

/// Vertex and vertex index buffer data used by a [Model](crate::mxmd::Model).
#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, PartialEq, Clone)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct VertexData {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    // TODO: Sometimes 80 and sometimes 84?
    #[br(parse_with = parse_offset32_count32)]
    #[br(args { offset: base_offset, inner: base_offset })]
    #[xc3(offset_count(u32, u32))]
    pub vertex_buffers: Vec<VertexBufferDescriptor>,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub index_buffers: Vec<IndexBufferDescriptor>,

    // padding?
    pub unk0: u32,
    pub unk1: u32,
    pub unk2: u32,

    #[br(parse_with = parse_ptr32)]
    #[br(args { offset: base_offset, inner: args! { count: buffer_info_count(&vertex_buffers) }})]
    #[xc3(offset(u32))]
    pub vertex_buffer_info: Vec<VertexBufferExtInfo>,

    // 332 bytes of data?
    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub outline_buffers: Vec<OutlineBufferDescriptor>,

    #[br(parse_with = parse_opt_ptr32)]
    #[br(args { offset: base_offset, inner: base_offset })]
    #[xc3(offset(u32))]
    pub vertex_morphs: Option<VertexMorphs>,

    /// The data buffer containing all the geometry data aligned to 4096.
    /// Buffers are typically packed in this buffer in the following order:
    /// [vertex_buffers](#structfield.vertex_buffers),
    /// [outline_buffers](#structfield.outline_buffers),
    /// [index_buffers](#structfield.index_buffers),
    /// [vertex_morphs](#structfield.vertex_morphs),
    /// [unk7](#structfield.unk7).
    #[br(parse_with = parse_count32_offset32, offset = base_offset)]
    #[xc3(count_offset(u32, u32), align(4096))]
    pub buffer: Vec<u8>,

    // TODO: particles?
    #[br(parse_with = parse_opt_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub unk_data: Option<UnkData>,

    #[br(parse_with = parse_opt_ptr32)]
    #[br(args { offset: base_offset, inner: base_offset })]
    #[xc3(offset(u32))]
    pub weights: Option<Weights>,

    #[br(parse_with = parse_opt_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub unk7: Option<Unk>,

    // TODO: padding?
    pub unks: [u32; 5],
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, PartialEq, Eq, Clone)]
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
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Eq, Clone, Copy)]
pub struct VertexAttribute {
    pub data_type: DataType,
    /// The size in bytes of [data_type](#structfield.data_type).
    pub data_size: u16,
}

// Format is taken from RenderDoc debugging.
// Names are taken from shader attribute metadata.
/// The data type, usage, and component count for a [VertexAttribute].
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, BinWrite, PartialEq, Eq, Clone, Copy)]
#[brw(repr(u16))]
pub enum DataType {
    /// Float32x3 "vPos" in shaders.
    Position = 0,
    /// Float32x3 "fWeight" in shaders.
    /// The fourth weight component is calculated as `w = 1.0 - x - y - z`.
    /// Only used for Xenoblade X.
    SkinWeights2 = 1,
    /// Uint8x4 bone indices for up to 4 bone infuences.
    /// Only used for Xenoblade X.
    BoneIndices2 = 2,
    /// Uint16x2 "nWgtIdx" in shaders.
    ///
    /// The index in the first component selects elements in the precomputed skinning matrices in the vertex shader.
    /// See [Weights] for details.
    WeightIndex = 3,
    /// Uint16x2 "nWgtIdx" in shaders.
    ///
    /// Used for some stage and object models.
    WeightIndex2 = 4,
    /// Float32x2 "vTex0" in shaders.
    TexCoord0 = 5,
    /// Float32x2 "vTex1" in shaders.
    TexCoord1 = 6,
    /// Float32x2 "vTex2" in shaders.
    TexCoord2 = 7,
    /// Float32x2 "vTex3" in shaders.
    TexCoord3 = 8,
    /// Float32x2 "vTex4" in shaders.
    TexCoord4 = 9,
    /// Float32x2 "vTex5" in shaders.
    TexCoord5 = 10,
    /// Float32x2 "vTex6" in shaders.
    TexCoord6 = 11,
    /// Float32x2 "vTex7" in shaders.
    TexCoord7 = 12,
    /// Float32x2 "vTex8" in shaders.
    TexCoord8 = 13,
    /// Unorm8x4 "vBlend" in shaders.
    Blend = 14,
    /// Float32x3 ??? in shaders.
    Unk15 = 15,
    Unk16 = 16, // TODO: 2 snorm8x4?
    /// Unorm8x4 "vColor" in shaders.
    VertexColor = 17,
    /// Float32x3 ??? in shaders.
    Unk18 = 18,
    /// ??? "vGmCal1" in shaders.
    Unk24 = 24,
    /// ??? "vGmCal2" in shaders.
    Unk25 = 25,
    /// ??? "vGmCal3" in shaders.
    Unk26 = 26,
    /// Snorm8x4 "vNormal" in shaders.
    Normal = 28,
    /// Snorm8x4 "vTan" in shaders with bitangent sign in the fourth component.
    Tangent = 29,
    /// ??? "fGmAl" in shaders.
    Unk30 = 30,
    Unk31 = 31, // TODO: xcx only?
    /// Snorm8x4 "vNormal" in shaders.
    Normal2 = 32,
    /// Snorm8x4 "vValInf" in shaders.
    // TODO: related to normals?
    ValInf = 33,
    /// Snorm8x4 "vNormal" in shaders.
    Normal3 = 34,
    /// Unorm8x4 "vColor" in shaders.
    VertexColor3 = 35,
    /// Float32x3 "vPos" in shaders.
    Position2 = 36,
    /// Unorm8x4 "vNormal" in shaders.
    /// Component values are in the range `[0.0, 1.0]` instead of `[-1.0, 1.0]`.
    /// Calculate the actual vector as `v * 2.0 -1.0`.
    Normal4 = 37,
    /// Float32x3 "vOldPos" in shaders.
    OldPosition = 39,
    /// Unorm8x4 "vTan" in shaders.
    /// Component values are in the range `[0.0, 1.0]` instead of `[-1.0, 1.0]`.
    /// Calculate the actual vector as `v * 2.0 -1.0`.
    Tangent2 = 40,
    /// Unorm16x4 skin weights for up to 4 bone influences.
    SkinWeights = 41,
    /// Uint8x4 bone indices for up to 4 bone infuences in the [Skinning](crate::mxmd::Skinning) in the [Mxmd](crate::mxmd::Mxmd).
    BoneIndices = 42,
    /// ??? "vFlow" in shaders.
    Flow = 52,
}

// TODO: Is this data always u16?
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Eq, Clone)]
pub struct IndexBufferDescriptor {
    /// The offset into [buffer](struct.VertexData.html#structfield.buffer).
    pub data_offset: u32,
    pub index_count: u32,
    pub primitive_type: PrimitiveType,
    pub index_format: IndexFormat,
    // TODO: padding?
    pub unk3: u32,
    pub unk4: u32,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, BinWrite, PartialEq, Eq, Clone, Copy)]
#[brw(repr(u16))]
pub enum PrimitiveType {
    TriangleList = 0,
    QuadList = 1,
    TriangleStrip = 2,
    /// TODO: GL_TRIANGLES_ADJACENCY helps with geometry shaders?
    TriangleListAdjacency = 3,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, BinWrite, PartialEq, Eq, Clone, Copy)]
#[brw(repr(u16))]
pub enum IndexFormat {
    Uint16 = 0,
    Uint32 = 1,
}

/// Vertex animation data often called "vertex morphs", "shape keys", or "blend shapes".
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct VertexMorphs {
    #[br(parse_with = parse_count32_offset32)]
    #[br(args { offset: base_offset, inner: base_offset })]
    #[xc3(count_offset(u32, u32))]
    pub descriptors: Vec<MorphDescriptor>,

    #[br(parse_with = parse_count32_offset32, offset = base_offset)]
    #[xc3(count_offset(u32, u32))]
    pub targets: Vec<MorphTarget>,

    // TODO: padding?
    pub unks: [u32; 4],
}

/// Morph targets assigned to a [VertexBufferDescriptor].
///
/// Each buffer has a blend target and default target followed by param targets.
/// This means the actual target count is two more than
/// the length of [param_indices](#structfield.param_indices).
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct MorphDescriptor {
    pub vertex_buffer_index: u32,
    pub target_start_index: u32,

    /// Indices into [controllers](../mxmd/struct.MorphControllers.html#structfield.controllers).
    #[br(parse_with = parse_count32_offset32, offset = base_offset)]
    #[xc3(count_offset(u32, u32))]
    pub param_indices: Vec<u16>,

    // flags?
    // TODO: 259 and 260 have twice as many targets (ch01031011)?
    // TODO: 3 also adds extra "vertices" with all 0's for data?
    pub unk2: u32, // 3, 4, 259, 260
}

// TODO: vertex attributes for vertex animation data?
/// A set of target vertex values similar to a keyframe in traditional animations.
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, BinWrite, PartialEq, Clone)]
pub struct MorphTarget {
    /// Relative to [data_base_offset](struct.ModelData.html#structfield.data_base_offset)
    pub data_offset: u32,
    pub vertex_count: u32,
    pub vertex_size: u32,

    pub flags: MorphTargetFlags,
}

#[bitsize(32)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(DebugBits, FromBits, BinRead, BinWrite, Clone, Copy, PartialEq)]
#[br(map = u32::into)]
#[bw(map = |&x| u32::from(x))]
pub struct MorphTargetFlags {
    pub unk1: u16, // always 0?
    /// The base values for each vertex.
    pub blend_target_buffer: bool,
    /// The base values for each of the vertices modified by any of the param targets.
    pub default_buffer: bool,
    /// Values for vertices modified by a param target.
    pub param_buffer: bool,
    pub unk5: u13, // always 0?
}

/// Information used for precomputing skinning matrices
/// based on a mesh's level of detail (LOD) and [RenderPassType](crate::mxmd::RenderPassType).
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct Weights {
    /// Selected based on the associated [WeightLod] for a [Mesh](crate::mxmd::Mesh).
    #[br(parse_with = parse_count32_offset32, offset = base_offset)]
    #[xc3(count_offset(u32, u32))]
    pub groups: Vec<WeightGroup>,

    /// The descriptor in [vertex_buffers](struct.VertexData.html#structfield.vertex_buffer) containing the weight data.
    /// This is typically the last element.
    pub vertex_buffer_index: u16,

    /// Selected based on the LOD of the [Mesh](crate::mxmd::Mesh).
    #[br(parse_with = parse_count16_offset32, offset = base_offset)]
    #[xc3(count_offset(u16, u32))]
    pub weight_lods: Vec<WeightLod>,

    // TODO: always 0 for xc2?
    pub unk4: u32, // 0, 1

    // TODO: padding?
    pub unks: [u32; 4],
}

/// A range of elements in the weights buffer.
/// Each element in the weights buffer is part of at least one [WeightGroup].
///
/// The [input_start_index](#structfield.input_start_index) and [count](#structfield.count)
/// select a range of [DataType::BoneIndices] and [DataType::SkinWeights] from the weights buffer.
/// The bone matrices for each bone index multiplied by the weights are written to the output buffer starting at [output_start_index](#structfield.output_start_index).
/// This precomputed skinning buffer is used to select transforms in the vertex shader using [DataType::WeightIndex].
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Clone, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct WeightGroup {
    /// Index into the skinning buffer used in the vertex shader with bone transforms multiplied by skin weights.
    /// These weighted bone matrices are selected using [DataType::WeightIndex].
    pub output_start_index: u32,
    /// Start of the elements in the weights buffer at [vertex_buffer_index](struct.Weights.html#structfield.vertex_buffer_index).
    pub input_start_index: u32,
    /// Number of elements in the weights buffer.
    pub count: u32,
    pub unks: [u32; 4], // TODO: always 0?
    /// Index into [group_indices_plus_one](struct.WeightLod.html#structfield.group_indices_plus_one)
    /// pointing back to this group.
    pub lod_group_index: u8,
    /// Index into [weight_lods](struct.Weights.html#structfield.weight_lods)
    /// for the [WeightLod] that references this [WeightGroup].
    pub lod_index: u8,
    /// The max number of non-zero bone influences per vertex
    /// for the range of elements in the weights buffer.
    pub max_influences: u8,
    pub unk4: u8,
    pub unks2: [u32; 2],
}

// TODO: The material's pass index indexes into this?
// TODO: Figure out by finding files with no more groups than pass ids?
// TODO: Is this actually the pass from mesh.flags2?
/// References to [WeightGroup] for each of the [RenderPassType](crate::mxmd::RenderPassType).
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Clone, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct WeightLod {
    /// One plus the indices pointing back to [groups](struct.Weights.html#structfield.groups).
    /// Unused entries use the value `0`.
    ///
    /// Each [Mesh](crate::mxmd::Mesh) indexes into this list using a hardcoded remapping
    /// for the [RenderPassType](crate::mxmd::RenderPassType) of the assigned material.
    // TODO: Document each entry.
    pub group_indices_plus_one: [u16; 9],
}

#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct Unk {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    #[br(parse_with = parse_count32_offset32, offset = base_offset)]
    #[xc3(count_offset(u32, u32))]
    pub buffers: Vec<UnkBufferDescriptor>,

    // The length of the data in bytes.
    pub data_length: u32,

    /// The offset into [buffer](struct.VertexData.html#structfield.buffer).
    pub data_offset: u32,

    // TODO: Padding?
    pub unks: [u32; 8],
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct UnkBufferDescriptor {
    pub unk1: u16,
    pub unk2: u16, // TODO: index?
    pub count: u32,
    pub offset: u32,
    pub unk5: u32,
    pub start_index: u32,
}

/// Extra data assigned to a non skin weights buffer.
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct VertexBufferExtInfo {
    pub flags: VertexBufferExtInfoFlags,
    // TODO: Extra attributes for outline meshes?
    pub outline_buffer_index: u16,
    /// Identical to [target_start_index](struct.MorphDescriptor.html#structfield.target_start_index)
    /// for the corresponding [MorphDescriptor].
    pub morph_target_start_index: u16,
    // TODO: Why is this off by 2?
    /// Identical to [target_count](struct.MorphDescriptor.html#structfield.target_count) + 2
    /// for the corresponding [MorphDescriptor].
    pub morph_target_count: u16,
    // TODO: padding?
    pub unk: u32,
}

#[bitsize(16)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(DebugBits, FromBits, BinRead, BinWrite, Clone, Copy, PartialEq)]
#[br(map = u16::into)]
#[bw(map = |&x| u16::from(x))]
pub struct VertexBufferExtInfoFlags {
    pub has_outline_buffer: bool,
    pub has_morph_targets: bool,
    pub unk: u14,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct OutlineBufferDescriptor {
    /// The offset into [buffer](struct.VertexData.html#structfield.buffer).
    pub data_offset: u32,
    pub vertex_count: u32,
    /// The size or stride of the vertex in bytes.
    pub vertex_size: u32,
    // TODO: padding?
    pub unk: u32,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct UnkData {
    pub unk1: u32, // 1
    pub vertex_data_offset: u32,
    pub vertex_data_length: u32,
    pub uniform_data_offset: u32,
    pub uniform_data_length: u32,
    pub vertex_count: u32,
    pub vertex_size: u32,
    // TODO: AABB for vertices?
    pub unk: [f32; 6],
}

impl DataType {
    pub fn size_in_bytes(&self) -> usize {
        match self {
            DataType::Position => 12,
            DataType::SkinWeights2 => 12,
            DataType::BoneIndices2 => 4,
            DataType::WeightIndex => 4,
            DataType::WeightIndex2 => 4,
            DataType::TexCoord0 => 8,
            DataType::TexCoord1 => 8,
            DataType::TexCoord2 => 8,
            DataType::TexCoord3 => 8,
            DataType::TexCoord4 => 8,
            DataType::TexCoord5 => 8,
            DataType::TexCoord6 => 8,
            DataType::TexCoord7 => 8,
            DataType::TexCoord8 => 8,
            DataType::Blend => 4,
            DataType::Unk15 => 12,
            DataType::Unk16 => 8,
            DataType::VertexColor => 4,
            DataType::Unk18 => 12,
            DataType::Unk24 => 16,
            DataType::Unk25 => 16,
            DataType::Unk26 => 16,
            DataType::Normal => 4,
            DataType::Tangent => 4,
            DataType::Unk30 => 4, // TODO: size?
            DataType::Unk31 => 4,
            DataType::Normal2 => 4,
            DataType::ValInf => 4,
            DataType::Normal3 => 4,
            DataType::VertexColor3 => 4,
            DataType::Position2 => 12,
            DataType::Normal4 => 4,
            DataType::OldPosition => 12,
            DataType::Tangent2 => 4,
            DataType::SkinWeights => 8,
            DataType::BoneIndices => 4,
            DataType::Flow => 2,
        }
    }
}

impl From<DataType> for VertexAttribute {
    fn from(data_type: DataType) -> Self {
        Self {
            data_type,
            data_size: data_type.size_in_bytes() as u16,
        }
    }
}

xc3_write_binwrite_impl!(
    DataType,
    PrimitiveType,
    IndexFormat,
    MorphTarget,
    VertexBufferExtInfoFlags
);

fn buffer_info_count(vertex_buffers: &[VertexBufferDescriptor]) -> usize {
    // TODO: Extra data for every buffer except the single weights buffer?
    vertex_buffers
        .iter()
        .filter(|b| {
            !b.attributes
                .iter()
                .any(|a| a.data_type == DataType::SkinWeights)
        })
        .count()
}

impl Xc3WriteOffsets for VertexDataOffsets<'_> {
    type Args = ();

    fn write_offsets<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        _base_offset: u64,
        data_ptr: &mut u64,
        endian: xc3_write::Endian,
        _args: Self::Args,
    ) -> xc3_write::Xc3Result<()> {
        let base_offset = self.base_offset;

        let vertex_buffers = self
            .vertex_buffers
            .write(writer, base_offset, data_ptr, endian)?;
        self.index_buffers
            .write(writer, base_offset, data_ptr, endian)?;
        self.vertex_buffer_info
            .write(writer, base_offset, data_ptr, endian)?;

        // TODO: Do all empty lists use offset 0?
        if !self.outline_buffers.data.is_empty() {
            self.outline_buffers
                .write(writer, base_offset, data_ptr, endian)?;
        }

        // The first attribute is aligned to 16.
        // TODO: This doesn't always happen?
        // *data_ptr = data_ptr.next_multiple_of(16);
        for vertex_buffer in vertex_buffers.0 {
            vertex_buffer
                .attributes
                .write(writer, base_offset, data_ptr, endian)?;
        }

        self.weights
            .write_full(writer, base_offset, data_ptr, endian, ())?;

        self.unk_data.write(writer, base_offset, data_ptr, endian)?;

        if let Some(vertex_animation) =
            self.vertex_morphs
                .write(writer, base_offset, data_ptr, endian)?
        {
            let descriptors =
                vertex_animation
                    .descriptors
                    .write(writer, base_offset, data_ptr, endian)?;
            vertex_animation
                .targets
                .write(writer, base_offset, data_ptr, endian)?;

            for descriptor in descriptors.0 {
                descriptor
                    .param_indices
                    .write(writer, base_offset, data_ptr, endian)?;
            }
        }

        self.unk7
            .write_full(writer, base_offset, data_ptr, endian, ())?;

        self.buffer.write(writer, base_offset, data_ptr, endian)?;

        Ok(())
    }
}
