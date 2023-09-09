use std::{io::Cursor, path::Path};

use glam::{Mat4, Vec3};
use rayon::prelude::*;
use xc3_lib::{
    map::{FoliageMaterials, PropInstance, PropLod, PropPositions},
    msmd::{ChannelType, MapParts, Msmd, StreamEntry},
    mxmd::{ShaderUnkType, StateFlags},
    vertex::VertexData,
};
use xc3_shader::gbuffer_database::GBufferDatabase;

use crate::{
    create_materials, create_samplers, model_name,
    texture::ImageTexture,
    vertex::{read_index_buffers, read_vertex_buffers},
    Material, Model, ModelBuffers, ModelGroup, ModelRoot, Models, Texture,
};

// TODO: Document loading the database in an example.
/// Load a map from a `.wismhd` file.
/// The corresponding `.wismda` should be in the same directory.
pub fn load_map<P: AsRef<Path>>(
    wismhd_path: P,
    shader_database: Option<&GBufferDatabase>,
) -> Vec<ModelRoot> {
    let msmd = Msmd::from_file(wismhd_path.as_ref()).unwrap();
    let wismda = std::fs::read(wismhd_path.as_ref().with_extension("wismda")).unwrap();

    // Loading is CPU intensive due to decompression and decoding.
    // The .wismda is loaded into memory as &[u8].
    // Extracting can be parallelized without locks by creating multiple readers.
    let model_folder = model_name(wismhd_path.as_ref());

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

    let group = map_models_group(&msmd, &wismda, compressed, &model_folder, shader_database);
    groups.push(group);

    let group = props_group(&msmd, &wismda, compressed, model_folder, shader_database);
    groups.push(group);

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

fn map_models_group(
    msmd: &Msmd,
    wismda: &Vec<u8>,
    compressed: bool,
    model_folder: &str,
    shader_database: Option<&GBufferDatabase>,
) -> ModelGroup {
    let buffers = create_buffers(&msmd.map_vertex_data, wismda, compressed);

    let mut models = Vec::new();
    models.par_extend(
        msmd.map_models
            .par_iter()
            .enumerate()
            .flat_map(|(i, model)| {
                let model_data = model.entry.extract(&mut Cursor::new(wismda), compressed);
                load_map_model_group(&model_data, i, model_folder, shader_database)
            }),
    );

    ModelGroup { models, buffers }
}

fn props_group(
    msmd: &Msmd,
    wismda: &Vec<u8>,
    compressed: bool,
    model_folder: String,
    shader_database: Option<&GBufferDatabase>,
) -> ModelGroup {
    let buffers = create_buffers(&msmd.prop_vertex_data, wismda, compressed);

    let prop_positions: Vec<_> = msmd
        .prop_positions
        .par_iter()
        .map(|p| p.extract(&mut Cursor::new(wismda), compressed))
        .collect();

    let models = msmd
        .prop_models
        .par_iter()
        .enumerate()
        .map(|(i, model)| {
            let model_data = model.entry.extract(&mut Cursor::new(wismda), compressed);

            load_prop_model_group(
                &model_data,
                i,
                msmd.parts.as_ref(),
                &prop_positions,
                &model_folder,
                shader_database,
            )
        })
        .collect();

    ModelGroup { models, buffers }
}

fn create_buffers(
    vertex_data: &[StreamEntry<VertexData>],
    wismda: &Vec<u8>,
    compressed: bool,
) -> Vec<ModelBuffers> {
    // Process vertex data ahead of time in parallel.
    // This gives better CPU utilization and avoids redundant processing.
    vertex_data
        .par_iter()
        .map(|e| {
            // Assume maps have no skeletons for now.
            let vertex_data = e.extract(&mut Cursor::new(wismda), compressed);
            ModelBuffers {
                vertex_buffers: read_vertex_buffers(&vertex_data, None),
                index_buffers: read_index_buffers(&vertex_data),
            }
        })
        .collect()
}

fn load_prop_model_group(
    model_data: &xc3_lib::map::PropModelData,
    model_index: usize,
    parts: Option<&MapParts>,
    prop_positions: &[PropPositions],
    model_folder: &str,
    shader_database: Option<&GBufferDatabase>,
) -> Models {
    let spch = shader_database
        .and_then(|database| database.map_files.get(model_folder))
        .and_then(|map| map.prop_models.get(model_index));

    // Calculate instances separately from models.
    // This allows us to avoid loading unused models later.
    let mut model_instances = vec![Vec::new(); model_data.models.models.len()];

    // Load instances for each base LOD model.
    add_prop_instances(
        &mut model_instances,
        &model_data.lods.props,
        &model_data.lods.instances,
    );

    // Add additional instances if present.
    for info in &model_data.prop_info {
        let additional_instances = &prop_positions[info.prop_position_entry_index as usize];
        add_prop_instances(
            &mut model_instances,
            &model_data.lods.props,
            &additional_instances.instances,
        );

        if let Some(parts) = parts {
            add_animated_part_instances(
                &mut model_instances,
                additional_instances.animated_parts_start_index as usize,
                additional_instances.animated_parts_count as usize,
                parts,
            );
        }
    }

    // TODO: Is this the correct way to handle animated props?
    // TODO: Document how this works in xc3_lib.
    // Add additional animated prop instances to the appropriate models.
    if let Some(parts) = parts {
        add_animated_part_instances(
            &mut model_instances,
            model_data.lods.animated_parts_start_index as usize,
            model_data.lods.animated_parts_count as usize,
            parts,
        );
    }

    // TODO: Group by vertex data index?
    // TODO: empty groups?

    // TODO: Create material data only once.
    let mut materials = create_materials(&model_data.materials, spch);
    apply_material_texture_indices(&mut materials, &model_data.textures);

    let samplers = create_samplers(&model_data.materials);

    let mut models = Models {
        models: Vec::new(),
        materials,
        samplers,
        skeleton: None,
        base_lod_indices: model_data
            .models
            .lod_data
            .as_ref()
            .map(|data| data.items2.iter().map(|i| i.base_lod_index).collect()),
        min_xyz: model_data.models.min_xyz,
        max_xyz: model_data.models.max_xyz,
    };

    for ((model, vertex_data_index), instances) in model_data
        .models
        .models
        .iter()
        .zip(model_data.model_vertex_data_indices.iter())
        .zip(model_instances.into_iter())
    {
        // Avoid loading unused prop models.
        if !instances.is_empty() {
            let group = Model::from_model(model, instances, *vertex_data_index as usize);
            models.models.push(group);
        }
    }

    models
}

fn add_prop_instances(
    model_instances: &mut [Vec<Mat4>],
    props: &[PropLod],
    instances: &[PropInstance],
) {
    for instance in instances {
        let prop_lod = &props[instance.prop_index as usize];
        let base_lod_index = prop_lod.base_lod_index as usize;
        // TODO: Should we also index into the PropModelLod?
        // TODO: Is PropModelLod.index always the same as its index in the list?
        model_instances[base_lod_index].push(Mat4::from_cols_array_2d(&instance.transform));
    }
}

fn add_animated_part_instances(
    model_instances: &mut [Vec<Mat4>],
    start_index: usize,
    count: usize,
    parts: &MapParts,
) {
    for i in start_index..start_index + count {
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
                // TODO: Handle other transforms.
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
        model_instances[instance.prop_index as usize].push(transform);
    }
}

fn load_map_model_group(
    model_data: &xc3_lib::map::MapModelData,
    model_index: usize,
    model_folder: &str,
    shader_database: Option<&GBufferDatabase>,
) -> Vec<Models> {
    let spch = shader_database
        .and_then(|database| database.map_files.get(model_folder))
        .and_then(|map| map.map_models.get(model_index));

    model_data
        .groups
        .groups
        .iter()
        .enumerate()
        .map(|(group_index, group)| {
            let vertex_data_index = group.vertex_data_index as usize;

            // Each group has a base and low detail vertex data index.
            // Each model has an assigned vertex data index.
            // Find all the base detail models and meshes for each group.
            // TODO: Why is the largest index twice the group count?
            // TODO: Are the larger indices LOD models?
            let mut models = Vec::new();
            for (model, index) in model_data
                .models
                .models
                .iter()
                .zip(model_data.groups.model_group_index.iter())
            {
                // TODO: Faster to just make empty groups and assign each model in a loop?
                if *index as usize == group_index {
                    let new_model =
                        Model::from_model(model, vec![Mat4::IDENTITY], vertex_data_index);
                    models.push(new_model);
                }
            }

            // TODO: Create material data only once.
            let mut materials = create_materials(&model_data.materials, spch);
            apply_material_texture_indices(&mut materials, &model_data.textures);

            let samplers = create_samplers(&model_data.materials);

            Models {
                models,
                materials,
                samplers,
                skeleton: None,
                base_lod_indices: model_data
                    .models
                    .lod_data
                    .as_ref()
                    .map(|data| data.items2.iter().map(|i| i.base_lod_index).collect()),
                min_xyz: model_data.models.min_xyz,
                max_xyz: model_data.models.max_xyz,
            }
        })
        .collect()
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

    let vertex_buffers = read_vertex_buffers(&model_data.vertex_data, None);
    let index_buffers = read_index_buffers(&model_data.vertex_data);

    ModelRoot {
        groups: vec![ModelGroup {
            models: vec![Models::from_models(
                &model_data.models,
                &model_data.materials,
                spch,
                None,
            )],
            buffers: vec![ModelBuffers {
                vertex_buffers,
                index_buffers,
            }],
        }],
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
        .map(|model| Model::from_model(model, vec![Mat4::IDENTITY], 0))
        .collect();

    let vertex_buffers = read_vertex_buffers(&model_data.vertex_data, None);
    let index_buffers = read_index_buffers(&model_data.vertex_data);

    // TODO: foliage samplers?
    // TODO: is it worth making a skeleton here?
    ModelRoot {
        groups: vec![ModelGroup {
            models: vec![Models {
                models,
                materials,
                samplers: Vec::new(),
                skeleton: None,
                base_lod_indices: model_data
                    .models
                    .lod_data
                    .map(|data| data.items2.iter().map(|i| i.base_lod_index).collect()),
                min_xyz: model_data.models.min_xyz,
                max_xyz: model_data.models.max_xyz,
            }],
            buffers: vec![ModelBuffers {
                vertex_buffers,
                index_buffers,
            }],
        }],
        image_textures,
    }
}

fn foliage_materials(materials: &FoliageMaterials) -> Vec<Material> {
    let materials = materials
        .materials
        .iter()
        .map(|material| {
            // TODO: Where are the textures?
            let textures = vec![Texture {
                image_texture_index: 0,
                sampler_index: 0,
            }];

            // TODO: Foliage shaders?
            let shader = None;

            // TODO: Flags?
            let flags = StateFlags {
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
                alpha_test: None,
                shader,
                unk_type: ShaderUnkType::Unk0,
                parameters: Default::default(),
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
