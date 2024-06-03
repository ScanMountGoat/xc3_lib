use std::collections::HashMap;

use glam::{uvec4, vec4, Mat4, Vec3, Vec4};
use log::{error, info};
use rayon::prelude::*;
use wgpu::util::DeviceExt;
use xc3_model::{vertex::AttributeData, ImageTexture, LodData, MeshRenderFlags2, MeshRenderPass};

use crate::{
    animation::animated_skinning_transforms,
    culling::is_within_frustum,
    material::{materials, Material},
    pipeline::{ModelPipelineData, Output5Type, PipelineKey},
    sampler::create_sampler,
    shader,
    texture::create_texture,
    CameraData, DeviceBufferExt, MonolibShaderTextures, QueueBufferExt,
};

// Organize the model data to ensure shared resources are created only once.
pub struct ModelGroup {
    pub models: Vec<Models>,
    buffers: Vec<ModelBuffers>,
    skeleton: Option<xc3_model::Skeleton>,
    per_group: crate::shader::model::bind_groups::BindGroup1,
    per_group_buffer: wgpu::Buffer,
    pub(crate) bone_animated_transforms: wgpu::Buffer,
    pub(crate) bone_count: usize,

    // Cache pipelines by their creation parameters.
    pipelines: HashMap<PipelineKey, wgpu::RenderPipeline>,
}

pub struct ModelBuffers {
    vertex_buffers: Vec<VertexBuffer>,
    index_buffers: Vec<IndexBuffer>,
}

impl ModelBuffers {
    fn from_buffers(device: &wgpu::Device, buffers: &xc3_model::vertex::ModelBuffers) -> Self {
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

// TODO: aabb tree for culling?
pub struct Models {
    pub models: Vec<Model>,
    materials: Vec<Material>,
    bounds: Bounds,

    // TODO: skinning?
    lod_data: Option<LodData>,
    morph_controller_names: Vec<String>,
    animation_morph_names: Vec<String>,
}

impl Models {
    fn from_models(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        models: &xc3_model::Models,
        buffers: &[xc3_model::vertex::ModelBuffers],
        skeleton: Option<&xc3_model::Skeleton>,
        pipelines: &mut HashMap<PipelineKey, wgpu::RenderPipeline>,
        pipeline_data: &ModelPipelineData,
        textures: &[wgpu::Texture],
        image_textures: &[ImageTexture],
        monolib_shader: &MonolibShaderTextures,
    ) -> Self {
        // In practice, weights are only used for wimdo files with one Models and one Model.
        // TODO: How to enforce this assumption?
        // TODO: Avoid clone.
        // Reindex to match the ordering defined in the current skeleton.
        let weights = buffers.first().and_then(|b| b.weights.as_ref());
        let bone_names: Option<Vec<_>> = skeleton
            .as_ref()
            .map(|s| s.bones.iter().map(|b| b.name.clone()).collect());

        let lod_data = models.lod_data.clone();
        let morph_controller_names = models.morph_controller_names.clone();
        let animation_morph_names = models.animation_morph_names.clone();

        let bounds = Bounds::new(device, models.max_xyz, models.min_xyz, &Mat4::IDENTITY);

        let samplers: Vec<_> = models
            .samplers
            .iter()
            .map(|s| create_sampler(device, s))
            .collect();

        let materials = materials(
            device,
            queue,
            pipelines,
            pipeline_data,
            &models.materials,
            textures,
            &samplers,
            image_textures,
            monolib_shader,
        );

        // TODO: Avoid clone?
        let models = models
            .models
            .iter()
            .map(|model| {
                create_model(
                    device,
                    model,
                    buffers,
                    &materials,
                    weights,
                    bone_names.as_deref(),
                )
            })
            .collect();

        // TODO: Store the samplers?
        Self {
            models,
            materials,
            lod_data,
            morph_controller_names,
            animation_morph_names,
            bounds,
        }
    }
}

pub struct Model {
    pub meshes: Vec<Mesh>,
    model_buffers_index: usize,
    instance_buffer: wgpu::Buffer,
    pub instance_count: usize,
}

pub struct Mesh {
    vertex_buffer_index: usize,
    index_buffer_index: usize,
    material_index: usize,
    flags2: MeshRenderFlags2,
    lod: Option<usize>,
    per_mesh: crate::shader::model::bind_groups::BindGroup3,
}

struct Bounds {
    max_xyz: Vec3,
    min_xyz: Vec3,
    bounds_vertex_buffer: wgpu::Buffer,
    bounds_index_buffer: wgpu::Buffer,
}

struct VertexBuffer {
    vertex_buffer0: wgpu::Buffer,
    vertex_buffer1: wgpu::Buffer,
    vertex_count: u32,
    morph_buffers: Option<MorphBuffers>,
}

struct MorphBuffers {
    vertex_buffer0: wgpu::Buffer,
    weights_buffer: wgpu::Buffer,
    bind_group0: crate::shader::morph::bind_groups::BindGroup0,
    morph_target_controller_indices: Vec<usize>,
}

struct IndexBuffer {
    index_buffer: wgpu::Buffer,
    vertex_index_count: u32,
}

impl Bounds {
    fn new(device: &wgpu::Device, max_xyz: Vec3, min_xyz: Vec3, transform: &Mat4) -> Self {
        let (bounds_vertex_buffer, bounds_index_buffer) =
            wireframe_aabb_box_vertex_index(device, min_xyz, max_xyz, transform);

        // TODO: include transform in the min/max xyz values.
        Self {
            max_xyz,
            min_xyz,
            bounds_vertex_buffer,
            bounds_index_buffer,
        }
    }

    fn draw<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        culled: bool,
        bind_group1: &'a crate::shader::solid::bind_groups::BindGroup1,
        culled_bind_group1: &'a crate::shader::solid::bind_groups::BindGroup1,
    ) {
        render_pass.set_vertex_buffer(0, self.bounds_vertex_buffer.slice(..));
        render_pass.set_index_buffer(
            self.bounds_index_buffer.slice(..),
            wgpu::IndexFormat::Uint16,
        );
        if culled {
            culled_bind_group1.set(render_pass);
        } else {
            bind_group1.set(render_pass);
        }

        // 12 lines with 2 points each.
        render_pass.draw_indexed(0..24, 0, 0..1);
    }
}

impl ModelGroup {
    /// Draw each mesh for each model.
    pub fn draw<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        is_transparent: bool,
        pass_id: MeshRenderPass,
        camera: &CameraData,
        output5_type: Option<Output5Type>,
    ) {
        self.per_group.set(render_pass);

        // TODO: This should account for the instance transforms.
        // Assume the models AABB contains each model AABB.
        // This allows for better culling efficiency.
        for models in self
            .models
            .iter()
            .filter(|m| is_within_frustum(m.bounds.min_xyz, m.bounds.max_xyz, camera))
        {
            // TODO: cull aabb with instance transforms.
            for model in models.models.iter() {
                for mesh in &model.meshes {
                    let material = &models.materials[mesh.material_index];

                    // TODO: Group these into passes with separate shaders for each pass?
                    // TODO: The main pass is shared with outline, ope, and zpre?
                    // TODO: How to handle transparency?
                    // Only check the output5 type if needed.
                    if (is_transparent != material.pipeline_key.write_to_all_outputs())
                        && !material.name.contains("_speff_")
                        && mesh.should_render_lod(models)
                        && mesh.flags2.render_pass() == pass_id
                        && output5_type
                            .map(|ty| material.pipeline_key.output5_type == ty)
                            .unwrap_or(true)
                    {
                        mesh.per_mesh.set(render_pass);

                        // TODO: How to make sure the pipeline outputs match the render pass?
                        let pipeline = &self.pipelines[&material.pipeline_key];
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

    /// Draw the bounding box for each model and group of models.
    pub fn draw_bounds<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        bind_group1: &'a crate::shader::solid::bind_groups::BindGroup1,
        culled_bind_group1: &'a crate::shader::solid::bind_groups::BindGroup1,
        camera: &CameraData,
    ) {
        for models in &self.models {
            let cull_models =
                !is_within_frustum(models.bounds.min_xyz, models.bounds.max_xyz, camera);
            models
                .bounds
                .draw(render_pass, cull_models, bind_group1, culled_bind_group1);

            // TODO: model specific culling?
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

    /// Animate each of the bone transforms in the current skeleton.
    pub fn update_bone_transforms(
        &self,
        queue: &wgpu::Queue,
        animation: &xc3_model::animation::Animation,
        current_time_seconds: f32,
    ) {
        if let Some(skeleton) = &self.skeleton {
            let animated_transforms =
                animated_skinning_transforms(skeleton, animation, current_time_seconds);
            let animated_transforms_inv_transpose =
                animated_transforms.map(|t| t.inverse().transpose());
            queue.write_uniform_data(
                &self.per_group_buffer,
                &crate::shader::model::PerGroup {
                    enable_skinning: uvec4(1, 0, 0, 0),
                    animated_transforms,
                    animated_transforms_inv_transpose,
                },
            );

            let bone_transforms = animation
                .model_space_transforms(skeleton, animation.current_frame(current_time_seconds));
            // TODO: Add an ext method to lib.rs?
            queue.write_buffer(
                &self.bone_animated_transforms,
                0,
                bytemuck::cast_slice(&bone_transforms),
            );
        }
    }

    /// Update morph weights for all vertex buffers.
    pub fn update_morph_weights(
        &self,
        queue: &wgpu::Queue,
        animation: &xc3_model::animation::Animation,
        current_time_seconds: f32,
    ) {
        // TODO: Tests for this?
        let morph_controller_names = &self.models[0].morph_controller_names;
        let animation_morph_names = &self.models[0].animation_morph_names;

        let frame = animation.current_frame(current_time_seconds);

        for buffers in &self.buffers {
            for buffer in &buffers.vertex_buffers {
                if let Some(morph_buffers) = &buffer.morph_buffers {
                    let weights = animation.morph_weights(
                        morph_controller_names,
                        animation_morph_names,
                        &morph_buffers.morph_target_controller_indices,
                        frame,
                    );

                    queue.write_storage_data(&morph_buffers.weights_buffer, &weights);
                }
            }
        }
    }
}

const fn div_round_up(x: u32, d: u32) -> u32 {
    (x + d - 1) / d
}

impl Mesh {
    fn should_render_lod(&self, models: &Models) -> bool {
        models
            .lod_data
            .as_ref()
            .map(|d| d.is_base_lod(self.lod))
            .unwrap_or(true)
    }
}

#[tracing::instrument(skip_all)]
pub fn load_model(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    roots: &[xc3_model::ModelRoot],
    monolib_shader: &MonolibShaderTextures,
) -> Vec<ModelGroup> {
    let start = std::time::Instant::now();

    // Compile shaders only once to improve loading times.
    let pipeline_data = ModelPipelineData::new(device);

    let mut groups = Vec::new();
    for root in roots {
        let textures = load_textures(device, queue, &root.image_textures);
        // TODO: Avoid clone?
        let group = create_model_group(
            device,
            queue,
            &xc3_model::ModelGroup {
                models: vec![root.models.clone()],
                buffers: vec![root.buffers.clone()],
            },
            &textures,
            &root.image_textures,
            &pipeline_data,
            root.skeleton.as_ref(),
            monolib_shader,
        );
        groups.push(group);
    }

    info!("Load {} model groups: {:?}", roots.len(), start.elapsed());

    groups
}

#[tracing::instrument(skip_all)]
pub fn load_map(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    roots: &[xc3_model::MapRoot],
    monolib_shader: &MonolibShaderTextures,
) -> Vec<ModelGroup> {
    let start = std::time::Instant::now();

    // Compile shaders only once to improve loading times.
    let pipeline_data = ModelPipelineData::new(device);

    let mut groups = Vec::new();
    for root in roots {
        let textures = load_textures(device, queue, &root.image_textures);
        groups.par_extend(root.groups.par_iter().map(|group| {
            create_model_group(
                device,
                queue,
                group,
                &textures,
                &root.image_textures,
                &pipeline_data,
                None,
                monolib_shader,
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
    image_textures: &[xc3_model::ImageTexture],
) -> Vec<wgpu::Texture> {
    image_textures
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
    skeleton: Option<&xc3_model::Skeleton>,
    monolib_shader: &MonolibShaderTextures,
) -> ModelGroup {
    let (per_group, per_group_buffer) = per_group_bind_group(device, skeleton);

    // TODO: Create helper ext method in lib.rs?
    let bone_transforms = skeleton
        .as_ref()
        .map(|s| s.model_space_transforms())
        .unwrap_or_default();
    let bone_count = skeleton.as_ref().map(|s| s.bones.len()).unwrap_or_default();

    let bone_animated_transforms = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Bone Animated Transforms"),
        contents: bytemuck::cast_slice(&bone_transforms),
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
    });

    let buffers = group
        .buffers
        .iter()
        .map(|buffers| ModelBuffers::from_buffers(device, buffers))
        .collect();

    let mut pipelines = HashMap::new();

    let models = group
        .models
        .iter()
        .map(|models| {
            Models::from_models(
                device,
                queue,
                models,
                &group.buffers,
                skeleton,
                &mut pipelines,
                pipeline_data,
                textures,
                image_textures,
                monolib_shader,
            )
        })
        .collect();

    ModelGroup {
        models,
        buffers,
        per_group,
        per_group_buffer,
        skeleton: skeleton.cloned(),
        bone_animated_transforms,
        bone_count,
        pipelines,
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
    materials: &[Material],
    weights: Option<&xc3_model::skinning::Weights>,
    bone_names: Option<&[String]>,
) -> Model {
    let model_buffers = &buffers[model.model_buffers_index];

    let meshes = model
        .meshes
        .iter()
        .map(|mesh| Mesh {
            vertex_buffer_index: mesh.vertex_buffer_index,
            index_buffer_index: mesh.index_buffer_index,
            material_index: mesh.material_index,
            lod: mesh.lod_item_index,
            flags2: mesh.flags2,
            per_mesh: per_mesh_bind_group(
                device,
                model_buffers,
                mesh,
                &materials[mesh.material_index],
                weights,
                bone_names,
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

fn wireframe_aabb_box_vertex_index(
    device: &wgpu::Device,
    min_xyz: Vec3,
    max_xyz: Vec3,
    transform: &Mat4,
) -> (wgpu::Buffer, wgpu::Buffer) {
    let corners = [
        vec4(min_xyz.x, min_xyz.y, min_xyz.z, 1.0),
        vec4(max_xyz.x, min_xyz.y, min_xyz.z, 1.0),
        vec4(max_xyz.x, max_xyz.y, min_xyz.z, 1.0),
        vec4(min_xyz.x, max_xyz.y, min_xyz.z, 1.0),
        vec4(min_xyz.x, min_xyz.y, max_xyz.z, 1.0),
        vec4(max_xyz.x, min_xyz.y, max_xyz.z, 1.0),
        vec4(max_xyz.x, max_xyz.y, max_xyz.z, 1.0),
        vec4(min_xyz.x, max_xyz.y, max_xyz.z, 1.0),
    ]
    .map(|c| *transform * c);

    let bounds_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("bounds vertex buffer"),
        contents: bytemuck::cast_slice(&corners),
        usage: wgpu::BufferUsages::VERTEX,
    });

    let bounds_index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("bounds index buffer"),
        contents: bytemuck::cast_slice(&[
            [0u16, 1u16],
            [1u16, 2u16],
            [2u16, 3u16],
            [3u16, 0u16],
            [0u16, 4u16],
            [1u16, 5u16],
            [2u16, 6u16],
            [3u16, 7u16],
            [4u16, 5u16],
            [5u16, 6u16],
            [6u16, 7u16],
            [7u16, 4u16],
        ]),
        usage: wgpu::BufferUsages::INDEX,
    });

    (bounds_vertex_buffer, bounds_index_buffer)
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
                    tex0: Vec3::ZERO,
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
                normal_deltas[*vertex_index as usize] = target.normals[i];
                tangent_deltas[*vertex_index as usize] = target.tangents[i];
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
    outline_buffers: &[xc3_model::vertex::OutlineBuffer],
) {
    set_buffer0_attributes(buffer0_vertices, &buffer.attributes);
    set_buffer0_attributes(buffer0_vertices, &buffer.morph_blend_target);
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
            // Morph blend target attributes
            AttributeData::Position2(vals) => {
                set_attribute0(verts, vals, |v, t| v.position = t.extend(1.0))
            }
            AttributeData::Normal4(vals) => {
                set_attribute0(verts, vals, |v, t| v.normal = t * 2.0 - 1.0)
            }
            AttributeData::Tangent2(vals) => {
                set_attribute0(verts, vals, |v, t| v.tangent = t * 2.0 - 1.0)
            }
            _ => (),
        }
    }
}

fn set_buffer1_attributes(verts: &mut [shader::model::VertexInput1], attributes: &[AttributeData]) {
    for attribute in attributes {
        match attribute {
            AttributeData::TexCoord0(vals) => {
                set_attribute1(verts, vals, |v, t| v.tex0 = t.extend(0.0))
            }
            AttributeData::VertexColor(vals) => {
                set_attribute1(verts, vals, |v, t| v.vertex_color = t)
            }
            AttributeData::WeightIndex(vals) => {
                // TODO: What does the second index component do?
                set_attribute1(verts, vals, |v, t| v.weight_index = t[0] as u32)
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
) -> (shader::model::bind_groups::BindGroup1, wgpu::Buffer) {
    let buffer = device.create_uniform_buffer(
        "per group buffer",
        &crate::shader::model::PerGroup {
            enable_skinning: uvec4(
                matches!(skeleton, Some(skeleton) if !skeleton.bones.is_empty()) as u32,
                0,
                0,
                0,
            ),
            animated_transforms: [Mat4::IDENTITY; 256],
            animated_transforms_inv_transpose: [Mat4::IDENTITY; 256],
        },
    );

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

fn per_mesh_bind_group(
    device: &wgpu::Device,
    buffers: &xc3_model::vertex::ModelBuffers,
    mesh: &xc3_model::Mesh,
    material: &Material,
    weights: Option<&xc3_model::skinning::Weights>,
    bone_names: Option<&[String]>,
) -> shader::model::bind_groups::BindGroup3 {
    // TODO: Fix weight indexing calculations.
    let start = buffers
        .weights
        .as_ref()
        .map(|weights| {
            weights.weight_groups.weights_start_index(
                mesh.flags2.into(),
                mesh.lod_item_index,
                material.pipeline_key.pass_type,
            )
        })
        .unwrap_or_default();

    let per_mesh = device.create_uniform_buffer(
        "per mesh buffer",
        &crate::shader::model::PerMesh {
            weight_group_indices: uvec4(start as u32, 0, 0, 0),
        },
    );

    // TODO: How to correctly handle a missing skeleton or weights?
    let skin_weights = weights.and_then(|w| {
        let skin_weights = w.weight_buffer(mesh.flags2.into())?;
        bone_names.map(|names| skin_weights.reindex_bones(names.to_vec()))
    });

    let skin_weight_count = skin_weights
        .as_ref()
        .map(|w| w.weights.len())
        .unwrap_or_default();

    for attribute in &buffers.vertex_buffers[mesh.vertex_buffer_index].attributes {
        if let AttributeData::WeightIndex(weight_indices) = attribute {
            let max_index = weight_indices.iter().map(|i| i[0]).max().unwrap() as usize;
            if max_index + start >= skin_weight_count {
                error!(
                "Weight index start {} and max weight index {} exceed weight count {} with {:?}",
                start, max_index, skin_weight_count,
                (mesh.flags2, mesh.lod_item_index, material.pipeline_key.pass_type)
            );
            }
        }
    }

    // Convert to u32 since WGSL lacks a vec4<u8> type.
    // This assumes the skinning shader code is skipped if anything is missing.
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

    // Bone indices and skin weights are technically part of the model buffers.
    // Each mesh selects a range of values based on weight lods.
    // Define skinning per mesh to avoid alignment requirements on buffer bindings.
    crate::shader::model::bind_groups::BindGroup3::from_bindings(
        device,
        crate::shader::model::bind_groups::BindGroupLayout3 {
            per_mesh: per_mesh.as_entire_buffer_binding(),
            // TODO: Is it worth caching skinning buffers based on flags and parameters?
            bone_indices: bone_indices.as_entire_buffer_binding(),
            skin_weights: skin_weights.as_entire_buffer_binding(),
        },
    )
}
