//! Conversions from xc3_model types to glTF.
use std::{collections::BTreeMap, path::Path};

use crate::{should_render_lod, ModelRoot};
use glam::Mat4;
use gltf::json::validation::Checked::Valid;
use rayon::prelude::*;

use self::{
    buffer::{BufferKey, Buffers},
    texture::{
        albedo_generated_key, image_name, metallic_roughness_generated_key, normal_generated_key,
        TextureCache,
    },
};

mod buffer;
mod texture;

// TODO: Module for materials
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct MaterialKey {
    root_index: usize,
    group_index: usize,
    models_index: usize,
    material_index: usize,
}

#[derive(Debug)]
pub struct GltfFile {
    pub root: gltf::json::Root,
    pub buffer_name: String,
    pub buffer: Vec<u8>,
    pub images: Vec<(String, image::RgbaImage)>,
}

impl GltfFile {
    /// Convert the Xenoblade model `roots` to glTF data.
    /// See [load_model](crate::load_model) or [load_map](crate::load_map) for loading files.
    ///
    /// The `model_name` is used to create resource file names and should
    /// usually match the file name used for [save](GltfFile::save) without the `.gltf` extension.
    pub fn new(model_name: &str, roots: &[ModelRoot]) -> Self {
        let mut texture_cache = TextureCache::new(roots);

        let mut materials = Vec::new();
        let mut material_indices = BTreeMap::new();

        for (root_index, root) in roots.iter().enumerate() {
            for (group_index, group) in root.groups.iter().enumerate() {
                for (models_index, models) in group.models.iter().enumerate() {
                    for (material_index, material) in models.materials.iter().enumerate() {
                        let albedo_key = albedo_generated_key(material, root_index);
                        let albedo_index = albedo_key.and_then(|key| texture_cache.insert(key));

                        let normal_key = normal_generated_key(material, root_index);
                        let normal_index = normal_key.and_then(|key| texture_cache.insert(key));

                        let metallic_roughness_key =
                            metallic_roughness_generated_key(material, root_index);
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
                                models_index,
                                material_index,
                            },
                            material_flattened_index,
                        );
                    }
                }
            }
        }

        let mut buffers = Buffers::new(roots);

        // TODO: select by LOD and skip outline meshes?
        let mut meshes = Vec::new();
        let mut nodes = Vec::new();
        let mut scene_nodes = Vec::new();
        let mut skins = Vec::new();

        for (root_index, root) in roots.iter().enumerate() {
            for (group_index, group) in root.groups.iter().enumerate() {
                for (models_index, models) in group.models.iter().enumerate() {
                    let skin_index = create_skin(
                        models.skeleton.as_ref(),
                        &mut nodes,
                        &mut scene_nodes,
                        &mut skins,
                        &mut buffers,
                    );

                    let mut group_children = Vec::new();

                    for model in &models.models {
                        let mut children = Vec::new();

                        for mesh in &model.meshes {
                            // TODO: Make LOD selection configurable?
                            if should_render_lod(mesh.lod, &models.base_lod_indices) {
                                let attributes_key = BufferKey {
                                    root_index,
                                    group_index,
                                    buffers_index: model.model_buffers_index,
                                    buffer_index: mesh.vertex_buffer_index,
                                };
                                let attributes = buffers
                                    .vertex_buffer_attributes
                                    .get(&attributes_key)
                                    .unwrap()
                                    .clone();

                                let indices_key = BufferKey {
                                    root_index,
                                    group_index,
                                    buffers_index: model.model_buffers_index,
                                    buffer_index: mesh.index_buffer_index,
                                };
                                let index_accessor =
                                    *buffers.index_buffer_accessors.get(&indices_key).unwrap()
                                        as u32;

                                let material_index = material_indices
                                    .get(&MaterialKey {
                                        root_index,
                                        group_index,
                                        models_index,
                                        material_index: mesh.material_index,
                                    })
                                    .unwrap();

                                let primitive = gltf::json::mesh::Primitive {
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

        let buffer_name = format!("{model_name}.buffer0.bin");

        let buffer = gltf::json::Buffer {
            byte_length: buffers.buffer_bytes.len() as u32,
            extensions: Default::default(),
            extras: Default::default(),
            name: None,
            uri: Some(buffer_name.clone()),
        };

        let root = gltf::json::Root {
            accessors: buffers.accessors,
            buffers: vec![buffer],
            buffer_views: buffers.buffer_views,
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

        let images = texture_cache
            .generated_images
            .into_par_iter()
            .map(|(key, image)| (image_name(&key), image))
            .collect();

        Self {
            root,
            buffer_name,
            buffer: buffers.buffer_bytes,
            images,
        }
    }

    /// Save the glTF data to the specified `path` with images and buffers stored in the same directory.
    ///
    /// # Examples
    ///
    /// ```rust no_run
    /// # use xc3_model::gltf::GltfFile;
    /// # let roots = Vec::new();
    /// let gltf_file = GltfFile::new("model", &roots);
    /// gltf_file.save("model.gltf");
    /// ```
    pub fn save<P: AsRef<Path>>(&self, path: P) {
        let path = path.as_ref();

        let json = gltf::json::serialize::to_string_pretty(&self.root).unwrap();
        std::fs::write(path, json).unwrap();

        std::fs::write(path.with_file_name(&self.buffer_name), &self.buffer).unwrap();

        // Save images in parallel since PNG encoding is CPU intensive.
        self.images.par_iter().for_each(|(name, image)| {
            let output = path.with_file_name(name);
            image.save(output).unwrap();
        });
    }
}

fn create_skin(
    skeleton: Option<&crate::skeleton::Skeleton>,
    nodes: &mut Vec<gltf::json::Node>,
    scene_nodes: &mut Vec<gltf::json::Index<gltf::json::Node>>,
    skins: &mut Vec<gltf::json::Skin>,
    buffers: &mut Buffers,
) -> Option<usize> {
    skeleton.as_ref().map(|skeleton| {
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

        let accessor_index = buffers.add_values(
            &inverse_bind_matrices,
            gltf::json::accessor::Type::Mat4,
            gltf::json::accessor::ComponentType::F32,
            None,
        );

        // TODO: Multiple roots for skeleton?
        let skin = gltf::json::Skin {
            extensions: Default::default(),
            extras: Default::default(),
            inverse_bind_matrices: Some(accessor_index),
            joints: (bone_start_index..bone_start_index + skeleton.bones.len() as u32)
                .map(gltf::json::Index::new)
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
