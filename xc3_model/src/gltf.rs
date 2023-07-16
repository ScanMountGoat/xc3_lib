use std::{collections::BTreeMap, path::Path};

use crate::{vertex::AttributeData, ModelRoot};
use glam::{Mat4, Vec4Swizzles};
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
    red_index: Option<usize>,
    green_index: Option<usize>,
    blue_index: Option<usize>,
    alpha_index: Option<usize>,
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

    fn insert(&mut self, key: GeneratedImageKey, recalculate_z: bool) -> Option<u32> {
        // Use a cache to avoid costly channel reconstructions if possible.
        self.generated_texture_indices
            .get(&key)
            .copied()
            .or_else(|| {
                // Only create an image if it has at least one input.
                generate_image(key, &self.original_images, recalculate_z).map(|image| {
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
                let albedo_index = albedo_key.and_then(|key| texture_cache.insert(key, false));

                let normal_key = normal_generated_key(material, root_index);
                let normal_index = normal_key.and_then(|key| texture_cache.insert(key, true));

                let metallic_roughness_key = metallic_roughness_generated_key(material, root_index);
                let metallic_roughness_index =
                    metallic_roughness_key.and_then(|key| texture_cache.insert(key, false));

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
        buffer,
        buffer_bytes,
        buffer_views,
        accessors,
        vertex_buffer_attributes,
        index_buffer_accessors,
    } = create_buffers(roots, buffer_name.clone());

    // TODO: select by LOD and skip outline meshes?
    let mut meshes = Vec::new();
    let mut nodes = Vec::new();
    let mut scene_nodes = Vec::new();

    for (root_index, root) in roots.iter().enumerate() {
        for (group_index, group) in root.groups.iter().enumerate() {
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
                            skin: None,
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
        ..Default::default()
    }
}

// TODO: Create consts for the gbuffer texture indices?
fn albedo_generated_key(
    material: &crate::Material,
    root_index: usize,
) -> Option<GeneratedImageKey> {
    let red_index = texture_index(material, 0, 'x');
    let green_index = texture_index(material, 0, 'y');
    let blue_index = texture_index(material, 0, 'z');
    let alpha_index = texture_index(material, 0, 'w');

    Some(GeneratedImageKey {
        root_index,
        red_index,
        green_index,
        blue_index,
        alpha_index,
    })
}

fn normal_generated_key(
    material: &crate::Material,
    root_index: usize,
) -> Option<GeneratedImageKey> {
    let red_index = texture_index(material, 2, 'x');
    let green_index = texture_index(material, 2, 'y');
    let blue_index = texture_index(material, 2, 'z');
    let alpha_index = texture_index(material, 2, 'w');

    Some(GeneratedImageKey {
        root_index,
        red_index,
        green_index,
        blue_index,
        alpha_index,
    })
}

fn metallic_roughness_generated_key(
    material: &crate::Material,
    root_index: usize,
) -> Option<GeneratedImageKey> {
    let metalness_index = texture_index(material, 1, 'x');
    // TODO: Generated roughness from glossiness?
    Some(GeneratedImageKey {
        root_index,
        red_index: None,
        green_index: None,
        blue_index: metalness_index,
        alpha_index: None,
    })
}

fn generate_image(
    key: GeneratedImageKey,
    original_images: &BTreeMap<ImageKey, image::RgbaImage>,
    recalculate_z: bool,
) -> Option<image::RgbaImage> {
    let find_image = |index: Option<usize>| {
        index.and_then(|image_index| {
            original_images.get(&ImageKey {
                root_index: key.root_index,
                image_index,
            })
        })
    };

    let red_image = find_image(key.red_index);
    let green_image = find_image(key.green_index);
    let blue_image = find_image(key.blue_index);
    let alpha_image = find_image(key.alpha_index);

    // Use the dimensions of the largest image to avoid quality loss.
    let (width, height) = [red_image, green_image, blue_image, alpha_image]
        .iter()
        .filter_map(|i| i.map(|i| i.dimensions()))
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

    if recalculate_z {
        // Reconstruct the normal map Z channel.
        for pixel in image.pixels_mut() {
            // x^y + y^2 + z^2 = 1 for unit vectors.
            let x = (pixel[0] as f32 / 255.0) * 2.0 - 1.0;
            let y = (pixel[1] as f32 / 255.0) * 2.0 - 1.0;
            let z = 1.0 - x * x - y * y;
            pixel[2] = (z * 255.0) as u8;
        }
    }

    Some(image)
}

fn assign_channel(
    image: &mut image::RgbaImage,
    channel: Option<&image::RgbaImage>,
    channel_index: usize,
) {
    if let Some(channel) = channel {
        for (pixel, channel_pixel) in image.pixels_mut().zip(channel.pixels()) {
            pixel[channel_index] = channel_pixel[channel_index];
        }
    }
}

fn image_name(key: &GeneratedImageKey) -> String {
    // TODO: Don't include missing channels?
    format!(
        "root{}_{}_{}_{}_{}.png",
        key.root_index,
        key.red_index.unwrap_or_default(),
        key.green_index.unwrap_or_default(),
        key.blue_index.unwrap_or_default(),
        key.alpha_index.unwrap_or_default()
    )
}

fn texture_index(material: &crate::Material, gbuffer_index: usize, channel: char) -> Option<usize> {
    // Find the sampler from the material.
    let (sampler_index, _) = material
        .shader
        .as_ref()?
        .material_channel_assignment(gbuffer_index, channel)?;

    // Find the texture referenced by this sampler.
    material
        .textures
        .get(sampler_index as usize)
        .map(|t| t.image_texture_index)
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
        // TODO: enforce little endian instead of casting
        let index_bytes: &[u8] = bytemuck::cast_slice(&index_buffer.indices);

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
        buffer_bytes.extend_from_slice(index_bytes);
    }
}

fn add_vertex_buffers(
    model: &crate::Model,
    root_index: usize,
    group_index: usize,
    model_index: usize,
    buffer_bytes: &mut Vec<u8>,
    buffer_views: &mut Vec<gltf::json::buffer::View>,
    accessors: &mut Vec<gltf::json::Accessor>,
    vertex_buffer_attributes: &mut BTreeMap<BufferKey, GltfAttributes>,
) {
    for (i, vertex_buffer) in model.vertex_buffers.iter().enumerate() {
        let mut attributes = BTreeMap::new();
        for attribute in &vertex_buffer.attributes {
            match attribute {
                AttributeData::Position(values) => {
                    add_attribute_values(
                        values,
                        gltf::Semantic::Positions,
                        gltf::json::accessor::Type::Vec3,
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
                        buffer_bytes,
                        buffer_views,
                        &mut attributes,
                        accessors,
                    );
                }
                AttributeData::WeightIndex(_) => (),
                AttributeData::Weights(_) => (),
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

fn add_attribute_values<T: bytemuck::Pod>(
    values: &[T],
    semantic: gltf::Semantic,
    components: gltf::json::accessor::Type,
    buffer_bytes: &mut Vec<u8>,
    buffer_views: &mut Vec<gltf::json::buffer::View>,
    attributes: &mut GltfAttributes,
    accessors: &mut Vec<gltf::json::Accessor>,
) {
    // TODO: enforce little endian instead of casting
    let attribute_bytes = bytemuck::cast_slice(values);

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
    buffer_bytes.extend_from_slice(attribute_bytes);
    // TODO: Alignment after each attribute?

    let accessor = gltf::json::Accessor {
        buffer_view: Some(gltf::json::Index::new(buffer_views.len() as u32)),
        byte_offset: 0,
        count: values.len() as u32,
        component_type: Valid(gltf::json::accessor::GenericComponentType(
            gltf::json::accessor::ComponentType::F32,
        )),
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
