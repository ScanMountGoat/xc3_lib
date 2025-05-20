use std::collections::HashSet;

use glam::{uvec4, vec3, vec4, Vec3, Vec4};
use indexmap::IndexMap;
use log::{error, warn};
use xc3_model::{
    material::assignments::{Assignment, AssignmentValue, OutputAssignments},
    ImageTexture, IndexMapExt,
};

use crate::{
    pipeline::{Output5Type, PipelineKey},
    shadergen::{
        generate_alpha_test_wgsl, generate_assignments_wgsl, generate_layering_wgsl,
        generate_normal_intensity_wgsl,
    },
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
pub fn create_material(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    pipelines: &mut HashSet<PipelineKey>,
    material: &xc3_model::material::Material,
    textures: &[wgpu::Texture],
    samplers: &[wgpu::Sampler],
    image_textures: &[ImageTexture],
    monolib_shader: &MonolibShaderTextures,
    is_instanced_static: bool,
) -> Material {
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

    // Assign material textures by index to make GPU debugging easier.
    // TODO: Match the ordering in the actual in game shader using technique?
    let mut name_to_index: IndexMap<_, _> = (0..material.textures.len())
        .map(|i| (format!("s{i}").into(), i))
        .collect();

    let material_assignments = material.output_assignments(image_textures);
    let assignments = output_assignments(&material_assignments);

    // Alpha textures might not be used in normal shaders.
    if let Some(a) = &material.alpha_test {
        name_to_index.entry_index(format!("s{}", a.texture_index).into());
    }

    let assignments_wgsl = generate_assignments_wgsl(&material_assignments, &mut name_to_index);
    let output_layers_wgsl = generate_layering_wgsl(&material_assignments);

    // Generate empty code if alpha testing is disabled.
    let alpha_test_wgsl = material
        .alpha_test
        .as_ref()
        .map(|a| generate_alpha_test_wgsl(a, &mut name_to_index))
        .unwrap_or_default();

    let normal_intensity_wgsl = material_assignments
        .normal_intensity
        .as_ref()
        .map(|i| generate_normal_intensity_wgsl(*i))
        .unwrap_or_default();

    let mut texture_views: [Option<_>; 16] = std::array::from_fn(|_| None);

    for (name, i) in &name_to_index {
        if let Some(texture) = assign_texture(material, textures, monolib_shader, name) {
            if *i < texture_views.len() {
                texture_views[*i] = Some(texture.create_view(&Default::default()));
            }
        } else {
            error!("Unable to assign texture {name:?} for {:?}", &material.name);
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
        value_channel_assignment(material_assignments.outline_width.as_ref()).unwrap_or(0.005);

    // TODO: This is normally done using a depth prepass.
    // TODO: Is it ok to combine the prepass alpha in the main pass like this?
    let per_material = device.create_uniform_buffer(
        // TODO: include model name?
        &format!(
            "PerMaterial {:?} shd{:04}",
            &material.name, material.technique_index
        ),
        &[crate::shader::model::PerMaterial {
            assignments,
            outline_width,
            fur_params,
            alpha_test_ref: material.alpha_test_ref,
        }],
    );

    // Bind all available textures and samplers.
    // Texture selection happens within generated shader code.
    // Any unused shader code will likely be removed during shader compilation.
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
            s10: texture_views[10].as_ref().unwrap_or(&default_black),
            s11: texture_views[11].as_ref().unwrap_or(&default_black),
            s12: texture_views[12].as_ref().unwrap_or(&default_black),
            s13: texture_views[13].as_ref().unwrap_or(&default_black),
            s14: texture_views[14].as_ref().unwrap_or(&default_black),
            s15: texture_views[15].as_ref().unwrap_or(&default_black),
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
            s10_sampler: material_sampler(material, samplers, 10).unwrap_or(&default_sampler),
            s11_sampler: material_sampler(material, samplers, 11).unwrap_or(&default_sampler),
            s12_sampler: material_sampler(material, samplers, 12).unwrap_or(&default_sampler),
            s13_sampler: material_sampler(material, samplers, 13).unwrap_or(&default_sampler),
            s14_sampler: material_sampler(material, samplers, 14).unwrap_or(&default_sampler),
            // TODO: Move alpha test to a separate pass?
            // s15_sampler: material_sampler(material, samplers, 15).unwrap_or(&default_sampler),
            alpha_test_sampler: material
                .alpha_test
                .as_ref()
                .map(|a| a.sampler_index)
                .and_then(|i| samplers.get(i))
                .unwrap_or(&default_sampler),
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
        assignments_wgsl,
        output_layers_wgsl,
        alpha_test_wgsl,
        normal_intensity_wgsl,
    };
    pipelines.insert(pipeline_key.clone());

    Material {
        name: material.name.clone(),
        bind_group2,
        pipeline_key,
        fur_shell_instance_count: material.fur_params.as_ref().map(|p| p.instance_count),
    }
}

fn output_assignments(
    assignments: &OutputAssignments,
) -> [crate::shader::model::OutputAssignment; 6] {
    [0, 1, 2, 3, 4, 5].map(|i| {
        let assignment = &assignments.output_assignments[i];

        crate::shader::model::OutputAssignment {
            has_channels: uvec4(
                has_value(&assignments.assignments, assignment.x) as u32,
                has_value(&assignments.assignments, assignment.y) as u32,
                has_value(&assignments.assignments, assignment.z) as u32,
                has_value(&assignments.assignments, assignment.w) as u32,
            ),
            default_value: output_default(i),
        }
    })
}

fn has_value(assignments: &[Assignment], i: Option<usize>) -> bool {
    if let Some(i) = i {
        match &assignments[i] {
            Assignment::Value(c) => c.is_some(),
            Assignment::Func { args, .. } => args.iter().any(|a| has_value(assignments, Some(*a))),
        }
    } else {
        false
    }
}

fn value_channel_assignment(assignment: Option<&AssignmentValue>) -> Option<f32> {
    if let Some(AssignmentValue::Float(f)) = assignment {
        Some(f.0)
    } else {
        None
    }
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

fn output_default(i: usize) -> Vec4 {
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
        output_defaults[i][0],
        output_defaults[i][1],
        output_defaults[i][2],
        output_defaults[i][3],
    )
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
