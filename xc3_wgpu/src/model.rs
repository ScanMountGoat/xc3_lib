use std::{
    io::{Cursor, Read, Seek},
    path::Path,
};

use binrw::BinReaderExt;
use glam::vec4;
use wgpu::util::DeviceExt;
use xc3_lib::{
    map::{MapModelData, PropModelData},
    mibl::Mibl,
    msmd::{Msmd, StreamEntry},
    msrd::Msrd,
    mxmd::{Mxmd, ShaderUnkType},
    vertex::VertexData,
    xbc1::Xbc1,
};
use xc3_model::vertex::{read_indices, read_vertices};

use crate::{
    material::{materials, Material},
    pipeline::ModelPipelineData,
    shader,
    texture::{create_texture, create_texture_with_base_mip},
};

pub struct Model {
    meshes: Vec<Mesh>,
    materials: Vec<Material>,
    vertex_buffers: Vec<VertexBuffer>,
    index_buffers: Vec<IndexBuffer>,
}

#[derive(Debug)]
struct Mesh {
    vertex_buffer_index: usize,
    index_buffer_index: usize,
    material_index: usize,
    // TODO: lod?
}

struct VertexBuffer {
    vertex_buffer: wgpu::Buffer,
}

struct IndexBuffer {
    index_buffer: wgpu::Buffer,
    vertex_index_count: u32,
}

impl Model {
    // TODO: Separate render pass for the transparent stuff in Unk7.
    // Only write to g0 and use the out_attr0 assignments.
    // Create the necessary pipeline with blending for each material.
    // TODO: How to handle Unk1?
    pub fn draw<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>, pass: ShaderUnkType) {
        for mesh in &self.meshes {
            // TODO: How does LOD selection work in game?
            let material = &self.materials[mesh.material_index];

            // TODO: Why are there materials with no textures?
            // TODO: Group these into passes with separate shaders for each pass?
            // TODO: The main pass is shared with outline, ope, and zpre?
            // TODO: How to handle transparency?
            if material.unk_type == pass
            // && material.texture_count > 0
            && !material.name.ends_with("_outline")
            && !material.name.ends_with("_ope")
            && !material.name.ends_with("_zpre")
            {
                // TODO: How to make sure the pipeline outputs match the render pass?
                render_pass.set_pipeline(&material.pipeline);

                material.bind_group1.set(render_pass);
                material.bind_group2.set(render_pass);

                self.draw_mesh(mesh, render_pass);
            }
        }
    }

    fn draw_mesh<'a>(&'a self, mesh: &Mesh, render_pass: &mut wgpu::RenderPass<'a>) {
        let vertex_data = &self.vertex_buffers[mesh.vertex_buffer_index];
        render_pass.set_vertex_buffer(0, vertex_data.vertex_buffer.slice(..));

        // TODO: Are all indices u16?
        // TODO: Why do maps not always refer to a valid index buffer?
        let index_data = &self.index_buffers[mesh.index_buffer_index];
        // let index_data = &self.index_buffers[mesh.index_buffer_index];
        render_pass.set_index_buffer(index_data.index_buffer.slice(..), wgpu::IndexFormat::Uint16);

        render_pass.draw_indexed(0..index_data.vertex_index_count, 0, 0..1);
    }
}

pub fn load_model(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    msrd: &Msrd,
    mxmd: &Mxmd,
    model_path: &str,
    shader_database: &xc3_shader::gbuffer_database::GBufferDatabase,
) -> Model {
    // Compile shaders only once to improve loading times.
    let pipeline_data = ModelPipelineData::new(device);

    // TODO: Avoid unwrap.
    let model_data = msrd.extract_vertex_data();

    // "chr/en/file.wismt" -> "chr/tex/nx/m"
    // TODO: Don't assume model_path is in the chr/ch or chr/en folders.
    let chr_folder = Path::new(model_path).parent().unwrap().parent().unwrap();
    let m_tex_folder = chr_folder.join("tex").join("nx").join("m");
    let h_tex_folder = chr_folder.join("tex").join("nx").join("h");

    let textures = load_textures(mxmd, device, queue, m_tex_folder, h_tex_folder);

    let cached_textures = load_cached_textures(device, queue, msrd);

    let vertex_buffers = vertex_buffers(device, &model_data);
    let index_buffers = index_buffers(device, &model_data);

    let materials = materials(
        device,
        queue,
        &pipeline_data,
        &mxmd.materials,
        &textures,
        &cached_textures,
        model_path,
        shader_database,
    );

    let meshes = meshes(&mxmd.models);

    Model {
        meshes,
        materials,
        vertex_buffers,
        index_buffers,
    }
}

// TODO: Separate module for this?
// TODO: Better way to pass the wismda file?
pub fn load_map<R: Read + Seek>(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    msmd: &Msmd,
    wismda: &mut R,
    model_path: &str,
    shader_database: &xc3_shader::gbuffer_database::GBufferDatabase,
) -> Vec<Model> {
    // Compile shaders only once to improve loading times.
    let pipeline_data = ModelPipelineData::new(device);

    // TODO: Are the msmd textures shared with all models?
    let textures: Vec<_> = msmd
        .textures
        .iter()
        .map(|texture| {
            let bytes = decompress_entry(wismda, &texture.mid);
            Mibl::read(&mut Cursor::new(&bytes)).unwrap()
        })
        .collect();

    // TODO: Better way to combine models?
    let mut combined_models = Vec::new();
    for map_model in &msmd.map_models {
        let new_models = load_map_models(
            wismda,
            map_model,
            &msmd.map_vertex_data,
            &textures,
            device,
            queue,
            model_path,
            shader_database,
            &pipeline_data,
        );
        combined_models.extend(new_models);
    }

    for prop_model in &msmd.prop_models {
        let new_models = load_prop_models(
            wismda,
            prop_model,
            &msmd.prop_vertex_data,
            &textures,
            device,
            queue,
            model_path,
            shader_database,
            &pipeline_data,
        );
        combined_models.extend(new_models);
    }

    combined_models
}

fn load_prop_models<R: Read + Seek>(
    wismda: &mut R,
    prop_model: &xc3_lib::msmd::PropModel,
    prop_vertex_data: &[StreamEntry],
    mibl_textures: &[Mibl],
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    model_path: &str,
    shader_database: &xc3_shader::gbuffer_database::GBufferDatabase,
    pipeline_data: &ModelPipelineData,
) -> Vec<Model> {
    let bytes = decompress_entry(wismda, &prop_model.entry);
    let prop_model_data: PropModelData = Cursor::new(bytes).read_le().unwrap();

    // Get the textures referenced by the materials in this model.
    let textures: Vec<_> = prop_model_data
        .textures
        .iter()
        .map(|item| {
            // TODO: Handle texture index being -1?
            let mibl = &mibl_textures[item.texture_index.max(0) as usize];
            Some(
                create_texture(device, queue, mibl)
                    .create_view(&wgpu::TextureViewDescriptor::default()),
            )
        })
        .collect();

    // TODO: Create the materials and meshes only once?
    // TODO: Create a separate data structure that indexes into the meshes?

    // Load the base LOD for each prop model.
    // TODO: Make sure this is documented in xc3_lib.
    prop_model_data
        .lods
        .props
        .iter()
        .map(|prop_lod| {
            let base_lod_index = prop_lod.base_lod_index as usize;
            let vertex_data_index = prop_model_data.vertex_data_indices[base_lod_index];

            let prop_model_entry = &prop_vertex_data[vertex_data_index as usize];

            let bytes = decompress_entry(wismda, prop_model_entry);
            let vertex_data: VertexData = Cursor::new(bytes).read_le().unwrap();

            let vertex_buffers = vertex_buffers(device, &vertex_data);
            let index_buffers = index_buffers(device, &vertex_data);

            let meshes = prop_model_data.models.models.elements[base_lod_index]
                .meshes
                .iter()
                .map(create_mesh)
                .collect();

            // TODO: cached textures?
            let materials = materials(
                device,
                queue,
                &pipeline_data,
                &prop_model_data.materials,
                &textures,
                &[],
                model_path,
                shader_database,
            );

            Model {
                meshes,
                materials,
                vertex_buffers,
                index_buffers,
            }
        })
        .collect()
}

fn load_map_models<R: Read + Seek>(
    wismda: &mut R,
    map_model: &xc3_lib::msmd::MapModel,
    map_vertex_data: &[xc3_lib::msmd::StreamEntry],
    mibl_textures: &[Mibl],
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    model_path: &str,
    shader_database: &xc3_shader::gbuffer_database::GBufferDatabase,
    pipeline_data: &ModelPipelineData,
) -> Vec<Model> {
    let bytes = decompress_entry(wismda, &map_model.entry);
    let map_model_data: MapModelData = Cursor::new(bytes).read_le().unwrap();

    // TODO: The mapping.indices and models.models always have the same length?
    // TODO: the mapping indices are in the range [0, 2*groups - 1]?
    // TODO: Some mapping sections assign to twice as many groups as actual groups?
    map_model_data
        .mapping
        .groups
        .iter()
        .enumerate()
        .map(|(group_index, group)| {
            // TODO: Load all groups?
            let vertex_data_entry = &map_vertex_data[group.vertex_data_index as usize];
            let bytes = decompress_entry(wismda, vertex_data_entry);
            let model_data: VertexData = Cursor::new(bytes).read_le().unwrap();

            let vertex_buffers = vertex_buffers(device, &model_data);
            let index_buffers = index_buffers(device, &model_data);

            // TODO: Select meshes based on the grouping?
            // TODO: Does the list of indices in the grouping assign items here to groups?
            // TODO: SHould we be creating multiple models in this step?
            let meshes = map_model_data
                .models
                .models
                .elements
                .iter()
                .zip(map_model_data.mapping.indices.iter())
                .find_map(|(model, index)| {
                    if *index as usize == group_index {
                        Some(model)
                    } else {
                        None
                    }
                })
                .unwrap()
                .meshes
                .iter()
                .map(create_mesh)
                .collect();

            // Get the textures referenced by the materials in this model.
            let textures: Vec<_> = map_model_data
                .textures
                .iter()
                .map(|item| {
                    // TODO: Handle texture index being -1?
                    let mibl = &mibl_textures[item.texture_index.max(0) as usize];
                    Some(
                        create_texture(device, queue, mibl)
                            .create_view(&wgpu::TextureViewDescriptor::default()),
                    )
                })
                .collect();

            let materials = materials(
                device,
                queue,
                &pipeline_data,
                &map_model_data.materials,
                &textures,
                &[],
                model_path,
                shader_database,
            );

            Model {
                meshes,
                materials,
                vertex_buffers,
                index_buffers,
            }
        })
        .collect()
}

fn meshes(models: &xc3_lib::mxmd::Models) -> Vec<Mesh> {
    models
        .models
        .elements
        .iter()
        .flat_map(|model| model.meshes.iter().map(create_mesh))
        .collect()
}

fn create_mesh(mesh: &xc3_lib::mxmd::Mesh) -> Mesh {
    Mesh {
        vertex_buffer_index: mesh.vertex_buffer_index as usize,
        index_buffer_index: mesh.index_buffer_index as usize,
        material_index: mesh.material_index as usize,
    }
}

fn decompress_entry<R: Read + Seek>(reader: &mut R, entry: &StreamEntry) -> Vec<u8> {
    reader
        .seek(std::io::SeekFrom::Start(entry.offset as u64))
        .unwrap();
    Xbc1::read(reader).unwrap().decompress().unwrap()
}

fn index_buffers(device: &wgpu::Device, model_data: &VertexData) -> Vec<IndexBuffer> {
    model_data
        .index_buffers
        .iter()
        .map(|info| {
            let indices = read_indices(model_data, info);

            let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("index buffer"),
                contents: bytemuck::cast_slice(&indices),
                usage: wgpu::BufferUsages::INDEX,
            });

            IndexBuffer {
                index_buffer,
                vertex_index_count: indices.len() as u32,
            }
        })
        .collect()
}

fn vertex_buffers(device: &wgpu::Device, model_data: &VertexData) -> Vec<VertexBuffer> {
    model_data
        .vertex_buffers
        .iter()
        .enumerate()
        .map(|(i, info)| {
            let vertices = read_vertices(info, i, model_data);

            // Start with default values for each attribute.
            // Convert the buffers to a standardized format.
            // This still tests the vertex buffer layouts and avoids needing multiple shaders.
            let buffer_vertices: Vec<_> = vertices
                .into_iter()
                .map(|v| shader::model::VertexInput {
                    position: v.position,
                    weight_index: v.weight_index,
                    vertex_color: v.vertex_color,
                    normal: v.normal,
                    tangent: v.tangent,
                    uv1: vec4(v.uv1.x, v.uv1.y, 0.0, 0.0),
                })
                .collect();

            let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("vertex buffer"),
                contents: bytemuck::cast_slice(&buffer_vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });

            VertexBuffer { vertex_buffer }
        })
        .collect()
}

fn load_textures(
    mxmd: &Mxmd,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    m_tex_folder: std::path::PathBuf,
    h_tex_folder: std::path::PathBuf,
) -> Vec<Option<wgpu::TextureView>> {
    mxmd.textures
        .items
        .as_ref()
        .unwrap()
        .textures
        .iter()
        .map(|item| load_wismt_mibl(device, queue, &m_tex_folder, &h_tex_folder, &item.name))
        .collect()
}

fn load_cached_textures(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    msrd: &Msrd,
) -> Vec<(String, wgpu::TextureView)> {
    let texture_data = msrd.extract_texture_data();

    msrd.texture_name_table
        .as_ref()
        .unwrap()
        .textures
        .iter()
        .map(|info| {
            let data =
                &texture_data[info.offset as usize..info.offset as usize + info.size as usize];
            let mibl = Mibl::read(&mut Cursor::new(&data)).unwrap();
            (
                info.name.clone(),
                create_texture(device, queue, &mibl)
                    .create_view(&wgpu::TextureViewDescriptor::default()),
            )
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
