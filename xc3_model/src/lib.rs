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

use std::{hash::Hash, io::Cursor, path::Path};

use animation::Animation;
use binrw::{BinRead, BinReaderExt};
use error::{LoadModelError, LoadModelLegacyError};
use glam::{Mat4, Vec3};
use indexmap::IndexMap;
use material::{create_materials, create_materials_samplers_legacy};
use model::import::{ModelFilesV40, ModelFilesV111, ModelFilesV112};
use shader_database::ShaderDatabase;
use skinning::Skinning;
use vertex::ModelBuffers;
use xc3_lib::{
    apmd::Apmd,
    bc::Bc,
    error::{DecompressStreamError, ReadFileError},
    hkt::Hkt,
    msrd::streaming::chr_folder,
    mxmd::{Mxmd, legacy::MxmdLegacy},
    sar1::Sar1,
    xbc1::MaybeXbc1,
};

pub use collision::load_collisions;
pub use map::load_map;
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
pub mod error;
mod map;
pub mod material;
pub mod model;
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
/// See [ModelsV112](xc3_lib::mxmd::ModelsV112).
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
    pub max_xyz: Vec3,

    /// The maximum XYZ coordinates of the bounding volume.
    pub min_xyz: Vec3,
}

/// See [ModelV112](xc3_lib::mxmd::ModelV112).
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Clone)]
pub struct Model {
    pub meshes: Vec<Mesh>,
    /// Each mesh has an instance for every transform in [instances](#structfield.instances).
    pub instances: Vec<Mat4>,
    /// The index of the [ModelBuffers] in [buffers](struct.ModelGroup.html#structfield.buffers).
    /// This will only be non zero for some map models.
    pub model_buffers_index: usize,

    pub max_xyz: Vec3,
    pub min_xyz: Vec3,
    pub bounding_radius: f32,
}

/// See [MeshV112](xc3_lib::mxmd::MeshV112).
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
    pub unk1: u32,
    pub items: Vec<LodItem>,
    pub groups: Vec<LodGroup>,
}

/// See [LodItem](xc3_lib::mxmd::LodItem).
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Clone)]
pub struct LodItem {
    pub unk2: f32,
    pub index: u8,
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
///
/// // Tatsu
/// let database = ShaderDatabase::from_file("xcxde.bin")?;
/// let root = load_model("xenox/chr/np/np009001.wimdo", Some(&database));
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
    let chr = chr_folder(wimdo_path);

    // Desktop PC models aren't used in game but are straightforward to support.
    let is_pc = wimdo_path.extension().and_then(|e| e.to_str()) == Some("pcmdo");
    let wismt_path = if is_pc {
        wimdo_path.with_extension("pcsmt")
    } else {
        wimdo_path.with_extension("wismt")
    };

    let model_name = model_name(wimdo_path);
    let skel = load_skel(wimdo_path, &model_name);

    match mxmd.inner {
        xc3_lib::mxmd::MxmdInner::V40(mxmd) => {
            let files = ModelFilesV40::from_files(&mxmd, &wismt_path, chr.as_deref())?;
            ModelRoot::from_mxmd_v40(&files, skel, shader_database)
        }
        xc3_lib::mxmd::MxmdInner::V111(mxmd) => {
            let files = ModelFilesV111::from_files(&mxmd, &wismt_path, chr.as_deref(), is_pc)?;
            ModelRoot::from_mxmd_v111(&files, skel, shader_database)
        }
        xc3_lib::mxmd::MxmdInner::V112(mxmd) => {
            let files = ModelFilesV112::from_files(&mxmd, &wismt_path, chr.as_deref(), is_pc)?;
            ModelRoot::from_mxmd_v112(&files, skel, shader_database)
        }
    }
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
            // TODO: Only try this for xcx de models (v40).
            // Xenoblade X DE uses a file for just the skeleton.
            Bc::from_file(wimdo.with_file_name(format!("{model_name}_rig.skl")))
                .ok()
                .or_else(|| {
                    let model_name = model_name.trim_end_matches("_us").trim_end_matches("_eu");
                    Bc::from_file(wimdo.with_file_name(format!("{model_name}_rig.skl"))).ok()
                })
                .and_then(|bc| match bc.data {
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
    // Most animations are in sar1 archives.
    // Xenoblade 1 DE compresses the sar1 archive.
    // Some animations are in standalone BC files.
    // Some Xenoblade X DE animations are in xcb1 archives.
    let anim_file = <MaybeXbc1<AnimFile>>::from_file(anim_path)?;

    let mut animations = Vec::new();
    match anim_file {
        MaybeXbc1::Uncompressed(anim) => add_anim_file(&mut animations, anim),
        MaybeXbc1::Xbc1(xbc1) => {
            if let Ok(anim) = xbc1.extract() {
                add_anim_file(&mut animations, anim);
            }
        }
    }

    Ok(animations)
}

#[derive(BinRead)]
enum AnimFile {
    Sar1(Sar1),
    Bc(Bc),
}

fn add_anim_file(animations: &mut Vec<Animation>, anim: AnimFile) {
    match anim {
        AnimFile::Sar1(sar1) => {
            for entry in &sar1.entries {
                if let Ok(bc) = entry.read_data() {
                    add_bc_animations(animations, bc);
                }
            }
        }
        AnimFile::Bc(bc) => {
            add_bc_animations(animations, bc);
        }
    }
}

fn add_bc_animations(animations: &mut Vec<Animation>, bc: Bc) {
    if let xc3_lib::bc::BcData::Anim(anim) = bc.data {
        let animation = Animation::from_anim(&anim);
        animations.push(animation);
    }
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
