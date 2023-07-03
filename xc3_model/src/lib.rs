//! # xc3_model
//! xc3_model provides high level data access for the files that make up a model.

use std::path::Path;

use glam::Mat4;
use texture::{load_textures, ImageTexture};
use vertex::{read_index_buffers, read_vertex_buffers, AttributeData};
use xc3_lib::{
    msrd::Msrd,
    mxmd::{MaterialFlags, Materials, Mxmd, ShaderUnkType},
};
use xc3_shader::gbuffer_database::{GBufferDatabase, Shader};

pub mod gltf;
pub mod map;
pub mod texture;
pub mod vertex;

// TODO: Come up with a better name
#[derive(Debug)]
pub struct ModelRoot {
    pub groups: Vec<ModelGroup>,
    pub image_textures: Vec<ImageTexture>,
}

#[derive(Debug)]
pub struct ModelGroup {
    pub models: Vec<Model>,
    pub materials: Vec<Material>,
    // TODO: Apply this ahead of time to simplify consuming code.
    pub image_texture_indices: Vec<usize>,
}

#[derive(Debug)]
pub struct Model {
    pub meshes: Vec<Mesh>,
    pub vertex_buffers: Vec<VertexBuffer>,
    pub index_buffers: Vec<IndexBuffer>,
    pub instances: Vec<Mat4>,
}

#[derive(Debug)]

pub struct Mesh {
    pub vertex_buffer_index: usize,
    pub index_buffer_index: usize,
    pub material_index: usize,
}

#[derive(Debug)]

pub struct Material {
    pub name: String,
    pub flags: MaterialFlags,
    pub textures: Vec<Texture>,
    /// Precomputed metadata from the decompiled shader source
    /// or [None] if the database does not contain this model.
    pub shader: Option<Shader>,
    // TODO: include with shader?
    pub unk_type: ShaderUnkType,
}

// TODO: sampler index or sampler flags?
#[derive(Debug)]

pub struct Texture {
    pub image_texture_index: usize,
}

#[derive(Debug)]
pub struct VertexBuffer {
    pub attributes: Vec<AttributeData>,
}

#[derive(Debug)]
pub struct IndexBuffer {
    // TODO: support u32?
    pub indices: Vec<u16>,
}

impl Model {
    pub fn from_model(
        model: &xc3_lib::mxmd::Model,
        vertex_data: &xc3_lib::vertex::VertexData,
        instances: Vec<Mat4>,
    ) -> Self {
        let meshes = model
            .meshes
            .iter()
            .map(|mesh| Mesh {
                vertex_buffer_index: mesh.vertex_buffer_index as usize,
                index_buffer_index: mesh.index_buffer_index as usize,
                material_index: mesh.material_index as usize,
            })
            .collect();

        let vertex_buffers = read_vertex_buffers(vertex_data);
        let index_buffers = read_index_buffers(vertex_data);

        Self {
            meshes,
            vertex_buffers,
            index_buffers,
            instances,
        }
    }
}

/// Load a character (ch), object (oj), weapon (wp), or enemy (en) model.
pub fn load_model(
    msrd: &Msrd,
    mxmd: &Mxmd,
    model_path: &str, // TODO: &Path?
    shader_database: &GBufferDatabase,
) -> ModelRoot {
    // TODO: Avoid unwrap.
    let vertex_data = msrd.extract_vertex_data();

    // "chr/en/file.wismt" -> "chr/tex/nx/m"
    // TODO: Don't assume model_path is in the chr/ch or chr/en folders.
    let chr_folder = Path::new(model_path).parent().unwrap().parent().unwrap();
    let m_tex_folder = chr_folder.join("tex").join("nx").join("m");
    let h_tex_folder = chr_folder.join("tex").join("nx").join("h");

    let image_textures = load_textures(msrd, mxmd, &m_tex_folder, &h_tex_folder);

    let model_folder = model_folder_name(model_path);
    let spch = shader_database.files.get(&model_folder);

    let materials = materials(&mxmd.materials, spch);

    // TODO: Don't assume there is only one model?
    let model = Model::from_model(&mxmd.models.models[0], &vertex_data, vec![Mat4::IDENTITY]);

    ModelRoot {
        groups: vec![ModelGroup {
            materials,
            models: vec![model],
            image_texture_indices: (0..image_textures.len()).collect(),
        }],
        image_textures,
    }
}

fn materials(
    materials: &Materials,
    spch: Option<&xc3_shader::gbuffer_database::Spch>,
) -> Vec<Material> {
    materials
        .materials
        .iter()
        .map(|material| {
            // TODO: How to choose between the two fragment shaders?
            let program_index = material.shader_programs[0].program_index as usize;
            let shader = spch
                .and_then(|spch| spch.programs.get(program_index))
                .map(|program| &program.shaders[0])
                .cloned();

            let textures = material
                .textures
                .iter()
                .map(|texture| Texture {
                    image_texture_index: texture.texture_index as usize,
                })
                .collect();

            Material {
                name: material.name.clone(),
                flags: material.flags,
                textures,
                shader,
                unk_type: material.shader_programs[0].unk_type,
            }
        })
        .collect()
}

// TODO: Move this to xc3_shader?
fn model_folder_name(model_path: &str) -> String {
    Path::new(model_path)
        .with_extension("")
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string()
}
