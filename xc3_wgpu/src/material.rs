use std::collections::HashMap;

use glam::{ivec4, uvec4, vec4, IVec4, UVec4, Vec4};
use indexmap::IndexMap;
use log::{error, warn};
use smol_str::SmolStr;
use xc3_model::{
    texture_layer_assignment, ChannelAssignment, ImageTexture, IndexMapExt, OutputAssignment,
    OutputAssignments, TextureAssignment,
};

use crate::{
    pipeline::{model_pipeline, ModelPipelineData, Output5Type, PipelineKey},
    texture::create_default_black_texture,
    DeviceBufferExt, MonolibShaderTextures,
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

// TODO: Create a special ID for unrecognized materials?
const MAT_ID_TOON: f32 = 2.0 / 255.0;

// Choose defaults that have as close to no effect as possible.
// TODO: Make a struct for this instead?
// TODO: Move these defaults to xc3_model?
const OUTPUT_DEFAULTS: [Vec4; 6] = [
    Vec4::ONE,
    Vec4::new(0.0, 0.0, 0.0, MAT_ID_TOON),
    Vec4::new(0.5, 0.5, 1.0, 0.0),
    Vec4::ZERO,
    Vec4::new(1.0, 1.0, 1.0, 0.0),
    Vec4::ZERO,
];

#[allow(clippy::too_many_arguments)]
#[tracing::instrument(skip_all)]
pub fn materials(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    pipelines: &mut HashMap<PipelineKey, wgpu::RenderPipeline>,
    pipeline_data: &ModelPipelineData,
    materials: &[xc3_model::Material],
    textures: &[wgpu::Texture],
    samplers: &[wgpu::Sampler],
    image_textures: &[ImageTexture],
    monolib_shader: &MonolibShaderTextures,
) -> Vec<Material> {
    // TODO: Is there a better way to handle missing textures?
    let default_black = create_default_black_texture(device, queue)
        .create_view(&wgpu::TextureViewDescriptor::default());

    let default_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        address_mode_u: wgpu::AddressMode::Repeat,
        address_mode_v: wgpu::AddressMode::Repeat,
        min_filter: wgpu::FilterMode::Linear,
        mag_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    });

    let materials = materials
        .iter()
        .map(|material| {
            let mut name_to_index = IndexMap::new();
            let mut name_to_transforms = IndexMap::new();

            let assignments = material.output_assignments(image_textures);
            let sampler_assignments =
                sampler_assignments(&assignments, &mut name_to_index, &mut name_to_transforms);
            let attribute_assignments = attribute_assignments(&assignments);
            let output_defaults = output_default_assignments(&assignments);

            // Alpha textures might not be used in normal shaders.
            if let Some(a) = &material.alpha_test {
                name_to_index.entry_index(format!("s{}", a.texture_index).into());
            }

            // It's possible that some material textures had no assignment.
            // Assign remaining textures by index to make GPU debugging easier.
            for i in 0..material.textures.len() {
                name_to_index.entry_index(format!("s{i}").into());
            }

            let mut texture_views: [Option<_>; 10] = std::array::from_fn(|_| None);
            let mut is_single_channel = [UVec4::ZERO; 10];
            for (name, i) in &name_to_index {
                if let Some(texture) = assign_texture(material, textures, monolib_shader, name) {
                    if *i < texture_views.len() {
                        texture_views[*i] = Some(texture.create_view(&Default::default()));
                        // TODO: Better way of doing this?
                        if texture.format() == wgpu::TextureFormat::Bc4RUnorm {
                            is_single_channel[*i] = uvec4(1, 0, 0, 0);
                        }
                    }
                } else {
                    warn!("Missing texture for {name:?}. Assigning default black texture.");
                }
            }

            // TODO: Is it ok to switch on the texcoord for each channel lookup?
            // TODO: can a texture be used with more than one scale?
            // TODO: Include this logic with xc3_model?
            let mut texture_transforms = [[Vec4::X, Vec4::Y]; 10];

            // Find the scale parameters for any textures assigned above.
            // TODO: Don't assume these are all scaled from a single vTex0 input attribute.
            // TODO: Is there a more efficient way of doing this?
            // TODO: xc1 needs more than 10 textures?
            for (name, (u, v)) in &name_to_transforms {
                if let Some(index) = name_to_index.get(name.as_str()) {
                    if let Some(transform) = texture_transforms.get_mut(*index) {
                        *transform = [*u, *v];
                    }
                }
            }

            // TODO: This is normally done using a depth prepass.
            // TODO: Is it ok to combine the prepass alpha in the main pass like this?
            let per_material = device.create_uniform_buffer(
                "PerMaterial",
                &[crate::shader::model::PerMaterial {
                    mat_color: material.parameters.mat_color.into(),
                    sampler_assignments,
                    attribute_assignments,
                    output_defaults,
                    texture_transforms,
                    alpha_test_texture: {
                        let (texture_index, channel_index) = material
                            .alpha_test
                            .as_ref()
                            .map(|a| {
                                let name: SmolStr = format!("s{}", a.texture_index).into();
                                (name_to_index[&name] as i32, a.channel_index as i32)
                            })
                            .unwrap_or((-1, 3));
                        IVec4::new(texture_index, channel_index, 0, 0)
                    },
                    // TODO: what is this ref value?
                    alpha_test_ref: Vec4::splat(
                        material.alpha_test.as_ref().map(|_| 0.5).unwrap_or(1.0),
                    ),
                    is_single_channel,
                }],
            );

            // Bind all available textures and samplers.
            // Texture selection happens within the shader itself.
            // This simulates having a unique shader for each material.
            // Reducing unique pipelines greatly improves loading times and enables compiled bindings.
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

            // Toon and hair materials seem to always use specular.
            // TODO: Is there a more reliable way to check this?
            let output5_type = match assignments.mat_id() {
                Some(mat_id) => {
                    if mat_id == 2 || mat_id == 5 {
                        Output5Type::Specular
                    } else {
                        Output5Type::Emission
                    }
                }
                // TODO: Set better defaults for xcx models?
                None => Output5Type::Specular,
            };

            // TODO: How to make sure the pipeline outputs match the render pass?
            // Each material only goes in exactly one pass?
            // TODO: Is it redundant to also store the unk type?
            // TODO: Find a more accurate way to detect outline shaders.
            let pipeline_key = PipelineKey {
                pass_type: material.pass_type,
                flags: material.state_flags,
                is_outline: material.name.ends_with("_outline"),
                output5_type,
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

    materials
}

// TODO: Test cases for this
fn sampler_assignments(
    assignments: &OutputAssignments,
    name_to_index: &mut IndexMap<SmolStr, usize>,
    name_to_transforms: &mut IndexMap<SmolStr, (Vec4, Vec4)>,
) -> [crate::shader::model::SamplerAssignment; 6] {
    // Each output channel may have a different input sampler and channel.
    [0, 1, 2, 3, 4, 5].map(|i| {
        sampler_assignment(
            &assignments.assignments[i],
            name_to_index,
            name_to_transforms,
        )
    })
}

fn sampler_assignment(
    a: &OutputAssignment,
    name_to_index: &mut IndexMap<SmolStr, usize>,
    name_to_transforms: &mut IndexMap<SmolStr, (Vec4, Vec4)>,
) -> crate::shader::model::SamplerAssignment {
    let (s00, c00) = texture_channel(a.x.as_ref(), name_to_index, name_to_transforms, 'x', false)
        .unwrap_or((-1, 0));
    let (s10, c10) = texture_channel(a.y.as_ref(), name_to_index, name_to_transforms, 'y', false)
        .unwrap_or((-1, 1));
    let (s20, c20) = texture_channel(a.z.as_ref(), name_to_index, name_to_transforms, 'z', false)
        .unwrap_or((-1, 2));
    let (s30, c30) = texture_channel(a.w.as_ref(), name_to_index, name_to_transforms, 'w', false)
        .unwrap_or((-1, 3));

    let (s01, c01) = texture_channel(a.x.as_ref(), name_to_index, name_to_transforms, 'x', true)
        .unwrap_or((-1, 0));
    let (s11, c11) = texture_channel(a.y.as_ref(), name_to_index, name_to_transforms, 'y', true)
        .unwrap_or((-1, 1));
    let (s21, c21) = texture_channel(a.z.as_ref(), name_to_index, name_to_transforms, 'z', true)
        .unwrap_or((-1, 2));
    let (s31, c31) = texture_channel(a.w.as_ref(), name_to_index, name_to_transforms, 'w', true)
        .unwrap_or((-1, 3));

    crate::shader::model::SamplerAssignment {
        sampler_indices: ivec4(s00, s10, s20, s30),
        channel_indices: uvec4(c00, c10, c20, c30),
        sampler_indices2: ivec4(s01, s11, s21, s31),
        channel_indices2: uvec4(c01, c11, c21, c31),
    }
}

fn texture_channel(
    assignment: Option<&ChannelAssignment>,
    name_to_index: &mut IndexMap<SmolStr, usize>,
    name_to_transforms: &mut IndexMap<SmolStr, (Vec4, Vec4)>,
    channel: char,
    is_second_layer: bool,
) -> Option<(i32, u32)> {
    if let Some(ChannelAssignment::Textures(textures)) = assignment {
        let texture = texture_layer_assignment(textures, channel, is_second_layer)?;

        let TextureAssignment {
            name,
            channels,
            texcoord_transforms,
            ..
        } = texture;

        // TODO: Also store the texcoord name?
        if let Some(transforms) = texcoord_transforms {
            name_to_transforms.insert(name.clone(), *transforms);
        }

        // TODO: how to handle empty input channels?
        let channel_index = if channels.contains(channel) || channels.is_empty() {
            "xyzw".find(channel).unwrap()
        } else {
            "xyzw".find(channels.chars().next().unwrap()).unwrap()
        };
        // TODO: Should this ever return -1?
        let index = name_to_index.entry_index(name.clone());
        Some((index as i32, channel_index as u32))
    } else {
        None
    }
}

fn attribute_assignments(
    assignments: &OutputAssignments,
) -> [crate::shader::model::AttributeAssignment; 6] {
    // Each output channel may have a different input sampler and channel.
    [0, 1, 2, 3, 4, 5].map(|i| attribute_assignment(&assignments.assignments[i]))
}

fn attribute_assignment(a: &OutputAssignment) -> crate::shader::model::AttributeAssignment {
    let c0 = attribute_channel_assignment(a.x.as_ref()).unwrap_or(-1);
    let c1 = attribute_channel_assignment(a.y.as_ref()).unwrap_or(-1);
    let c2 = attribute_channel_assignment(a.z.as_ref()).unwrap_or(-1);
    let c3 = attribute_channel_assignment(a.w.as_ref()).unwrap_or(-1);

    crate::shader::model::AttributeAssignment {
        channel_indices: ivec4(c0, c1, c2, c3),
    }
}

fn attribute_channel_assignment(assignment: Option<&ChannelAssignment>) -> Option<i32> {
    if let Some(ChannelAssignment::Attribute {
        name,
        channel_index,
    }) = assignment
    {
        // TODO: Support attributes other than vColor.
        if name == "vColor" {
            Some(*channel_index as i32)
        } else {
            None
        }
    } else {
        None
    }
}

fn output_default_assignments(assignments: &OutputAssignments) -> [Vec4; 6] {
    [0, 1, 2, 3, 4, 5].map(|i| output_default(&assignments.assignments[i], i))
}

fn output_default(a: &OutputAssignment, i: usize) -> Vec4 {
    vec4(
        value_channel_assignment(a.x.as_ref()).unwrap_or(OUTPUT_DEFAULTS[i][0]),
        value_channel_assignment(a.y.as_ref()).unwrap_or(OUTPUT_DEFAULTS[i][1]),
        value_channel_assignment(a.z.as_ref()).unwrap_or(OUTPUT_DEFAULTS[i][2]),
        value_channel_assignment(a.w.as_ref()).unwrap_or(OUTPUT_DEFAULTS[i][3]),
    )
}

fn value_channel_assignment(assignment: Option<&ChannelAssignment>) -> Option<f32> {
    if let Some(ChannelAssignment::Value(f)) = assignment {
        Some(*f)
    } else {
        None
    }
}

fn assign_texture<'a>(
    material: &xc3_model::Material,
    textures: &'a [wgpu::Texture],
    monolib_shader: &'a MonolibShaderTextures,
    name: &str,
) -> Option<&'a wgpu::Texture> {
    let texture = match material_texture_index(name) {
        Some(texture_index) => {
            // Search the material textures like "s0" or "s3".
            // TODO: Why is this sometimes out of range for XC2 maps?
            let image_texture_index = material.textures.get(texture_index)?.image_texture_index;
            textures.get(image_texture_index)
        }
        None => {
            // Search global textures from monolib/shader like "gTResidentTex44".
            monolib_shader.global_texture(name)
        }
    }?;

    // TODO: How to handle 3D textures and cube maps within the shader?
    if texture.dimension() == wgpu::TextureDimension::D2 && texture.depth_or_array_layers() == 1 {
        Some(texture)
    } else {
        error!(
            "Expected 2D texture but found dimension {:?} and {} layers.",
            texture.dimension(),
            texture.depth_or_array_layers()
        );
        None
    }
}

fn material_texture_index(sampler_name: &str) -> Option<usize> {
    // Convert names like "s3" to index 3.
    // Materials always use this naming convention in the shader.
    // Xenoblade 1 DE uses up to 14 material samplers.
    sampler_name.strip_prefix('s')?.parse().ok()
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
