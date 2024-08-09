use std::collections::BTreeMap;

use crate::gltf::texture::{
    albedo_generated_key, metallic_roughness_generated_key, normal_generated_key, TextureCache,
};
use crate::{AddressMode, ImageTexture, Sampler};
use gltf::json::validation::Checked::Valid;

use super::texture::{emissive_generated_key, GeneratedImageKey, ImageIndex};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct MaterialKey {
    pub root_index: usize,
    pub group_index: usize,
    pub models_index: usize,
    pub material_index: usize,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct SamplerKey {
    root_index: usize,
    group_index: usize,
    models_index: usize,
}

#[derive(Default)]
pub struct MaterialCache {
    pub materials: Vec<gltf::json::Material>,
    material_indices: BTreeMap<MaterialKey, usize>,

    pub textures: Vec<gltf::json::Texture>,

    pub samplers: Vec<gltf::json::texture::Sampler>,
    sampler_base_indices: BTreeMap<SamplerKey, usize>,
}

impl MaterialCache {
    pub fn insert_samplers(
        &mut self,
        models: &crate::Models,
        root_index: usize,
        group_index: usize,
        models_index: usize,
    ) {
        let sampler_base_index = self.samplers.len();
        self.samplers
            .extend(models.samplers.iter().map(create_sampler));

        self.sampler_base_indices.insert(
            SamplerKey {
                root_index,
                group_index,
                models_index,
            },
            sampler_base_index,
        );
    }

    pub fn insert(
        &mut self,
        material: &crate::Material,
        texture_cache: &mut TextureCache,
        image_textures: &[ImageTexture],
        key: MaterialKey,
    ) -> usize {
        match self.material_indices.get(&key) {
            Some(index) => *index,
            None => {
                let sampler_base_index = self
                    .sampler_base_indices
                    .get(&SamplerKey {
                        root_index: key.root_index,
                        group_index: key.group_index,
                        models_index: key.models_index,
                    })
                    .copied()
                    .unwrap_or_default();

                let material = create_material(
                    material,
                    texture_cache,
                    &mut self.textures,
                    key.root_index,
                    sampler_base_index,
                    image_textures,
                );
                let new_index = self.materials.len();
                self.materials.push(material);

                self.material_indices.insert(key, new_index);
                new_index
            }
        }
    }
}

fn create_sampler(sampler: &Sampler) -> gltf::json::texture::Sampler {
    gltf::json::texture::Sampler {
        mag_filter: match sampler.mag_filter {
            crate::FilterMode::Nearest => Some(Valid(gltf::json::texture::MagFilter::Nearest)),
            crate::FilterMode::Linear => Some(Valid(gltf::json::texture::MagFilter::Linear)),
        },
        min_filter: match sampler.mag_filter {
            crate::FilterMode::Nearest => Some(Valid(gltf::json::texture::MinFilter::Nearest)),
            crate::FilterMode::Linear => Some(Valid(gltf::json::texture::MinFilter::Linear)),
        },
        wrap_s: Valid(wrapping_mode(sampler.address_mode_u)),
        wrap_t: Valid(wrapping_mode(sampler.address_mode_v)),
        ..Default::default()
    }
}

fn wrapping_mode(address_mode: AddressMode) -> gltf::json::texture::WrappingMode {
    match address_mode {
        AddressMode::ClampToEdge => gltf::json::texture::WrappingMode::ClampToEdge,
        AddressMode::Repeat => gltf::json::texture::WrappingMode::Repeat,
        AddressMode::MirrorRepeat => gltf::json::texture::WrappingMode::MirroredRepeat,
    }
}

fn create_material(
    material: &crate::Material,
    texture_cache: &mut TextureCache,
    textures: &mut Vec<gltf::json::Texture>,
    root_index: usize,
    sampler_base_index: usize,
    image_textures: &[ImageTexture],
) -> gltf::json::Material {
    let assignments = material.output_assignments(image_textures);

    let albedo_key = albedo_generated_key(material, &assignments, root_index);
    let albedo_index = texture_cache.insert(albedo_key.clone());

    let normal_key = normal_generated_key(material, &assignments, root_index);
    let normal_index = texture_cache.insert(normal_key.clone());

    let metallic_roughness_key =
        metallic_roughness_generated_key(material, &assignments, root_index);
    let metallic_roughness_index = texture_cache.insert(metallic_roughness_key.clone());

    let emissive_key = emissive_generated_key(material, &assignments, root_index);
    let emissive_index = texture_cache.insert(emissive_key.clone());

    gltf::json::Material {
        name: Some(material.name.clone()),
        pbr_metallic_roughness: gltf::json::material::PbrMetallicRoughness {
            base_color_texture: albedo_index.map(|i| {
                let texture_index = add_texture(textures, &albedo_key, i, sampler_base_index);
                texture_info(texture_index, &albedo_key)
            }),
            metallic_roughness_texture: metallic_roughness_index.map(|i| {
                let texture_index =
                    add_texture(textures, &metallic_roughness_key, i, sampler_base_index);

                texture_info(texture_index, &metallic_roughness_key)
            }),
            ..Default::default()
        },
        normal_texture: normal_index.map(|i| {
            let texture_index = add_texture(textures, &normal_key, i, sampler_base_index);

            // TODO: Scale normal maps?
            gltf::json::material::NormalTexture {
                index: gltf::json::Index::new(texture_index),
                scale: 1.0,
                tex_coord: tex_coord(&normal_key),
                extensions: None,
                extras: Default::default(),
            }
        }),
        occlusion_texture: metallic_roughness_index.map(|i| {
            let texture_index =
                add_texture(textures, &metallic_roughness_key, i, sampler_base_index);

            // TODO: Occlusion map scale?
            gltf::json::material::OcclusionTexture {
                // Only the red channel is sampled for the occlusion texture.
                // We can reuse the metallic roughness texture red channel here.
                index: gltf::json::Index::new(texture_index),
                strength: gltf::json::material::StrengthFactor(1.0),
                tex_coord: 0,
                extensions: None,
                extras: Default::default(),
            }
        }),
        emissive_texture: emissive_index.map(|i| texture_info(i, &emissive_key)),
        alpha_mode: if material.alpha_test.is_some() {
            Valid(gltf::json::material::AlphaMode::Mask)
        } else {
            Valid(gltf::json::material::AlphaMode::Opaque)
        },
        alpha_cutoff: material
            .alpha_test
            .as_ref()
            .map(|_| gltf::json::material::AlphaCutoff(0.5)),
        ..Default::default()
    }
}

fn texture_info(texture_index: u32, key: &GeneratedImageKey) -> gltf_json::texture::Info {
    let tex_coord = tex_coord(key);
    let scale = texture_scale(key);

    gltf::json::texture::Info {
        index: gltf::json::Index::new(texture_index),
        tex_coord,
        extensions: texture_transform_ext(scale, tex_coord),
        extras: Default::default(),
    }
}

fn texture_scale(key: &GeneratedImageKey) -> Option<[ordered_float::OrderedFloat<f32>; 2]> {
    // Assume all channels have the same UV attribute and scale.
    match &key.red_index {
        Some(ImageIndex::Image { texcoord_scale, .. }) => *texcoord_scale,
        _ => None,
    }
}

fn tex_coord(key: &GeneratedImageKey) -> u32 {
    // Assume all channels have the same UV attribute and scale.
    match &key.red_index {
        // Match the indices assigned for "TexCoord0" to "TexCoord8" attributes.
        Some(ImageIndex::Image { texcoord_name, .. }) => match texcoord_name.as_str() {
            "vTex0" => 0,
            "vTex1" => 1,
            "vTex2" => 2,
            "vTex3" => 3,
            "vTex4" => 4,
            "vTex5" => 5,
            "vTex6" => 6,
            "vTex7" => 7,
            "vTex8" => 8,
            _ => 0,
        },
        _ => 0,
    }
}

fn texture_transform_ext(
    scale: Option<[ordered_float::OrderedFloat<f32>; 2]>,
    tex_coord: u32,
) -> Option<gltf_json::extensions::texture::Info> {
    // TODO: Don't assume the first UV map?
    scale.map(|[u, v]| gltf::json::extensions::texture::Info {
        texture_transform: Some(gltf::json::extensions::texture::TextureTransform {
            offset: gltf::json::extensions::texture::TextureTransformOffset([0.0; 2]),
            rotation: gltf::json::extensions::texture::TextureTransformRotation(0.0),
            scale: gltf::json::extensions::texture::TextureTransformScale([u.0, v.0]),
            tex_coord: Some(tex_coord),
            extras: None,
        }),
    })
}

fn add_texture(
    textures: &mut Vec<gltf::json::Texture>,
    image_key: &GeneratedImageKey,
    image_index: u32,
    sampler_base_index: usize,
) -> u32 {
    // The channel packing means an image could theoretically require 4 samplers.
    // The samplers are unlikely to differ in practice, so just pick one.
    let sampler_index = image_key.red_index.as_ref().and_then(|i| match i {
        ImageIndex::Image { sampler, .. } => Some(sampler),
        ImageIndex::Value(_) => None,
    });

    let texture_index = textures.len() as u32;
    textures.push(gltf::json::Texture {
        name: None,
        sampler: sampler_index.map(|sampler_index| {
            gltf::json::Index::new((sampler_index + sampler_base_index) as u32)
        }),
        source: gltf::json::Index::new(image_index),
        extensions: None,
        extras: Default::default(),
    });
    texture_index
}
