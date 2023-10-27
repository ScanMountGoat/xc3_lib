//! # xc3_model
//! xc3_model provides high level data access for the files that make up a model.

use std::path::Path;

use glam::Mat4;
use log::warn;
use skinning::SkinWeights;
use texture::load_textures;
use vertex::{read_index_buffers, read_vertex_buffers, AttributeData};
use xc3_lib::{
    apmd::Apmd,
    msrd::Msrd,
    mxmd::{Materials, Mxmd, ShaderUnkType, StateFlags},
    sar1::Sar1,
};

pub use map::load_map;
pub use sampler::{AddressMode, FilterMode, Sampler};
pub use skeleton::{Bone, Skeleton};
pub use texture::{ImageFormat, ImageTexture, ViewDimension};

pub use xc3_shader::gbuffer_database::{GBufferDatabase, Shader};

pub mod animation;
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
    pub weights: Option<Weights>,
}

// TODO: come up with a better name?
#[derive(Debug)]
pub struct Weights {
    // TODO: have each Models have its own reindexed set of indices based on skeleton names?
    pub skin_weights: SkinWeights,

    // TODO: Is this the best way to represent this information?
    pub weight_groups: Vec<xc3_lib::vertex::WeightGroup>,
    pub weight_lods: Vec<xc3_lib::vertex::WeightLod>,
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

    // TODO: make this a function instead to avoid dependencies?
    /// The minimum XYZ coordinates of the bounding volume.
    pub max_xyz: [f32; 3],
    /// The maximum XYZ coordinates of the bounding volume.
    pub min_xyz: [f32; 3],
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
    pub skin_flags: u32,
}

#[derive(Debug)]
pub struct Material {
    pub name: String,
    pub flags: StateFlags,
    pub textures: Vec<Texture>,

    pub alpha_test: Option<TextureAlphaTest>,

    // TODO: Also store parameters?
    /// Precomputed metadata from the decompiled shader source
    /// or [None] if the database does not contain this model.
    pub shader: Option<Shader>,

    // TODO: include with shader?
    pub unk_type: ShaderUnkType,
    pub parameters: MaterialParameters,
}

/// Information for alpha testing based on sampled texture values.
#[derive(Debug)]
pub struct TextureAlphaTest {
    /// The texture in [textures](struct.Material.html#structfield.textures) used for alpha testing.
    pub texture_index: usize,
    /// The RGBA channel to sample for the comparison.
    pub channel_index: usize,
    // TODO: alpha test ref value?
    pub ref_value: f32,
}

/// Values assigned to known shader uniforms or `None` if not present.
#[derive(Debug)]
pub struct MaterialParameters {
    pub mat_color: [f32; 4],
    pub alpha_test_ref: f32,
    // Assume each param type is used at most once.
    pub tex_matrix: Option<Vec<[f32; 8]>>,
    pub work_float4: Option<Vec<[f32; 4]>>,
    pub work_color: Option<Vec<[f32; 4]>>,
}

impl Default for MaterialParameters {
    fn default() -> Self {
        Self {
            mat_color: [1.0; 4],
            alpha_test_ref: 1.0,
            tex_matrix: None,
            work_float4: None,
            work_color: None,
        }
    }
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
    /// Animation targets for vertex attributes like positions and normals.
    /// The first target can be assumed to be the base target.
    pub morph_targets: Vec<MorphTarget>,
}

#[derive(Debug)]
pub struct MorphTarget {
    // TODO: add names from mxmd?
    pub attributes: Vec<AttributeData>,
}

#[derive(Debug)]
pub struct IndexBuffer {
    // TODO: support u32?
    pub indices: Vec<u16>,
}

impl VertexBuffer {
    pub fn vertex_count(&self) -> usize {
        // TODO: Check all attributes for consistency?
        self.attributes.first().map(|a| a.len()).unwrap_or_default()
    }
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
                .map(|data| data.groups.iter().map(|i| i.base_lod_index).collect()),
            min_xyz: models.min_xyz,
            max_xyz: models.max_xyz,
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
                skin_flags: mesh.skin_flags,
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

    let mxmd = Mxmd::from_file(wimdo_path).unwrap_or_else(|e| {
        warn!("Failed to read Mxmd: {e}. Trying Apmd.");
        // Some wimdo files have the mxmd in an archive.
        Apmd::from_file(wimdo_path)
            .unwrap()
            .entries
            .iter()
            .find_map(|e| {
                if let Ok(xc3_lib::apmd::EntryData::Mxmd(mxmd)) = e.read_data() {
                    Some(mxmd)
                } else {
                    None
                }
            })
            .unwrap()
    });

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
    // TODO: make this optional?
    let chr = Sar1::from_file(&wimdo_path.with_extension("chr"))
        .or_else(|_| Sar1::from_file(wimdo_path.with_extension("arc")))
        .or_else(|_| {
            // TODO: Is the last digit always 0 like in ch01012013.wimdo -> ch01012010.chr?
            let mut chr_name = model_name.clone();
            chr_name.pop();
            chr_name.push('0');

            let chr_path = wimdo_path.with_file_name(chr_name).with_extension("chr");
            Sar1::from_file(chr_path)
        })
        .ok();

    let skeleton = create_skeleton(chr.as_ref(), &mxmd);

    let (vertex_buffers, weights) = read_vertex_buffers(vertex_data, mxmd.models.skinning.as_ref());
    let index_buffers = read_index_buffers(vertex_data);

    let models = Models::from_models(&mxmd.models, &mxmd.materials, spch, skeleton);

    ModelRoot {
        groups: vec![ModelGroup {
            models: vec![models],
            buffers: vec![ModelBuffers {
                vertex_buffers,
                index_buffers,
                weights,
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

fn create_skeleton(chr: Option<&Sar1>, mxmd: &Mxmd) -> Option<Skeleton> {
    // Merge both skeletons since the bone lists may be different.
    // TODO: Create a skeleton even without the chr?
    let skel = chr?
        .entries
        .iter()
        .find_map(|e| match e.read_data().unwrap() {
            xc3_lib::sar1::EntryData::Bc(bc) => match bc.data {
                xc3_lib::bc::BcData::Skel(skel) => Some(skel),
                _ => None,
            },
            _ => None,
        })?;

    Some(Skeleton::from_skel(
        &skel.skeleton,
        mxmd.models.skinning.as_ref()?,
    ))
}

// TODO: material module?
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

            let parameters = assign_parameters(materials, material);

            let alpha_test = find_alpha_test_texture(materials, material);

            Material {
                name: material.name.clone(),
                flags: material.state_flags,
                textures,
                alpha_test,
                shader,
                unk_type: material.shader_programs[0].unk_type,
                parameters,
            }
        })
        .collect()
}

fn find_alpha_test_texture(
    materials: &Materials,
    material: &xc3_lib::mxmd::Material,
) -> Option<TextureAlphaTest> {
    // Find the texture used for alpha testing in the shader.
    // TODO: investigate how this works in game.
    let alpha_texture = materials
        .alpha_test_textures
        .get(material.alpha_test_texture_index as usize)?;
    if material.flags.alpha_mask() {
        // TODO: Do some materials require separate textures in a separate pass?
        let texture_index = material
            .textures
            .iter()
            .position(|t| t.texture_index == alpha_texture.texture_index)?;

        // Some materials use the red channel of a dedicated mask instead of alpha.
        let channel_index = if material.flags.separate_mask() { 0 } else { 3 };

        Some(TextureAlphaTest {
            texture_index,
            channel_index,
            ref_value: 0.5,
        })
    } else {
        None
    }
}

// TODO: Some elements get set by values not in the floats array?
// TODO: How to test this?
// TODO: This doesn't work properly for all models?
fn assign_parameters(
    materials: &Materials,
    material: &xc3_lib::mxmd::Material,
) -> MaterialParameters {
    // TODO: Don't assume a single program info?
    let info = &materials.shader_programs[material.shader_programs[0].program_index as usize];
    let floats = &materials.floats;
    let start_index = material.floats_start_index;

    let mut parameters = MaterialParameters {
        mat_color: material.color,
        alpha_test_ref: 0.5, //material.alpha_test_ref,
        tex_matrix: None,
        work_float4: None,
        work_color: None,
    };

    for param in &info.parameters {
        match param.param_type {
            xc3_lib::mxmd::ParamType::Unk0 => (),
            xc3_lib::mxmd::ParamType::TexMatrix => {
                parameters.tex_matrix = Some(read_param(param, floats, start_index));
            }
            xc3_lib::mxmd::ParamType::WorkFloat4 => {
                parameters.work_float4 = Some(read_param(param, floats, start_index));
            }
            xc3_lib::mxmd::ParamType::WorkColor => {
                parameters.work_color = Some(read_param(param, floats, start_index));
            }
            xc3_lib::mxmd::ParamType::Unk4 => (),
            xc3_lib::mxmd::ParamType::Unk5 => (),
            xc3_lib::mxmd::ParamType::Unk6 => (),
            xc3_lib::mxmd::ParamType::Unk7 => (),
            xc3_lib::mxmd::ParamType::Unk10 => (),
        }
    }

    parameters
}

fn read_param<const N: usize>(
    param: &xc3_lib::mxmd::MaterialParameter,
    floats: &[f32],
    start_index: u32,
) -> Vec<[f32; N]> {
    // Assume any parameter can be an array, so read a vec.
    // TODO: avoid unwrap.
    let start = param.floats_index_offset as usize + start_index as usize;
    floats[start..]
        .chunks_exact(N)
        .take(param.count as usize)
        .map(|v| v.try_into().unwrap())
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

// TODO: Module and tests for this?
impl Weights {
    pub fn weight_group_index(
        &self,
        skin_flags: u32,
        lod: u16,
        unk_type: xc3_lib::mxmd::ShaderUnkType,
    ) -> Option<usize> {
        // TODO: Is this the correct flags check?
        // TODO: This doesn't work for other unk type or lod?
        if (skin_flags & 0x1) == 0 {
            let lod_index = lod.saturating_sub(1) as usize;
            let weight_lod = &self.weight_lods[lod_index];

            // TODO: bit mask?
            let pass_index = match unk_type {
                xc3_lib::mxmd::ShaderUnkType::Unk0 => 0,
                xc3_lib::mxmd::ShaderUnkType::Unk1 => 1,
                xc3_lib::mxmd::ShaderUnkType::Unk6 => todo!(),
                xc3_lib::mxmd::ShaderUnkType::Unk7 => 3,
                xc3_lib::mxmd::ShaderUnkType::Unk9 => todo!(),
            };
            Some(weight_lod.group_indices_plus_one[pass_index].saturating_sub(1) as usize)
            // None
        } else {
            None
        }
    }

    pub fn weights_starting_index(
        &self,
        skin_flags: u32,
        lod: u16,
        unk_type: xc3_lib::mxmd::ShaderUnkType,
    ) -> usize {
        self.weight_group_index(skin_flags, lod, unk_type)
            .map(|group_index| self.weight_groups[group_index].input_start_index as usize)
            .unwrap_or_default()
    }
}
