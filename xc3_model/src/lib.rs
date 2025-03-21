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
//! let database = ShaderDatabase::from_file("xc3.bin")?;
//!
//! let root = xc3_model::load_model("ch01011013.wimdo", Some(&database))?;
//! println!("{}", root.image_textures.len());
//!
//! let roots = xc3_model::load_map("ma59a.wismhd", Some(&database))?;
//! println!("{}", roots[0].image_textures.len());
//! # Ok(())
//! # }
//! ```

use std::{borrow::Cow, hash::Hash, io::Cursor, path::Path};

use animation::Animation;
use binrw::{BinRead, BinReaderExt, Endian};
use glam::{Mat4, Vec3};
use indexmap::IndexMap;
use log::error;
use material::{create_materials, create_materials_samplers_legacy};
use shader_database::ShaderDatabase;
use skinning::{create_skinning, Skinning};
use texture::{load_textures, load_textures_legacy};
use thiserror::Error;
use vertex::ModelBuffers;
use xc3_lib::{
    apmd::Apmd,
    bc::{skel::Skel, Bc},
    error::DecompressStreamError,
    hkt::Hkt,
    mibl::Mibl,
    msrd::{
        streaming::{chr_folder, ExtractedTexture},
        Msrd,
    },
    mxmd::{legacy::MxmdLegacy, AlphaTable, Materials, Mxmd},
    sar1::Sar1,
    xbc1::MaybeXbc1,
    ReadFileError,
};

pub use collision::load_collisions;
pub use map::{load_map, LoadMapError};
use material::{Material, Texture};
pub use sampler::{AddressMode, FilterMode, Sampler};
pub use skeleton::{Bone, Skeleton};
pub use texture::{ExtractedTextures, ImageFormat, ImageTexture, ViewDimension};
pub use transform::Transform;
pub use xc3_lib::mxmd::{MeshRenderFlags2, MeshRenderPass};

#[cfg(feature = "gltf")]
pub mod gltf;

pub mod animation;
pub mod collision;
mod map;
pub mod material;
mod model;
pub mod monolib;
mod sampler;
pub mod shader_database;
mod skeleton;
pub mod skinning;
mod texture;
mod transform;
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

    // TODO: should skinning information be combined with the skeleton?
    pub skinning: Option<Skinning>,

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
        texture_indices: Option<&[u16]>,
        spch: &xc3_lib::spch::Spch,
        shader_database: Option<&ShaderDatabase>,
    ) -> Self {
        Self {
            models: models
                .models
                .iter()
                .map(|model| {
                    Model::from_model(model, vec![Mat4::IDENTITY], 0, models.alpha_table.as_ref())
                })
                .collect(),
            materials: create_materials(materials, texture_indices, spch, shader_database),
            samplers: create_samplers(materials),
            skinning: models.skinning.as_ref().map(create_skinning),
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
        shaders: Option<&xc3_lib::mxmd::legacy::Shaders>,
        shader_database: Option<&ShaderDatabase>,
        texture_indices: &[u16],
    ) -> Self {
        let (materials, samplers) =
            create_materials_samplers_legacy(materials, texture_indices, shaders, shader_database);
        Self {
            models: models.models.iter().map(Model::from_model_legacy).collect(),
            materials,
            samplers,
            lod_data: None,
            skinning: None, // TODO: how to set this?
            morph_controller_names: Vec::new(),
            animation_morph_names: Vec::new(),
            max_xyz: models.max_xyz.into(),
            min_xyz: models.min_xyz.into(),
        }
    }
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
                flags2: mesh
                    .flags2
                    .try_into()
                    .unwrap_or(MeshRenderFlags2::new(MeshRenderPass::Unk0, 0u8.into())), // TODO: same type?
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
    #[error("error reading wimdo file")]
    Wimdo(#[source] ReadFileError),

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

    #[error("expected packed wimdo shader data but found none")]
    MissingMxmdShaderData,

    #[error("error loading image texture")]
    Image(#[from] texture::CreateImageTextureError),

    #[error("error decompressing stream")]
    Stream(#[from] xc3_lib::error::DecompressStreamError),

    #[error("error extracting stream data")]
    ExtractFiles(#[from] xc3_lib::msrd::streaming::ExtractFilesError),

    #[error("error reading legacy wismt streaming file")]
    WismtLegacy(#[source] ReadFileError),

    #[error("error reading wismt streaming file")]
    Wismt(#[source] ReadFileError),
}

#[derive(Debug, Error)]
pub enum LoadModelLegacyError {
    #[error("error reading camdo file")]
    Camdo(#[source] ReadFileError),

    #[error("error reading vertex data")]
    VertexData(binrw::Error),

    #[error("error loading image texture")]
    Image(#[from] texture::CreateImageTextureError),

    #[error("error reading casmt streaming file")]
    Casmt(#[source] std::io::Error),
}

#[derive(Debug, Error)]
pub enum CreateModelError {
    #[error("error extracting stream data")]
    ExtractFiles(#[from] xc3_lib::msrd::streaming::ExtractFilesError),
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
/// let database = ShaderDatabase::from_file("xc1.bin")?;
/// let root = load_model("xeno1/chr/pc/pc010101.wimdo", Some(&database));
///
/// // Pyra
/// let database = ShaderDatabase::from_file("xc2.bin")?;
/// let root = load_model("xeno2/model/bl/bl000101.wimdo", Some(&database));
///
/// // Mio military uniform
/// let database = ShaderDatabase::from_file("xc3.bin")?;
/// let root = load_model("xeno3/chr/ch/ch01027000.wimdo", Some(&database));
/// # Ok(())
/// # }
/// ```
///
/// For models split into multiple files, simply combine the roots.
/// ```rust no_run
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # use xc3_model::{load_model, shader_database::ShaderDatabase};
/// let database = ShaderDatabase::from_file("xc1.bin")?;
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
#[tracing::instrument(skip_all)]
pub fn load_model<P: AsRef<Path>>(
    wimdo_path: P,
    shader_database: Option<&ShaderDatabase>,
) -> Result<ModelRoot, LoadModelError> {
    let wimdo_path = wimdo_path.as_ref();

    let mxmd = load_wimdo(wimdo_path)?;
    let chr_tex_folder = chr_folder(wimdo_path);

    // Desktop PC models aren't used in game but are straightforward to support.
    let is_pc = wimdo_path.extension().and_then(|e| e.to_str()) == Some("pcmdo");
    let wismt_path = if is_pc {
        wimdo_path.with_extension("pcsmt")
    } else {
        wimdo_path.with_extension("wismt")
    };
    let streaming_data =
        StreamingData::from_files(&mxmd, &wismt_path, is_pc, chr_tex_folder.as_deref())?;

    let model_name = model_name(wimdo_path);
    let skel = load_skel(wimdo_path, &model_name);

    ModelRoot::from_mxmd_model(&mxmd, skel, &streaming_data, shader_database)
}

pub fn load_skel(wimdo: &Path, model_name: &str) -> Option<xc3_lib::bc::skel::Skel> {
    load_chr(wimdo, model_name)
        .and_then(|chr| {
            // Xenoblade 3 embeds skeletons in chr files.
            chr.entries
                .iter()
                .find_map(|e| match e.read_data::<xc3_lib::bc::Bc>() {
                    Ok(bc) => match bc.data {
                        xc3_lib::bc::BcData::Skel(skel) => Some(skel),
                        _ => None,
                    },
                    _ => None,
                })
        })
        .or_else(|| {
            // Xenoblade X DE uses a file for just the skeleton.
            let skel_path = wimdo.with_file_name(format!("{model_name}_rig.skl"));
            Bc::from_file(skel_path).ok().and_then(|bc| match bc.data {
                xc3_lib::bc::BcData::Skel(skel) => Some(skel),
                _ => None,
            })
        })
}

fn load_chr(wimdo: &Path, model_name: &str) -> Option<Sar1> {
    // TODO: Does every wimdo have a chr file?
    // TODO: Does something control the chr name used?
    // Try to find the base skeleton file first if it exists.
    // This avoids loading incomplete skeletons specific to each model.
    // XC1: pc010101.wimdo -> pc010000.chr.
    // XC3: ch01012013.wimdo -> ch01012000.chr.
    let base_name = base_chr_name(model_name);
    Sar1::from_file(wimdo.with_file_name(&base_name).with_extension("chr"))
        .ok()
        .or_else(|| Sar1::from_file(wimdo.with_file_name(&base_name).with_extension("arc")).ok())
        .or_else(|| Sar1::from_file(wimdo.with_extension("chr")).ok())
        .or_else(|| Sar1::from_file(wimdo.with_extension("arc")).ok())
        .or_else(|| {
            // Keep trying with more 0's at the end to match in game naming conventions.
            // This usually only requires one additional 0.
            // XC3: ch01056013.wimdo -> ch01056010.chr.
            (0..model_name.len()).find_map(|i| {
                let mut chr_name = model_name.to_string();
                chr_name.replace_range(chr_name.len() - i.., &"0".repeat(i));
                let chr_path = wimdo.with_file_name(chr_name).with_extension("chr");
                Sar1::from_file(chr_path).ok()
            })
        })
}

fn base_chr_name(model_name: &str) -> String {
    let mut chr_name = model_name.to_string();
    chr_name.replace_range(chr_name.len() - 3.., "000");
    chr_name
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
/// let database = ShaderDatabase::from_file("xcx.bin")?;
/// let root = load_model_legacy("xenox/chr_np/np009001.camdo", Some(&database))?;
/// # Ok(())
/// # }
/// ```
#[tracing::instrument(skip_all)]
pub fn load_model_legacy<P: AsRef<Path>>(
    camdo_path: P,
    shader_database: Option<&ShaderDatabase>,
) -> Result<ModelRoot, LoadModelLegacyError> {
    let camdo_path = camdo_path.as_ref();
    let mxmd = MxmdLegacy::from_file(camdo_path).map_err(LoadModelLegacyError::Camdo)?;

    let casmt = mxmd
        .streaming
        .as_ref()
        .map(|_| {
            std::fs::read(camdo_path.with_extension("casmt")).map_err(LoadModelLegacyError::Casmt)
        })
        .transpose()?;

    let model_name = model_name(camdo_path);
    let hkt_path = camdo_path.with_file_name(format!("{model_name}_rig.hkt"));
    let hkt = Hkt::from_file(hkt_path).ok();

    ModelRoot::from_mxmd_model_legacy(&mxmd, casmt, hkt.as_ref(), shader_database)
}

impl ModelRoot {
    /// Load models from parsed file data for Xenoblade 1 DE, Xenoblade 2, or Xenoblade 3.
    pub fn from_mxmd_model(
        mxmd: &Mxmd,
        skel: Option<Skel>,
        streaming_data: &StreamingData<'_>,
        shader_database: Option<&ShaderDatabase>,
    ) -> Result<Self, LoadModelError> {
        match &mxmd.inner {
            xc3_lib::mxmd::MxmdInner::V112(mxmd) => {
                if mxmd.models.skinning.is_some() && skel.is_none() {
                    error!("Failed to load .arc or .skel skeleton for model with vertex skinning.");
                }

                // TODO: Store the skeleton with the root since this is the only place we actually make one?
                // TODO: Some sort of error if maps have any skinning set?
                let skeleton = create_skeleton(skel.as_ref(), mxmd.models.skinning.as_ref());

                let buffers = match &streaming_data.vertex {
                    VertexData::Modern(vertex) => {
                        ModelBuffers::from_vertex_data(vertex, mxmd.models.skinning.as_ref())
                            .map_err(LoadModelError::VertexData)?
                    }
                    VertexData::Legacy(_) => {
                        // TODO: Rework code since this shouldn't happen.
                        todo!()
                    }
                };

                let models = Models::from_models(
                    &mxmd.models,
                    &mxmd.materials,
                    streaming_data.texture_indices.as_deref(),
                    &streaming_data.spch,
                    shader_database,
                );

                let image_textures = load_textures(&streaming_data.textures)?;

                Ok(Self {
                    models,
                    buffers,
                    image_textures,
                    skeleton,
                })
            }
            xc3_lib::mxmd::MxmdInner::V40(mxmd) => {
                let buffers = match &streaming_data.vertex {
                    VertexData::Modern(_) => {
                        // TODO: Rework code since this shouldn't happen.
                        todo!()
                    }
                    VertexData::Legacy(vertex) => {
                        ModelBuffers::from_vertex_data_legacy(vertex, &mxmd.models, Endian::Little)
                            .map_err(LoadModelError::VertexData)?
                    }
                };

                let image_textures = load_textures(&streaming_data.textures)?;
                // TODO: Can these be remapped like with xcx?
                let texture_indices: Vec<_> = (0..image_textures.len() as u16).collect();

                // TODO: Create special loading function instead of making shaders optional.
                let models = Models::from_models_legacy(
                    &mxmd.models,
                    &mxmd.materials,
                    None,
                    shader_database,
                    &texture_indices,
                );

                let skeleton = create_skeleton(skel.as_ref(), None);

                Ok(Self {
                    models,
                    buffers,
                    image_textures,
                    skeleton,
                })
            }
        }
    }

    /// Load models from legacy parsed file data for Xenoblade X.
    pub fn from_mxmd_model_legacy(
        mxmd: &MxmdLegacy,
        casmt: Option<Vec<u8>>,
        hkt: Option<&Hkt>,
        shader_database: Option<&ShaderDatabase>,
    ) -> Result<Self, LoadModelLegacyError> {
        let skeleton = hkt.map(Skeleton::from_legacy_skeleton);

        let buffers =
            ModelBuffers::from_vertex_data_legacy(&mxmd.vertex, &mxmd.models, Endian::Big)
                .map_err(LoadModelLegacyError::VertexData)?;

        let (texture_indices, image_textures) = load_textures_legacy(mxmd, casmt)?;

        let models = Models::from_models_legacy(
            &mxmd.models,
            &mxmd.materials,
            Some(&mxmd.shaders),
            shader_database,
            &texture_indices,
        );

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
    let mut reader = Cursor::new(std::fs::read(wimdo_path).map_err(|e| {
        LoadModelError::Wimdo(ReadFileError {
            path: wimdo_path.to_owned(),
            source: e.into(),
        })
    })?);
    let wimdo: Wimdo = reader.read_le().map_err(|e| {
        LoadModelError::Wimdo(ReadFileError {
            path: wimdo_path.to_owned(),
            source: e,
        })
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
                r.map_err(|e| {
                    LoadModelError::Wimdo(ReadFileError {
                        path: wimdo_path.to_owned(),
                        source: e,
                    })
                })
            }),
    }
}

// Use Cow::Borrowed to avoid copying data embedded in the mxmd.
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug)]
pub struct StreamingData<'a> {
    pub vertex: VertexData<'a>,
    pub spch: Cow<'a, xc3_lib::spch::Spch>,
    pub textures: ExtractedTextures,
    pub texture_indices: Option<Vec<u16>>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Clone)]
pub enum VertexData<'a> {
    Modern(Cow<'a, xc3_lib::vertex::VertexData>),
    Legacy(Cow<'a, xc3_lib::mxmd::legacy::VertexData>),
}

impl<'a> StreamingData<'a> {
    pub fn from_files(
        mxmd: &'a Mxmd,
        wismt_path: &Path,
        is_pc: bool,
        chr_folder: Option<&Path>,
    ) -> Result<StreamingData<'a>, LoadModelError> {
        // Handle the different ways to store the streaming data.
        match &mxmd.inner {
            xc3_lib::mxmd::MxmdInner::V112(mxmd) => {
                streaming_data(mxmd, wismt_path, chr_folder, is_pc)
            }
            xc3_lib::mxmd::MxmdInner::V40(mxmd) => {
                streaming_data_v40(mxmd, wismt_path, chr_folder, is_pc)
            }
        }
    }
}

fn streaming_data_v40<'a>(
    mxmd: &'a xc3_lib::mxmd::legacy2::MxmdV40,
    wismt_path: &Path,
    chr_folder: Option<&Path>,
    is_pc: bool,
) -> Result<StreamingData<'a>, LoadModelError> {
    mxmd.streaming
        .as_ref()
        .map(|streaming| match &streaming.inner {
            xc3_lib::msrd::StreamingInner::StreamingLegacy(_legacy) => {
                // TODO: Does xcx de use lagacy stream data?
                todo!()
            }
            xc3_lib::msrd::StreamingInner::Streaming(_) => {
                let msrd = Msrd::from_file(wismt_path).map_err(LoadModelError::Wismt)?;
                if is_pc {
                    // TODO: Does xcx de have pc files?
                    todo!()
                } else {
                    let (vertex, spco, textures) = msrd.extract_files_legacy(chr_folder)?;
                    // TODO: avoid index panic.
                    let spch = spco.items[0].spch.clone();

                    Ok(StreamingData {
                        vertex: VertexData::Legacy(Cow::Owned(vertex)),
                        spch: Cow::Owned(spch),
                        textures: ExtractedTextures::Switch(textures),
                        texture_indices: None,
                    })
                }
            }
        })
        .unwrap_or_else(|| {
            let textures = match &mxmd.packed_textures {
                Some(textures) => textures
                    .textures
                    .iter()
                    .map(|t| {
                        Ok(ExtractedTexture {
                            name: t.name.clone(),
                            usage: t.usage,
                            low: Mibl::from_bytes(&t.mibl_data)
                                .map_err(|e| LoadModelError::WimdoPackedTexture { source: e })?,
                            high: None,
                        })
                    })
                    .collect::<Result<Vec<_>, LoadModelError>>()?,
                None => Vec::new(),
            };

            Ok(StreamingData {
                vertex: VertexData::Legacy(Cow::Borrowed(
                    mxmd.vertex_data
                        .as_ref()
                        .ok_or(LoadModelError::MissingMxmdVertexData)?,
                )),
                spch: Cow::Borrowed(
                    mxmd.shaders
                        .as_ref()
                        .and_then(|s| s.items.first().map(|i| &i.spch))
                        .ok_or(LoadModelError::MissingMxmdShaderData)?,
                ),
                textures: ExtractedTextures::Switch(textures),
                texture_indices: None,
            })
        })
}

fn streaming_data<'a>(
    mxmd: &'a xc3_lib::mxmd::MxmdV112,
    wismt_path: &Path,
    chr_folder: Option<&Path>,
    is_pc: bool,
) -> Result<StreamingData<'a>, LoadModelError> {
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

                let (texture_indices, textures) = legacy.extract_textures(&data)?;

                // TODO: Error on missing vertex data?
                Ok(StreamingData {
                    vertex: VertexData::Modern(Cow::Borrowed(
                        mxmd.vertex_data
                            .as_ref()
                            .ok_or(LoadModelError::MissingMxmdVertexData)?,
                    )),
                    spch: Cow::Borrowed(
                        mxmd.spch
                            .as_ref()
                            .ok_or(LoadModelError::MissingMxmdShaderData)?,
                    ),
                    textures: ExtractedTextures::Switch(textures),
                    texture_indices: Some(texture_indices),
                })
            }
            xc3_lib::msrd::StreamingInner::Streaming(_) => {
                let msrd = Msrd::from_file(wismt_path).map_err(LoadModelError::Wismt)?;
                if is_pc {
                    let (vertex, spch, textures) = msrd.extract_files_pc()?;

                    Ok(StreamingData {
                        vertex: VertexData::Modern(Cow::Owned(vertex)),
                        spch: Cow::Owned(spch),
                        textures: ExtractedTextures::Pc(textures),
                        texture_indices: None,
                    })
                } else {
                    let (vertex, spch, textures) = msrd.extract_files(chr_folder)?;

                    Ok(StreamingData {
                        vertex: VertexData::Modern(Cow::Owned(vertex)),
                        spch: Cow::Owned(spch),
                        textures: ExtractedTextures::Switch(textures),
                        texture_indices: None,
                    })
                }
            }
        })
        .unwrap_or_else(|| {
            let textures = match &mxmd.packed_textures {
                Some(textures) => textures
                    .textures
                    .iter()
                    .map(|t| {
                        Ok(ExtractedTexture {
                            name: t.name.clone(),
                            usage: t.usage,
                            low: Mibl::from_bytes(&t.mibl_data)
                                .map_err(|e| LoadModelError::WimdoPackedTexture { source: e })?,
                            high: None,
                        })
                    })
                    .collect::<Result<Vec<_>, LoadModelError>>()?,
                None => Vec::new(),
            };

            Ok(StreamingData {
                vertex: VertexData::Modern(Cow::Borrowed(
                    mxmd.vertex_data
                        .as_ref()
                        .ok_or(LoadModelError::MissingMxmdVertexData)?,
                )),
                spch: Cow::Borrowed(
                    mxmd.spch
                        .as_ref()
                        .ok_or(LoadModelError::MissingMxmdShaderData)?,
                ),
                textures: ExtractedTextures::Switch(textures),
                texture_indices: None,
            })
        })
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
#[tracing::instrument(skip_all)]
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
    skel: Option<&Skel>,
    skinning: Option<&xc3_lib::mxmd::Skinning>,
) -> Option<Skeleton> {
    // Merge both skeletons since the bone lists may be different.
    // TODO: Create a skeleton even without the chr?
    Some(Skeleton::from_skeleton(&skel?.skeleton, skinning))
}

// TODO: Move this to xc3_shader?
fn model_name(model_path: &Path) -> String {
    model_path
        .file_stem()
        .unwrap_or_default()
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
fn arbitrary_quat(u: &mut arbitrary::Unstructured) -> arbitrary::Result<glam::Quat> {
    let array: [f32; 4] = u.arbitrary()?;
    Ok(glam::Quat::from_array(array))
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

#[cfg(feature = "arbitrary")]
fn arbitrary_smolstr(u: &mut arbitrary::Unstructured) -> arbitrary::Result<smol_str::SmolStr> {
    let text: String = u.arbitrary()?;
    Ok(text.into())
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

fn get_bytes(bytes: &[u8], offset: u32, size: Option<u32>) -> std::io::Result<&[u8]> {
    let start = offset as usize;

    match size {
        Some(size) => {
            let end = start + size as usize;
            bytes.get(start..end).ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    format!(
                        "byte range {start}..{end} out of range for length {}",
                        bytes.len()
                    ),
                )
            })
        }
        None => bytes.get(start..).ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                format!(
                    "byte offset {start} out of range for length {}",
                    bytes.len()
                ),
            )
        }),
    }
}
