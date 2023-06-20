use std::io::{Cursor, Seek, SeekFrom};

use binrw::BinReaderExt;
use bytemuck::{Pod, Zeroable};
use glam::{vec4, Vec3, Vec4};
use xc3_lib::vertex::{
    IndexBufferDescriptor, VertexAnimationTarget, VertexBufferDescriptor, VertexData,
};

// TODO: Switch to struct of arrays instead of array of structs.
// This would better encode which attributes are actually present and is easier for applications.
// TODO: Add array of structs as an option for realtime rendering?
// TODO: Share code between these two representations?
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Default, Pod, Zeroable)]
pub struct Vertex {
    pub position: glam::Vec3,
    pub weight_index: u32,
    pub vertex_color: glam::Vec4,
    pub normal: glam::Vec4,
    pub tangent: glam::Vec4,
    // TODO: use vec2 for this?
    pub uv1: glam::Vec4,
}

pub fn read_indices(vertex_data: &VertexData, descriptor: &IndexBufferDescriptor) -> Vec<u16> {
    // TODO: Are all index buffers using u16 for indices?
    let mut reader = Cursor::new(&vertex_data.buffer);
    reader
        .seek(SeekFrom::Start(descriptor.data_offset as u64))
        .unwrap();

    let mut indices = Vec::new();
    for _ in 0..descriptor.index_count {
        let index: u16 = reader.read_le().unwrap();
        indices.push(index);
    }
    indices
}

// TODO: rename to VertexBufferDescriptor?
/// Reads the vertex attributes for `buffer` at index `buffer_index`.
pub fn read_vertices(
    buffer: &VertexBufferDescriptor,
    buffer_index: usize,
    vertex_data: &VertexData,
) -> Vec<Vertex> {
    // Start with default values for each attribute.
    let mut vertices = vec![
        Vertex {
            position: Vec3::ZERO,
            weight_index: 0,
            vertex_color: Vec4::ZERO,
            normal: Vec4::ZERO,
            tangent: Vec4::ZERO,
            uv1: Vec4::ZERO
        };
        buffer.vertex_count as usize
    ];

    // The game renders attributes from both the vertex and optional animation buffer.
    // Merge attributes into a single buffer to allow using the same shader.
    // TODO: Which buffer takes priority?
    assign_vertex_buffer_attributes(&mut vertices, &vertex_data.buffer, buffer);

    if let Some(base_target) = base_vertex_target(vertex_data, buffer_index) {
        assign_animation_buffer_attributes(&mut vertices, &vertex_data.buffer, buffer, base_target);
    }

    vertices
}

fn assign_vertex_buffer_attributes(
    vertices: &mut [Vertex],
    bytes: &[u8],
    descriptor: &VertexBufferDescriptor,
) {
    let mut reader = Cursor::new(bytes);

    for i in 0..descriptor.vertex_count as u64 {
        reader
            .seek(SeekFrom::Start(
                descriptor.data_offset as u64 + i * descriptor.vertex_size as u64,
            ))
            .unwrap();

        // TODO: How to handle missing attributes.
        // TODO: Document conversion formulas to float in xc3_lib.
        // TODO: Is switching for each vertex the base way to do this?
        for a in &descriptor.attributes {
            match a.data_type {
                xc3_lib::vertex::DataType::Position => {
                    let value: [f32; 3] = reader.read_le().unwrap();
                    vertices[i as usize].position = value.into();
                }
                xc3_lib::vertex::DataType::VertexColor => {
                    let value: [u8; 4] = reader.read_le().unwrap();
                    let u_to_f = |u| u as f32 / 255.0;
                    vertices[i as usize].vertex_color = value.map(u_to_f).into();
                }
                // TODO: How are these different?
                xc3_lib::vertex::DataType::Normal | xc3_lib::vertex::DataType::Unk32 => {
                    vertices[i as usize].normal = read_snorm8x4(&mut reader);
                }
                xc3_lib::vertex::DataType::Tangent => {
                    vertices[i as usize].tangent = read_snorm8x4(&mut reader);
                }
                xc3_lib::vertex::DataType::Uv1 => {
                    let value: [f32; 2] = reader.read_le().unwrap();
                    vertices[i as usize].uv1 = vec4(value[0], value[1], 0.0, 0.0);
                }
                _ => {
                    // Just skip unsupported attributes for now.
                    reader.seek(SeekFrom::Current(a.data_size as i64)).unwrap();
                }
            }
        }
    }
}

fn read_unorm8x4(reader: &mut Cursor<&[u8]>) -> Vec4 {
    let value: [u8; 4] = reader.read_le().unwrap();
    value.map(|u| u as f32 / 255.0).into()
}

fn read_snorm8x4(reader: &mut Cursor<&[u8]>) -> Vec4 {
    let value: [i8; 4] = reader.read_le().unwrap();
    value.map(|i| i as f32 / 255.0).into()
}

fn assign_animation_buffer_attributes(
    vertices: &mut [Vertex],
    model_bytes: &[u8],
    descriptor: &VertexBufferDescriptor,
    base_target: &VertexAnimationTarget,
) {
    let mut reader = Cursor::new(model_bytes);

    for i in 0..descriptor.vertex_count as u64 {
        reader
            .seek(SeekFrom::Start(
                base_target.data_offset as u64 + i * base_target.vertex_size as u64,
            ))
            .unwrap();

        // TODO: What are the attributes for these buffers?
        // Values taken from RenderDoc until the attributes can be found.
        let value: [f32; 3] = reader.read_le().unwrap();
        vertices[i as usize].position = value.into();

        // TODO: Does the vertex shader always apply this transform?
        vertices[i as usize].normal = read_unorm8x4(&mut reader) * 2.0 - 1.0;

        // Second position?
        let _unk1: [f32; 3] = reader.read_le().unwrap();

        // TODO: Does the vertex shader always apply this transform?
        vertices[i as usize].tangent = read_unorm8x4(&mut reader) * 2.0 - 1.0;
    }
}

fn base_vertex_target(
    vertex_data: &VertexData,
    vertex_buffer_index: usize,
) -> Option<&VertexAnimationTarget> {
    // TODO: Easier to loop over each descriptor and assign by vertex buffer index?
    let vertex_animation = vertex_data.vertex_animation.as_ref()?;
    vertex_animation
        .descriptors
        .iter()
        .find(|d| d.vertex_buffer_index as usize == vertex_buffer_index)
        .and_then(|d| vertex_animation.targets.get(d.target_start_index as usize))
}

#[cfg(test)]
mod tests {
    use super::*;

    use glam::{vec3, vec4};
    use hexlit::hex;
    use xc3_lib::vertex::{DataType, VertexAttribute};

    // TODO: Test weight buffers.
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

        let mut vertices = vec![Vertex::default(); 2];
        assign_vertex_buffer_attributes(&mut vertices, &data, &descriptor);

        // TODO: Use strict equality for float comparisons?
        assert_eq!(
            vec![
                Vertex {
                    position: vec3(0.10039953, 0.9038166, 0.07162084),
                    weight_index: 0,
                    vertex_color: vec4(0.49803922, 0.0, 1.0, 1.0),
                    normal: vec4(0.12941177, -0.019607844, 0.47843137, 0.0),
                    tangent: vec4(0.47843137, 0.0, -0.12941177, 0.49803922),
                    uv1: vec4(0.75997907, 0.6079358, 0.0, 0.0)
                },
                Vertex {
                    position: vec3(0.14499485, 0.91730505, 0.050502136),
                    weight_index: 0,
                    vertex_color: vec4(0.49803922, 0.0, 1.0, 1.0),
                    normal: vec4(0.38431373, 0.047058824, 0.30980393, 0.0),
                    tangent: vec4(0.30980393, 0.0, -0.38431373, 0.49803922),
                    uv1: vec4(0.79126656, 0.6000591, 0.0, 0.0)
                }
            ],
            vertices
        );
    }
}