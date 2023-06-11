use std::io::Cursor;

use glam::vec4;
use wgpu::util::DeviceExt;
use xc3_lib::{
    mibl::Mibl,
    model::ModelData,
    msrd::Msrd,
    mxmd::{Mxmd, ShaderUnkType},
};
use xc3_model::vertex::{read_indices, read_vertices};

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
    // TODO: Separate render pass for the transparent stuff in Unk7.
    // Only write to g0 and use the out_attr0 assignments.
    // Create the necessary pipeline with blending for each material.
    // TODO: How to handle Unk1?
    pub fn draw<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>, pass: ShaderUnkType) {
        for mesh in &self.meshes {
            // TODO: How does LOD selection work in game?
            let material = &self.materials[mesh.material_index];

            // TODO: Why are there materials with no textures?
            // TODO: Group these into passes with separate shaders for each pass?
            // TODO: The main pass is shared with outline, ope, and zpre?
            // TODO: How to handle transparency?
            if material.unk_type == pass
            // && material.texture_count > 0
            && !material.name.ends_with("_outline")
            && !material.name.ends_with("_ope")
            && !material.name.ends_with("_zpre")
            {
                // TODO: How to make sure the pipeline outputs match the render pass?
                render_pass.set_pipeline(&material.pipeline);

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
    shader_database: &xc3_shader::gbuffer_database::GBufferDatabase,
) -> Model {
    let model_data = msrd.extract_model_data();

    // TODO: Avoid unwrap.
    // Load cached textures
    let cached_textures = load_cached_textures(msrd);

    let vertex_buffers = vertex_buffers(device, &model_data);
    let index_buffers = index_buffers(device, &model_data);

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

fn load_cached_textures(msrd: &Msrd) -> Vec<(String, Mibl)> {
    let texture_data = msrd.extract_texture_data();

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

fn index_buffers(device: &wgpu::Device, model_data: &ModelData) -> Vec<IndexData> {
    model_data
        .index_buffers
        .iter()
        .map(|info| {
            let indices = read_indices(model_data, info);

            let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("index buffer"),
                contents: bytemuck::cast_slice(&indices),
                usage: wgpu::BufferUsages::INDEX,
            });

            IndexData {
                index_buffer,
                vertex_index_count: indices.len() as u32,
            }
        })
        .collect()
}

fn vertex_buffers(device: &wgpu::Device, model_data: &ModelData) -> Vec<VertexData> {
    model_data
        .vertex_buffers
        .iter()
        .enumerate()
        .map(|(i, info)| {
            let vertices = read_vertices(info, i, model_data);

            // Start with default values for each attribute.
            // Convert the buffers to a standardized format.
            // This still tests the vertex buffer layouts and avoids needing multiple shaders.
            let buffer_vertices: Vec<_> = vertices
                .into_iter()
                .map(|v| shader::model::VertexInput {
                    position: v.position,
                    weight_index: v.weight_index,
                    vertex_color: v.vertex_color,
                    normal: v.normal,
                    tangent: v.tangent,
                    uv1: vec4(v.uv1.x, v.uv1.y, 0.0, 0.0),
                })
                .collect();

            let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("vertex buffer"),
                contents: bytemuck::cast_slice(&buffer_vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });

            VertexData { vertex_buffer }
        })
        .collect()
}
