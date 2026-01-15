use glam::Mat4;
use indexmap::IndexMap;
use log::warn;
use xc3_lib::{
    mibl::Mibl,
    msrd::{Msrd, streaming::ExtractedTexture},
    mxmd::{
        self, AlphaTable, LodData, LodGroup, LodItem, Mxmd, MxmdV112, SamplerFlags, TextureUsage,
        VertexAttribute, legacy2::MxmdV40,
    },
    vertex::{DataType, VertexData},
};

use crate::{
    ImageTexture, IndexMapExt, ModelRoot,
    error::CreateModelError,
    skinning::BoneConstraintType,
    vertex::{AttributeData, ModelBuffers},
};

/// Apply the values from this model onto the original `mxmd` and `msrd`.
///
/// Some of the original values will be retained due to exporting limitations.
/// For best results, use the [Mxmd] and [Msrd] used to initialize this model.
///
/// If no edits were made to this model, the resulting files will attempt
/// to recreate the original data used to initialize this model as closely as possible.
impl ModelRoot {
    pub fn to_mxmd_model(
        &self,
        mxmd: &Mxmd,
        msrd: &Msrd,
    ) -> Result<(Mxmd, Msrd), CreateModelError> {
        match &mxmd.inner {
            xc3_lib::mxmd::MxmdInner::V40(inner) => {
                // TODO: Does this need to even extract vertex/textures?
                let spco = msrd.extract_files_legacy(None)?.shader;
                let (mut new_mxmd, vertex, textures) = self.to_mxmd_v40_model_files(inner)?;

                let use_chr_textures = inner
                    .streaming
                    .as_ref()
                    .map(|s| s.inner.has_chr_textures())
                    .unwrap_or_default();
                let new_msrd =
                    Msrd::from_extracted_files_legacy(&vertex, &spco, &textures, use_chr_textures)
                        .unwrap();

                // The mxmd and msrd streaming header need to match exactly.
                new_mxmd.streaming = Some(new_msrd.streaming.clone());

                let new_mxmd = Mxmd {
                    version: mxmd.version,
                    inner: xc3_lib::mxmd::MxmdInner::V40(new_mxmd),
                };
                Ok((new_mxmd, new_msrd))
            }
            xc3_lib::mxmd::MxmdInner::V111(_) => Err(CreateModelError::UnsupportedVersion {
                version: mxmd.version,
            }),
            xc3_lib::mxmd::MxmdInner::V112(inner) => {
                // TODO: Does this need to even extract vertex/textures?
                let spch = msrd.extract_files(None)?.shader;
                let (mut new_mxmd, vertex, textures) = self.to_mxmd_v112_model_files(inner)?;

                let use_chr_textures = inner
                    .streaming
                    .as_ref()
                    .map(|s| s.inner.has_chr_textures())
                    .unwrap_or_default();
                let new_msrd =
                    Msrd::from_extracted_files(&vertex, &spch, &textures, use_chr_textures)
                        .unwrap();

                // The mxmd and msrd streaming header need to match exactly.
                new_mxmd.streaming = Some(new_msrd.streaming.clone());

                let new_mxmd = Mxmd {
                    version: mxmd.version,
                    inner: xc3_lib::mxmd::MxmdInner::V112(new_mxmd),
                };
                Ok((new_mxmd, new_msrd))
            }
        }
    }

    /// Similar to [Self::to_mxmd_model] but does not compress the new data or update streaming information.
    pub fn to_mxmd_v112_model_files(
        &self,
        mxmd: &MxmdV112,
    ) -> Result<
        (
            MxmdV112,
            VertexData,
            Vec<ExtractedTexture<Mibl, TextureUsage>>,
        ),
        CreateModelError,
    > {
        let textures: Vec<_> = self
            .image_textures
            .iter()
            .map(ImageTexture::to_extracted_texture)
            .collect();

        let mut buffers = self.buffers.clone();
        self.match_technique_attributes(&mut buffers, mxmd);
        let new_vertex = buffers.to_vertex_data().unwrap();

        let mut new_mxmd = mxmd.clone();

        // TODO: Rebuild materials.
        // TODO: How many of these mesh fields can use a default value?
        let mut alpha_table = IndexMap::new();

        let has_speff_materials = self
            .models
            .materials
            .iter()
            .any(|m| m.name.contains("speff"));
        let default_base_mesh_index = if has_speff_materials {
            -1
        } else {
            // xc1 and xc2 always set this to 0.
            0
        };

        new_mxmd.models.models = self
            .models
            .models
            .iter()
            .map(|model| xc3_lib::mxmd::ModelV112 {
                meshes: model
                    .meshes
                    .iter()
                    .map(|m| {
                        // Generate the mapping for unique ext mesh and lod values.
                        // An index of 0 represents no ext mesh.
                        // TODO: Why is the lod index set to 0 for some xc3 models?
                        let ext_index = m.ext_mesh_index.map(|i| i + 1).unwrap_or_default() as u16;
                        let alpha_table_index = alpha_table.entry_index((
                            ext_index,
                            m.lod_item_index.map(|i| i as u16 + 1).unwrap_or_default(),
                        )) as u16;

                        // TODO: How to set these indices in applications?
                        let base_mesh_index = m
                            .base_mesh_index
                            .map(|i| i as i32)
                            .unwrap_or(default_base_mesh_index);

                        xc3_lib::mxmd::MeshV112 {
                            flags1: m.flags1,
                            flags2: m.flags2,
                            vertex_buffer_index: m.vertex_buffer_index as u16,
                            index_buffer_index: m.index_buffer_index as u16,
                            index_buffer_index2: m.index_buffer_index2 as u16,
                            material_index: m.material_index as u16,
                            unk2: 0,
                            unk3: 0,
                            ext_mesh_index: m.ext_mesh_index.unwrap_or_default() as u16,
                            unk4: 0,
                            unk5: 0, // TODO: flags?
                            lod_item_index: m
                                .lod_item_index
                                .map(|i| i as u8 + 1)
                                .unwrap_or_default(),
                            unk_mesh_index2: 0, // TODO: how to set this?
                            alpha_table_index,
                            unk6: 0, // TODO: flags?
                            base_mesh_index,
                            unk8: 0,
                            unk9: 0,
                        }
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

        new_mxmd.models.alpha_table = Some(AlphaTable {
            items: alpha_table.keys().copied().collect(),
            unks: [0; 4],
        });

        new_mxmd.models.lod_data = self.models.lod_data.as_ref().map(|data| LodData {
            unk1: data.unk1,
            items: data
                .items
                .iter()
                .map(|i| LodItem {
                    unk1: [0; 4],
                    unk2: i.unk2,
                    unk3: 0,
                    index: i.index,
                    unk5: if i.unk2 == 0.0 { 2 } else { 1 },
                    unk6: 0,
                    unk7: [0; 2],
                })
                .collect(),
            groups: data
                .groups
                .iter()
                .map(|g| LodGroup {
                    base_lod_index: g.base_lod_index as u16,
                    lod_count: g.lod_count as u16,
                })
                .collect(),
            unks: [0; 4],
        });

        if new_vertex.vertex_morphs.is_none() {
            // Remove morph controllers if not needed to prevent crashes.
            // TODO: Why does setting this to None not work?
            if let Some(morph_controllers) = &mut new_mxmd.models.morph_controllers {
                // TODO: remove unused morphs and items from model_unk1?
                morph_controllers.controllers = Vec::new();
            }
        }

        if let Some(skinning) = &self.models.skinning
            && let Some(new_skinning) = &mut new_mxmd.models.skinning
        {
            apply_skinning(new_skinning, skinning);
        }

        // TODO: Is it possible to calculate transforms without a skeleton?
        if let Some(skeleton) = &self.skeleton
            && let Some(skinning) = &mut new_mxmd.models.skinning
        {
            let transforms = skeleton.model_space_transforms();

            // Rebuild all transforms to support adding new bones.
            // TODO: is it safe to assume all bones are part of the skeleton?
            skinning.inverse_bind_transforms = skinning
                    .bones
                    .iter()
                    .map(|bone| {
                        if let Some(index) = skeleton.bones.iter().position(|b| b.name == bone.name)
                        {
                            transforms[index].to_matrix().inverse().to_cols_array_2d()
                        } else {
                            warn!("Setting identity inverse bind transform for skinning bone {:?} not in skeleton.", &bone.name);
                            Mat4::IDENTITY.to_cols_array_2d()
                        }
                    })
                    .collect();
        }

        self.apply_materials_v112(&mut new_mxmd);

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

        // This should be updated later.
        new_mxmd.streaming = None;

        Ok((new_mxmd, new_vertex, textures))
    }

    fn apply_materials_v112(&self, mxmd: &mut MxmdV112) {
        // Recreate start indices and counts by assuming value ranges don't overlap.
        mxmd.materials.materials.clear();
        mxmd.materials.work_values.clear();
        mxmd.materials.shader_vars.clear();

        // Don't assume callbacks are used.
        let mut callbacks = mxmd.materials.callbacks.as_mut();
        if let Some(callbacks) = callbacks.as_mut() {
            callbacks.work_callbacks.clear();
            callbacks.material_indices = (0..self.models.materials.len() as u16).collect();
        }

        let mut fur_params = Vec::new();
        let mut fur_param_indices = Vec::new();

        // Recreate materials to avoid restrictions with referencing existing ones.
        for (i, m) in self.models.materials.iter().enumerate() {
            // TODO: Is it ok to potentially add a new buffer index here?
            let technique = xc3_lib::mxmd::MaterialTechnique {
                technique_index: m.technique_index as u32,
                pass_type: m.pass_type,
                material_buffer_index: i as u16,
                flags: 1,
            };

            // TODO: Also rebuild alpha textures in case we need to add more.
            let new_material = xc3_lib::mxmd::Material {
                name: m.name.clone(),
                flags: m.flags,
                render_flags: m.render_flags,
                color: m.color,
                alpha_test_ref: m.alpha_test_ref,
                textures: m
                    .textures
                    .iter()
                    .map(|t| {
                        // TODO: How should the second sampler be set?
                        xc3_lib::mxmd::Texture {
                            texture_index: t.image_texture_index as u16,
                            sampler_index: t.sampler_index as u16,
                            sampler_index2: t.sampler_index as u16,
                            unk3: 0,
                        }
                    })
                    .collect(),
                state_flags: m.state_flags,
                m_unks1_1: m.m_unks1_1,
                m_unks1_2: m.m_unks1_2,
                m_unks1_3: m.m_unks1_3,
                m_unks1_4: m.m_unks1_4,
                work_value_start_index: mxmd.materials.work_values.len() as u32,
                shader_var_start_index: mxmd.materials.shader_vars.len() as u32,
                shader_var_count: m.shader_vars.len() as u32,
                techniques: vec![technique],
                unk5: 0,
                callback_start_index: callbacks
                    .as_ref()
                    .map(|c| c.work_callbacks.len() as u16)
                    .unwrap_or_default(),
                callback_count: m.work_callbacks.len() as u16,
                m_unks2: [0, 0, m.m_unks2_2],
                alpha_test_texture_index: m
                    .alpha_test
                    .as_ref()
                    .and_then(|a| {
                        let alpha_image_index =
                            m.textures[a.texture_index].image_texture_index as u16;
                        // TODO: This won't work since textures can be used more than once.
                        mxmd.materials
                            .alpha_test_textures
                            .iter()
                            .position(|t| t.texture_index == alpha_image_index)
                    })
                    .unwrap_or_default() as u16,
                m_unk3: 0,
                gbuffer_flags: m.gbuffer_flags,
                m_unk4: [0; 6],
            };
            mxmd.materials.materials.push(new_material);

            mxmd.materials.work_values.extend_from_slice(&m.work_values);
            mxmd.materials.shader_vars.extend_from_slice(&m.shader_vars);
            if let Some(callbacks) = callbacks.as_mut() {
                callbacks
                    .work_callbacks
                    .extend_from_slice(&m.work_callbacks);
            }

            if let Some(params) = &m.fur_params {
                // Each material uses its own params in practice.
                fur_param_indices.push(fur_params.len() as u16);
                fur_params.push(params.clone());
            } else {
                fur_param_indices.push(0);
            }
        }

        mxmd.materials.fur_shells = if !fur_params.is_empty() {
            Some(xc3_lib::mxmd::FurShells {
                material_param_indices: fur_param_indices,
                params: fur_params,
                unk: [0; 4],
            })
        } else {
            None
        };

        // TODO: Update samplers?
    }

    fn apply_materials_v40(&self, mxmd: &mut MxmdV40) {
        // Recreate start indices and counts by assuming value ranges don't overlap.
        mxmd.materials.materials.clear();
        mxmd.materials.work_values.clear();
        mxmd.materials.shader_vars.clear();

        // Don't assume callbacks are used.
        let mut callbacks = mxmd.materials.callbacks.as_mut();
        if let Some(callbacks) = callbacks.as_mut() {
            callbacks.work_callbacks.clear();
            callbacks.material_indices = (0..self.models.materials.len() as u16).collect();
        }

        // Recreate materials to avoid restrictions with referencing existing ones.
        for (i, m) in self.models.materials.iter().enumerate() {
            // TODO: Is it ok to potentially add a new buffer index here?
            let technique = xc3_lib::mxmd::MaterialTechnique {
                technique_index: m.technique_index as u32,
                pass_type: m.pass_type,
                material_buffer_index: i as u16,
                flags: 1,
            };

            // TODO: Also rebuild alpha textures in case we need to add more.
            let new_material = xc3_lib::mxmd::legacy::Material {
                name: m.name.clone(),
                flags: m.flags,
                color: m.color,
                unk2: [0.0; 6],          // TODO: not always 0
                unk3: [0.0, 0.0, 0.999], // TODO: not always 0
                textures: m
                    .textures
                    .iter()
                    .map(|t| {
                        // TODO: How should the second sampler be set?
                        xc3_lib::mxmd::legacy::Texture {
                            texture_index: t.image_texture_index as u16,
                            sampler_flags: SamplerFlags::from(
                                &self.models.samplers[t.sampler_index],
                            ),
                        }
                    })
                    .collect(),
                state_flags: m.state_flags,
                m_unks1_1: m.m_unks1_1,
                m_unks1_2: m.m_unks1_2,
                m_unks1_3: m.m_unks1_3,
                m_unks1_4: m.m_unks1_4,
                work_value_start_index: mxmd.materials.work_values.len() as u32,
                shader_var_start_index: mxmd.materials.shader_vars.len() as u32,
                shader_var_count: m.shader_vars.len() as u32,
                techniques: vec![technique],
                unk4: [0; 8], // TODO: elements not always zero?
                unk5: 0,
                alpha_test_texture_index: m
                    .alpha_test
                    .as_ref()
                    .and_then(|a| {
                        let alpha_image_index =
                            m.textures[a.texture_index].image_texture_index as u16;
                        // TODO: This won't work since textures can be used more than once.
                        mxmd.materials
                            .alpha_test_textures
                            .as_ref()
                            .and_then(|textures| {
                                textures
                                    .iter()
                                    .position(|t| t.texture_index == alpha_image_index)
                            })
                    })
                    .unwrap_or_default() as u16,
                unk7: 0,
            };
            mxmd.materials.materials.push(new_material);

            mxmd.materials.work_values.extend_from_slice(&m.work_values);
            mxmd.materials.shader_vars.extend_from_slice(&m.shader_vars);

            // TODO: edit global color parameters like gAvaSkin?
        }
    }

    fn match_technique_attributes(&self, buffers: &mut ModelBuffers, mxmd: &MxmdV112) {
        let attribute_count =
            |attrs: &[VertexAttribute]| attrs.iter().filter(|a| a.buffer_index == 0).count();

        // Make sure the vertex buffers have an attribute for each vertex shader attribute.
        for (i, buffer) in buffers.vertex_buffers.iter_mut().enumerate() {
            let techniques = self.models.models.iter().flat_map(|m| {
                m.meshes.iter().find_map(|m| {
                    if m.vertex_buffer_index == i {
                        let technique_index =
                            self.models.materials[m.material_index].technique_index;
                        Some(&mxmd.materials.techniques[technique_index])
                    } else {
                        None
                    }
                })
            });
            // Buffers can be used with more than one material technique.
            // TODO: Will using the technique with the most buffer 0 attributes always work?
            if let Some(attributes) = techniques.map(|t| &t.attributes).reduce(|acc, e| {
                if attribute_count(e) > attribute_count(acc) {
                    e
                } else {
                    acc
                }
            }) {
                match_technique_attributes(buffer, attributes);
            }
        }
    }

    fn match_technique_attributes_legacy(&self, buffers: &mut ModelBuffers, mxmd: &MxmdV40) {
        let attribute_count =
            |attrs: &[VertexAttribute]| attrs.iter().filter(|a| a.buffer_index == 0).count();

        // Make sure the vertex buffers have an attribute for each vertex shader attribute.
        for (i, buffer) in buffers.vertex_buffers.iter_mut().enumerate() {
            let techniques = self.models.models.iter().flat_map(|m| {
                m.meshes.iter().find_map(|m| {
                    if m.vertex_buffer_index == i {
                        let technique_index =
                            self.models.materials[m.material_index].technique_index;
                        Some(&mxmd.materials.techniques[technique_index])
                    } else {
                        None
                    }
                })
            });
            // Buffers can be used with more than one material technique.
            // TODO: Will using the technique with the most buffer 0 attributes always work?
            if let Some(attributes) = techniques.map(|t| &t.attributes).reduce(|acc, e| {
                if attribute_count(e) > attribute_count(acc) {
                    e
                } else {
                    acc
                }
            }) {
                match_technique_attributes(buffer, attributes);
            }
        }
    }

    /// Similar to [Self::to_mxmd_model] but does not compress the new data or update streaming information.
    pub fn to_mxmd_v40_model_files(
        &self,
        mxmd: &MxmdV40,
    ) -> Result<
        (
            MxmdV40,
            mxmd::legacy::VertexData,
            Vec<ExtractedTexture<Mibl, TextureUsage>>,
        ),
        CreateModelError,
    > {
        let textures: Vec<_> = self
            .image_textures
            .iter()
            .map(ImageTexture::to_extracted_texture)
            .collect();

        let mut buffers = self.buffers.clone();
        self.match_technique_attributes_legacy(&mut buffers, mxmd);
        let new_vertex = buffers.to_vertex_data_legacy().unwrap();

        let mut new_mxmd = mxmd.clone();

        self.apply_materials_v40(&mut new_mxmd);

        new_mxmd.models.models = self
            .models
            .models
            .iter()
            .map(|model| xc3_lib::mxmd::legacy::Model {
                meshes: model
                    .meshes
                    .iter()
                    .map(|m| {
                        // TODO: Fill in remaining fields.
                        xc3_lib::mxmd::legacy::Mesh {
                            flags1: m.flags1,
                            flags2: m.flags2.into(),
                            vertex_buffer_index: m.vertex_buffer_index as u32,
                            index_buffer_index: m.index_buffer_index as u32,
                            material_index: m.material_index as u32,
                            unk2: 1,
                            unk3: 0,
                            unk4: 0,
                            ext_mesh_index: xc3_lib::mxmd::legacy::ExtMeshIndex::new(
                                0u16,
                                m.ext_mesh_index.unwrap_or_default() as u8,
                                0u8,
                            ),
                            unk6: 0,
                            index_buffer_index2: xc3_lib::mxmd::legacy::IndexBufferIndex2::new(
                                m.index_buffer_index2 as u8,
                                0u8.into(),
                            ),
                            unk8: 0,
                            unk9: 0,
                            unk10: 0,
                            unk11: 0,
                            unk12: 0,
                        }
                    })
                    .collect(),
                unk1: 0,
                max_xyz: model.max_xyz.to_array(),
                min_xyz: model.min_xyz.to_array(),
                bounding_radius: model.bounding_radius,
                unks: [0; 7],
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

        // This should be updated later.
        new_mxmd.streaming = None;

        Ok((new_mxmd, new_vertex, textures))
    }
}

// TODO: validate this in xc3_model on load?
fn match_technique_attributes(
    buffer: &mut crate::vertex::VertexBuffer,
    technique_attributes: &[VertexAttribute],
) {
    // TODO: Morph targets always require positions, normals, and tangents.
    // TODO: Update positions to use the existing positions?

    // Make sure the buffer attributes match the vertex shader's input attributes.
    // TODO: Is there a better way to match the shader order?
    // TODO: Do we ever need to add buffer1 attributes?
    let count = buffer.vertex_count();
    buffer.attributes = technique_attributes
        .iter()
        .filter(|a| a.buffer_index == 0)
        .map(|a| match_attribute(a.data_type, buffer, count))
        .collect();
}

macro_rules! attribute {
    ($buffer:ident, $count: expr, $variant:path $(, $fallback_variant:path)*) => {
        $buffer
            .attributes
            .iter()
            .find_map(|a| {
                if matches!(a, $variant(_)) {
                    Some(a.clone())
                } else {
                    None
                }
            })
            $(
                .or_else(|| {
                    $buffer
                        .attributes
                        .iter()
                        .find_map(|a| {
                            if matches!(a, $fallback_variant(_)) {
                                Some(a.clone())
                            } else {
                                None
                            }
                    })
                })
            )*
            .unwrap_or_else(|| {
                log::warn!(
                    "Assigning default values for missing required attribute {}",
                    stringify!($variant)
                );
                $variant(vec![Default::default(); $count])
            })
    };
}

fn match_attribute(
    data_type: xc3_lib::vertex::DataType,
    buffer: &crate::vertex::VertexBuffer,
    count: usize,
) -> AttributeData {
    // Find the corresponding attribute or fill in a default value.
    // Try attributes with matching usage in order for data like normals.
    match data_type {
        DataType::Position => attribute!(buffer, count, AttributeData::Position),
        DataType::SkinWeights2 => attribute!(buffer, count, AttributeData::SkinWeights),
        DataType::BoneIndices2 => attribute!(buffer, count, AttributeData::BoneIndices2),
        DataType::WeightIndex => attribute!(buffer, count, AttributeData::WeightIndex),
        DataType::WeightIndex2 => attribute!(buffer, count, AttributeData::WeightIndex2),
        DataType::TexCoord0 => attribute!(buffer, count, AttributeData::TexCoord0),
        DataType::TexCoord1 => attribute!(buffer, count, AttributeData::TexCoord1),
        DataType::TexCoord2 => attribute!(buffer, count, AttributeData::TexCoord2),
        DataType::TexCoord3 => attribute!(buffer, count, AttributeData::TexCoord3),
        DataType::TexCoord4 => attribute!(buffer, count, AttributeData::TexCoord4),
        DataType::TexCoord5 => attribute!(buffer, count, AttributeData::TexCoord5),
        DataType::TexCoord6 => attribute!(buffer, count, AttributeData::TexCoord6),
        DataType::TexCoord7 => attribute!(buffer, count, AttributeData::TexCoord7),
        DataType::TexCoord8 => attribute!(buffer, count, AttributeData::TexCoord8),
        DataType::Blend => attribute!(buffer, count, AttributeData::Blend),
        DataType::Unk15 => attribute!(buffer, count, AttributeData::Unk15),
        DataType::Unk16 => attribute!(buffer, count, AttributeData::Unk16),
        DataType::VertexColor => attribute!(buffer, count, AttributeData::VertexColor),
        DataType::Unk18 => attribute!(buffer, count, AttributeData::Unk18),
        DataType::Unk24 => attribute!(buffer, count, AttributeData::Unk24),
        DataType::Unk25 => attribute!(buffer, count, AttributeData::Unk25),
        DataType::Unk26 => attribute!(buffer, count, AttributeData::Unk26),
        DataType::Normal => {
            attribute!(
                buffer,
                count,
                AttributeData::Normal,
                AttributeData::Normal2,
                AttributeData::Normal3
            )
        }
        DataType::Tangent => attribute!(buffer, count, AttributeData::Tangent),
        DataType::Unk30 => attribute!(buffer, count, AttributeData::Unk30),
        DataType::Unk31 => attribute!(buffer, count, AttributeData::Unk31),
        DataType::Normal2 => {
            attribute!(
                buffer,
                count,
                AttributeData::Normal2,
                AttributeData::Normal,
                AttributeData::Normal3
            )
        }
        DataType::ValInf => attribute!(
            buffer,
            count,
            AttributeData::ValInf,
            AttributeData::Normal,
            AttributeData::Normal2,
            AttributeData::Normal3
        ),
        DataType::Normal3 => {
            attribute!(
                buffer,
                count,
                AttributeData::Normal3,
                AttributeData::Normal,
                AttributeData::Normal2
            )
        }
        DataType::VertexColor3 => attribute!(buffer, count, AttributeData::VertexColor3),
        DataType::Position2 => attribute!(buffer, count, AttributeData::Position2),
        DataType::Normal4 => attribute!(buffer, count, AttributeData::Normal4),
        DataType::OldPosition => attribute!(buffer, count, AttributeData::OldPosition),
        DataType::Tangent2 => attribute!(buffer, count, AttributeData::Tangent2),
        DataType::SkinWeights => attribute!(buffer, count, AttributeData::SkinWeights),
        DataType::BoneIndices => attribute!(buffer, count, AttributeData::BoneIndices),
        DataType::Flow => attribute!(buffer, count, AttributeData::Flow),
    }
}

fn apply_skinning(
    new_skinning: &mut xc3_lib::mxmd::Skinning,
    skinning: &crate::skinning::Skinning,
) {
    // TODO: How to preserve the case where render count < count?
    new_skinning.render_bone_count = skinning.bones.len() as u32;
    new_skinning.bone_count = skinning.bones.len() as u32;

    let mut bounds = Vec::new();
    let mut constraints = Vec::new();

    new_skinning.bones = skinning
        .bones
        .iter()
        .map(|bone| {
            let bounds_index = if let Some(b) = &bone.bounds {
                // Assume each bone has a unique index.
                let index = bounds.len() as u32;
                bounds.push(xc3_lib::mxmd::BoneBounds {
                    center: b.center.extend(0.0).to_array(),
                    size: b.size.extend(0.0).to_array(),
                });
                index
            } else {
                0
            };

            let constraint_index = if let Some(c) = &bone.constraint {
                // Assume each bone has a unique index.
                let index = constraints.len() as u8;
                constraints.push(xc3_lib::mxmd::BoneConstraint {
                    fixed_offset: c.fixed_offset.to_array(),
                    max_distance: c.max_distance,
                });
                index
            } else {
                0
            };

            xc3_lib::mxmd::Bone {
                name: bone.name.clone(),
                bounds_radius: bone.bounds.as_ref().map(|b| b.radius).unwrap_or_default(), // TODO: not always 0.0 if none?
                flags: xc3_lib::mxmd::BoneFlags::new(
                    matches!(&bone.constraint, Some(c) if c.constraint_type == BoneConstraintType::FixedOffset),
                    bone.bounds.is_some(),
                    matches!(&bone.constraint, Some(c) if c.constraint_type == BoneConstraintType::Distance),
                    bone.no_camera_overlap,
                    0u8.into(),
                ),
                constraint_index,
                parent_index: bone.constraint.as_ref().and_then(|c| c.parent_index).unwrap_or_default() as u8,
                bounds_index,
                unk: [0; 2],
            }
        })
        .collect();

    // Preserve the case where data is empty but not None.
    // TODO: Investigate why setting these to None can cause extra bone data to not be read in game.
    if !bounds.is_empty() {
        new_skinning.bounds = Some(bounds);
    }
    if !constraints.is_empty() {
        new_skinning.constraints = Some(constraints);
    }
}
