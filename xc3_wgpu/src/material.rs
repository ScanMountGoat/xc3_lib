use std::{collections::HashMap, path::Path};

use glam::{ivec4, uvec4};
use wgpu::util::DeviceExt;
use xc3_lib::{
    map::FoliageMaterials,
    mxmd::{MaterialFlags, Materials, ShaderUnkType},
};

use crate::{
    pipeline::{model_pipeline, ModelPipelineData, PipelineKey},
    texture::create_default_black_texture,
};

// TODO: Don't make this public outside the crate?
// TODO: Store material parameter values.
pub struct Material {
    pub name: String,
    pub bind_group1: crate::shader::model::bind_groups::BindGroup1,
    pub bind_group2: crate::shader::model::bind_groups::BindGroup2,

    // The material flags may require a separate pipeline per material.
    // We only store a key here to allow caching.
    pub pipeline_key: PipelineKey,

    pub texture_count: usize,
}

pub fn materials(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    pipeline_data: &ModelPipelineData,
    materials: &Materials,
    textures: &[wgpu::TextureView],
    spch: Option<&xc3_shader::gbuffer_database::Spch>,
) -> (Vec<Material>, HashMap<PipelineKey, wgpu::RenderPipeline>) {
    // TODO: Is there a better way to handle missing textures?
    // TODO: Is it worth creating a separate shaders for each material?
    // TODO: Just use booleans to indicate which textures are present?
    // TODO: How to handle some inputs using buffer parameters instead of textures?
    let default_black = create_default_black_texture(device, queue)
        .create_view(&wgpu::TextureViewDescriptor::default());

    // TODO: Does each texture in the material have its own sampler parameters?
    let default_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        address_mode_u: wgpu::AddressMode::Repeat,
        address_mode_v: wgpu::AddressMode::Repeat,
        min_filter: wgpu::FilterMode::Linear,
        mag_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    });

    let mut pipelines = HashMap::new();

    let materials = materials
        .materials
        .iter()
        .map(|material| {
            // Bind all available textures and samplers.
            // Texture selection happens within the shader itself.
            // This simulates having a unique shader for each material.
            let bind_group1 = crate::shader::model::bind_groups::BindGroup1::from_bindings(
                device,
                crate::shader::model::bind_groups::BindGroupLayout1 {
                    s0: material_texture(material, textures, 0).unwrap_or(&default_black),
                    s1: material_texture(material, textures, 1).unwrap_or(&default_black),
                    s2: material_texture(material, textures, 2).unwrap_or(&default_black),
                    s3: material_texture(material, textures, 3).unwrap_or(&default_black),
                    s4: material_texture(material, textures, 4).unwrap_or(&default_black),
                    s5: material_texture(material, textures, 5).unwrap_or(&default_black),
                    s6: material_texture(material, textures, 6).unwrap_or(&default_black),
                    s7: material_texture(material, textures, 7).unwrap_or(&default_black),
                    s8: material_texture(material, textures, 8).unwrap_or(&default_black),
                    s9: material_texture(material, textures, 9).unwrap_or(&default_black),
                    shared_sampler: &default_sampler,
                },
            );

            // TODO: How to choose between the two fragment shaders?
            let program_index = material.shader_programs[0].program_index as usize;
            let shader = spch
                .and_then(|spch| spch.programs.get(program_index))
                .map(|program| &program.shaders[0]);

            // TODO: Default assignments?
            let assignments = shader
                .map(gbuffer_assignments)
                .unwrap_or_else(default_gbuffer_assignments);

            let gbuffer_assignments =
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("GBuffer Assignments"),
                    contents: bytemuck::cast_slice(&assignments),
                    usage: wgpu::BufferUsages::UNIFORM,
                });

            let bind_group2 = crate::shader::model::bind_groups::BindGroup2::from_bindings(
                device,
                crate::shader::model::bind_groups::BindGroupLayout2 {
                    gbuffer_assignments: gbuffer_assignments.as_entire_buffer_binding(),
                },
            );

            // TODO: How to make sure the pipeline outputs match the render pass?
            // Each material only goes in exactly one pass?
            // TODO: Is it redundant to also store the unk type?
            let pipeline_key = PipelineKey {
                write_to_all_outputs: material.shader_programs[0].unk_type == ShaderUnkType::Unk0,
                flags: material.flags,
            };
            pipelines
                .entry(pipeline_key)
                .or_insert_with(|| model_pipeline(device, pipeline_data, &pipeline_key));

            Material {
                name: material.name.clone(),
                bind_group1,
                bind_group2,
                pipeline_key,
                texture_count: material.textures.len(),
            }
        })
        .collect();

    // TODO: is this the best place to cache pipelines?
    (materials, pipelines)
}

pub fn foliage_materials(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    pipeline_data: &ModelPipelineData,
    materials: &FoliageMaterials,
    textures: &[wgpu::TextureView],
    model_path: &str,
    shader_database: &xc3_shader::gbuffer_database::GBufferDatabase,
) -> (Vec<Material>, HashMap<PipelineKey, wgpu::RenderPipeline>) {
    // TODO: Is there a better way to handle missing textures?
    // TODO: Is it worth creating a separate shaders for each material?
    // TODO: Just use booleans to indicate which textures are present?
    // TODO: How to handle some inputs using buffer parameters instead of textures?
    let default_black = create_default_black_texture(device, queue)
        .create_view(&wgpu::TextureViewDescriptor::default());

    // TODO: Does each texture in the material have its own sampler parameters?
    let default_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        address_mode_u: wgpu::AddressMode::Repeat,
        address_mode_v: wgpu::AddressMode::Repeat,
        min_filter: wgpu::FilterMode::Linear,
        mag_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    });

    let model_folder = Path::new(model_path)
        .with_extension("")
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string();

    let mut pipelines = HashMap::new();

    let materials = materials
        .materials
        .iter()
        .map(|material| {
            // TODO: Where are the textures?
            let bind_group1 = crate::shader::model::bind_groups::BindGroup1::from_bindings(
                device,
                crate::shader::model::bind_groups::BindGroupLayout1 {
                    s0: &textures[0],
                    s1: &default_black,
                    s2: &default_black,
                    s3: &default_black,
                    s4: &default_black,
                    s5: &default_black,
                    s6: &default_black,
                    s7: &default_black,
                    s8: &default_black,
                    s9: &default_black,
                    shared_sampler: &default_sampler,
                },
            );

            // TODO: Foliage shaders?
            let shader = None;

            // TODO: Default assignments?
            let assignments = shader
                .map(gbuffer_assignments)
                .unwrap_or_else(default_gbuffer_assignments);

            let gbuffer_assignments =
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("GBuffer Assignments"),
                    contents: bytemuck::cast_slice(&assignments),
                    usage: wgpu::BufferUsages::UNIFORM,
                });

            let bind_group2 = crate::shader::model::bind_groups::BindGroup2::from_bindings(
                device,
                crate::shader::model::bind_groups::BindGroupLayout2 {
                    gbuffer_assignments: gbuffer_assignments.as_entire_buffer_binding(),
                },
            );

            // TODO: Flags?
            let pipeline_key = PipelineKey {
                write_to_all_outputs: true,
                flags: MaterialFlags {
                    flag0: 0,
                    blend_state: xc3_lib::mxmd::BlendState::Disabled,
                    cull_mode: xc3_lib::mxmd::CullMode::Disabled,
                    flag3: 0,
                    stencil_state1: xc3_lib::mxmd::StencilState1::Always,
                    stencil_state2: xc3_lib::mxmd::StencilState2::Disabled,
                    depth_func: xc3_lib::mxmd::DepthFunc::LessEqual,
                    flag7: 0,
                },
            };
            pipelines
                .entry(pipeline_key)
                .or_insert_with(|| model_pipeline(device, pipeline_data, &pipeline_key));

            Material {
                name: material.name.clone(),
                bind_group1,
                bind_group2,
                pipeline_key,
                texture_count: 0,
            }
        })
        .collect();

    // TODO: is this the best place to cache pipelines?
    (materials, pipelines)
}

// TODO: submodule for this?
// TODO: Store this information already parsed in the JSON?
// TODO: Test cases for this
fn gbuffer_assignments(
    shader: &xc3_shader::gbuffer_database::Shader,
) -> Vec<crate::shader::model::GBufferAssignment> {
    (0..=5)
        .map(|i| {
            // Each output channel may have a different input sampler and channel.
            // TODO: How to properly handle missing assignment information?
            // TODO: How to encode constants and buffer values?
            let (s0, c0) = shader
                .material_channel_assignment(i, 'x')
                .map(|(s, c)| (s as i32, c))
                .unwrap_or((-1, 0));

            let (s1, c1) = shader
                .material_channel_assignment(i, 'y')
                .map(|(s, c)| (s as i32, c))
                .unwrap_or((-1, 0));

            let (s2, c2) = shader
                .material_channel_assignment(i, 'z')
                .map(|(s, c)| (s as i32, c))
                .unwrap_or((-1, 0));

            let (s3, c3) = shader
                .material_channel_assignment(i, 'w')
                .map(|(s, c)| (s as i32, c))
                .unwrap_or((-1, 0));

            crate::shader::model::GBufferAssignment {
                sampler_indices: ivec4(s0, s1, s2, s3),
                channel_indices: uvec4(c0, c1, c2, c3),
            }
        })
        .collect()
}

// TODO: Does this need to be public?
pub fn load_database<P: AsRef<Path>>(path: P) -> xc3_shader::gbuffer_database::GBufferDatabase {
    let json = std::fs::read_to_string(path).unwrap();
    serde_json::from_str(&json).unwrap()
}

fn material_texture<'a>(
    material: &xc3_lib::mxmd::Material,
    textures: &'a [wgpu::TextureView],
    index: usize,
) -> Option<&'a wgpu::TextureView> {
    material
        .textures
        .get(index)
        .map(|texture| &textures[texture.texture_index as usize])
}

fn default_gbuffer_assignments() -> Vec<crate::shader::model::GBufferAssignment> {
    // We can only assume that the first texture is probably albedo.
    vec![
        crate::shader::model::GBufferAssignment {
            sampler_indices: ivec4(0, 0, 0, 0),
            channel_indices: uvec4(0, 1, 2, 3),
        },
        crate::shader::model::GBufferAssignment {
            sampler_indices: ivec4(-1, -1, -1, -1),
            channel_indices: uvec4(0, 1, 2, 3),
        },
        crate::shader::model::GBufferAssignment {
            sampler_indices: ivec4(-1, -1, -1, -1),
            channel_indices: uvec4(0, 1, 2, 3),
        },
        crate::shader::model::GBufferAssignment {
            sampler_indices: ivec4(-1, -1, -1, -1),
            channel_indices: uvec4(0, 1, 2, 3),
        },
        crate::shader::model::GBufferAssignment {
            sampler_indices: ivec4(-1, -1, -1, -1),
            channel_indices: uvec4(0, 1, 2, 3),
        },
        crate::shader::model::GBufferAssignment {
            sampler_indices: ivec4(-1, -1, -1, -1),
            channel_indices: uvec4(0, 1, 2, 3),
        },
    ]
}
