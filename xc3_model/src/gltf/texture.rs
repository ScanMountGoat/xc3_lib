use crate::{ChannelAssignment, GBufferAssignments, ModelRoot};
use image_dds::image::{codecs::png::PngEncoder, RgbaImage};
use indexmap::IndexMap;
use ordered_float::OrderedFloat;
use rayon::prelude::*;

// TODO: This will eventually need to account for parameters and constants.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GeneratedImageKey {
    pub root_index: usize,
    pub red_index: Option<ImageIndex>,
    pub green_index: Option<ImageIndex>,
    pub blue_index: Option<ImageIndex>,
    pub alpha_index: Option<ImageIndex>,
    pub recalculate_normal_z: bool,
    pub invert_green: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ImageIndex {
    pub image_texture: usize,
    // TODO: This shouldn't be keyed as part of the generated images.
    pub sampler: usize,
    pub channel: usize,
    pub texcoord_scale: Option<[OrderedFloat<f32>; 2]>,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ImageKey {
    root_index: usize,
    image_index: usize,
}

// TODO: Share this functionality with map texture loading?
#[derive(Default)]
pub struct TextureCache {
    original_images: IndexMap<ImageKey, RgbaImage>,
    // Use a map that preserves insertion order to get consistent ordering.
    pub generated_texture_indices: IndexMap<GeneratedImageKey, u32>,
}

impl TextureCache {
    pub fn new(roots: &[ModelRoot]) -> Self {
        // Get the base images used for channel reconstruction.
        let original_images = create_images(roots);

        Self {
            generated_texture_indices: IndexMap::new(),
            original_images,
        }
    }

    pub fn insert(&mut self, key: GeneratedImageKey) -> Option<u32> {
        // Use a cache to avoid costly image generation if possible.
        let new_index = self.generated_texture_indices.len() as u32;

        // TODO: Find a cleaner way to prevent generating empty images.
        if key.red_index.is_some()
            || key.green_index.is_some()
            || key.blue_index.is_some()
            || key.alpha_index.is_some()
        {
            Some(
                *self
                    .generated_texture_indices
                    .entry(key)
                    .or_insert(new_index),
            )
        } else {
            None
        }
    }

    // TODO: Avoid unwrap?
    pub fn generate_png_images(&self, model_name: &str) -> Vec<(String, Vec<u8>)> {
        self.generated_texture_indices
            .par_iter()
            .map(|(key, _)| {
                // TODO: Why does this panic?
                let image = generate_image(*key, &self.original_images).unwrap();

                // Compress ahead of time to reduce memory usage.
                // The final results will need to be saved as PNG anyway.
                let mut png_bytes = Vec::new();
                let encoder = PngEncoder::new(&mut png_bytes);
                image.write_with_encoder(encoder).unwrap();

                let name = image_name(key, model_name);
                (name, png_bytes)
            })
            .collect()
    }
}

// TODO: Create consts for the gbuffer texture indices?
pub fn albedo_generated_key(
    material: &crate::Material,
    assignments: &GBufferAssignments,
    root_index: usize,
) -> GeneratedImageKey {
    // Assume the first texture is albedo if no assignments are possible.
    let red_index = image_index(material, assignments.assignments[0].x.as_ref()).or_else(|| {
        material.textures.first().map(|t| ImageIndex {
            image_texture: t.image_texture_index,
            sampler: 0,
            channel: 0,
            texcoord_scale: None,
        })
    });
    let green_index = image_index(material, assignments.assignments[0].y.as_ref()).or_else(|| {
        material.textures.first().map(|t| ImageIndex {
            image_texture: t.image_texture_index,
            sampler: 0,
            channel: 1,
            texcoord_scale: None,
        })
    });
    let blue_index = image_index(material, assignments.assignments[0].z.as_ref()).or_else(|| {
        material.textures.first().map(|t| ImageIndex {
            image_texture: t.image_texture_index,
            sampler: 0,
            channel: 2,
            texcoord_scale: None,
        })
    });

    // Some materials have alpha testing in a separate depth prepass.
    // glTF expects the alpha to be part of the main albedo texture.
    // We'll cheat a little here and convert the mask texture to albedo alpha.
    // If no alpha test texture is assigned, the PNG will use an alpha of 1.0.
    // This avoids issues with applications that always treat alpha as transparency.
    let alpha_index = material.alpha_test.as_ref().map(|a| {
        let texture = &material.textures[a.texture_index];
        ImageIndex {
            image_texture: texture.image_texture_index,
            sampler: texture.sampler_index,
            channel: a.channel_index,
            texcoord_scale: None,
        }
    });

    // TODO: Default to the first texture for albedo if no database entry?
    GeneratedImageKey {
        root_index,
        red_index,
        green_index,
        blue_index,
        alpha_index,
        recalculate_normal_z: false,
        invert_green: false,
    }
}

pub fn normal_generated_key(
    material: &crate::Material,
    assignments: &GBufferAssignments,
    root_index: usize,
) -> GeneratedImageKey {
    let red_index = image_index(material, assignments.assignments[2].x.as_ref());
    let green_index = image_index(material, assignments.assignments[2].y.as_ref());

    GeneratedImageKey {
        root_index,
        red_index,
        green_index,
        blue_index: None,
        alpha_index: None,
        recalculate_normal_z: true,
        invert_green: false,
    }
}

pub fn metallic_roughness_generated_key(
    material: &crate::Material,
    assignments: &GBufferAssignments,
    root_index: usize,
) -> GeneratedImageKey {
    // The red channel is unused, we can pack occlusion here.
    let occlusion_index = image_index(material, assignments.assignments[2].z.as_ref());
    let metalness_index = image_index(material, assignments.assignments[1].x.as_ref());
    let glossiness_index = image_index(material, assignments.assignments[1].y.as_ref());

    // Invert the glossiness since glTF uses roughness.
    GeneratedImageKey {
        root_index,
        red_index: occlusion_index,
        green_index: glossiness_index,
        blue_index: metalness_index,
        alpha_index: None,
        recalculate_normal_z: false,
        invert_green: true,
    }
}

// TODO: how to make this faster?
fn generate_image(
    key: GeneratedImageKey,
    original_images: &IndexMap<ImageKey, RgbaImage>,
) -> Option<RgbaImage> {
    let find_image_channel = |index: Option<ImageIndex>| {
        index.and_then(|index| {
            original_images
                .get(&ImageKey {
                    root_index: key.root_index,
                    image_index: index.image_texture,
                })
                .map(|image| (image, index.channel))
        })
    };

    let red_image = find_image_channel(key.red_index);
    let green_image = find_image_channel(key.green_index);
    let blue_image = find_image_channel(key.blue_index);
    let alpha_image = find_image_channel(key.alpha_index);

    // Use the dimensions of the largest image to avoid quality loss.
    // Return None if no images are assigned.
    let (width, height) = [red_image, green_image, blue_image, alpha_image]
        .iter()
        .filter_map(|i| i.map(|(i, _)| i.dimensions()))
        .max()?;

    // Start with a fully opaque black image.
    let mut image = RgbaImage::new(width, height);
    for pixel in image.pixels_mut() {
        pixel[3] = 255u8;
    }

    // TODO: optimize assigning multiple channels in order?
    // TODO: cache resizing operations for images?
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
    output: &mut RgbaImage,
    image_channel: Option<(&RgbaImage, usize)>,
    output_channel: usize,
) {
    if let Some((input, input_channel)) = image_channel {
        // Ensure the input and output images have the same dimensions.
        // TODO: Is it worth caching this operation?
        // TODO: Avoid resizing the same image for each channel?
        if input.dimensions() != output.dimensions() {
            let resized = image_dds::image::imageops::resize(
                input,
                output.width(),
                output.height(),
                image_dds::image::imageops::FilterType::Triangle,
            );
            assign_pixels(output, &resized, output_channel, input_channel);
        } else {
            assign_pixels(output, input, output_channel, input_channel);
        }
    }
}

fn assign_pixels(
    output: &mut RgbaImage,
    input: &RgbaImage,
    output_channel: usize,
    input_channel: usize,
) {
    for (output_pixel, input_pixel) in output.pixels_mut().zip(input.pixels()) {
        output_pixel[output_channel] = input_pixel[input_channel];
    }
}

pub fn image_name(key: &GeneratedImageKey, model_name: &str) -> String {
    let mut name = format!("{model_name}_root{}", key.root_index);
    if let Some(ImageIndex {
        image_texture: image_texture_index,
        channel: channel_index,
        ..
    }) = key.red_index
    {
        name += &format!("_r{image_texture_index}[{channel_index}]");
    }
    if let Some(ImageIndex {
        image_texture: image_texture_index,
        channel: channel_index,
        ..
    }) = key.green_index
    {
        name += &format!("_g{image_texture_index}[{channel_index}]");
    }
    if let Some(ImageIndex {
        image_texture: image_texture_index,
        channel: channel_index,
        ..
    }) = key.blue_index
    {
        name += &format!("_b{image_texture_index}[{channel_index}]");
    }
    if let Some(ImageIndex {
        image_texture: image_texture_index,
        channel: channel_index,
        ..
    }) = key.alpha_index
    {
        name += &format!("_a{image_texture_index}[{channel_index}]");
    }
    // Use PNG since it's lossless and widely supported.
    name + ".png"
}

fn image_index(
    material: &crate::Material,
    assignment: Option<&ChannelAssignment>,
) -> Option<ImageIndex> {
    // Find the sampler from the material.
    // TODO: scale?
    match assignment? {
        crate::ChannelAssignment::Texture {
            name,
            channel_index,
            texcoord_scale,
        } => {
            let sampler_index = material_texture_index(name)?;
            // Find the texture referenced by this sampler.
            material.textures.get(sampler_index).map(|t| ImageIndex {
                image_texture: t.image_texture_index,
                sampler: t.sampler_index,
                channel: *channel_index,
                texcoord_scale: texcoord_scale.map(|(u, v)| [u.into(), v.into()]),
            })
        }
        // TODO: Also handle constant values?
        crate::ChannelAssignment::Value(_) => None,
    }
}

fn material_texture_index(sampler: &str) -> Option<usize> {
    match sampler {
        "s0" => Some(0),
        "s1" => Some(1),
        "s2" => Some(2),
        "s3" => Some(3),
        "s4" => Some(4),
        "s5" => Some(5),
        "s6" => Some(6),
        "s7" => Some(7),
        "s8" => Some(8),
        "s9" => Some(9),
        // TODO: How to handle this case?
        _ => None,
    }
}

pub fn create_images(roots: &[ModelRoot]) -> IndexMap<ImageKey, RgbaImage> {
    let mut png_images = IndexMap::new();
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
