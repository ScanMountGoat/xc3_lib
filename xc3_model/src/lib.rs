//! # xc3_model
//! xc3_model provides high level data access for the files that make up a model.

use std::path::Path;

use glam::Mat4;
use skinning::Influence;
use texture::load_textures;
use vertex::{read_index_buffers, read_vertex_buffers, AttributeData};
use xc3_lib::{
    msrd::Msrd,
    mxmd::{MaterialFlags, Materials, Mxmd, ShaderUnkType},
    sar1::Sar1,
};

pub use map::load_map;
pub use sampler::{AddressMode, FilterMode, Sampler};
pub use skeleton::{Bone, Skeleton};
pub use texture::{ImageFormat, ImageTexture, ViewDimension};

// TODO: Export from a shader module instead of the crate root?
pub use xc3_shader::gbuffer_database::{GBufferDatabase, Shader};

pub mod gltf;
mod map;
mod sampler;
mod skeleton;
pub mod skinning;
mod texture;
pub mod vertex;

// TODO: Come up with a better name
#[derive(Debug)]
pub struct ModelRoot {
    pub groups: Vec<ModelGroup>,
    pub image_textures: Vec<ImageTexture>,
}

#[derive(Debug)]
pub struct ModelGroup {
    pub models: Vec<Models>,
    pub buffers: Vec<ModelBuffers>,
}

#[derive(Debug)]
pub struct ModelBuffers {
    pub vertex_buffers: Vec<VertexBuffer>,
    pub index_buffers: Vec<IndexBuffer>,
}

// TODO: Should samplers be optional?
// TODO: Come up with a better name?
#[derive(Debug)]
pub struct Models {
    pub models: Vec<Model>,
    pub materials: Vec<Material>,
    pub samplers: Vec<Sampler>,
    pub skeleton: Option<Skeleton>,
    // TODO: Better way of organizing this data?
    // TODO: How to handle the indices being off by 1?
    // TODO: when is this None?
    // TODO: Create a type for this constructed from Models?
    pub base_lod_indices: Option<Vec<u16>>,
}

#[derive(Debug)]
pub struct Model {
    pub meshes: Vec<Mesh>,
    /// Each mesh has instance for every transform in [instances](#structfield.instances).
    pub instances: Vec<Mat4>,
    pub model_buffers_index: usize,
}

#[derive(Debug)]
pub struct Mesh {
    pub vertex_buffer_index: usize,
    pub index_buffer_index: usize,
    pub material_index: usize,
    pub lod: u16,
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

#[derive(Debug)]
pub struct Texture {
    /// The index of the [ImageTexture] in [image_textures](struct.ModelRoot.html#structfield.image_textures).
    pub image_texture_index: usize,
    /// The index of the [Sampler] in [samplers](struct.ModelGroup.html#structfield.samplers).
    pub sampler_index: usize,
}

#[derive(Debug)]
pub struct VertexBuffer {
    pub attributes: Vec<AttributeData>,
    // TODO: Buffers can be shared between models with different bone names?
    pub influences: Vec<Influence>,
}

#[derive(Debug)]
pub struct IndexBuffer {
    // TODO: support u32?
    pub indices: Vec<u16>,
}

impl Models {
    pub fn from_models(
        models: &xc3_lib::mxmd::Models,
        materials: &xc3_lib::mxmd::Materials,
        spch: Option<&xc3_shader::gbuffer_database::Spch>,
        skeleton: Option<Skeleton>,
    ) -> Models {
        Models {
            models: models
                .models
                .iter()
                .map(|model| Model::from_model(model, vec![Mat4::IDENTITY], 0))
                .collect(),
            materials: create_materials(materials, spch),
            samplers: create_samplers(materials),
            skeleton,
            base_lod_indices: models
                .lod_data
                .as_ref()
                .map(|data| data.items2.iter().map(|i| i.base_lod_index).collect()),
        }
    }
}

impl Model {
    pub fn from_model(
        model: &xc3_lib::mxmd::Model,
        instances: Vec<Mat4>,
        model_buffers_index: usize,
    ) -> Self {
        let meshes = model
            .meshes
            .iter()
            .map(|mesh| Mesh {
                vertex_buffer_index: mesh.vertex_buffer_index as usize,
                index_buffer_index: mesh.index_buffer_index as usize,
                material_index: mesh.material_index as usize,
                lod: mesh.lod,
            })
            .collect();

        Self {
            meshes,
            instances,
            model_buffers_index,
        }
    }
}

/// Returns `true` if a mesh with `lod` should be rendered
/// as part of the highest detail or base level of detail (LOD).
pub fn should_render_lod(lod: u16, base_lod_indices: &Option<Vec<u16>>) -> bool {
    // TODO: Why are the mesh values 1-indexed and the models lod data 0-indexed?
    // TODO: should this also include 0?
    // TODO: How to handle the none case?
    // TODO: Add test cases for this?
    base_lod_indices
        .as_ref()
        .map(|indices| indices.contains(&lod.saturating_sub(1)))
        .unwrap_or(true)
}

// TODO: Document loading the database in an example.
/// Load a character (ch), object (oj), weapon (wp), or enemy (en) model.
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

    let model_name = model_name(wimdo_path);
    let spch = shader_database.and_then(|database| database.files.get(&model_name));

    // TODO: Does every wimdo have a chr file?
    // TODO: Does something control the chr name used?
    let chr = Sar1::from_file(&wimdo_path.with_extension("chr")).unwrap_or_else(|_| {
        // TODO: Is the last digit always 0 like in ch01012013.wimdo -> ch01012010.chr?
        let mut chr_name = model_name.clone();
        chr_name.pop();
        chr_name.push('0');

        let chr_path = wimdo_path.with_file_name(chr_name).with_extension("chr");
        Sar1::from_file(&chr_path).unwrap()
    });

    let skeleton = create_skeleton(&chr, &mxmd);

    let vertex_buffers = read_vertex_buffers(vertex_data, mxmd.models.skeleton.as_ref());
    let index_buffers = read_index_buffers(vertex_data);

    let models = Models::from_models(&mxmd.models, &mxmd.materials, spch, skeleton);

    ModelRoot {
        groups: vec![ModelGroup {
            models: vec![models],
            buffers: vec![ModelBuffers {
                vertex_buffers,
                index_buffers,
            }],
        }],
        image_textures,
    }
}

fn create_samplers(materials: &Materials) -> Vec<Sampler> {
    materials
        .samplers
        .as_ref()
        .map(|samplers| samplers.samplers.iter().map(|s| s.flags.into()).collect())
        .unwrap_or_default()
}

fn create_skeleton(chr: &Sar1, mxmd: &Mxmd) -> Option<Skeleton> {
    // Merge both skeletons since the bone lists may be different.
    let skel = chr
        .entries
        .iter()
        .find_map(|e| match e.read_data().unwrap() {
            xc3_lib::sar1::EntryData::Bc(bc) => match bc.data {
                xc3_lib::bc::BcData::Skel(skel) => Some(skel),
                _ => None,
            },
            _ => None,
        })?;

    Some(Skeleton::from_skel(&skel, mxmd.models.skeleton.as_ref()?))
}

fn create_materials(
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
                    sampler_index: texture.sampler_index as usize,
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
