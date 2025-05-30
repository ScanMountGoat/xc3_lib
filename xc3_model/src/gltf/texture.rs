use crate::{
    material::assignments::{Assignment, AssignmentValue, OutputAssignments, TextureAssignment},
    monolib::ShaderTextures,
    ImageTexture, IndexMapExt,
};
use image_dds::image::{codecs::png::PngEncoder, RgbaImage};
use indexmap::IndexMap;
use ordered_float::OrderedFloat;
use rayon::prelude::*;
use smol_str::SmolStr;

// TODO: This will eventually need to account for parameters and constants.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GeneratedImageKey {
    pub root_index: usize,
    pub red_index: Option<ImageIndex>,
    pub green_index: Option<ImageIndex>,
    pub blue_index: Option<ImageIndex>,
    pub alpha_index: Option<ImageIndex>,
    pub recalculate_normal_z: bool,
    pub invert_green: bool,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ImageIndex {
    Image {
        image_texture: usize,
        // TODO: This shouldn't be keyed as part of the generated images.
        sampler: usize,
        channel: usize,
        texcoord_name: SmolStr,
        texcoord_scale: Option<[OrderedFloat<f32>; 2]>,
    },
    GlobalImage {
        name: SmolStr,
        channel: usize,
        texcoord_name: SmolStr,
        texcoord_scale: Option<[OrderedFloat<f32>; 2]>,
    },
    Value(OrderedFloat<f32>),
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
    pub generated_texture_indices: IndexMap<GeneratedImageKey, usize>,

    shader_images: IndexMap<String, RgbaImage>,
}

impl TextureCache {
    pub fn new<'a>(
        root_textures: impl Iterator<Item = &'a Vec<ImageTexture>>,
        shader_textures: &ShaderTextures,
    ) -> Self {
        // Get the base images used for channel reconstruction.
        let original_images = create_images(root_textures);

        let shader_images = shader_textures
            .textures
            .par_iter()
            .filter_map(|(name, texture)| {
                Some((
                    name.to_string(),
                    texture.as_ref().map(|t| t.to_image().unwrap())?,
                ))
            })
            .collect();

        Self {
            generated_texture_indices: IndexMap::new(),
            original_images,
            shader_images,
        }
    }

    pub fn insert(&mut self, key: GeneratedImageKey) -> Option<u32> {
        // Use a cache to avoid costly image generation if possible.
        // TODO: Find a cleaner way to prevent generating empty images.
        if key.red_index.is_some()
            || key.green_index.is_some()
            || key.blue_index.is_some()
            || key.alpha_index.is_some()
        {
            Some(self.generated_texture_indices.entry_index(key) as u32)
        } else {
            None
        }
    }

    // TODO: Avoid unwrap?
    pub fn generate_png_images(
        &self,
        model_name: &str,
        flip_vertical: bool,
    ) -> Vec<(String, Vec<u8>)> {
        self.generated_texture_indices
            .par_iter()
            .map(|(key, _)| {
                let image = generate_image(
                    key.clone(),
                    &self.original_images,
                    &self.shader_images,
                    flip_vertical,
                );

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
    assignments: &OutputAssignments,
    root_index: usize,
) -> GeneratedImageKey {
    // Assume the first texture is albedo if no assignments are possible.
    let red_index = image_index(material, assignments, assignments.output_assignments[0].x)
        .or_else(|| {
            material.textures.first().map(|t| ImageIndex::Image {
                image_texture: t.image_texture_index,
                sampler: 0,
                channel: 0,
                texcoord_name: SmolStr::default(),
                texcoord_scale: None,
            })
        });
    let green_index = image_index(material, assignments, assignments.output_assignments[0].y)
        .or_else(|| {
            material.textures.first().map(|t| ImageIndex::Image {
                image_texture: t.image_texture_index,
                sampler: 0,
                channel: 1,
                texcoord_name: SmolStr::default(),
                texcoord_scale: None,
            })
        });
    let blue_index = image_index(material, assignments, assignments.output_assignments[0].z)
        .or_else(|| {
            material.textures.first().map(|t| ImageIndex::Image {
                image_texture: t.image_texture_index,
                sampler: 0,
                channel: 2,
                texcoord_name: SmolStr::default(),
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
        ImageIndex::Image {
            image_texture: texture.image_texture_index,
            sampler: texture.sampler_index,
            channel: a.channel_index,
            texcoord_name: SmolStr::default(),
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
    assignments: &OutputAssignments,
    root_index: usize,
) -> GeneratedImageKey {
    let red_index = image_index(material, assignments, assignments.output_assignments[2].x);
    let green_index = image_index(material, assignments, assignments.output_assignments[2].y);

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
    assignments: &OutputAssignments,
    root_index: usize,
) -> GeneratedImageKey {
    // The red channel is unused, we can pack occlusion here.
    let occlusion_index = image_index(material, assignments, assignments.output_assignments[2].z);
    let metalness_index = image_index(material, assignments, assignments.output_assignments[1].x);
    let glossiness_index = image_index(material, assignments, assignments.output_assignments[1].y);

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

pub fn emissive_generated_key(
    material: &crate::Material,
    assignments: &OutputAssignments,
    root_index: usize,
) -> GeneratedImageKey {
    // TODO: Is it correct to assume only toon and hair materials use specular?
    let has_emission = !matches!(assignments.mat_id(), Some(2 | 5));
    if has_emission {
        let red_index = image_index(material, assignments, assignments.output_assignments[5].x);
        let green_index = image_index(material, assignments, assignments.output_assignments[5].y);
        let blue_index = image_index(material, assignments, assignments.output_assignments[5].z);

        GeneratedImageKey {
            root_index,
            red_index,
            green_index,
            blue_index,
            alpha_index: None,
            recalculate_normal_z: false,
            invert_green: false,
        }
    } else {
        GeneratedImageKey {
            root_index,
            red_index: None,
            green_index: None,
            blue_index: None,
            alpha_index: None,
            recalculate_normal_z: false,
            invert_green: false,
        }
    }
}

// TODO: how to make this faster?
fn generate_image(
    key: GeneratedImageKey,
    original_images: &IndexMap<ImageKey, RgbaImage>,
    shader_images: &IndexMap<String, RgbaImage>,
    flip_vertical: bool,
) -> RgbaImage {
    let red_image = find_image_channel(
        original_images,
        shader_images,
        &key.red_index,
        key.root_index,
    );
    let green_image = find_image_channel(
        original_images,
        shader_images,
        &key.green_index,
        key.root_index,
    );
    let blue_image = find_image_channel(
        original_images,
        shader_images,
        &key.blue_index,
        key.root_index,
    );
    let alpha_image = find_image_channel(
        original_images,
        shader_images,
        &key.alpha_index,
        key.root_index,
    );

    // Use the dimensions of the largest image to avoid quality loss.
    // Choose a small default size to avoid crashes on images with only constants.
    let (width, height) = [red_image, green_image, blue_image, alpha_image]
        .iter()
        .filter_map(|i| i.map(|(i, _)| i.dimensions()))
        .max()
        .unwrap_or((4, 4));

    // Start with a fully opaque black image.
    let mut image = RgbaImage::new(width, height);
    for pixel in image.pixels_mut() {
        pixel[3] = 255u8;
    }

    let red_value = find_value(&key.red_index);
    let green_value = find_value(&key.green_index);
    let blue_value = find_value(&key.blue_index);
    let alpha_value = find_value(&key.alpha_index);

    // TODO: optimize assigning multiple channels in order?
    // TODO: cache resizing operations for images?
    assign_channel(&mut image, red_image, red_value, 0);
    assign_channel(&mut image, green_image, green_value, 1);
    assign_channel(&mut image, blue_image, blue_value, 2);
    assign_channel(&mut image, alpha_image, alpha_value, 3);

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

    if flip_vertical {
        image = image_dds::image::imageops::flip_vertical(&image);
    }

    image
}

fn find_image_channel<'a>(
    original_images: &'a IndexMap<ImageKey, RgbaImage>,
    shader_images: &'a IndexMap<String, RgbaImage>,
    index: &Option<ImageIndex>,
    root_index: usize,
) -> Option<(&'a RgbaImage, usize)> {
    index.as_ref().and_then(|index| match index {
        ImageIndex::Image {
            image_texture,
            channel,
            ..
        } => original_images
            .get(&ImageKey {
                root_index,
                image_index: *image_texture,
            })
            .map(|i| (i, *channel)),
        ImageIndex::GlobalImage { name, channel, .. } => {
            shader_images.get(name.as_str()).map(|i| (i, *channel))
        }
        ImageIndex::Value(_) => None,
    })
}

fn find_value(index: &Option<ImageIndex>) -> Option<f32> {
    match index {
        Some(ImageIndex::Value(v)) => Some(**v),
        _ => None,
    }
}

fn assign_channel(
    output: &mut RgbaImage,
    image_channel: Option<(&RgbaImage, usize)>,
    value: Option<f32>,
    output_channel: usize,
) {
    // TODO: Make this an enum instead?
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
    } else if let Some(value) = value {
        for output_pixel in output.pixels_mut() {
            output_pixel[output_channel] = (value * 255.0) as u8;
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
    if let Some(text) = channel_name(&key.red_index) {
        name += &format!("_r{text}");
    }
    if let Some(text) = channel_name(&key.green_index) {
        name += &format!("_g{text}");
    }
    if let Some(text) = channel_name(&key.blue_index) {
        name += &format!("_b{text}");
    }
    if let Some(text) = channel_name(&key.alpha_index) {
        name += &format!("_a{text}");
    }
    // Use PNG since it's lossless and widely supported.
    name + ".png"
}

fn channel_name(index: &Option<ImageIndex>) -> Option<String> {
    // TODO: Include sampler data?
    match index {
        Some(ImageIndex::Image {
            image_texture,
            channel,
            ..
        }) => Some(format!("{image_texture}[{channel}]")),
        Some(ImageIndex::GlobalImage { name, channel, .. }) => Some(format!("{name}[{channel}]")),
        Some(ImageIndex::Value(v)) => Some(v.to_string()),
        None => None,
    }
}

fn image_index(
    material: &crate::Material,
    assignments: &OutputAssignments,
    value: Option<usize>,
) -> Option<ImageIndex> {
    match &assignments.assignments[value?] {
        Assignment::Value(assignment) => assignment_image_index(material, assignment),
        // TODO: Find a better heuristic for determining the base value.
        Assignment::Func { args, .. } => image_index(material, assignments, args.first().copied()),
    }
}

fn assignment_image_index(
    material: &crate::Material,
    assignment: &Option<AssignmentValue>,
) -> Option<ImageIndex> {
    match assignment.as_ref()? {
        AssignmentValue::Texture(texture) => {
            let TextureAssignment { name, channel, .. } = texture;

            let channel_index = "xyzw".find(channel.unwrap()).unwrap();

            // TODO: proper mat2x4 support?
            // TODO: how to get the texcoord name and scale?
            let texcoord_name: SmolStr = "vTex0".into();

            // Find the sampler from the material.
            // Find the texture referenced by this sampler.
            let material_image = material_texture_index(name).and_then(|sampler_index| {
                material
                    .textures
                    .get(sampler_index)
                    .map(|t| ImageIndex::Image {
                        image_texture: t.image_texture_index,
                        sampler: t.sampler_index,
                        channel: channel_index,
                        texcoord_name: texcoord_name.clone(),
                        texcoord_scale: None,
                    })
            });

            // Assume any unrecognized textures reference monolib/shader textures.
            Some(material_image.unwrap_or(ImageIndex::GlobalImage {
                name: name.clone(),
                channel: channel_index,
                texcoord_name,
                texcoord_scale: None,
            }))
        }
        AssignmentValue::Attribute { .. } => None,
        AssignmentValue::Float(v) => Some(ImageIndex::Value(*v)),
    }
}

fn material_texture_index(sampler: &str) -> Option<usize> {
    // Convert names like "s3" to index 3.
    // Materials always use this naming convention in the shader.
    // Xenoblade 1 DE uses up to 14 material samplers.
    sampler.strip_prefix('s')?.parse().ok()
}

pub fn create_images<'a>(
    root_textures: impl Iterator<Item = &'a Vec<ImageTexture>>,
) -> IndexMap<ImageKey, RgbaImage> {
    let mut png_images = IndexMap::new();
    for (root_index, image_textures) in root_textures.into_iter().enumerate() {
        // Decode images in parallel to boost performance.
        png_images.par_extend(image_textures.par_iter().enumerate().map(|(i, texture)| {
            // Convert to PNG since DDS is not well supported.
            let image = texture.to_image().unwrap();
            let key = ImageKey {
                root_index,
                image_index: i,
            };
            (key, image)
        }));
    }
    png_images
}
