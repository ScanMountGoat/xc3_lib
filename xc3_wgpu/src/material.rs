use std::{io::Cursor, path::Path};

use glam::{ivec4, uvec4};
use wgpu::util::DeviceExt;
use xc3_lib::{mibl::Mibl, mxmd::Mxmd, xbc1::Xbc1};

use crate::texture::{create_default_black_texture, create_texture, create_texture_with_base_mip};

pub struct Material {
    pub name: String,
    pub bind_group1: crate::shader::model::bind_groups::BindGroup1,
    pub bind_group2: crate::shader::model::bind_groups::BindGroup2,

    pub texture_count: usize,
    pub unk_type: xc3_lib::mxmd::ShaderUnkType,
}

pub fn materials(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    mxmd: &Mxmd,
    cached_textures: &[(String, Mibl)],
    model_path: &str,
    shader_database: &[xc3_shader::gbuffer_database::File],
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

    let name = Path::new(model_path)
        .with_extension("")
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string();

    // TODO: Make this a map instead of a vec?
    let shaders = &shader_database
        .iter()
        .find(|f| f.file == name)
        .unwrap()
        .shaders;

    // "chr/en/file.wismt" -> "chr/tex/nx/m"
    // TODO: Don't assume model_path is in the chr/ch or chr/en folders.
    let chr_folder = Path::new(model_path).parent().unwrap().parent().unwrap();
    let m_tex_folder = chr_folder.join("tex").join("nx").join("m");

    let h_tex_folder = chr_folder.join("tex").join("nx").join("h");

    mxmd.materials
        .materials
        .elements
        .iter()
        .map(|material| {
            let texture_views = load_textures(
                material,
                mxmd,
                &m_tex_folder,
                &h_tex_folder,
                device,
                queue,
                cached_textures,
            );

            // Bind all available textures and samplers.
            // Texture selection happens within the shader itself.
            // This simulates having a unique shader for each material.
            let bind_group1 = crate::shader::model::bind_groups::BindGroup1::from_bindings(
                device,
                crate::shader::model::bind_groups::BindGroupLayout1 {
                    s0: texture_views.get(0).unwrap_or(&default_black),
                    s1: texture_views.get(1).unwrap_or(&default_black),
                    s2: texture_views.get(2).unwrap_or(&default_black),
                    s3: texture_views.get(3).unwrap_or(&default_black),
                    s4: texture_views.get(4).unwrap_or(&default_black),
                    s5: texture_views.get(5).unwrap_or(&default_black),
                    s6: texture_views.get(6).unwrap_or(&default_black),
                    s7: texture_views.get(7).unwrap_or(&default_black),
                    s8: texture_views.get(8).unwrap_or(&default_black),
                    s9: texture_views.get(9).unwrap_or(&default_black),
                    shared_sampler: &default_sampler,
                },
            );

            // TODO: Is it better to store these in a hashmap instead of a vec?
            // TODO: Do all shaders have the naming convention "shd{program index}"?
            // TODO: Store a list of shaders for each index/name?
            let shader_name = format!(
                "shd{:0>4}",
                material.shader_programs.elements[0].program_index
            );
            let shader = shaders
                .iter()
                .find(|s| s.name.starts_with(&shader_name))
                .unwrap();
            let assignments = gbuffer_assignments(shader);

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

            Material {
                name: material.name.clone(),
                bind_group1,
                bind_group2,
                texture_count: material.textures.elements.len(),
                unk_type: material.shader_programs.elements[0].unk_type,
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
        shader.output_dependencies[&output]
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
pub fn load_database<P: AsRef<Path>>(path: P) -> Vec<xc3_shader::gbuffer_database::File> {
    let json = std::fs::read_to_string(path).unwrap();
    serde_json::from_str(&json).unwrap()
}

fn load_textures(
    material: &xc3_lib::mxmd::Material,
    mxmd: &Mxmd,
    m_texture_folder: &Path,
    h_texture_folder: &Path,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    cached_textures: &[(String, Mibl)],
) -> Vec<wgpu::TextureView> {
    // TODO: Store wgpu texture instead?
    // TODO: Access by name instead of index?
    material
        .textures
        .elements
        .iter()
        .map(|t| {
            // TODO: Why are the indices off by 1?
            let tex_name = &mxmd.textures.items.textures[t.texture_index as usize + 1].name;

            load_wismt_mibl(device, queue, m_texture_folder, h_texture_folder, tex_name)
                .unwrap_or_else(|| {
                    // Not all textures have higher resolution versions in the tex folder.
                    // Fall back to the cached textures if loading high res textures fails.
                    let mibl = cached_textures
                        .iter()
                        .find_map(|(name, mibl)| if name == tex_name { Some(mibl) } else { None })
                        .unwrap();

                    create_texture(device, queue, mibl)
                        .create_view(&wgpu::TextureViewDescriptor::default())
                })
        })
        .collect()
}

// TODO: Split into two functions?
fn load_wismt_mibl(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    m_texture_folder: &Path,
    h_texture_folder: &Path,
    texture_name: &str,
) -> Option<wgpu::TextureView> {
    // TODO: Create a helper function in xc3_lib for this?
    let xbc1 = Xbc1::from_file(m_texture_folder.join(texture_name).with_extension("wismt")).ok()?;
    let mut reader = Cursor::new(xbc1.decompress().unwrap());

    let mibl = Mibl::read(&mut reader).unwrap();

    let base_mip_level =
        Xbc1::from_file(&h_texture_folder.join(texture_name).with_extension("wismt"))
            .unwrap()
            .decompress()
            .unwrap();

    Some(
        create_texture_with_base_mip(device, queue, &mibl, &base_mip_level)
            .create_view(&wgpu::TextureViewDescriptor::default()),
    )
}
