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

use std::{borrow::Cow, path::Path};

use animation::Animation;
use glam::{Mat4, Vec3, Vec4};
use image_dds::ddsfile::Dds;
use log::{error, warn};
use material::create_materials;
use shader_database::ShaderDatabase;
use skinning::SkinWeights;
use texture::load_textures;
use vertex::{read_index_buffers, read_vertex_buffers, AttributeData};
use xc3_lib::{
    apmd::Apmd,
    mibl::Mibl,
    msrd::{streaming::ExtractedTexture, Msrd},
    mxmd::{Materials, Mxmd},
    sar1::Sar1,
    vertex::{VertexData, WeightLod},
};

pub use map::load_map;
pub use material::{
    ChannelAssignment, GBufferAssignment, GBufferAssignments, Material, MaterialParameters,
    Texture, TextureAlphaTest,
};
pub use sampler::{AddressMode, FilterMode, Sampler};
pub use skeleton::{Bone, Skeleton};
pub use texture::{ImageFormat, ImageTexture, ViewDimension};
pub use xc3_lib::mxmd::{BlendState, ShaderUnkType, StateFlags};

pub mod animation;
pub mod gltf;
mod map;
mod material;
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

/// See [VertexData].
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
/// let database = ShaderDatabase::from_file("xc1.json");
/// let root = load_model("xeno1/chr/pc/pc010101.wimdo", Some(&database));
///
/// // Pyra
/// let database = ShaderDatabase::from_file("xc2.json");
/// let root = load_model("xeno2/model/bl/bl000101.wimdo", Some(&database));
///
/// // Mio military uniform
/// let database = ShaderDatabase::from_file("xc3.json");
/// let root = load_model("xeno3/chr/ch/ch01027000.wimdo", Some(&database));
/// # Ok(())
/// # }
/// ```
///
/// For models split into multiple files, simply combine the roots.
/// ```rust no_run
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # use xc3_model::{load_model, shader_database::ShaderDatabase};
/// let database = ShaderDatabase::from_file("xc1.json");
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
/// let roots: Vec<_> = paths
///     .iter()
///     .map(|path| xc3_model::load_model(path, Some(&database)))
///     .collect();
/// # Ok(())
/// # }
/// ```
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

    std::fs::write("mxmd.txt", format!("{:#?}", mxmd)).unwrap();

    // Desktop PC models aren't used in game but are straightforward to support.
    let is_pc = wimdo_path.extension().and_then(|e| e.to_str()) == Some("pcmdo");
    let wismt_path = if is_pc {
        wimdo_path.with_extension("pcsmt")
    } else {
        wimdo_path.with_extension("wismt")
    };
    let streaming_data = load_streaming_data(&mxmd, &wismt_path, is_pc);

    // TODO: Avoid unwrap.
    // "chr/en/file.wismt" -> "chr/tex/nx/m"
    // TODO: Don't assume model_path is in the chr/ch or chr/en folders.
    let chr_folder = wimdo_path.parent().unwrap().parent().unwrap();
    let m_tex_folder = chr_folder.join("tex").join("nx").join("m");
    let h_tex_folder = chr_folder.join("tex").join("nx").join("h");

    let image_textures = load_textures(&streaming_data.textures, &m_tex_folder, &h_tex_folder);

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

    let (vertex_buffers, weights) =
        read_vertex_buffers(&streaming_data.vertex, mxmd.models.skinning.as_ref());
    let index_buffers = read_index_buffers(&streaming_data.vertex);

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

// Use Cow::Borrowed to avoid copying data embedded in the mxmd.
struct StreamingData<'a> {
    vertex: Cow<'a, VertexData>,
    textures: ExtractedTextures,
}

enum ExtractedTextures {
    Switch(Vec<ExtractedTexture<Mibl>>),
    Pc(Vec<ExtractedTexture<Dds>>),
}

fn load_streaming_data<'a>(mxmd: &'a Mxmd, wismt_path: &Path, is_pc: bool) -> StreamingData<'a> {
    // TODO: Avoid unwrap.
    // Handle the different ways to store the streaming data.
    mxmd.streaming
        .as_ref()
        .map(|streaming| match &streaming.inner {
            xc3_lib::msrd::StreamingInner::StreamingLegacy(legacy) => {
                let data = std::fs::read(wismt_path).unwrap();

                StreamingData {
                    vertex: Cow::Borrowed(mxmd.vertex_data.as_ref().unwrap()),
                    textures: ExtractedTextures::Switch(legacy.extract_textures(&data)),
                }
            }
            xc3_lib::msrd::StreamingInner::Streaming(_) => {
                let msrd = Msrd::from_file(wismt_path).unwrap();
                if is_pc {
                    let (vertex, _, textures) = msrd.extract_files_pc().unwrap();

                    StreamingData {
                        vertex: Cow::Owned(vertex),
                        textures: ExtractedTextures::Pc(textures),
                    }
                } else {
                    let (vertex, _, textures) = msrd.extract_files().unwrap();

                    StreamingData {
                        vertex: Cow::Owned(vertex),
                        textures: ExtractedTextures::Switch(textures),
                    }
                }
            }
        })
        .unwrap_or_else(|| StreamingData {
            vertex: Cow::Borrowed(mxmd.vertex_data.as_ref().unwrap()),
            textures: ExtractedTextures::Switch(
                mxmd.packed_textures
                    .as_ref()
                    .unwrap()
                    .textures
                    .iter()
                    .map(|t| ExtractedTexture {
                        name: t.name.clone(),
                        usage: t.usage,
                        low: Mibl::from_bytes(&t.mibl_data).unwrap(),
                        high: None,
                    })
                    .collect(),
            ),
        })
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

// TODO: Move this to xc3_shader?
fn model_name(model_path: &Path) -> String {
    model_path
        .with_extension("")
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string()
}

impl Weights {
    pub fn weight_group_index(
        &self,
        skin_flags: u32,
        lod: u16,
        unk_type: ShaderUnkType,
    ) -> Option<usize> {
        weight_group_index(&self.weight_lods, skin_flags, lod, unk_type)
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

fn weight_group_index(
    weight_lods: &[WeightLod],
    skin_flags: u32,
    lod: u16,
    unk_type: ShaderUnkType,
) -> Option<usize> {
    // TODO: Is this the correct flags check?
    // TODO: This doesn't work for other unk type or lod?
    if (skin_flags & 0x1) == 0 && skin_flags != 16400 {
        // TODO: Is this actually some sort of flags?
        let lod_index = lod.saturating_sub(1) as usize;
        let weight_lod = weight_lods.get(lod_index)?;

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

#[cfg(test)]
#[macro_export]
macro_rules! assert_hex_eq {
    ($a:expr, $b:expr) => {
        pretty_assertions::assert_str_eq!(hex::encode($a), hex::encode($b))
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weight_group_index_pc082402_fiora() {
        // xeno1/chr/pc/pc082402.wimdo
        let weight_lods = [WeightLod {
            group_indices_plus_one: [1, 0, 0, 2, 0, 0, 0, 0, 0],
        }];
        assert_eq!(
            None,
            weight_group_index(&weight_lods, 16385, 0, ShaderUnkType::Unk0)
        );
        assert_eq!(
            Some(1),
            weight_group_index(&weight_lods, 16392, 0, ShaderUnkType::Unk7)
        );
    }

    #[test]
    fn weight_group_index_bl301501_ursula() {
        // xeno2/model/bl/bl301501.wimdo
        let weight_lods = [
            WeightLod {
                group_indices_plus_one: [1, 2, 0, 0, 0, 0, 0, 0, 0],
            },
            WeightLod {
                group_indices_plus_one: [3, 4, 0, 0, 0, 0, 0, 0, 0],
            },
            WeightLod {
                group_indices_plus_one: [5, 6, 0, 0, 0, 0, 0, 0, 0],
            },
        ];
        assert_eq!(
            None,
            weight_group_index(&weight_lods, 16385, 1, ShaderUnkType::Unk0)
        );
        assert_eq!(
            None,
            weight_group_index(&weight_lods, 1, 1, ShaderUnkType::Unk0)
        );
        assert_eq!(
            Some(3),
            weight_group_index(&weight_lods, 2, 2, ShaderUnkType::Unk1)
        );
        assert_eq!(
            Some(5),
            weight_group_index(&weight_lods, 2, 3, ShaderUnkType::Unk1)
        );
    }

    #[test]
    fn weight_group_index_ch01011023_noah() {
        // xeno3/chr/ch/ch01011023.wimdo
        let weight_lods = [
            WeightLod {
                group_indices_plus_one: [4, 0, 0, 3, 0, 1, 2, 0, 0],
            },
            WeightLod {
                group_indices_plus_one: [7, 0, 0, 6, 0, 5, 0, 0, 0],
            },
            WeightLod {
                group_indices_plus_one: [10, 0, 0, 9, 0, 8, 0, 0, 0],
            },
        ];
        assert_eq!(
            Some(3),
            weight_group_index(&weight_lods, 64, 1, ShaderUnkType::Unk0)
        );
        assert_eq!(
            None,
            weight_group_index(&weight_lods, 16400, 2, ShaderUnkType::Unk0)
        );
    }
}
