use glam::{UVec4, Vec4};
use wgpu::util::DeviceExt;
use xc3_model::vertex::AttributeData;

use crate::{DeviceBufferExt, shader};

pub struct ModelBuffers {
    pub vertex_buffers: Vec<VertexBuffer>,
    pub index_buffers: Vec<IndexBuffer>,
}

pub struct VertexBuffer {
    pub vertex_buffer0: wgpu::Buffer,
    pub vertex_buffer1: wgpu::Buffer,
    pub outline_vertex_buffer0: wgpu::Buffer,
    pub outline_vertex_buffer1: wgpu::Buffer,
    pub vertex_count: u32,
    pub morph_buffers: Option<MorphBuffers>,
}

pub struct MorphBuffers {
    pub vertex_buffer0: wgpu::Buffer,
    pub weights_buffer: wgpu::Buffer,
    pub bind_group0: crate::shader::morph::bind_groups::BindGroup0,
    pub morph_target_controller_indices: Vec<usize>,
}

pub struct IndexBuffer {
    pub index_buffer: wgpu::Buffer,
    pub vertex_index_count: u32,
}

impl ModelBuffers {
    pub fn from_buffers(device: &wgpu::Device, buffers: &xc3_model::vertex::ModelBuffers) -> Self {
        // TODO: How to handle vertex buffers being used with multiple skeletons?
        let vertex_buffers = model_vertex_buffers(device, buffers);
        let index_buffers = model_index_buffers(device, buffers);

        // TODO: Each vertex buffer needs its own transformed matrices?
        Self {
            vertex_buffers,
            index_buffers,
        }
    }
}

fn model_index_buffers(
    device: &wgpu::Device,
    buffer: &xc3_model::vertex::ModelBuffers,
) -> Vec<IndexBuffer> {
    buffer
        .index_buffers
        .iter()
        .map(|buffer| {
            let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("index buffer"),
                contents: bytemuck::cast_slice(&buffer.indices),
                usage: wgpu::BufferUsages::INDEX,
            });

            IndexBuffer {
                index_buffer,
                vertex_index_count: buffer.indices.len() as u32,
            }
        })
        .collect()
}

fn model_vertex_buffers(
    device: &wgpu::Device,
    buffers: &xc3_model::vertex::ModelBuffers,
) -> Vec<VertexBuffer> {
    buffers
        .vertex_buffers
        .iter()
        .map(|buffer| {
            // Convert the attributes back to an interleaved representation for rendering.
            // Unused attributes will use a default value.
            // Using a single vertex representation reduces shader permutations.
            let vertex_count = buffer.vertex_count();
            let mut buffer0_vertices = vec![
                shader::model::VertexInput0 {
                    position: Vec4::ZERO,
                    normal: Vec4::ZERO,
                    tangent: Vec4::ZERO,
                };
                vertex_count
            ];

            let mut buffer1_vertices = vec![
                shader::model::VertexInput1 {
                    vertex_color: Vec4::ONE,
                    weight_index: UVec4::ZERO,
                    tex01: Vec4::ZERO,
                    tex23: Vec4::ZERO,
                    tex45: Vec4::ZERO,
                    tex67: Vec4::ZERO,
                    tex8: Vec4::ZERO,
                };
                vertex_count
            ];

            set_attributes(&mut buffer0_vertices, &mut buffer1_vertices, buffer);

            // Avoid overwriting the existing attributes.
            let mut outline_buffer0_vertices = buffer0_vertices.clone();
            let mut outline_buffer1_vertices = buffer1_vertices.clone();
            if let Some(outline_buffer) = buffer
                .outline_buffer_index
                .and_then(|i| buffers.outline_buffers.get(i))
            {
                set_buffer0_attributes(&mut outline_buffer0_vertices, &outline_buffer.attributes);
                set_buffer1_attributes(&mut outline_buffer1_vertices, &outline_buffer.attributes);
            }

            let vertex_buffer0 = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("vertex buffer 0"),
                contents: bytemuck::cast_slice(&buffer0_vertices),
                usage: wgpu::BufferUsages::VERTEX
                    | wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::COPY_SRC,
            });

            let vertex_buffer1 = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("vertex buffer 1"),
                contents: bytemuck::cast_slice(&buffer1_vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });

            let outline_vertex_buffer0 =
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("outline vertex buffer 0"),
                    contents: bytemuck::cast_slice(&outline_buffer0_vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                });

            let outline_vertex_buffer1 =
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("outline vertex buffer 1"),
                    contents: bytemuck::cast_slice(&outline_buffer1_vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                });

            // TODO: morph targets?
            let morph_buffers = if !buffer.morph_targets.is_empty() {
                Some(morph_buffers(device, buffer0_vertices, buffer))
            } else {
                None
            };

            VertexBuffer {
                vertex_buffer0,
                vertex_buffer1,
                outline_vertex_buffer0,
                outline_vertex_buffer1,
                morph_buffers,
                vertex_count: vertex_count as u32,
            }
        })
        .collect()
}

fn morph_buffers(
    device: &wgpu::Device,
    buffer0_vertices: Vec<shader::model::VertexInput0>,
    buffer: &xc3_model::vertex::VertexBuffer,
) -> MorphBuffers {
    // Initialize to the unmodified vertices.
    let morph_vertex_buffer0 = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("vertex buffer 0 morph"),
        contents: bytemuck::cast_slice(&buffer0_vertices),
        usage: wgpu::BufferUsages::VERTEX
            | wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::COPY_DST,
    });

    // TODO: Optimize this?
    let deltas: Vec<_> = buffer
        .morph_targets
        .iter()
        .flat_map(|target| {
            // Convert from a sparse to a dense representation.
            let vertex_count = buffer.vertex_count();
            let mut position_deltas = vec![Vec4::ZERO; vertex_count];
            let mut normal_deltas = vec![Vec4::ZERO; vertex_count];
            let mut tangent_deltas = vec![Vec4::ZERO; vertex_count];

            for (i, vertex_index) in target.vertex_indices.iter().enumerate() {
                let vertex_index = *vertex_index as usize;

                position_deltas[vertex_index] = target.position_deltas[i].extend(0.0);
                normal_deltas[vertex_index] =
                    target.normals[i] - buffer0_vertices[vertex_index].normal;
                tangent_deltas[vertex_index] =
                    target.tangents[i] - buffer0_vertices[vertex_index].tangent;
            }

            position_deltas
                .iter()
                .zip(normal_deltas.iter())
                .zip(tangent_deltas.iter())
                .map(move |((p, n), t)| crate::shader::morph::MorphVertexDelta {
                    position_delta: *p,
                    normal_delta: *n,
                    tangent_delta: *t,
                })
                .collect::<Vec<_>>()
        })
        .collect();

    let morph_deltas = device.create_storage_buffer("morph deltas", &deltas);

    let weights = vec![0.0f32; buffer.morph_targets.len()];
    let morph_weights = device.create_storage_buffer("morph weights", &weights);

    let bind_group0 = crate::shader::morph::bind_groups::BindGroup0::from_bindings(
        device,
        crate::shader::morph::bind_groups::BindGroupLayout0 {
            vertices: morph_vertex_buffer0.as_entire_buffer_binding(),
            morph_deltas: morph_deltas.as_entire_buffer_binding(),
            morph_weights: morph_weights.as_entire_buffer_binding(),
        },
    );

    let morph_target_controller_indices = buffer
        .morph_targets
        .iter()
        .map(|t| t.morph_controller_index)
        .collect();

    MorphBuffers {
        vertex_buffer0: morph_vertex_buffer0,
        weights_buffer: morph_weights,
        morph_target_controller_indices,
        bind_group0,
    }
}

fn set_attributes(
    buffer0_vertices: &mut [shader::model::VertexInput0],
    buffer1_vertices: &mut [shader::model::VertexInput1],
    buffer: &xc3_model::vertex::VertexBuffer,
) {
    set_buffer0_attributes(buffer0_vertices, &buffer.attributes);
    set_buffer0_attributes(buffer0_vertices, &buffer.morph_blend_target);
    set_buffer1_attributes(buffer1_vertices, &buffer.attributes);
}

fn set_buffer0_attributes(verts: &mut [shader::model::VertexInput0], attributes: &[AttributeData]) {
    for attribute in attributes {
        match attribute {
            AttributeData::Position(vals) => {
                set_attribute0(verts, vals, |v, t| v.position = t.extend(1.0))
            }
            AttributeData::Normal(vals) => set_attribute0(verts, vals, |v, t| v.normal = t),
            AttributeData::Normal2(vals) => set_attribute0(verts, vals, |v, t| v.normal = t),
            AttributeData::Tangent(vals) => set_attribute0(verts, vals, |v, t| v.tangent = t),
            // Morph blend target attributes
            AttributeData::Position2(vals) => {
                set_attribute0(verts, vals, |v, t| v.position = t.extend(1.0))
            }
            AttributeData::Normal4(vals) => set_attribute0(verts, vals, |v, t| v.normal = t),
            AttributeData::Tangent2(vals) => set_attribute0(verts, vals, |v, t| v.tangent = t),
            _ => (),
        }
    }
}

fn set_buffer1_attributes(verts: &mut [shader::model::VertexInput1], attributes: &[AttributeData]) {
    for attribute in attributes {
        match attribute {
            AttributeData::TexCoord0(vals) => set_attribute1(verts, vals, |v, t| {
                v.tex01.x = t.x;
                v.tex01.y = t.y;
            }),
            AttributeData::TexCoord1(vals) => set_attribute1(verts, vals, |v, t| {
                v.tex01.z = t.x;
                v.tex01.w = t.y;
            }),
            AttributeData::TexCoord2(vals) => set_attribute1(verts, vals, |v, t| {
                v.tex23.z = t.x;
                v.tex23.w = t.y;
            }),
            AttributeData::TexCoord3(vals) => set_attribute1(verts, vals, |v, t| {
                v.tex23.z = t.x;
                v.tex23.w = t.y;
            }),
            AttributeData::TexCoord4(vals) => set_attribute1(verts, vals, |v, t| {
                v.tex45.x = t.x;
                v.tex45.y = t.y;
            }),
            AttributeData::TexCoord5(vals) => set_attribute1(verts, vals, |v, t| {
                v.tex45.z = t.x;
                v.tex45.w = t.y;
            }),
            AttributeData::TexCoord6(vals) => set_attribute1(verts, vals, |v, t| {
                v.tex67.x = t.x;
                v.tex67.y = t.y;
            }),
            AttributeData::TexCoord7(vals) => set_attribute1(verts, vals, |v, t| {
                v.tex67.z = t.x;
                v.tex67.w = t.y;
            }),
            AttributeData::TexCoord8(vals) => set_attribute1(verts, vals, |v, t| {
                v.tex8.z = t.x;
                v.tex8.w = t.y;
            }),
            AttributeData::VertexColor(vals) => {
                set_attribute1(verts, vals, |v, t| v.vertex_color = t)
            }
            AttributeData::WeightIndex(vals) => {
                // TODO: What does the second index component do?
                set_attribute1(verts, vals, |v, t| {
                    v.weight_index.x = t[0] as u32;
                    v.weight_index.y = t[1] as u32;
                })
            }
            _ => (),
        }
    }
}

fn set_attribute0<T, F>(vertices: &mut [shader::model::VertexInput0], values: &[T], assign: F)
where
    T: Copy,
    F: Fn(&mut shader::model::VertexInput0, T),
{
    for (vertex, value) in vertices.iter_mut().zip(values) {
        assign(vertex, *value);
    }
}

fn set_attribute1<T, F>(vertices: &mut [shader::model::VertexInput1], values: &[T], assign: F)
where
    T: Copy,
    F: Fn(&mut shader::model::VertexInput1, T),
{
    for (vertex, value) in vertices.iter_mut().zip(values) {
        assign(vertex, *value);
    }
}
