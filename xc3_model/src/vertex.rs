//! Utilities for working with vertex buffer data.
//!
//! The main type for representing vertex data is [AttributeData].
//! Storing the values separately like this is often called a "struct of arrays" layout.
//! This makes editing individual attributes cache friendly and makes it easy to define different attributes.
//! This approach is often preferred for 3D modeling applications and some file formats.
//!
//! The vertex buffers in game use an interleaved or "array of structs" approach.
//! This makes rendering each vertex cache friendly.
//! A collection of [AttributeData] can always be packed into an interleaved form for rendering.
use std::{
    collections::BTreeSet,
    io::{Cursor, Read, Seek, SeekFrom, Write},
};

use binrw::{BinRead, BinReaderExt, BinResult, BinWrite, Endian};
use glam::{Vec2, Vec3, Vec4};
use xc3_lib::vertex::{
    DataType, IndexBufferDescriptor, MorphDescriptor, MorphTargetFlags, OutlineBufferDescriptor,
    Unk, UnkBufferDescriptor, UnkData, VertexBufferDescriptor, VertexBufferExtInfo,
    VertexBufferExtInfoFlags, VertexData,
};

pub use xc3_lib::vertex::{WeightGroup, WeightLod};

use crate::skinning::{SkinWeights, WeightGroups, Weights};

#[cfg(feature = "arbitrary")]
use crate::{arbitrary_vec2s, arbitrary_vec3s, arbitrary_vec4s};

// Reduce out of memory errors by not preallocating unrealistic buffer sizes.
const MAX_VERT_CAPACITY: u32 = u16::MAX as u32;

/// See [VertexData].
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Clone)]
pub struct ModelBuffers {
    pub vertex_buffers: Vec<VertexBuffer>,
    pub outline_buffers: Vec<OutlineBuffer>,
    pub index_buffers: Vec<IndexBuffer>,
    pub unk_buffers: Vec<UnkBuffer>,
    pub unk_data: Option<UnkDataBuffer>,
    pub weights: Option<Weights>,
}

/// See [VertexBufferDescriptor].
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Clone)]
pub struct VertexBuffer {
    pub attributes: Vec<AttributeData>,
    /// Default values for positions, normals, and tangents if morph targets are present.
    pub morph_blend_target: Vec<AttributeData>,
    /// Animation targets for vertex attributes like positions and normals.
    /// The base target is already applied to [attributes](#structfield.attributes).
    pub morph_targets: Vec<MorphTarget>,
    pub outline_buffer_index: Option<usize>,
}

/// Morph target attributes defined as a difference or deformation from the base target.
///
/// The final attribute values are simply `base + target * weight`.
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Clone)]
pub struct MorphTarget {
    /// Index into [morph_controller_names](../struct.Models.html#structfield.morph_controller_names).
    pub morph_controller_index: usize,

    // TODO: Add a method with tests to blend with base target?
    #[cfg_attr(feature = "arbitrary", arbitrary(with = arbitrary_vec3s))]
    pub position_deltas: Vec<Vec3>,

    #[cfg_attr(feature = "arbitrary", arbitrary(with = arbitrary_vec4s))]
    pub normals: Vec<Vec4>,

    #[cfg_attr(feature = "arbitrary", arbitrary(with = arbitrary_vec4s))]
    pub tangents: Vec<Vec4>,

    /// The indices of the vertices affected by the deltas.
    // TODO: method to convert to a non sparse format?
    pub vertex_indices: Vec<u32>,
}

/// See [OutlineBufferDescriptor].
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Clone)]
pub struct OutlineBuffer {
    /// The additional attributes used for outline mesh rendering.
    /// This should usually be [AttributeData::Normal] and [AttributeData::VertexColor].
    /// Buffers for meshes with [MorphTarget] should omit the [AttributeData::Normal].
    pub attributes: Vec<AttributeData>,
}

/// See [UnkBufferDescriptor].
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Clone)]
pub struct UnkBuffer {
    pub unk2: u16,
    pub attributes: Vec<AttributeData>,
}

/// See [IndexBufferDescriptor].
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Clone)]
pub struct IndexBuffer {
    // TODO: support u32?
    pub indices: Vec<u16>,
}

// TODO: particles in xc2f wiki?
/// See [UnkData].
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Clone)]
pub struct UnkDataBuffer {
    // TODO: find the data type used in game?
    pub attribute1: Vec<[u32; 3]>,
    pub attribute2: Vec<u32>,
    pub uniform_data: Vec<u8>,
    pub unk: [f32; 6],
}

impl VertexBuffer {
    pub fn vertex_count(&self) -> usize {
        // TODO: Check all attributes for consistency?
        self.attributes.first().map(|a| a.len()).unwrap_or_default()
    }
}

// TODO: Add an option to convert a collection of these to the vertex above?
// TODO: How to handle normalized attributes?
// TODO: Link to appropriate xc3_lib types and fields.
/// Per vertex values for a vertex attribute.
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Clone)]
pub enum AttributeData {
    /// Data for [DataType::Position].
    Position(#[cfg_attr(feature = "arbitrary", arbitrary(with = arbitrary_vec3s))] Vec<Vec3>),

    /// Data for [DataType::SkinWeights2].
    SkinWeights2(#[cfg_attr(feature = "arbitrary", arbitrary(with = arbitrary_vec3s))] Vec<Vec3>),

    /// Data for [DataType::BoneIndices2].
    BoneIndices2(Vec<[u8; 4]>),

    /// Data for [DataType::WeightIndex].
    WeightIndex(Vec<[u16; 2]>),

    /// Data for [DataType::WeightIndex2].
    WeightIndex2(Vec<[u16; 2]>),

    /// Data for [DataType::TexCoord0].
    TexCoord0(#[cfg_attr(feature = "arbitrary", arbitrary(with = arbitrary_vec2s))] Vec<Vec2>),

    /// Data for [DataType::TexCoord1].
    TexCoord1(#[cfg_attr(feature = "arbitrary", arbitrary(with = arbitrary_vec2s))] Vec<Vec2>),

    /// Data for [DataType::TexCoord2].
    TexCoord2(#[cfg_attr(feature = "arbitrary", arbitrary(with = arbitrary_vec2s))] Vec<Vec2>),

    /// Data for [DataType::TexCoord3].
    TexCoord3(#[cfg_attr(feature = "arbitrary", arbitrary(with = arbitrary_vec2s))] Vec<Vec2>),

    /// Data for [DataType::TexCoord4].
    TexCoord4(#[cfg_attr(feature = "arbitrary", arbitrary(with = arbitrary_vec2s))] Vec<Vec2>),

    /// Data for [DataType::TexCoord5].
    TexCoord5(#[cfg_attr(feature = "arbitrary", arbitrary(with = arbitrary_vec2s))] Vec<Vec2>),

    /// Data for [DataType::TexCoord6].
    TexCoord6(#[cfg_attr(feature = "arbitrary", arbitrary(with = arbitrary_vec2s))] Vec<Vec2>),

    /// Data for [DataType::TexCoord7].
    TexCoord7(#[cfg_attr(feature = "arbitrary", arbitrary(with = arbitrary_vec2s))] Vec<Vec2>),

    /// Data for [DataType::TexCoord8].
    TexCoord8(#[cfg_attr(feature = "arbitrary", arbitrary(with = arbitrary_vec2s))] Vec<Vec2>),

    /// Data for [DataType::Blend].
    Blend(#[cfg_attr(feature = "arbitrary", arbitrary(with = arbitrary_vec4s))] Vec<Vec4>),

    /// Data for [DataType::Unk15].
    Unk15(#[cfg_attr(feature = "arbitrary", arbitrary(with = arbitrary_vec3s))] Vec<Vec3>),

    // TODO: Is this actually 2 snorm8x4?
    /// Data for [DataType::Unk16].
    Unk16(Vec<[u16; 2]>),

    // TODO: unk16
    /// Data for [DataType::VertexColor].
    VertexColor(#[cfg_attr(feature = "arbitrary", arbitrary(with = arbitrary_vec4s))] Vec<Vec4>),

    /// Data for [DataType::Unk18].
    Unk18(#[cfg_attr(feature = "arbitrary", arbitrary(with = arbitrary_vec3s))] Vec<Vec3>),

    // TODO: unk24, unk25, unk26
    /// Data for [DataType::Normal].
    Normal(#[cfg_attr(feature = "arbitrary", arbitrary(with = arbitrary_vec4s))] Vec<Vec4>),

    /// Data for [DataType::Tangent].
    Tangent(#[cfg_attr(feature = "arbitrary", arbitrary(with = arbitrary_vec4s))] Vec<Vec4>),

    // TODO: unk30
    /// Data for [DataType::Unk31].
    Unk31(#[cfg_attr(feature = "arbitrary", arbitrary(with = arbitrary_vec4s))] Vec<Vec4>),

    /// Data for [DataType::Normal2].
    Normal2(#[cfg_attr(feature = "arbitrary", arbitrary(with = arbitrary_vec4s))] Vec<Vec4>),

    /// Data for [DataType::ValInf].
    ValInf(#[cfg_attr(feature = "arbitrary", arbitrary(with = arbitrary_vec4s))] Vec<Vec4>),

    // TODO: normal3, vertexcolor3

    // TODO: morph only?
    /// Data for [DataType::Position2].
    Position2(#[cfg_attr(feature = "arbitrary", arbitrary(with = arbitrary_vec3s))] Vec<Vec3>),

    /// Data for [DataType::Normal4].
    /// Values should be used as `v * 2.0 - 1.0`.
    Normal4(#[cfg_attr(feature = "arbitrary", arbitrary(with = arbitrary_vec4s))] Vec<Vec4>),

    /// Data for [DataType::OldPosition].
    OldPosition(#[cfg_attr(feature = "arbitrary", arbitrary(with = arbitrary_vec3s))] Vec<Vec3>),

    /// Data for [DataType::Tangent2].
    /// Values should be used as `v * 2.0 - 1.0`.
    Tangent2(#[cfg_attr(feature = "arbitrary", arbitrary(with = arbitrary_vec4s))] Vec<Vec4>),

    // TODO: weight buffer only?
    /// Data for [DataType::SkinWeights].
    SkinWeights(#[cfg_attr(feature = "arbitrary", arbitrary(with = arbitrary_vec4s))] Vec<Vec4>),

    /// Data for [DataType::BoneIndices].
    BoneIndices(Vec<[u8; 4]>),
    // TODO: flow
}

impl AttributeData {
    pub fn len(&self) -> usize {
        match self {
            AttributeData::Position(v) => v.len(),
            AttributeData::SkinWeights2(v) => v.len(),
            AttributeData::BoneIndices2(v) => v.len(),
            AttributeData::WeightIndex(v) => v.len(),
            AttributeData::WeightIndex2(v) => v.len(),
            AttributeData::TexCoord0(v) => v.len(),
            AttributeData::TexCoord1(v) => v.len(),
            AttributeData::TexCoord2(v) => v.len(),
            AttributeData::TexCoord3(v) => v.len(),
            AttributeData::TexCoord4(v) => v.len(),
            AttributeData::TexCoord5(v) => v.len(),
            AttributeData::TexCoord6(v) => v.len(),
            AttributeData::TexCoord7(v) => v.len(),
            AttributeData::TexCoord8(v) => v.len(),
            AttributeData::Blend(v) => v.len(),
            AttributeData::Unk15(v) => v.len(),
            AttributeData::Unk16(v) => v.len(),
            AttributeData::VertexColor(v) => v.len(),
            AttributeData::Unk18(v) => v.len(),
            AttributeData::Normal(v) => v.len(),
            AttributeData::Tangent(v) => v.len(),
            AttributeData::Unk31(v) => v.len(),
            AttributeData::Normal2(v) => v.len(),
            AttributeData::ValInf(v) => v.len(),
            AttributeData::Position2(v) => v.len(),
            AttributeData::Normal4(v) => v.len(),
            AttributeData::OldPosition(v) => v.len(),
            AttributeData::Tangent2(v) => v.len(),
            AttributeData::SkinWeights(v) => v.len(),
            AttributeData::BoneIndices(v) => v.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn write<W: Write + Seek>(
        &self,
        writer: &mut W,
        offset: u64,
        stride: u64,
        endian: Endian,
    ) -> BinResult<()> {
        let a = AttributeWriteArgs {
            offset,
            stride,
            endian,
        };
        match self {
            AttributeData::Position(values) => a.write(writer, values, write_f32x3),
            AttributeData::SkinWeights2(values) => a.write(writer, values, write_f32x3),
            AttributeData::BoneIndices2(values) => a.write(writer, values, write_u8x4),
            AttributeData::WeightIndex(values) => a.write(writer, values, write_u16x2),
            AttributeData::WeightIndex2(values) => a.write(writer, values, write_u16x2),
            AttributeData::TexCoord0(values) => a.write(writer, values, write_f32x2),
            AttributeData::TexCoord1(values) => a.write(writer, values, write_f32x2),
            AttributeData::TexCoord2(values) => a.write(writer, values, write_f32x2),
            AttributeData::TexCoord3(values) => a.write(writer, values, write_f32x2),
            AttributeData::TexCoord4(values) => a.write(writer, values, write_f32x2),
            AttributeData::TexCoord5(values) => a.write(writer, values, write_f32x2),
            AttributeData::TexCoord6(values) => a.write(writer, values, write_f32x2),
            AttributeData::TexCoord7(values) => a.write(writer, values, write_f32x2),
            AttributeData::TexCoord8(values) => a.write(writer, values, write_f32x2),
            AttributeData::Blend(values) => a.write(writer, values, write_unorm8x4),
            AttributeData::Unk15(values) => a.write(writer, values, write_f32x3),
            AttributeData::Unk16(values) => a.write(writer, values, write_u16x2),
            AttributeData::VertexColor(values) => a.write(writer, values, write_unorm8x4),
            AttributeData::Unk18(values) => a.write(writer, values, write_f32x3),
            AttributeData::Normal(values) => a.write(writer, values, write_snorm8x4),
            AttributeData::Tangent(values) => a.write(writer, values, write_snorm8x4),
            AttributeData::Unk31(values) => a.write(writer, values, write_unorm8x4),
            AttributeData::Normal2(values) => a.write(writer, values, write_snorm8x4),
            AttributeData::ValInf(values) => a.write(writer, values, write_snorm8x4),
            AttributeData::Position2(values) => a.write(writer, values, write_f32x3),
            AttributeData::Normal4(values) => a.write(writer, values, write_unorm8x4),
            AttributeData::OldPosition(values) => a.write(writer, values, write_f32x3),
            AttributeData::Tangent2(values) => a.write(writer, values, write_unorm8x4),
            AttributeData::SkinWeights(values) => a.write(writer, values, write_unorm16x4),
            AttributeData::BoneIndices(values) => a.write(writer, values, write_u8x4),
        }
    }

    pub fn data_type(&self) -> DataType {
        match self {
            AttributeData::Position(_) => DataType::Position,
            AttributeData::SkinWeights2(_) => DataType::SkinWeights2,
            AttributeData::BoneIndices2(_) => DataType::BoneIndices2,
            AttributeData::WeightIndex(_) => DataType::WeightIndex,
            AttributeData::WeightIndex2(_) => DataType::WeightIndex2,
            AttributeData::TexCoord0(_) => DataType::TexCoord0,
            AttributeData::TexCoord1(_) => DataType::TexCoord1,
            AttributeData::TexCoord2(_) => DataType::TexCoord2,
            AttributeData::TexCoord3(_) => DataType::TexCoord3,
            AttributeData::TexCoord4(_) => DataType::TexCoord4,
            AttributeData::TexCoord5(_) => DataType::TexCoord5,
            AttributeData::TexCoord6(_) => DataType::TexCoord6,
            AttributeData::TexCoord7(_) => DataType::TexCoord7,
            AttributeData::TexCoord8(_) => DataType::TexCoord8,
            AttributeData::VertexColor(_) => DataType::VertexColor,
            AttributeData::Unk18(_) => DataType::Unk18,
            AttributeData::Blend(_) => DataType::Blend,
            AttributeData::Unk15(_) => DataType::Unk15,
            AttributeData::Unk16(_) => DataType::Unk16,
            AttributeData::Normal(_) => DataType::Normal,
            AttributeData::Tangent(_) => DataType::Tangent,
            AttributeData::Unk31(_) => DataType::Unk31,
            AttributeData::Normal2(_) => DataType::Normal2,
            AttributeData::ValInf(_) => DataType::ValInf,
            AttributeData::Position2(_) => DataType::Position2,
            AttributeData::Normal4(_) => DataType::Normal4,
            AttributeData::OldPosition(_) => DataType::OldPosition,
            AttributeData::Tangent2(_) => DataType::Tangent2,
            AttributeData::SkinWeights(_) => DataType::SkinWeights,
            AttributeData::BoneIndices(_) => DataType::BoneIndices,
        }
    }
}

fn read_vertex_buffers(
    vertex_data: &VertexData,
    skinning: Option<&xc3_lib::mxmd::Skinning>,
) -> BinResult<(Vec<VertexBuffer>, Option<Weights>)> {
    // TODO: This skips the weights buffer since it doesn't have ext info?
    // TODO: Save the weights buffer for converting back to xc3_lib types?
    // TODO: Panic if the weights buffer is not the last buffer?
    // println!("{:#?}", vertex_data.vertex_buffers);
    let mut buffers = vertex_data
        .vertex_buffers
        .iter()
        .zip(vertex_data.vertex_buffer_info.iter())
        .map(|(descriptor, ext)| {
            let attributes = read_attributes(
                descriptor.data_offset as u64,
                descriptor.vertex_count,
                descriptor.vertex_size,
                &descriptor.attributes,
                &vertex_data.buffer,
                Endian::Little,
            )?;

            Ok(VertexBuffer {
                attributes,
                morph_blend_target: Vec::new(),
                morph_targets: Vec::new(),
                outline_buffer_index: ext
                    .flags
                    .has_outline_buffer()
                    .then_some(ext.outline_buffer_index as usize),
            })
        })
        .collect::<BinResult<Vec<_>>>()?;

    // TODO: Get names from the mxmd?
    // TODO: Add better tests for morph target data.
    if let Some(vertex_morphs) = &vertex_data.vertex_morphs {
        assign_morph_targets(vertex_morphs, &mut buffers, vertex_data)?;
    }

    // TODO: Is this the best place to do this?
    let skin_weights = skinning.and_then(|skinning| {
        let vertex_weights = vertex_data.weights.as_ref()?;
        let weights_index = vertex_weights.vertex_buffer_index as usize;

        let descriptor = vertex_data.vertex_buffers.get(weights_index)?;
        // TODO: return error instead?
        let attributes = read_attributes(
            descriptor.data_offset as u64,
            descriptor.vertex_count,
            descriptor.vertex_size,
            &descriptor.attributes,
            &vertex_data.buffer,
            Endian::Little,
        )
        .ok()?;

        let (weights, bone_indices) = skin_weights_bone_indices(&attributes)?;

        Some(Weights {
            weight_buffers: vec![SkinWeights {
                bone_indices,
                weights,
                // TODO: Will this cover all bone indices?
                bone_names: skinning.bones.iter().map(|b| b.name.clone()).collect(),
            }],
            weight_groups: WeightGroups::Groups {
                weight_groups: vertex_weights.groups.clone(),
                weight_lods: vertex_weights.weight_lods.clone(),
            },
        })
    });

    Ok((buffers, skin_weights))
}

fn outline_buffer(descriptor: &OutlineBufferDescriptor, buffer: &[u8]) -> BinResult<OutlineBuffer> {
    // TODO: This fails for legacy files like xc2 oj108004?
    Ok(OutlineBuffer {
        attributes: read_outline_buffer(descriptor, buffer)?,
    })
}

fn assign_morph_targets(
    vertex_morphs: &xc3_lib::vertex::VertexMorphs,
    buffers: &mut [VertexBuffer],
    vertex_data: &VertexData,
) -> BinResult<()> {
    // TODO: Find a cleaner way to write this.
    for descriptor in &vertex_morphs.descriptors {
        if let Some(buffer) = buffers.get_mut(descriptor.vertex_buffer_index as usize) {
            // Skip the default target since it can be generated when writing.
            if let Some((blend, _default, params)) = split_targets(descriptor, vertex_morphs) {
                let attributes = read_morph_blend_target(blend, &vertex_data.buffer)?;

                // TODO: What to do with the default target?
                buffer.morph_blend_target = attributes;
                buffer.morph_targets = params
                    .iter()
                    .zip(descriptor.param_indices.iter())
                    .map(|(target, param_index)| {
                        // Apply remaining targets onto the base target values.
                        // TODO: Lots of morph targets use the exact same bytes?
                        read_morph_target(target, &vertex_data.buffer, *param_index)
                    })
                    .collect::<BinResult<Vec<_>>>()?;
            }
        }
    }

    Ok(())
}

fn read_morph_target(
    target: &xc3_lib::vertex::MorphTarget,
    buffer: &[u8],
    param_index: u16,
) -> BinResult<MorphTarget> {
    let vertices = read_morph_buffer_target(target, buffer)?;

    let mut position_deltas = Vec::new();
    let mut normals = Vec::new();
    let mut tangents = Vec::new();
    let mut vertex_indices = Vec::new();

    // Keep the sparse representation to save space.
    // The vertex indices only contain affected vertices.
    for vertex in vertices {
        vertex_indices.push(vertex.vertex_index);
        position_deltas.push(vertex.position_delta);
        normals.push(vertex.normal);
        tangents.push(vertex.tangent);
    }

    Ok(MorphTarget {
        morph_controller_index: param_index as usize,
        position_deltas,
        normals,
        tangents,
        vertex_indices,
    })
}

fn split_targets<'a>(
    descriptor: &MorphDescriptor,
    vertex_morphs: &'a xc3_lib::vertex::VertexMorphs,
) -> Option<(
    &'a xc3_lib::vertex::MorphTarget,
    &'a xc3_lib::vertex::MorphTarget,
    &'a [xc3_lib::vertex::MorphTarget],
)> {
    // TODO: Check flags to get the type instead?
    // Assume the order is blend, default, params.
    let start = descriptor.target_start_index as usize;
    let count = descriptor.param_indices.len() + 2;
    let targets = vertex_morphs.targets.get(start..start + count)?;

    let (blend_target, targets) = targets.split_first()?;
    let (default_target, param_targets) = targets.split_first()?;

    Some((blend_target, default_target, param_targets))
}

fn skin_weights_bone_indices(attributes: &[AttributeData]) -> Option<(Vec<Vec4>, Vec<[u8; 4]>)> {
    // Support both modern and legacy attributes.
    let weights = attributes.iter().find_map(|a| match a {
        AttributeData::SkinWeights(values) => Some(values.clone()),
        AttributeData::SkinWeights2(values) => Some(
            values
                .iter()
                .map(|v| {
                    // Assume weights sum to 1.0.
                    v.extend(1.0 - v.element_sum())
                })
                .collect(),
        ),
        _ => None,
    })?;
    let indices = attributes.iter().find_map(|a| match a {
        AttributeData::BoneIndices(values) => Some(values.clone()),
        AttributeData::BoneIndices2(values) => Some(values.clone()),
        _ => None,
    })?;

    Some((weights, indices))
}

fn read_index_buffers(vertex_data: &VertexData, endian: Endian) -> BinResult<Vec<IndexBuffer>> {
    vertex_data
        .index_buffers
        .iter()
        .map(|descriptor| {
            Ok(IndexBuffer {
                indices: read_indices(descriptor, &vertex_data.buffer, endian)?,
            })
        })
        .collect()
}

fn read_indices(
    descriptor: &IndexBufferDescriptor,
    buffer: &[u8],
    endian: Endian,
) -> BinResult<Vec<u16>> {
    // TODO: Also read u32 indices.
    let mut reader = Cursor::new(buffer);
    reader.seek(SeekFrom::Start(descriptor.data_offset as u64))?;

    let mut indices = Vec::with_capacity(descriptor.index_count.min(MAX_VERT_CAPACITY) as usize);
    for _ in 0..descriptor.index_count {
        let index: u16 = reader.read_type(endian)?;
        indices.push(index);
    }
    Ok(indices)
}

fn read_attributes(
    data_offset: u64,
    vertex_count: u32,
    vertex_size: u32,
    attributes: &[xc3_lib::vertex::VertexAttribute],
    buffer: &[u8],
    endian: Endian,
) -> BinResult<Vec<AttributeData>> {
    let mut offset = 0;
    attributes
        .iter()
        .map(|a| {
            let data = read_attribute(
                a.data_type,
                data_offset,
                vertex_count,
                vertex_size,
                offset,
                buffer,
                endian,
            );
            offset += a.data_size as u64;
            data
        })
        .collect()
}

fn read_attribute(
    data_type: DataType,
    data_offset: u64,
    vertex_count: u32,
    vertex_size: u32,
    relative_offset: u64,
    buffer: &[u8],
    endian: Endian,
) -> BinResult<AttributeData> {
    let a = AttributeReadArgs {
        offset: data_offset,
        count: vertex_count,
        stride: vertex_size,
        relative_offset,
        endian,
    };
    let b = buffer;

    match data_type {
        DataType::Position => a.read(b, read_f32x3).map(AttributeData::Position),
        DataType::SkinWeights2 => a.read(b, read_f32x3).map(AttributeData::SkinWeights2),
        DataType::BoneIndices2 => a.read(b, read_u8x4).map(AttributeData::BoneIndices2),
        DataType::WeightIndex => a.read(b, read_u16x2).map(AttributeData::WeightIndex),
        DataType::WeightIndex2 => a.read(b, read_u16x2).map(AttributeData::WeightIndex2),
        DataType::TexCoord0 => a.read(b, read_f32x2).map(AttributeData::TexCoord0),
        DataType::TexCoord1 => a.read(b, read_f32x2).map(AttributeData::TexCoord1),
        DataType::TexCoord2 => a.read(b, read_f32x2).map(AttributeData::TexCoord2),
        DataType::TexCoord3 => a.read(b, read_f32x2).map(AttributeData::TexCoord3),
        DataType::TexCoord4 => a.read(b, read_f32x2).map(AttributeData::TexCoord4),
        DataType::TexCoord5 => a.read(b, read_f32x2).map(AttributeData::TexCoord5),
        DataType::TexCoord6 => a.read(b, read_f32x2).map(AttributeData::TexCoord6),
        DataType::TexCoord7 => a.read(b, read_f32x2).map(AttributeData::TexCoord7),
        DataType::TexCoord8 => a.read(b, read_f32x2).map(AttributeData::TexCoord8),
        DataType::Blend => a.read(b, read_unorm8x4).map(AttributeData::Blend),
        DataType::Unk15 => a.read(b, read_f32x3).map(AttributeData::Unk15),
        DataType::Unk16 => a.read(b, read_u16x2).map(AttributeData::Unk16),
        DataType::VertexColor => a.read(b, read_unorm8x4).map(AttributeData::VertexColor),
        DataType::Unk18 => a.read(b, read_f32x3).map(AttributeData::Unk18),
        DataType::Unk24 => todo!(),
        DataType::Unk25 => todo!(),
        DataType::Unk26 => todo!(),
        DataType::Normal => a.read(b, read_snorm8x4).map(AttributeData::Normal),
        DataType::Tangent => a.read(b, read_snorm8x4).map(AttributeData::Tangent),
        DataType::Unk30 => todo!(),
        DataType::Unk31 => a.read(b, read_unorm8x4).map(AttributeData::Unk31),
        DataType::Normal2 => a.read(b, read_snorm8x4).map(AttributeData::Normal2),
        DataType::ValInf => a.read(b, read_snorm8x4).map(AttributeData::ValInf),
        DataType::Normal3 => todo!(),
        DataType::VertexColor3 => todo!(),
        DataType::Position2 => a.read(b, read_f32x3).map(AttributeData::Position2),
        DataType::Normal4 => a.read(b, read_unorm8x4).map(AttributeData::Normal4),
        DataType::OldPosition => a.read(b, read_f32x3).map(AttributeData::OldPosition),
        DataType::Tangent2 => a.read(b, read_unorm8x4).map(AttributeData::Tangent2),
        DataType::SkinWeights => a.read(b, read_unorm16x4).map(AttributeData::SkinWeights),
        DataType::BoneIndices => a.read(b, read_u8x4).map(AttributeData::BoneIndices),
        DataType::Flow => todo!(),
    }
}

struct AttributeReadArgs {
    offset: u64,
    count: u32,
    stride: u32,
    relative_offset: u64,
    endian: Endian,
}

impl AttributeReadArgs {
    fn read<T, F>(&self, buffer: &[u8], read_item: F) -> BinResult<Vec<T>>
    where
        F: Fn(&mut Cursor<&[u8]>, Endian) -> BinResult<T>,
    {
        if self.stride == 0 {
            return Err(binrw::Error::AssertFail {
                pos: self.offset,
                message: "Attribute stride must not be 0".to_string(),
            });
        }

        let mut reader = Cursor::new(buffer);

        let mut values = Vec::with_capacity(self.count.min(MAX_VERT_CAPACITY) as usize);
        for i in 0..self.count as u64 {
            let offset = self.offset + i * self.stride as u64 + self.relative_offset;
            reader.seek(SeekFrom::Start(offset))?;

            values.push(read_item(&mut reader, self.endian)?);
        }
        Ok(values)
    }
}

struct AttributeWriteArgs {
    offset: u64,
    stride: u64,
    endian: Endian,
}

impl AttributeWriteArgs {
    fn write<T, F, W>(&self, writer: &mut W, values: &[T], write_item: F) -> BinResult<()>
    where
        W: Write + Seek,
        F: Fn(&mut W, &T, Endian) -> BinResult<()>,
    {
        for (i, value) in values.iter().enumerate() {
            writer.seek(SeekFrom::Start(self.offset + i as u64 * self.stride))?;
            write_item(writer, value, self.endian)?;
        }

        Ok(())
    }
}

fn read_u16x2(reader: &mut Cursor<&[u8]>, endian: Endian) -> BinResult<[u16; 2]> {
    reader.read_type(endian)
}

fn read_u8x4(reader: &mut Cursor<&[u8]>, endian: Endian) -> BinResult<[u8; 4]> {
    reader.read_type(endian)
}

fn read_f32x2(reader: &mut Cursor<&[u8]>, endian: Endian) -> BinResult<Vec2> {
    let value: [f32; 2] = reader.read_type(endian)?;
    Ok(value.into())
}

fn read_f32x3(reader: &mut Cursor<&[u8]>, endian: Endian) -> BinResult<Vec3> {
    let value: [f32; 3] = reader.read_type(endian)?;
    Ok(value.into())
}

fn read_unorm8x4(reader: &mut Cursor<&[u8]>, endian: Endian) -> BinResult<Vec4> {
    let value: [u8; 4] = reader.read_type(endian)?;
    Ok(value.map(|u| u as f32 / 255.0).into())
}

fn read_snorm8x4(reader: &mut Cursor<&[u8]>, endian: Endian) -> BinResult<Vec4> {
    let value: [i8; 4] = reader.read_type(endian)?;
    Ok(value.map(|i| i as f32 / 127.0).into())
}

fn read_unorm16x4(reader: &mut Cursor<&[u8]>, endian: Endian) -> BinResult<Vec4> {
    let value: [u16; 4] = reader.read_type(endian)?;
    Ok(value.map(|u| u as f32 / 65535.0).into())
}

// Default and param buffer attributes.
#[derive(BinRead, BinWrite)]
struct MorphBufferTargetVertex {
    // Relative to blend target.
    position_delta: [f32; 3],
    _unk1: u32,
    normal: [u8; 4],
    tangent: [u8; 4],
    _unk2: u32,
    vertex_index: u32,
}

// Final data as interpreted by the shader.
// This simplifies non rendering applications.
#[derive(Debug, PartialEq)]
struct MorphTargetVertex {
    position_delta: Vec3,
    normal: Vec4,
    tangent: Vec4,
    vertex_index: u32,
}

fn read_morph_blend_target(
    base_target: &xc3_lib::vertex::MorphTarget,
    model_bytes: &[u8],
) -> BinResult<Vec<AttributeData>> {
    // Only the base target contains data for all vertices.
    // This includes required position, normal, and tangent attributes.
    // TODO: return values directly instead of enums?
    // TODO: Custom reader for normal4 and tangent2 that does unorm8x4 * 2 - 1?
    read_attributes(
        base_target.data_offset as u64,
        base_target.vertex_count,
        base_target.vertex_size,
        &[
            DataType::Position2.into(),
            DataType::Normal4.into(),
            DataType::OldPosition.into(),
            DataType::Tangent2.into(),
        ],
        model_bytes,
        Endian::Little,
    )
}

fn read_morph_buffer_target(
    morph_target: &xc3_lib::vertex::MorphTarget,
    model_bytes: &[u8],
) -> BinResult<Vec<MorphTargetVertex>> {
    let mut reader = Cursor::new(model_bytes);

    (0..morph_target.vertex_count as u64)
        .map(|i| {
            // TODO: assume data is tightly packed and seek once?
            reader.seek(SeekFrom::Start(
                morph_target.data_offset as u64 + i * morph_target.vertex_size as u64,
            ))?;

            let vertex: MorphBufferTargetVertex = reader.read_le()?;

            // TODO: Don't remap for consistency?
            // TODO: Read individual attributes?
            Ok(MorphTargetVertex {
                position_delta: vertex.position_delta.into(),
                normal: vertex.normal.map(|u| u as f32 / 255.0 * 2.0 - 1.0).into(),
                tangent: vertex.tangent.map(|u| u as f32 / 255.0 * 2.0 - 1.0).into(),
                vertex_index: vertex.vertex_index,
            })
        })
        .collect()
}

fn read_outline_buffer(
    descriptor: &xc3_lib::vertex::OutlineBufferDescriptor,
    buffer: &[u8],
) -> BinResult<Vec<AttributeData>> {
    if descriptor.vertex_size == 8 {
        // vNormal and vColor in shaders.
        // Buffer 0: vertex buffer
        // Buffer 1: outline buffer
        read_attributes(
            descriptor.data_offset as u64,
            descriptor.vertex_count,
            descriptor.vertex_size,
            &[DataType::Normal.into(), DataType::VertexColor.into()],
            buffer,
            Endian::Little,
        )
    } else {
        // vColor in shaders.
        // Buffer 0: vertex buffer
        // Buffer 1: outline buffer
        // Buffer 2: morph target
        read_attributes(
            descriptor.data_offset as u64,
            descriptor.vertex_count,
            descriptor.vertex_size,
            &[DataType::VertexColor.into()],
            buffer,
            Endian::Little,
        )
    }
}

impl ModelBuffers {
    /// Decode all the attributes from `vertex_data`.
    pub fn from_vertex_data(
        vertex_data: &VertexData,
        skinning: Option<&xc3_lib::mxmd::Skinning>,
    ) -> BinResult<Self> {
        let (vertex_buffers, weights) = read_vertex_buffers(vertex_data, skinning)?;
        let index_buffers = read_index_buffers(vertex_data, Endian::Little)?;

        let outline_buffers = vertex_data
            .outline_buffers
            .iter()
            .map(|descriptor| outline_buffer(descriptor, &vertex_data.buffer))
            .collect::<BinResult<Vec<_>>>()?;

        // TODO: Preserve if this is none or not?
        let unk_buffers = match &vertex_data.unk7 {
            Some(unk) => read_unk_buffers(unk, &vertex_data.buffer)?,
            None => Vec::new(),
        };

        let unk_data = vertex_data
            .unk_data
            .as_ref()
            .map(|u| read_unk_data_buffer(u, &vertex_data.buffer))
            .transpose()?;

        Ok(Self {
            vertex_buffers,
            outline_buffers,
            index_buffers,
            unk_buffers,
            unk_data,
            weights,
        })
    }

    /// Decode all the attributes from `vertex_data`.
    pub fn from_vertex_data_legacy(
        vertex_data: &xc3_lib::mxmd::legacy::VertexData,
        models: &xc3_lib::mxmd::legacy::Models,
    ) -> BinResult<Self> {
        let vertex_buffers = read_vertex_buffers_legacy(vertex_data)?;

        let index_buffers = read_index_buffers_legacy(vertex_data)?;

        // TODO: don't duplicate the weights buffers?
        let weights = weights_legacy(&vertex_buffers, models, vertex_data.weight_buffer_indices);

        Ok(Self {
            vertex_buffers,
            outline_buffers: Vec::new(),
            index_buffers,
            unk_buffers: Vec::new(),
            unk_data: None,
            weights,
        })
    }

    // TODO: Test this in xc3_test?
    /// Encode and write all the attributes to a new [VertexData].
    pub fn to_vertex_data(&self) -> BinResult<VertexData> {
        // TODO: recreate vertex buffers and match original ordering?
        // TODO: vertex, outline, index, align 256, morph, align 256, unk7
        let mut vertex_buffers = Vec::new();
        let mut index_buffers = Vec::new();
        let mut outline_buffers = Vec::new();

        // Match the ordering and alignment from in game.
        let mut writer = Cursor::new(Vec::new());

        for buffer in &self.vertex_buffers {
            let vertex_buffer =
                write_vertex_buffer(&mut writer, &buffer.attributes, Endian::Little)?;
            vertex_buffers.push(vertex_buffer);
        }

        if let Some(weights) = &self.weights {
            let weights_buffer = write_vertex_buffer(
                &mut writer,
                &[
                    AttributeData::SkinWeights(weights.weight_buffers[0].weights.clone()),
                    AttributeData::BoneIndices(weights.weight_buffers[0].bone_indices.clone()),
                ],
                Endian::Little,
            )?;
            vertex_buffers.push(weights_buffer);
        }

        for buffer in &self.outline_buffers {
            let outline_buffer = write_outline_buffer(&mut writer, &buffer.attributes)?;
            outline_buffers.push(outline_buffer);
        }

        for buffer in &self.index_buffers {
            align(&mut writer, 4)?;
            let index_buffer = write_index_buffer(&mut writer, &buffer.indices, Endian::Little)?;
            index_buffers.push(index_buffer);
        }

        align(&mut writer, 256)?;

        let vertex_morphs = if self
            .vertex_buffers
            .iter()
            .any(|b| !b.morph_targets.is_empty())
        {
            Some(self.write_morph_targets(&mut writer)?)
        } else {
            None
        };

        align(&mut writer, 256)?;

        let unk7 = if !self.unk_buffers.is_empty() {
            Some(write_unk_buffers(&mut writer, &self.unk_buffers)?)
        } else {
            None
        };

        align(&mut writer, 256)?;

        // TODO: Offset 2 is just bytes?
        // TODO: unk data offset 2 and then offset 1
        let unk_data = self
            .unk_data
            .as_ref()
            .map(|unk_data| write_unk_data_buffer(&mut writer, unk_data))
            .transpose()?;

        align(&mut writer, 4096)?;

        let mut vertex_buffer_info: Vec<_> = self
            .vertex_buffers
            .iter()
            .map(|buffer| VertexBufferExtInfo {
                flags: VertexBufferExtInfoFlags::new(
                    buffer.outline_buffer_index.is_some(),
                    !buffer.morph_targets.is_empty(),
                    0u8.into(),
                ),
                outline_buffer_index: buffer.outline_buffer_index.unwrap_or_default() as u16,
                morph_target_start_index: 0,
                morph_target_count: 0,
                unk: 0,
            })
            .collect();

        if let Some(morphs) = &vertex_morphs {
            for descriptor in &morphs.descriptors {
                let count = if descriptor.param_indices.is_empty() {
                    0
                } else {
                    // Count the blend and default targets.
                    descriptor.param_indices.len() + 2
                };

                let info = &mut vertex_buffer_info[descriptor.vertex_buffer_index as usize];
                info.morph_target_start_index = descriptor.target_start_index as u16;
                info.morph_target_count = count as u16;
            }
        }

        // TODO: Support converting legacy data?
        let weights = self
            .weights
            .as_ref()
            .and_then(|weights| match &weights.weight_groups {
                WeightGroups::Legacy { .. } => None,
                WeightGroups::Groups {
                    weight_groups,
                    weight_lods,
                } => Some(xc3_lib::vertex::Weights {
                    groups: weight_groups.clone(),
                    vertex_buffer_index: vertex_buffers.len() as u16 - 1,
                    weight_lods: weight_lods.clone(),
                    unk4: 1,
                    unks: [0; 4],
                }),
            });

        Ok(VertexData {
            vertex_buffers,
            index_buffers,
            unk0: 0,
            unk1: 0,
            unk2: 0,
            vertex_buffer_info,
            outline_buffers,
            // TODO: Set remaining data.
            vertex_morphs,
            buffer: writer.into_inner(),
            unk_data,
            weights,
            unk7,
            unks: [0; 5],
        })
    }

    fn write_morph_targets(
        &self,
        writer: &mut Cursor<Vec<u8>>,
    ) -> BinResult<xc3_lib::vertex::VertexMorphs> {
        let mut targets = Vec::new();
        let mut descriptors = Vec::new();

        for (i, buffer) in self
            .vertex_buffers
            .iter()
            .enumerate()
            .filter(|(_, b)| !b.morph_targets.is_empty())
        {
            let descriptor = MorphDescriptor {
                vertex_buffer_index: i as u32,
                target_start_index: targets.len() as u32,
                param_indices: (0..buffer.morph_targets.len() as u16).collect(),
                unk2: 3, // TODO: how to set this?
            };
            descriptors.push(descriptor);

            let target = write_morph_blend_target(writer, &buffer.morph_blend_target)?;
            targets.push(target);
            align(writer, 256)?;

            // The default target stores base values for modified vertices.
            let modified_indices: BTreeSet<_> = buffer
                .morph_targets
                .iter()
                .flat_map(|t| &t.vertex_indices)
                .copied()
                .collect();
            let target = write_morph_default_target(writer, modified_indices, buffer)?;
            targets.push(target);
            align(writer, 256)?;

            for morph_target in &buffer.morph_targets {
                let target = write_morph_param_target(writer, morph_target)?;
                targets.push(target);
                align(writer, 256)?;
            }
        }

        Ok(xc3_lib::vertex::VertexMorphs {
            descriptors,
            targets,
            unks: [0; 4],
        })
    }
}

fn write_unk_data_buffer(
    writer: &mut Cursor<Vec<u8>>,
    unk_data: &UnkDataBuffer,
) -> BinResult<UnkData> {
    let uniform_data_offset = writer.stream_position()? as u32;
    writer.write_all(&unk_data.uniform_data)?;

    let vertex_data_offset = writer.stream_position()? as u32;
    // TODO: Store this using AttributeData?
    if unk_data.attribute2.is_empty() {
        for a1 in &unk_data.attribute1 {
            a1.write_le(writer)?;
        }
    } else {
        for (a1, a2) in unk_data.attribute1.iter().zip(unk_data.attribute2.iter()) {
            a1.write_le(writer)?;
            a2.write_le(writer)?;
        }
    }

    let vertex_size = if unk_data.attribute2.is_empty() {
        12
    } else {
        16
    };
    let vertex_count = unk_data.attribute1.len() as u32;

    Ok(UnkData {
        unk1: if vertex_size == 12 { 1 } else { 3 },
        vertex_data_offset,
        vertex_data_length: vertex_size * vertex_count,
        uniform_data_offset,
        uniform_data_length: unk_data.uniform_data.len() as u32,
        vertex_count,
        vertex_size,
        unk: unk_data.unk,
    })
}

// TODO: share writing code with param target.
fn write_morph_default_target(
    writer: &mut Cursor<Vec<u8>>,
    modified_indices: BTreeSet<u32>,
    buffer: &VertexBuffer,
) -> Result<xc3_lib::vertex::MorphTarget, binrw::Error> {
    let offset = writer.stream_position()?;

    // TODO: None of these attributes are deltas?
    // TODO: Is there a cleaner way of doing this?
    let positions: Vec<_> = buffer
        .morph_blend_target
        .iter()
        .find_map(|a| {
            if let AttributeData::Position2(values) = a {
                Some(
                    modified_indices
                        .iter()
                        .map(|i| values[*i as usize])
                        .collect(),
                )
            } else {
                None
            }
        })
        .unwrap();
    write_data(writer, &positions, offset, 32, Endian::Little, write_f32x3)?;

    write_data(
        writer,
        &vec![0u32; modified_indices.len()],
        offset + 12,
        32,
        Endian::Little,
        write_u32,
    )?;

    let normals: Vec<_> = buffer
        .morph_blend_target
        .iter()
        .find_map(|a| {
            if let AttributeData::Normal4(values) = a {
                Some(
                    modified_indices
                        .iter()
                        .map(|i| values[*i as usize])
                        .collect(),
                )
            } else {
                None
            }
        })
        .unwrap();
    write_data(
        writer,
        &normals,
        offset + 16,
        32,
        Endian::Little,
        write_unorm8x4,
    )?;

    let tangents: Vec<_> = buffer
        .morph_blend_target
        .iter()
        .find_map(|a| {
            if let AttributeData::Tangent2(values) = a {
                Some(
                    modified_indices
                        .iter()
                        .map(|i| values[*i as usize])
                        .collect(),
                )
            } else {
                None
            }
        })
        .unwrap();
    write_data(
        writer,
        &tangents,
        offset + 20,
        32,
        Endian::Little,
        write_unorm8x4,
    )?;

    write_data(
        writer,
        &vec![0u32; modified_indices.len()],
        offset + 24,
        32,
        Endian::Little,
        write_u32,
    )?;

    let indices: Vec<_> = modified_indices.iter().copied().collect();
    write_data(writer, &indices, offset + 28, 32, Endian::Little, write_u32)?;

    Ok(xc3_lib::vertex::MorphTarget {
        data_offset: offset as u32,
        vertex_count: modified_indices.len() as u32,
        vertex_size: 32,
        flags: MorphTargetFlags::new(0, false, true, false, 0u8.into()),
    })
}

fn write_morph_param_target(
    writer: &mut Cursor<Vec<u8>>,
    morph_target: &MorphTarget,
) -> Result<xc3_lib::vertex::MorphTarget, binrw::Error> {
    let offset = writer.stream_position()?;

    // TODO: validate that lengths are equal?
    for (((position, normal), tangent), index) in morph_target
        .position_deltas
        .iter()
        .zip(morph_target.normals.iter())
        .zip(morph_target.tangents.iter())
        .zip(morph_target.vertex_indices.iter())
    {
        write_f32x3(writer, position, Endian::Little)?;
        write_u32(writer, &0, Endian::Little)?;
        write_unorm8x4(writer, &(*normal * 0.5 + 0.5), Endian::Little)?;
        write_unorm8x4(writer, &(*tangent * 0.5 + 0.5), Endian::Little)?;
        write_u32(writer, &0, Endian::Little)?;
        write_u32(writer, index, Endian::Little)?;
    }

    Ok(xc3_lib::vertex::MorphTarget {
        data_offset: offset as u32,
        vertex_count: morph_target.position_deltas.len() as u32,
        vertex_size: 32,
        flags: MorphTargetFlags::new(0, false, false, true, 0u8.into()),
    })
}

fn write_morph_blend_target(
    writer: &mut Cursor<Vec<u8>>,
    blend_target: &[AttributeData],
) -> Result<xc3_lib::vertex::MorphTarget, binrw::Error> {
    let descriptor = write_vertex_buffer(writer, blend_target, Endian::Little)?;
    Ok(xc3_lib::vertex::MorphTarget {
        data_offset: descriptor.data_offset,
        vertex_count: descriptor.vertex_count,
        vertex_size: descriptor.vertex_size, // TODO: assert that this is 32?
        flags: MorphTargetFlags::new(0, true, false, false, 0u8.into()),
    })
}

fn read_index_buffers_legacy(
    vertex_data: &xc3_lib::mxmd::legacy::VertexData,
) -> BinResult<Vec<IndexBuffer>> {
    // Each buffer already has the data at the appropriate offset.
    let data_offset = 0;

    vertex_data
        .index_buffers
        .iter()
        .map(|descriptor| {
            Ok(IndexBuffer {
                indices: read_indices(
                    &IndexBufferDescriptor {
                        data_offset,
                        index_count: descriptor.index_count,
                        primitive_type: xc3_lib::vertex::PrimitiveType::TriangleList,
                        index_format: xc3_lib::vertex::IndexFormat::Uint16,
                        unk3: 0,
                        unk4: 0,
                    },
                    &descriptor.data,
                    Endian::Big,
                )?,
            })
        })
        .collect()
}

fn read_vertex_buffers_legacy(
    vertex_data: &xc3_lib::mxmd::legacy::VertexData,
) -> BinResult<Vec<VertexBuffer>> {
    // Each buffer already has the data at the appropriate offset.
    let data_offset = 0;

    vertex_data
        .vertex_buffers
        .iter()
        .map(|descriptor| {
            Ok(VertexBuffer {
                attributes: read_attributes(
                    data_offset,
                    descriptor.vertex_count,
                    descriptor.vertex_size,
                    &descriptor.attributes,
                    &descriptor.data,
                    Endian::Big,
                )?,
                morph_blend_target: Vec::new(),
                morph_targets: Vec::new(),
                outline_buffer_index: None,
            })
        })
        .collect()
}

fn weights_legacy(
    vertex_buffers: &[VertexBuffer],
    models: &xc3_lib::mxmd::legacy::Models,
    weight_buffer_indices: [u16; 6],
) -> Option<Weights> {
    // TODO: Find a better way of organizing these types?
    // TODO: Don't store this with the vertex data?
    // TODO: Is this correct?
    // TODO: Does this also depend on the skinning indices?
    let bone_names: Vec<_> = models.bone_names.iter().map(|n| n.name.clone()).collect();

    // Xenoblade X uses multiple weight buffers.
    let weight_buffers = vertex_buffers
        .iter()
        .filter_map(|b| {
            let (weights, bone_indices) = skin_weights_bone_indices(&b.attributes)?;
            Some(SkinWeights {
                bone_indices,
                weights,
                bone_names: bone_names.clone(),
            })
        })
        .collect();

    // Reindex to account for flattening the buffers.
    // TODO: Store the original index with each weight buffer to handle unused indices?
    let weight_buffer_start = vertex_buffers
        .iter()
        .position(|b| skin_weights_bone_indices(&b.attributes).is_some())
        .unwrap_or_default();

    Some(Weights {
        weight_buffers,
        weight_groups: WeightGroups::Legacy {
            weight_buffer_indices: weight_buffer_indices
                .map(|i| (i as usize).saturating_sub(weight_buffer_start)),
        },
    })
}

fn write_unk_buffers(
    writer: &mut Cursor<Vec<u8>>,
    unk_buffers: &[UnkBuffer],
) -> Result<Unk, binrw::Error> {
    let data_offset = writer.stream_position()? as u32;

    let mut buffers = Vec::new();
    let mut start_index = 0;

    for buffer in unk_buffers {
        let unk_buffer = write_unk_buffer(writer, buffer, data_offset, start_index)?;
        start_index += unk_buffer.count;
        buffers.push(unk_buffer);
    }

    let data_length = writer.stream_position()? as u32 - data_offset;

    Ok(Unk {
        buffers,
        data_length,
        data_offset,
        unks: [0; 8],
    })
}

fn write_unk_buffer<W: Write + Seek>(
    writer: &mut W,
    buffer: &UnkBuffer,
    data_offset: u32,
    start_index: u32,
) -> BinResult<UnkBufferDescriptor> {
    let descriptor = write_vertex_buffer(writer, &buffer.attributes, Endian::Little)?;

    // Offsets are relative to the start of the section.
    Ok(UnkBufferDescriptor {
        unk1: if descriptor.vertex_size == 16 { 0 } else { 1 },
        unk2: buffer.unk2,
        count: descriptor.vertex_count,
        offset: descriptor.data_offset - data_offset,
        unk5: 0,
        start_index,
    })
}

fn read_unk_buffers(unk: &xc3_lib::vertex::Unk, buffer: &[u8]) -> BinResult<Vec<UnkBuffer>> {
    unk.buffers
        .iter()
        .map(|descriptor| read_unk_buffer(descriptor, unk.data_offset as u64, buffer))
        .collect()
}

fn read_unk_buffer(
    descriptor: &UnkBufferDescriptor,
    data_offset: u64,
    buffer: &[u8],
) -> Result<UnkBuffer, binrw::Error> {
    // TODO: why is this 16 or 24 bytes?
    Ok(UnkBuffer {
        unk2: descriptor.unk2,
        attributes: if descriptor.unk1 == 0 {
            read_attributes(
                data_offset + descriptor.offset as u64,
                descriptor.count,
                16,
                &[DataType::Position.into(), DataType::VertexColor.into()],
                buffer,
                Endian::Little,
            )?
        } else {
            read_attributes(
                data_offset + descriptor.offset as u64,
                descriptor.count,
                24,
                &[
                    DataType::Position.into(),
                    DataType::VertexColor.into(),
                    DataType::VertexColor.into(),
                    DataType::VertexColor.into(),
                ],
                buffer,
                Endian::Little,
            )?
        },
    })
}

// TODO: Test case for 12 and 16 bytes.
fn read_unk_data_buffer(unk: &UnkData, buffer: &[u8]) -> BinResult<UnkDataBuffer> {
    let mut reader = Cursor::new(buffer);
    reader.set_position(unk.vertex_data_offset as u64);

    // TODO: Just store these as is for now to support rebuilding.
    let mut attribute1 = Vec::new();
    let mut attribute2 = Vec::new();
    for _ in 0..unk.vertex_count {
        let xyz: [u32; 3] = reader.read_le()?;
        attribute1.push(xyz);
        if unk.vertex_size == 16 {
            let unk: u32 = reader.read_le()?;
            attribute2.push(unk);
        }
    }

    let mut uniform_data = vec![0u8; unk.uniform_data_length as usize];
    reader.set_position(unk.uniform_data_offset as u64);
    reader.read_exact(&mut uniform_data)?;

    Ok(UnkDataBuffer {
        attribute1,
        attribute2,
        uniform_data,
        unk: unk.unk,
    })
}

fn align(buffer_writer: &mut Cursor<Vec<u8>>, align: u64) -> Result<(), binrw::Error> {
    let aligned_size = buffer_writer.position().next_multiple_of(align);
    let padding = aligned_size - buffer_writer.position();
    buffer_writer.write_all(&vec![0u8; padding as usize])?;
    Ok(())
}

// TODO: support u32?
fn write_index_buffer<W: Write + Seek>(
    writer: &mut W,
    indices: &[u16],
    endian: Endian,
) -> BinResult<IndexBufferDescriptor> {
    let data_offset = writer.stream_position()? as u32;

    indices.write_options(writer, endian, ())?;

    Ok(IndexBufferDescriptor {
        data_offset,
        index_count: indices.len() as u32,
        primitive_type: xc3_lib::vertex::PrimitiveType::TriangleList,
        index_format: xc3_lib::vertex::IndexFormat::Uint16,
        unk3: 0,
        unk4: 0,
    })
}

fn write_vertex_buffer<W: Write + Seek>(
    writer: &mut W,
    attribute_data: &[AttributeData],
    endian: Endian,
) -> BinResult<VertexBufferDescriptor> {
    let data_offset = writer.stream_position()? as u32;

    let attributes: Vec<xc3_lib::vertex::VertexAttribute> = attribute_data
        .iter()
        .map(|a| a.data_type().into())
        .collect();

    let vertex_size = attributes.iter().map(|a| a.data_size as u32).sum();

    // TODO: Check if all the arrays have the same length.
    let vertex_count = attribute_data[0].len() as u32;

    // TODO: Include a base offset?
    let mut offset = writer.stream_position()?;
    for (a, data) in attributes.iter().zip(attribute_data) {
        data.write(writer, offset, vertex_size as u64, endian)?;
        offset += a.data_size as u64;
    }

    Ok(VertexBufferDescriptor {
        data_offset,
        vertex_count,
        vertex_size,
        attributes,
        unk1: 0,
        unk2: 0,
        unk3: 0,
    })
}

fn write_outline_buffer<W: Write + Seek>(
    writer: &mut W,
    attribute_data: &[AttributeData],
) -> BinResult<OutlineBufferDescriptor> {
    let buffer = write_vertex_buffer(writer, attribute_data, Endian::Little)?;

    Ok(OutlineBufferDescriptor {
        data_offset: buffer.data_offset,
        vertex_count: buffer.vertex_count,
        vertex_size: buffer.vertex_size,
        unk: 0,
    })
}

fn write_data<T, F, W>(
    writer: &mut W,
    values: &[T],
    offset: u64,
    stride: u64,
    endian: Endian,
    write_item: F,
) -> BinResult<()>
where
    W: Write + Seek,
    F: Fn(&mut W, &T, Endian) -> BinResult<()>,
{
    AttributeWriteArgs {
        offset,
        stride,
        endian,
    }
    .write(writer, values, write_item)
}

fn write_u16x2<W: Write + Seek>(writer: &mut W, value: &[u16; 2], endian: Endian) -> BinResult<()> {
    value.write_options(writer, endian, ())
}

fn write_u32<W: Write + Seek>(writer: &mut W, value: &u32, endian: Endian) -> BinResult<()> {
    value.write_options(writer, endian, ())
}

fn write_u8x4<W: Write + Seek>(writer: &mut W, value: &[u8; 4], endian: Endian) -> BinResult<()> {
    value.write_options(writer, endian, ())
}

fn write_f32x2<W: Write + Seek>(writer: &mut W, value: &Vec2, endian: Endian) -> BinResult<()> {
    value.to_array().write_options(writer, endian, ())
}

fn write_f32x3<W: Write + Seek>(writer: &mut W, value: &Vec3, endian: Endian) -> BinResult<()> {
    value.to_array().write_options(writer, endian, ())
}

fn write_unorm8x4<W: Write + Seek>(writer: &mut W, value: &Vec4, endian: Endian) -> BinResult<()> {
    value
        .to_array()
        .map(|f| (f * 255.0) as u8)
        .write_options(writer, endian, ())
}

fn write_unorm16x4<W: Write + Seek>(writer: &mut W, value: &Vec4, endian: Endian) -> BinResult<()> {
    value
        .to_array()
        .map(|f| (f * 65535.0) as u16)
        .write_options(writer, endian, ())
}

fn write_snorm8x4<W: Write + Seek>(writer: &mut W, value: &Vec4, endian: Endian) -> BinResult<()> {
    value
        .to_array()
        .map(|f| (f * 127.0) as i8)
        .write_options(writer, endian, ())
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::assert_hex_eq;

    use glam::{vec2, vec3, vec4};
    use hexlit::hex;
    use xc3_lib::vertex::VertexAttribute;

    #[test]
    fn vertex_buffer_indices() {
        // xeno3/chr/ch/ch01012013.wismt, index buffer 0
        let data = hex!(00000100 02000100);

        let descriptor = IndexBufferDescriptor {
            data_offset: 0,
            index_count: 4,
            primitive_type: xc3_lib::vertex::PrimitiveType::TriangleList,
            index_format: xc3_lib::vertex::IndexFormat::Uint16,
            unk3: 0,
            unk4: 0,
        };

        // Test read.
        let indices = read_indices(&descriptor, &data, Endian::Little).unwrap();
        assert_eq!(vec![0, 1, 2, 1], indices);

        // Test write.
        let mut writer = Cursor::new(Vec::new());
        let new_descriptor = write_index_buffer(&mut writer, &indices, Endian::Little).unwrap();
        assert_eq!(new_descriptor, descriptor);
        assert_hex_eq!(data, writer.into_inner());
    }

    #[test]
    fn vertex_buffer_vertices() {
        // xeno3/chr/ch/ch01012013.wismt, vertex buffer 0
        let data = hex!(
            // vertex 0
            0x459ecd3d 8660673f f2ad923d
            13010000
            fd8d423f aea11b3f
            7f00ffff
            21fb7a00
            7a00df7f
            // vertex 1
            0x8879143e 81d46a3f 54db4e3d
            14010000
            72904a3f 799d193f
            7f00ffff
            620c4f00
            4f009e7f
        );

        let descriptor = VertexBufferDescriptor {
            data_offset: 0,
            vertex_count: 2,
            vertex_size: 36,
            attributes: vec![
                VertexAttribute {
                    data_type: DataType::Position,
                    data_size: 12,
                },
                VertexAttribute {
                    data_type: DataType::WeightIndex,
                    data_size: 4,
                },
                VertexAttribute {
                    data_type: DataType::TexCoord0,
                    data_size: 8,
                },
                VertexAttribute {
                    data_type: DataType::VertexColor,
                    data_size: 4,
                },
                VertexAttribute {
                    data_type: DataType::Normal,
                    data_size: 4,
                },
                VertexAttribute {
                    data_type: DataType::Tangent,
                    data_size: 4,
                },
            ],
            unk1: 0,
            unk2: 0,
            unk3: 0,
        };

        // Test read.
        let attributes = vec![
            AttributeData::Position(vec![
                vec3(0.10039953, 0.9038166, 0.07162084),
                vec3(0.14499485, 0.91730505, 0.050502136),
            ]),
            AttributeData::WeightIndex(vec![[275, 0], [276, 0]]),
            AttributeData::TexCoord0(vec![
                vec2(0.75997907, 0.6079358),
                vec2(0.79126656, 0.6000591),
            ]),
            AttributeData::VertexColor(vec![
                vec4(0.49803922, 0.0, 1.0, 1.0),
                vec4(0.49803922, 0.0, 1.0, 1.0),
            ]),
            AttributeData::Normal(vec![
                vec4(0.25984251, -0.03937008, 0.96062994, 0.0),
                vec4(0.77165353, 0.09448819, 0.62204725, 0.0),
            ]),
            AttributeData::Tangent(vec![
                vec4(0.96062994, 0.0, -0.25984251, 1.0),
                vec4(0.62204725, 0.0, -0.77165353, 1.0),
            ]),
        ];
        assert_eq!(
            attributes,
            read_attributes(
                descriptor.data_offset as u64,
                descriptor.vertex_count,
                descriptor.vertex_size,
                &descriptor.attributes,
                &data,
                Endian::Little
            )
            .unwrap()
        );

        // Test write.
        let mut writer = Cursor::new(Vec::new());
        let new_descriptor = write_vertex_buffer(&mut writer, &attributes, Endian::Little).unwrap();
        assert_eq!(new_descriptor, descriptor);
        assert_hex_eq!(data, writer.into_inner());
    }

    #[test]
    fn vertex_buffer_vertices_val_inf() {
        // xeno3/chr/en/en01000230.wismt, vertex buffer 2
        let data = hex!(
            // vertex 0
            0x95085ebd 7d3c9a3f 7ac87c3f
            ee080000
            60269e3e 3038873d
            bb9be181
            38bd5b7f
            38bd5b7f
            // vertex 1
            eb8274bd 098c9d3f e90e7d3f
            ee080000
            30259e3e 6017983c
            a83dbd81
            0f66497f
            0f66497f
        );

        let descriptor = VertexBufferDescriptor {
            data_offset: 0,
            vertex_count: 2,
            vertex_size: 36,
            attributes: vec![
                VertexAttribute {
                    data_type: DataType::Position,
                    data_size: 12,
                },
                VertexAttribute {
                    data_type: DataType::WeightIndex,
                    data_size: 4,
                },
                VertexAttribute {
                    data_type: DataType::TexCoord0,
                    data_size: 8,
                },
                VertexAttribute {
                    data_type: DataType::Tangent,
                    data_size: 4,
                },
                VertexAttribute {
                    data_type: DataType::Normal2,
                    data_size: 4,
                },
                VertexAttribute {
                    data_type: DataType::ValInf,
                    data_size: 4,
                },
            ],
            unk1: 0,
            unk2: 0,
            unk3: 0,
        };

        // Test read.
        let attributes = vec![
            AttributeData::Position(vec![
                vec3(-0.054207403, 1.204971, 0.987434),
                vec3(-0.059695166, 1.230836, 0.98850876),
            ]),
            AttributeData::WeightIndex(vec![[2286, 0], [2286, 0]]),
            AttributeData::TexCoord0(vec![
                vec2(0.30888653, 0.06602514),
                vec2(0.30887747, 0.018565834),
            ]),
            AttributeData::Tangent(vec![
                vec4(-0.54330707, -0.79527557, -0.24409449, -1.0),
                vec4(-0.6929134, 0.48031497, -0.52755904, -1.0),
            ]),
            AttributeData::Normal2(vec![
                vec4(0.44094488, -0.52755904, 0.71653545, 1.0),
                vec4(0.11811024, 0.8031496, 0.5748032, 1.0),
            ]),
            AttributeData::ValInf(vec![
                vec4(0.44094488, -0.52755904, 0.71653545, 1.0),
                vec4(0.11811024, 0.8031496, 0.5748032, 1.0),
            ]),
        ];
        assert_eq!(
            attributes,
            read_attributes(
                descriptor.data_offset as u64,
                descriptor.vertex_count,
                descriptor.vertex_size,
                &descriptor.attributes,
                &data,
                Endian::Little
            )
            .unwrap()
        );

        // Test write.
        let mut writer = Cursor::new(Vec::new());
        let new_descriptor = write_vertex_buffer(&mut writer, &attributes, Endian::Little).unwrap();
        assert_eq!(new_descriptor, descriptor);
        assert_hex_eq!(data, writer.into_inner());
    }

    #[test]
    fn weight_buffer_vertices() {
        // xeno3/chr/ch/ch01012013.wismt, vertex buffer 12
        let data = hex!(
            // vertex 0
            aec75138 00000000 18170000
            // vertex 1
            0x1ec5e13a 00000000 18170000
        );

        let descriptor = VertexBufferDescriptor {
            data_offset: 0,
            vertex_count: 2,
            vertex_size: 12,
            attributes: vec![
                VertexAttribute {
                    data_type: DataType::SkinWeights,
                    data_size: 8,
                },
                VertexAttribute {
                    data_type: DataType::BoneIndices,
                    data_size: 4,
                },
            ],
            unk1: 0,
            unk2: 0,
            unk3: 0,
        };

        // Test read.
        let attributes = vec![
            AttributeData::SkinWeights(vec![
                vec4(0.7800107, 0.21998931, 0.0, 0.0),
                vec4(0.77000076, 0.22999924, 0.0, 0.0),
            ]),
            AttributeData::BoneIndices(vec![[24, 23, 0, 0], [24, 23, 0, 0]]),
        ];
        assert_eq!(
            attributes,
            read_attributes(
                descriptor.data_offset as u64,
                descriptor.vertex_count,
                descriptor.vertex_size,
                &descriptor.attributes,
                &data,
                Endian::Little
            )
            .unwrap()
        );

        // Test write.
        let mut writer = Cursor::new(Vec::new());
        let new_descriptor = write_vertex_buffer(&mut writer, &attributes, Endian::Little).unwrap();
        assert_eq!(new_descriptor, descriptor);
        assert_hex_eq!(data, writer.into_inner());
    }

    #[test]
    fn map_vertex_buffer_vertices() {
        // xeno1/map/ma0301.wismhd, map vertex data 4, vertex buffer 13
        let data = hex!(
            // vertex 0
            3c873845 d0a15c43 988cbcc3
            dc92fd3f c6913dc2
            588b0e40 9a103ec2
            dc92fd3f c6913dc2
            8e691940 d8cd16c0
            b4401a40 113a17c0
            8e691940 d8cd16c0
            bca0333e d801133f
            493e223f dec2e33e
            0e5cd2be e062dd3d
            7f007f00
            ffffffff
            f1782300
            7d10017f
            // vertex 1
            42823845 fe6b5c43 c159bcc3
            42a1f83f 955b3dc2
            0x1ecd0b40 3de23dc2
            8898f83f ef5e3dc2
            ce471940 9a9f16c0
            401b1a40 811217c0
            92471940 77a216c0
            c0674f3e 8a09163f
            1c78233f f2c31b3f
            fbaedabe 20fa093e
            0000ff00
            ffffffff
            e8752a00
            7c1a007f
        );

        let descriptor = VertexBufferDescriptor {
            data_offset: 0,
            vertex_count: 2,
            vertex_size: 100,
            attributes: vec![
                VertexAttribute {
                    data_type: DataType::Position,
                    data_size: 12,
                },
                VertexAttribute {
                    data_type: DataType::TexCoord0,
                    data_size: 8,
                },
                VertexAttribute {
                    data_type: DataType::TexCoord1,
                    data_size: 8,
                },
                VertexAttribute {
                    data_type: DataType::TexCoord2,
                    data_size: 8,
                },
                VertexAttribute {
                    data_type: DataType::TexCoord3,
                    data_size: 8,
                },
                VertexAttribute {
                    data_type: DataType::TexCoord4,
                    data_size: 8,
                },
                VertexAttribute {
                    data_type: DataType::TexCoord5,
                    data_size: 8,
                },
                VertexAttribute {
                    data_type: DataType::TexCoord6,
                    data_size: 8,
                },
                VertexAttribute {
                    data_type: DataType::TexCoord7,
                    data_size: 8,
                },
                VertexAttribute {
                    data_type: DataType::TexCoord8,
                    data_size: 8,
                },
                VertexAttribute {
                    data_type: DataType::Blend,
                    data_size: 4,
                },
                VertexAttribute {
                    data_type: DataType::VertexColor,
                    data_size: 4,
                },
                VertexAttribute {
                    data_type: DataType::Normal,
                    data_size: 4,
                },
                VertexAttribute {
                    data_type: DataType::Tangent,
                    data_size: 4,
                },
            ],
            unk1: 0,
            unk2: 0,
            unk3: 0,
        };

        // Test read.
        let attributes = vec![
            AttributeData::Position(vec![
                vec3(2952.4521, 220.63208, -377.0984),
                vec3(2952.141, 220.42184, -376.7012),
            ]),
            AttributeData::TexCoord0(vec![
                vec2(1.9810443, -47.392357),
                vec2(1.9424212, -47.339436),
            ]),
            AttributeData::TexCoord1(vec![
                vec2(2.2272549, -47.516212),
                vec2(2.1843944, -47.470936),
            ]),
            AttributeData::TexCoord2(vec![
                vec2(1.9810443, -47.392357),
                vec2(1.9421549, -47.34271),
            ]),
            AttributeData::TexCoord3(vec![
                vec2(2.3970675, -2.3563137),
                vec2(2.3950076, -2.3534913),
            ]),
            AttributeData::TexCoord4(vec![
                vec2(2.4101992, -2.362919),
                vec2(2.4079132, -2.3605044),
            ]),
            AttributeData::TexCoord5(vec![
                vec2(2.3970675, -2.3563137),
                vec2(2.3949933, -2.353666),
            ]),
            AttributeData::TexCoord6(vec![
                vec2(0.17541784, 0.5742469),
                vec2(0.20254421, 0.58608305),
            ]),
            AttributeData::TexCoord7(vec![
                vec2(0.6337629, 0.4448461),
                vec2(0.6385515, 0.60845864),
            ]),
            AttributeData::TexCoord8(vec![
                vec2(-0.41085857, 0.108098745),
                vec2(-0.42711625, 0.13474321),
            ]),
            AttributeData::Blend(vec![
                vec4(0.49803922, 0.0, 0.49803922, 0.0),
                vec4(0.0, 0.0, 1.0, 0.0),
            ]),
            AttributeData::VertexColor(vec![vec4(1.0, 1.0, 1.0, 1.0), vec4(1.0, 1.0, 1.0, 1.0)]),
            AttributeData::Normal(vec![
                vec4(-0.11811024, 0.9448819, 0.27559054, 0.0),
                vec4(-0.18897638, 0.9212598, 0.33070865, 0.0),
            ]),
            AttributeData::Tangent(vec![
                vec4(0.984252, 0.12598425, 0.007874016, 1.0),
                vec4(0.97637796, 0.20472442, 0.0, 1.0),
            ]),
        ];
        assert_eq!(
            attributes,
            read_attributes(
                descriptor.data_offset as u64,
                descriptor.vertex_count,
                descriptor.vertex_size,
                &descriptor.attributes,
                &data,
                Endian::Little
            )
            .unwrap()
        );

        // Test write.
        let mut writer = Cursor::new(Vec::new());
        let new_descriptor = write_vertex_buffer(&mut writer, &attributes, Endian::Little).unwrap();
        assert_eq!(new_descriptor, descriptor);
        assert_hex_eq!(data, writer.into_inner());
    }

    #[test]
    fn morph_blend_target_vertices() {
        // xeno3/chr/ch/ch01027000.wismt, "face_D2_shape", target 324.
        let data = hex!(
            // vertex 0
            2828333d 9bdcae3f e9c508bd
            e7415a01
            2828333d 9bdcae3f e9c508bd
            7dbe11ff
            // vertex 1
            52c6463d 8cddaf3f 56bf0abd
            ed4c5901
            52c6463d 8cddaf3f 56bf0abd
            7bc516ff
        );

        let target = xc3_lib::vertex::MorphTarget {
            data_offset: 0,
            vertex_count: 2,
            vertex_size: 32,
            flags: xc3_lib::vertex::MorphTargetFlags::new(0u16, true, false, false, 0u8.into()),
        };

        // Test read.
        let attributes = vec![
            AttributeData::Position2(vec![
                vec3(0.043739468, 1.3661073, -0.033391867),
                vec3(0.048528977, 1.3739486, -0.03387388),
            ]),
            AttributeData::Normal4(vec![
                vec4(0.90588236, 0.25490198, 0.3529412, 0.003921569),
                vec4(0.92941177, 0.29803923, 0.34901962, 0.003921569),
            ]),
            AttributeData::OldPosition(vec![
                vec3(0.043739468, 1.3661073, -0.033391867),
                vec3(0.048528977, 1.3739486, -0.03387388),
            ]),
            AttributeData::Tangent2(vec![
                vec4(0.49019608, 0.74509805, 0.06666667, 1.0),
                vec4(0.48235294, 0.77254903, 0.08627451, 1.0),
            ]),
        ];
        assert_eq!(attributes, read_morph_blend_target(&target, &data).unwrap());

        // Test write.
        let mut writer = Cursor::new(Vec::new());
        let new_target = write_morph_blend_target(&mut writer, &attributes).unwrap();
        assert_eq!(new_target, target);
        assert_hex_eq!(data, writer.into_inner());
    }

    #[test]
    fn read_morph_default_buffer_vertices() {
        // xeno3/chr/ch/ch01027000.wismt, "face_D2_shape", target index 325.
        let data = hex!(
            // vertex 0
            8c54023d bc27ac3f 72dd93bc
            00000000
            d6237601
            a0a90cff
            00000000
            04000000
            // vertex 1
            2b28153d 27e7ac3f 06d8b2bc
            00000000
            dd2c6b01
            0x8ead0aff
            00000000
            06000000
        );

        let target = xc3_lib::vertex::MorphTarget {
            data_offset: 0,
            vertex_count: 2,
            vertex_size: 32,
            flags: xc3_lib::vertex::MorphTargetFlags::new(0u16, false, true, false, 0u8.into()),
        };

        // TODO: These aren't actually deltas?
        assert_eq!(
            vec![
                MorphTargetVertex {
                    position_delta: vec3(0.03181891, 1.3449626, -0.01804993),
                    normal: vec4(0.6784314, -0.7254902, -0.0745098, -0.99215686),
                    tangent: vec4(0.254902, 0.32549024, -0.90588236, 1.0),
                    vertex_index: 4
                },
                MorphTargetVertex {
                    position_delta: vec3(0.03641526, 1.3508042, -0.021831524),
                    normal: vec4(0.73333335, -0.654902, -0.1607843, -0.99215686),
                    tangent: vec4(0.11372554, 0.35686278, -0.92156863, 1.0),
                    vertex_index: 6
                }
            ],
            read_morph_buffer_target(&target, &data).unwrap()
        );
        // TODO: Test write.
    }

    #[test]
    fn read_morph_param_buffer_vertices() {
        // xeno3/chr/ch/ch01027000.wismt, "face_D2_shape", target index 326.
        let data = hex!(
            // vertex 0
            f0462abb 00f0a4bb 80b31a39
            00000000
            f770a800
            6ad3ddff
            00000000
            d8000000
            // vertex 1
            c03fd9ba 005245bb 002027b7
            00000000
            f66fa900
            90fd83ff
            00000000
            d9000000
        );

        let target = xc3_lib::vertex::MorphTarget {
            data_offset: 0,
            vertex_count: 2,
            vertex_size: 32,
            flags: xc3_lib::vertex::MorphTargetFlags::new(0u16, false, false, true, 0u8.into()),
        };

        let actual_target = read_morph_target(&target, &data, 0).unwrap();
        assert_eq!(
            MorphTarget {
                morph_controller_index: 0,
                position_deltas: vec![
                    vec3(-0.0025982223, -0.005033493, 0.00014753453),
                    vec3(-0.0016574785, -0.003010869, -9.961426e-6)
                ],
                normals: vec![
                    vec4(0.9372549, -0.12156862, 0.3176471, -1.0),
                    vec4(0.92941177, -0.12941176, 0.32549024, -1.0)
                ],
                tangents: vec![
                    vec4(-0.16862744, 0.654902, 0.73333335, 1.0),
                    vec4(0.12941182, 0.9843137, 0.027451038, 1.0)
                ],
                vertex_indices: vec![216, 217]
            },
            actual_target
        );

        // Test write.
        let mut writer = Cursor::new(Vec::new());
        let new_target = write_morph_param_target(&mut writer, &actual_target).unwrap();
        assert_eq!(new_target, target);
        assert_hex_eq!(data, writer.into_inner());
    }

    #[test]
    fn unk_buffer_vertices_size24() {
        // xeno3/chr/ch/ch01011011.wismt, unk buffer starting from offset 1148672.
        let data = hex!(
            // vertex 0
            7db21bbd 32f3ce3f 9d9ddbbd
            ff000000
            02000000
            c6e69300
            // vertex 1
            2c1bdbbc 3dd3ce3f a664e2bd
            ff000000
            02000000
            e1ed8700
        );

        let descriptor = xc3_lib::vertex::UnkBufferDescriptor {
            unk1: 1,
            unk2: 1,
            count: 2,
            offset: 0,
            unk5: 0,
            start_index: 0,
        };

        // Test read.
        let buffer = read_unk_buffer(&descriptor, 0, &data).unwrap();
        assert_eq!(
            UnkBuffer {
                unk2: 1,
                attributes: vec![
                    AttributeData::Position(vec![
                        vec3(-0.038012017, 1.6167967, -0.10723422),
                        vec3(-0.026746355, 1.6158215, -0.110543534)
                    ]),
                    AttributeData::VertexColor(vec![
                        vec4(1.0, 0.0, 0.0, 0.0),
                        vec4(1.0, 0.0, 0.0, 0.0)
                    ]),
                    AttributeData::VertexColor(vec![
                        vec4(0.007843138, 0.0, 0.0, 0.0),
                        vec4(0.007843138, 0.0, 0.0, 0.0)
                    ]),
                    AttributeData::VertexColor(vec![
                        vec4(0.7764706, 0.9019608, 0.5764706, 0.0),
                        vec4(0.88235295, 0.92941177, 0.5294118, 0.0)
                    ])
                ]
            },
            buffer
        );

        // Test write.
        let mut writer = Cursor::new(Vec::new());
        let new_descriptor = write_unk_buffer(&mut writer, &buffer, 0, 0).unwrap();
        assert_eq!(new_descriptor, descriptor);
        assert_hex_eq!(data, writer.into_inner());
    }

    #[test]
    fn unk_buffer_vertices_size16() {
        // xeno3/chr/ch/ch06002301.wismt, unk buffer starting from offset 18944.
        let data = hex!(
            // vertex 0
            80d31dbd 4565813c 573535be
            b2fe9d00
            // vertex 1
            94d1dbbc 5c83693c de9e37be
            fa820000
        );

        let descriptor = xc3_lib::vertex::UnkBufferDescriptor {
            unk1: 0,
            unk2: 0,
            count: 2,
            offset: 0,
            unk5: 0,
            start_index: 0,
        };

        // Test read.
        let buffer = read_unk_buffer(&descriptor, 0, &data).unwrap();
        assert_eq!(
            UnkBuffer {
                unk2: 0,
                attributes: vec![
                    AttributeData::Position(vec![
                        vec3(-0.03853178, 0.01579536, -0.17696129),
                        vec3(-0.026833333, 0.01425251, -0.17931697)
                    ]),
                    AttributeData::VertexColor(vec![
                        vec4(0.69803923, 0.99607843, 0.6156863, 0.0),
                        vec4(0.98039216, 0.50980395, 0.0, 0.0)
                    ])
                ]
            },
            buffer
        );

        // Test write.
        let mut writer = Cursor::new(Vec::new());
        let new_descriptor = write_unk_buffer(&mut writer, &buffer, 0, 0).unwrap();
        assert_eq!(new_descriptor, descriptor);
        assert_hex_eq!(data, writer.into_inner());
    }

    #[test]
    fn read_outline_buffer_vertices_size4() {
        // xeno3/chr/ch/ch01011011.wismt, outline buffer 0.
        let data = hex!(
            // vertex 0
            5d2f1f00
            // vertex 1
            5d2f1f0c
        );

        let descriptor = xc3_lib::vertex::OutlineBufferDescriptor {
            data_offset: 0,
            vertex_count: 2,
            vertex_size: 4,
            unk: 0,
        };

        assert_eq!(
            vec![AttributeData::VertexColor(vec![
                vec4(0.3647059, 0.18431373, 0.12156863, 0.0),
                vec4(0.3647059, 0.18431373, 0.12156863, 0.047058824)
            ])],
            read_outline_buffer(&descriptor, &data).unwrap()
        );
    }

    #[test]
    fn read_outline_buffer_vertices_size8() {
        // xeno3/chr/ch/ch01011011.wismt, outline buffer 3.
        let data = hex!(
            // vertex 0
            7adffc00
            4b37294c
            // vertex 1
            7adffc00
            4b37294c
        );

        let descriptor = xc3_lib::vertex::OutlineBufferDescriptor {
            data_offset: 0,
            vertex_count: 2,
            vertex_size: 8,
            unk: 0,
        };

        assert_eq!(
            vec![
                AttributeData::Normal(vec![
                    vec4(0.96062994, -0.25984251, -0.031496063, 0.0),
                    vec4(0.96062994, -0.25984251, -0.031496063, 0.0)
                ]),
                AttributeData::VertexColor(vec![
                    vec4(0.29411766, 0.21568628, 0.16078432, 0.29803923),
                    vec4(0.29411766, 0.21568628, 0.16078432, 0.29803923)
                ])
            ],
            read_outline_buffer(&descriptor, &data).unwrap()
        );
    }

    #[test]
    fn vertex_buffer_vertices_legacy() {
        // xenox/chr_en/en010201.camdo, vertex buffer 0, offset 159624 (vertex 4434)
        let data = hex!(
            // vertex 0
            bf2339ac be3e416c 3c94aa00
            002a0000
            3e11f7c1 3f255b32
            ffffffff
            e5a45300
            e457577f
            // vertex 1
            bf247df6 bdf6f646 3c6e6dc0
            002a0000
            0x3ec5d2b6 3f2253e6
            ffffffff
            9a004a00
            007f007f
        );

        let descriptor = VertexBufferDescriptor {
            data_offset: 0,
            vertex_count: 2,
            vertex_size: 36,
            attributes: vec![
                VertexAttribute {
                    data_type: DataType::Position,
                    data_size: 12,
                },
                VertexAttribute {
                    data_type: DataType::WeightIndex,
                    data_size: 4,
                },
                VertexAttribute {
                    data_type: DataType::TexCoord0,
                    data_size: 8,
                },
                VertexAttribute {
                    data_type: DataType::VertexColor,
                    data_size: 4,
                },
                VertexAttribute {
                    data_type: DataType::Normal,
                    data_size: 4,
                },
                VertexAttribute {
                    data_type: DataType::Tangent,
                    data_size: 4,
                },
            ],
            unk1: 0,
            unk2: 0,
            unk3: 0,
        };

        // Test read.
        let attributes = vec![
            AttributeData::Position(vec![
                vec3(-0.63759875, -0.18579644, 0.018147469),
                vec3(-0.642547, -0.12058692, 0.014552534),
            ]),
            AttributeData::WeightIndex(vec![[42, 0], [42, 0]]),
            AttributeData::TexCoord0(vec![
                vec2(0.14254667, 0.6459228),
                vec2(0.38637322, 0.6340927),
            ]),
            AttributeData::VertexColor(vec![vec4(1.0, 1.0, 1.0, 1.0), vec4(1.0, 1.0, 1.0, 1.0)]),
            AttributeData::Normal(vec![
                vec4(-0.21259843, -0.72440946, 0.6535433, 0.0),
                vec4(-0.8031496, 0.0, 0.5826772, 0.0),
            ]),
            AttributeData::Tangent(vec![
                vec4(-0.22047244, 0.68503934, 0.68503934, 1.0),
                vec4(0.0, 1.0, 0.0, 1.0),
            ]),
        ];
        assert_eq!(
            attributes,
            read_attributes(
                descriptor.data_offset as u64,
                descriptor.vertex_count,
                descriptor.vertex_size,
                &descriptor.attributes,
                &data,
                Endian::Big
            )
            .unwrap()
        );

        // Test write.
        let mut writer = Cursor::new(Vec::new());
        let new_descriptor = write_vertex_buffer(&mut writer, &attributes, Endian::Big).unwrap();
        assert_eq!(new_descriptor, descriptor);
        assert_hex_eq!(data, writer.into_inner());
    }

    #[test]
    fn weight_buffer_vertices_legacy() {
        // xenox/chr_en/en010201.camdo, vertex buffer 1
        let data = hex!(
            // vertex 0
            3f800000 00000000 00000000
            00000000
            // vertex 1
            3f800000 00000000 00000000
            01000000
        );

        let descriptor = VertexBufferDescriptor {
            data_offset: 0,
            vertex_count: 2,
            vertex_size: 16,
            attributes: vec![
                VertexAttribute {
                    data_type: DataType::SkinWeights2,
                    data_size: 12,
                },
                VertexAttribute {
                    data_type: DataType::BoneIndices2,
                    data_size: 4,
                },
            ],
            unk1: 0,
            unk2: 0,
            unk3: 0,
        };

        // TODO: Separate 3 component attribute for skin weights to have eventual write support?
        // Test read.
        let attributes = vec![
            AttributeData::SkinWeights2(vec![vec3(1.0, 0.0, 0.0), vec3(1.0, 0.0, 0.0)]),
            AttributeData::BoneIndices2(vec![[0, 0, 0, 0], [1, 0, 0, 0]]),
        ];
        assert_eq!(
            attributes,
            read_attributes(
                descriptor.data_offset as u64,
                descriptor.vertex_count,
                descriptor.vertex_size,
                &descriptor.attributes,
                &data,
                Endian::Big
            )
            .unwrap()
        );

        // Test write.
        let mut writer = Cursor::new(Vec::new());
        let new_descriptor = write_vertex_buffer(&mut writer, &attributes, Endian::Big).unwrap();
        assert_eq!(new_descriptor, descriptor);
        assert_hex_eq!(data, writer.into_inner());
    }

    #[test]
    fn vertex_buffer_indices_legacy() {
        // xenox/chr_en/en010201.camdo,  index buffer 0
        let data = hex!(00000001 00020002);

        let descriptor = IndexBufferDescriptor {
            data_offset: 0,
            index_count: 4,
            primitive_type: xc3_lib::vertex::PrimitiveType::TriangleList,
            index_format: xc3_lib::vertex::IndexFormat::Uint16,
            unk3: 0,
            unk4: 0,
        };

        // Test read.
        let indices = read_indices(&descriptor, &data, Endian::Big).unwrap();
        assert_eq!(vec![0, 1, 2, 2], indices);

        // Test write.
        let mut writer = Cursor::new(Vec::new());
        let new_descriptor = write_index_buffer(&mut writer, &indices, Endian::Big).unwrap();
        assert_eq!(new_descriptor, descriptor);
        assert_hex_eq!(data, writer.into_inner());
    }
}
