use std::collections::HashMap;

use glam::{ivec4, uvec4, IVec4, Vec4};
use wgpu::util::DeviceExt;
use xc3_lib::{
    map::FoliageMaterials,
    mxmd::{MaterialFlags, ShaderUnkType},
};

use crate::{
    pipeline::{model_pipeline, ModelPipelineData, PipelineKey},
    texture::create_default_black_texture,
};

// TODO: Don't make this public outside the crate?
// TODO: Store material parameter values.
pub struct Material {
    pub name: String,
    pub bind_group2: crate::shader::model::bind_groups::BindGroup2,

    // The material flags may require a separate pipeline per material.
    // We only store a key here to allow caching.
    pub pipeline_key: PipelineKey,

    pub texture_count: usize,
}

// Choose defaults that have as close to no effect as possible.
const GBUFFER_DEFAULTS: [Vec4; 6] = [
    Vec4::ONE,
    Vec4::ZERO,
    Vec4::new(0.5, 0.5, 1.0, 0.0),
    Vec4::ZERO,
    Vec4::ZERO,
    Vec4::ZERO,
];

// We can only assume that the first texture is probably albedo.
const DEFAULT_GBUFFER_ASSIGNMENTS: [crate::shader::model::GBufferAssignment; 6] = [
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
];

#[tracing::instrument]
pub fn materials(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    pipeline_data: &ModelPipelineData,
    materials: &[xc3_model::Material],
    textures: &[wgpu::TextureView],
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
        .iter()
        .map(|material| {
            let gbuffer_assignments = material
                .shader
                .as_ref()
                .map(parse_gbuffer_assignments)
                .unwrap_or(DEFAULT_GBUFFER_ASSIGNMENTS);

            let gbuffer_defaults = material
                .shader
                .as_ref()
                .map(|s| parse_gbuffer_params_consts(s, &material.parameters))
                .unwrap_or(GBUFFER_DEFAULTS);

            let per_material = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("PerMaterial"),
                contents: bytemuck::cast_slice(&[crate::shader::model::PerMaterial {
                    mat_color: material.parameters.mat_color.into(),
                    gbuffer_assignments,
                    gbuffer_defaults,
                    alpha_test_texture: IVec4::splat(
                        material.alpha_test_texture_index.map(|i| i as i32).unwrap_or(-1),
                    ),
                }]),
                usage: wgpu::BufferUsages::UNIFORM,
            });

            // Bind all available textures and samplers.
            // Texture selection happens within the shader itself.
            // This simulates having a unique shader for each material.
            let bind_group2 = crate::shader::model::bind_groups::BindGroup2::from_bindings(
                device,
                crate::shader::model::bind_groups::BindGroupLayout2 {
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
                    per_material: per_material.as_entire_buffer_binding(),
                },
            );

            // TODO: How to make sure the pipeline outputs match the render pass?
            // Each material only goes in exactly one pass?
            // TODO: Is it redundant to also store the unk type?
            let pipeline_key = PipelineKey {
                write_to_all_outputs: material.unk_type == ShaderUnkType::Unk0,
                flags: material.flags,
            };
            pipelines
                .entry(pipeline_key)
                .or_insert_with(|| model_pipeline(device, pipeline_data, &pipeline_key));

            Material {
                name: material.name.clone(),
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
            // TODO: Foliage shaders?
            let shader = None;

            // TODO: Handle constants in defaults?
            let gbuffer_assignments = shader
                .map(parse_gbuffer_assignments)
                .unwrap_or(DEFAULT_GBUFFER_ASSIGNMENTS);

            let gbuffer_defaults = shader
                .map(|s| parse_gbuffer_params_consts(s, &Default::default()))
                .unwrap_or(GBUFFER_DEFAULTS);

            let per_material = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("PerMaterial"),
                contents: bytemuck::cast_slice(&[crate::shader::model::PerMaterial {
                    mat_color: Vec4::ONE,
                    gbuffer_assignments,
                    gbuffer_defaults,
                    alpha_test_texture: IVec4::splat(-1),
                }]),
                usage: wgpu::BufferUsages::UNIFORM,
            });

            let bind_group2 = crate::shader::model::bind_groups::BindGroup2::from_bindings(
                device,
                crate::shader::model::bind_groups::BindGroupLayout2 {
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
                    per_material: per_material.as_entire_buffer_binding(),
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
                bind_group2,
                pipeline_key,
                texture_count: 0,
            }
        })
        .collect();

    (materials, pipelines)
}

// TODO: submodule for this?
// TODO: Store this information already parsed in the JSON?
// TODO: Test cases for this
fn parse_gbuffer_assignments(
    shader: &xc3_shader::gbuffer_database::Shader,
) -> [crate::shader::model::GBufferAssignment; 6] {
    [0, 1, 2, 3, 4, 5].map(|i| {
        // Each output channel may have a different input sampler and channel.
        // TODO: How to properly handle missing assignment information?
        // TODO: How to encode constants and buffer values?
        let (s0, c0) = shader
            .sampler_channel_index(i, 'x')
            .map(|(s, c)| (s as i32, c))
            .unwrap_or((-1, 0));

        let (s1, c1) = shader
            .sampler_channel_index(i, 'y')
            .map(|(s, c)| (s as i32, c))
            .unwrap_or((-1, 0));

        let (s2, c2) = shader
            .sampler_channel_index(i, 'z')
            .map(|(s, c)| (s as i32, c))
            .unwrap_or((-1, 0));

        let (s3, c3) = shader
            .sampler_channel_index(i, 'w')
            .map(|(s, c)| (s as i32, c))
            .unwrap_or((-1, 0));

        crate::shader::model::GBufferAssignment {
            sampler_indices: ivec4(s0, s1, s2, s3),
            channel_indices: uvec4(c0, c1, c2, c3),
        }
    })
}

fn parse_gbuffer_params_consts(
    shader: &xc3_shader::gbuffer_database::Shader,
    parameters: &xc3_model::MaterialParameters,
) -> [Vec4; 6] {
    // TODO: Update the database to also handle parameters?
    [0, 1, 2, 3, 4, 5].map(|i| {
        Vec4::new(
            param_const_or_default(shader, parameters, i, 0),
            param_const_or_default(shader, parameters, i, 1),
            param_const_or_default(shader, parameters, i, 2),
            param_const_or_default(shader, parameters, i, 3),
        )
    })
}

// TODO: Tests for this?
fn param_const_or_default(
    shader: &xc3_shader::gbuffer_database::Shader,
    parameters: &xc3_model::MaterialParameters,
    i: usize,
    c: usize,
) -> f32 {
    let channel = ['x', 'y', 'z', 'w'][c];
    shader
        .buffer_parameter(i, channel)
        .and_then(|p| extract_parameter(p, parameters))
        .or_else(|| shader.float_constant(i, channel))
        .unwrap_or(GBUFFER_DEFAULTS[i][c])
}

fn extract_parameter(
    p: xc3_shader::gbuffer_database::BufferParameter,
    parameters: &xc3_model::MaterialParameters,
) -> Option<f32> {
    // TODO: Also check for U_Mate?
    let c = "xyzw".find(p.channel).unwrap();
    match p.uniform.as_str() {
        "gWrkFl4" => Some(parameters.work_float4.as_ref()?.get(p.index)?[c]),
        "gWrkCol" => Some(parameters.work_color.as_ref()?.get(p.index)?[c]),
        _ => None,
    }
}

fn material_texture<'a>(
    material: &xc3_model::Material,
    textures: &'a [wgpu::TextureView],
    index: usize,
) -> Option<&'a wgpu::TextureView> {
    material
        .textures
        .get(index)
        .map(|texture| &textures[texture.image_texture_index])
}
