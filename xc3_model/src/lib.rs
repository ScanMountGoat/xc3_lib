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
    io::Cursor,
    path::{Path, PathBuf},
};

use animation::Animation;
use binrw::{BinRead, BinReaderExt};
use glam::{Mat4, Vec3};
use log::error;
use material::create_materials;
use shader_database::ShaderDatabase;
use skinning::SkinWeights;
use texture::load_textures;
use thiserror::Error;
use vertex::ModelBuffers;
use xc3_lib::{
    apmd::Apmd,
    bc::Bc,
    error::DecompressStreamError,
    mibl::Mibl,
    msrd::{
        streaming::{chr_tex_nx_folder, ExtractedTexture},
        Msrd,
    },
    mxmd::{legacy::MxmdLegacy, Materials, Mxmd},
    sar1::Sar1,
    vertex::WeightLod,
    xbc1::MaybeXbc1,
    ReadFileError,
};

pub use map::{load_map, LoadMapError};
pub use material::{
    ChannelAssignment, Material, MaterialParameters, OutputAssignment, OutputAssignments, Texture,
    TextureAlphaTest,
};
pub use sampler::{AddressMode, FilterMode, Sampler};
pub use skeleton::{Bone, Skeleton};
pub use texture::{ExtractedTextures, ImageFormat, ImageTexture, ViewDimension};
pub use xc3_lib::mxmd::{
    BlendMode, CullMode, DepthFunc, RenderPassType, StateFlags, StencilMode, StencilValue,
    TextureUsage,
};

pub mod animation;

#[cfg(feature = "gltf")]
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
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Clone)]
pub struct ModelRoot {
    pub groups: Vec<ModelGroup>,
    /// The textures selected by each [Material].
    /// This includes all packed and embedded textures after
    /// combining all mip levels.
    pub image_textures: Vec<ImageTexture>,

    // TODO: Do we even need to store the skinning if the weights already have the skinning bone name list?
    pub skeleton: Option<Skeleton>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Clone)]
pub struct ModelGroup {
    pub models: Vec<Models>,
    /// The vertex data selected by each [Model].
    pub buffers: Vec<ModelBuffers>,
}

// TODO: come up with a better name?
/// See [Weights](xc3_lib::vertex::Weights).
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Clone)]
pub struct Weights {
    // TODO: This is tied to the Models?
    // TODO: have each Models have its own reindexed set of indices based on skeleton names?
    pub skin_weights: SkinWeights,

    // TODO: Is this the best way to represent this information?
    pub weight_groups: Vec<xc3_lib::vertex::WeightGroup>,
    pub weight_lods: Vec<xc3_lib::vertex::WeightLod>,
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

    // TODO: Better way of organizing this data?
    // TODO: How to handle the indices being off by 1?
    // TODO: when is this None?
    // TODO: Create a type for this constructed from Models?
    pub base_lod_indices: Option<Vec<u16>>,

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
    pub vertex_buffer_index: usize,
    pub index_buffer_index: usize,
    pub material_index: usize,
    pub lod: u16,
    pub flags1: u32,
    pub skin_flags: u32,
}

impl Models {
    pub fn from_models(
        models: &xc3_lib::mxmd::Models,
        materials: &xc3_lib::mxmd::Materials,
        spch: Option<&shader_database::Spch>,
    ) -> Models {
        Models {
            models: models
                .models
                .iter()
                .map(|model| Model::from_model(model, vec![Mat4::IDENTITY], 0))
                .collect(),
            materials: create_materials(materials, spch),
            samplers: create_samplers(materials),
            base_lod_indices: models
                .lod_data
                .as_ref()
                .map(|data| data.groups.iter().map(|i| i.base_lod_index).collect()),
            min_xyz: models.min_xyz.into(),
            max_xyz: models.max_xyz.into(),
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
                flags1: mesh.flags1,
                skin_flags: mesh.skin_flags,
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
                vertex_buffer_index: mesh.vertex_buffer_index as usize,
                index_buffer_index: mesh.index_buffer_index as usize,
                material_index: mesh.material_index as usize,
                lod: 0,
                flags1: mesh.flags,
                skin_flags: mesh.skin_flags,
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
    let spch = shader_database.and_then(|database| database.files.get(&model_name));

    // TODO: Does every wimdo have a chr file?
    // TODO: Does something control the chr name used?
    // TODO: This won't load the base skeleton chr for xc3.
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

    ModelRoot::from_mxmd_model(&mxmd, chr, &streaming_data, spch)
}

// TODO: docs and avoid unwrap.
pub fn load_model_legacy<P: AsRef<Path>>(camdo_path: P) -> ModelRoot {
    // TODO: texture streaming data.
    let mut reader = Cursor::new(std::fs::read(camdo_path).unwrap());
    let mxmd: MxmdLegacy = reader.read_be().unwrap();
    ModelRoot::from_mxmd_model_legacy(&mxmd).unwrap()
}

// TODO: fuzz test this?
impl ModelRoot {
    /// Load models from parsed file data.
    pub fn from_mxmd_model(
        mxmd: &Mxmd,
        chr: Option<Sar1>,
        streaming_data: &StreamingData<'_>,
        spch: Option<&shader_database::Spch>,
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

        let models = Models::from_models(&mxmd.models, &mxmd.materials, spch);

        let image_textures = load_textures(&streaming_data.textures)?;

        // TODO: Find a way to specify at the type level that this has only one element?
        Ok(Self {
            groups: vec![ModelGroup {
                models: vec![models],
                buffers: vec![buffers],
            }],
            image_textures,
            skeleton,
        })
    }

    /// Load models from legacy parsed file data for Xenoblade Chronicles X.
    pub fn from_mxmd_model_legacy(mxmd: &MxmdLegacy) -> Result<Self, LoadModelError> {
        // TODO: How to make this work?
        // TODO: separate type for legacy streaming data?
        // TODO: dedicated module for loading stream data?

        let models = Models {
            models: mxmd
                .models
                .models
                .iter()
                .map(Model::from_model_legacy)
                .collect(),
            materials: mxmd
                .materials
                .materials
                .iter()
                .map(|m| Material {
                    name: m.name.clone(),
                    flags: StateFlags {
                        depth_write_mode: 0,
                        blend_mode: BlendMode::Disabled,
                        cull_mode: CullMode::Back,
                        unk4: 0,
                        stencil_value: StencilValue::Unk0,
                        stencil_mode: StencilMode::Unk0,
                        depth_func: DepthFunc::LessEqual,
                        color_write_mode: 0,
                    },
                    textures: Vec::new(),
                    alpha_test: None,
                    shader: None,
                    pass_type: RenderPassType::Unk0,
                    parameters: MaterialParameters {
                        mat_color: [1.0; 4],
                        alpha_test_ref: 0.0,
                        tex_matrix: None,
                        work_float4: None,
                        work_color: None,
                    },
                })
                .collect(),
            samplers: Vec::new(),
            base_lod_indices: None,
            max_xyz: mxmd.models.max_xyz.into(),
            min_xyz: mxmd.models.min_xyz.into(),
        };

        let buffers = ModelBuffers::from_vertex_data_legacy(&mxmd.vertex)
            .map_err(LoadModelError::VertexData)?;

        // TODO: Find a way to specify at the type level that this has only one element?
        Ok(Self {
            groups: vec![ModelGroup {
                models: vec![models],
                buffers: vec![buffers],
            }],
            image_textures: Vec::new(),
            skeleton: None,
        })
    }

    // TODO: module for conversions?
    // TODO: Not possible to make files compatible with all game versions?
    // TODO: Will it be possible to do full imports in the future?
    // TODO: Include chr to support skeleton edits?
    // TODO: How to properly test this?
    /// Apply the values from this model onto the original `mxmd` and `msrd`.
    ///
    /// Some of the original values will be retained due to exporting limitations.
    /// For best results, use the [Mxmd] and [Msrd] used to initialize this model.
    ///
    /// If no edits were made to this model, the resulting files will attempt
    /// to recreate the originals used to initialize this model as closely as possible.
    pub fn to_mxmd_model(&self, mxmd: &Mxmd, msrd: &Msrd) -> (Mxmd, Msrd) {
        // TODO: Does this need to even extract vertex/textures?
        let (_, spch, _) = msrd.extract_files(None).unwrap();

        // TODO: Assume the same ordering instead of recreating from scratch?
        // TODO: Create a method that converts ImageTexture to ExtractedTexture?
        // TODO: What to use for the low texture?
        let textures: Vec<_> = self
            .image_textures
            .iter()
            .map(|image| ExtractedTexture {
                name: image.name.clone().unwrap(),
                usage: image.usage.unwrap(),
                low: image.to_mibl().unwrap(),
                high: None,
            })
            .collect();

        // TODO: Create a separate root type that enforces this structure?
        let new_vertex = self.groups[0].buffers[0].to_vertex_data().unwrap();

        let mut new_mxmd = mxmd.clone();

        // TODO: Rebuild materials.
        // TODO: How many of these mesh fields can use a default value?
        let models = &self.groups[0].models[0];
        new_mxmd.models.models = models
            .models
            .iter()
            .map(|model| xc3_lib::mxmd::Model {
                meshes: model
                    .meshes
                    .iter()
                    .map(|m| xc3_lib::mxmd::Mesh {
                        flags1: m.flags1,
                        skin_flags: m.skin_flags,
                        vertex_buffer_index: m.vertex_buffer_index as u16,
                        index_buffer_index: m.index_buffer_index as u16,
                        unk_index: 0,
                        material_index: m.material_index as u16,
                        unk2: 0,
                        unk3: 0,
                        ext_mesh_index: 0,
                        unk4: 0,
                        unk5: 0,
                        lod: m.lod,
                        alpha_table_index: 0,
                        unk6: 0,
                        unk7: 0,
                        unk8: 0,
                        unk9: 0,
                    })
                    .collect(),
                unk1: 0,
                max_xyz: model.max_xyz.to_array(),
                min_xyz: model.min_xyz.to_array(),
                bounding_radius: model.bounding_radius,
                unks1: [0; 3],
                unk2: mxmd.models.models[0].unk2,
                unks: [0; 3],
            })
            .collect();
        new_mxmd.models.min_xyz = new_mxmd
            .models
            .models
            .iter()
            .map(|m| m.min_xyz)
            .reduce(|[ax, ay, az], [bx, by, bz]| [ax.min(bx), ay.min(by), az.min(bz)])
            .unwrap_or_default();
        new_mxmd.models.max_xyz = new_mxmd
            .models
            .models
            .iter()
            .map(|m| m.max_xyz)
            .reduce(|[ax, ay, az], [bx, by, bz]| [ax.max(bx), ay.max(by), az.max(bz)])
            .unwrap_or_default();

        let use_chr_textures = mxmd
            .streaming
            .as_ref()
            .map(|s| s.inner.has_chr_textures())
            .unwrap_or_default();

        let new_msrd =
            Msrd::from_extracted_files(&new_vertex, &spch, &textures, use_chr_textures).unwrap();
        new_mxmd.streaming = Some(new_msrd.streaming.clone());

        (new_mxmd, new_msrd)
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
    Bc(Bc),
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
                    let bc = entry.read_data::<xc3_lib::bc::Bc>()?;
                    add_bc_animations(&mut animations, bc);
                }
            }
            MaybeXbc1::Xbc1(xbc1) => {
                let sar1: Sar1 = xbc1.extract()?;
                for entry in &sar1.entries {
                    let bc = entry.read_data::<xc3_lib::bc::Bc>()?;
                    add_bc_animations(&mut animations, bc);
                }
            }
        },
        AnimFile::Bc(bc) => {
            add_bc_animations(&mut animations, bc);
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

    Some(Skeleton::from_skel(&skel.skeleton, skinning?))
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
    /// Get the assigned weight group based on [Mesh] and [Material] parameters.
    pub fn weight_group(
        &self,
        skin_flags: u32,
        lod: u16,
        unk_type: xc3_lib::mxmd::RenderPassType,
    ) -> Option<&xc3_lib::vertex::WeightGroup> {
        let group_index = weight_group_index(&self.weight_lods, skin_flags, lod, unk_type);
        self.weight_groups.get(group_index)
    }

    /// The offset to add to [vertex::AttributeData::WeightIndex]
    /// when selecting [vertex::AttributeData::BoneIndices] and [vertex::AttributeData::SkinWeights].
    ///
    /// Preskinned matrices starting from the input index are written to the output index.
    /// This means the final index value is `weight_index = nWgtIndex + input_start - output_start`.
    /// Equivalent bone indices and weights are simply `indices[weight_index]` and `weights[weight_index]`.
    /// A mesh has only one assigned weight group, so this is sufficient to recreate the in game behavior
    /// without any complex precomputation of skinning matrices.
    pub fn weights_start_index(
        &self,
        skin_flags: u32,
        lod: u16,
        unk_type: xc3_lib::mxmd::RenderPassType,
    ) -> usize {
        // TODO: Error if none?
        self.weight_group(skin_flags, lod, unk_type)
            .map(|group| (group.input_start_index - group.output_start_index) as usize)
            .unwrap_or_default()
    }
}

fn weight_group_index(
    weight_lods: &[WeightLod],
    skin_flags: u32,
    lod: u16,
    unk_type: RenderPassType,
) -> usize {
    // TODO: Should this check skin flags?
    // TODO: Is lod actually some sort of flags?
    // TODO: Return none if skin_flags == 64?
    let lod_index = (lod & 0xff).saturating_sub(1) as usize;
    // TODO: More mesh lods than weight lods for models with multiple lod groups?
    let weight_lod = &weight_lods[lod_index % weight_lods.len()];

    let pass_index = weight_pass_index(unk_type, skin_flags);
    weight_lod.group_indices_plus_one[pass_index].saturating_sub(1) as usize
}

fn weight_pass_index(unk_type: RenderPassType, skin_flags: u32) -> usize {
    // TODO: skin_flags & 0xF has a max value of group_indices.len() - 1?
    // TODO: bit mask?
    // TODO: Test possible values by checking mesh flags and pass types in xc3_test?
    // TODO: Compare this with non zero entries in group indices?
    // TODO: Assume all weight groups are assigned to at least one mesh?
    // TODO: get unique parameters for this function for each wimdo?

    // TODO: Find a way to determine the group selected in game?
    // TODO: Test unique parameter combination using a modified weight group?
    // TODO: Detect if vertices move in game?
    let mut pass_index = match unk_type {
        RenderPassType::Unk0 => 0,
        RenderPassType::Unk1 => 1,
        RenderPassType::Unk6 => todo!(),
        RenderPassType::Unk7 => 3, // TODO: also 4?
        RenderPassType::Unk9 => todo!(),
    };
    if skin_flags == 64 {
        pass_index = 4;
    }
    pass_index
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
            0,
            weight_group_index(&weight_lods, 16385, 0, RenderPassType::Unk0)
        );
        assert_eq!(
            1,
            weight_group_index(&weight_lods, 16392, 0, RenderPassType::Unk7)
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
            0,
            weight_group_index(&weight_lods, 16385, 1, RenderPassType::Unk0)
        );
        assert_eq!(
            0,
            weight_group_index(&weight_lods, 1, 1, RenderPassType::Unk0)
        );
        assert_eq!(
            3,
            weight_group_index(&weight_lods, 2, 2, RenderPassType::Unk1)
        );
        assert_eq!(
            5,
            weight_group_index(&weight_lods, 2, 3, RenderPassType::Unk1)
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
            0,
            weight_group_index(&weight_lods, 64, 1, RenderPassType::Unk0)
        );
        assert_eq!(
            6,
            weight_group_index(&weight_lods, 16400, 2, RenderPassType::Unk0)
        );
    }
}
