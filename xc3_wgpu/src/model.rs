use std::{
    collections::HashMap,
    io::{Read, Seek},
    path::Path,
};

use glam::vec4;
use wgpu::util::DeviceExt;
use xc3_lib::{
    map::MapModelData,
    mibl::Mibl,
    msmd::{Msmd, StreamEntry},
    msrd::Msrd,
    mxmd::{Mxmd, ShaderUnkType},
    vertex::VertexData,
};
use xc3_model::vertex::{read_indices, read_vertices};

use crate::{
    material::{materials, Material},
    pipeline::{ModelPipelineData, PipelineKey},
    shader,
    texture::create_texture,
};

// Organize the model data to ensure shared resources are created only once.
pub struct ModelGroup {
    pub models: Vec<Model>,
    materials: Vec<Material>,
    // Cache pipelines by their creation parameters.
    pipelines: HashMap<PipelineKey, wgpu::RenderPipeline>,
}

pub struct Model {
    pub meshes: Vec<Mesh>,
    vertex_buffers: Vec<VertexBuffer>,
    index_buffers: Vec<IndexBuffer>,
    // Use a collection to support "instancing" for map props.
    pub instances: Vec<ModelInstance>,
}

pub struct ModelInstance {
    per_model: crate::shader::model::bind_groups::BindGroup3,
}

#[derive(Debug)]
pub struct Mesh {
    vertex_buffer_index: usize,
    index_buffer_index: usize,
    material_index: usize,
    // TODO: lod?
}

struct VertexBuffer {
    vertex_buffer: wgpu::Buffer,
}

struct IndexBuffer {
    index_buffer: wgpu::Buffer,
    vertex_index_count: u32,
}

impl ModelGroup {
    // TODO: How to handle other unk types?
    pub fn draw<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>, pass: ShaderUnkType) {
        // TODO: Is this the best way to "instance" models?
        for model in &self.models {
            for instance in &model.instances {
                instance.per_model.set(render_pass);

                // Each "instance" repeats the same meshes with different transforms.
                for mesh in &model.meshes {
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
                        let pipeline = &self.pipelines[&material.pipeline_key];
                        render_pass.set_pipeline(pipeline);

                        material.bind_group1.set(render_pass);
                        material.bind_group2.set(render_pass);

                        self.draw_mesh(model, mesh, render_pass);
                    }
                }
            }
        }
    }

    fn draw_mesh<'a>(
        &'a self,
        model: &'a Model,
        mesh: &Mesh,
        render_pass: &mut wgpu::RenderPass<'a>,
    ) {
        let vertex_data = &model.vertex_buffers[mesh.vertex_buffer_index];
        render_pass.set_vertex_buffer(0, vertex_data.vertex_buffer.slice(..));

        // TODO: Are all indices u16?
        // TODO: Why do maps not always refer to a valid index buffer?
        let index_data = &model.index_buffers[mesh.index_buffer_index];
        // let index_data = &self.index_buffers[mesh.index_buffer_index];
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
) -> ModelGroup {
    // Compile shaders only once to improve loading times.
    let pipeline_data = ModelPipelineData::new(device);

    // TODO: Avoid unwrap.
    let model_data = msrd.extract_vertex_data();

    // "chr/en/file.wismt" -> "chr/tex/nx/m"
    // TODO: Don't assume model_path is in the chr/ch or chr/en folders.
    let chr_folder = Path::new(model_path).parent().unwrap().parent().unwrap();
    let m_tex_folder = chr_folder.join("tex").join("nx").join("m");
    let h_tex_folder = chr_folder.join("tex").join("nx").join("h");

    let textures = load_textures(device, queue, msrd, mxmd, &m_tex_folder, &h_tex_folder);

    let vertex_buffers = vertex_buffers(device, &model_data);
    let index_buffers = index_buffers(device, &model_data);

    let (materials, pipelines) = materials(
        device,
        queue,
        &pipeline_data,
        &mxmd.materials,
        &textures,
        model_path,
        shader_database,
    );

    let meshes = meshes(&mxmd.models);

    let per_model = per_model_bind_group(device, glam::Mat4::IDENTITY);

    ModelGroup {
        materials,
        pipelines,
        models: vec![Model {
            vertex_buffers,
            index_buffers,
            meshes,
            instances: vec![ModelInstance { per_model }],
        }],
    }
}

// TODO: Separate module for this?
// TODO: Better way to pass the wismda file?
pub fn load_map<R: Read + Seek>(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    msmd: &Msmd,
    wismda: &mut R,
    model_path: &str,
    shader_database: &xc3_shader::gbuffer_database::GBufferDatabase,
) -> Vec<ModelGroup> {
    // Compile shaders only once to improve loading times.
    let pipeline_data = ModelPipelineData::new(device);

    // TODO: Are the msmd textures shared with all models?
    // TODO: Load high resolution textures?
    let textures: Vec<_> = msmd
        .textures
        .iter()
        .map(|texture| texture.mid.extract(wismda))
        .collect();

    // TODO: Better way to combine models?
    let mut combined_models = Vec::new();
    for map_model in &msmd.map_models {
        let model = load_map_model_group(
            device,
            queue,
            wismda,
            map_model,
            &msmd.map_vertex_data,
            &textures,
            model_path,
            shader_database,
            &pipeline_data,
        );
        combined_models.push(model);
    }

    for prop_model in &msmd.prop_models {
        let model = load_prop_model_group(
            device,
            queue,
            wismda,
            prop_model,
            &msmd.prop_vertex_data,
            &textures,
            model_path,
            shader_database,
            &pipeline_data,
        );
        combined_models.push(model);
    }

    combined_models
}

fn load_prop_model_group<R: Read + Seek>(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    wismda: &mut R,
    prop_model: &xc3_lib::msmd::PropModel,
    prop_vertex_data: &[StreamEntry<VertexData>],
    mibl_textures: &[Mibl],
    model_path: &str,
    shader_database: &xc3_shader::gbuffer_database::GBufferDatabase,
    pipeline_data: &ModelPipelineData,
) -> ModelGroup {
    let prop_model_data = prop_model.entry.extract(wismda);

    // Get the textures referenced by the materials in this model.
    let textures = load_map_textures(device, queue, &prop_model_data.textures, mibl_textures);

    // TODO: cached textures?
    let (materials, pipelines) = materials(
        device,
        queue,
        pipeline_data,
        &prop_model_data.materials,
        &textures,
        model_path,
        shader_database,
    );

    // Load the base LOD for each prop model.
    // TODO: Also cache vertex and index buffer creation?
    let models = prop_model_data
        .lods
        .props
        .iter()
        .enumerate()
        .map(|(i, prop_lod)| {
            let base_lod_index = prop_lod.base_lod_index as usize;
            let vertex_data_index = prop_model_data.vertex_data_indices[base_lod_index];

            let vertex_data = prop_vertex_data[vertex_data_index as usize].extract(wismda);

            let vertex_buffers = vertex_buffers(device, &vertex_data);
            let index_buffers = index_buffers(device, &vertex_data);

            let meshes: Vec<_> = prop_model_data.models.models[base_lod_index]
                .meshes
                .iter()
                .map(create_mesh)
                .collect();

            // Find all the instances referencing this prop.
            // TODO: Will all props be referenced?
            let instances = prop_model_data
                .lods
                .instances
                .iter()
                .filter(|instance| instance.prop_index as usize == i)
                .map(|instance| {
                    let transform = glam::Mat4::from_cols_array_2d(&instance.transform);
                    let per_model = per_model_bind_group(device, transform);

                    ModelInstance { per_model }
                })
                .collect();

            Model {
                vertex_buffers,
                index_buffers,
                meshes,
                instances,
            }
        })
        .collect();

    ModelGroup {
        materials,
        pipelines,
        models,
    }
}

fn load_map_model_group<R: Read + Seek>(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    wismda: &mut R,
    map_model: &xc3_lib::msmd::MapModel,
    map_vertex_data: &[xc3_lib::msmd::StreamEntry<VertexData>],
    mibl_textures: &[Mibl],
    model_path: &str,
    shader_database: &xc3_shader::gbuffer_database::GBufferDatabase,
    pipeline_data: &ModelPipelineData,
) -> ModelGroup {
    let map_model_data: MapModelData = map_model.entry.extract(wismda);

    // Get the textures referenced by the materials in this model.
    let textures = load_map_textures(device, queue, &map_model_data.textures, mibl_textures);

    let (materials, pipelines) = materials(
        device,
        queue,
        pipeline_data,
        &map_model_data.materials,
        &textures,
        model_path,
        shader_database,
    );

    // TODO: The mapping.indices and models.models always have the same length?
    // TODO: the mapping indices are in the range [0, 2*groups - 1]?
    // TODO: Some mapping sections assign to twice as many groups as actual groups?
    let models = map_model_data
        .mapping
        .groups
        .iter()
        .enumerate()
        .map(|(group_index, group)| {
            // TODO: Load all groups?
            let vertex_data = map_vertex_data[group.vertex_data_index as usize].extract(wismda);

            let vertex_buffers = vertex_buffers(device, &vertex_data);
            let index_buffers = index_buffers(device, &vertex_data);

            // TODO: Select meshes based on the grouping?
            // TODO: Does the list of indices in the grouping assign items here to groups?
            // TODO: Should we be creating multiple models in this step?
            let meshes = map_model_data
                .models
                .models
                .iter()
                .zip(map_model_data.mapping.indices.iter())
                .find_map(|(model, index)| {
                    if *index as usize == group_index {
                        Some(model)
                    } else {
                        None
                    }
                })
                .unwrap()
                .meshes
                .iter()
                .map(create_mesh)
                .collect();

            let per_model = per_model_bind_group(device, glam::Mat4::IDENTITY);

            Model {
                vertex_buffers,
                index_buffers,
                meshes,
                instances: vec![ModelInstance { per_model }],
            }
        })
        .collect();

    ModelGroup {
        materials,
        pipelines,
        models,
    }
}

fn load_map_textures(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    textures: &[xc3_lib::map::Texture],
    mibl_textures: &[Mibl],
) -> Vec<wgpu::TextureView> {
    textures
        .iter()
        .map(|item| {
            // TODO: Handle texture index being -1?
            let mibl = &mibl_textures[item.texture_index.max(0) as usize];
            create_texture(device, queue, mibl).create_view(&wgpu::TextureViewDescriptor::default())
        })
        .collect()
}

fn meshes(models: &xc3_lib::mxmd::Models) -> Vec<Mesh> {
    models
        .models
        .iter()
        .flat_map(|model| model.meshes.iter().map(create_mesh))
        .collect()
}

fn create_mesh(mesh: &xc3_lib::mxmd::Mesh) -> Mesh {
    Mesh {
        vertex_buffer_index: mesh.vertex_buffer_index as usize,
        index_buffer_index: mesh.index_buffer_index as usize,
        material_index: mesh.material_index as usize,
    }
}

fn index_buffers(device: &wgpu::Device, vertex_data: &VertexData) -> Vec<IndexBuffer> {
    vertex_data
        .index_buffers
        .iter()
        .map(|info| {
            let indices = read_indices(vertex_data, info);

            let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("index buffer"),
                contents: bytemuck::cast_slice(&indices),
                usage: wgpu::BufferUsages::INDEX,
            });

            IndexBuffer {
                index_buffer,
                vertex_index_count: indices.len() as u32,
            }
        })
        .collect()
}

fn vertex_buffers(device: &wgpu::Device, vertex_data: &VertexData) -> Vec<VertexBuffer> {
    vertex_data
        .vertex_buffers
        .iter()
        .enumerate()
        .map(|(i, info)| {
            let vertices = read_vertices(info, i, vertex_data);

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

            VertexBuffer { vertex_buffer }
        })
        .collect()
}

fn load_textures(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    msrd: &Msrd,
    mxmd: &Mxmd,
    m_tex_folder: &Path,
    h_tex_folder: &Path,
) -> Vec<wgpu::TextureView> {
    let mibls = xc3_model::texture::load_textures(msrd, mxmd, m_tex_folder, h_tex_folder);
    mibls
        .iter()
        .map(|mibl| {
            create_texture(device, queue, mibl).create_view(&wgpu::TextureViewDescriptor::default())
        })
        .collect()
}

fn per_model_bind_group(
    device: &wgpu::Device,
    transform: glam::Mat4,
) -> shader::model::bind_groups::BindGroup3 {
    let per_model_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("per model buffer"),
        contents: bytemuck::cast_slice(&[crate::shader::model::PerModel { matrix: transform }]),
        usage: wgpu::BufferUsages::UNIFORM,
    });

    crate::shader::model::bind_groups::BindGroup3::from_bindings(
        device,
        crate::shader::model::bind_groups::BindGroupLayout3 {
            per_model: per_model_buffer.as_entire_buffer_binding(),
        },
    )
}
