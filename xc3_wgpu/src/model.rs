use std::io::{Cursor, Seek, SeekFrom};

use binrw::BinReaderExt;
use glam::{vec4, Vec3, Vec4};
use wgpu::util::DeviceExt;
use xc3_lib::{
    model::ModelData,
    msrd::{DataItemType, Msrd},
    mxmd::Mxmd,
};

use crate::{
    material::{materials, Material},
    shader,
};

pub struct Model {
    meshes: Vec<Mesh>,
    materials: Vec<Material>,
    vertex_buffers: Vec<VertexData>,
    index_buffers: Vec<IndexData>,
}

struct Mesh {
    vertex_buffer_index: usize,
    index_buffer_index: usize,
    material_index: usize,
    // TODO: How does this work?
    lod: usize,
}

struct VertexData {
    vertex_buffer: wgpu::Buffer,
}

struct IndexData {
    index_buffer: wgpu::Buffer,
    vertex_index_count: u32,
}

impl Model {
    pub fn draw<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        for mesh in &self.meshes {
            // TODO: How does LOD selection work in game?
            let material = &self.materials[mesh.material_index];

            material.bind_group1.set(render_pass);
            material.bind_group2.set(render_pass);

            self.draw_mesh(mesh, render_pass);
        }
    }

    fn draw_mesh<'a>(&'a self, mesh: &Mesh, render_pass: &mut wgpu::RenderPass<'a>) {
        let vertex_data = &self.vertex_buffers[mesh.vertex_buffer_index];
        render_pass.set_vertex_buffer(0, vertex_data.vertex_buffer.slice(..));

        // TODO: Are all indices u16?
        let index_data = &self.index_buffers[mesh.index_buffer_index];
        render_pass.set_index_buffer(index_data.index_buffer.slice(..), wgpu::IndexFormat::Uint16);

        render_pass.draw_indexed(0..index_data.vertex_index_count, 0, 0..1);
    }
}

pub fn load_model(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    msrd: &Msrd,
    mxmd: &Mxmd,
    model_path: &str,
    shader_database: &[xc3_shader::gbuffer_database::File],
) -> Model {
    // TODO: add this to xc3_lib?
    // TODO: Only decompress the stream that's needed?
    let toc_streams: Vec<_> = msrd
        .tocs
        .iter()
        .map(|toc| toc.xbc1.decompress().unwrap())
        .collect();

    let model_bytes = msrd
        .data_items
        .iter()
        .find_map(|item| match &item.item_type {
            DataItemType::Model => {
                let stream = &toc_streams[item.toc_index as usize];
                let data = &stream[item.offset as usize..item.offset as usize + item.size as usize];
                Some(data)
            }
            _ => None,
        })
        .unwrap();

    let model_data = ModelData::read(&mut Cursor::new(&model_bytes)).unwrap();

    let vertex_buffers = vertex_buffers(device, &model_data, model_bytes);
    let index_buffers = index_buffers(device, &model_data, model_bytes);

    let materials = materials(device, queue, mxmd, model_path, shader_database);

    let meshes = mxmd
        .mesh
        .items
        .elements
        .iter()
        .flat_map(|item| {
            item.sub_items.elements.iter().map(|sub_item| Mesh {
                vertex_buffer_index: sub_item.vertex_buffer_index as usize,
                index_buffer_index: sub_item.index_buffer_index as usize,
                material_index: sub_item.material_index as usize,
                lod: sub_item.lod as usize,
            })
        })
        .collect();

    Model {
        meshes,
        materials,
        vertex_buffers,
        index_buffers,
    }
}

fn index_buffers(
    device: &wgpu::Device,
    model_data: &ModelData,
    model_bytes: &[u8],
) -> Vec<IndexData> {
    model_data
        .index_buffers
        .iter()
        .map(|info| {
            // TODO: Are all index buffers using u16 for indices?
            let mut reader = Cursor::new(&model_bytes[model_data.data_base_offset as usize..]);
            reader
                .seek(SeekFrom::Start(info.data_offset as u64))
                .unwrap();

            let mut indices = Vec::new();
            let vertex_index_count = info.index_count;
            for _ in 0..vertex_index_count {
                let index: u16 = reader.read_le().unwrap();
                indices.push(index);
            }

            let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("index buffer"),
                contents: bytemuck::cast_slice(&indices),
                usage: wgpu::BufferUsages::INDEX,
            });

            IndexData {
                index_buffer,
                vertex_index_count,
            }
        })
        .collect()
}

fn vertex_buffers(
    device: &wgpu::Device,
    model_data: &ModelData,
    model_bytes: &[u8],
) -> Vec<VertexData> {
    // TODO: Add some of this logic to xc3_lib?
    model_data
        .vertex_buffers
        .iter()
        .map(|info| {
            // TODO: Dedicated vertex accessor module with tests?
            // Convert the buffers to a standardized format.
            // This still tests the vertex buffer layouts and avoids needing multiple shaders.
            let mut reader = Cursor::new(&model_bytes[model_data.data_base_offset as usize..]);

            let mut vertices = Vec::new();
            for i in 0..info.vertex_count as u64 {
                reader
                    .seek(SeekFrom::Start(
                        info.data_offset as u64 + i * info.vertex_size as u64,
                    ))
                    .unwrap();

                // TODO: How to handle missing attributes.
                let mut position = Vec3::ZERO;
                let weight_index = 0;
                let mut vertex_color = Vec4::ZERO;
                let mut normal = Vec4::ZERO;
                let mut tangent = Vec4::ZERO;
                let mut uv1 = Vec4::ZERO;

                // TODO: Document conversion formulas to float in xc3_lib.
                // TODO: Is switching for each vertex the base way to do this?
                for a in &info.attributes {
                    match a.data_type {
                        xc3_lib::model::DataType::Position => {
                            let value: [f32; 3] = reader.read_le().unwrap();
                            position = value.into();
                        }
                        xc3_lib::model::DataType::VertexColor => {
                            let value: [u8; 4] = reader.read_le().unwrap();
                            let u_to_f = |u| u as f32 / 255.0;
                            vertex_color = value.map(u_to_f).into();
                        }
                        // TODO: How are these different?
                        xc3_lib::model::DataType::Normal | xc3_lib::model::DataType::Unk32 => {
                            let value: [i8; 4] = reader.read_le().unwrap();
                            let i_to_f = |i| i as f32 / 255.0;
                            normal = value.map(i_to_f).into();
                        }
                        xc3_lib::model::DataType::Tangent => {
                            let value: [i8; 4] = reader.read_le().unwrap();
                            let i_to_f = |i| i as f32 / 255.0;
                            tangent = value.map(i_to_f).into();
                        }
                        xc3_lib::model::DataType::Uv1 => {
                            let value: [f32; 2] = reader.read_le().unwrap();
                            uv1 = vec4(value[0], value[1], 0.0, 0.0);
                        }
                        _ => {
                            // Just skip unsupported attributes for now.
                            reader.seek(SeekFrom::Current(a.data_size as i64)).unwrap();
                        }
                    }
                }

                // TODO: The last vertex buffer is just for weight data?
                let vertex = shader::model::VertexInput {
                    position,
                    weight_index,
                    uv1,
                    vertex_color,
                    normal,
                    tangent,
                };
                vertices.push(vertex);
            }

            let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("vertex buffer"),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });

            VertexData { vertex_buffer }
        })
        .collect()
}
