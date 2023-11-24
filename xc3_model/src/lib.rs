//! # xc3_model
//! xc3_model provides high level data access for the files that make up a model.
//!
//! Each type typically represents the decoded data associated with one or more types in [xc3_lib].
//! This simplifies the processing that needs to be done to access model data
//! and abstracts away most of the game specific complexities.
//! This conversion is currently one way, so saving types back to files is not yet supported.
//!
//! # Getting Started
//! Loading a normal model returns a single [ModelRoot].
//! Loading a map returns multiple [ModelRoot].
//! Each [ModelRoot] has its own set of images.
//!
//! The [ShaderDatabase] is optional and improves the accuracy of texture and material assignments.
//!
//! ```rust no_run
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! use xc3_model::shader_database::ShaderDatabase;
//!
//! let database = ShaderDatabase::from_file("xc3.json");
//!
//! let root = xc3_model::load_model("ch01011013.wimdo", Some(&database));
//! println!("{}", root.image_textures.len());
//!
//! let roots = xc3_model::load_map("ma59a.wismhd", Some(&database));
//! println!("{}", root.image_textures.len());
//! # Ok(())
//! # }
//! ```

use std::path::Path;

use animation::Animation;
use glam::{Mat4, Vec3, Vec4};
use log::{error, warn};
use shader_database::{Shader, ShaderDatabase};
use skinning::SkinWeights;
use texture::load_textures;
use vertex::{read_index_buffers, read_vertex_buffers, AttributeData};
use xc3_lib::{
    apmd::Apmd,
    msrd::Msrd,
    mxmd::{Materials, Mxmd},
    sar1::Sar1,
};

pub use map::load_map;
pub use sampler::{AddressMode, FilterMode, Sampler};
pub use skeleton::{Bone, Skeleton};
pub use texture::{ImageFormat, ImageTexture, ViewDimension};
pub use xc3_lib::mxmd::{BlendState, ShaderUnkType, StateFlags};

pub mod animation;
pub mod gltf;
mod map;
mod sampler;
pub mod shader_database;
mod skeleton;
pub mod skinning;
mod texture;
pub mod vertex;

// TODO: Come up with a better name
#[derive(Debug, Clone, PartialEq)]
pub struct ModelRoot {
    pub groups: Vec<ModelGroup>,
    /// The textures selected by each [Material].
    /// This includes all packed and embedded textures after
    /// combining all mip levels.
    pub image_textures: Vec<ImageTexture>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModelGroup {
    pub models: Vec<Models>,
    /// The vertex data selected by each [Model].
    pub buffers: Vec<ModelBuffers>,
}

/// See [VertexData](xc3_lib::vertex::VertexData).
#[derive(Debug, Clone, PartialEq)]
pub struct ModelBuffers {
    pub vertex_buffers: Vec<VertexBuffer>,
    pub index_buffers: Vec<IndexBuffer>,
    pub weights: Option<Weights>,
}

// TODO: come up with a better name?
/// See [Weights](xc3_lib::vertex::Weights).
#[derive(Debug, Clone, PartialEq)]
pub struct Weights {
    // TODO: have each Models have its own reindexed set of indices based on skeleton names?
    pub skin_weights: SkinWeights,

    // TODO: Is this the best way to represent this information?
    pub weight_groups: Vec<xc3_lib::vertex::WeightGroup>,
    pub weight_lods: Vec<xc3_lib::vertex::WeightLod>,
}

// TODO: Should samplers be optional?
// TODO: Come up with a better name?
/// See [Models](xc3_lib::mxmd::Models).
#[derive(Debug, Clone, PartialEq)]
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

/// See [Model](xc3_lib::mxmd::Model).
#[derive(Debug, Clone, PartialEq)]
pub struct Model {
    pub meshes: Vec<Mesh>,
    /// Each mesh has an instance for every transform in [instances](#structfield.instances).
    pub instances: Vec<Mat4>,
    /// The index of the [ModelBuffers] in [buffers](struct.ModelGroup.html#structfield.buffers).
    /// This will only be non zero for some map models.
    pub model_buffers_index: usize,
}

/// See [Mesh](xc3_lib::mxmd::Mesh).
#[derive(Debug, Clone, PartialEq)]
pub struct Mesh {
    pub vertex_buffer_index: usize,
    pub index_buffer_index: usize,
    pub material_index: usize,
    pub lod: u16,
    pub skin_flags: u32,
}

/// See [Material](xc3_lib::mxmd::Material) and [FoliageMaterial](xc3_lib::map::FoliageMaterial).
#[derive(Debug, Clone, PartialEq)]
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
#[derive(Debug, Clone, PartialEq)]
pub struct TextureAlphaTest {
    /// The texture in [textures](struct.Material.html#structfield.textures) used for alpha testing.
    pub texture_index: usize,
    /// The RGBA channel to sample for the comparison.
    pub channel_index: usize,
    // TODO: alpha test ref value?
    pub ref_value: f32,
}

/// Values assigned to known shader uniforms or `None` if not present.
#[derive(Debug, Clone, PartialEq)]
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

/// Selects an [ImageTexture] and [Sampler].
#[derive(Debug, Clone, PartialEq)]
pub struct Texture {
    /// The index of the [ImageTexture] in [image_textures](struct.ModelRoot.html#structfield.image_textures).
    pub image_texture_index: usize,
    /// The index of the [Sampler] in [samplers](struct.ModelGroup.html#structfield.samplers).
    pub sampler_index: usize,
}

/// See [VertexBufferDescriptor](xc3_lib::vertex::VertexBufferDescriptor).
#[derive(Debug, Clone, PartialEq)]
pub struct VertexBuffer {
    pub attributes: Vec<AttributeData>,
    /// Animation targets for vertex attributes like positions and normals.
    /// The base target is already applied to [attributes](#structfield.attributes).
    pub morph_targets: Vec<MorphTarget>,
}

/// Morph target attributes defined as a difference or deformation from the base target.
///
/// The final attribute values are simply `base + target * weight`.
#[derive(Debug, Clone, PartialEq)]
pub struct MorphTarget {
    // TODO: add names from mxmd?
    // TODO: Add a method with tests to blend with base target?
    pub position_deltas: Vec<Vec3>,
    // TODO: Exclude the 4th sign component?
    pub normal_deltas: Vec<Vec4>,
    pub tangent_deltas: Vec<Vec4>,
    /// The index of the vertex affected by each offset deltas.
    pub vertex_indices: Vec<u32>,
}

// TODO: method to convert to a non sparse format?

/// See [IndexBufferDescriptor](xc3_lib::vertex::IndexBufferDescriptor).
#[derive(Debug, Clone, PartialEq)]
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
        spch: Option<&shader_database::Spch>,
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

// TODO: Take an iterator for wimdo paths and merge to support xc1?
// TODO: Document using iter::once?
// TODO: Document loading the database in an example.
/// Load a model from a `.wimdo` file.
/// The corresponding `.chr` or `.arc` should be in the same directory.
pub fn load_model<P: AsRef<Path>>(
    wimdo_path: P,
    shader_database: Option<&ShaderDatabase>,
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
    let msrd_vertex_data = msrd
        .as_ref()
        .map(|msrd| msrd.extract_vertex_data().unwrap());
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
    let chr = Sar1::from_file(wimdo_path.with_extension("chr"))
        .ok()
        .or_else(|| Sar1::from_file(wimdo_path.with_extension("arc")).ok())
        .or_else(|| {
            // Keep trying with more 0's at the end to match in game naming conventions.
            // XC1: pc010101.wimdo -> pc010000.chr.
            // XC3: ch01012013.wimdo -> ch01012010.chr.
            (0..model_name.len()).find_map(|i| {
                let mut chr_name = model_name.clone();
                chr_name.replace_range(chr_name.len() - i.., &"0".repeat(i));
                let chr_path = wimdo_path.with_file_name(chr_name).with_extension("chr");
                Sar1::from_file(chr_path).ok()
            })
        });

    if mxmd.models.skinning.is_some() && chr.is_none() {
        error!("Failed to load .arc or .chr skeleton for model with vertex skinning.");
    }

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

/// Load all animations from a `.anm`, `.mot`, or `.motstm_data` file.
pub fn load_animations<P: AsRef<Path>>(anim_path: P) -> Vec<Animation> {
    // TODO: Avoid unwrap and return errors.
    // TODO: Avoid repetition.
    let mut animations = Vec::new();
    if let Ok(sar1) = xc3_lib::sar1::Sar1::from_file(anim_path.as_ref()) {
        // Most xenoblade 2 and xenoblade 3 animations are in sar archives.
        for entry in &sar1.entries {
            match entry.read_data::<xc3_lib::bc::Bc>() {
                Ok(bc) => {
                    if let xc3_lib::bc::BcData::Anim(anim) = bc.data {
                        let animation = Animation::from_anim(&anim);
                        animations.push(animation);
                    }
                }
                Err(e) => error!("error reading {}; {e}", entry.name),
            }
        }
    } else if let Ok(bc) = xc3_lib::bc::Bc::from_file(anim_path.as_ref()) {
        // Some animations are in standalone BC archives.
        if let xc3_lib::bc::BcData::Anim(anim) = bc.data {
            let animation = Animation::from_anim(&anim);
            animations.push(animation);
        }
    } else if let Ok(xbc1) = xc3_lib::xbc1::Xbc1::from_file(anim_path.as_ref()) {
        // Xenoblade 1 DE compresses the sar archive.
        if let Ok(sar1) = xbc1.extract::<xc3_lib::sar1::Sar1>() {
            for entry in &sar1.entries {
                if let Ok(bc) = entry.read_data::<xc3_lib::bc::Bc>() {
                    if let xc3_lib::bc::BcData::Anim(anim) = bc.data {
                        let animation = Animation::from_anim(&anim);
                        animations.push(animation);
                    }
                }
            }
        }
    }
    animations
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
        .find_map(|e| match e.read_data::<xc3_lib::bc::Bc>() {
            Ok(bc) => match bc.data {
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
fn create_materials(materials: &Materials, spch: Option<&shader_database::Spch>) -> Vec<Material> {
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
        unk_type: ShaderUnkType,
    ) -> Option<usize> {
        // TODO: Is this the correct flags check?
        // TODO: This doesn't work for other unk type or lod?
        if (skin_flags & 0x1) == 0 {
            // TODO: Is this actually some sort of flags?
            let lod_index = lod.saturating_sub(1) as usize;
            let weight_lod = self.weight_lods.get(lod_index)?;

            // TODO: bit mask?
            let pass_index = match unk_type {
                ShaderUnkType::Unk0 => 0,
                ShaderUnkType::Unk1 => 1,
                ShaderUnkType::Unk6 => todo!(),
                ShaderUnkType::Unk7 => 3,
                ShaderUnkType::Unk9 => todo!(),
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

#[cfg(test)]
#[macro_export]
macro_rules! assert_hex_eq {
    ($a:expr, $b:expr) => {
        pretty_assertions::assert_str_eq!(hex::encode($a), hex::encode($b))
    };
}
