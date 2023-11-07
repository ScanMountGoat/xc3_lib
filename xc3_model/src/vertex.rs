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
use std::io::{Cursor, Seek, SeekFrom};

use binrw::{BinRead, BinReaderExt, BinResult};
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
    VertexColor(Vec<Vec4>),
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
            AttributeData::VertexColor(v) => v.len(),
            AttributeData::WeightIndex(v) => v.len(),
            AttributeData::SkinWeights(v) => v.len(),
            AttributeData::BoneIndices(v) => v.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
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

    // Assign morph target data to the appropriate vertex buffers.
    // TODO: Get names from the mxmd?
    // TODO: Add better tests for morph target data.
    if let Some(vertex_morphs) = &vertex_data.vertex_morphs {
        for descriptor in &vertex_morphs.descriptors {
            if let Some(buffer) = buffers.get_mut(descriptor.vertex_buffer_index as usize) {
                let start = descriptor.target_start_index as usize;
                let count = descriptor.target_count as usize;
                if let Some(targets) = vertex_morphs.targets.get(start..start + count) {
                    // TODO: Lots of morph targets use the exact same bytes?
                    dbg!(&vertex_data.vertex_buffers[descriptor.vertex_buffer_index as usize]);
                    for target in targets {
                        let attributes =
                            read_morph_attributes(target, &vertex_data.buffer).unwrap();
                        buffer.morph_targets.push(MorphTarget { attributes })
                    }
                }
            }
        }
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
        DataType::Uv3 => None,
        DataType::Uv4 => None,
        DataType::Unk9 => None,
        DataType::Unk10 => None,
        DataType::Unk11 => None,
        DataType::Unk12 => None,
        DataType::Unk13 => None,
        DataType::Unk14 => None,
        DataType::Unk15 => None,
        DataType::Unk16 => None,
        DataType::VertexColor => Some(AttributeData::VertexColor(
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
struct MorphBlendTargetVertex {
    position1: [f32; 3],
    normal: [u8; 4],
    _position2: [f32; 3],
    tangent: [u8; 4],
}

// Default and param buffer attributes.
#[derive(BinRead)]
struct MorphBufferVertex {
    position1: [f32; 3],
    _unk1: u32,
    normal: [u8; 4],
    tangent: [u8; 4],
    _unk2: u32,
    vertex_index: u32,
}

fn read_morph_attributes(
    morph_target: &xc3_lib::vertex::MorphTarget,
    model_bytes: &[u8],
) -> BinResult<Vec<AttributeData>> {
    let mut reader = Cursor::new(model_bytes);

    let mut positions = Vec::with_capacity(morph_target.vertex_count as usize);
    let mut normals = Vec::with_capacity(morph_target.vertex_count as usize);
    let mut tangents = Vec::with_capacity(morph_target.vertex_count as usize);

    // TODO: Compare xc3_wgpu to renderdoc for mio face.
    // TODO: Morph buffers other than the base have indices to overwrite values in the base target?
    // TODO: Preapply these adjustments so all targets have the base length?
    // TODO: Don't even save the base target as a morph target?
    // TODO: The morph vertex count doesn't always match the vertex buffer?
    for i in 0..morph_target.vertex_count as u64 {
        reader
            .seek(SeekFrom::Start(
                morph_target.data_offset as u64 + i * morph_target.vertex_size as u64,
            ))
            .unwrap();

        // These three bits define an enum for the buffer type.
        // Assume only one bit can be set.
        // TODO: Find a way to express this with bitflags?
        if morph_target.flags.blend_target_buffer() {
            let vertex: MorphBlendTargetVertex = reader.read_le().unwrap();
            positions.push(vertex.position1.into());
            normals.push(vertex.normal.map(|u| u as f32 / 255.0 * 2.0 - 1.0).into());
            tangents.push(vertex.tangent.map(|u| u as f32 / 255.0 * 2.0 - 1.0).into());
        } else {
            // TODO: How to handle the vertex index?
            let vertex: MorphBufferVertex = reader.read_le().unwrap();
            positions.push(vertex.position1.into());
            normals.push(vertex.normal.map(|u| u as f32 / 255.0 * 2.0 - 1.0).into());
            tangents.push(vertex.tangent.map(|u| u as f32 / 255.0 * 2.0 - 1.0).into());
        }
    }

    Ok(vec![
        AttributeData::Position(positions),
        AttributeData::Normal(normals),
        AttributeData::Tangent(tangents),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    use glam::{vec2, vec3, vec4};
    use hexlit::hex;
    use xc3_lib::vertex::{DataType, VertexAttribute};

    #[test]
    fn read_vertex_buffer_vertices() {
        // chr/ch/ch01012013.wismt, vertex buffer 0
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

        // TODO: Use strict equality for float comparisons?
        assert_eq!(
            vec![
                AttributeData::Position(vec![
                    vec3(0.10039953, 0.9038166, 0.07162084),
                    vec3(0.14499485, 0.91730505, 0.050502136)
                ]),
                AttributeData::WeightIndex(vec![275, 276]),
                AttributeData::Uv1(vec![
                    vec2(0.75997907, 0.6079358),
                    vec2(0.79126656, 0.6000591)
                ]),
                AttributeData::VertexColor(vec![
                    vec4(0.49803922, 0.0, 1.0, 1.0),
                    vec4(0.49803922, 0.0, 1.0, 1.0)
                ]),
                AttributeData::Normal(vec![
                    vec4(0.12941177, -0.019607844, 0.47843137, 0.0),
                    vec4(0.38431373, 0.047058824, 0.30980393, 0.0)
                ]),
                AttributeData::Tangent(vec![
                    vec4(0.47843137, 0.0, -0.12941177, 0.49803922),
                    vec4(0.30980393, 0.0, -0.38431373, 0.49803922)
                ])
            ],
            read_vertex_attributes(&descriptor, &data)
        );
    }

    #[test]
    fn read_weight_buffer_vertices() {
        // chr/ch/ch01012013.wismt, vertex buffer 12
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

        // TODO: Use strict equality for float comparisons?
        assert_eq!(
            vec![
                AttributeData::SkinWeights(vec![
                    vec4(0.7800107, 0.21998931, 0.0, 0.0),
                    vec4(0.77000076, 0.22999924, 0.0, 0.0)
                ]),
                AttributeData::BoneIndices(vec![[24, 23, 0, 0], [24, 23, 0, 0]]),
            ],
            read_vertex_attributes(&descriptor, &data)
        );
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

        // TODO: Use strict equality for float comparisons?
        assert_eq!(
            vec![
                AttributeData::Position(vec![
                    vec3(0.043739468, 1.3661073, -0.033391867),
                    vec3(0.048528977, 1.3739486, -0.03387388)
                ]),
                AttributeData::Normal(vec![
                    vec4(0.8117647, -0.49019605, -0.29411763, -0.99215686),
                    vec4(0.85882354, -0.40392154, -0.30196077, -0.99215686)
                ]),
                AttributeData::Tangent(vec![
                    vec4(-0.019607842, 0.4901961, -0.8666667, 1.0),
                    vec4(-0.035294116, 0.54509807, -0.827451, 1.0)
                ])
            ],
            read_morph_attributes(&target, &data).unwrap()
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

        // TODO: Use strict equality for float comparisons?
        assert_eq!(
            vec![
                AttributeData::Position(vec![
                    vec3(0.03181891, 1.3449626, -0.01804993),
                    vec3(0.03641526, 1.3508042, -0.021831524)
                ]),
                AttributeData::Normal(vec![
                    vec4(0.6784314, -0.7254902, -0.0745098, -0.99215686),
                    vec4(0.73333335, -0.654902, -0.1607843, -0.99215686)
                ]),
                AttributeData::Tangent(vec![
                    vec4(0.254902, 0.32549024, -0.90588236, 1.0),
                    vec4(0.11372554, 0.35686278, -0.92156863, 1.0)
                ])
            ],
            read_morph_attributes(&target, &data).unwrap()
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

        // TODO: Use strict equality for float comparisons?
        assert_eq!(
            vec![
                AttributeData::Position(vec![
                    vec3(-0.0025982223, -0.005033493, 0.00014753453),
                    vec3(-0.0016574785, -0.003010869, -9.961426e-6)
                ]),
                AttributeData::Normal(vec![
                    vec4(0.9372549, -0.12156862, 0.3176471, -1.0),
                    vec4(0.92941177, -0.12941176, 0.32549024, -1.0)
                ]),
                AttributeData::Tangent(vec![
                    vec4(-0.16862744, 0.654902, 0.73333335, 1.0),
                    vec4(0.12941182, 0.9843137, 0.027451038, 1.0)
                ])
            ],
            read_morph_attributes(&target, &data).unwrap()
        );
    }
}
