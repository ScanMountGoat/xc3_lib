use std::io::{Cursor, Read, Seek};

use glam::{Mat4, Vec3};
use xc3_lib::{
    map::FoliageMaterials,
    mibl::Mibl,
    msmd::{ChannelType, MapParts, Msmd, StreamEntry},
    mxmd::{MaterialFlags, ShaderUnkType},
    vertex::VertexData,
};
use xc3_shader::gbuffer_database::GBufferDatabase;

use crate::{
    materials, model_folder_name,
    texture::{merge_mibl, ImageTexture},
    Material, Model, ModelGroup, Texture,
};

// TODO: Assume all stream entries are used and extract them into temporary arrays?
// TODO: Will this reduce loading times?
// TODO: Rayon for loading?

pub fn load_map<R: Read + Seek>(
    msmd: &Msmd,
    wismda: &mut R,
    model_path: &str,
    shader_database: &xc3_shader::gbuffer_database::GBufferDatabase,
) -> Vec<ModelGroup> {
    let model_folder = model_folder_name(model_path);

    let textures: Vec<_> = msmd
        .textures
        .iter()
        .map(|texture| {
            // Load high resolution textures.
            // TODO: Merging doesn't always work?
            let base_mip_level = texture.high.decompress(wismda);
            let mibl_m = texture.mid.extract(wismda);
            // merge_mibl(base_mip_level, mibl_m)
            mibl_m.try_into().unwrap()
        })
        .collect();

    // TODO: Better way to combine models?
    let mut combined_models = Vec::new();
    for (i, env_model) in msmd.env_models.iter().enumerate() {
        let model = load_env_model(wismda, env_model, i, &model_folder, shader_database);
        combined_models.push(model);
    }

    for foliage_model in &msmd.foliage_models {
        let model = load_foliage_model(wismda, foliage_model);
        combined_models.push(model);
    }

    for (i, map_model) in msmd.map_models.iter().enumerate() {
        let model = load_map_model_group(
            wismda,
            map_model,
            i,
            &msmd.map_vertex_data,
            &textures,
            &model_folder,
            shader_database,
        );
        combined_models.push(model);
    }

    for (i, prop_model) in msmd.prop_models.iter().enumerate() {
        let model = load_prop_model_group(
            wismda,
            prop_model,
            i,
            &msmd.prop_vertex_data,
            &textures,
            msmd.parts.as_ref(),
            &model_folder,
            shader_database,
        );
        combined_models.push(model);
    }

    combined_models
}

fn load_prop_model_group<R: Read + Seek>(
    wismda: &mut R,
    prop_model: &xc3_lib::msmd::PropModel,
    model_index: usize,
    prop_vertex_data: &[StreamEntry<VertexData>],
    image_textures: &[ImageTexture],
    parts: Option<&MapParts>,
    model_folder: &str,
    shader_database: &xc3_shader::gbuffer_database::GBufferDatabase,
) -> ModelGroup {
    let prop_model_data = prop_model.entry.extract(wismda);

    // Get the textures referenced by the materials in this model.
    let image_textures = load_map_textures(&prop_model_data.textures, image_textures);

    let spch = shader_database
        .map_files
        .get(model_folder)
        .and_then(|map| map.prop_models.get(model_index));

    // TODO: cached textures?
    let materials = materials(&prop_model_data.materials, spch);

    // Load the base LOD model for each prop model.
    let mut models: Vec<_> = prop_model_data
        .lods
        .props
        .iter()
        .enumerate()
        .map(|(i, prop_lod)| {
            let base_lod_index = prop_lod.base_lod_index as usize;
            let vertex_data_index = prop_model_data.model_vertex_data_indices[base_lod_index];

            // TODO: Also cache vertex and index buffer creation?
            let vertex_data = prop_vertex_data[vertex_data_index as usize].extract(wismda);

            // Find all the instances referencing this prop.
            let instances = prop_model_data
                .lods
                .instances
                .iter()
                .filter(|instance| instance.prop_index as usize == i)
                .map(|instance| Mat4::from_cols_array_2d(&instance.transform))
                .collect();

            Model::from_model(
                &prop_model_data.models.models[base_lod_index],
                &vertex_data,
                instances,
            )
        })
        .collect();

    // TODO: Is this the correct way to handle animated props?
    // TODO: Document how this works in xc3_lib.
    // Add additional animated prop instances to the appropriate models.
    if let Some(parts) = parts {
        add_animated_part_instances(&mut models, &prop_model_data, parts);
    }

    ModelGroup {
        models,
        materials,
        image_textures,
    }
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

fn load_map_model_group<R: Read + Seek>(
    wismda: &mut R,
    model: &xc3_lib::msmd::MapModel,
    model_index: usize,
    vertex_data: &[xc3_lib::msmd::StreamEntry<VertexData>],
    image_textures: &[ImageTexture],
    model_folder: &str,
    shader_database: &GBufferDatabase,
) -> ModelGroup {
    let model_data = model.entry.extract(wismda);

    // Get the textures referenced by the materials in this model.
    let image_textures = load_map_textures(&model_data.textures, image_textures);

    let spch = shader_database
        .map_files
        .get(model_folder)
        .and_then(|map| map.map_models.get(model_index));

    let materials = materials(&model_data.materials, spch);

    let mut models = Vec::new();

    for group in model_data.groups.groups {
        let vertex_data_index = group.vertex_data_index as usize;
        let vertex_data = vertex_data[vertex_data_index].extract(wismda);

        // Each group has a base and low detail vertex data index.
        // Each model has an assigned vertex data index.
        // Find all the base detail models and meshes for each group.
        for (model, index) in model_data
            .models
            .models
            .iter()
            .zip(model_data.groups.model_vertex_data_indices.iter())
        {
            if *index as usize == vertex_data_index {
                let new_model = Model::from_model(model, &vertex_data, vec![Mat4::IDENTITY]);
                models.push(new_model);
            }
        }
    }

    ModelGroup {
        models,
        materials,
        image_textures,
    }
}

fn load_env_model<R: Read + Seek>(
    wismda: &mut R,
    model: &xc3_lib::msmd::EnvModel,
    model_index: usize,
    model_folder: &str,
    shader_database: &GBufferDatabase,
) -> ModelGroup {
    let model_data = model.entry.extract(wismda);

    // Environment models embed their own textures instead of using the MSMD.
    let image_textures: Vec<_> = model_data
        .textures
        .textures
        .iter()
        .map(|texture| {
            Mibl::read(&mut Cursor::new(&texture.mibl_data))
                .unwrap()
                .try_into()
                .unwrap()
        })
        .collect();

    let spch = shader_database
        .map_files
        .get(model_folder)
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

    ModelGroup {
        models,
        materials,
        image_textures,
    }
}

fn load_foliage_model<R: Read + Seek>(
    wismda: &mut R,
    model: &xc3_lib::msmd::FoliageModel,
) -> ModelGroup {
    let model_data = model.entry.extract(wismda);

    // Foliage models embed their own textures instead of using the MSMD.
    let image_textures: Vec<_> = model_data
        .textures
        .textures
        .iter()
        .map(|texture| {
            Mibl::read(&mut Cursor::new(&texture.mibl_data))
                .unwrap()
                .try_into()
                .unwrap()
        })
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

    ModelGroup {
        models,
        materials,
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

fn load_map_textures(
    textures: &[xc3_lib::map::Texture],
    image_textures: &[ImageTexture],
) -> Vec<ImageTexture> {
    textures
        .iter()
        .map(|item| {
            // TODO: Find a way to do this without expensive clones.
            // TODO: Handle texture index being -1?
            image_textures[item.texture_index.max(0) as usize].clone()
        })
        .collect()
}
