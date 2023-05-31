use std::{io::Cursor, path::Path};

use glam::uvec4;
use wgpu::util::DeviceExt;
use xc3_lib::{mibl::Mibl, mxmd::Mxmd, xbc1::Xbc1};

use crate::texture::{create_default_black_texture, create_texture};

pub struct Material {
    pub name: String,
    pub bind_group1: crate::shader::model::bind_groups::BindGroup1,
    pub bind_group2: crate::shader::model::bind_groups::BindGroup2,
}

pub fn materials(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    mxmd: &Mxmd,
    model_path: &str,
    shader_database: &[xc3_shader::gbuffer_database::File],
) -> Vec<Material> {
    // TODO: Is there a better way to handle missing textures?
    // TODO: Is it worth creating a separate shaders for each material?
    // TODO: Just use booleans to indicate which textures are present?
    // TODO: How to handle some inputs using materials instead of textures?
    let default_black = create_default_black_texture(device, queue)
        .create_view(&wgpu::TextureViewDescriptor::default());

    let default_sampler = device.create_sampler(&wgpu::SamplerDescriptor::default());

    let name = Path::new(model_path)
        .with_extension("")
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string();

    let shaders = &shader_database
        .iter()
        .find(|f| f.file == name)
        .unwrap()
        .shaders;

    // "chr/en/file.wismt" -> "chr/tex/nx/m"
    // TODO: Don't assume model_path is in the chr/ch or chr/en folders.
    let texture_folder = Path::new(model_path)
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tex")
        .join("nx")
        .join("m");

    mxmd.materials
        .materials
        .elements
        .iter()
        .map(|material| {
            // TODO: store wgpu texture instead?
            // TODO: How should these be cached?
            let texture_views: Vec<_> = material
                .textures
                .elements
                .iter()
                .map(|t| {
                    // TODO: Are textures always in the tex folder?
                    // TODO: Also load high res textures from nx/h?
                    // TODO: Why are the indices off by 1?
                    let name = &mxmd.textures.items.textures[t.texture_index as usize + 1].name;
                    let path = texture_folder.join(name).with_extension("wismt");

                    load_wismt_texture(path, device, queue)
                })
                .collect();

            let shader = &shaders[material.shader_programs.elements[0].program_index as usize];

            // Bind all available textures and samplers.
            // Texture selection happens within the shader itself.
            // This simulates having a unique shader for each material.
            // TODO: Macro for this?
            let bind_group1 = crate::shader::model::bind_groups::BindGroup1::from_bindings(
                device,
                crate::shader::model::bind_groups::BindGroupLayout1 {
                    s0: texture_views
                        .get(0)
                        .and_then(|s| s.as_ref())
                        .unwrap_or(&default_black),
                    s1: texture_views
                        .get(1)
                        .and_then(|s| s.as_ref())
                        .unwrap_or(&default_black),
                    s2: texture_views
                        .get(2)
                        .and_then(|s| s.as_ref())
                        .unwrap_or(&default_black),
                    s3: texture_views
                        .get(3)
                        .and_then(|s| s.as_ref())
                        .unwrap_or(&default_black),
                    s4: texture_views
                        .get(4)
                        .and_then(|s| s.as_ref())
                        .unwrap_or(&default_black),
                    s5: texture_views
                        .get(5)
                        .and_then(|s| s.as_ref())
                        .unwrap_or(&default_black),
                    shared_sampler: &default_sampler,
                },
            );

            let gbuffer_assignments =
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("GBuffer Assignments"),
                    contents: bytemuck::cast_slice(&gbuffer_assignments(shader)),
                    usage: wgpu::BufferUsages::UNIFORM,
                });

            let bind_group2 = crate::shader::model::bind_groups::BindGroup2::from_bindings(
                device,
                crate::shader::model::bind_groups::BindGroupLayout2 {
                    gbuffer_assignments: gbuffer_assignments.as_entire_buffer_binding(),
                },
            );

            Material {
                name: material.name.clone(),
                bind_group1,
                bind_group2,
            }
        })
        .collect()
}

// TODO: Store this information already parsed in the JSON?
// TODO: Test cases for this
fn gbuffer_assignments(
    shader: &xc3_shader::gbuffer_database::Shader,
) -> Vec<crate::shader::model::GBufferAssignment> {
    (0..3)
        .map(|i| {
            // Each output channel may have a different input sampler and channel.
            // TODO: How to properly handle missing assignment information?
            let (s0, c0) = channel_assignment(shader, i, 'x').unwrap_or_default();
            let (s1, c1) = channel_assignment(shader, i, 'y').unwrap_or_default();
            let (s2, c2) = channel_assignment(shader, i, 'z').unwrap_or_default();
            let (s3, c3) = channel_assignment(shader, i, 'w').unwrap_or_default();

            crate::shader::model::GBufferAssignment {
                sampler_indices: uvec4(s0, s1, s2, s3),
                channel_indices: uvec4(c0, c1, c2, c3),
            }
        })
        .collect()
}

fn channel_assignment(
    shader: &xc3_shader::gbuffer_database::Shader,
    index: usize,
    channel: char,
) -> Option<(u32, u32)> {
    let output = format!("out_attr{index}.{channel}");

    // TODO: How to handle multiple texture dependencies?
    let (sampler, channels) = shader.output_dependencies[&output]
        .get(0)?
        .split_once('.')?;

    let sampler_index = match sampler {
        "s0" => 0,
        "s1" => 1,
        "s2" => 2,
        "s3" => 3,
        "s4" => 4,
        "s5" => 5,
        "s6" => 6,
        "s7" => 7,
        // TODO: How to handle this case?
        _ => todo!(),
    };
    // TODO: Pass a channel index instead?
    // TODO: How to handle multiple channels like normal maps?
    // Just see if the current channel is used first for now.
    let c = if channels.contains(channel) {
        channel
    } else {
        channels.chars().next().unwrap()
    };
    let channel_index = "xyzw".find(c).unwrap() as u32;
    Some((sampler_index, channel_index))
}

fn load_wismt_texture(
    path: std::path::PathBuf,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> Option<wgpu::TextureView> {
    // TODO: Create a helper function in xc3_lib for this?
    // TODO: Why do not all paths exist?
    let xbc1 = Xbc1::from_file(&path).ok()?;
    let mut reader = Cursor::new(xbc1.decompress().unwrap());
    let mibl = Mibl::read(&mut reader).unwrap();

    Some(create_texture(device, queue, &mibl).create_view(&wgpu::TextureViewDescriptor::default()))
}

// TODO: Does this need to be public?
pub fn load_database<P: AsRef<Path>>(path: P) -> Vec<xc3_shader::gbuffer_database::File> {
    let json = std::fs::read_to_string(path).unwrap();
    serde_json::from_str(&json).unwrap()
}
