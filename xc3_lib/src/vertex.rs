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
//! and additional indexing information defined in [Weights] for the starting index.
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
#[derive(Debug, Xc3Write, PartialEq, Clone)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct VertexData {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    // TODO: Sometimes 80 and sometimes 84?
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

    #[br(parse_with = parse_ptr32)]
    #[br(args { offset: base_offset, inner: args! { count: buffer_info_count(&vertex_buffers) }})]
    #[xc3(offset(u32))]
    pub vertex_buffer_info: Vec<VertexBufferExtInfo>,

    // 332 bytes of data?
    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub outline_buffers: Vec<OutlineBuffer>,

    #[br(parse_with = parse_opt_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub vertex_morphs: Option<VertexMorphs>,

    /// The data buffer containing all the geometry data.
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
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Eq, Clone, Copy)]
pub struct VertexAttribute {
    pub data_type: DataType,
    /// The size in bytes of [data_type](#structfield.data_type).
    pub data_size: u16,
}

// Format is taken from RenderDoc debugging.
// Names are taken from shader attribute metadata.
/// The data type, usage, and component count for a [VertexAttribute].
#[derive(Debug, BinRead, BinWrite, PartialEq, Eq, Clone, Copy)]
#[brw(repr(u16))]
pub enum DataType {
    /// Float32x3 "vPos" in shaders.
    Position = 0,
    /// ??? "fWeight" in shaders.
    Unk1 = 1,
    Unk2 = 2,
    /// Uint16x2 "nWgtIdx" in shaders.
    ///
    /// The index in the first component selects elements in the precomputed skinning matrices in the vertex shader.
    /// See [Weights] for details.
    WeightIndex = 3,
    /// Uint16x2 "nWgtIdx" in shaders.
    ///
    /// Used for some stage models.
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
    TexCoord9 = 13,
    /// Unorm8x4 "vBlend" in shaders.
    Blend = 14,
    Unk15 = 15,
    Unk16 = 16,
    /// Unorm8x4 "vColor" in shaders.
    VertexColor = 17,
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
    /// Snorm8x4 "vNormal" in shaders.
    Normal2 = 32,
    Unk33 = 33,
    /// Snorm8x4 "vNormal" in shaders.
    Normal3 = 34,
    /// Unorm8x4 "vColor" in shaders.
    VertexColor3 = 35,
    /// Float32x3 "vPos" in shaders.
    Position2 = 36,
    /// Unorm8x4 "vNormal" in shaders.
    Normal4 = 37,
    /// Float32x3 "vOldPos" in shaders.
    OldPosition = 39,
    /// Unorm8x4 "vTan" in shaders.
    Tangent2 = 40,
    /// Unorm16x4 skin weights for up to 4 bone influences.
    SkinWeights = 41,
    /// Uint8x4 bone indices for up to 4 bone infuences in the [Skinning](crate::mxmd::Skinning) in the [Mxmd](crate::mxmd::Mxmd).
    BoneIndices = 42,
    /// ??? "vFlow" in shaders.
    Flow = 52,
}

// TODO: Is this data always u16?
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Eq, Clone)]
pub struct IndexBufferDescriptor {
    /// The offset into [buffer](struct.VertexData.html#structfield.buffer).
    pub data_offset: u32,
    pub index_count: u32,
    pub unk1: Unk1, // TODO: primitive type?
    pub unk2: Unk2, // TODO: index format?
    // TODO: padding?
    pub unk3: u32,
    pub unk4: u32,
}

#[derive(Debug, BinRead, BinWrite, PartialEq, Eq, Clone, Copy)]
#[brw(repr(u16))]
pub enum Unk1 {
    Unk0 = 0,
    Unk3 = 3,
}

#[derive(Debug, BinRead, BinWrite, PartialEq, Eq, Clone, Copy)]
#[brw(repr(u16))]
pub enum Unk2 {
    Unk0 = 0,
}

/// Vertex animation data often called "vertex morphs", "shape keys", or "blend shapes".
#[derive(Debug, BinRead, Xc3Write, Clone, PartialEq)]
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

#[derive(Debug, BinRead, Xc3Write, Clone, PartialEq)]
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
#[derive(Debug, BinRead, BinWrite, Clone, PartialEq)]
pub struct MorphTarget {
    /// Relative to [data_base_offset](struct.ModelData.html#structfield.data_base_offset)
    pub data_offset: u32,
    pub vertex_count: u32,
    pub vertex_size: u32,

    pub flags: MorphTargetFlags,
}

#[bitsize(32)]
#[derive(DebugBits, FromBits, BinRead, BinWrite, Clone, Copy, PartialEq)]
#[br(map = u32::into)]
#[bw(map = |&x| u32::from(x))]
pub struct MorphTargetFlags {
    pub unk1: u16,                 // always 0?
    pub blend_target_buffer: bool, // once per descriptor?
    pub default_buffer: bool,      // once per descriptor?
    pub param_buffer: bool,
    pub unk5: u13, // always 0?
}

// TODO: document the entire process using all attributes?
/// Information used for precomputing skinning matrices
/// based on a mesh's level of detail (LOD) and render pass type.
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, Clone, PartialEq)]
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

/// A range of elements in the weights buffer.
/// Each element in the weights buffer is part of at least one [WeightGroup].
#[derive(Debug, Clone, PartialEq, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct WeightGroup {
    /// Index into the skinning buffer used in the vertex shader with bone transforms multiplied by skin weights.
    // TODO: Is this essentially added to WeightIndex attribute?
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
// [unk0, unk1 ???, unk7, ???, ???, ???, ???, ???]

// group_index = weights.weight_lods[mesh.lod].group_indices_plus_one[material.program.pass_index] - 1
// group = weights.groups[group_index]

// TODO: What indexes into this?
// TODO: something related to render pass?
#[derive(Debug, Clone, PartialEq, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct WeightLod {
    /// One plus the indices pointing back to [groups](struct.Weights.html#structfield.groups).
    /// Unused entries use the value `0`.
    pub group_indices_plus_one: [u16; 9],
}

#[binread]
#[derive(Debug, Xc3Write, Xc3WriteOffsets, Clone, PartialEq)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct Unk {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    #[br(parse_with = parse_count32_offset32, offset = base_offset)]
    #[xc3(count_offset(u32, u32))]
    pub unk1: Vec<UnkInner>,

    // The length of the data in bytes.
    pub data_length: u32,

    /// The offset into [buffer](struct.VertexData.html#structfield.buffer).
    pub data_offset: u32,

    // TODO: Padding?
    pub unks: [u32; 8],
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, Clone, PartialEq)]
pub struct UnkInner {
    pub unk1: u16,
    pub unk2: u16,
    pub count: u32,
    pub offset: u32,
    pub unk5: u32,
    // sum of previous counts?
    pub start_index: u32,
}

/// Extra data assigned to a non skin weights buffer.
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, Clone, PartialEq)]
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
#[derive(DebugBits, FromBits, BinRead, BinWrite, Clone, Copy, PartialEq)]
#[br(map = u16::into)]
#[bw(map = |&x| u16::from(x))]
pub struct VertexBufferExtInfoFlags {
    pub has_outline_buffer: bool,
    pub unk2: bool,
    pub unk: u14,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, Clone, PartialEq)]
pub struct OutlineBuffer {
    /// The offset into [buffer](struct.VertexData.html#structfield.buffer).
    pub data_offset: u32,
    pub vertex_count: u32,
    /// The size or stride of the vertex in bytes.
    pub vertex_size: u32,
    // TODO: padding?
    pub unk: u32,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct UnkData {
    pub unk: [u32; 17],
}

xc3_write_binwrite_impl!(DataType, Unk1, Unk2, MorphTarget, VertexBufferExtInfoFlags);

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

impl<'a> Xc3WriteOffsets for VertexDataOffsets<'a> {
    fn write_offsets<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        _base_offset: u64,
        data_ptr: &mut u64,
    ) -> xc3_write::Xc3Result<()> {
        let base_offset = self.base_offset;

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
