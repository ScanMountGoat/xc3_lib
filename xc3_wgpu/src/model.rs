use std::io::{Cursor, Seek, SeekFrom};

use binrw::BinReaderExt;
use glam::{vec4, Vec3, Vec4};
use wgpu::util::DeviceExt;
use xc3_lib::{
    mibl::Mibl,
    model::{ModelData, VertexAnimationTarget},
    msrd::Msrd,
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

            // TODO: Why are there materials with no textures?
            // TODO: Group these into passes with separate shaders for each pass?
            // TODO: The main pass is shared with outline, ope, and zpre?
            // TODO: How to handle transparency?
            if material.unk_type == xc3_lib::mxmd::ShaderUnkType::Unk0
                && material.texture_count > 0
                && !material.name.ends_with("_outline")
                && !material.name.ends_with("_ope")
                && !material.name.ends_with("_zpre")
            {
                material.bind_group1.set(render_pass);
                material.bind_group2.set(render_pass);

                self.draw_mesh(mesh, render_pass);
            }
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
    let decompressed_streams: Vec<_> = msrd
        .streams
        .iter()
        .map(|stream| stream.xbc1.decompress().unwrap())
        .collect();

    let item = &msrd.stream_entries[msrd.model_entry_index as usize];
    let stream = &decompressed_streams[item.stream_index as usize];
    let model_bytes = &stream[item.offset as usize..item.offset as usize + item.size as usize];

    // TODO: Avoid unwrap.
    // Load cached textures
    let cached_textures = load_cached_textures(msrd, &decompressed_streams);

    let model_data = ModelData::read(&mut Cursor::new(&model_bytes)).unwrap();

    let vertex_buffers = vertex_buffers(device, &model_data, model_bytes);
    let index_buffers = index_buffers(device, &model_data, model_bytes);

    let materials = materials(
        device,
        queue,
        mxmd,
        &cached_textures,
        model_path,
        shader_database,
    );

    let meshes = mxmd
        .mesh
        .items
        .elements
        .iter()
        .flat_map(|item| {
            item.sub_items.iter().map(|sub_item| Mesh {
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

fn load_cached_textures(msrd: &Msrd, decompressed_streams: &[Vec<u8>]) -> Vec<(String, Mibl)> {
    let item = &msrd.stream_entries[msrd.texture_entry_index as usize];
    let stream = &decompressed_streams[item.stream_index as usize];
    let texture_data = &stream[item.offset as usize..item.offset as usize + item.size as usize];

    msrd.texture_name_table
        .as_ref()
        .unwrap()
        .textures
        .iter()
        .map(|info| {
            let data =
                &texture_data[info.offset as usize..info.offset as usize + info.size as usize];
            (
                info.name.clone(),
                Mibl::read(&mut Cursor::new(&data)).unwrap(),
            )
        })
        .collect()
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
        .enumerate()
        .map(|(i, info)| {
            // TODO: Dedicated vertex accessor module with tests?
            // Convert the buffers to a standardized format.
            // This still tests the vertex buffer layouts and avoids needing multiple shaders.

            // Start with default values for each attribute.
            let mut vertices = vec![
                shader::model::VertexInput {
                    position: Vec3::ZERO,
                    weight_index: 0,
                    vertex_color: Vec4::ZERO,
                    normal: Vec4::ZERO,
                    tangent: Vec4::ZERO,
                    uv1: Vec4::ZERO
                };
                info.vertex_count as usize
            ];

            // The game renders attributes from both the vertex and optional animation buffer.
            // Merge attributes into a single buffer to allow using the same shader.
            // TODO: Which buffer takes priority?
            assign_vertex_buffer_attributes(&mut vertices, model_data, model_bytes, info);

            if let Some(base_target) = base_vertex_target(model_data, i) {
                assign_animation_buffer_attributes(
                    &mut vertices,
                    model_data,
                    model_bytes,
                    info,
                    base_target,
                );
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

fn assign_vertex_buffer_attributes(
    vertices: &mut [shader::model::VertexInput],
    model_data: &ModelData,
    model_bytes: &[u8],
    info: &xc3_lib::model::VertexBuffer,
) {
    let mut reader = Cursor::new(&model_bytes[model_data.data_base_offset as usize..]);

    for i in 0..info.vertex_count as u64 {
        reader
            .seek(SeekFrom::Start(
                info.data_offset as u64 + i * info.vertex_size as u64,
            ))
            .unwrap();

        // TODO: How to handle missing attributes.
        // TODO: Document conversion formulas to float in xc3_lib.
        // TODO: Is switching for each vertex the base way to do this?
        for a in &info.attributes {
            match a.data_type {
                xc3_lib::model::DataType::Position => {
                    let value: [f32; 3] = reader.read_le().unwrap();
                    vertices[i as usize].position = value.into();
                }
                xc3_lib::model::DataType::VertexColor => {
                    let value: [u8; 4] = reader.read_le().unwrap();
                    let u_to_f = |u| u as f32 / 255.0;
                    vertices[i as usize].vertex_color = value.map(u_to_f).into();
                }
                // TODO: How are these different?
                xc3_lib::model::DataType::Normal | xc3_lib::model::DataType::Unk32 => {
                    vertices[i as usize].normal = read_snorm8x4(&mut reader);
                }
                xc3_lib::model::DataType::Tangent => {
                    vertices[i as usize].tangent = read_snorm8x4(&mut reader);
                }
                xc3_lib::model::DataType::Uv1 => {
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
    vertices: &mut [shader::model::VertexInput],
    model_data: &ModelData,
    model_bytes: &[u8],
    info: &xc3_lib::model::VertexBuffer,
    base_target: &VertexAnimationTarget,
) {
    let mut reader = Cursor::new(&model_bytes[model_data.data_base_offset as usize..]);

    for i in 0..info.vertex_count as u64 {
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
    model_data: &ModelData,
    vertex_buffer_index: usize,
) -> Option<&VertexAnimationTarget> {
    // TODO: Easier to loop over each descriptor and assign by vertex buffer index?
    let vertex_animation = model_data.vertex_animation.as_ref()?;
    vertex_animation
        .descriptors
        .iter()
        .find(|d| d.vertex_buffer_index as usize == vertex_buffer_index)
        .and_then(|d| vertex_animation.targets.get(d.target_start_index as usize))
}
