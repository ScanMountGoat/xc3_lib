use std::collections::HashMap;

use glam::{uvec4, vec4, Mat4, Vec3, Vec4};
use log::info;
use wgpu::util::DeviceExt;
use xc3_model::{skinning::bone_indices_weights, vertex::AttributeData};

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
    per_group: crate::shader::model::bind_groups::BindGroup1,
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
    pub fn draw<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>, is_transparent: bool) {
        self.per_group.set(render_pass);

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
                    // TODO: Characters render as solid white?
                    if (is_transparent != material.pipeline_key.write_to_all_outputs)
                        && !material.name.ends_with("_outline")
                        && !material.name.ends_with("_ope")
                        && !material.name.ends_with("_zpre")
                        && material.texture_count > 0
                    {
                        // TODO: How to make sure the pipeline outputs match the render pass?
                        let pipeline = &self.pipelines[&material.pipeline_key];
                        render_pass.set_pipeline(pipeline);

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
    roots: &[xc3_model::ModelRoot],
) -> Vec<ModelGroup> {
    let start = std::time::Instant::now();

    // Compile shaders only once to improve loading times.
    let pipeline_data = ModelPipelineData::new(device);

    let mut groups = Vec::new();

    for root in roots {
        let textures = load_textures(device, queue, root);
        for group in &root.groups {
            let model_group = create_model_group(device, queue, group, &textures, &pipeline_data);
            groups.push(model_group);
        }
    }
    info!("Load {} model groups: {:?}", roots.len(), start.elapsed());

    groups
}

fn load_textures(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    root: &xc3_model::ModelRoot,
) -> Vec<wgpu::TextureView> {
    root.image_textures
        .iter()
        .map(|texture| {
            create_texture(device, queue, texture)
                .create_view(&wgpu::TextureViewDescriptor::default())
        })
        .collect()
}

// TODO: Make this a method?
fn create_model_group(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    group: &xc3_model::ModelGroup,
    textures: &[wgpu::TextureView],
    pipeline_data: &ModelPipelineData,
) -> ModelGroup {
    let (materials, pipelines) =
        materials(device, queue, pipeline_data, &group.materials, textures);

    let models = group
        .models
        .iter()
        .map(|model| create_model(device, model, group.skeleton.as_ref()))
        .collect();

    let per_group = per_group_bind_group(device, group.skeleton.as_ref());

    ModelGroup {
        materials,
        pipelines,
        models,
        per_group,
    }
}

fn model_index_buffers(device: &wgpu::Device, model: &xc3_model::Model) -> Vec<IndexBuffer> {
    model
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

fn create_model(
    device: &wgpu::Device,
    model: &xc3_model::Model,
    skeleton: Option<&xc3_model::Skeleton>,
) -> Model {
    let vertex_buffers = model_vertex_buffers(device, model, skeleton);
    let index_buffers = model_index_buffers(device, model);

    let meshes = model
        .meshes
        .iter()
        .map(|mesh| Mesh {
            vertex_buffer_index: mesh.vertex_buffer_index,
            index_buffer_index: mesh.index_buffer_index,
            material_index: mesh.material_index,
        })
        .collect();

    let instances = model
        .instances
        .iter()
        .map(|t| {
            let per_model = per_model_bind_group(device, *t);

            ModelInstance { per_model }
        })
        .collect();

    Model {
        vertex_buffers,
        index_buffers,
        meshes,
        instances,
    }
}

fn model_vertex_buffers(
    device: &wgpu::Device,
    model: &xc3_model::Model,
    skeleton: Option<&xc3_model::Skeleton>,
) -> Vec<VertexBuffer> {
    model
        .vertex_buffers
        .iter()
        .map(|buffer| {
            let vertex_count = buffer
                .attributes
                .first()
                .map(|a| a.len())
                .unwrap_or_default();

            let mut vertices = vec![
                shader::model::VertexInput {
                    position: Vec3::ZERO,
                    bone_indices: 0,
                    skin_weights: Vec4::ZERO,
                    vertex_color: Vec4::ZERO,
                    normal: Vec4::ZERO,
                    tangent: Vec4::ZERO,
                    uv1: Vec4::ZERO,
                };
                vertex_count
            ];

            // Convert the attributes back to an interleaved representation for rendering.
            // Unused attributes will use the default values defined above.
            // Using a single vertex representation reduces the number of shaders.
            set_attributes(&mut vertices, buffer, skeleton);

            let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("vertex buffer"),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });

            VertexBuffer { vertex_buffer }
        })
        .collect()
}

fn set_attributes(
    verts: &mut [shader::model::VertexInput],
    buffer: &xc3_model::VertexBuffer,
    skeleton: Option<&xc3_model::Skeleton>,
) {
    for attribute in &buffer.attributes {
        match attribute {
            AttributeData::Position(vals) => set_attribute(verts, vals, |v, t| v.position = t),
            AttributeData::Normal(vals) => set_attribute(verts, vals, |v, t| v.normal = t),
            AttributeData::Tangent(vals) => set_attribute(verts, vals, |v, t| v.tangent = t),
            AttributeData::Uv1(vals) => {
                set_attribute(verts, vals, |v, t| v.uv1 = vec4(t.x, t.y, 0.0, 0.0))
            }
            AttributeData::Uv2(_) => (),
            AttributeData::VertexColor(vals) => {
                set_attribute(verts, vals, |v, t| v.vertex_color = t)
            }
            // Bone influences are handled separately.
            AttributeData::WeightIndex(_) => {}
            AttributeData::SkinWeights(_) => (),
            AttributeData::BoneIndices(_) => (),
        }
    }

    if let Some(skeleton) = skeleton {
        // TODO: Avoid collect?
        let bone_names: Vec<_> = skeleton.bones.iter().map(|b| b.name.as_str()).collect();
        let (indices, weights) = bone_indices_weights(&buffer.influences, verts.len(), &bone_names);

        set_attribute(verts, &indices, |v, t| {
            // TODO: Will this always work as little endian?
            v.bone_indices = u32::from_le_bytes(t)
        });
        set_attribute(verts, &weights, |v, t| v.skin_weights = t);
    }
}

fn set_attribute<T, F>(vertices: &mut [shader::model::VertexInput], values: &[T], assign: F)
where
    T: Copy,
    F: Fn(&mut shader::model::VertexInput, T),
{
    for (vertex, value) in vertices.iter_mut().zip(values) {
        assign(vertex, *value);
    }
}

fn per_group_bind_group(
    device: &wgpu::Device,
    skeleton: Option<&xc3_model::Skeleton>,
) -> shader::model::bind_groups::BindGroup1 {
    // TODO: Set bones from skeletons.
    // TODO: Store the buffer to support animation?
    let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("per group buffer"),
        contents: bytemuck::cast_slice(&[crate::shader::model::PerGroup {
            enable_skinning: uvec4(skeleton.is_some() as u32, 0, 0, 0),
            animated_transforms: [Mat4::IDENTITY; 256],
        }]),
        usage: wgpu::BufferUsages::UNIFORM,
    });

    crate::shader::model::bind_groups::BindGroup1::from_bindings(
        device,
        crate::shader::model::bind_groups::BindGroupLayout1 {
            per_group: buffer.as_entire_buffer_binding(),
        },
    )
}

fn per_model_bind_group(
    device: &wgpu::Device,
    transform: glam::Mat4,
) -> shader::model::bind_groups::BindGroup3 {
    let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("per model buffer"),
        contents: bytemuck::cast_slice(&[crate::shader::model::PerModel { matrix: transform }]),
        usage: wgpu::BufferUsages::UNIFORM,
    });

    crate::shader::model::bind_groups::BindGroup3::from_bindings(
        device,
        crate::shader::model::bind_groups::BindGroupLayout3 {
            per_model: buffer.as_entire_buffer_binding(),
        },
    )
}
