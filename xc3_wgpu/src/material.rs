use std::collections::HashSet;

use glam::{ivec2, ivec4, uvec2, uvec4, vec2, vec3, vec4, UVec2, Vec2, Vec3, Vec4};
use indexmap::IndexMap;
use log::{error, warn};
use smol_str::SmolStr;
use xc3_model::{
    material::{ChannelAssignment, OutputAssignment, OutputAssignments, TextureAssignment},
    ImageTexture, IndexMapExt,
};

use crate::{
    pipeline::{Output5Type, PipelineKey},
    shadergen::{generate_alpha_test_wgsl, generate_assignment_wgsl, generate_layering_wgsl},
    texture::create_default_black_texture,
    DeviceBufferExt, MonolibShaderTextures,
};

#[derive(Debug)]
pub(crate) struct Material {
    pub name: String,
    pub bind_group2: crate::shader::model::bind_groups::BindGroup2,

    // The material flags may require a separate pipeline per material.
    // We only store a key here to allow caching.
    pub pipeline_key: PipelineKey,

    pub fur_shell_instance_count: Option<u32>,
}

#[allow(clippy::too_many_arguments)]
#[tracing::instrument(skip_all)]
pub fn materials(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    pipelines: &mut HashSet<PipelineKey>,
    materials: &[xc3_model::material::Material],
    textures: &[wgpu::Texture],
    samplers: &[wgpu::Sampler],
    image_textures: &[ImageTexture],
    monolib_shader: &MonolibShaderTextures,
    is_instanced_static: bool,
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
            // Assign material textures by index to make GPU debugging easier.
            // TODO: Match the ordering in the actual in game shader using technique?
            let mut name_to_index = (0..material.textures.len())
                .map(|i| (format!("s{i}").into(), i))
                .collect();

            let mut name_to_info = IndexMap::new();
            let material_assignments = material.output_assignments(image_textures);
            let assignments =
                output_assignments(&material_assignments, &mut name_to_index, &mut name_to_info);

            // Alpha textures might not be used in normal shaders.
            if let Some(a) = &material.alpha_test {
                name_to_index.entry_index(format!("s{}", a.texture_index).into());
            }

            let output_assignments_wgsl = material_assignments
                .assignments
                .iter()
                .map(|a| generate_assignment_wgsl(&a, &mut name_to_index))
                .collect();

            let output_layers_wgsl = material_assignments
                .assignments
                .iter()
                .map(|a| generate_layering_wgsl(&a, &mut name_to_index))
                .collect();

            // Generate empty code if alpha testing is disabled.
            let alpha_test_wgsl = material
                .alpha_test
                .as_ref()
                .map(|a| generate_alpha_test_wgsl(a, &mut name_to_index))
                .unwrap_or_default();

            let mut texture_views: [Option<_>; 10] = std::array::from_fn(|_| None);

            // TODO: Is it ok to switch on the texcoord for each channel lookup?
            // TODO: can a texture be used with more than one scale?
            // TODO: Include this logic with xc3_model?
            let mut texture_info = [crate::shader::model::TextureInfo {
                texcoord_index: 0,
                is_bc4_single_channel: 0,
                parallax_sampler_indices: ivec2(-1, -1),
                parallax_channel_indices: UVec2::ZERO,
                parallax_default_values: Vec2::ZERO,
                parallax_ratio: 0.0,
                transform: [Vec4::X, Vec4::Y],
            }; 10];

            // Find the scale parameters for any textures assigned above.
            // TODO: Is there a more efficient way of doing this?
            // TODO: xc1 needs more than 10 textures?
            for (name, i) in &name_to_info {
                if let Some(index) = name_to_index.get(name.as_str()) {
                    if let Some(info) = texture_info.get_mut(*index) {
                        *info = *i;
                    }
                }
            }

            for (name, i) in &name_to_index {
                if let Some(texture) = assign_texture(material, textures, monolib_shader, name) {
                    if *i < texture_views.len() {
                        texture_views[*i] = Some(texture.create_view(&Default::default()));
                        // TODO: Better way of doing this?
                        if texture.format() == wgpu::TextureFormat::Bc4RUnorm {
                            texture_info[*i].is_bc4_single_channel = 1;
                        }
                    }
                } else {
                    warn!("Missing texture for {name:?}. Assigning default black texture.");
                }
            }

            // Use similar calculated parameter values as in game vertex shaders.
            let fur_params = material
                .fur_params
                .as_ref()
                .map(|p| crate::shader::model::FurShellParams {
                    xyz_offset: vec3(0.0, p.y_offset * p.shell_width, 0.0),
                    instance_count: p.instance_count as f32,
                    shell_width: 1.0 / (p.instance_count as f32) * p.shell_width,
                    alpha: (1.0 - p.alpha) / p.instance_count as f32,
                })
                .unwrap_or(crate::shader::model::FurShellParams {
                    xyz_offset: Vec3::ZERO,
                    instance_count: 0.0,
                    shell_width: 0.0,
                    alpha: 0.0,
                });

            // TODO: What is a good default outline width?
            let outline_width =
                value_channel_assignment(material_assignments.outline_width.as_ref())
                    .unwrap_or(0.005);

            // TODO: This is normally done using a depth prepass.
            // TODO: Is it ok to combine the prepass alpha in the main pass like this?
            let per_material = device.create_uniform_buffer(
                "PerMaterial",
                &[crate::shader::model::PerMaterial {
                    assignments,
                    texture_info,
                    outline_width,
                    fur_params,
                    alpha_test_ref: material.alpha_test_ref,
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
            // TODO: Is any frag shader with 7 outputs using specular?
            // TODO: Something in the wimdo matches up with shader outputs?
            // TODO: unk12-14 in material render flags?
            let output5_type = if material_assignments.mat_id().is_some() {
                if material.render_flags.specular() {
                    Output5Type::Specular
                } else {
                    // TODO: This case isn't always accurate.
                    Output5Type::Emission
                }
            } else {
                // TODO: Set better defaults for xcx models?
                Output5Type::Specular
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
                is_instanced_static,
                output_assignments_wgsl,
                output_layers_wgsl,
                alpha_test_wgsl,
            };
            pipelines.insert(pipeline_key.clone());

            Material {
                name: material.name.clone(),
                bind_group2,
                pipeline_key,
                fur_shell_instance_count: material.fur_params.as_ref().map(|p| p.instance_count),
            }
        })
        .collect();

    materials
}

fn output_assignments(
    assignments: &OutputAssignments,
    name_to_index: &mut IndexMap<SmolStr, usize>,
    name_to_info: &mut IndexMap<SmolStr, crate::shader::model::TextureInfo>,
) -> [crate::shader::model::OutputAssignment; 6] {
    // Each output channel may have a different input sampler and channel.
    [0, 1, 2, 3, 4, 5].map(|i| {
        let assignment = &assignments.assignments[i];

        crate::shader::model::OutputAssignment {
            samplers: sampler_assignment(assignment, name_to_index, name_to_info, 0),
            default_value: output_default(assignment, i),
        }
    })
}

fn sampler_assignment(
    a: &OutputAssignment,
    name_to_index: &mut IndexMap<SmolStr, usize>,
    name_to_info: &mut IndexMap<SmolStr, crate::shader::model::TextureInfo>,
    layer_index: usize,
) -> crate::shader::model::SamplerAssignment {
    let (x, y, z, w) = layer_channel_assignments(a, layer_index);

    let (s0, c0) = texture_channel(x, name_to_index, name_to_info, 'x').unwrap_or((-1, 0));
    let (s1, c1) = texture_channel(y, name_to_index, name_to_info, 'y').unwrap_or((-1, 1));
    let (s2, c2) = texture_channel(z, name_to_index, name_to_info, 'z').unwrap_or((-1, 2));
    let (s3, c3) = texture_channel(w, name_to_index, name_to_info, 'w').unwrap_or((-1, 3));

    crate::shader::model::SamplerAssignment {
        sampler_indices: ivec4(s0, s1, s2, s3),
        channel_indices: uvec4(c0, c1, c2, c3),
    }
}

fn layer_channel_assignments(
    a: &OutputAssignment,
    layer_index: usize,
) -> (
    Option<&ChannelAssignment>,
    Option<&ChannelAssignment>,
    Option<&ChannelAssignment>,
    Option<&ChannelAssignment>,
) {
    let (x, y, z, w) = if layer_index == 0 {
        (a.x.as_ref(), a.y.as_ref(), a.z.as_ref(), a.w.as_ref())
    } else {
        (
            a.x_layers
                .get(layer_index - 1)
                .and_then(|l| l.value.as_ref()),
            a.y_layers
                .get(layer_index - 1)
                .and_then(|l| l.value.as_ref()),
            a.z_layers
                .get(layer_index - 1)
                .and_then(|l| l.value.as_ref()),
            a.w_layers
                .get(layer_index - 1)
                .and_then(|l| l.value.as_ref()),
        )
    };
    (x, y, z, w)
}

fn texture_channel(
    assignment: Option<&ChannelAssignment>,
    name_to_index: &mut IndexMap<SmolStr, usize>,
    name_to_info: &mut IndexMap<SmolStr, crate::shader::model::TextureInfo>,
    channel: char,
) -> Option<(i32, u32)> {
    if let Some(ChannelAssignment::Texture(texture)) = assignment {
        let TextureAssignment {
            name,
            channels,
            texcoord_name,
            texcoord_transforms,
            parallax,
        } = texture;

        let (ps_x, ps_y, pc_x, pc_y, p_default_x, p_default_y, parallax_ratio) =
            if let Some(parallax) = parallax {
                let (s_x, c_x) =
                    texture_channel(Some(&parallax.mask_a), name_to_index, name_to_info, 'x')
                        .unzip();

                let (s_y, c_y) =
                    texture_channel(Some(&parallax.mask_b), name_to_index, name_to_info, 'x')
                        .unzip();

                let default_x =
                    value_channel_assignment(Some(&parallax.mask_a)).unwrap_or_default();
                let default_y =
                    value_channel_assignment(Some(&parallax.mask_b)).unwrap_or_default();

                (
                    s_x.unwrap_or(-1),
                    s_y.unwrap_or(-1),
                    c_x.unwrap_or_default(),
                    c_y.unwrap_or_default(),
                    default_x,
                    default_y,
                    parallax.ratio,
                )
            } else {
                (-1, -1, 0, 0, 0.0, 0.0, 0.0)
            };

        name_to_info.insert(
            name.clone(),
            crate::shader::model::TextureInfo {
                is_bc4_single_channel: 0,
                texcoord_index: texcoord_name
                    .as_ref()
                    .and_then(|s| texcoord_index(s))
                    .unwrap_or_default(),
                parallax_sampler_indices: ivec2(ps_x, ps_y),
                parallax_channel_indices: uvec2(pc_x, pc_y),
                parallax_default_values: vec2(p_default_x, p_default_y),
                parallax_ratio,
                transform: texcoord_transforms.unwrap_or((Vec4::X, Vec4::Y)).into(),
            },
        );

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

fn texcoord_index(name: &str) -> Option<u32> {
    // vTex1 -> 1
    name.strip_prefix("vTex")?.parse().ok()
}

fn create_bit_info(
    mat_id: u32,
    mat_flag: bool,
    hatching_flag: bool,
    specular_col: bool,
    ssr: bool,
) -> f32 {
    // Adapted from xeno3/chr/ch/ch11021013.pcsmt, shd00036, createBitInfo,
    let n_val = mat_id
        | ((ssr as u32) << 3)
        | ((specular_col as u32) << 4)
        | ((mat_flag as u32) << 5)
        | ((hatching_flag as u32) << 6);
    (n_val as f32 + 0.1) / 255.0
}

fn output_default(a: &OutputAssignment, i: usize) -> Vec4 {
    // TODO: Create a special ID for unrecognized materials instead of toon?
    let etc_flags = create_bit_info(2, false, false, true, false);

    // Choose defaults that have as close to no effect as possible.
    // TODO: Make a struct for this instead?
    // TODO: Move these defaults to xc3_model?
    let output_defaults: [Vec4; 6] = [
        Vec4::ONE,
        Vec4::new(0.0, 0.0, 0.0, etc_flags),
        Vec4::new(0.5, 0.5, 1.0, 0.0),
        Vec4::ZERO,
        Vec4::new(1.0, 1.0, 1.0, 0.0),
        Vec4::ZERO,
    ];

    vec4(
        value_channel_assignment(a.x.as_ref()).unwrap_or(output_defaults[i][0]),
        value_channel_assignment(a.y.as_ref()).unwrap_or(output_defaults[i][1]),
        value_channel_assignment(a.z.as_ref()).unwrap_or(output_defaults[i][2]),
        value_channel_assignment(a.w.as_ref()).unwrap_or(output_defaults[i][3]),
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
    material: &xc3_model::material::Material,
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
    material: &xc3_model::material::Material,
    samplers: &'a [wgpu::Sampler],
    index: usize,
) -> Option<&'a wgpu::Sampler> {
    // TODO: Why is this sometimes out of range for XC2 maps?
    material
        .textures
        .get(index)
        .and_then(|texture| samplers.get(texture.sampler_index))
}
