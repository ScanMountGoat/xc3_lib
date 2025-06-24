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
    shader::model::TEXTURE_SAMPLER_COUNT,
    shadergen::ShaderWgsl,
    texture::{default_black_3d_texture, default_black_cube_texture, default_black_texture},
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
    let default_2d =
        default_black_texture(device, queue).create_view(&wgpu::TextureViewDescriptor::default());
    let default_3d =
        default_black_3d_texture(device, queue).create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D3),
            ..Default::default()
        });
    let default_cube =
        default_black_cube_texture(device, queue).create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::Cube),
            ..Default::default()
        });

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

    let wgsl = ShaderWgsl::new(
        &material_assignments,
        material.alpha_test.as_ref(),
        &mut name_to_index,
    );

    let mut material_textures: [Option<_>; TEXTURE_SAMPLER_COUNT as usize] =
        std::array::from_fn(|_| None);

    for (name, i) in &name_to_index {
        if let Some(texture) = assign_texture(material, textures, monolib_shader, name) {
            if let Some(material_texture) = material_textures.get_mut(*i) {
                *material_texture = Some(texture);
            }
        } else {
            error!("Unable to assign {name} for {:?}", &material.name);
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

    // Use a storage buffer since wgpu doesn't allow binding arrays and uniform buffers in a bind group.
    let per_material = device.create_storage_buffer(
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

    let texture_views = material_textures.map(|t| {
        t.map(|t| {
            t.create_view(&wgpu::TextureViewDescriptor {
                dimension: if t.dimension() == wgpu::TextureDimension::D3 {
                    Some(wgpu::TextureViewDimension::D3)
                } else if t.dimension() == wgpu::TextureDimension::D2
                    && t.depth_or_array_layers() == 6
                {
                    Some(wgpu::TextureViewDimension::Cube)
                } else {
                    Some(wgpu::TextureViewDimension::D2)
                },
                ..Default::default()
            })
        })
    });

    // TODO: better way of handling this?
    let texture_array = texture_view_array(
        &material_textures,
        &texture_views,
        |t| t.dimension() == wgpu::TextureDimension::D2 && t.depth_or_array_layers() == 1,
        &default_2d,
    );
    let texture_array_3d = texture_view_array(
        &material_textures,
        &texture_views,
        |t| t.dimension() == wgpu::TextureDimension::D3,
        &default_3d,
    );
    let texture_array_cube = texture_view_array(
        &material_textures,
        &texture_views,
        |t| t.dimension() == wgpu::TextureDimension::D2 && t.depth_or_array_layers() == 6,
        &default_cube,
    );

    let sampler_array = std::array::from_fn(|i| {
        material_sampler(material, samplers, i).unwrap_or(&default_sampler)
    });

    // Bind all available textures and samplers.
    // Texture selection happens within generated shader code.
    // Any unused shader code will likely be removed during shader compilation.
    let bind_group2 = crate::shader::model::bind_groups::BindGroup2::from_bindings(
        device,
        crate::shader::model::bind_groups::BindGroupLayout2 {
            textures: &texture_array,
            textures_d3: &texture_array_3d,
            textures_cube: &texture_array_cube,
            samplers: &sampler_array,
            // TODO: Move alpha test to a separate pass?
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
        wgsl,
    };
    pipelines.insert(pipeline_key.clone());

    Material {
        name: material.name.clone(),
        bind_group2,
        pipeline_key,
        fur_shell_instance_count: material.fur_params.as_ref().map(|p| p.instance_count),
    }
}

fn texture_view_array<'a, const N: usize, F: Fn(&wgpu::Texture) -> bool>(
    textures: &[Option<&wgpu::Texture>],
    texture_views: &'a [Option<wgpu::TextureView>],
    check: F,
    default: &'a wgpu::TextureView,
) -> [&'a wgpu::TextureView; N] {
    std::array::from_fn(|i| {
        textures[i]
            .as_ref()
            .and_then(|t| {
                if check(t) {
                    texture_views[i].as_ref()
                } else {
                    None
                }
            })
            .unwrap_or(default)
    })
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
    match material_texture_index(name) {
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
