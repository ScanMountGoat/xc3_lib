use std::collections::BTreeMap;

use crate::ModelRoot;
use rayon::prelude::*;

// TODO: This will eventually need to account for parameters and constants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct GeneratedImageKey {
    root_index: usize,
    red_index: Option<(usize, usize)>,
    green_index: Option<(usize, usize)>,
    blue_index: Option<(usize, usize)>,
    alpha_index: Option<(usize, usize)>,
    recalculate_normal_z: bool,
    invert_green: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ImageKey {
    root_index: usize,
    image_index: usize,
}

#[derive(Default)]
pub struct TextureCache {
    pub textures: Vec<gltf::json::Texture>,
    pub generated_images: BTreeMap<GeneratedImageKey, image::RgbaImage>,
    pub generated_texture_indices: BTreeMap<GeneratedImageKey, u32>,
    pub original_images: BTreeMap<ImageKey, image::RgbaImage>,
}

impl TextureCache {
    pub fn new(roots: &[ModelRoot]) -> Self {
        // Get the base images used for channel reconstruction.
        let original_images = create_images(roots);

        Self {
            textures: Vec::new(),
            generated_images: BTreeMap::new(),
            generated_texture_indices: BTreeMap::new(),
            original_images,
        }
    }

    pub fn insert(&mut self, key: GeneratedImageKey) -> Option<u32> {
        // Use a cache to avoid costly channel reconstructions if possible.
        self.generated_texture_indices
            .get(&key)
            .copied()
            .or_else(|| {
                // Only create an image if it has at least one input.
                generate_image(key, &self.original_images).map(|image| {
                    let texture_index = self.textures.len() as u32;
                    self.textures.push(gltf::json::Texture {
                        name: None,
                        sampler: None,
                        source: gltf::json::Index::new(texture_index),
                        extensions: None,
                        extras: Default::default(),
                    });
                    self.generated_images.insert(key, image);
                    self.generated_texture_indices.insert(key, texture_index);

                    texture_index
                })
            })
    }
}

// TODO: Create consts for the gbuffer texture indices?
pub fn albedo_generated_key(
    material: &crate::Material,
    root_index: usize,
) -> Option<GeneratedImageKey> {
    let red_index = texture_channel_index(material, 0, 'x');
    let green_index = texture_channel_index(material, 0, 'y');
    let blue_index = texture_channel_index(material, 0, 'z');
    let alpha_index = texture_channel_index(material, 0, 'w');

    Some(GeneratedImageKey {
        root_index,
        red_index,
        green_index,
        blue_index,
        alpha_index,
        recalculate_normal_z: false,
        invert_green: false,
    })
}

pub fn normal_generated_key(
    material: &crate::Material,
    root_index: usize,
) -> Option<GeneratedImageKey> {
    let red_index = texture_channel_index(material, 2, 'x');
    let green_index = texture_channel_index(material, 2, 'y');

    Some(GeneratedImageKey {
        root_index,
        red_index,
        green_index,
        blue_index: None,
        alpha_index: None,
        recalculate_normal_z: true,
        invert_green: false,
    })
}

pub fn metallic_roughness_generated_key(
    material: &crate::Material,
    root_index: usize,
) -> Option<GeneratedImageKey> {
    // The red channel is unused, we can pack occlusion here.
    let occlusion_index = texture_channel_index(material, 2, 'z');
    let metalness_index = texture_channel_index(material, 1, 'x');
    let glossiness_index = texture_channel_index(material, 1, 'y');

    // Invert the glossiness since glTF uses roughness.
    Some(GeneratedImageKey {
        root_index,
        red_index: occlusion_index,
        green_index: glossiness_index,
        blue_index: metalness_index,
        alpha_index: None,
        recalculate_normal_z: false,
        invert_green: true,
    })
}

fn generate_image(
    key: GeneratedImageKey,
    original_images: &BTreeMap<ImageKey, image::RgbaImage>,
) -> Option<image::RgbaImage> {
    let find_image_channel = |index: Option<(usize, usize)>| {
        index.and_then(|(image_index, channel)| {
            original_images
                .get(&ImageKey {
                    root_index: key.root_index,
                    image_index,
                })
                .map(|image| (image, channel))
        })
    };

    let red_image = find_image_channel(key.red_index);
    let green_image = find_image_channel(key.green_index);
    let blue_image = find_image_channel(key.blue_index);
    let alpha_image = find_image_channel(key.alpha_index);

    // Use the dimensions of the largest image to avoid quality loss.
    let (width, height) = [red_image, green_image, blue_image, alpha_image]
        .iter()
        .filter_map(|i| i.map(|(i, _)| i.dimensions()))
        .max()?;

    // Start with a fully opaque black image.
    let mut image = image::RgbaImage::new(width, height);
    for pixel in image.pixels_mut() {
        pixel[3] = 255u8;
    }

    // TODO: These images may need to be resized.
    assign_channel(&mut image, red_image, 0);
    assign_channel(&mut image, green_image, 1);
    assign_channel(&mut image, blue_image, 2);
    assign_channel(&mut image, alpha_image, 3);

    if key.recalculate_normal_z {
        // Reconstruct the normal map Z channel.
        for pixel in image.pixels_mut() {
            // x^y + y^2 + z^2 = 1 for unit vectors.
            let x = (pixel[0] as f32 / 255.0) * 2.0 - 1.0;
            let y = (pixel[1] as f32 / 255.0) * 2.0 - 1.0;
            let z = 1.0 - x * x - y * y;
            pixel[2] = (z * 255.0) as u8;
        }
    }

    if key.invert_green {
        // Used to convert glossiness to roughness.
        for pixel in image.pixels_mut() {
            pixel[1] = 255u8 - pixel[1];
        }
    }

    Some(image)
}

fn assign_channel(
    output: &mut image::RgbaImage,
    image_channel: Option<(&image::RgbaImage, usize)>,
    output_channel: usize,
) {
    if let Some((image, channel)) = image_channel {
        for (pixel, channel_pixel) in output.pixels_mut().zip(image.pixels()) {
            pixel[output_channel] = channel_pixel[channel];
        }
    }
}

pub fn image_name(key: &GeneratedImageKey) -> String {
    let mut name = format!("root{}", key.root_index);
    if let Some((i, c)) = key.red_index {
        name += &format!("_r{i}[{c}]");
    }
    if let Some((i, c)) = key.green_index {
        name += &format!("_g{i}[{c}]");
    }
    if let Some((i, c)) = key.blue_index {
        name += &format!("_b{i}[{c}]");
    }
    if let Some((i, c)) = key.alpha_index {
        name += &format!("_a{i}[{c}]");
    }
    name + ".png"
}

fn texture_channel_index(
    material: &crate::Material,
    gbuffer_index: usize,
    channel: char,
) -> Option<(usize, usize)> {
    // Find the sampler from the material.
    let (sampler_index, channel_index) = material
        .shader
        .as_ref()?
        .sampler_channel_index(gbuffer_index, channel)?;

    // Find the texture referenced by this sampler.
    material
        .textures
        .get(sampler_index as usize)
        .map(|t| (t.image_texture_index, channel_index as usize))
}

pub fn create_images(roots: &[ModelRoot]) -> BTreeMap<ImageKey, image::RgbaImage> {
    // TODO: Is it worth giving images their in game names?
    let mut png_images = BTreeMap::new();
    for (root_index, root) in roots.iter().enumerate() {
        // Decode images in parallel to boost performance.
        png_images.par_extend(
            root.image_textures
                .par_iter()
                .enumerate()
                .map(|(i, texture)| {
                    // Convert to PNG since DDS is not well supported.
                    let image = texture.to_image().unwrap();
                    let key = ImageKey {
                        root_index,
                        image_index: i,
                    };
                    (key, image)
                }),
        );
    }
    png_images
}