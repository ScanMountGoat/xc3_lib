use std::collections::BTreeMap;

use crate::gltf::texture::{
    albedo_generated_key, metallic_roughness_generated_key, normal_generated_key, TextureCache,
};
use crate::ModelRoot;
use gltf::json::validation::Checked::Valid;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct MaterialKey {
    pub root_index: usize,
    pub group_index: usize,
    pub models_index: usize,
    pub material_index: usize,
}

pub fn create_materials(
    roots: &[ModelRoot],
    texture_cache: &mut TextureCache,
) -> (Vec<gltf::json::Material>, BTreeMap<MaterialKey, usize>) {
    let mut materials = Vec::new();
    let mut material_indices = BTreeMap::new();

    for (root_index, root) in roots.iter().enumerate() {
        for (group_index, group) in root.groups.iter().enumerate() {
            for (models_index, models) in group.models.iter().enumerate() {
                for (material_index, material) in models.materials.iter().enumerate() {
                    let albedo_key = albedo_generated_key(material, root_index);
                    let albedo_index = texture_cache.insert(albedo_key);

                    let normal_key = normal_generated_key(material, root_index);
                    let normal_index = texture_cache.insert(normal_key);

                    let metallic_roughness_key =
                        metallic_roughness_generated_key(material, root_index);
                    let metallic_roughness_index = texture_cache.insert(metallic_roughness_key);

                    let material = create_material(
                        material,
                        albedo_index,
                        normal_index,
                        metallic_roughness_index,
                    );
                    let material_flattened_index = materials.len();
                    materials.push(material);

                    material_indices.insert(
                        MaterialKey {
                            root_index,
                            group_index,
                            models_index,
                            material_index,
                        },
                        material_flattened_index,
                    );
                }
            }
        }
    }

    (materials, material_indices)
}

fn create_material(
    material: &crate::Material,
    albedo_index: Option<u32>,
    normal_index: Option<u32>,
    metallic_roughness_index: Option<u32>,
) -> gltf::json::Material {
    gltf::json::Material {
        name: Some(material.name.clone()),
        pbr_metallic_roughness: gltf::json::material::PbrMetallicRoughness {
            base_color_texture: albedo_index.map(|i| gltf::json::texture::Info {
                index: gltf::json::Index::new(i),
                tex_coord: 0,
                extensions: None,
                extras: Default::default(),
            }),
            metallic_roughness_texture: metallic_roughness_index.map(|i| {
                gltf::json::texture::Info {
                    index: gltf::json::Index::new(i),
                    tex_coord: 0,
                    extensions: None,
                    extras: Default::default(),
                }
            }),
            ..Default::default()
        },
        normal_texture: normal_index.map(|i| gltf::json::material::NormalTexture {
            index: gltf::json::Index::new(i),
            scale: 1.0,
            tex_coord: 0,
            extensions: None,
            extras: Default::default(),
        }),
        occlusion_texture: metallic_roughness_index.map(|i| {
            gltf::json::material::OcclusionTexture {
                // Only the red channel is sampled for the occlusion texture.
                // We can reuse the metallic roughness texture red channel here.
                index: gltf::json::Index::new(i),
                strength: gltf::json::material::StrengthFactor(1.0),
                tex_coord: 0,
                extensions: None,
                extras: Default::default(),
            }
        }),
        alpha_mode: if material.alpha_test.is_some() {
            Valid(gltf::json::material::AlphaMode::Mask)
        } else {
            Valid(gltf::json::material::AlphaMode::Opaque)
        },
        alpha_cutoff: material
            .alpha_test
            .as_ref()
            .map(|a| gltf::json::material::AlphaCutoff(a.ref_value)),
        ..Default::default()
    }
}
