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
use std::io::{Cursor, Seek, SeekFrom, Write};

use binrw::{BinRead, BinReaderExt, BinResult, BinWrite};
use glam::{Vec2, Vec3, Vec4};
use xc3_lib::vertex::{DataType, IndexBufferDescriptor, VertexBufferDescriptor, VertexData};

use crate::{skinning::SkinWeights, IndexBuffer, MorphTarget, VertexBuffer, Weights};

// TODO: Add an option to convert a collection of these to the vertex above?
// TODO: How to handle normalized attributes?
// TODO: Link to appropriate xc3_lib types and fields.
/// The per vertex values for a vertex attribute.
#[derive(Debug, PartialEq)]
pub enum AttributeData {
    Position(Vec<Vec3>),
    Normal(Vec<Vec4>),
    Tangent(Vec<Vec4>),
    Uv1(Vec<Vec2>),
    Uv2(Vec<Vec2>),
    Uv3(Vec<Vec2>),
    Uv4(Vec<Vec2>),
    Uv5(Vec<Vec2>),
    Uv6(Vec<Vec2>),
    Uv7(Vec<Vec2>),
    Uv8(Vec<Vec2>),
    Uv9(Vec<Vec2>),
    VertexColor(Vec<Vec4>),
    VertexColor2(Vec<Vec4>),
    WeightIndex(Vec<u32>), // TODO: [u8; 4]?
    // TODO: Should these be handled separately?
    SkinWeights(Vec<Vec4>),
    BoneIndices(Vec<[u8; 4]>),
}

impl AttributeData {
    pub fn len(&self) -> usize {
        match self {
            AttributeData::Position(v) => v.len(),
            AttributeData::Normal(v) => v.len(),
            AttributeData::Tangent(v) => v.len(),
            AttributeData::Uv1(v) => v.len(),
            AttributeData::Uv2(v) => v.len(),
            AttributeData::Uv3(v) => v.len(),
            AttributeData::Uv4(v) => v.len(),
            AttributeData::Uv5(v) => v.len(),
            AttributeData::Uv6(v) => v.len(),
            AttributeData::Uv7(v) => v.len(),
            AttributeData::Uv8(v) => v.len(),
            AttributeData::Uv9(v) => v.len(),
            AttributeData::VertexColor(v) => v.len(),
            AttributeData::VertexColor2(v) => v.len(),
            AttributeData::WeightIndex(v) => v.len(),
            AttributeData::SkinWeights(v) => v.len(),
            AttributeData::BoneIndices(v) => v.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn write<W: Write + Seek>(
        &self,
        writer: &mut W,
        offset: u64,
        stride: u64,
    ) -> BinResult<()> {
        match self {
            AttributeData::Position(values) => {
                write_data(writer, values, offset, stride, write_f32x3)
            }
            AttributeData::Normal(values) => {
                write_data(writer, values, offset, stride, write_snorm8x4)
            }
            AttributeData::Tangent(values) => {
                write_data(writer, values, offset, stride, write_snorm8x4)
            }
            AttributeData::Uv1(values) => write_data(writer, values, offset, stride, write_f32x2),
            AttributeData::Uv2(values) => write_data(writer, values, offset, stride, write_f32x2),
            AttributeData::Uv3(values) => write_data(writer, values, offset, stride, write_f32x2),
            AttributeData::Uv4(values) => write_data(writer, values, offset, stride, write_f32x2),
            AttributeData::Uv5(values) => write_data(writer, values, offset, stride, write_f32x2),
            AttributeData::Uv6(values) => write_data(writer, values, offset, stride, write_f32x2),
            AttributeData::Uv7(values) => write_data(writer, values, offset, stride, write_f32x2),
            AttributeData::Uv8(values) => write_data(writer, values, offset, stride, write_f32x2),
            AttributeData::Uv9(values) => write_data(writer, values, offset, stride, write_f32x2),
            AttributeData::VertexColor(values) => {
                write_data(writer, values, offset, stride, write_unorm8x4)
            }
            AttributeData::VertexColor2(values) => {
                write_data(writer, values, offset, stride, write_unorm8x4)
            }
            AttributeData::WeightIndex(values) => {
                write_data(writer, values, offset, stride, write_u32)
            }
            AttributeData::SkinWeights(values) => {
                write_data(writer, values, offset, stride, write_unorm16x4)
            }
            AttributeData::BoneIndices(values) => {
                write_data(writer, values, offset, stride, write_u8x4)
            }
        }
    }
}

impl From<&AttributeData> for xc3_lib::vertex::VertexAttribute {
    fn from(value: &AttributeData) -> Self {
        match value {
            AttributeData::Position(_) => xc3_lib::vertex::VertexAttribute {
                data_type: xc3_lib::vertex::DataType::Position,
                data_size: 12,
            },
            AttributeData::Normal(_) => xc3_lib::vertex::VertexAttribute {
                data_type: xc3_lib::vertex::DataType::Normal,
                data_size: 4,
            },
            AttributeData::Tangent(_) => xc3_lib::vertex::VertexAttribute {
                data_type: xc3_lib::vertex::DataType::Tangent,
                data_size: 4,
            },
            AttributeData::Uv1(_) => xc3_lib::vertex::VertexAttribute {
                data_type: xc3_lib::vertex::DataType::Uv1,
                data_size: 8,
            },
            AttributeData::Uv2(_) => xc3_lib::vertex::VertexAttribute {
                data_type: xc3_lib::vertex::DataType::Uv2,
                data_size: 8,
            },
            AttributeData::Uv3(_) => xc3_lib::vertex::VertexAttribute {
                data_type: xc3_lib::vertex::DataType::Uv3,
                data_size: 8,
            },
            AttributeData::Uv4(_) => xc3_lib::vertex::VertexAttribute {
                data_type: xc3_lib::vertex::DataType::Uv4,
                data_size: 8,
            },
            AttributeData::Uv5(_) => xc3_lib::vertex::VertexAttribute {
                data_type: xc3_lib::vertex::DataType::Uv5,
                data_size: 8,
            },
            AttributeData::Uv6(_) => xc3_lib::vertex::VertexAttribute {
                data_type: xc3_lib::vertex::DataType::Uv6,
                data_size: 8,
            },
            AttributeData::Uv7(_) => xc3_lib::vertex::VertexAttribute {
                data_type: xc3_lib::vertex::DataType::Uv7,
                data_size: 8,
            },
            AttributeData::Uv8(_) => xc3_lib::vertex::VertexAttribute {
                data_type: xc3_lib::vertex::DataType::Uv8,
                data_size: 8,
            },
            AttributeData::Uv9(_) => xc3_lib::vertex::VertexAttribute {
                data_type: xc3_lib::vertex::DataType::Uv9,
                data_size: 8,
            },
            AttributeData::VertexColor(_) => xc3_lib::vertex::VertexAttribute {
                data_type: xc3_lib::vertex::DataType::VertexColorUnk17,
                data_size: 4,
            },
            AttributeData::VertexColor2(_) => xc3_lib::vertex::VertexAttribute {
                data_type: xc3_lib::vertex::DataType::VertexColorUnk14,
                data_size: 4,
            },
            AttributeData::WeightIndex(_) => xc3_lib::vertex::VertexAttribute {
                data_type: xc3_lib::vertex::DataType::WeightIndex,
                data_size: 4,
            },
            AttributeData::SkinWeights(_) => xc3_lib::vertex::VertexAttribute {
                data_type: xc3_lib::vertex::DataType::SkinWeights,
                data_size: 8,
            },
            AttributeData::BoneIndices(_) => xc3_lib::vertex::VertexAttribute {
                data_type: xc3_lib::vertex::DataType::BoneIndices,
                data_size: 4,
            },
        }
    }
}

pub fn read_vertex_buffers(
    vertex_data: &VertexData,
    skinning: Option<&xc3_lib::mxmd::Skinning>,
) -> (Vec<VertexBuffer>, Option<Weights>) {
    // TODO: Don't save the weights buffer.
    // TODO: Panic if the weights buffer is not the last buffer?
    let mut buffers: Vec<_> = vertex_data
        .vertex_buffers
        .iter()
        .map(|descriptor| {
            let attributes = read_vertex_attributes(descriptor, &vertex_data.buffer);
            VertexBuffer {
                attributes,
                morph_targets: Vec::new(),
            }
        })
        .collect();

    // TODO: Get names from the mxmd?
    // TODO: Add better tests for morph target data.
    if let Some(vertex_morphs) = &vertex_data.vertex_morphs {
        assign_morph_targets(vertex_morphs, &mut buffers, vertex_data);
    }

    // TODO: Buffers have skinning indices but not weights?
    // TODO: Is this the best place to do this?
    let skin_weights = skinning.and_then(|skinning| {
        let vertex_weights = vertex_data.weights.as_ref()?;
        let weights_index = vertex_weights.vertex_buffer_index as usize;
        let weights_buffer = buffers.get(weights_index)?;
        let (weights, bone_indices) = skin_weights_bone_indices(weights_buffer)?;
        Some(Weights {
            skin_weights: SkinWeights {
                bone_indices,
                weights,
                // TODO: Will this cover all bone indices?
                bone_names: skinning.bones.iter().map(|b| b.name.clone()).collect(),
            },
            weight_groups: vertex_weights.groups.clone(),
            weight_lods: vertex_weights.weight_lods.clone(),
        })
    });

    (buffers, skin_weights)
}

fn assign_morph_targets(
    vertex_morphs: &xc3_lib::vertex::VertexMorphs,
    buffers: &mut [VertexBuffer],
    vertex_data: &VertexData,
) {
    // TODO: Find a cleaner way to write this.
    for descriptor in &vertex_morphs.descriptors {
        if let Some(buffer) = buffers.get_mut(descriptor.vertex_buffer_index as usize) {
            let start = descriptor.target_start_index as usize;
            let count = descriptor.target_count as usize;
            if let Some(targets) = vertex_morphs.targets.get(start..start + count) {
                // TODO: Lots of morph targets use the exact same bytes?
                // Assume the first target is the base target.
                if let Some((base_target, targets)) = targets.split_first() {
                    // TODO: Check flags?
                    // These three bits define an enum for the buffer type.
                    // Assume only one bit can be set.
                    // TODO: Find a way to express this with bitflags?
                    let base = read_morph_blend_target(base_target, &vertex_data.buffer).unwrap();

                    buffer.morph_targets = targets
                        .iter()
                        .map(|target| {
                            // Apply remaining targets onto the base target values.
                            let vertices =
                                read_morph_buffer_target(target, &vertex_data.buffer).unwrap();
                            let mut position_deltas = vec![Vec3::ZERO; base.positions.len()];
                            let mut normal_deltas = base.normals.clone();
                            let mut tangent_deltas = base.tangents.clone();

                            for vertex in vertices {
                                let i = vertex.vertex_index as usize;
                                position_deltas[i] = vertex.position_delta;
                                normal_deltas[i] = vertex.normal - base.normals[i];
                                tangent_deltas[i] = vertex.tangent - base.tangents[i];
                            }

                            MorphTarget {
                                position_deltas,
                                normal_deltas,
                                tangent_deltas,
                            }
                        })
                        .collect();

                    buffer
                        .attributes
                        .push(AttributeData::Position(base.positions));
                    buffer.attributes.push(AttributeData::Normal(base.normals));
                    buffer
                        .attributes
                        .push(AttributeData::Tangent(base.tangents));
                }
            }
        }
    }
}

fn skin_weights_bone_indices(buffer: &VertexBuffer) -> Option<(Vec<Vec4>, Vec<[u8; 4]>)> {
    let weights = buffer.attributes.iter().find_map(|a| match a {
        AttributeData::SkinWeights(values) => Some(values.clone()),
        _ => None,
    })?;
    let indices = buffer.attributes.iter().find_map(|a| match a {
        AttributeData::BoneIndices(values) => Some(values.clone()),
        _ => None,
    })?;

    Some((weights, indices))
}

pub fn read_index_buffers(vertex_data: &VertexData) -> Vec<IndexBuffer> {
    vertex_data
        .index_buffers
        .iter()
        .map(|descriptor| IndexBuffer {
            indices: read_indices(descriptor, &vertex_data.buffer).unwrap(),
        })
        .collect()
}

pub fn read_indices(descriptor: &IndexBufferDescriptor, buffer: &[u8]) -> BinResult<Vec<u16>> {
    // TODO: Are all index buffers using u16 for indices?
    let mut reader = Cursor::new(buffer);
    reader.seek(SeekFrom::Start(descriptor.data_offset as u64))?;

    let mut indices = Vec::with_capacity(descriptor.index_count as usize);
    for _ in 0..descriptor.index_count {
        let index: u16 = reader.read_le()?;
        indices.push(index);
    }
    Ok(indices)
}

pub fn read_vertex_attributes(
    descriptor: &VertexBufferDescriptor,
    buffer: &[u8],
) -> Vec<AttributeData> {
    let mut offset = 0;
    descriptor
        .attributes
        .iter()
        .filter_map(|a| {
            let data = read_attribute(a, descriptor, offset, buffer);
            offset += a.data_size as u64;

            data
        })
        .collect()
}

fn read_attribute(
    a: &xc3_lib::vertex::VertexAttribute,
    d: &VertexBufferDescriptor,
    offset: u64,
    buffer: &[u8],
) -> Option<AttributeData> {
    // TODO: handle all cases and don't return option.
    match a.data_type {
        DataType::Position => Some(AttributeData::Position(
            read_data(d, offset, buffer, read_f32x3).ok()?,
        )),
        DataType::Unk1 => None,
        DataType::Unk2 => None,
        DataType::WeightIndex => Some(AttributeData::WeightIndex(
            read_data(d, offset, buffer, read_u32).ok()?,
        )),
        DataType::Unk4 => None,
        DataType::Uv1 => Some(AttributeData::Uv1(
            read_data(d, offset, buffer, read_f32x2).ok()?,
        )),
        DataType::Uv2 => Some(AttributeData::Uv2(
            read_data(d, offset, buffer, read_f32x2).ok()?,
        )),
        DataType::Uv3 => Some(AttributeData::Uv3(
            read_data(d, offset, buffer, read_f32x2).ok()?,
        )),
        DataType::Uv4 => Some(AttributeData::Uv4(
            read_data(d, offset, buffer, read_f32x2).ok()?,
        )),
        DataType::Uv5 => Some(AttributeData::Uv5(
            read_data(d, offset, buffer, read_f32x2).ok()?,
        )),
        DataType::Uv6 => Some(AttributeData::Uv6(
            read_data(d, offset, buffer, read_f32x2).ok()?,
        )),
        DataType::Uv7 => Some(AttributeData::Uv7(
            read_data(d, offset, buffer, read_f32x2).ok()?,
        )),
        DataType::Uv8 => Some(AttributeData::Uv8(
            read_data(d, offset, buffer, read_f32x2).ok()?,
        )),
        DataType::Uv9 => Some(AttributeData::Uv9(
            read_data(d, offset, buffer, read_f32x2).ok()?,
        )),
        DataType::VertexColorUnk14 => Some(AttributeData::VertexColor2(
            read_data(d, offset, buffer, read_unorm8x4).ok()?,
        )),
        DataType::Unk15 => None,
        DataType::Unk16 => None,
        DataType::VertexColorUnk17 => Some(AttributeData::VertexColor(
            read_data(d, offset, buffer, read_unorm8x4).ok()?,
        )),
        DataType::Unk18 => None,
        DataType::Normal => Some(AttributeData::Normal(
            read_data(d, offset, buffer, read_snorm8x4).ok()?,
        )),
        DataType::Tangent => Some(AttributeData::Tangent(
            read_data(d, offset, buffer, read_snorm8x4).ok()?,
        )),
        DataType::Normal2 => Some(AttributeData::Normal(
            read_data(d, offset, buffer, read_snorm8x4).ok()?,
        )),
        DataType::Unk33 => None,
        DataType::SkinWeights => Some(AttributeData::SkinWeights(
            read_data(d, offset, buffer, read_unorm16x4).ok()?,
        )),
        DataType::BoneIndices => Some(AttributeData::BoneIndices(
            read_data(d, offset, buffer, read_u8x4).ok()?,
        )),
        DataType::Unk52 => None,
    }
}

fn read_data<T, F>(
    descriptor: &VertexBufferDescriptor,
    offset: u64,
    buffer: &[u8],
    read_item: F,
) -> BinResult<Vec<T>>
where
    F: Fn(&mut Cursor<&[u8]>) -> BinResult<T>,
{
    let mut reader = Cursor::new(buffer);

    let mut values = Vec::with_capacity(descriptor.vertex_count as usize);
    for i in 0..descriptor.vertex_count as u64 {
        let offset = descriptor.data_offset as u64 + i * descriptor.vertex_size as u64 + offset;
        reader.seek(SeekFrom::Start(offset)).unwrap();

        values.push(read_item(&mut reader)?);
    }
    Ok(values)
}

fn read_u32(reader: &mut Cursor<&[u8]>) -> BinResult<u32> {
    reader.read_le()
}

fn read_u8x4(reader: &mut Cursor<&[u8]>) -> BinResult<[u8; 4]> {
    reader.read_le()
}

fn read_f32x2(reader: &mut Cursor<&[u8]>) -> BinResult<Vec2> {
    let value: [f32; 2] = reader.read_le()?;
    Ok(value.into())
}

fn read_f32x3(reader: &mut Cursor<&[u8]>) -> BinResult<Vec3> {
    let value: [f32; 3] = reader.read_le()?;
    Ok(value.into())
}

fn read_unorm8x4(reader: &mut Cursor<&[u8]>) -> BinResult<Vec4> {
    let value: [u8; 4] = reader.read_le()?;
    Ok(value.map(|u| u as f32 / 255.0).into())
}

fn read_snorm8x4(reader: &mut Cursor<&[u8]>) -> BinResult<Vec4> {
    let value: [i8; 4] = reader.read_le()?;
    Ok(value.map(|i| i as f32 / 255.0).into())
}

fn read_unorm16x4(reader: &mut Cursor<&[u8]>) -> BinResult<Vec4> {
    let value: [u16; 4] = reader.read_le()?;
    Ok(value.map(|u| u as f32 / 65535.0).into())
}

// The base target matches vertex attributes from RenderDoc.
// 0 Float32x3 Position
// 1 Unorm8x4 Normals
// 2 Float32x3 Position
// 3 Unorm8x4 Tangent
#[derive(BinRead)]
struct MorphBufferBlendTargetVertex {
    position1: [f32; 3],
    normal: [u8; 4],
    _position2: [f32; 3],
    tangent: [u8; 4],
}

// Default and param buffer attributes.
#[derive(BinRead)]
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
struct MorphBlendTargetAttributes {
    positions: Vec<Vec3>,
    normals: Vec<Vec4>,
    tangents: Vec<Vec4>,
}

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
) -> BinResult<MorphBlendTargetAttributes> {
    // Only the base target contains data for all vertices.
    // This includes required position, normal, and tangent attributes.
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut tangents = Vec::new();

    let mut reader = Cursor::new(model_bytes);
    for i in 0..base_target.vertex_count as u64 {
        // TODO: assume data is tightly packed and seek once?
        reader
            .seek(SeekFrom::Start(
                base_target.data_offset as u64 + i * base_target.vertex_size as u64,
            ))
            .unwrap();

        let vertex: MorphBufferBlendTargetVertex = reader.read_le()?;
        positions.push(vertex.position1.into());
        normals.push(vertex.normal.map(|u| u as f32 / 255.0 * 2.0 - 1.0).into());
        tangents.push(vertex.tangent.map(|u| u as f32 / 255.0 * 2.0 - 1.0).into());
    }

    Ok(MorphBlendTargetAttributes {
        positions,
        normals,
        tangents,
    })
}

fn read_morph_buffer_target(
    morph_target: &xc3_lib::vertex::MorphTarget,
    model_bytes: &[u8],
) -> BinResult<Vec<MorphTargetVertex>> {
    let mut reader = Cursor::new(model_bytes);

    (0..morph_target.vertex_count as u64)
        .map(|i| {
            // TODO: assume data is tightly packed and seek once?
            reader
                .seek(SeekFrom::Start(
                    morph_target.data_offset as u64 + i * morph_target.vertex_size as u64,
                ))
                .unwrap();

            let vertex: MorphBufferTargetVertex = reader.read_le()?;

            Ok(MorphTargetVertex {
                position_delta: vertex.position_delta.into(),
                normal: vertex.normal.map(|u| u as f32 / 255.0 * 2.0 - 1.0).into(),
                tangent: vertex.tangent.map(|u| u as f32 / 255.0 * 2.0 - 1.0).into(),
                vertex_index: vertex.vertex_index,
            })
        })
        .collect()
}

// TODO: support u32?
pub fn write_index_buffer<W: Write + Seek>(
    writer: &mut W,
    indices: &[u16],
) -> BinResult<IndexBufferDescriptor> {
    let data_offset = writer.stream_position()? as u32;

    indices.write_le(writer)?;

    Ok(IndexBufferDescriptor {
        data_offset,
        index_count: indices.len() as u32,
        unk1: xc3_lib::vertex::Unk1::Unk0,
        unk2: xc3_lib::vertex::Unk2::Unk0,
        unk3: 0,
        unk4: 0,
    })
}

pub fn write_vertex_buffer<W: Write + Seek>(
    writer: &mut W,
    attribute_data: &[AttributeData],
) -> BinResult<VertexBufferDescriptor> {
    let data_offset = writer.stream_position()? as u32;

    let attributes: Vec<_> = attribute_data
        .iter()
        .map(xc3_lib::vertex::VertexAttribute::from)
        .collect();

    let vertex_size = attributes.iter().map(|a| a.data_size as u32).sum();

    // TODO: Check if all the arrays have the same length.
    let vertex_count = attribute_data[0].len() as u32;

    // TODO: Include a base offset?
    let mut offset = writer.stream_position()?;
    for (a, data) in attributes.iter().zip(attribute_data) {
        data.write(writer, offset, vertex_size as u64)?;
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

fn write_data<T, F, W>(
    writer: &mut W,
    values: &[T],
    offset: u64,
    stride: u64,
    write_item: F,
) -> BinResult<()>
where
    W: Write + Seek,
    F: Fn(&mut W, &T) -> BinResult<()>,
{
    for (i, value) in values.iter().enumerate() {
        writer.seek(SeekFrom::Start(offset + i as u64 * stride))?;
        write_item(writer, value)?;
    }

    Ok(())
}

fn write_u32<W: Write + Seek>(writer: &mut W, value: &u32) -> BinResult<()> {
    value.write_le(writer)
}

fn write_u8x4<W: Write + Seek>(writer: &mut W, value: &[u8; 4]) -> BinResult<()> {
    value.write_le(writer)
}

fn write_f32x2<W: Write + Seek>(writer: &mut W, value: &Vec2) -> BinResult<()> {
    value.to_array().write_le(writer)
}

fn write_f32x3<W: Write + Seek>(writer: &mut W, value: &Vec3) -> BinResult<()> {
    value.to_array().write_le(writer)
}

fn write_unorm8x4<W: Write + Seek>(writer: &mut W, value: &Vec4) -> BinResult<()> {
    value.to_array().map(|f| (f * 255.0) as u8).write_le(writer)
}

fn write_unorm16x4<W: Write + Seek>(writer: &mut W, value: &Vec4) -> BinResult<()> {
    value
        .to_array()
        .map(|f| (f * 65535.0) as u16)
        .write_le(writer)
}

fn write_snorm8x4<W: Write + Seek>(writer: &mut W, value: &Vec4) -> BinResult<()> {
    value.to_array().map(|f| (f * 255.0) as i8).write_le(writer)
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::assert_hex_eq;

    use glam::{vec2, vec3, vec4};
    use hexlit::hex;
    use xc3_lib::vertex::{DataType, VertexAttribute};

    #[test]
    fn vertex_buffer_indices() {
        // xeno3/chr/ch/ch01012013.wismt, index buffer 0
        let data = hex!(00000100 02000100);

        let descriptor = IndexBufferDescriptor {
            data_offset: 0,
            index_count: 4,
            unk1: xc3_lib::vertex::Unk1::Unk0,
            unk2: xc3_lib::vertex::Unk2::Unk0,
            unk3: 0,
            unk4: 0,
        };

        // Test read.
        let indices = read_indices(&descriptor, &data).unwrap();
        assert_eq!(vec![0, 1, 2, 1], indices);

        // Test write.
        let mut writer = Cursor::new(Vec::new());
        let new_descriptor = write_index_buffer(&mut writer, &indices).unwrap();
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
                    data_type: DataType::Uv1,
                    data_size: 8,
                },
                VertexAttribute {
                    data_type: DataType::VertexColorUnk17,
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
            AttributeData::WeightIndex(vec![275, 276]),
            AttributeData::Uv1(vec![
                vec2(0.75997907, 0.6079358),
                vec2(0.79126656, 0.6000591),
            ]),
            AttributeData::VertexColor(vec![
                vec4(0.49803922, 0.0, 1.0, 1.0),
                vec4(0.49803922, 0.0, 1.0, 1.0),
            ]),
            AttributeData::Normal(vec![
                vec4(0.12941177, -0.019607844, 0.47843137, 0.0),
                vec4(0.38431373, 0.047058824, 0.30980393, 0.0),
            ]),
            AttributeData::Tangent(vec![
                vec4(0.47843137, 0.0, -0.12941177, 0.49803922),
                vec4(0.30980393, 0.0, -0.38431373, 0.49803922),
            ]),
        ];
        assert_eq!(attributes, read_vertex_attributes(&descriptor, &data));

        // Test write.
        let mut writer = Cursor::new(Vec::new());
        let new_descriptor = write_vertex_buffer(&mut writer, &attributes).unwrap();
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
        assert_eq!(attributes, read_vertex_attributes(&descriptor, &data));

        // Test write.
        let mut writer = Cursor::new(Vec::new());
        let new_descriptor = write_vertex_buffer(&mut writer, &attributes).unwrap();
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
                    data_type: DataType::Uv1,
                    data_size: 8,
                },
                VertexAttribute {
                    data_type: DataType::Uv2,
                    data_size: 8,
                },
                VertexAttribute {
                    data_type: DataType::Uv3,
                    data_size: 8,
                },
                VertexAttribute {
                    data_type: DataType::Uv4,
                    data_size: 8,
                },
                VertexAttribute {
                    data_type: DataType::Uv5,
                    data_size: 8,
                },
                VertexAttribute {
                    data_type: DataType::Uv6,
                    data_size: 8,
                },
                VertexAttribute {
                    data_type: DataType::Uv7,
                    data_size: 8,
                },
                VertexAttribute {
                    data_type: DataType::Uv8,
                    data_size: 8,
                },
                VertexAttribute {
                    data_type: DataType::Uv9,
                    data_size: 8,
                },
                VertexAttribute {
                    data_type: DataType::VertexColorUnk14,
                    data_size: 4,
                },
                VertexAttribute {
                    data_type: DataType::VertexColorUnk17,
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
            AttributeData::Uv1(vec![
                vec2(1.9810443, -47.392357),
                vec2(1.9424212, -47.339436),
            ]),
            AttributeData::Uv2(vec![
                vec2(2.2272549, -47.516212),
                vec2(2.1843944, -47.470936),
            ]),
            AttributeData::Uv3(vec![
                vec2(1.9810443, -47.392357),
                vec2(1.9421549, -47.34271),
            ]),
            AttributeData::Uv4(vec![
                vec2(2.3970675, -2.3563137),
                vec2(2.3950076, -2.3534913),
            ]),
            AttributeData::Uv5(vec![
                vec2(2.4101992, -2.362919),
                vec2(2.4079132, -2.3605044),
            ]),
            AttributeData::Uv6(vec![
                vec2(2.3970675, -2.3563137),
                vec2(2.3949933, -2.353666),
            ]),
            AttributeData::Uv7(vec![
                vec2(0.17541784, 0.5742469),
                vec2(0.20254421, 0.58608305),
            ]),
            AttributeData::Uv8(vec![
                vec2(0.6337629, 0.4448461),
                vec2(0.6385515, 0.60845864),
            ]),
            AttributeData::Uv9(vec![
                vec2(-0.41085857, 0.108098745),
                vec2(-0.42711625, 0.13474321),
            ]),
            AttributeData::VertexColor2(vec![
                vec4(0.49803922, 0.0, 0.49803922, 0.0),
                vec4(0.0, 0.0, 1.0, 0.0),
            ]),
            AttributeData::VertexColor(vec![vec4(1.0, 1.0, 1.0, 1.0), vec4(1.0, 1.0, 1.0, 1.0)]),
            AttributeData::Normal(vec![
                vec4(-0.05882353, 0.47058824, 0.13725491, 0.0),
                vec4(-0.09411765, 0.45882353, 0.16470589, 0.0),
            ]),
            AttributeData::Tangent(vec![
                vec4(0.49019608, 0.0627451, 0.003921569, 0.49803922),
                vec4(0.4862745, 0.101960786, 0.0, 0.49803922),
            ]),
        ];
        assert_eq!(attributes, read_vertex_attributes(&descriptor, &data));

        // Test write.
        let mut writer = Cursor::new(Vec::new());
        let new_descriptor = write_vertex_buffer(&mut writer, &attributes).unwrap();
        assert_eq!(new_descriptor, descriptor);
        assert_hex_eq!(data, writer.into_inner());
    }

    #[test]
    fn read_morph_blend_target_vertices() {
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

        assert_eq!(
            MorphBlendTargetAttributes {
                positions: vec![
                    vec3(0.043739468, 1.3661073, -0.033391867),
                    vec3(0.048528977, 1.3739486, -0.03387388)
                ],
                normals: vec![
                    vec4(0.8117647, -0.49019605, -0.29411763, -0.99215686),
                    vec4(0.85882354, -0.40392154, -0.30196077, -0.99215686)
                ],
                tangents: vec![
                    vec4(-0.019607842, 0.4901961, -0.8666667, 1.0),
                    vec4(-0.035294116, 0.54509807, -0.827451, 1.0)
                ]
            },
            read_morph_blend_target(&target, &data).unwrap()
        );
    }

    #[test]
    fn read_morph_default_buffer_vertices() {
        // xeno3/chr/ch/ch01027000.wismt, "face_D2_shape", target index 325.
        let data = hex!(
            // vertex 0
            8c54023d bc27ac3f 72dd93bc 00000000
            d6237601
            a0a90cff
            00000000
            04000000
            // vertex 1
            2b28153d 27e7ac3f 06d8b2bc 00000000
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
    }

    #[test]
    fn read_morph_param_buffer_vertices() {
        // xeno3/chr/ch/ch01027000.wismt, "face_D2_shape", target index 326.
        let data = hex!(
            // vertex 0
            f0462abb 00f0a4bb 80b31a39 00000000
            f770a800 6ad3ddff 00000000
            d8000000
            // vertex 1
            c03fd9ba 005245bb 002027b7 00000000
            f66fa900 90fd83ff 00000000
            d9000000
        );

        let target = xc3_lib::vertex::MorphTarget {
            data_offset: 0,
            vertex_count: 2,
            vertex_size: 32,
            flags: xc3_lib::vertex::MorphTargetFlags::new(0u16, false, false, true, 0u8.into()),
        };

        assert_eq!(
            vec![
                MorphTargetVertex {
                    position_delta: vec3(-0.0025982223, -0.005033493, 0.00014753453),
                    normal: vec4(0.9372549, -0.12156862, 0.3176471, -1.0),
                    tangent: vec4(-0.16862744, 0.654902, 0.73333335, 1.0),
                    vertex_index: 216
                },
                MorphTargetVertex {
                    position_delta: vec3(-0.0016574785, -0.003010869, -9.961426e-6),
                    normal: vec4(0.92941177, -0.12941176, 0.32549024, -1.0),
                    tangent: vec4(0.12941182, 0.9843137, 0.027451038, 1.0),
                    vertex_index: 217
                }
            ],
            read_morph_buffer_target(&target, &data).unwrap()
        );
    }
}
