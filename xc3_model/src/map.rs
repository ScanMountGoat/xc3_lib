use std::{io::Cursor, path::Path};

use glam::{Mat4, Vec3};
use rayon::prelude::*;
use xc3_lib::{
    map::FoliageMaterials,
    msmd::{ChannelType, MapParts, Msmd, StreamEntry},
    mxmd::{MaterialFlags, ShaderUnkType},
    vertex::VertexData,
};
use xc3_shader::gbuffer_database::GBufferDatabase;

use crate::{
    materials, model_folder_name, texture::ImageTexture, Material, Model, ModelGroup, ModelRoot,
    Texture,
};

pub fn load_map<P: AsRef<Path>>(
    wismhd_path: P,
    shader_database: Option<&GBufferDatabase>,
) -> Vec<ModelRoot> {
    let msmd = Msmd::from_file(wismhd_path.as_ref()).unwrap();
    let wismda = std::fs::read(wismhd_path.as_ref().with_extension("wismda")).unwrap();

    // Loading is CPU intensive due to decompression and decoding.
    // The .wismda is loaded into memory as &[u8].
    // Extracting can be parallelized without locks by creating multiple readers.
    let model_folder = model_folder_name(wismhd_path.as_ref());

    // Some maps don't use XBC1 compressed archives in the .wismda file.
    let compressed = msmd.wismda_info.compressed_length != msmd.wismda_info.decompressed_length;

    // TODO: Better way to combine models?
    let mut roots = Vec::new();

    roots.par_extend(msmd.env_models.par_iter().enumerate().map(|(i, model)| {
        load_env_model(
            &wismda,
            compressed,
            model,
            i,
            &model_folder,
            shader_database,
        )
    }));

    roots.par_extend(
        msmd.foliage_models
            .par_iter()
            .map(|foliage_model| load_foliage_model(&wismda, compressed, foliage_model)),
    );

    let mut groups = Vec::new();

    // Process vertex data ahead of time in parallel.
    // This gives better CPU utilization and avoids redundant processing.
    let map_vertex_data = extract_vertex_data(&msmd.map_vertex_data, &wismda, compressed);

    groups.par_extend(msmd.map_models.par_iter().enumerate().map(|(i, model)| {
        let model_data = model.entry.extract(&mut Cursor::new(&wismda), compressed);

        load_map_model_group(
            &model_data,
            i,
            &map_vertex_data,
            &model_folder,
            shader_database,
        )
    }));

    let prop_vertex_data = extract_vertex_data(&msmd.prop_vertex_data, &wismda, compressed);

    groups.par_extend(msmd.prop_models.par_iter().enumerate().map(|(i, model)| {
        let model_data = model.entry.extract(&mut Cursor::new(&wismda), compressed);

        load_prop_model_group(
            &model_data,
            i,
            &prop_vertex_data,
            msmd.parts.as_ref(),
            &model_folder,
            shader_database,
        )
    }));

    let image_textures: Vec<_> = msmd
        .textures
        .par_iter()
        .map(|texture| {
            // TODO: Merging doesn't always work?
            // TODO: Do all textures load a separate base mip level?
            let mut wismda = Cursor::new(&wismda);
            let mibl_m = texture.mid.extract(&mut wismda, compressed);
            ImageTexture::from_mibl(&mibl_m, None).unwrap()
        })
        .collect();

    roots.push(ModelRoot {
        groups,
        image_textures,
    });

    roots
}

fn extract_vertex_data(
    vertex_data: &[StreamEntry<VertexData>],
    wismda: &[u8],
    compressed: bool,
) -> Vec<VertexData> {
    vertex_data
        .par_iter()
        .map(|e| e.extract(&mut Cursor::new(wismda), compressed))
        .collect()
}

fn load_prop_model_group(
    model_data: &xc3_lib::map::PropModelData,
    model_index: usize,
    prop_vertex_data: &[VertexData],
    parts: Option<&MapParts>,
    model_folder: &str,
    shader_database: Option<&GBufferDatabase>,
) -> ModelGroup {
    let spch = shader_database
        .and_then(|database| database.map_files.get(model_folder))
        .and_then(|map| map.prop_models.get(model_index));

    let mut materials = materials(&model_data.materials, spch);
    apply_material_texture_indices(&mut materials, &model_data.textures);

    // Load the base LOD model for each prop model.
    let mut models: Vec<_> = model_data
        .lods
        .props
        .iter()
        .enumerate()
        .filter_map(|(i, prop_lod)| {
            let base_lod_index = prop_lod.base_lod_index as usize;
            let vertex_data_index = model_data.model_vertex_data_indices[base_lod_index];

            // TODO: Also cache vertex and index buffer creation?
            let vertex_data = &prop_vertex_data[vertex_data_index as usize];

            // Find all the instances referencing this prop.
            let instances = model_data
                .lods
                .instances
                .iter()
                .filter(|instance| instance.prop_index as usize == i)
                .map(|instance| Mat4::from_cols_array_2d(&instance.transform))
                .collect();

            Some(Model::from_model(
                model_data.models.models.get(base_lod_index)?,
                vertex_data,
                instances,
            ))
        })
        .collect();

    // TODO: Is this the correct way to handle animated props?
    // TODO: Document how this works in xc3_lib.
    // Add additional animated prop instances to the appropriate models.
    if let Some(parts) = parts {
        add_animated_part_instances(&mut models, model_data, parts);
    }

    ModelGroup { models, materials }
}

fn add_animated_part_instances(
    models: &mut [Model],
    prop_model_data: &xc3_lib::map::PropModelData,
    parts: &MapParts,
) {
    let start = prop_model_data.lods.animated_parts_start_index as usize;
    let count = prop_model_data.lods.animated_parts_count as usize;

    for i in start..start + count {
        let instance = &parts.animated_instances[i];
        let animation = &parts.instance_animations[i];

        // Each instance has a base transform as well as animation data.
        let mut transform = Mat4::from_cols_array_2d(&instance.transform);

        // Get the first frame of the animation channels.
        let mut translation: Vec3 = animation.translation.into();

        // TODO: Do these add to or replace the base values?
        for channel in &animation.channels {
            match channel.channel_type {
                ChannelType::TranslationX => {
                    translation.x += channel
                        .keyframes
                        .get(0)
                        .map(|f| f.value)
                        .unwrap_or_default()
                }
                ChannelType::TranslationY => {
                    translation.y += channel
                        .keyframes
                        .get(0)
                        .map(|f| f.value)
                        .unwrap_or_default()
                }
                ChannelType::TranslationZ => {
                    translation.z += channel
                        .keyframes
                        .get(0)
                        .map(|f| f.value)
                        .unwrap_or_default()
                }
                ChannelType::RotationX => (),
                ChannelType::RotationY => (),
                ChannelType::RotationZ => (),
                ChannelType::ScaleX => (),
                ChannelType::ScaleY => (),
                ChannelType::ScaleZ => (),
            }
        }
        // TODO: transform order?
        transform = Mat4::from_translation(translation) * transform;

        models[instance.prop_index as usize]
            .instances
            .push(transform);
    }
}

fn load_map_model_group(
    model_data: &xc3_lib::map::MapModelData,
    model_index: usize,
    vertex_data: &[VertexData],
    model_folder: &str,
    shader_database: Option<&GBufferDatabase>,
) -> ModelGroup {
    let spch = shader_database
        .and_then(|database| database.map_files.get(model_folder))
        .and_then(|map| map.map_models.get(model_index));

    let mut materials = materials(&model_data.materials, spch);
    apply_material_texture_indices(&mut materials, &model_data.textures);

    let mut models = Vec::new();

    for (group_index, group) in model_data.groups.groups.iter().enumerate() {
        let vertex_data_index = group.vertex_data_index as usize;
        let vertex_data = &vertex_data[vertex_data_index];

        // Each group has a base and low detail vertex data index.
        // Each model has an assigned vertex data index.
        // Find all the base detail models and meshes for each group.
        // TODO: Why is the largest index twice the group count?
        // TODO: Are the larger indices LOD models?
        for (model, index) in model_data
            .models
            .models
            .iter()
            .zip(model_data.groups.model_group_index.iter())
        {
            // TODO: Faster to just make empty groups and assign each model in a loop?
            if *index as usize == group_index {
                let new_model = Model::from_model(model, vertex_data, vec![Mat4::IDENTITY]);
                models.push(new_model);
            }
        }
    }

    ModelGroup { models, materials }
}

fn load_env_model(
    wismda: &[u8],
    compressed: bool,
    model: &xc3_lib::msmd::EnvModel,
    model_index: usize,
    model_folder: &str,
    shader_database: Option<&GBufferDatabase>,
) -> ModelRoot {
    let mut wismda = Cursor::new(&wismda);

    let model_data = model.entry.extract(&mut wismda, compressed);

    // Environment models embed their own textures instead of using the MSMD.
    let image_textures: Vec<_> = model_data
        .textures
        .textures
        .iter()
        .map(ImageTexture::from_packed_texture)
        .collect();

    let spch = shader_database
        .and_then(|database| database.map_files.get(model_folder))
        .and_then(|map| map.env_models.get(model_index));

    let materials = materials(&model_data.materials, spch);

    let models = model_data
        .models
        .models
        .iter()
        .map(|model| {
            // TODO: Avoid creating vertex buffers more than once?
            Model::from_model(model, &model_data.vertex_data, vec![Mat4::IDENTITY])
        })
        .collect();

    ModelRoot {
        groups: vec![ModelGroup { models, materials }],
        image_textures,
    }
}

fn load_foliage_model(
    wismda: &[u8],
    compressed: bool,
    model: &xc3_lib::msmd::FoliageModel,
) -> ModelRoot {
    let mut wismda = Cursor::new(&wismda);

    let model_data = model.entry.extract(&mut wismda, compressed);

    // Foliage models embed their own textures instead of using the MSMD.
    let image_textures: Vec<_> = model_data
        .textures
        .textures
        .iter()
        .map(ImageTexture::from_packed_texture)
        .collect();

    let materials = foliage_materials(&model_data.materials);

    // TODO: foliage models are instanced somehow for grass clumps?
    let models = model_data
        .models
        .models
        .iter()
        .map(|model| {
            // TODO: Avoid creating vertex buffers more than once?
            Model::from_model(model, &model_data.vertex_data, vec![Mat4::IDENTITY])
        })
        .collect();

    ModelRoot {
        groups: vec![ModelGroup { models, materials }],
        image_textures,
    }
}

pub fn foliage_materials(materials: &FoliageMaterials) -> Vec<Material> {
    let materials = materials
        .materials
        .iter()
        .map(|material| {
            // TODO: Where are the textures?
            let textures = vec![Texture {
                image_texture_index: 0,
            }];

            // TODO: Foliage shaders?
            let shader = None;

            // TODO: Flags?
            let flags = MaterialFlags {
                flag0: 0,
                blend_state: xc3_lib::mxmd::BlendState::Disabled,
                cull_mode: xc3_lib::mxmd::CullMode::Disabled,
                flag3: 0,
                stencil_state1: xc3_lib::mxmd::StencilState1::Always,
                stencil_state2: xc3_lib::mxmd::StencilState2::Disabled,
                depth_func: xc3_lib::mxmd::DepthFunc::LessEqual,
                flag7: 0,
            };

            Material {
                name: material.name.clone(),
                flags,
                textures,
                shader,
                unk_type: ShaderUnkType::Unk0,
            }
        })
        .collect();

    materials
}

fn apply_material_texture_indices(
    materials: &mut Vec<Material>,
    textures: &[xc3_lib::map::Texture],
) {
    // Not all textures are referenced by each material.
    // Apply indices here to reduce indirection for consuming code.
    for material in materials {
        for texture in &mut material.textures {
            // TODO: How to handle texture index being -1?
            let index = textures[texture.image_texture_index].texture_index.max(0) as usize;
            texture.image_texture_index = index;
        }
    }
}
