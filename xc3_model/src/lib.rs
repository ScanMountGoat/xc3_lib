//! # xc3_model
//! xc3_model provides high level data access for the files that make up a model.
//!
//! Each type represents fully compressed and decoded data associated with one or more [xc3_lib] types.
//! This simplifies the processing that needs to be done to access model data
//! and abstracts away most of the game specific complexities.
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
//! let database = ShaderDatabase::from_file("xc3.json")?;
//!
//! let root = xc3_model::load_model("ch01011013.wimdo", Some(&database))?;
//! println!("{}", root.image_textures.len());
//!
//! let roots = xc3_model::load_map("ma59a.wismhd", Some(&database))?;
//! println!("{}", roots[0].image_textures.len());
//! # Ok(())
//! # }
//! ```

use std::{
    borrow::Cow,
    hash::Hash,
    io::Cursor,
    path::{Path, PathBuf},
};

use animation::Animation;
use binrw::{BinRead, BinReaderExt};
use glam::{Mat4, Vec3};
use indexmap::IndexMap;
use log::error;
use material::create_materials;
use shader_database::{ModelPrograms, ShaderDatabase, ShaderProgram};
use texture::{load_textures, load_textures_legacy};
use thiserror::Error;
use vertex::ModelBuffers;
use xc3_lib::{
    apmd::Apmd,
    bc::Bc,
    error::DecompressStreamError,
    hkt::Hkt,
    mibl::Mibl,
    msrd::{
        streaming::{chr_tex_nx_folder, ExtractedTexture},
        Msrd,
    },
    mxmd::{legacy::MxmdLegacy, AlphaTable, Materials, Mxmd},
    sar1::Sar1,
    xbc1::MaybeXbc1,
    ReadFileError,
};

pub use map::{load_map, LoadMapError};
pub use material::{
    texture_layer_assignment, ChannelAssignment, Material, MaterialParameters, OutputAssignment,
    OutputAssignments, Texture, TextureAlphaTest, TextureAssignment,
};
pub use sampler::{AddressMode, FilterMode, Sampler};
pub use skeleton::{Bone, Skeleton};
pub use texture::{ExtractedTextures, ImageFormat, ImageTexture, ViewDimension};
pub use xc3_lib::mxmd::{
    BlendMode, ColorWriteMode, CullMode, DepthFunc, MaterialFlags, MaterialRenderFlags,
    MeshRenderFlags2, MeshRenderPass, RenderPassType, StateFlags, StencilMode, StencilValue,
    TextureUsage,
};

#[cfg(feature = "gltf")]
pub mod gltf;

pub mod animation;
mod map;
mod material;
mod model;
pub mod monolib;
mod sampler;
pub mod shader_database;
mod skeleton;
pub mod skinning;
mod texture;
pub mod vertex;

// TODO: Document why these are different.
// TODO: Come up with a better name
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Clone)]
pub struct ModelRoot {
    pub models: Models,
    /// The vertex data for each [Model].
    pub buffers: ModelBuffers,

    /// The textures selected by each [Material].
    /// This includes all packed and embedded textures after
    /// combining all mip levels.
    pub image_textures: Vec<ImageTexture>,

    // TODO: Do we even need to store the skinning if the weights already have the skinning bone name list?
    pub skeleton: Option<Skeleton>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Clone)]
pub struct MapRoot {
    pub groups: Vec<ModelGroup>,

    /// The textures selected by each [Material].
    /// This includes all packed and embedded textures after
    /// combining all mip levels.
    pub image_textures: Vec<ImageTexture>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Clone)]
pub struct ModelGroup {
    pub models: Vec<Models>,
    /// The vertex data selected by each [Model].
    pub buffers: Vec<ModelBuffers>,
}

// TODO: Should samplers be optional?
// TODO: Come up with a better name?
/// See [Models](xc3_lib::mxmd::Models).
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Clone)]
pub struct Models {
    pub models: Vec<Model>,
    pub materials: Vec<Material>,
    pub samplers: Vec<Sampler>,

    // TODO: Worth storing skinning here?

    // TODO: when is this None?
    pub lod_data: Option<LodData>,

    // TODO: Use none instead of empty?
    /// The name of the controller for each morph target like "mouth_shout".
    pub morph_controller_names: Vec<String>,

    /// The the morph controller names used for animations.
    pub animation_morph_names: Vec<String>,

    // TODO: make this a function instead to avoid dependencies?
    /// The minimum XYZ coordinates of the bounding volume.
    #[cfg_attr(feature = "arbitrary", arbitrary(with = arbitrary_vec3))]
    pub max_xyz: Vec3,

    /// The maximum XYZ coordinates of the bounding volume.
    #[cfg_attr(feature = "arbitrary", arbitrary(with = arbitrary_vec3))]
    pub min_xyz: Vec3,
}

/// See [Model](xc3_lib::mxmd::Model).
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Clone)]
pub struct Model {
    pub meshes: Vec<Mesh>,
    /// Each mesh has an instance for every transform in [instances](#structfield.instances).
    #[cfg_attr(feature = "arbitrary", arbitrary(with = arbitrary_mat4s))]
    pub instances: Vec<Mat4>,
    /// The index of the [ModelBuffers] in [buffers](struct.ModelGroup.html#structfield.buffers).
    /// This will only be non zero for some map models.
    pub model_buffers_index: usize,

    #[cfg_attr(feature = "arbitrary", arbitrary(with = arbitrary_vec3))]
    pub max_xyz: Vec3,
    #[cfg_attr(feature = "arbitrary", arbitrary(with = arbitrary_vec3))]
    pub min_xyz: Vec3,
    pub bounding_radius: f32,
}

/// See [Mesh](xc3_lib::mxmd::Mesh).
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Clone)]
pub struct Mesh {
    pub flags1: u32,
    pub flags2: MeshRenderFlags2,
    pub vertex_buffer_index: usize,
    pub index_buffer_index: usize,
    pub index_buffer_index2: usize,
    pub material_index: usize,
    pub ext_mesh_index: Option<usize>,
    pub lod_item_index: Option<usize>,
    pub base_mesh_index: Option<usize>,
}

/// See [LodData](xc3_lib::mxmd::LodData).
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Clone)]
pub struct LodData {
    pub items: Vec<LodItem>,
    pub groups: Vec<LodGroup>,
}

/// See [LodItem](xc3_lib::mxmd::LodItem).
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Clone)]
pub struct LodItem {
    pub unk2: f32,
    pub index: u8,
    pub unk5: u8,
}

/// See [LodGroup](xc3_lib::mxmd::LodGroup).
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Clone)]
pub struct LodGroup {
    pub base_lod_index: usize,
    pub lod_count: usize,
}

impl LodData {
    /// Returns `true` if a mesh with `lod_item_index` should be rendered
    /// as part of the highest detailed or base level of detail (LOD).
    pub fn is_base_lod(&self, lod_item_index: Option<usize>) -> bool {
        match lod_item_index {
            Some(i) => self.groups.iter().any(|g| g.base_lod_index == i),
            None => true,
        }
    }
}

impl Models {
    pub fn from_models(
        models: &xc3_lib::mxmd::Models,
        materials: &xc3_lib::mxmd::Materials,
        model_programs: Option<&shader_database::ModelPrograms>,
    ) -> Self {
        Self {
            models: models
                .models
                .iter()
                .map(|model| {
                    Model::from_model(model, vec![Mat4::IDENTITY], 0, models.alpha_table.as_ref())
                })
                .collect(),
            materials: create_materials(materials, model_programs),
            samplers: create_samplers(materials),
            lod_data: models.lod_data.as_ref().map(lod_data),
            morph_controller_names: models
                .morph_controllers
                .as_ref()
                .map(|m| m.controllers.iter().map(|c| c.name1.clone()).collect())
                .unwrap_or_default(),
            animation_morph_names: models
                .model_unk1
                .as_ref()
                .map(|u| u.items1.iter().map(|i| i.name.clone()).collect())
                .unwrap_or_default(),
            min_xyz: models.min_xyz.into(),
            max_xyz: models.max_xyz.into(),
        }
    }

    pub fn from_models_legacy(
        models: &xc3_lib::mxmd::legacy::Models,
        materials: &xc3_lib::mxmd::legacy::Materials,
        model_programs: Option<&shader_database::ModelPrograms>,
    ) -> Self {
        // TODO: move material code to material module
        Self {
            models: models.models.iter().map(Model::from_model_legacy).collect(),
            materials: materials
                .materials
                .iter()
                .map(|m| Material {
                    name: m.name.clone(),
                    flags: MaterialFlags::from(0u32),
                    render_flags: MaterialRenderFlags::from(0u32),
                    state_flags: m.state_flags,
                    color: m.color,
                    textures: m
                        .textures
                        .iter()
                        .map(|t| Texture {
                            image_texture_index: t.texture_index as usize,
                            sampler_index: 0,
                        })
                        .collect(),
                    alpha_test: materials.alpha_test_textures.first().and_then(|a| {
                        // TODO: alpha test texture index in material?
                        m.textures
                            .iter()
                            .position(|t| t.texture_index == a.texture_index)
                            .map(|texture_index| TextureAlphaTest {
                                texture_index,
                                channel_index: 3,
                            })
                    }),
                    alpha_test_ref: [0; 4],
                    shader: get_shader_legacy(m, model_programs),
                    technique_index: 0,
                    pass_type: match m.techniques[0].unk1 {
                        xc3_lib::mxmd::legacy::UnkPassType::Unk0 => RenderPassType::Unk0,
                        xc3_lib::mxmd::legacy::UnkPassType::Unk1 => RenderPassType::Unk1,
                        // TODO: How to handle these variants?
                        xc3_lib::mxmd::legacy::UnkPassType::Unk2 => RenderPassType::Unk0,
                        xc3_lib::mxmd::legacy::UnkPassType::Unk3 => RenderPassType::Unk0,
                        xc3_lib::mxmd::legacy::UnkPassType::Unk5 => RenderPassType::Unk0,
                        xc3_lib::mxmd::legacy::UnkPassType::Unk8 => RenderPassType::Unk0,
                    },
                    parameters: MaterialParameters {
                        alpha_test_ref: 0.0,
                        tex_matrix: None,
                        work_float4: None,
                        work_color: None,
                    },
                    work_values: Vec::new(),
                    shader_vars: Vec::new(),
                    work_callbacks: Vec::new(),
                    m_unks1_1: 0,
                    m_unks1_2: 0,
                    m_unks1_3: 0,
                    m_unks1_4: 0,
                    m_unks2_2: 0,
                    m_unks3_1: 0,
                })
                .collect(),
            samplers: Vec::new(),
            lod_data: None,
            morph_controller_names: Vec::new(),
            animation_morph_names: Vec::new(),
            max_xyz: models.max_xyz.into(),
            min_xyz: models.min_xyz.into(),
        }
    }
}

fn get_shader_legacy(
    material: &xc3_lib::mxmd::legacy::Material,
    model_programs: Option<&ModelPrograms>,
) -> Option<ShaderProgram> {
    // TODO: Some alpha materials have two techniques?
    let program_index = material.techniques.last()?.technique_index as usize;
    let program = model_programs?.programs.get(program_index)?;

    // The texture outputs are different in Xenoblade X compared to Switch.
    // We handle this here to avoid needing to regenerate the database for updates.
    // G-Buffer Textures:
    // 0: lighting (ao * ???, alpha is specular brdf?)
    // 1: color (alpha is emission?)
    // 2: normal (only xy)
    // 3: specular (alpha is spec?)
    // 4: depth (alpha is glossiness)
    let output_dependencies = if program.output_dependencies.len() > 4 {
        program
            .output_dependencies
            .iter()
            .filter_map(|(k, v)| match k.as_str() {
                "o0.x" => Some(("o2.z".into(), v.clone())),
                "o1.x" => Some(("o0.x".into(), v.clone())),
                "o1.y" => Some(("o0.y".into(), v.clone())),
                "o1.z" => Some(("o0.z".into(), v.clone())),
                "o1.w" => Some(("o0.w".into(), v.clone())),
                // The normal output has only RG channels.
                "o2.x" => Some(("o2.x".into(), v.clone())),
                "o2.y" => Some(("o2.y".into(), v.clone())),
                "o3.x" => Some(("o5.x".into(), v.clone())),
                "o3.y" => Some(("o5.y".into(), v.clone())),
                "o3.z" => Some(("o5.z".into(), v.clone())),
                "o3.w" => Some(("o5.w".into(), v.clone())),
                "o4.x" => Some(("o4.x".into(), v.clone())),
                "o4.y" => Some(("o4.y".into(), v.clone())),
                "o4.z" => Some(("o4.z".into(), v.clone())),
                "o4.w" => Some(("o1.y".into(), v.clone())),
                _ => None,
            })
            .collect()
    } else {
        // Some shaders only write to color and shouldn't be remapped.
        program.output_dependencies.clone()
    };

    Some(ShaderProgram {
        output_dependencies,
    })
}

fn lod_data(data: &xc3_lib::mxmd::LodData) -> LodData {
    LodData {
        items: data
            .items
            .iter()
            .map(|i| LodItem {
                unk2: i.unk2,
                index: i.index,
                unk5: i.unk5,
            })
            .collect(),
        groups: data
            .groups
            .iter()
            .map(|g| LodGroup {
                base_lod_index: g.base_lod_index as usize,
                lod_count: g.lod_count as usize,
            })
            .collect(),
    }
}

impl Model {
    pub fn from_model(
        model: &xc3_lib::mxmd::Model,
        instances: Vec<Mat4>,
        model_buffers_index: usize,
        alpha_table: Option<&AlphaTable>,
    ) -> Self {
        let meshes = model
            .meshes
            .iter()
            .map(|mesh| {
                // TODO: Is there also a flag that disables the ext mesh?
                let ext_mesh_index = if let Some(a) = alpha_table {
                    // This uses 1-based indexing so 0 is disabled.
                    if matches!(a.items.get(mesh.alpha_table_index as usize), Some((0, _))) {
                        None
                    } else {
                        Some(mesh.ext_mesh_index as usize)
                    }
                } else {
                    Some(mesh.ext_mesh_index as usize)
                };

                // TODO: This should also be None for xc1 and xc2?
                let base_mesh_index = mesh.base_mesh_index.try_into().ok();

                let lod_item_index = if mesh.lod_item_index > 0 {
                    Some(mesh.lod_item_index as usize - 1)
                } else {
                    None
                };

                Mesh {
                    flags1: mesh.flags1,
                    flags2: mesh.flags2,
                    vertex_buffer_index: mesh.vertex_buffer_index as usize,
                    index_buffer_index: mesh.index_buffer_index as usize,
                    index_buffer_index2: mesh.index_buffer_index2 as usize,
                    material_index: mesh.material_index as usize,
                    ext_mesh_index,
                    lod_item_index,
                    base_mesh_index,
                }
            })
            .collect();

        Self {
            meshes,
            instances,
            model_buffers_index,
            max_xyz: model.max_xyz.into(),
            min_xyz: model.min_xyz.into(),
            bounding_radius: model.bounding_radius,
        }
    }

    pub fn from_model_legacy(model: &xc3_lib::mxmd::legacy::Model) -> Self {
        let meshes = model
            .meshes
            .iter()
            .map(|mesh| Mesh {
                flags1: mesh.flags1,
                flags2: mesh.flags2.try_into().unwrap(), // TODO: same type?
                vertex_buffer_index: mesh.vertex_buffer_index as usize,
                index_buffer_index: mesh.index_buffer_index as usize,
                index_buffer_index2: 0,
                material_index: mesh.material_index as usize,
                ext_mesh_index: None,
                lod_item_index: None,
                base_mesh_index: None,
            })
            .collect();

        Self {
            meshes,
            instances: vec![Mat4::IDENTITY],
            model_buffers_index: 0,
            max_xyz: model.max_xyz.into(),
            min_xyz: model.min_xyz.into(),
            bounding_radius: model.bounding_radius,
        }
    }
}

#[derive(Debug, Error)]
pub enum LoadModelError {
    #[error("error reading wimdo file from {path:?}")]
    Wimdo {
        path: PathBuf,
        #[source]
        source: binrw::Error,
    },

    #[error("error extracting texture from wimdo file")]
    WimdoPackedTexture {
        #[source]
        source: binrw::Error,
    },

    #[error("error reading vertex data")]
    VertexData(binrw::Error),

    #[error("failed to find Mxmd in Apmd file")]
    MissingApmdMxmdEntry,

    #[error("expected packed wimdo vertex data but found none")]
    MissingMxmdVertexData,

    #[error("error loading image texture")]
    Image(#[from] texture::CreateImageTextureError),

    #[error("error decompressing stream")]
    Stream(#[from] xc3_lib::error::DecompressStreamError),

    #[error("error extracting stream data")]
    ExtractFiles(#[from] xc3_lib::msrd::streaming::ExtractFilesError),

    #[error("error reading legacy wismt streaming")]
    WismtLegacy(#[source] ReadFileError),

    #[error("error reading wismt streaming data")]
    Wismt(#[source] ReadFileError),
}

// TODO: Take an iterator for wimdo paths and merge to support xc1?
/// Load a model from a `.wimdo` or `.pcmdo` file.
/// The corresponding `.wismt` or `.pcsmt` and `.chr` or `.arc` should be in the same directory.
///
/// # Examples
/// Most models use a single file and return a single root.
///
/// ``` rust no_run
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// use xc3_model::{load_model, shader_database::ShaderDatabase};
///
/// // Shulk's hair
/// let database = ShaderDatabase::from_file("xc1.json")?;
/// let root = load_model("xeno1/chr/pc/pc010101.wimdo", Some(&database));
///
/// // Pyra
/// let database = ShaderDatabase::from_file("xc2.json")?;
/// let root = load_model("xeno2/model/bl/bl000101.wimdo", Some(&database));
///
/// // Mio military uniform
/// let database = ShaderDatabase::from_file("xc3.json")?;
/// let root = load_model("xeno3/chr/ch/ch01027000.wimdo", Some(&database));
/// # Ok(())
/// # }
/// ```
///
/// For models split into multiple files, simply combine the roots.
/// ```rust no_run
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # use xc3_model::{load_model, shader_database::ShaderDatabase};
/// let database = ShaderDatabase::from_file("xc1.json")?;
///
/// // Shulk's main outfit.
/// let paths = [
///     "xeno1/chr/pc/pc010201.wimdo",
///     "xeno1/chr/pc/pc010202.wimdo",
///     "xeno1/chr/pc/pc010203.wimdo",
///     "xeno1/chr/pc/pc010204.wimdo",
///     "xeno1/chr/pc/pc010205.wimdo",
///     "xeno1/chr/pc/pc010109.wimdo",
/// ];
///
/// let mut roots = Vec::new();
/// for path in paths {
///     let root = xc3_model::load_model(path, Some(&database))?;
///     roots.push(root);
/// }
/// # Ok(())
/// # }
/// ```
pub fn load_model<P: AsRef<Path>>(
    wimdo_path: P,
    shader_database: Option<&ShaderDatabase>,
) -> Result<ModelRoot, LoadModelError> {
    let wimdo_path = wimdo_path.as_ref();

    let mxmd = load_wimdo(wimdo_path)?;
    let chr_tex_folder = chr_tex_nx_folder(wimdo_path);

    // Desktop PC models aren't used in game but are straightforward to support.
    let is_pc = wimdo_path.extension().and_then(|e| e.to_str()) == Some("pcmdo");
    let wismt_path = if is_pc {
        wimdo_path.with_extension("pcsmt")
    } else {
        wimdo_path.with_extension("wismt")
    };
    let streaming_data = StreamingData::new(&mxmd, &wismt_path, is_pc, chr_tex_folder.as_deref())?;

    let model_name = model_name(wimdo_path);
    let model_programs = shader_database.and_then(|database| database.model(&model_name));

    let chr = load_chr(wimdo_path, model_name);

    ModelRoot::from_mxmd_model(&mxmd, chr, &streaming_data, model_programs.as_ref())
}

fn load_chr(wimdo_path: &Path, model_name: String) -> Option<Sar1> {
    // TODO: Does every wimdo have a chr file?
    // TODO: Does something control the chr name used?
    // TODO: This won't load the base skeleton chr for xc3.
    Sar1::from_file(wimdo_path.with_extension("chr"))
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
        })
}

// TODO: separate legacy module with its own error type?
/// Load a model from a `.camdo` file.
/// The corresponding `.casmt`should be in the same directory.
///
/// # Examples
///
/// ``` rust no_run
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// use xc3_model::{load_model_legacy, shader_database::ShaderDatabase};
///
/// // Tatsu
/// let database = ShaderDatabase::from_file("xcx.json")?;
/// let root = load_model_legacy("xenox/chr_np/np009001.camdo", Some(&database))?;
/// # Ok(())
/// # }
/// ```
pub fn load_model_legacy<P: AsRef<Path>>(
    camdo_path: P,
    shader_database: Option<&ShaderDatabase>,
) -> Result<ModelRoot, LoadModelError> {
    // TODO: avoid unwrap.
    let camdo_path = camdo_path.as_ref();
    let mxmd: MxmdLegacy = MxmdLegacy::from_file(camdo_path).unwrap();
    let casmt = mxmd
        .streaming
        .as_ref()
        .map(|_| std::fs::read(camdo_path.with_extension("casmt")).unwrap());

    let model_name = model_name(camdo_path);
    let hkt_path = camdo_path
        .with_file_name(model_name.clone() + "_rig")
        .with_extension("hkt");
    let hkt = Hkt::from_file(hkt_path).ok();

    let model_programs = shader_database.and_then(|database| database.model(&model_name));

    ModelRoot::from_mxmd_model_legacy(&mxmd, casmt, hkt.as_ref(), model_programs.as_ref())
}

impl ModelRoot {
    // TODO: fuzz test this?
    /// Load models from parsed file data for Xenoblade 1 DE, Xenoblade 2, or Xenoblade 3.
    pub fn from_mxmd_model(
        mxmd: &Mxmd,
        chr: Option<Sar1>,
        streaming_data: &StreamingData<'_>,
        model_programs: Option<&shader_database::ModelPrograms>,
    ) -> Result<Self, LoadModelError> {
        if mxmd.models.skinning.is_some() && chr.is_none() {
            error!("Failed to load .arc or .chr skeleton for model with vertex skinning.");
        }

        // TODO: Store the skeleton with the root since this is the only place we actually make one?
        // TODO: Some sort of error if maps have any skinning set?
        let skeleton = create_skeleton(chr.as_ref(), mxmd.models.skinning.as_ref());

        let buffers =
            ModelBuffers::from_vertex_data(&streaming_data.vertex, mxmd.models.skinning.as_ref())
                .map_err(LoadModelError::VertexData)?;

        let models = Models::from_models(&mxmd.models, &mxmd.materials, model_programs);

        let image_textures = load_textures(&streaming_data.textures)?;

        Ok(Self {
            models,
            buffers,
            image_textures,
            skeleton,
        })
    }

    // TODO: fuzz test this?
    /// Load models from legacy parsed file data for Xenoblade X.
    pub fn from_mxmd_model_legacy(
        mxmd: &MxmdLegacy,
        casmt: Option<Vec<u8>>,
        hkt: Option<&Hkt>,
        model_programs: Option<&shader_database::ModelPrograms>,
    ) -> Result<Self, LoadModelError> {
        let skeleton = hkt.map(Skeleton::from_legacy_skeleton);

        let buffers = ModelBuffers::from_vertex_data_legacy(&mxmd.vertex, &mxmd.models)
            .map_err(LoadModelError::VertexData)?;

        let models = Models::from_models_legacy(&mxmd.models, &mxmd.materials, model_programs);

        let image_textures = load_textures_legacy(mxmd, casmt)?;

        Ok(Self {
            models,
            buffers,
            image_textures,
            skeleton,
        })
    }
}

// TODO: move this to xc3_lib?
#[derive(BinRead)]
enum Wimdo {
    Mxmd(Box<Mxmd>),
    Apmd(Apmd),
}

fn load_wimdo(wimdo_path: &Path) -> Result<Mxmd, LoadModelError> {
    let mut reader = Cursor::new(
        std::fs::read(wimdo_path).map_err(|e| LoadModelError::Wimdo {
            path: wimdo_path.to_owned(),
            source: e.into(),
        })?,
    );
    let wimdo: Wimdo = reader.read_le().map_err(|e| LoadModelError::Wimdo {
        path: wimdo_path.to_owned(),
        source: e,
    })?;
    match wimdo {
        Wimdo::Mxmd(mxmd) => Ok(*mxmd),
        Wimdo::Apmd(apmd) => apmd
            .entries
            .iter()
            .find_map(|e| {
                if e.entry_type == xc3_lib::apmd::EntryType::Mxmd {
                    Some(Mxmd::from_bytes(&e.entry_data))
                } else {
                    None
                }
            })
            .map_or(Err(LoadModelError::MissingApmdMxmdEntry), |r| {
                r.map_err(|e| LoadModelError::Wimdo {
                    path: wimdo_path.to_owned(),
                    source: e,
                })
            }),
    }
}

// Use Cow::Borrowed to avoid copying data embedded in the mxmd.
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug)]
pub struct StreamingData<'a> {
    pub vertex: Cow<'a, xc3_lib::vertex::VertexData>,
    pub textures: ExtractedTextures,
}

impl<'a> StreamingData<'a> {
    pub fn new(
        mxmd: &'a Mxmd,
        wismt_path: &Path,
        is_pc: bool,
        chr_tex_folder: Option<&Path>,
    ) -> Result<StreamingData<'a>, LoadModelError> {
        // Handle the different ways to store the streaming data.
        mxmd.streaming
            .as_ref()
            .map(|streaming| match &streaming.inner {
                xc3_lib::msrd::StreamingInner::StreamingLegacy(legacy) => {
                    let data = std::fs::read(wismt_path).map_err(|e| {
                        LoadModelError::WismtLegacy(ReadFileError {
                            path: wismt_path.to_owned(),
                            source: e.into(),
                        })
                    })?;

                    // TODO: Error on missing vertex data?
                    Ok(StreamingData {
                        vertex: Cow::Borrowed(
                            mxmd.vertex_data
                                .as_ref()
                                .ok_or(LoadModelError::MissingMxmdVertexData)?,
                        ),
                        textures: ExtractedTextures::Switch(legacy.extract_textures(&data)?),
                    })
                }
                xc3_lib::msrd::StreamingInner::Streaming(_) => {
                    let msrd = Msrd::from_file(wismt_path).map_err(LoadModelError::Wismt)?;
                    if is_pc {
                        let (vertex, _, textures) = msrd.extract_files_pc()?;

                        Ok(StreamingData {
                            vertex: Cow::Owned(vertex),
                            textures: ExtractedTextures::Pc(textures),
                        })
                    } else {
                        let (vertex, _, textures) = msrd.extract_files(chr_tex_folder)?;

                        Ok(StreamingData {
                            vertex: Cow::Owned(vertex),
                            textures: ExtractedTextures::Switch(textures),
                        })
                    }
                }
            })
            .unwrap_or_else(|| {
                Ok(StreamingData {
                    vertex: Cow::Borrowed(
                        mxmd.vertex_data
                            .as_ref()
                            .ok_or(LoadModelError::MissingMxmdVertexData)?,
                    ),
                    textures: ExtractedTextures::Switch(match &mxmd.packed_textures {
                        Some(textures) => textures
                            .textures
                            .iter()
                            .map(|t| {
                                Ok(ExtractedTexture {
                                    name: t.name.clone(),
                                    usage: t.usage,
                                    low: Mibl::from_bytes(&t.mibl_data).map_err(|e| {
                                        LoadModelError::WimdoPackedTexture { source: e }
                                    })?,
                                    high: None,
                                })
                            })
                            .collect::<Result<Vec<_>, LoadModelError>>()?,
                        None => Vec::new(),
                    }),
                })
            })
    }
}

#[derive(BinRead)]
enum AnimFile {
    Sar1(MaybeXbc1<Sar1>),
    Bc(Box<Bc>),
}

/// Load all animations from a `.anm`, `.mot`, or `.motstm_data` file.
///
/// # Examples
/// ``` rust no_run
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// // Fiora
/// let animations = xc3_model::load_animations("xeno1/chr/pc/mp080000.mot")?;
/// println!("{}", animations.len());
///
/// // Pyra
/// let animations = xc3_model::load_animations("xeno2/model/bl/bl000101.mot")?;
/// println!("{}", animations.len());
///
/// // Mio military uniform
/// let animations = xc3_model::load_animations("xeno3/chr/ch/ch01027000_event.mot")?;
/// println!("{}", animations.len());
/// # Ok(())
/// # }
/// ```
pub fn load_animations<P: AsRef<Path>>(
    anim_path: P,
) -> Result<Vec<Animation>, DecompressStreamError> {
    let mut reader = Cursor::new(std::fs::read(anim_path)?);
    let anim_file: AnimFile = reader.read_le()?;

    let mut animations = Vec::new();

    // Most animations are in sar1 archives.
    // Xenoblade 1 DE compresses the sar1 archive.
    // Some animations are in standalone BC files.
    match anim_file {
        AnimFile::Sar1(sar1) => match sar1 {
            MaybeXbc1::Uncompressed(sar1) => {
                for entry in &sar1.entries {
                    if let Ok(bc) = entry.read_data() {
                        add_bc_animations(&mut animations, bc);
                    }
                }
            }
            MaybeXbc1::Xbc1(xbc1) => {
                let sar1: Sar1 = xbc1.extract()?;
                for entry in &sar1.entries {
                    if let Ok(bc) = entry.read_data() {
                        add_bc_animations(&mut animations, bc);
                    }
                }
            }
        },
        AnimFile::Bc(bc) => {
            add_bc_animations(&mut animations, *bc);
        }
    }

    Ok(animations)
}

fn add_bc_animations(animations: &mut Vec<Animation>, bc: Bc) {
    if let xc3_lib::bc::BcData::Anim(anim) = bc.data {
        let animation = Animation::from_anim(&anim);
        animations.push(animation);
    }
}

fn create_samplers(materials: &Materials) -> Vec<Sampler> {
    materials
        .samplers
        .as_ref()
        .map(|samplers| samplers.samplers.iter().map(|s| s.flags.into()).collect())
        .unwrap_or_default()
}

fn create_skeleton(
    chr: Option<&Sar1>,
    skinning: Option<&xc3_lib::mxmd::Skinning>,
) -> Option<Skeleton> {
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

    Some(Skeleton::from_skeleton(&skel.skeleton, skinning?))
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

#[cfg(feature = "arbitrary")]
fn arbitrary_vec2s(u: &mut arbitrary::Unstructured) -> arbitrary::Result<Vec<glam::Vec2>> {
    let len = u.arbitrary_len::<[f32; 2]>()?;
    let mut elements = Vec::with_capacity(len);
    for _ in 0..len {
        let array: [f32; 2] = u.arbitrary()?;
        let element = glam::Vec2::from_array(array);
        elements.push(element);
    }
    Ok(elements)
}

#[cfg(feature = "arbitrary")]
fn arbitrary_vec3(u: &mut arbitrary::Unstructured) -> arbitrary::Result<glam::Vec3> {
    let array: [f32; 3] = u.arbitrary()?;
    Ok(glam::Vec3::from_array(array))
}

#[cfg(feature = "arbitrary")]
fn arbitrary_vec3s(u: &mut arbitrary::Unstructured) -> arbitrary::Result<Vec<glam::Vec3>> {
    let len = u.arbitrary_len::<[f32; 3]>()?;
    let mut elements = Vec::with_capacity(len);
    for _ in 0..len {
        let array: [f32; 3] = u.arbitrary()?;
        let element = glam::Vec3::from_array(array);
        elements.push(element);
    }
    Ok(elements)
}

#[cfg(feature = "arbitrary")]
fn arbitrary_vec4s(u: &mut arbitrary::Unstructured) -> arbitrary::Result<Vec<glam::Vec4>> {
    let len = u.arbitrary_len::<[f32; 4]>()?;
    let mut elements = Vec::with_capacity(len);
    for _ in 0..len {
        let array: [f32; 4] = u.arbitrary()?;
        let element = glam::Vec4::from_array(array);
        elements.push(element);
    }
    Ok(elements)
}

#[cfg(feature = "arbitrary")]
fn arbitrary_mat4(u: &mut arbitrary::Unstructured) -> arbitrary::Result<glam::Mat4> {
    let array: [f32; 16] = u.arbitrary()?;
    Ok(glam::Mat4::from_cols_array(&array))
}

#[cfg(feature = "arbitrary")]
fn arbitrary_mat4s(u: &mut arbitrary::Unstructured) -> arbitrary::Result<Vec<glam::Mat4>> {
    let len = u.arbitrary_len::<[f32; 16]>()?;
    let mut elements = Vec::with_capacity(len);
    for _ in 0..len {
        let array: [f32; 16] = u.arbitrary()?;
        let element = glam::Mat4::from_cols_array(&array);
        elements.push(element);
    }
    Ok(elements)
}

#[cfg(test)]
#[macro_export]
macro_rules! assert_hex_eq {
    ($a:expr, $b:expr) => {
        pretty_assertions::assert_str_eq!(hex::encode($a), hex::encode($b))
    };
}

/// A trait for mapping unique items to an index.
pub trait IndexMapExt<T> {
    /// The index value associated with `key`.
    /// Inserts `key` with an index equal to the current length if not present.
    fn entry_index(&mut self, key: T) -> usize;
}

impl<T> IndexMapExt<T> for IndexMap<T, usize>
where
    T: Hash + Eq,
{
    fn entry_index(&mut self, key: T) -> usize {
        let new_value = self.len();
        *self.entry(key).or_insert(new_value)
    }
}
