//! Conversions from xc3_model types to glTF.
//!
//! # Getting Started
//! ```rust no_run
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! use xc3_model::gltf::GltfFile;
//! use xc3_model::shader_database::ShaderDatabase;
//!
//! let database = ShaderDatabase::from_file("xc3.json")?;
//!
//! // Models have only one root.
//! let root = xc3_model::load_model("xeno3/chr/ch/ch01027000.wimdo", Some(&database))?;
//! let gltf = GltfFile::new("mio_military", &[root])?;
//! gltf.save("mio_military.gltf")?;
//!
//! // Maps have multiple roots.
//! let roots = xc3_model::load_map("xeno3/map/ma59a.wismhd", Some(&database))?;
//! let gltf = GltfFile::new("map", &roots)?;
//! gltf.save("map.gltf")?;
//! # Ok(())
//! # }
//! ```
use std::path::Path;

use crate::{should_render_lod, ModelRoot};
use glam::Mat4;
use gltf::json::validation::Checked::Valid;
use rayon::prelude::*;
use thiserror::Error;

use self::{
    buffer::{BufferKey, Buffers, WeightGroupKey},
    material::{create_materials, MaterialKey},
    texture::{image_name, TextureCache},
};

mod buffer;
mod material;
mod texture;

// TODO: Add more error variants.
#[derive(Debug, Error)]
pub enum CreateGltfError {
    #[error("error writing buffers")]
    Binrw(#[from] binrw::Error),
}

#[derive(Debug, Error)]
pub enum SaveGltfError {
    #[error("error writing files")]
    Io(#[from] std::io::Error),

    #[error("error serializing JSON file")]
    Json(#[from] serde_json::Error),
}

/// glTF JSON, binary, and image data for a model or map.
#[derive(Debug)]
pub struct GltfFile {
    /// The glTF file JSON object.
    pub root: gltf::json::Root,
    /// The name of the bin file formated as `"{model_name}.buffer0.bin"`.
    pub buffer_name: String,
    /// The data for the bin file with vertex data for all models.
    pub buffer: Vec<u8>,
    // These have to be png or jpeg anyway.
    // Use PNG instead of RgbaImage to losslessly reduce memory usage.
    /// The file name with PNG extension and PNG file data for all generated textures.
    pub png_images: Vec<(String, Vec<u8>)>,
}

impl GltfFile {
    /// Convert the Xenoblade model `roots` to glTF data.
    /// See [load_model](crate::load_model) or [load_map](crate::load_map) for loading files.
    ///
    /// The `model_name` is used to create resource file names and should
    /// usually match the file name for [save](GltfFile::save) without the `.gltf` extension.
    pub fn new(model_name: &str, roots: &[ModelRoot]) -> Result<Self, CreateGltfError> {
        let mut texture_cache = TextureCache::new(roots);

        let (materials, material_indices, textures, samplers) =
            create_materials(roots, &mut texture_cache);

        let mut buffers = Buffers::default();

        let mut meshes = Vec::new();
        let mut nodes = Vec::new();
        let mut scene_nodes = Vec::new();
        let mut skins = Vec::new();

        for (root_index, root) in roots.iter().enumerate() {
            // TODO: Also include models skinning?
            let skin_index = create_skin(
                root.skeleton.as_ref(),
                &mut nodes,
                &mut scene_nodes,
                &mut skins,
                &mut buffers,
            );

            for (group_index, group) in root.groups.iter().enumerate() {
                for (models_index, models) in group.models.iter().enumerate() {
                    let mut group_children = Vec::new();

                    for model in &models.models {
                        let mut children = Vec::new();

                        let model_buffers = &group.buffers[model.model_buffers_index];

                        for mesh in &model.meshes {
                            // TODO: Make LOD selection configurable?
                            // TODO: Add an option to export all material passes?
                            let material = &models.materials[mesh.material_index];
                            if should_render_lod(mesh.lod, &models.base_lod_indices)
                                && !material.name.ends_with("_outline")
                                && !material.name.contains("_speff_")
                            {
                                // Lazy load vertex buffers since not all are unused.
                                // TODO: How expensive is this clone?
                                let vertex_buffer = buffers
                                    .insert_vertex_buffer(
                                        &model_buffers.vertex_buffers[mesh.vertex_buffer_index],
                                        root_index,
                                        group_index,
                                        model.model_buffers_index,
                                        mesh.vertex_buffer_index,
                                    )?
                                    .clone();
                                let mut attributes = vertex_buffer.attributes.clone();

                                // Load skinning attributes separately to handle per mesh indexing.
                                let weights_start_index = model_buffers
                                    .weights
                                    .as_ref()
                                    .map(|w| {
                                        w.weights_start_index(
                                            mesh.skin_flags,
                                            mesh.lod,
                                            material.pass_type,
                                        )
                                    })
                                    .unwrap_or_default();

                                if let Some(weight_group) = buffers.insert_weight_group(
                                    model_buffers,
                                    root.skeleton.as_ref(),
                                    WeightGroupKey {
                                        weights_start_index,
                                        buffer: BufferKey {
                                            root_index,
                                            group_index,
                                            buffers_index: model.model_buffers_index,
                                            buffer_index: mesh.vertex_buffer_index,
                                        },
                                    },
                                ) {
                                    attributes.insert(
                                        weight_group.weights.0.clone(),
                                        weight_group.weights.1,
                                    );
                                    attributes.insert(
                                        weight_group.indices.0.clone(),
                                        weight_group.indices.1,
                                    );
                                }

                                // Lazy load index buffers since not all are unused.
                                let index_accessor = buffers.insert_index_buffer(
                                    &model_buffers.index_buffers[mesh.index_buffer_index],
                                    root_index,
                                    group_index,
                                    model.model_buffers_index,
                                    mesh.index_buffer_index,
                                )? as u32;

                                let material_index = material_indices
                                    .get(&MaterialKey {
                                        root_index,
                                        group_index,
                                        models_index,
                                        material_index: mesh.material_index,
                                    })
                                    .unwrap();

                                let targets = morph_targets(&vertex_buffer);
                                // The first target is baked into vertices, so don't set weights.
                                let weights =
                                    targets.as_ref().map(|targets| vec![0.0; targets.len()]);

                                let primitive = gltf::json::mesh::Primitive {
                                    attributes,
                                    extensions: Default::default(),
                                    extras: Default::default(),
                                    indices: Some(gltf::json::Index::new(index_accessor)),
                                    material: Some(gltf::json::Index::new(*material_index as u32)),
                                    mode: Valid(gltf::json::mesh::Mode::Triangles),
                                    targets,
                                };

                                // Assign one primitive per mesh to create distinct objects in applications.
                                // In game meshes aren't named, so just use the material name.
                                let mesh = gltf::json::Mesh {
                                    extensions: Default::default(),
                                    extras: Default::default(),
                                    name: Some(material.name.clone()),
                                    primitives: vec![primitive],
                                    weights,
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

        // The textures assume the images are in ascending order by index.
        // The texture cache already preserves insertion order.
        let mut images = Vec::new();
        for key in texture_cache.generated_texture_indices.keys() {
            images.push(gltf::json::Image {
                buffer_view: None,
                mime_type: None,
                name: None,
                uri: Some(image_name(key, model_name)),
                extensions: None,
                extras: Default::default(),
            });
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
            textures,
            images,
            skins,
            samplers,
            ..Default::default()
        };

        let png_images = texture_cache.generate_png_images(model_name);

        Ok(Self {
            root,
            buffer_name,
            buffer: buffers.buffer_bytes,
            png_images,
        })
    }

    /// Save the glTF data to the specified `path` with images and buffers stored in the same directory.
    ///
    /// # Examples
    ///
    /// ```rust no_run
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # use xc3_model::gltf::GltfFile;
    /// # let roots = Vec::new();
    /// let gltf_file = GltfFile::new("model", &roots)?;
    /// gltf_file.save("model.gltf")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), SaveGltfError> {
        let path = path.as_ref();

        let json = gltf::json::serialize::to_string_pretty(&self.root)?;
        std::fs::write(path, json)?;

        std::fs::write(path.with_file_name(&self.buffer_name), &self.buffer)?;

        // Save images in parallel since PNG encoding is CPU intensive.
        self.png_images.par_iter().try_for_each(|(name, image)| {
            let output = path.with_file_name(name);
            std::fs::write(output, image)
        })?;
        Ok(())
    }
}

fn morph_targets(
    vertex_buffer: &buffer::VertexBuffer,
) -> Option<Vec<gltf::json::mesh::MorphTarget>> {
    if !vertex_buffer.morph_targets.is_empty() {
        Some(
            vertex_buffer
                .morph_targets
                .iter()
                .map(|attributes| gltf::json::mesh::MorphTarget {
                    positions: attributes.get(&Valid(gltf::Semantic::Positions)).copied(),
                    normals: attributes.get(&Valid(gltf::Semantic::Normals)).copied(),
                    tangents: attributes.get(&Valid(gltf::Semantic::Tangents)).copied(),
                })
                .collect(),
        )
    } else {
        None
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
            .model_space_transforms()
            .iter()
            .map(|t| t.inverse())
            .collect();

        let accessor_index = buffers
            .add_values(
                &inverse_bind_matrices,
                gltf::json::accessor::Type::Mat4,
                gltf::json::accessor::ComponentType::F32,
                None,
            )
            .unwrap();

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
