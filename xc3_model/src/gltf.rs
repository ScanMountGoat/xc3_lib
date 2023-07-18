use std::{
    collections::BTreeMap,
    io::{Cursor, Seek, Write},
    path::Path,
};

use crate::{vertex::AttributeData, ModelRoot};
use binrw::BinWrite;
use glam::{Mat4, Vec2, Vec3, Vec4, Vec4Swizzles};
use gltf::json::validation::Checked::Valid;
use rayon::prelude::*;

type GltfAttributes = BTreeMap<
    gltf::json::validation::Checked<gltf::Semantic>,
    gltf::json::Index<gltf::json::Accessor>,
>;

// gltf stores flat lists of attributes and accessors at the root level.
// Create mappings to properly differentiate models and groups.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct BufferKey {
    root_index: usize,
    group_index: usize,
    model_index: usize,
    buffer_index: usize,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct ImageKey {
    root_index: usize,
    image_index: usize,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct MaterialKey {
    root_index: usize,
    group_index: usize,
    material_index: usize,
}

// TODO: This will eventually need to account for parameters and constants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct GeneratedImageKey {
    root_index: usize,
    red_index: Option<(usize, usize)>,
    green_index: Option<(usize, usize)>,
    blue_index: Option<(usize, usize)>,
    alpha_index: Option<(usize, usize)>,
    recalculate_normal_z: bool,
    invert_green: bool,
}

#[derive(Default)]
struct TextureCache {
    textures: Vec<gltf::json::Texture>,
    generated_images: BTreeMap<GeneratedImageKey, image::RgbaImage>,
    generated_texture_indices: BTreeMap<GeneratedImageKey, u32>,
    original_images: BTreeMap<ImageKey, image::RgbaImage>,
}

impl TextureCache {
    fn new(roots: &[ModelRoot]) -> Self {
        // Get the base images used for channel reconstruction.
        let original_images = create_images(roots);

        Self {
            textures: Vec::new(),
            generated_images: BTreeMap::new(),
            generated_texture_indices: BTreeMap::new(),
            original_images,
        }
    }

    fn insert(&mut self, key: GeneratedImageKey) -> Option<u32> {
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

// Combined vertex data for a gltf buffer.
struct Buffers {
    buffer: gltf::json::Buffer,
    buffer_bytes: Vec<u8>,
    buffer_views: Vec<gltf::json::buffer::View>,
    accessors: Vec<gltf::json::Accessor>,
    // Map group and model specific indices to flattened indices.
    vertex_buffer_attributes: BTreeMap<BufferKey, GltfAttributes>,
    index_buffer_accessors: BTreeMap<BufferKey, usize>,
}

// TODO: Clean this up.
// TODO: Make returning and writing the data separate functions.
pub fn export_gltf<P: AsRef<Path>>(path: P, roots: &[ModelRoot]) {
    let mut texture_cache = TextureCache::new(roots);

    let mut materials = Vec::new();
    let mut material_indices = BTreeMap::new();

    for (root_index, root) in roots.iter().enumerate() {
        for (group_index, group) in root.groups.iter().enumerate() {
            for (material_index, material) in group.materials.iter().enumerate() {
                let albedo_key = albedo_generated_key(material, root_index);
                let albedo_index = albedo_key.and_then(|key| texture_cache.insert(key));

                let normal_key = normal_generated_key(material, root_index);
                let normal_index = normal_key.and_then(|key| texture_cache.insert(key));

                let metallic_roughness_key = metallic_roughness_generated_key(material, root_index);
                let metallic_roughness_index =
                    metallic_roughness_key.and_then(|key| texture_cache.insert(key));

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
                        material_index,
                    },
                    material_flattened_index,
                );
            }
        }
    }

    let model_name = path
        .as_ref()
        .with_extension("")
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string();

    let buffer_name = format!("{model_name}.buffer0.bin");

    let Buffers {
        mut buffer,
        mut buffer_bytes,
        mut buffer_views,
        mut accessors,
        vertex_buffer_attributes,
        index_buffer_accessors,
    } = create_buffers(roots, buffer_name.clone());

    // TODO: select by LOD and skip outline meshes?
    let mut meshes = Vec::new();
    let mut nodes = Vec::new();
    let mut scene_nodes = Vec::new();
    let mut skins = Vec::new();

    for (root_index, root) in roots.iter().enumerate() {
        for (group_index, group) in root.groups.iter().enumerate() {
            let skin_index = create_skin(
                group,
                &mut nodes,
                &mut scene_nodes,
                &mut skins,
                &mut buffer_bytes,
                &mut buffer_views,
                &mut accessors,
            );

            let mut group_children = Vec::new();

            for (model_index, model) in group.models.iter().enumerate() {
                let mut children = Vec::new();

                for mesh in &model.meshes {
                    let attributes_key = BufferKey {
                        root_index,
                        group_index,
                        model_index,
                        buffer_index: mesh.vertex_buffer_index,
                    };
                    let attributes = vertex_buffer_attributes
                        .get(&attributes_key)
                        .unwrap()
                        .clone();

                    let indices_key = BufferKey {
                        root_index,

                        group_index,
                        model_index,
                        buffer_index: mesh.index_buffer_index,
                    };
                    let index_accessor = *index_buffer_accessors.get(&indices_key).unwrap() as u32;

                    let material_index = material_indices
                        .get(&MaterialKey {
                            root_index,
                            group_index,
                            material_index: mesh.material_index,
                        })
                        .unwrap();

                    let primitive = gltf::json::mesh::Primitive {
                        // TODO: Store this with the buffers?
                        attributes,
                        extensions: Default::default(),
                        extras: Default::default(),
                        indices: Some(gltf::json::Index::new(index_accessor)),
                        material: Some(gltf::json::Index::new(*material_index as u32)),
                        mode: Valid(gltf::json::mesh::Mode::Triangles),
                        targets: None,
                    };

                    // Assign one primitive per mesh to create distinct objects in applications.
                    // In game meshes aren't named, so just use the material name.
                    let material_name = materials[*material_index].name.clone();

                    let mesh = gltf::json::Mesh {
                        extensions: Default::default(),
                        extras: Default::default(),
                        name: material_name,
                        primitives: vec![primitive],
                        weights: None,
                    };
                    let mesh_index = meshes.len() as u32;
                    meshes.push(mesh);

                    // Instancing is applied at the model level.
                    // Instance meshes instead so each node has only one parent.
                    // TODO: Use None instead of a single instance transform?
                    for instance in &model.instances {
                        let mesh_node = gltf::json::Node {
                            camera: None,
                            children: None,
                            extensions: Default::default(),
                            extras: Default::default(),
                            matrix: if *instance == Mat4::IDENTITY {
                                None
                            } else {
                                Some(instance.to_cols_array())
                            },
                            mesh: Some(gltf::json::Index::new(mesh_index)),
                            name: None,
                            rotation: None,
                            scale: None,
                            translation: None,
                            skin: skin_index.map(|i| gltf::json::Index::new(i as u32)),
                            weights: None,
                        };
                        let child_index = nodes.len() as u32;
                        nodes.push(mesh_node);

                        children.push(gltf::json::Index::new(child_index))
                    }
                }

                let model_node = gltf::json::Node {
                    camera: None,
                    children: Some(children.clone()),
                    extensions: Default::default(),
                    extras: Default::default(),
                    matrix: None,
                    mesh: None,
                    name: None,
                    rotation: None,
                    scale: None,
                    translation: None,
                    skin: None,
                    weights: None,
                };
                let model_node_index = nodes.len() as u32;
                nodes.push(model_node);

                group_children.push(gltf::json::Index::new(model_node_index));
            }

            let group_node_index = nodes.len() as u32;

            let group_node = gltf::json::Node {
                camera: None,
                children: Some(group_children),
                extensions: Default::default(),
                extras: Default::default(),
                matrix: None,
                mesh: None,
                name: None,
                rotation: None,
                scale: None,
                translation: None,
                skin: None,
                weights: None,
            };
            nodes.push(group_node);

            // Only include root nodes.
            scene_nodes.push(gltf::json::Index::new(group_node_index));
        }
    }

    // Update the length in case we added any skinning data.
    // TODO: Find a cleaner way to do this.
    buffer.byte_length = buffer_bytes.len() as u32;

    // The texture assume the images are in ascending order by index.
    // The sorted order of the keys may not match this order.
    // TODO: Find a faster way to do this.
    let mut images = Vec::new();
    for i in 0..texture_cache.generated_texture_indices.len() {
        for (key, index) in &texture_cache.generated_texture_indices {
            if *index as usize == i {
                images.push(gltf::json::Image {
                    buffer_view: None,
                    mime_type: None,
                    name: None,
                    uri: Some(image_name(key)),
                    extensions: None,
                    extras: Default::default(),
                });
            }
        }
    }

    let root = gltf::json::Root {
        accessors,
        buffers: vec![buffer],
        buffer_views,
        meshes,
        nodes,
        scenes: vec![gltf::json::Scene {
            extensions: Default::default(),
            extras: Default::default(),
            name: None,
            nodes: scene_nodes,
        }],
        materials,
        textures: texture_cache.textures,
        images,
        skins,
        ..Default::default()
    };

    let writer = std::fs::File::create(path.as_ref()).unwrap();
    gltf::json::serialize::to_writer_pretty(writer, &root).unwrap();

    std::fs::write(path.as_ref().with_file_name(buffer_name), buffer_bytes).unwrap();

    let path = path.as_ref();
    // Encode and save images in parallel to boost performance.
    texture_cache
        .generated_images
        .par_iter()
        .for_each(|(key, image)| {
            let output = path.with_file_name(image_name(key));
            image.save(output).unwrap();
        });
}

fn create_skin(
    group: &crate::ModelGroup,
    nodes: &mut Vec<gltf::json::Node>,
    scene_nodes: &mut Vec<gltf::json::Index<gltf::json::Node>>,
    skins: &mut Vec<gltf::json::Skin>,
    buffer_bytes: &mut Vec<u8>,
    buffer_views: &mut Vec<gltf::json::buffer::View>,
    accessors: &mut Vec<gltf::json::Accessor>,
) -> Option<usize> {
    group.skeleton.as_ref().map(|skeleton| {
        let bone_start_index = nodes.len() as u32;
        for (i, bone) in skeleton.bones.iter().enumerate() {
            let children = find_children(skeleton, i);

            let joint_node = gltf::json::Node {
                camera: None,
                children: if !children.is_empty() {
                    Some(children)
                } else {
                    None
                },
                extensions: Default::default(),
                extras: Default::default(),
                matrix: Some(bone.transform.to_cols_array()),
                mesh: None,
                name: Some(bone.name.clone()),
                rotation: None,
                scale: None,
                translation: None,
                skin: None,
                weights: None,
            };
            // Joint nodes must belong to the scene.
            let joint_node_index = nodes.len() as u32;
            nodes.push(joint_node);
            scene_nodes.push(gltf::json::Index::new(joint_node_index));
        }

        // TODO: Add this to skeleton.rs?
        let inverse_bind_matrices: Vec<_> = skeleton
            .world_transforms()
            .iter()
            .map(|t| t.inverse())
            .collect();

        // TODO: Create functions for creating views and attributes?
        // TODO: Create a buffer type with add_data method?
        let matrix_bytes = write_bytes(&inverse_bind_matrices);
        let view = gltf::json::buffer::View {
            buffer: gltf::json::Index::new(0),
            byte_length: matrix_bytes.len() as u32,
            byte_offset: Some(buffer_bytes.len() as u32),
            byte_stride: None,
            extensions: Default::default(),
            extras: Default::default(),
            name: None,
            target: None,
        };
        buffer_bytes.extend_from_slice(&matrix_bytes);

        let accessor = gltf::json::Accessor {
            buffer_view: Some(gltf::json::Index::new(buffer_views.len() as u32)),
            byte_offset: 0,
            count: inverse_bind_matrices.len() as u32,
            component_type: Valid(gltf::json::accessor::GenericComponentType(
                gltf::json::accessor::ComponentType::F32,
            )),
            extensions: Default::default(),
            extras: Default::default(),
            type_: Valid(gltf::json::accessor::Type::Mat4),
            min: None,
            max: None,
            name: None,
            normalized: false,
            sparse: None,
        };
        let accessor_index = accessors.len() as u32;
        buffer_views.push(view);
        accessors.push(accessor);

        // TODO: Multiple roots for skeleton?
        // TODO: inverse_bind_matrices accessor?
        let skin = gltf::json::Skin {
            extensions: Default::default(),
            extras: Default::default(),
            inverse_bind_matrices: Some(gltf::json::Index::new(accessor_index)),
            joints: (bone_start_index..bone_start_index + skeleton.bones.len() as u32)
                .map(|i| gltf::json::Index::new(i))
                .collect(),
            name: None,
            skeleton: None,
        };
        let skin_index = skins.len();
        skins.push(skin);
        skin_index
    })
}

fn find_children(
    skeleton: &crate::skeleton::Skeleton,
    bone_index: usize,
) -> Vec<gltf::json::Index<gltf::json::Node>> {
    // TODO: is is worth optimizing this lookup?
    skeleton
        .bones
        .iter()
        .enumerate()
        .filter_map(|(child_index, b)| {
            if b.parent_index == Some(bone_index) {
                Some(gltf::json::Index::new(child_index as u32))
            } else {
                None
            }
        })
        .collect()
}

fn create_images(roots: &[ModelRoot]) -> BTreeMap<ImageKey, image::RgbaImage> {
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
                    let dds = texture.to_dds().unwrap();
                    let image = image_dds::image_from_dds(&dds, 0).unwrap();
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
        ..Default::default()
    }
}

// TODO: Create consts for the gbuffer texture indices?
fn albedo_generated_key(
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

fn normal_generated_key(
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

fn metallic_roughness_generated_key(
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

fn image_name(key: &GeneratedImageKey) -> String {
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
        .material_channel_assignment(gbuffer_index, channel)?;

    // Find the texture referenced by this sampler.
    material
        .textures
        .get(sampler_index as usize)
        .map(|t| (t.image_texture_index, channel_index as usize))
}

fn create_buffers(roots: &[ModelRoot], buffer_name: String) -> Buffers {
    let mut buffer_bytes = Vec::new();
    let mut buffer_views = Vec::new();
    let mut accessors = Vec::new();
    let mut vertex_buffer_attributes = BTreeMap::new();
    let mut index_buffer_accessors = BTreeMap::new();

    for (root_index, root) in roots.iter().enumerate() {
        for (group_index, group) in root.groups.iter().enumerate() {
            for (model_index, model) in group.models.iter().enumerate() {
                // TODO: Handle the weight buffers separately?

                add_vertex_buffers(
                    model,
                    group.skeleton.as_ref(),
                    root_index,
                    group_index,
                    model_index,
                    &mut buffer_bytes,
                    &mut buffer_views,
                    &mut accessors,
                    &mut vertex_buffer_attributes,
                );

                // Place indices after the vertices to use a single buffer.
                // TODO: Alignment?
                add_index_buffers(
                    model,
                    root_index,
                    group_index,
                    model_index,
                    &mut buffer_bytes,
                    &mut buffer_views,
                    &mut index_buffer_accessors,
                    &mut accessors,
                );
            }
        }
    }

    let buffer = gltf::json::Buffer {
        byte_length: buffer_bytes.len() as u32,
        extensions: Default::default(),
        extras: Default::default(),
        name: None,
        uri: Some(buffer_name),
    };

    Buffers {
        buffer,
        buffer_bytes,
        buffer_views,
        accessors,
        vertex_buffer_attributes,
        index_buffer_accessors,
    }
}

fn add_index_buffers(
    model: &crate::Model,
    root_index: usize,
    group_index: usize,
    model_index: usize,
    buffer_bytes: &mut Vec<u8>,
    buffer_views: &mut Vec<gltf::json::buffer::View>,
    index_buffer_accessors: &mut BTreeMap<BufferKey, usize>,
    accessors: &mut Vec<gltf::json::Accessor>,
) {
    for (i, index_buffer) in model.index_buffers.iter().enumerate() {
        let index_bytes = write_bytes(&index_buffer.indices);

        // Assume everything uses the same buffer for now.
        let view = gltf::json::buffer::View {
            buffer: gltf::json::Index::new(0),
            byte_length: index_bytes.len() as u32,
            byte_offset: Some(buffer_bytes.len() as u32),
            byte_stride: None,
            extensions: Default::default(),
            extras: Default::default(),
            name: None,
            target: Some(Valid(gltf::json::buffer::Target::ElementArrayBuffer)),
        };

        let indices = gltf::json::Accessor {
            buffer_view: Some(gltf::json::Index::new(buffer_views.len() as u32)),
            byte_offset: 0,
            count: index_buffer.indices.len() as u32,
            component_type: Valid(gltf::json::accessor::GenericComponentType(
                gltf::json::accessor::ComponentType::U16,
            )),
            extensions: Default::default(),
            extras: Default::default(),
            type_: Valid(gltf::json::accessor::Type::Scalar),
            min: None,
            max: None,
            name: None,
            normalized: false,
            sparse: None,
        };
        index_buffer_accessors.insert(
            BufferKey {
                root_index,
                group_index,
                model_index,
                buffer_index: i,
            },
            accessors.len(),
        );

        accessors.push(indices);
        buffer_views.push(view);
        buffer_bytes.extend_from_slice(&index_bytes);
    }
}

fn add_vertex_buffers(
    model: &crate::Model,
    skeleton: Option<&crate::skeleton::Skeleton>,
    root_index: usize,
    group_index: usize,
    model_index: usize,
    buffer_bytes: &mut Vec<u8>,
    buffer_views: &mut Vec<gltf::json::buffer::View>,
    accessors: &mut Vec<gltf::json::Accessor>,
    vertex_buffer_attributes: &mut BTreeMap<BufferKey, GltfAttributes>,
) {
    // TODO: Separate weights buffer in xc3_model itself?
    let mut skin_weights = &Vec::new();
    let mut bone_indices = &Vec::new();
    for attribute in &model.vertex_buffers.last().unwrap().attributes {
        match attribute {
            AttributeData::SkinWeights(values) => skin_weights = values,
            AttributeData::BoneIndices(values) => bone_indices = values,
            _ => (),
        }
    }

    for (i, vertex_buffer) in model
        .vertex_buffers
        .iter()
        .take(model.vertex_buffers.len() - 1)
        .enumerate()
    {
        let mut attributes = BTreeMap::new();
        for attribute in &vertex_buffer.attributes {
            match attribute {
                AttributeData::Position(values) => {
                    add_attribute_values(
                        values,
                        gltf::Semantic::Positions,
                        gltf::json::accessor::Type::Vec3,
                        gltf::json::accessor::ComponentType::F32,
                        buffer_bytes,
                        buffer_views,
                        &mut attributes,
                        accessors,
                    );
                }
                AttributeData::Normal(values) => {
                    // Not all applications will normalize the vertex normals.
                    // Use Vec3 instead of Vec4 since it's better supported.
                    let values: Vec<_> = values.iter().map(|v| v.xyz().normalize()).collect();
                    add_attribute_values(
                        &values,
                        gltf::Semantic::Normals,
                        gltf::json::accessor::Type::Vec3,
                        gltf::json::accessor::ComponentType::F32,
                        buffer_bytes,
                        buffer_views,
                        &mut attributes,
                        accessors,
                    );
                }
                AttributeData::Tangent(values) => {
                    // TODO: do these values need to be scaled/normalized?
                    add_attribute_values(
                        values,
                        gltf::Semantic::Tangents,
                        gltf::json::accessor::Type::Vec4,
                        gltf::json::accessor::ComponentType::F32,
                        buffer_bytes,
                        buffer_views,
                        &mut attributes,
                        accessors,
                    );
                }
                AttributeData::Uv1(values) => {
                    add_attribute_values(
                        values,
                        gltf::Semantic::TexCoords(0),
                        gltf::json::accessor::Type::Vec2,
                        gltf::json::accessor::ComponentType::F32,
                        buffer_bytes,
                        buffer_views,
                        &mut attributes,
                        accessors,
                    );
                }
                AttributeData::Uv2(values) => {
                    add_attribute_values(
                        values,
                        gltf::Semantic::TexCoords(1),
                        gltf::json::accessor::Type::Vec2,
                        gltf::json::accessor::ComponentType::F32,
                        buffer_bytes,
                        buffer_views,
                        &mut attributes,
                        accessors,
                    );
                }
                AttributeData::VertexColor(values) => {
                    add_attribute_values(
                        values,
                        gltf::Semantic::Colors(0),
                        gltf::json::accessor::Type::Vec4,
                        gltf::json::accessor::ComponentType::F32,
                        buffer_bytes,
                        buffer_views,
                        &mut attributes,
                        accessors,
                    );
                }
                AttributeData::WeightIndex(indices) => {
                    // Skin weights and indices are shared among all buffers.
                    // TODO: handle in xc3_model types and test.
                    let mut actual_weights = Vec::new();
                    let mut actual_indices = Vec::new();

                    // The vertex attributes use the ordering of the mxmd skeleton.
                    // Create a mapping so we can still use the chr skeleton.
                    // TODO: Modify the attributes ahead of time or make a function for this?
                    let mut mxmd_to_chr = Vec::new();
                    if let Some(skeleton) = skeleton {
                        for (i, mxmd_name) in skeleton.mxmd_names.iter().enumerate() {
                            if let Some(chr_index) =
                                skeleton.bones.iter().position(|b| &b.name == mxmd_name)
                            {
                                mxmd_to_chr.push(chr_index as u8);
                            } else {
                                // TODO: how to handle unmapped bones?
                                mxmd_to_chr.push(i as u8);
                            }
                        }
                    }

                    for index in indices {
                        actual_weights.push(skin_weights[*index as usize]);

                        let remapped_indices =
                            bone_indices[*index as usize].map(|i| mxmd_to_chr[i as usize]);
                        actual_indices.push(remapped_indices);
                    }

                    add_attribute_values(
                        &actual_weights,
                        gltf::Semantic::Weights(0),
                        gltf::json::accessor::Type::Vec4,
                        gltf::json::accessor::ComponentType::F32,
                        buffer_bytes,
                        buffer_views,
                        &mut attributes,
                        accessors,
                    );
                    add_attribute_values(
                        &actual_indices,
                        gltf::Semantic::Joints(0),
                        gltf::json::accessor::Type::Vec4,
                        gltf::json::accessor::ComponentType::U8,
                        buffer_bytes,
                        buffer_views,
                        &mut attributes,
                        accessors,
                    );
                }
                AttributeData::SkinWeights(_) => (),
                AttributeData::BoneIndices(_) => (),
            }
        }

        vertex_buffer_attributes.insert(
            BufferKey {
                root_index,
                group_index,
                model_index,
                buffer_index: i,
            },
            attributes,
        );
    }
}

fn add_attribute_values<T: WriteBytes>(
    values: &[T],
    semantic: gltf::Semantic,
    components: gltf::json::accessor::Type,
    component_type: gltf::json::accessor::ComponentType,
    buffer_bytes: &mut Vec<u8>,
    buffer_views: &mut Vec<gltf::json::buffer::View>,
    attributes: &mut GltfAttributes,
    accessors: &mut Vec<gltf::json::Accessor>,
) {
    let attribute_bytes = write_bytes(values);

    // Assume everything uses the same buffer for now.
    // Each attribute is in its own section and thus has its own view.
    let view = gltf::json::buffer::View {
        buffer: gltf::json::Index::new(0),
        byte_length: attribute_bytes.len() as u32,
        byte_offset: Some(buffer_bytes.len() as u32),
        byte_stride: Some(std::mem::size_of::<T>() as u32),
        extensions: Default::default(),
        extras: Default::default(),
        name: None,
        target: Some(Valid(gltf::json::buffer::Target::ArrayBuffer)),
    };
    buffer_bytes.extend_from_slice(&attribute_bytes);

    // TODO: min/max for positions.
    let accessor = gltf::json::Accessor {
        buffer_view: Some(gltf::json::Index::new(buffer_views.len() as u32)),
        byte_offset: 0,
        count: values.len() as u32,
        component_type: Valid(gltf::json::accessor::GenericComponentType(component_type)),
        extensions: Default::default(),
        extras: Default::default(),
        type_: Valid(components),
        min: None,
        max: None,
        name: None,
        normalized: false,
        sparse: None,
    };
    // Assume the buffer has only one of each attribute semantic.
    attributes.insert(
        Valid(semantic),
        gltf::json::Index::new(accessors.len() as u32),
    );
    accessors.push(accessor);
    buffer_views.push(view);
}

// gltf requires little endian for byte buffers.
// Create a trait instead of using bytemuck.
trait WriteBytes {
    fn write<W: Write + Seek>(&self, writer: &mut W);
}

impl WriteBytes for u16 {
    fn write<W: Write + Seek>(&self, writer: &mut W) {
        self.write_le(writer).unwrap();
    }
}

impl WriteBytes for [u8; 4] {
    fn write<W: Write + Seek>(&self, writer: &mut W) {
        self.write_le(writer).unwrap();
    }
}

impl WriteBytes for Vec2 {
    fn write<W: Write + Seek>(&self, writer: &mut W) {
        self.to_array().write_le(writer).unwrap();
    }
}

impl WriteBytes for Vec3 {
    fn write<W: Write + Seek>(&self, writer: &mut W) {
        self.to_array().write_le(writer).unwrap();
    }
}

impl WriteBytes for Vec4 {
    fn write<W: Write + Seek>(&self, writer: &mut W) {
        self.to_array().write_le(writer).unwrap();
    }
}

impl WriteBytes for Mat4 {
    fn write<W: Write + Seek>(&self, writer: &mut W) {
        self.to_cols_array().write_le(writer).unwrap();
    }
}

fn write_bytes<T: WriteBytes>(values: &[T]) -> Vec<u8> {
    let mut writer = Cursor::new(Vec::new());
    for v in values {
        v.write(&mut writer);
    }
    writer.into_inner()
}
