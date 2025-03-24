use std::{borrow::Cow, path::Path};

use binrw::Endian;
use glam::Mat4;
use log::error;
use xc3_lib::{
    bc::skel::Skel,
    error::ReadFileError,
    hkt::Hkt,
    msrd::Msrd,
    mxmd::{legacy::MxmdLegacy, AlphaTable, Materials, MeshRenderFlags2, MeshRenderPass, Mxmd},
};

use crate::{
    create_materials, create_materials_samplers_legacy,
    error::{LoadModelError, LoadModelLegacyError},
    material::GetProgramHash,
    shader_database::ShaderDatabase,
    skinning::create_skinning,
    texture::{load_packed_textures, load_textures, load_textures_legacy},
    vertex::ModelBuffers,
    ExtractedTextures, LodData, LodGroup, LodItem, Mesh, Model, ModelRoot, Models, Sampler,
    Skeleton,
};

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
                // TODO: Does xcx de use legacy stream data?
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
            let textures = load_packed_textures(mxmd.packed_textures.as_ref())
                .map_err(|e| LoadModelError::WimdoPackedTexture { source: e })?;

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
            let textures = load_packed_textures(mxmd.packed_textures.as_ref())
                .map_err(|e| LoadModelError::WimdoPackedTexture { source: e })?;

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

impl ModelRoot {
    /// Load models from parsed file data for Xenoblade 1 DE, Xenoblade 2, Xenoblade 3, or Xenoblade X DE.
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
                // TODO: Can xcx de texture indices be remapped like with xcx?
                let texture_indices: Vec<_> = (0..image_textures.len() as u16).collect();

                let models = Models::from_models_legacy(
                    &mxmd.models,
                    &mxmd.materials,
                    Some(streaming_data.spch.as_ref()),
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

    pub fn from_models_legacy<S>(
        models: &xc3_lib::mxmd::legacy::Models,
        materials: &xc3_lib::mxmd::legacy::Materials,
        shaders: Option<&S>,
        shader_database: Option<&ShaderDatabase>,
        texture_indices: &[u16],
    ) -> Self
    where
        S: GetProgramHash,
    {
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

pub fn lod_data(data: &xc3_lib::mxmd::LodData) -> LodData {
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

pub fn create_samplers(materials: &Materials) -> Vec<Sampler> {
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
