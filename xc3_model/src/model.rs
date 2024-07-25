use glam::{Vec2, Vec3, Vec4};
use indexmap::IndexMap;
use xc3_lib::{
    msrd::Msrd,
    mxmd::{AlphaTable, LodData, LodGroup, LodItem, Mxmd, VertexAttribute},
    vertex::DataType,
};

use crate::{
    vertex::{AttributeData, ModelBuffers},
    ImageTexture, IndexMapExt, ModelRoot,
};

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

impl ModelRoot {
    pub fn to_mxmd_model(&self, mxmd: &Mxmd, msrd: &Msrd) -> (Mxmd, Msrd) {
        // TODO: Does this need to even extract vertex/textures?
        let (_, spch, _) = msrd.extract_files(None).unwrap();

        let textures: Vec<_> = self
            .image_textures
            .iter()
            .map(ImageTexture::extracted_texture)
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
            .map(|model| xc3_lib::mxmd::Model {
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

                        xc3_lib::mxmd::Mesh {
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
            unk1: 0,
            items: data
                .items
                .iter()
                .map(|i| LodItem {
                    unk1: [0; 4],
                    unk2: i.unk2,
                    unk3: 0,
                    index: i.index,
                    unk5: i.unk5,
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

        self.apply_materials(&mut new_mxmd);

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

    fn apply_materials(&self, mxmd: &mut Mxmd) {
        // Recreate start indices and counts by assuming value ranges don't overlap.
        mxmd.materials.materials.clear();
        mxmd.materials.work_values.clear();
        mxmd.materials.shader_vars.clear();

        // TODO: Don't assume callbacks are used?
        let callbacks = mxmd.materials.callbacks.as_mut().unwrap();
        callbacks.work_callbacks.clear();
        callbacks.material_indices = (0..self.models.materials.len() as u16).collect();

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
                    .map(|t| xc3_lib::mxmd::Texture {
                        texture_index: t.image_texture_index as u16,
                        sampler_index: t.sampler_index as u16,
                        unk2: 0,
                        unk3: 0,
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
                callback_start_index: callbacks.work_callbacks.len() as u16,
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
                m_unks3: [0, m.m_unks3_1, 0, 0, 0, 0, 0, 0],
            };
            mxmd.materials.materials.push(new_material);

            mxmd.materials.work_values.extend_from_slice(&m.work_values);
            mxmd.materials.shader_vars.extend_from_slice(&m.shader_vars);
            callbacks
                .work_callbacks
                .extend_from_slice(&m.work_callbacks);
        }
    }

    fn match_technique_attributes(&self, buffers: &mut ModelBuffers, mxmd: &Mxmd) {
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
    ($buffer:ident, $variant:path, $default:expr, $count: expr) => {
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
            .unwrap_or_else(|| {
                log::warn!(
                    "Assigning default values for missing required attribute {}",
                    stringify!($variant)
                );
                $variant(vec![$default; $count])
            })
    };
}

fn match_attribute(
    data_type: xc3_lib::vertex::DataType,
    buffer: &crate::vertex::VertexBuffer,
    count: usize,
) -> AttributeData {
    // Find the corresponding attribute or fill in a default value.
    match data_type {
        DataType::Position => attribute!(buffer, AttributeData::Position, Vec3::ZERO, count),
        DataType::SkinWeights2 => attribute!(buffer, AttributeData::SkinWeights, Vec4::ZERO, count),
        DataType::BoneIndices2 => attribute!(buffer, AttributeData::BoneIndices, [0; 4], count),
        DataType::WeightIndex => attribute!(buffer, AttributeData::WeightIndex, [0; 2], count),
        DataType::WeightIndex2 => attribute!(buffer, AttributeData::WeightIndex, [0; 2], count),
        DataType::TexCoord0 => attribute!(buffer, AttributeData::TexCoord0, Vec2::ZERO, count),
        DataType::TexCoord1 => attribute!(buffer, AttributeData::TexCoord1, Vec2::ZERO, count),
        DataType::TexCoord2 => attribute!(buffer, AttributeData::TexCoord2, Vec2::ZERO, count),
        DataType::TexCoord3 => attribute!(buffer, AttributeData::TexCoord3, Vec2::ZERO, count),
        DataType::TexCoord4 => attribute!(buffer, AttributeData::TexCoord4, Vec2::ZERO, count),
        DataType::TexCoord5 => attribute!(buffer, AttributeData::TexCoord5, Vec2::ZERO, count),
        DataType::TexCoord6 => attribute!(buffer, AttributeData::TexCoord6, Vec2::ZERO, count),
        DataType::TexCoord7 => attribute!(buffer, AttributeData::TexCoord7, Vec2::ZERO, count),
        DataType::TexCoord8 => attribute!(buffer, AttributeData::TexCoord8, Vec2::ZERO, count),
        DataType::Blend => attribute!(buffer, AttributeData::Blend, Vec4::ZERO, count),
        DataType::Unk15 => todo!(),
        DataType::Unk16 => todo!(),
        DataType::VertexColor => attribute!(buffer, AttributeData::VertexColor, Vec4::ZERO, count),
        DataType::Unk18 => todo!(),
        DataType::Unk24 => todo!(),
        DataType::Unk25 => todo!(),
        DataType::Unk26 => todo!(),
        DataType::Normal => attribute!(buffer, AttributeData::Normal, Vec4::ZERO, count),
        DataType::Tangent => attribute!(buffer, AttributeData::Tangent, Vec4::ZERO, count),
        DataType::Unk30 => todo!(),
        DataType::Unk31 => todo!(),
        DataType::Normal2 => attribute!(buffer, AttributeData::Normal, Vec4::ZERO, count),
        DataType::Unk33 => todo!(),
        DataType::Normal3 => attribute!(buffer, AttributeData::Normal, Vec4::ZERO, count),
        DataType::VertexColor3 => attribute!(buffer, AttributeData::VertexColor, Vec4::ZERO, count),
        DataType::Position2 => attribute!(buffer, AttributeData::Position, Vec3::ZERO, count),
        DataType::Normal4 => attribute!(buffer, AttributeData::Normal4, Vec4::ZERO, count),
        DataType::OldPosition => attribute!(buffer, AttributeData::OldPosition, Vec3::ZERO, count),
        DataType::Tangent2 => attribute!(buffer, AttributeData::Tangent2, Vec4::ZERO, count),
        DataType::SkinWeights => attribute!(buffer, AttributeData::SkinWeights, Vec4::ZERO, count),
        DataType::BoneIndices => attribute!(buffer, AttributeData::BoneIndices, [0; 4], count),
        DataType::Flow => todo!(),
    }
}
