use std::collections::{BTreeMap, HashMap, HashSet};

use glam::{Mat4, Vec3, Vec4, uvec4};
use log::{error, info};
use rayon::prelude::*;
use wgpu::util::DeviceExt;
use xc3_model::{ImageTexture, MeshRenderFlags2, MeshRenderPass, vertex::AttributeData};

mod bounds;
mod vertex;

use crate::{
    CameraData, DeviceBufferExt, MonolibShaderTextures, QueueBufferExt,
    culling::is_within_frustum,
    material::{Material, create_material},
    model::{bounds::Bounds, vertex::ModelBuffers},
    pipeline::{ModelPipelineData, Output5Type, PipelineKey, model_pipeline},
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
    pub(crate) bone_animated_transforms: wgpu::Buffer,
    pub(crate) bone_count: usize,
    animated_transforms: wgpu::Buffer,
    animated_transforms_inv_transpose: wgpu::Buffer,

    // Cache pipelines by their creation parameters.
    pipelines: HashMap<PipelineKey, wgpu::RenderPipeline>,
}

// TODO: aabb tree for culling?
pub struct Models {
    pub models: Vec<Model>,
    index_to_materials: BTreeMap<usize, Material>,
    bounds: Bounds,

    // TODO: skinning?
    morph_controller_names: Vec<String>,
    animation_morph_names: Vec<String>,
}

pub struct Model {
    pub meshes: Vec<Mesh>,
    pub min_xyz: Vec3,
    pub max_xyz: Vec3,
    model_buffers_index: usize,
    instances: Instances,
}

pub struct Mesh {
    vertex_buffer_index: usize,
    index_buffer_index: usize,
    material_index: usize,
    flags2: MeshRenderFlags2,
    per_mesh: crate::shader::model::bind_groups::BindGroup3,
}

struct Instances {
    transforms: wgpu::Buffer,
    count: u32,
}

impl Models {
    #[allow(clippy::too_many_arguments)]
    fn from_models(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        models: &xc3_model::Models,
        buffers: &[xc3_model::vertex::ModelBuffers],
        pipelines: &mut HashSet<PipelineKey>,
        textures: &[wgpu::Texture],
        image_textures: &[ImageTexture],
        monolib_shader: &MonolibShaderTextures,
    ) -> Self {
        // In practice, weights are only used for wimdo files with one Models and one Model.
        // TODO: How to enforce this assumption?
        // Reindex to match the ordering defined in the current skeleton.
        let weights = buffers.first().and_then(|b| b.weights.as_ref());

        let morph_controller_names = models.morph_controller_names.clone();
        let animation_morph_names = models.animation_morph_names.clone();

        let bounds = Bounds::new(device, models.max_xyz, models.min_xyz, &Mat4::IDENTITY);

        let samplers: Vec<_> = models
            .samplers
            .iter()
            .map(|s| create_sampler(device, s))
            .collect();

        // TODO: Should instances be empty for character models instead of a single identity transform?
        let is_instanced_static = models.models.iter().any(|m| {
            m.instances.len() > 1 || matches!(m.instances.first(), Some(t) if *t != Mat4::IDENTITY)
        });

        let mut index_to_materials = BTreeMap::new();

        let models = models
            .models
            .iter()
            .map(|model| {
                create_model(
                    device,
                    queue,
                    model,
                    models,
                    buffers,
                    weights,
                    &models.materials,
                    &mut index_to_materials,
                    image_textures,
                    monolib_shader,
                    pipelines,
                    textures,
                    &samplers,
                    is_instanced_static,
                )
            })
            .collect();

        // TODO: Store the samplers?
        Self {
            models,
            index_to_materials,
            morph_controller_names,
            animation_morph_names,
            bounds,
        }
    }

    pub fn bounds_min_max_xyz(&self) -> (Vec3, Vec3) {
        (self.bounds.min_xyz, self.bounds.max_xyz)
    }
}

fn remap_bone_indices(
    skinning_names: &[String],
    skeleton: Option<&xc3_model::Skeleton>,
) -> Vec<u32> {
    // Remap skinning bone indices to skeleton bone indices.
    skeleton
        .as_ref()
        .map(|skeleton| {
            skinning_names
                .iter()
                .map(|n| {
                    let i = skeleton.bones.iter().position(|b2| &b2.name == n);
                    i.unwrap_or_default() as u32
                })
                .collect()
        })
        .unwrap_or((0..256).collect())
}

impl ModelGroup {
    /// Draw each mesh for each model.
    pub fn draw<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        write_to_all_outputs: bool,
        pass_id: MeshRenderPass,
        camera: &CameraData,
        output5_type: Option<Output5Type>,
        models_index: Option<usize>,
        model_index: Option<usize>,
    ) {
        self.per_group.set(render_pass);

        // TODO: This should account for the instance transforms.
        // Assume the models AABB contains each model AABB.
        // This allows for better culling efficiency.
        if let Some(i) = models_index {
            self.draw_models(
                render_pass,
                &self.models[i],
                write_to_all_outputs,
                pass_id,
                output5_type,
                model_index,
            );
        } else {
            // TODO: Why does frustum culling not work for xcx de maps?
            for models in self.models.iter()
            // .filter(|m| is_within_frustum(m.bounds.min_xyz, m.bounds.max_xyz, camera))
            {
                self.draw_models(
                    render_pass,
                    models,
                    write_to_all_outputs,
                    pass_id,
                    output5_type,
                    model_index,
                );
            }
        }
    }

    fn draw_models<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        models: &'a Models,
        write_to_all_outputs: bool,
        pass_id: MeshRenderPass,
        output5_type: Option<Output5Type>,
        model_index: Option<usize>,
    ) {
        // TODO: cull aabb with instance transforms.
        if let Some(i) = model_index {
            self.draw_model(
                render_pass,
                models,
                &models.models[i],
                write_to_all_outputs,
                pass_id,
                output5_type,
            );
        } else {
            for model in &models.models {
                self.draw_model(
                    render_pass,
                    models,
                    model,
                    write_to_all_outputs,
                    pass_id,
                    output5_type,
                );
            }
        }
    }

    fn draw_model<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        models: &Models,
        model: &'a Model,
        write_to_all_outputs: bool,
        pass_id: MeshRenderPass,
        output5_type: Option<Output5Type>,
    ) {
        for mesh in &model.meshes {
            let material = &models.index_to_materials[&mesh.material_index];

            // TODO: Is there a flag that controls this?
            let is_outline = material.name.contains("outline");

            // TODO: Group these into passes with separate shaders for each pass?
            // TODO: The main pass is shared with outline, ope, and zpre?
            // TODO: How to handle transparency?
            // Only check the output5 type if needed.
            if (write_to_all_outputs == material.pipeline_key.write_to_all_outputs())
                && mesh.flags2.render_pass() == pass_id
                && output5_type
                    .map(|ty| material.pipeline_key.output5_type == ty)
                    .unwrap_or(true)
            {
                mesh.per_mesh.set(render_pass);

                // TODO: Depth prepass stored with material based on flags?
                // TODO: prepass should use an entirely different shader?
                // TODO: prepass can share a shader?

                let stencil_reference = material.pipeline_key.stencil_reference();
                render_pass.set_stencil_reference(stencil_reference);

                material.bind_group2.set(render_pass);

                // Assume meshes have either instance transforms or fur shells.
                let instance_count = material
                    .fur_shell_instance_count
                    .unwrap_or(model.instances.count);

                let is_instanced_static = material.pipeline_key.is_instanced_static;

                if let Some(key) = &material.prepass_pipeline_key {
                    // TODO: How to make sure the pipeline outputs match the render pass?
                    // TODO: Should the prepass be a separate render pass entirely?
                    let pipeline = &self.pipelines[key];
                    render_pass.set_pipeline(pipeline);

                    self.draw_mesh(
                        render_pass,
                        model,
                        mesh,
                        is_outline,
                        is_instanced_static,
                        instance_count,
                    );
                }

                // TODO: How to make sure the pipeline outputs match the render pass?
                let pipeline = &self.pipelines[&material.pipeline_key];
                render_pass.set_pipeline(pipeline);

                self.draw_mesh(
                    render_pass,
                    model,
                    mesh,
                    is_outline,
                    is_instanced_static,
                    instance_count,
                );
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
        render_pass: &mut wgpu::RenderPass<'a>,
        model: &'a Model,
        mesh: &Mesh,
        is_outline: bool,
        is_instanced_static: bool,
        instance_count: u32,
    ) {
        let buffers = &self.buffers[model.model_buffers_index];
        let vertex_buffers = &buffers.vertex_buffers[mesh.vertex_buffer_index];

        if let Some(morph_buffers) = &vertex_buffers.morph_buffers {
            render_pass.set_vertex_buffer(0, morph_buffers.vertex_buffer0.slice(..));
        } else if is_outline {
            render_pass.set_vertex_buffer(0, vertex_buffers.outline_vertex_buffer0.slice(..));
        } else {
            render_pass.set_vertex_buffer(0, vertex_buffers.vertex_buffer0.slice(..));
        }

        if is_outline {
            render_pass.set_vertex_buffer(1, vertex_buffers.outline_vertex_buffer1.slice(..));
        } else {
            render_pass.set_vertex_buffer(1, vertex_buffers.vertex_buffer1.slice(..));
        }

        if is_instanced_static {
            render_pass.set_vertex_buffer(2, model.instances.transforms.slice(..));
        }

        // TODO: Are all indices u16?
        let index_buffer = &buffers.index_buffers[mesh.index_buffer_index];
        render_pass.set_index_buffer(
            index_buffer.index_buffer.slice(..),
            wgpu::IndexFormat::Uint16,
        );

        render_pass.draw_indexed(0..index_buffer.vertex_index_count, 0, 0..instance_count);
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
                    let x = vertex_buffer.vertex_count.div_ceil(size_x);
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
            let frame = animation.current_frame(current_time_seconds);
            let animated_transforms = animation.skinning_transforms(skeleton, frame);
            queue.write_storage_data(&self.animated_transforms, &animated_transforms);

            let animated_transforms_inv_transpose: Vec<_> = animated_transforms
                .iter()
                .map(|t| t.inverse().transpose())
                .collect();
            queue.write_storage_data(
                &self.animated_transforms_inv_transpose,
                &animated_transforms_inv_transpose,
            );

            let bone_transforms: Vec<_> = animation
                .model_space_transforms(skeleton, animation.current_frame(current_time_seconds))
                .into_iter()
                .map(|t| t.to_matrix())
                .collect();
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

fn should_render_lod(lod: Option<usize>, models: &xc3_model::Models) -> bool {
    models
        .lod_data
        .as_ref()
        .map(|d| d.is_base_lod(lod))
        .unwrap_or(true)
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
#[allow(clippy::too_many_arguments)]
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
    // Disable vertex skinning if the model does not have bones or weights.
    let enable_skinning = matches!(skeleton, Some(skeleton) if !skeleton.bones.is_empty())
        && group.models.iter().any(|g| g.skinning.is_some())
        && group.buffers.iter().any(|b| b.weights.is_some());

    let skinning_names: Vec<_> = group
        .models
        .first()
        .and_then(|m| {
            m.skinning
                .as_ref()
                .map(|s| s.bones.iter().map(|b| b.name.clone()).collect())
        })
        .unwrap_or_default();

    // TODO: Create helper ext method in lib.rs?
    let bone_transforms: Vec<_> = skeleton
        .as_ref()
        .map(|s| {
            s.model_space_transforms()
                .into_iter()
                .map(|t| t.to_matrix())
                .collect()
        })
        .unwrap_or_default();
    let bone_count = skeleton.as_ref().map(|s| s.bones.len()).unwrap_or_default();

    let bone_animated_transforms = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Bone Animated Transforms"),
        contents: bytemuck::cast_slice(&bone_transforms),
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
    });

    let animated_transforms =
        device.create_storage_buffer("Animated Transforms", &vec![Mat4::IDENTITY; bone_count]);
    let animated_transforms_inv_transpose = device.create_storage_buffer(
        "Animated Transforms Inv Transpose",
        &vec![Mat4::IDENTITY; bone_count],
    );
    let per_group = per_group_bind_group(
        device,
        enable_skinning,
        &skinning_names,
        skeleton,
        &animated_transforms,
        &animated_transforms_inv_transpose,
    );

    let buffers = group
        .buffers
        .iter()
        .map(|buffers| ModelBuffers::from_buffers(device, buffers))
        .collect();

    let mut pipeline_keys = HashSet::new();

    let models = group
        .models
        .iter()
        .map(|models| {
            Models::from_models(
                device,
                queue,
                models,
                &group.buffers,
                &mut pipeline_keys,
                textures,
                image_textures,
                monolib_shader,
            )
        })
        .collect();

    let start = std::time::Instant::now();

    // Pipeline creation is slow and benefits from parallelism.
    // Repeat compiles will be much faster for drivers implementing pipeline caching.
    let pipelines: HashMap<_, _> = pipeline_keys
        .into_par_iter()
        .map(|key| {
            let pipeline = model_pipeline(device, pipeline_data, &key);
            (key, pipeline)
        })
        .collect();

    info!(
        "Created {} pipelines in {:?}",
        pipelines.len(),
        start.elapsed()
    );

    ModelGroup {
        models,
        buffers,
        per_group,
        skeleton: skeleton.cloned(),
        animated_transforms,
        animated_transforms_inv_transpose,
        bone_animated_transforms,
        bone_count,
        pipelines,
    }
}

#[tracing::instrument(skip_all)]
fn create_model(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    model: &xc3_model::Model,
    models: &xc3_model::Models,
    buffers: &[xc3_model::vertex::ModelBuffers],
    weights: Option<&xc3_model::skinning::Weights>,
    materials: &[xc3_model::material::Material],
    index_to_material: &mut BTreeMap<usize, Material>,
    image_textures: &[ImageTexture],
    monolib_shader: &MonolibShaderTextures,
    pipelines: &mut HashSet<PipelineKey>,
    textures: &[wgpu::Texture],
    samplers: &[wgpu::Sampler],
    is_instanced_static: bool,
) -> Model {
    let model_buffers = &buffers[model.model_buffers_index];

    let meshes = model
        .meshes
        .iter()
        .filter(|m| {
            should_render_lod(m.lod_item_index, models)
                && !materials[m.material_index].name.contains("_speff_")
        })
        .map(|mesh| {
            // Lazy load materials to compile fewer pipelines.
            let material = index_to_material
                .entry(mesh.material_index)
                .or_insert(create_material(
                    device,
                    queue,
                    pipelines,
                    &materials[mesh.material_index],
                    textures,
                    samplers,
                    image_textures,
                    monolib_shader,
                    is_instanced_static,
                ));

            Mesh {
                vertex_buffer_index: mesh.vertex_buffer_index,
                index_buffer_index: mesh.index_buffer_index,
                material_index: mesh.material_index,
                flags2: mesh.flags2,
                per_mesh: per_mesh_bind_group(device, model_buffers, mesh, material, weights),
            }
        })
        .collect();

    let transforms = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("instance transforms"),
        contents: bytemuck::cast_slice(&model.instances),
        usage: wgpu::BufferUsages::VERTEX,
    });
    let instances = Instances {
        transforms,
        count: model.instances.len() as u32,
    };

    Model {
        meshes,
        min_xyz: model.min_xyz,
        max_xyz: model.max_xyz,
        model_buffers_index: model.model_buffers_index,
        instances,
    }
}

fn per_group_bind_group(
    device: &wgpu::Device,
    enable_skinning: bool,
    skinning_names: &[String],
    skeleton: Option<&xc3_model::Skeleton>,
    animated_transforms: &wgpu::Buffer,
    animated_transforms_inv_transpose: &wgpu::Buffer,
) -> shader::model::bind_groups::BindGroup1 {
    let buffer = device.create_uniform_buffer(
        "per group buffer",
        &crate::shader::model::PerGroup {
            enable_skinning: uvec4(enable_skinning as u32, 0, 0, 0),
        },
    );

    let bone_indices_remap = remap_bone_indices(skinning_names, skeleton);

    let remap_buffer =
        device.create_storage_buffer("bone indices remap buffer", &bone_indices_remap);

    crate::shader::model::bind_groups::BindGroup1::from_bindings(
        device,
        crate::shader::model::bind_groups::BindGroupLayout1 {
            per_group: buffer.as_entire_buffer_binding(),
            animated_transforms: animated_transforms.as_entire_buffer_binding(),
            animated_transforms_inv_transpose: animated_transforms_inv_transpose
                .as_entire_buffer_binding(),
            bone_indices_remap: remap_buffer.as_entire_buffer_binding(),
        },
    )
}

fn per_mesh_bind_group(
    device: &wgpu::Device,
    buffers: &xc3_model::vertex::ModelBuffers,
    mesh: &xc3_model::Mesh,
    material: &Material,
    weights: Option<&xc3_model::skinning::Weights>,
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

    let skin_weights = weights.and_then(|w| w.weight_buffer(mesh.flags2.into()));

    let skin_weight_count = skin_weights
        .as_ref()
        .map(|w| w.weights.len())
        .unwrap_or_default();

    for attribute in &buffers.vertex_buffers[mesh.vertex_buffer_index].attributes {
        if let AttributeData::WeightIndex(weight_indices) = attribute {
            let max_index = weight_indices
                .iter()
                .map(|i| i[0])
                .max()
                .unwrap_or_default() as usize;
            if max_index + start >= skin_weight_count {
                error!(
                    "Weight index start {} and max weight index {} exceed weight count {} with {:?}",
                    start,
                    max_index,
                    skin_weight_count,
                    (
                        mesh.flags2,
                        mesh.lod_item_index,
                        material.pipeline_key.pass_type
                    )
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
