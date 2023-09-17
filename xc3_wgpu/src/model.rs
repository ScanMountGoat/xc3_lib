use std::collections::HashMap;

use glam::{uvec4, vec3, vec4, Mat4, Quat, Vec3, Vec4};
use log::{error, info};
use rayon::prelude::*;
use wgpu::util::DeviceExt;
use xc3_lib::bc::murmur3;
use xc3_model::{skinning::bone_indices_weights, vertex::AttributeData};

use crate::{
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
    // TODO: how to handle LOD?
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
                        // TODO: Characters render as solid white?
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
    pub fn update_bone_transforms(&self, queue: &wgpu::Queue, anim: &xc3_lib::bc::Anim) {
        if let Some(skeleton) = &self.skeleton {
            let hash_to_index: HashMap<_, _> = skeleton
                .bones
                .iter()
                .enumerate()
                .map(|(i, b)| (murmur3(b.name.as_bytes()), i))
                .collect();

            // Just create a copy of the skeleton to simplify the code for now.
            let mut animated_skeleton = skeleton.clone();

            // TODO: Load all key frames?
            match &anim.binding.animation.data {
                xc3_lib::bc::AnimationData::Unk0 => todo!(),
                xc3_lib::bc::AnimationData::Cubic(cubic) => {
                    // TODO: Does each of these tracks have a corresponding hash?
                    // TODO: Also check the bone indices?
                    for (track, bone_index) in cubic
                        .tracks
                        .elements
                        .iter()
                        .zip(anim.binding.bone_indices.elements.iter())
                    {
                        // TODO: cubic interpolation?
                        // TODO: Add sample methods to the keyframe types?
                        // TODO: Will the first key always be at time 0?
                        let key = &track.translation.elements[0];
                        let translation = vec3(key.x[3], key.y[3], key.z[3]);

                        let key = &track.rotation.elements[0];
                        let rotation = Quat::from_xyzw(key.x[3], key.y[3], key.z[3], key.w[3]);

                        let key = &track.scale.elements[0];
                        let scale = vec3(key.x[3], key.y[3], key.z[3]);

                        if *bone_index >= 0 {
                            // TODO: Does this work in any tools yet?
                            // TODO: Should this use mxmd ordering?
                            let transform = Mat4::from_translation(translation)
                                * Mat4::from_quat(rotation)
                                * Mat4::from_scale(scale);
                            animated_skeleton.bones[*bone_index as usize].transform = transform;
                        }
                    }
                }
                xc3_lib::bc::AnimationData::Unk2 => todo!(),
                xc3_lib::bc::AnimationData::PackedCubic(cubic) => {
                    // TODO: Does each of these tracks have a corresponding hash?
                    // TODO: Also check the bone indices?
                    if let xc3_lib::bc::ExtraTrackAnimationData::PackedCubic(extra) =
                        &anim.binding.extra_track_animation.data
                    {
                        for (track, hash) in cubic
                            .tracks
                            .elements
                            .iter()
                            .zip(extra.hashes.elements.iter())
                        {
                            // TODO: cubic interpolation?
                            let translation = sample_vec3_packed_cubic(
                                cubic,
                                track.translation.curves_start_index as usize,
                            );
                            let rotation = sample_quat_packed_cubic(
                                cubic,
                                track.rotation.curves_start_index as usize,
                            );
                            let scale = sample_vec3_packed_cubic(
                                cubic,
                                track.scale.curves_start_index as usize,
                            );

                            if let Some(bone_index) = hash_to_index.get(hash) {
                                // TODO: Does every track start at time 0?
                                let transform = Mat4::from_translation(translation)
                                    * Mat4::from_quat(rotation)
                                    * Mat4::from_scale(scale);
                                animated_skeleton.bones[*bone_index].transform = transform;
                            } else {
                                error!("No matching bone for hash {hash:x}");
                            }
                        }
                    }
                }
            }

            let rest_pose_world = skeleton.world_transforms();
            let animated_world = animated_skeleton.world_transforms();

            let mut animated_transforms = [Mat4::IDENTITY; 256];
            for i in (0..skeleton.bones.len()).take(animated_transforms.len()) {
                animated_transforms[i] = animated_world[i] * rest_pose_world[i].inverse();
            }

            queue.write_buffer(
                &self.per_group_buffer,
                0,
                bytemuck::cast_slice(&[crate::shader::model::PerGroup {
                    enable_skinning: uvec4(self.skeleton.is_some() as u32, 0, 0, 0),
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

fn sample_vec3_packed_cubic(cubic: &xc3_lib::bc::PackedCubic, start_index: usize) -> Vec3 {
    let x_coeffs = cubic.vectors.elements[start_index];
    let y_coeffs = cubic.vectors.elements[start_index + 1];
    let z_coeffs = cubic.vectors.elements[start_index + 2];
    vec3(x_coeffs[3], y_coeffs[3], z_coeffs[3])
}

fn sample_quat_packed_cubic(cubic: &xc3_lib::bc::PackedCubic, start_index: usize) -> Quat {
    let x_coeffs = cubic.quaternions.elements[start_index];
    let y_coeffs = cubic.quaternions.elements[start_index + 1];
    let z_coeffs = cubic.quaternions.elements[start_index + 2];
    let w_coeffs = cubic.quaternions.elements[start_index + 3];
    Quat::from_xyzw(x_coeffs[3], y_coeffs[3], z_coeffs[3], w_coeffs[3])
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
