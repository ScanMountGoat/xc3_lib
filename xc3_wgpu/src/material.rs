use std::path::Path;

use glam::{ivec4, uvec4};
use wgpu::util::DeviceExt;
use xc3_lib::mxmd::{Materials, ShaderUnkType};

use crate::{
    pipeline::{model_pipeline, model_transparent_pipeline},
    texture::create_default_black_texture,
};

// TODO: Don't make this public outside the crate?
// TODO: Store material parameter values.
pub struct Material {
    pub name: String,
    pub bind_group1: crate::shader::model::bind_groups::BindGroup1,
    pub bind_group2: crate::shader::model::bind_groups::BindGroup2,

    // The material flags require a separate pipeline per material.
    pub pipeline: wgpu::RenderPipeline,

    pub texture_count: usize,
    pub unk_type: xc3_lib::mxmd::ShaderUnkType,
}

pub fn materials(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    materials: &Materials,
    textures: &[Option<wgpu::TextureView>],
    cached_textures: &[(String, wgpu::TextureView)],
    model_path: &str,
    shader_database: &xc3_shader::gbuffer_database::GBufferDatabase,
) -> Vec<Material> {
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

    // TODO: Make this a map instead of a vec?
    let shaders = shader_database.files.get(&model_folder).map(|f| &f.shaders);

    materials
        .materials
        .elements
        .iter()
        .map(|material| {
            let texture_views = load_material_textures(material, textures, cached_textures);

            // Bind all available textures and samplers.
            // Texture selection happens within the shader itself.
            // This simulates having a unique shader for each material.
            let bind_group1 = crate::shader::model::bind_groups::BindGroup1::from_bindings(
                device,
                crate::shader::model::bind_groups::BindGroupLayout1 {
                    s0: texture_views.get(0).unwrap_or(&&default_black),
                    s1: texture_views.get(1).unwrap_or(&&default_black),
                    s2: texture_views.get(2).unwrap_or(&&default_black),
                    s3: texture_views.get(3).unwrap_or(&&default_black),
                    s4: texture_views.get(4).unwrap_or(&&default_black),
                    s5: texture_views.get(5).unwrap_or(&&default_black),
                    s6: texture_views.get(6).unwrap_or(&&default_black),
                    s7: texture_views.get(7).unwrap_or(&&default_black),
                    s8: texture_views.get(8).unwrap_or(&&default_black),
                    s9: texture_views.get(9).unwrap_or(&&default_black),
                    shared_sampler: &default_sampler,
                },
            );

            // TODO: Is it better to store these in a hashmap instead of a vec?
            // TODO: Do all shaders have the naming convention "shd{program index}"?
            // TODO: Store a list of shaders for each index/name?
            // TODO: How to choose between the two fragment shaders?
            let shader_name = format!(
                "shd{:0>4}_FS0.glsl",
                material.shader_programs[0].program_index
            );
            let shader = shaders.and_then(|shaders| shaders.get(&shader_name));
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
            // TODO: Cache the compiled shaders for faster loading times.
            let pipeline = if material.shader_programs[0].unk_type == ShaderUnkType::Unk0 {
                model_pipeline(device)
            } else {
                model_transparent_pipeline(device, &material.flags)
            };

            Material {
                name: material.name.clone(),
                bind_group1,
                bind_group2,
                pipeline,
                texture_count: material.textures.len(),
                unk_type: material.shader_programs[0].unk_type,
            }
        })
        .collect()
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
            let (s0, c0) = channel_assignment(shader, i, 'x').unwrap_or((-1, 0));
            let (s1, c1) = channel_assignment(shader, i, 'y').unwrap_or((-1, 0));
            let (s2, c2) = channel_assignment(shader, i, 'z').unwrap_or((-1, 0));
            let (s3, c3) = channel_assignment(shader, i, 'w').unwrap_or((-1, 0));

            crate::shader::model::GBufferAssignment {
                sampler_indices: ivec4(s0, s1, s2, s3),
                channel_indices: uvec4(c0, c1, c2, c3),
            }
        })
        .collect()
}

fn channel_assignment(
    shader: &xc3_shader::gbuffer_database::Shader,
    index: usize,
    channel: char,
) -> Option<(i32, u32)> {
    let output = format!("out_attr{index}.{channel}");

    // Find the first material referenced sampler like "s0" or "s1".
    let (sampler_index, channels) =
        shader
            .output_dependencies
            .get(&output)?
            .iter()
            .find_map(|sampler_name| {
                let (sampler, channels) = sampler_name.split_once('.')?;
                let sampler_index = material_sampler_index(sampler)?;

                Some((sampler_index, channels))
            })?;

    // Textures may have multiple accessed channels like normal maps.
    // First check if the current channel is used.
    // TODO: Does this always work as intended?
    let c = if channels.contains(channel) {
        channel
    } else {
        channels.chars().next().unwrap()
    };
    let channel_index = "xyzw".find(c).unwrap() as u32;
    Some((sampler_index, channel_index))
}

fn material_sampler_index(sampler: &str) -> Option<i32> {
    match sampler {
        "s0" => Some(0),
        "s1" => Some(1),
        "s2" => Some(2),
        "s3" => Some(3),
        "s4" => Some(4),
        "s5" => Some(5),
        "s6" => Some(6),
        "s7" => Some(7),
        "s8" => Some(8),
        "s9" => Some(9),
        // TODO: How to handle this case?
        _ => None,
    }
}

// TODO: Does this need to be public?
pub fn load_database<P: AsRef<Path>>(path: P) -> xc3_shader::gbuffer_database::GBufferDatabase {
    let json = std::fs::read_to_string(path).unwrap();
    serde_json::from_str(&json).unwrap()
}

fn load_material_textures<'a>(
    material: &xc3_lib::mxmd::Material,
    textures: &'a [Option<wgpu::TextureView>],
    cached_textures: &'a [(String, wgpu::TextureView)],
) -> Vec<&'a wgpu::TextureView> {
    material
        .textures
        .iter()
        .map(|t| {
            textures
                .get(t.texture_index as usize)
                .and_then(|t| t.as_ref())
                .unwrap_or_else(|| &cached_textures[t.texture_index as usize].1)
        })
        .collect()
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
