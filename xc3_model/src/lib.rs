//! # xc3_model
//! xc3_model provides high level data access for the files that make up a model.

use std::path::Path;

use glam::Mat4;
use skeleton::Skeleton;
use skinning::Influence;
use texture::{load_textures, ImageTexture};
use vertex::{read_index_buffers, read_vertex_buffers, AttributeData};
use xc3_lib::{
    msrd::Msrd,
    mxmd::{MaterialFlags, Materials, Mxmd, ShaderUnkType},
    sar1::Sar1,
};
use xc3_shader::gbuffer_database::{GBufferDatabase, Shader};

pub use map::load_map;

pub mod gltf;
mod map;
pub mod skeleton;
pub mod skinning;
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
    pub skeleton: Option<Skeleton>,
}

#[derive(Debug)]
pub struct Model {
    pub meshes: Vec<Mesh>,
    pub vertex_buffers: Vec<VertexBuffer>,
    pub index_buffers: Vec<IndexBuffer>,
    // TODO: Separate weight buffer?
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
    /// The index of the image in [image_textures](struct.ModelRoot.html#structfield.image_textures).
    pub image_texture_index: usize,
}

#[derive(Debug)]
pub struct VertexBuffer {
    // TODO: Remove the attributes for skin weights?
    pub attributes: Vec<AttributeData>,
    pub influences: Vec<Influence>,
}

#[derive(Debug)]
pub struct IndexBuffer {
    // TODO: support u32?
    pub indices: Vec<u16>,
}

impl Model {
    pub fn from_model(
        model: &xc3_lib::mxmd::Model,
        skeleton: Option<&xc3_lib::mxmd::Skeleton>,
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

        let vertex_buffers = read_vertex_buffers(vertex_data, skeleton);
        let index_buffers = read_index_buffers(vertex_data);

        Self {
            meshes,
            vertex_buffers,
            index_buffers,
            instances,
        }
    }
}

// TODO: Document loading the database in an example.
/// Load a character (ch), object (oj), weapon (wp), or enemy (en) model
/// for Xenoblade 2 or Xenoblade 3.
pub fn load_model<P: AsRef<Path>>(
    wimdo_path: P,
    shader_database: Option<&GBufferDatabase>,
) -> ModelRoot {
    let wimdo_path = wimdo_path.as_ref();

    let mxmd = Mxmd::from_file(wimdo_path).unwrap();
    // TODO: Some files don't have a wismt?
    let msrd = Msrd::from_file(wimdo_path.with_extension("wismt")).ok();
    // TODO: Avoid unwrap.
    let msrd_vertex_data = msrd.as_ref().map(|msrd| msrd.extract_vertex_data());
    let vertex_data = mxmd
        .vertex_data
        .as_ref()
        .unwrap_or_else(|| msrd_vertex_data.as_ref().unwrap());

    // "chr/en/file.wismt" -> "chr/tex/nx/m"
    // TODO: Don't assume model_path is in the chr/ch or chr/en folders.
    let chr_folder = wimdo_path.parent().unwrap().parent().unwrap();
    let m_tex_folder = chr_folder.join("tex").join("nx").join("m");
    let h_tex_folder = chr_folder.join("tex").join("nx").join("h");

    let image_textures = load_textures(&mxmd, msrd.as_ref(), &m_tex_folder, &h_tex_folder);

    let model_name = model_name(wimdo_path.as_ref());
    let spch = shader_database.and_then(|database| database.files.get(&model_name));

    let materials = materials(&mxmd.materials, spch);

    // TODO: Is the last digit always 0 like in ch01012013.wimdo -> ch01012010.chr?
    let mut chr_name = model_name.clone();
    chr_name.pop();
    chr_name.push('0');

    let chr = Sar1::from_file(wimdo_path.with_file_name(chr_name).with_extension("chr")).unwrap();
    let skeleton = create_skeleton(&chr, &mxmd);

    let models = mxmd
        .models
        .models
        .iter()
        .map(|model| {
            Model::from_model(
                model,
                mxmd.models.skeleton.as_ref(),
                vertex_data,
                vec![Mat4::IDENTITY],
            )
        })
        .collect();

    ModelRoot {
        groups: vec![ModelGroup {
            materials,
            models,
            skeleton,
        }],
        image_textures,
    }
}

fn create_skeleton(chr: &Sar1, mxmd: &Mxmd) -> Option<Skeleton> {
    // Merge both skeletons since the bone lists may be different.
    let skel = chr.entries.iter().find_map(|e| match &e.data {
        xc3_lib::sar1::EntryData::Bc(bc) => match &bc.data {
            xc3_lib::sar1::BcData::Skel(skel) => Some(skel),
            _ => None,
        },
        _ => None,
    })?;

    Some(Skeleton::from_skel(skel, mxmd.models.skeleton.as_ref()?))
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
fn model_name(model_path: &Path) -> String {
    model_path
        .with_extension("")
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string()
}
