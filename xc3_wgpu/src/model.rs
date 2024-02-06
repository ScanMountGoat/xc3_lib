use std::collections::HashMap;

use glam::{uvec4, Mat4, Vec3, Vec4};
use log::{error, info};
use rayon::prelude::*;
use wgpu::util::DeviceExt;
use xc3_model::{vertex::AttributeData, ImageTexture};

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
    skeleton: Option<xc3_model::Skeleton>,
    per_group: crate::shader::model::bind_groups::BindGroup1,
    per_group_buffer: wgpu::Buffer,
}

pub struct ModelBuffers {
    vertex_buffers: Vec<VertexBuffer>,
    index_buffers: Vec<IndexBuffer>,
}

pub struct Models {
    pub models: Vec<Model>,
    materials: Vec<Material>,

    // TODO: skinning?
    base_lod_indices: Option<Vec<u16>>,
    // Cache pipelines by their creation parameters.
    pipelines: HashMap<PipelineKey, wgpu::RenderPipeline>,
}

pub struct Model {
    pub meshes: Vec<Mesh>,
    instance_buffer: wgpu::Buffer,
    pub instance_count: usize,
    model_buffers_index: usize,
}

pub struct Mesh {
    vertex_buffer_index: usize,
    index_buffer_index: usize,
    material_index: usize,
    lod: u16,
    per_mesh: crate::shader::model::bind_groups::BindGroup3,
}

struct VertexBuffer {
    vertex_buffer0: wgpu::Buffer,
    vertex_buffer1: wgpu::Buffer,
    vertex_count: u32,
    morph_buffers: Option<MorphBuffers>,
}

struct MorphBuffers {
    vertex_buffer0: wgpu::Buffer,
    bind_group0: crate::shader::morph::bind_groups::BindGroup0,
}

struct IndexBuffer {
    index_buffer: wgpu::Buffer,
    vertex_index_count: u32,
}

impl ModelGroup {
    pub fn draw<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>, is_transparent: bool) {
        self.per_group.set(render_pass);

        for models in &self.models {
            for model in &models.models {
                for mesh in &model.meshes {
                    mesh.per_mesh.set(render_pass);

                    let material = &models.materials[mesh.material_index];

                    // TODO: Group these into passes with separate shaders for each pass?
                    // TODO: The main pass is shared with outline, ope, and zpre?
                    // TODO: How to handle transparency?
                    if (is_transparent != material.pipeline_key.write_to_all_outputs())
                        && !material.name.contains("_speff_")
                        && mesh.should_render_lod(models)
                    {
                        // TODO: How to make sure the pipeline outputs match the render pass?
                        let pipeline = &models.pipelines[&material.pipeline_key];
                        render_pass.set_pipeline(pipeline);

                        let stencil_reference = material.pipeline_key.stencil_reference();
                        render_pass.set_stencil_reference(stencil_reference);

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
        let vertex_buffers =
            &self.buffers[model.model_buffers_index].vertex_buffers[mesh.vertex_buffer_index];

        if let Some(morph_buffers) = &vertex_buffers.morph_buffers {
            render_pass.set_vertex_buffer(0, morph_buffers.vertex_buffer0.slice(..));
        } else {
            render_pass.set_vertex_buffer(0, vertex_buffers.vertex_buffer0.slice(..));
        }

        render_pass.set_vertex_buffer(1, vertex_buffers.vertex_buffer1.slice(..));
        render_pass.set_vertex_buffer(2, model.instance_buffer.slice(..));

        // TODO: Are all indices u16?
        let index_buffer =
            &self.buffers[model.model_buffers_index].index_buffers[mesh.index_buffer_index];
        render_pass.set_index_buffer(
            index_buffer.index_buffer.slice(..),
            wgpu::IndexFormat::Uint16,
        );

        render_pass.draw_indexed(
            0..index_buffer.vertex_index_count,
            0,
            0..model.instance_count as u32,
        );
    }

    pub fn reset_morphs(&self, encoder: &mut wgpu::CommandEncoder) {
        for buffers in &self.buffers {
            for vertex_buffer in &buffers.vertex_buffers {
                if let Some(morph_buffers) = &vertex_buffer.morph_buffers {
                    encoder.copy_buffer_to_buffer(
                        &vertex_buffer.vertex_buffer0,
                        0,
                        &morph_buffers.vertex_buffer0,
                        0,
                        vertex_buffer.vertex_buffer0.size(),
                    );
                }
            }
        }
    }

    pub fn compute_morphs<'a>(&'a self, compute_pass: &mut wgpu::ComputePass<'a>) {
        for buffers in &self.buffers {
            for vertex_buffer in &buffers.vertex_buffers {
                if let Some(morph_buffers) = &vertex_buffer.morph_buffers {
                    morph_buffers.bind_group0.set(compute_pass);
                    let [size_x, _, _] = crate::shader::morph::compute::MAIN_WORKGROUP_SIZE;
                    let x = div_round_up(vertex_buffer.vertex_count, size_x);
                    compute_pass.dispatch_workgroups(x, 1, 1);
                }
            }
        }
    }
}

const fn div_round_up(x: u32, d: u32) -> u32 {
    (x + d - 1) / d
}

impl ModelGroup {
    pub fn update_bone_transforms(
        &self,
        queue: &wgpu::Queue,
        animation: &xc3_model::animation::Animation,
        current_time_seconds: f32,
    ) {
        if let Some(skeleton) = &self.skeleton {
            let animated_transforms = animate_skeleton(skeleton, animation, current_time_seconds);
            let animated_transforms_inv_transpose =
                animated_transforms.map(|t| t.inverse().transpose());
            queue.write_buffer(
                &self.per_group_buffer,
                0,
                bytemuck::cast_slice(&[crate::shader::model::PerGroup {
                    enable_skinning: uvec4(1, 0, 0, 0),
                    animated_transforms,
                    animated_transforms_inv_transpose,
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

#[tracing::instrument(skip_all)]
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
        groups.par_extend(root.groups.par_iter().map(|group| {
            create_model_group(
                device,
                queue,
                group,
                &textures,
                &root.image_textures,
                &pipeline_data,
                root.skeleton.clone(),
            )
        }));
    }

    info!("Load {} model groups: {:?}", roots.len(), start.elapsed());

    groups
}

#[tracing::instrument(skip_all)]
fn load_textures(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    root: &xc3_model::ModelRoot,
) -> Vec<wgpu::Texture> {
    // TODO: Store the texture usage?
    root.image_textures
        .iter()
        .map(|texture| create_texture(device, queue, texture))
        .collect()
}

// TODO: Make this a method?
#[tracing::instrument(skip_all)]
fn create_model_group(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    group: &xc3_model::ModelGroup,
    textures: &[wgpu::Texture],
    image_textures: &[ImageTexture],
    pipeline_data: &ModelPipelineData,
    skeleton: Option<xc3_model::Skeleton>,
) -> ModelGroup {
    let buffers: Vec<_> = group
        .buffers
        .iter()
        .map(|buffers| {
            // TODO: How to handle vertex buffers being used with multiple skeletons?
            let vertex_buffers = model_vertex_buffers(device, buffers);
            let index_buffers = model_index_buffers(device, buffers);

            // TODO: Each vertex buffer needs its own transformed matrices?
            ModelBuffers {
                vertex_buffers,
                index_buffers,
            }
        })
        .collect();

    let models = group
        .models
        .iter()
        .map(|models| {
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
                image_textures,
            );

            let models = models
                .models
                .iter()
                .map(|model| {
                    create_model(device, model, &group.buffers, skeleton.as_ref(), &materials)
                })
                .collect();

            // TODO: Store the samplers?
            Models {
                models,
                materials,
                pipelines,
                base_lod_indices,
            }
        })
        .collect();

    // In practice, weights are only used for wimdo files with one Models and one Model.
    // TODO: How to enforce this assumption?
    // TODO: Avoid clone.
    // Reindex to match the ordering defined in the current skeleton.
    let skin_weights = group.buffers[0].weights.as_ref().map(|weights| {
        skeleton
            .as_ref()
            .map(|skeleton| {
                let bone_names = skeleton.bones.iter().map(|b| b.name.clone()).collect();
                weights.skin_weights.reindex_bones(bone_names)
            })
            .unwrap_or_else(|| weights.skin_weights.clone())
    });

    let (per_group, per_group_buffer) =
        per_group_bind_group(device, skeleton.as_ref(), skin_weights.as_ref());

    ModelGroup {
        models,
        buffers,
        per_group,
        per_group_buffer,
        skeleton,
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

#[tracing::instrument(skip_all)]
fn create_model(
    device: &wgpu::Device,
    model: &xc3_model::Model,
    buffers: &[xc3_model::vertex::ModelBuffers],
    skeleton: Option<&xc3_model::Skeleton>,
    materials: &[Material],
) -> Model {
    let model_buffers = &buffers[model.model_buffers_index];
    // Reindex to match the ordering defined in the current skeleton.
    let skin_weights = model_buffers.weights.as_ref().map(|weights| {
        skeleton
            .map(|skeleton| {
                let bone_names = skeleton.bones.iter().map(|b| b.name.clone()).collect();
                weights.skin_weights.reindex_bones(bone_names)
            })
            .unwrap_or_else(|| weights.skin_weights.clone())
    });

    let meshes = model
        .meshes
        .iter()
        .map(|mesh| Mesh {
            vertex_buffer_index: mesh.vertex_buffer_index,
            index_buffer_index: mesh.index_buffer_index,
            material_index: mesh.material_index,
            lod: mesh.lod,
            per_mesh: per_mesh_bind_group(
                device,
                model_buffers,
                skin_weights.as_ref(),
                mesh.lod,
                mesh.skin_flags,
                mesh.vertex_buffer_index,
                &materials[mesh.material_index],
            ),
        })
        .collect();

    let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("instance buffer"),
        contents: bytemuck::cast_slice(&model.instances),
        usage: wgpu::BufferUsages::VERTEX,
    });

    Model {
        meshes,
        instance_buffer,
        instance_count: model.instances.len(),
        model_buffers_index: model.model_buffers_index,
    }
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
                    vertex_color: Vec4::ZERO,
                    uv1: Vec3::ZERO,
                    weight_index: 0
                };
                vertex_count
            ];

            set_attributes(
                &mut buffer0_vertices,
                &mut buffer1_vertices,
                buffer,
                &buffers.outline_buffers,
            );

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

            // TODO: morph targets?
            let morph_buffers = if !buffer.morph_targets.is_empty() {
                Some(morph_buffers(device, buffer0_vertices, buffer))
            } else {
                None
            };

            VertexBuffer {
                vertex_buffer0,
                vertex_buffer1,
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
                position_deltas[*vertex_index as usize] = target.position_deltas[i].extend(0.0);
                normal_deltas[*vertex_index as usize] = target.normal_deltas[i];
                tangent_deltas[*vertex_index as usize] = target.tangent_deltas[i];
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

    let morph_deltas = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("morph deltas"),
        contents: bytemuck::cast_slice(&deltas),
        usage: wgpu::BufferUsages::STORAGE,
    });

    let weights = vec![0.0f32; buffer.morph_targets.len()];
    let morph_weights = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("morph weights"),
        contents: bytemuck::cast_slice(&weights),
        usage: wgpu::BufferUsages::STORAGE,
    });

    let bind_group0 = crate::shader::morph::bind_groups::BindGroup0::from_bindings(
        device,
        crate::shader::morph::bind_groups::BindGroupLayout0 {
            vertices: morph_vertex_buffer0.as_entire_buffer_binding(),
            morph_deltas: morph_deltas.as_entire_buffer_binding(),
            morph_weights: morph_weights.as_entire_buffer_binding(),
        },
    );

    MorphBuffers {
        vertex_buffer0: morph_vertex_buffer0,
        bind_group0,
    }
}

fn set_attributes(
    buffer0_vertices: &mut [shader::model::VertexInput0],
    buffer1_vertices: &mut [shader::model::VertexInput1],
    buffer: &xc3_model::vertex::VertexBuffer,
    outline_buffers: &[xc3_model::vertex::OutlineBuffer],
) {
    set_buffer0_attributes(buffer0_vertices, &buffer.attributes);
    set_buffer1_attributes(buffer1_vertices, &buffer.attributes);

    if let Some(outline_buffer) = buffer
        .outline_buffer_index
        .and_then(|i| outline_buffers.get(i))
    {
        // TODO: Should outline attributes not override existing attributes?
        set_buffer1_attributes(buffer1_vertices, &outline_buffer.attributes)
    }
}

fn set_buffer0_attributes(verts: &mut [shader::model::VertexInput0], attributes: &[AttributeData]) {
    for attribute in attributes {
        match attribute {
            AttributeData::Position(vals) => {
                set_attribute0(verts, vals, |v, t| v.position = t.extend(1.0))
            }
            AttributeData::Normal(vals) => set_attribute0(verts, vals, |v, t| v.normal = t),
            AttributeData::Tangent(vals) => set_attribute0(verts, vals, |v, t| v.tangent = t),
            _ => (),
        }
    }
}

fn set_buffer1_attributes(verts: &mut [shader::model::VertexInput1], attributes: &[AttributeData]) {
    for attribute in attributes {
        match attribute {
            AttributeData::TexCoord0(vals) => {
                set_attribute1(verts, vals, |v, t| v.uv1 = t.extend(0.0))
            }
            AttributeData::VertexColor(vals) => {
                set_attribute1(verts, vals, |v, t| v.vertex_color = t)
            }
            AttributeData::WeightIndex(vals) => {
                set_attribute1(verts, vals, |v, t| v.weight_index = t)
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

fn per_group_bind_group(
    device: &wgpu::Device,
    skeleton: Option<&xc3_model::Skeleton>,
    skin_weights: Option<&xc3_model::skinning::SkinWeights>,
) -> (shader::model::bind_groups::BindGroup1, wgpu::Buffer) {
    let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("per group buffer"),
        contents: bytemuck::cast_slice(&[crate::shader::model::PerGroup {
            enable_skinning: uvec4(skeleton.is_some() as u32, 0, 0, 0),
            animated_transforms: [Mat4::IDENTITY; 256],
            animated_transforms_inv_transpose: [Mat4::IDENTITY; 256],
        }]),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    // Convert to u32 since WGSL lacks a vec4<u8> type.
    // This assumes the skinning shader code is skipped if anything is missing.
    // TODO: How to correctly handle a missing skeleton or weights?
    let indices: Vec<_> = skin_weights
        .as_ref()
        .map(|skin_weights| {
            skin_weights
                .bone_indices
                .iter()
                .map(|indices| indices.map(|i| i as u32))
                .collect()
        })
        .unwrap_or_else(|| vec![[0; 4]]);

    let weights = skin_weights
        .as_ref()
        .map(|skin_weights| skin_weights.weights.as_slice())
        .unwrap_or(&[Vec4::ZERO]);

    let bone_indices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("bone indices buffer"),
        contents: bytemuck::cast_slice(&indices),
        usage: wgpu::BufferUsages::STORAGE,
    });

    let skin_weights = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("skin weights buffer"),
        contents: bytemuck::cast_slice(weights),
        usage: wgpu::BufferUsages::STORAGE,
    });

    (
        crate::shader::model::bind_groups::BindGroup1::from_bindings(
            device,
            crate::shader::model::bind_groups::BindGroupLayout1 {
                per_group: buffer.as_entire_buffer_binding(),
                bone_indices: bone_indices.as_entire_buffer_binding(),
                skin_weights: skin_weights.as_entire_buffer_binding(),
            },
        ),
        buffer,
    )
}

fn per_mesh_bind_group(
    device: &wgpu::Device,
    buffers: &xc3_model::vertex::ModelBuffers,
    skin_weights: Option<&xc3_model::skinning::SkinWeights>,
    lod: u16,
    skin_flags: u32,
    vertex_buffer_index: usize,
    material: &Material,
) -> shader::model::bind_groups::BindGroup3 {
    let weight_count = skin_weights.map(|w| w.weights.len()).unwrap_or_default();

    // TODO: Fix weight indexing calculations.
    let start = buffers
        .weights
        .as_ref()
        .map(|weights| weights.weights_start_index(skin_flags, lod, material.pipeline_key.unk_type))
        .unwrap_or_default();

    for attribute in &buffers.vertex_buffers[vertex_buffer_index].attributes {
        if let AttributeData::WeightIndex(weight_indices) = attribute {
            let max_index = *weight_indices.iter().max().unwrap() as usize;
            if max_index + start >= weight_count {
                error!(
                    "Weight index start {} and max weight index {} exceed weight count {} with {:?}",
                    start, max_index, weight_count,
                    (skin_flags, lod, material.pipeline_key.unk_type)
                );
            }
        }
    }

    let per_mesh = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("per mesh buffer"),
        contents: bytemuck::cast_slice(&[crate::shader::model::PerMesh {
            weight_group_indices: uvec4(start as u32, 0, 0, 0),
        }]),
        usage: wgpu::BufferUsages::UNIFORM,
    });

    // Bone indices and skin weights are technically part of the model buffers.
    // Each mesh selects a range of values based on weight lods.
    // Define skinning per mesh to avoid alignment requirements on buffer bindings.
    crate::shader::model::bind_groups::BindGroup3::from_bindings(
        device,
        crate::shader::model::bind_groups::BindGroupLayout3 {
            per_mesh: per_mesh.as_entire_buffer_binding(),
        },
    )
}
