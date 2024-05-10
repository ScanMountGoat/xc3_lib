use glam::{Vec2, Vec3, Vec4};
use indexmap::IndexMap;
use xc3_lib::{
    msrd::Msrd,
    mxmd::{AlphaTable, Mxmd, VertexAttribute},
};

use crate::{vertex::AttributeData, ImageTexture, ModelRoot};

// TODO: Not possible to make files compatible with all game versions?
// TODO: Will it be possible to do full imports in the future?
// TODO: Include chr to support skeleton edits?
// TODO: How to properly test this?
pub fn create_mxmd_model(root: &ModelRoot, mxmd: &Mxmd, msrd: &Msrd) -> (Mxmd, Msrd) {
    // TODO: Does this need to even extract vertex/textures?
    let (_, spch, _) = msrd.extract_files(None).unwrap();

    let textures: Vec<_> = root
        .image_textures
        .iter()
        .map(ImageTexture::extracted_texture)
        .collect();

    let mut buffers = root.buffers.clone();
    match_buffers_technique_attributes(&mut buffers.vertex_buffers, &root.models, mxmd);

    let new_vertex = buffers.to_vertex_data().unwrap();

    let mut new_mxmd = mxmd.clone();

    // TODO: Rebuild materials.
    // TODO: How many of these mesh fields can use a default value?
    let mut alpha_table = IndexMap::new();

    let has_speff_materials = root
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

    new_mxmd.models.models = root
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
                    let new_index = alpha_table.len() as u16;
                    let alpha_table_index = *alpha_table
                        .entry((ext_index, m.lod & 0xff))
                        .or_insert(new_index);

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
                        unk_mesh_index1: m.unk_mesh_index1 as u16,
                        material_index: m.material_index as u16,
                        unk2: 0,
                        unk3: 0,
                        ext_mesh_index: m.ext_mesh_index.unwrap_or_default() as u16,
                        unk4: 0,
                        unk5: 0, // TODO: flags?
                        lod: m.lod,
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

fn match_buffers_technique_attributes(
    buffers: &mut [crate::vertex::VertexBuffer],
    models: &crate::Models,
    mxmd: &Mxmd,
) {
    let attribute_count =
        |attrs: &[VertexAttribute]| attrs.iter().filter(|a| a.buffer_index == 0).count();

    // Make sure the vertex buffers have an attribute for each vertex shader attribute.
    for (i, buffer) in buffers.iter_mut().enumerate() {
        let techniques = models.models.iter().flat_map(|m| {
            m.meshes.iter().find_map(|m| {
                if m.vertex_buffer_index == i {
                    let technique_index =
                        &mxmd.materials.materials[m.material_index].techniques[0].technique_index;
                    Some(&mxmd.materials.techniques[*technique_index as usize])
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

// TODO: validate this in xc3_model on load?
fn match_technique_attributes(
    buffer: &mut crate::vertex::VertexBuffer,
    technique_attributes: &[VertexAttribute],
) {
    // Make sure the buffer attributes match the vertex shader's input attributes.
    // TODO: Is there a better way to match the shader order?
    // TODO: How to handle morph targets and other buffers?
    if buffer.morph_targets.is_empty() {
        let count = buffer.vertex_count();
        buffer.attributes = technique_attributes
            .iter()
            .map(|a| match_attribute(a, buffer, count))
            .collect();
    }
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
            .unwrap_or_else(|| $variant(vec![$default; $count]))
    };
}

fn match_attribute(
    attribute: &VertexAttribute,
    buffer: &crate::vertex::VertexBuffer,
    count: usize,
) -> AttributeData {
    // Find the corresponding attribute or fill in a default value.
    use xc3_lib::vertex::DataType;
    match attribute.data_type {
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
        DataType::Normal4 => attribute!(buffer, AttributeData::Normal, Vec4::ZERO, count),
        DataType::OldPosition => attribute!(buffer, AttributeData::Position, Vec3::ZERO, count),
        DataType::Tangent2 => attribute!(buffer, AttributeData::Tangent, Vec4::ZERO, count),
        DataType::SkinWeights => attribute!(buffer, AttributeData::SkinWeights, Vec4::ZERO, count),
        DataType::BoneIndices => attribute!(buffer, AttributeData::BoneIndices, [0; 4], count),
        DataType::Flow => todo!(),
    }
}
