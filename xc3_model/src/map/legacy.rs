use std::{collections::BTreeSet, io::Cursor, sync::Mutex};

use glam::Mat4;
use image_dds::Surface;
use rayon::prelude::*;
use xc3_lib::{
    error::DecompressStreamError,
    map::legacy::{
        MapModelData, MapModelGroups, PropInstance, PropLod, PropLods, PropModelData, PropModelLod,
        PropPositions,
    },
    msmd::{
        StreamEntry,
        legacy::{MsmdV11, MsmdV11Legacy},
    },
    mtxt::Mtxt,
    mxmd::TextureUsage,
    spco::Spco,
};

use crate::{
    Model, ModelBuffers, ModelGroup, Models,
    error::LoadMapError,
    map::{TextureCache, apply_material_texture_indices},
    material::{GetProgramHash, create_materials_samplers_legacy},
    shader_database::ShaderDatabase,
};

#[tracing::instrument(skip_all)]
pub fn map_models_group_legacy(
    msmd: &MsmdV11,
    wismda: &[u8],
    compressed: bool,
    texture_indices: &[u16],
    texture_cache: &Mutex<TextureCache>,
    shader_database: Option<&ShaderDatabase>,
    part_id: Option<u16>,
) -> Result<ModelGroup, LoadMapError> {
    let buffers = create_buffers_legacy(&msmd.map_vertex_data, wismda, compressed)?;

    // Decompression is expensive, so run in parallel ahead of time.
    let map_model_data = msmd
        .map_models
        .par_iter()
        .filter_map(|m| {
            if part_id.map(|i| m.map_part_id == i).unwrap_or(true) {
                Some(m.entry.extract(&mut Cursor::new(wismda), compressed))
            } else {
                None
            }
        })
        .collect::<Result<Vec<_>, _>>()?;

    let models = map_model_data
        .par_iter()
        .map(|model_data| {
            // Remove one layer of indirection from texture lookups.
            let material_root_texture_indices: Vec<_> = model_data
                .textures
                .iter()
                .map(|t| {
                    texture_cache
                        .lock()
                        .unwrap()
                        .insert(t, &model_data.low_texture_indices)
                })
                .collect();

            load_map_model_group_legacy(
                model_data,
                texture_indices,
                &material_root_texture_indices,
                shader_database,
            )
        })
        .collect();

    Ok(ModelGroup { models, buffers })
}

// TODO: come up with a better name
#[tracing::instrument(skip_all)]
pub fn map_models_group_legacy2(
    msmd: &MsmdV11Legacy,
    casmda: &[u8],
    texture_indices: &[u16],
    texture_cache: &Mutex<TextureCache>,
    shader_database: Option<&ShaderDatabase>,
) -> Result<ModelGroup, LoadMapError> {
    let buffers = create_buffers_legacy2(&msmd.map_vertex_data, casmda)?;

    // TODO: does par_iter matter if this is never compressed?
    let map_model_data = msmd
        .map_models
        .iter()
        .map(|m| m.entry.extract_legacy(&mut Cursor::new(casmda)))
        .collect::<Result<Vec<_>, _>>()?;

    let models = map_model_data
        .par_iter()
        .map(|model_data| {
            // Remove one layer of indirection from texture lookups.
            let material_root_texture_indices: Vec<_> = model_data
                .textures
                .iter()
                .map(|t| {
                    texture_cache
                        .lock()
                        .unwrap()
                        .insert(t, &model_data.low_texture_indices)
                })
                .collect();

            load_map_model_group_legacy2(
                model_data,
                texture_indices,
                &material_root_texture_indices,
                shader_database,
            )
        })
        .collect();

    Ok(ModelGroup { models, buffers })
}

#[tracing::instrument(skip_all)]
fn load_map_model_group_legacy(
    model_data: &MapModelData<Spco>,
    texture_indices: &[u16],
    material_root_texture_indices: &[usize],
    shader_database: Option<&ShaderDatabase>,
) -> Models {
    load_map_model_group_legacy_inner(
        &model_data.models,
        &model_data.materials,
        &model_data.groups,
        model_data.shaders.items.first().map(|i| &i.spch),
        texture_indices,
        material_root_texture_indices,
        shader_database,
    )
}

#[tracing::instrument(skip_all)]
fn load_map_model_group_legacy2(
    model_data: &MapModelData<xc3_lib::mxmd::legacy::Shaders>,
    texture_indices: &[u16],
    material_root_texture_indices: &[usize],
    shader_database: Option<&ShaderDatabase>,
) -> Models {
    load_map_model_group_legacy_inner(
        &model_data.models,
        &model_data.materials,
        &model_data.groups,
        Some(&model_data.shaders),
        texture_indices,
        material_root_texture_indices,
        shader_database,
    )
}

fn load_map_model_group_legacy_inner<S: GetProgramHash>(
    map_models: &xc3_lib::mxmd::legacy::Models,
    materials: &xc3_lib::mxmd::legacy::Materials,
    unk8: &MapModelGroups,
    shader: Option<&S>,
    texture_indices: &[u16],
    material_root_texture_indices: &[usize],
    shader_database: Option<&ShaderDatabase>,
) -> Models {
    let (mut materials, samplers) =
        create_materials_samplers_legacy(&materials, texture_indices, shader, shader_database);
    apply_material_texture_indices(&mut materials, material_root_texture_indices);

    let models = unk8
        .model_group_indices
        .iter()
        .zip(map_models.models.iter())
        .filter_map(|(group_index, model)| {
            // TODO: Does switch use the same lookup for lods?
            let item1_index = *group_index as usize % unk8.groups.len();
            let lod_index = *group_index as usize / unk8.groups.len();
            let item1 = &unk8.groups[item1_index];
            let vertex_data_index = item1.unk1[lod_index] as usize;
            // Only load the high quality base LOD terrain for now.
            if lod_index == 0 {
                Some(Model::from_model_legacy(
                    model,
                    vec![Mat4::IDENTITY],
                    vertex_data_index,
                ))
            } else {
                None
            }
        })
        .collect();

    Models {
        models,
        materials,
        samplers,
        skinning: None,
        lod_data: None,
        morph_controller_names: Vec::new(),
        animation_morph_names: Vec::new(),
        material_animations: None, // TODO: material anims for map models?
        min_xyz: map_models.min_xyz.into(),
        max_xyz: map_models.max_xyz.into(),
    }
}

#[tracing::instrument(skip_all)]
pub fn props_group_legacy(
    msmd: &MsmdV11,
    wismda: &[u8],
    compressed: bool,
    texture_indices: &[u16],
    texture_cache: &Mutex<TextureCache>,
    shader_database: Option<&ShaderDatabase>,
    part_id: Option<u16>,
) -> Result<ModelGroup, LoadMapError> {
    let buffers = create_buffers_legacy(&msmd.prop_vertex_data, wismda, compressed)?;

    // Decompression is expensive, so run in parallel ahead of time.
    let prop_positions: Vec<_> = msmd
        .prop_positions
        .par_iter()
        .map(|p| p.extract(&mut Cursor::new(wismda), compressed))
        .collect::<Result<Vec<_>, _>>()?;

    let prop_model_data: Vec<_> = msmd
        .prop_models
        .par_iter()
        .filter_map(|m| {
            if part_id.map(|i| m.map_part_id == i).unwrap_or(true) {
                Some(m.entry.extract(&mut Cursor::new(wismda), compressed))
            } else {
                None
            }
        })
        .collect::<Result<Vec<_>, _>>()?;

    let models = prop_model_data
        .par_iter()
        .map(|model_data| {
            // Remove layers of indirection from texture lookups.
            let material_root_texture_indices: Vec<_> = model_data
                .textures
                .iter()
                .map(|t| {
                    texture_cache
                        .lock()
                        .unwrap()
                        .insert(t, &model_data.low_texture_entry_indices)
                })
                .collect();

            load_prop_model_group_legacy(
                model_data,
                &prop_positions,
                texture_indices,
                &material_root_texture_indices,
                shader_database,
            )
        })
        .collect();

    Ok(ModelGroup { models, buffers })
}

// TODO: come up with a better name
#[tracing::instrument(skip_all)]
pub fn props_group_legacy2(
    msmd: &MsmdV11Legacy,
    casmda: &[u8],
    texture_indices: &[u16],
    texture_cache: &Mutex<TextureCache>,
    shader_database: Option<&ShaderDatabase>,
) -> Result<ModelGroup, LoadMapError> {
    let buffers = create_buffers_legacy2(&msmd.prop_vertex_data, casmda)?;

    // TODO: does par_iter matter if this is never compressed?
    let prop_positions: Vec<_> = msmd
        .prop_positions
        .par_iter()
        .map(|p| p.extract_legacy(&mut Cursor::new(casmda)))
        .collect::<Result<Vec<_>, _>>()?;

    let prop_model_data: Vec<_> = msmd
        .prop_models
        .par_iter()
        .map(|m| m.entry.extract_legacy(&mut Cursor::new(casmda)))
        .collect::<Result<Vec<_>, _>>()?;

    let models = prop_model_data
        .par_iter()
        .map(|model_data| {
            // Remove layers of indirection from texture lookups.
            let material_root_texture_indices: Vec<_> = model_data
                .textures
                .iter()
                .map(|t| {
                    texture_cache
                        .lock()
                        .unwrap()
                        .insert(t, &model_data.low_texture_entry_indices)
                })
                .collect();

            load_prop_model_group_legacy2(
                model_data,
                &prop_positions,
                texture_indices,
                &material_root_texture_indices,
                shader_database,
            )
        })
        .collect();

    Ok(ModelGroup { models, buffers })
}

#[tracing::instrument(skip_all)]
fn load_prop_model_group_legacy(
    model_data: &PropModelData<Spco>,
    prop_positions: &[PropPositions],
    texture_indices: &[u16],
    material_root_texture_indices: &[usize],
    shader_database: Option<&ShaderDatabase>,
) -> Models {
    load_prop_model_group_legacy_inner(
        &model_data.models,
        &model_data.lods,
        &model_data.materials,
        &model_data.model_vertex_data_indices,
        model_data.shaders.items.first().map(|i| &i.spch),
        prop_positions,
        texture_indices,
        material_root_texture_indices,
        shader_database,
    )
}

#[tracing::instrument(skip_all)]
fn load_prop_model_group_legacy2(
    model_data: &PropModelData<xc3_lib::mxmd::legacy::Shaders>,
    prop_positions: &[PropPositions],
    texture_indices: &[u16],
    material_root_texture_indices: &[usize],
    shader_database: Option<&ShaderDatabase>,
) -> Models {
    load_prop_model_group_legacy_inner(
        &model_data.models,
        &model_data.lods,
        &model_data.materials,
        &model_data.model_vertex_data_indices,
        Some(&model_data.shaders),
        prop_positions,
        texture_indices,
        material_root_texture_indices,
        shader_database,
    )
}

fn load_prop_model_group_legacy_inner<S: GetProgramHash>(
    map_models: &xc3_lib::mxmd::legacy::Models,
    lods: &PropLods,
    materials: &xc3_lib::mxmd::legacy::Materials,
    model_vertex_data_indices: &[u32],
    shader: Option<&S>,
    _prop_positions: &[PropPositions],
    texture_indices: &[u16],
    material_root_texture_indices: &[usize],
    shader_database: Option<&ShaderDatabase>,
) -> Models {
    // Calculate instances separately from models.
    // This allows us to avoid loading unused models later.
    let mut model_instances = vec![Vec::new(); map_models.models.len()];

    // Load instances for each base LOD model.
    add_prop_instances_legacy(
        &mut model_instances,
        &lods.props,
        &lods.lods,
        &lods.instances,
    );

    // TODO: Group by vertex data index?
    // TODO: empty groups?

    // TODO: Prop info and prop positions?

    let (mut materials, samplers) =
        create_materials_samplers_legacy(&materials, texture_indices, shader, shader_database);
    apply_material_texture_indices(&mut materials, material_root_texture_indices);

    let mut models = Models {
        models: Vec::new(),
        materials,
        samplers,
        skinning: None,
        lod_data: None,
        morph_controller_names: Vec::new(),
        animation_morph_names: Vec::new(),
        material_animations: None, // TODO: material anims for prop models?
        min_xyz: map_models.min_xyz.into(),
        max_xyz: map_models.max_xyz.into(),
    };

    let base_lods: BTreeSet<_> = lods.props.iter().map(|p| p.base_lod_index).collect();

    for (i, ((model, vertex_data_index), instances)) in map_models
        .models
        .iter()
        .zip(model_vertex_data_indices.iter())
        .zip(model_instances.into_iter())
        .enumerate()
    {
        // Only keep the base LOD.
        if base_lods.contains(&(i as u32)) {
            let group = Model::from_model_legacy(model, instances, *vertex_data_index as usize);
            models.models.push(group);
        }
    }

    models
}

fn add_prop_instances_legacy(
    model_instances: &mut [Vec<Mat4>],
    prop_lods: &[PropLod],
    model_lods: &[PropModelLod],
    instances: &[PropInstance],
) {
    // The indices only use the first 28 bits (0xFFFFFF).
    for instance in instances {
        let prop_lod = &prop_lods[instance.prop_lod_index as usize];
        let base_lod_index = (prop_lod.base_lod_index & 0xFFFFFFF) as usize;

        let model_lod = &model_lods[base_lod_index];
        let model_index = (model_lod.model_index & 0xFFFFFF) as usize;

        model_instances[model_index].push(Mat4::from_cols_array_2d(&instance.transform));
    }
}

#[tracing::instrument(skip_all)]
fn create_buffers_legacy(
    vertex_data: &[StreamEntry<xc3_lib::mxmd::legacy::VertexData>],
    wismda: &[u8],
    compressed: bool,
) -> Result<Vec<ModelBuffers>, DecompressStreamError> {
    // Process vertex data ahead of time in parallel.
    // This gives better CPU utilization and avoids redundant processing.
    vertex_data
        .par_iter()
        .map(|e| {
            let vertex_data = e.extract(&mut Cursor::new(wismda), compressed)?;
            ModelBuffers::from_vertex_data_legacy(&vertex_data, binrw::Endian::Little)
                .map_err(Into::into)
        })
        .collect()
}

// TODO: share code for xcx de and xcx
#[tracing::instrument(skip_all)]
fn create_buffers_legacy2(
    vertex_data: &[StreamEntry<xc3_lib::mxmd::legacy::VertexData>],
    casmda: &[u8],
) -> Result<Vec<ModelBuffers>, DecompressStreamError> {
    // Process vertex data ahead of time in parallel.
    // This gives better CPU utilization and avoids redundant processing.
    vertex_data
        .par_iter()
        .map(|e| {
            let vertex_data = e.extract_legacy(&mut Cursor::new(casmda))?;
            ModelBuffers::from_vertex_data_legacy(&vertex_data, binrw::Endian::Little)
                .map_err(Into::into)
        })
        .collect()
}

#[tracing::instrument(skip_all)]
pub fn load_low_textures_legacy(
    msmd: &MsmdV11Legacy,
    casmda: &[u8],
    e: &StreamEntry<xc3_lib::msmd::legacy::LowTexturesLegacy>,
) -> Result<Vec<(TextureUsage, Surface<Vec<u8>>)>, LoadMapError> {
    let textures = e.extract_legacy(&mut Cursor::new(&casmda))?;
    textures
        .textures
        .iter()
        .map(|t| {
            let usage = xc3_lib::mxmd::TextureUsage::Unk4; // TODO: keep the legacy texture usage;

            // TODO: investigate how this index is used.
            // TODO: avoid unwrap.
            let mtxt = if let Ok(i) = usize::try_from(t.unk2) {
                // TODO: Are these ever reused?
                let mut reader = Cursor::new(casmda);
                msmd.terrain_streaming_textures[i]
                    .extract_legacy(&mut reader)
                    .unwrap()
            } else {
                Mtxt::from_bytes(&t.mtxt_data).unwrap()
            };

            Ok((usage, mtxt.to_surface().unwrap()))
        })
        .collect::<Result<Vec<_>, LoadMapError>>()
}

#[tracing::instrument(skip_all)]
pub fn load_high_texture_legacy(
    casmda: &[u8],
    texture: &xc3_lib::msmd::legacy::TextureLegacy,
) -> Result<Surface<Vec<u8>>, LoadMapError> {
    let mut casmda = Cursor::new(&casmda);
    let mtxt_m = texture.mid.extract_legacy(&mut casmda)?;

    // TODO: load base mip level for mtxt
    if texture.base_mip.decompressed_size > 0 {
        let base_mip_level = texture.base_mip.decompress(&mut casmda, false)?;

        // TODO: Avoid unwrap.
        Ok(mtxt_m.to_surface_with_base_mip(&base_mip_level).unwrap())
    } else {
        Ok(mtxt_m.to_surface().unwrap())
    }
}
