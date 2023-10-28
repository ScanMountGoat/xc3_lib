use std::collections::HashMap;

use glam::{ivec4, uvec4, IVec4, UVec4, Vec4};
use log::error;
use wgpu::util::DeviceExt;

use crate::{
    pipeline::{model_pipeline, ModelPipelineData, PipelineKey},
    texture::create_default_black_texture,
};

// TODO: Don't make this public outside the crate?
// TODO: Store material parameter values.
#[derive(Debug)]
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
    textures: &[wgpu::Texture],
    samplers: &[wgpu::Sampler],
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

            let mut texture_views: [Option<_>; 10] = std::array::from_fn(|_| None);
            let mut is_single_channel = [UVec4::ZERO; 10];
            for i in 0..10 {
                if let Some(texture) = material_texture(material, textures, i) {
                    texture_views[i] = Some(texture.create_view(&Default::default()));
                    // TODO: Better way of doing this?
                    if texture.format() == wgpu::TextureFormat::Bc4RUnorm {
                        is_single_channel[i] = uvec4(1, 0, 0, 0);
                    }
                }
            }

            // TODO: This is normally done using a depth prepass.
            // TODO: Is it ok to combine the prepass alpha in the main pass like this?
            let per_material = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("PerMaterial"),
                contents: bytemuck::cast_slice(&[crate::shader::model::PerMaterial {
                    mat_color: material.parameters.mat_color.into(),
                    gbuffer_assignments,
                    gbuffer_defaults,
                    alpha_test_texture: {
                        let (texture_index, channel_index) = material
                            .alpha_test
                            .as_ref()
                            .map(|a| (a.texture_index as i32, a.channel_index as i32))
                            .unwrap_or((-1, 3));
                        IVec4::new(texture_index, channel_index, 0, 0)
                    },
                    alpha_test_ref: Vec4::splat(
                        material
                            .alpha_test
                            .as_ref()
                            .map(|a| a.ref_value)
                            .unwrap_or(1.0),
                    ),
                    is_single_channel,
                }]),
                usage: wgpu::BufferUsages::UNIFORM,
            });

            // Bind all available textures and samplers.
            // Texture selection happens within the shader itself.
            // This simulates having a unique shader for each material.
            let bind_group2 = crate::shader::model::bind_groups::BindGroup2::from_bindings(
                device,
                crate::shader::model::bind_groups::BindGroupLayout2 {
                    s0: texture_views[0].as_ref().unwrap_or(&default_black),
                    s1: texture_views[1].as_ref().unwrap_or(&default_black),
                    s2: texture_views[2].as_ref().unwrap_or(&default_black),
                    s3: texture_views[3].as_ref().unwrap_or(&default_black),
                    s4: texture_views[4].as_ref().unwrap_or(&default_black),
                    s5: texture_views[5].as_ref().unwrap_or(&default_black),
                    s6: texture_views[6].as_ref().unwrap_or(&default_black),
                    s7: texture_views[7].as_ref().unwrap_or(&default_black),
                    s8: texture_views[8].as_ref().unwrap_or(&default_black),
                    s9: texture_views[9].as_ref().unwrap_or(&default_black),
                    s0_sampler: material_sampler(material, samplers, 0).unwrap_or(&default_sampler),
                    s1_sampler: material_sampler(material, samplers, 1).unwrap_or(&default_sampler),
                    s2_sampler: material_sampler(material, samplers, 2).unwrap_or(&default_sampler),
                    s3_sampler: material_sampler(material, samplers, 3).unwrap_or(&default_sampler),
                    s4_sampler: material_sampler(material, samplers, 4).unwrap_or(&default_sampler),
                    s5_sampler: material_sampler(material, samplers, 5).unwrap_or(&default_sampler),
                    s6_sampler: material_sampler(material, samplers, 6).unwrap_or(&default_sampler),
                    s7_sampler: material_sampler(material, samplers, 7).unwrap_or(&default_sampler),
                    s8_sampler: material_sampler(material, samplers, 8).unwrap_or(&default_sampler),
                    s9_sampler: material_sampler(material, samplers, 9).unwrap_or(&default_sampler),
                    per_material: per_material.as_entire_buffer_binding(),
                },
            );

            // TODO: How to make sure the pipeline outputs match the render pass?
            // Each material only goes in exactly one pass?
            // TODO: Is it redundant to also store the unk type?
            let pipeline_key = PipelineKey {
                unk_type: material.unk_type,
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

// TODO: submodule for this?
// TODO: Store this information already parsed in the JSON?
// TODO: Test cases for this
fn parse_gbuffer_assignments(
    shader: &xc3_model::shader_database::Shader,
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
    shader: &xc3_model::shader_database::Shader,
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
    shader: &xc3_model::shader_database::Shader,
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
    p: xc3_model::shader_database::BufferParameter,
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
    textures: &'a [wgpu::Texture],
    index: usize,
) -> Option<&'a wgpu::Texture> {
    // TODO: Why is this sometimes out of range for XC2 maps?
    material
        .textures
        .get(index)
        .and_then(|texture| textures.get(texture.image_texture_index))
        .and_then(|texture| {
            // TODO: How to handle 3D textures within the shader?
            if texture.dimension() == wgpu::TextureDimension::D2 {
                Some(texture)
            } else {
                error!(
                    "Expected 2D texture but found dimension {:?}.",
                    texture.dimension()
                );
                None
            }
        })
}

fn material_sampler<'a>(
    material: &xc3_model::Material,
    samplers: &'a [wgpu::Sampler],
    index: usize,
) -> Option<&'a wgpu::Sampler> {
    // TODO: Why is this sometimes out of range for XC2 maps?
    material
        .textures
        .get(index)
        .and_then(|texture| samplers.get(texture.sampler_index))
}
