use std::collections::HashMap;

use glam::{uvec4, vec4, Mat4, Vec3, Vec4};
use log::info;
use rayon::prelude::*;
use wgpu::util::DeviceExt;
use xc3_model::{skinning::bone_indices_weights, vertex::AttributeData};

use crate::{
    animation::animate_skeleton,
    material::{materials, Material},
    pipeline::{ModelPipelineData, PipelineKey},
    sampler::create_sampler,
    shader,
    texture::create_texture,
};

// Organize the model data to ensure shared resources are created only once.
pub struct ModelGroup {
    pub models: Vec<Models>,
    buffers: Vec<ModelBuffers>,
}

pub struct ModelBuffers {
    vertex_buffers: Vec<VertexBuffer>,
    index_buffers: Vec<IndexBuffer>,
}

pub struct Models {
    pub models: Vec<Model>,
    materials: Vec<Material>,
    per_group: crate::shader::model::bind_groups::BindGroup1,
    per_group_buffer: wgpu::Buffer,
    skeleton: Option<xc3_model::Skeleton>,
    base_lod_indices: Option<Vec<u16>>,
    // Cache pipelines by their creation parameters.
    pipelines: HashMap<PipelineKey, wgpu::RenderPipeline>,
}

pub struct Model {
    pub meshes: Vec<Mesh>,
    // Use a collection to support "instancing" for map props.
    pub instances: Vec<ModelInstance>,
    pub model_buffers_index: usize,
}

pub struct ModelInstance {
    per_model: crate::shader::model::bind_groups::BindGroup3,
}

#[derive(Debug)]
pub struct Mesh {
    vertex_buffer_index: usize,
    index_buffer_index: usize,
    material_index: usize,
    lod: u16,
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
        for models in &self.models {
            models.per_group.set(render_pass);

            // TODO: Is this the best way to "instance" models?
            for model in &models.models {
                for instance in &model.instances {
                    instance.per_model.set(render_pass);

                    // Each "instance" repeats the same meshes with different transforms.
                    for mesh in &model.meshes {
                        let material = &models.materials[mesh.material_index];

                        // TODO: Group these into passes with separate shaders for each pass?
                        // TODO: The main pass is shared with outline, ope, and zpre?
                        // TODO: How to handle transparency?
                        if (is_transparent != material.pipeline_key.write_to_all_outputs)
                            && !material.name.ends_with("_outline")
                            && !material.name.contains("_speff_")
                            && mesh.should_render_lod(models)
                        {
                            // TODO: How to make sure the pipeline outputs match the render pass?
                            let pipeline = &models.pipelines[&material.pipeline_key];
                            render_pass.set_pipeline(pipeline);

                            material.bind_group2.set(render_pass);

                            self.draw_mesh(model, mesh, render_pass);
                        }
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
        let vertex_data =
            &self.buffers[model.model_buffers_index].vertex_buffers[mesh.vertex_buffer_index];
        render_pass.set_vertex_buffer(0, vertex_data.vertex_buffer.slice(..));

        // TODO: Are all indices u16?
        let index_buffer =
            &self.buffers[model.model_buffers_index].index_buffers[mesh.index_buffer_index];
        render_pass.set_index_buffer(
            index_buffer.index_buffer.slice(..),
            wgpu::IndexFormat::Uint16,
        );

        render_pass.draw_indexed(0..index_buffer.vertex_index_count, 0, 0..1);
    }
}

impl Models {
    pub fn update_bone_transforms(
        &self,
        queue: &wgpu::Queue,
        anim: &xc3_lib::bc::Anim,
        frame: f32,
    ) {
        if let Some(skeleton) = &self.skeleton {
            let animated_transforms = animate_skeleton(skeleton, anim, frame);
            queue.write_buffer(
                &self.per_group_buffer,
                0,
                bytemuck::cast_slice(&[crate::shader::model::PerGroup {
                    enable_skinning: uvec4(1, 0, 0, 0),
                    animated_transforms,
                }]),
            );
        }
    }
}

impl Mesh {
    fn should_render_lod(&self, models: &Models) -> bool {
        xc3_model::should_render_lod(self.lod, &models.base_lod_indices)
    }
}

#[tracing::instrument]
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
        groups.par_extend(
            root.groups
                .par_iter()
                .map(|group| create_model_group(device, queue, group, &textures, &pipeline_data)),
        );
    }

    info!("Load {} model groups: {:?}", roots.len(), start.elapsed());

    groups
}

#[tracing::instrument]
fn load_textures(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    root: &xc3_model::ModelRoot,
) -> Vec<(wgpu::TextureViewDimension, wgpu::TextureView)> {
    root.image_textures
        .iter()
        .map(|texture| {
            // Track the view dimension since shaders expect 2D.
            let dimension = match &texture.view_dimension {
                xc3_model::ViewDimension::D2 => wgpu::TextureViewDimension::D2,
                xc3_model::ViewDimension::D3 => wgpu::TextureViewDimension::D3,
                xc3_model::ViewDimension::Cube => wgpu::TextureViewDimension::Cube,
            };
            let texture = create_texture(device, queue, texture)
                .create_view(&wgpu::TextureViewDescriptor::default());
            (dimension, texture)
        })
        .collect()
}

// TODO: Make this a method?
#[tracing::instrument]
fn create_model_group(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    group: &xc3_model::ModelGroup,
    textures: &[(wgpu::TextureViewDimension, wgpu::TextureView)],
    pipeline_data: &ModelPipelineData,
) -> ModelGroup {
    let models = group
        .models
        .iter()
        .map(|models| {
            let skeleton = models.skeleton.clone();
            let (per_group, per_group_buffer) = per_group_bind_group(device, skeleton.as_ref());

            let base_lod_indices = models.base_lod_indices.clone();

            let samplers: Vec<_> = models
                .samplers
                .iter()
                .map(|s| create_sampler(device, s))
                .collect();

            let (materials, pipelines) = materials(
                device,
                queue,
                pipeline_data,
                &models.materials,
                textures,
                &samplers,
            );

            let models = models
                .models
                .iter()
                .map(|model| create_model(device, model))
                .collect();

            // TODO: Store the samplers?
            Models {
                models,
                materials,
                per_group,
                per_group_buffer,
                pipelines,
                skeleton,
                base_lod_indices,
            }
        })
        .collect();

    let buffers = group
        .buffers
        .iter()
        .map(|buffers| {
            // TODO: How to handle vertex buffers being used with multiple skeletons?
            let vertex_buffers = model_vertex_buffers(
                device,
                buffers,
                group.models.first().and_then(|m| m.skeleton.as_ref()),
            );
            let index_buffers = model_index_buffers(device, buffers);

            ModelBuffers {
                vertex_buffers,
                index_buffers,
            }
        })
        .collect();

    ModelGroup { models, buffers }
}

fn model_index_buffers(
    device: &wgpu::Device,
    buffer: &xc3_model::ModelBuffers,
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

#[tracing::instrument]
fn create_model(device: &wgpu::Device, model: &xc3_model::Model) -> Model {
    let meshes = model
        .meshes
        .iter()
        .map(|mesh| Mesh {
            vertex_buffer_index: mesh.vertex_buffer_index,
            index_buffer_index: mesh.index_buffer_index,
            material_index: mesh.material_index,
            lod: mesh.lod,
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
        meshes,
        instances,
        model_buffers_index: model.model_buffers_index,
    }
}

fn model_vertex_buffers(
    device: &wgpu::Device,
    buffer: &xc3_model::ModelBuffers,
    skeleton: Option<&xc3_model::Skeleton>,
) -> Vec<VertexBuffer> {
    buffer
        .vertex_buffers
        .iter()
        .map(|buffer| {
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
                buffer.vertex_count()
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
    set_buffer_attributes(verts, &buffer.attributes);

    // Just apply the base morph target for now.
    // TODO: Do the morph attributes always override the buffer attributes?
    // TODO: Render morph target animations?
    if let Some(target) = buffer.morph_targets.first() {
        set_buffer_attributes(verts, &target.attributes);
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

fn set_buffer_attributes(verts: &mut [shader::model::VertexInput], attributes: &[AttributeData]) {
    for attribute in attributes {
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
) -> (shader::model::bind_groups::BindGroup1, wgpu::Buffer) {
    // TODO: Store the buffer to support animation?
    let animated_transforms = skeleton
        .map(|skeleton| {
            let mut result = [Mat4::IDENTITY; 256];
            for (transform, result) in skeleton
                .world_transforms()
                .into_iter()
                .zip(result.iter_mut())
            {
                *result = transform * transform.inverse();
            }
            result
        })
        .unwrap_or([Mat4::IDENTITY; 256]);

    let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("per group buffer"),
        contents: bytemuck::cast_slice(&[crate::shader::model::PerGroup {
            enable_skinning: uvec4(skeleton.is_some() as u32, 0, 0, 0),
            animated_transforms,
        }]),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    (
        crate::shader::model::bind_groups::BindGroup1::from_bindings(
            device,
            crate::shader::model::bind_groups::BindGroupLayout1 {
                per_group: buffer.as_entire_buffer_binding(),
            },
        ),
        buffer,
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
