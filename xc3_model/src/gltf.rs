//! Conversions from xc3_model types to glTF.
//!
//! # Getting Started
//! ```rust no_run
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! use xc3_model::gltf::GltfFile;
//! use xc3_model::shader_database::ShaderDatabase;
//! use xc3_model::monolib::ShaderTextures;
//!
//! let database = ShaderDatabase::from_file("xc3.bin")?;
//!
//! // Models have only one root.
//! let root = xc3_model::load_model("xeno3/chr/ch/ch01027000.wimdo", Some(&database))?;
//! let shader_textures = ShaderTextures::from_folder("xeno3/monolib/shader");
//! let animations = xc3_model::load_animations("xeno3/chr/ch/ch01027000_event.mot")?;
//! let gltf = GltfFile::from_model("mio_military", &[root], &animations, &shader_textures, false)?;
//! gltf.save("mio_military.gltf")?;
//!
//! // Xenoblade X models need to have images and UVs flipped.
//! let root = xc3_model::load_model("xenox/chr_np/np009001.camdo", Some(&database))?;
//! let gltf = GltfFile::from_model("tatsu", &[root], &animations, &ShaderTextures::default(), true)?;
//! gltf.save("tatsu.gltf")?;
//!
//! // Maps have multiple roots.
//! let roots = xc3_model::load_map("xeno3/map/ma59a.wismhd", Some(&database))?;
//! let shader_textures = ShaderTextures::from_folder("xeno3/monolib/shader");
//! let gltf = GltfFile::from_map("map", &roots, &shader_textures, false)?;
//! gltf.save("map.gltf")?;
//! # Ok(())
//! # }
//! ```
use std::{borrow::Cow, collections::BTreeMap, io::BufWriter, path::Path};

use crate::{
    animation::Animation, monolib::ShaderTextures, skeleton::merge_skeletons, MapRoot, ModelRoot,
};
use animation::add_animations;
use glam::Mat4;
use gltf::json::validation::Checked::Valid;
use rayon::prelude::*;
use thiserror::Error;

use self::{
    buffer::{BufferKey, Buffers, WeightGroupKey},
    material::{MaterialCache, MaterialKey},
    texture::{image_name, TextureCache},
};

mod animation;
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

/// glb data for a model or map.
#[derive(Debug)]
pub struct GlbFile {
    /// The glTF file JSON object.
    pub root: gltf::json::Root,
    /// The data for the bin file with vertex and image data for all models.
    pub buffer: Vec<u8>,
}

#[derive(Default)]
struct GltfData {
    texture_cache: TextureCache,
    material_cache: MaterialCache,
    buffers: Buffers,
    meshes: Vec<gltf::json::Mesh>,
    nodes: Vec<gltf::json::Node>,
    scene_nodes: Vec<gltf::json::Index<gltf::json::Node>>,
    skins: Vec<gltf::json::Skin>,
    animations: Vec<gltf::json::animation::Animation>,
}

impl GltfData {
    fn add_node(&mut self, node: gltf::json::Node) -> u32 {
        let index = self.nodes.len() as u32;
        self.nodes.push(node);
        index
    }

    fn into_gltf(
        self,
        model_name: &str,
        flip_images_uvs: bool,
    ) -> Result<GltfFile, CreateGltfError> {
        // The textures assume the images are in ascending order by index.
        // The texture cache already preserves insertion order.
        let mut images = Vec::new();
        for key in self.texture_cache.generated_texture_indices.keys() {
            images.push(gltf::json::Image {
                buffer_view: None,
                mime_type: Some(gltf::json::image::MimeType("image/png".to_string())),
                name: None,
                uri: Some(image_name(key, model_name)),
                extensions: None,
                extras: Default::default(),
            });
        }

        let buffer_name = format!("{model_name}.buffer0.bin");

        let buffer = gltf::json::Buffer {
            byte_length: self.buffers.buffer_bytes.len().into(),
            extensions: Default::default(),
            extras: Default::default(),
            name: None,
            uri: Some(buffer_name.clone()),
        };

        let root = gltf::json::Root {
            accessors: self.buffers.accessors,
            buffers: vec![buffer],
            buffer_views: self.buffers.buffer_views,
            meshes: self.meshes,
            nodes: self.nodes,
            scenes: vec![gltf::json::Scene {
                extensions: Default::default(),
                extras: Default::default(),
                name: None,
                nodes: self.scene_nodes,
            }],
            materials: self.material_cache.materials,
            textures: self.material_cache.textures,
            images,
            skins: self.skins,
            samplers: self.material_cache.samplers,
            animations: self.animations,
            ..Default::default()
        };

        let png_images = self
            .texture_cache
            .generate_png_images(model_name, flip_images_uvs);

        Ok(GltfFile {
            root,
            buffer_name,
            buffer: self.buffers.buffer_bytes,
            png_images,
        })
    }

    fn into_glb(self, model_name: &str, flip_images_uvs: bool) -> Result<GlbFile, CreateGltfError> {
        // TODO: Avoid clone?
        let mut buffers = self.buffers.clone();
        align_bytes(&mut buffers.buffer_bytes, 4);

        let png_images = self
            .texture_cache
            .generate_png_images(model_name, flip_images_uvs);

        let mut images = Vec::new();
        for (name, png_bytes) in png_images {
            // Embed images in the same buffer as vertex data.
            let view = gltf::json::buffer::View {
                buffer: gltf::json::Index::new(0),
                byte_length: png_bytes.len().into(),
                byte_offset: Some(buffers.buffer_bytes.len().into()),
                byte_stride: None,
                extensions: Default::default(),
                extras: Default::default(),
                name: Some(name),
                target: None,
            };
            let index = gltf::json::Index::new(buffers.buffer_views.len() as u32);
            buffers.buffer_views.push(view);

            buffers.buffer_bytes.extend_from_slice(&png_bytes);
            align_bytes(&mut buffers.buffer_bytes, 4);

            images.push(gltf::json::Image {
                buffer_view: Some(index),
                mime_type: Some(gltf::json::image::MimeType("image/png".to_string())),
                name: None,
                uri: None,
                extensions: None,
                extras: Default::default(),
            });
        }

        let root = gltf::json::Root {
            accessors: buffers.accessors,
            buffers: vec![gltf::json::Buffer {
                byte_length: buffers.buffer_bytes.len().into(),
                extensions: Default::default(),
                extras: Default::default(),
                name: None,
                uri: None,
            }],
            buffer_views: buffers.buffer_views,
            meshes: self.meshes,
            nodes: self.nodes,
            scenes: vec![gltf::json::Scene {
                extensions: Default::default(),
                extras: Default::default(),
                name: None,
                nodes: self.scene_nodes,
            }],
            materials: self.material_cache.materials,
            textures: self.material_cache.textures,
            images,
            skins: self.skins,
            samplers: self.material_cache.samplers,
            animations: self.animations,
            ..Default::default()
        };

        Ok(GlbFile {
            root,
            buffer: buffers.buffer_bytes,
        })
    }

    fn from_model(
        model_name: &str,
        roots: &[ModelRoot],
        animations: &[Animation],
        shader_textures: &ShaderTextures,
        flip_images_uvs: bool,
    ) -> Result<Self, CreateGltfError> {
        let mut data = GltfData {
            texture_cache: TextureCache::new(
                roots.iter().map(|r| &r.image_textures),
                shader_textures,
            ),
            ..Default::default()
        };

        let skeletons: Vec<_> = roots.iter().filter_map(|r| r.skeleton.clone()).collect();
        let combined_skeleton = merge_skeletons(&skeletons);

        let (root_bone_index, skin_index) = create_skin(
            combined_skeleton.as_ref(),
            &mut data.nodes,
            &mut data.skins,
            &mut data.buffers,
            model_name,
        )
        .unzip();

        let mut root_children = Vec::new();
        if let Some(i) = root_bone_index {
            root_children.push(gltf::json::Index::new(i));
        }

        for (root_index, root) in roots.iter().enumerate() {
            // TODO: Avoid clone?
            let group_buffers = &[root.buffers.clone()];

            data.material_cache
                .insert_samplers(&root.models, root_index, 0, 0);

            let models_node_index = add_models(
                &root.models,
                group_buffers,
                &mut data,
                &root.image_textures,
                root_index,
                0,
                0,
                skin_index,
                combined_skeleton.as_ref(),
                flip_images_uvs,
            )?;
            root_children.push(gltf::json::Index::new(models_node_index));
        }

        let root_node_index = data.add_node(gltf::json::Node {
            children: Some(root_children),
            name: Some(model_name.to_string()),
            ..default_node()
        });
        data.scene_nodes
            .push(gltf::json::Index::new(root_node_index));

        if let Some(root_bone_index) = root_bone_index {
            if let Some(skeleton) = &combined_skeleton {
                add_animations(&mut data, animations, skeleton, root_bone_index)?;
            }
        }

        Ok(data)
    }

    fn from_map(
        model_name: &str,
        roots: &[MapRoot],
        shader_textures: &ShaderTextures,
        flip_images_uvs: bool,
    ) -> Result<Self, CreateGltfError> {
        let mut data = GltfData {
            texture_cache: TextureCache::new(
                roots.iter().map(|r| &r.image_textures),
                shader_textures,
            ),
            ..Default::default()
        };

        for (root_index, root) in roots.iter().enumerate() {
            let mut root_children = Vec::new();
            for (group_index, group) in root.groups.iter().enumerate() {
                let mut group_children = Vec::new();
                for (models_index, models) in group.models.iter().enumerate() {
                    data.material_cache.insert_samplers(
                        models,
                        root_index,
                        group_index,
                        models_index,
                    );

                    let models_node_index = add_models(
                        models,
                        &group.buffers,
                        &mut data,
                        &root.image_textures,
                        root_index,
                        group_index,
                        models_index,
                        None,
                        None,
                        flip_images_uvs,
                    )?;
                    group_children.push(gltf::json::Index::new(models_node_index));
                }

                let group_node_index = data.add_node(gltf::json::Node {
                    name: Some(format!("group{group_index}")),
                    children: Some(group_children),
                    ..default_node()
                });
                root_children.push(gltf::json::Index::new(group_node_index));
            }

            let root_node_index = data.add_node(gltf::json::Node {
                name: Some(format!("{model_name}.root{root_index}")),
                children: Some(root_children),
                ..default_node()
            });
            data.scene_nodes
                .push(gltf::json::Index::new(root_node_index));
        }

        Ok(data)
    }
}

impl GltfFile {
    /// Convert the Xenoblade model `roots` to glTF data.
    /// See [load_model](crate::load_model) or [load_model_legacy](crate::load_model_legacy) for loading files.
    ///
    /// The `model_name` is used to create resource file names and should
    /// usually match the file name for [save](GltfFile::save) without the `.gltf` extension.
    ///
    /// `flip_image_uvs` should only be set to `true` for Xenoblade X models.
    ///
    /// Skeletons from all `roots` will be merged into a single skeleton with all bones.
    /// Each animation in `animations` will apply to this combined skeleton.
    pub fn from_model(
        model_name: &str,
        roots: &[ModelRoot],
        animations: &[Animation],
        shader_textures: &ShaderTextures,
        flip_images_uvs: bool,
    ) -> Result<Self, CreateGltfError> {
        GltfData::from_model(
            model_name,
            roots,
            animations,
            shader_textures,
            flip_images_uvs,
        )?
        .into_gltf(model_name, flip_images_uvs)
    }

    /// Convert the Xenoblade map `roots` to glTF data.
    /// See [load_map](crate::load_map) for loading files.
    ///
    /// The `model_name` is used to create resource file names and should
    /// usually match the file name for [save](GltfFile::save) without the `.gltf` extension.
    ///
    /// `flip_images_uvs` should only be set to `true` for Xenoblade X maps.
    pub fn from_map(
        model_name: &str,
        roots: &[MapRoot],
        shader_textures: &ShaderTextures,
        flip_images_uvs: bool,
    ) -> Result<Self, CreateGltfError> {
        GltfData::from_map(model_name, roots, shader_textures, flip_images_uvs)?
            .into_gltf(model_name, flip_images_uvs)
    }

    /// Save the glTF data to the specified `path` with images and buffers stored in the same directory.
    ///
    /// # Examples
    ///
    /// ```rust no_run
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # use xc3_model::gltf::GltfFile;
    /// # let roots = Vec::new();
    /// let gltf_file = GltfFile::from_model("model", &roots, &[], &Default::default(), false)?;
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

impl GlbFile {
    /// Convert the Xenoblade model `roots` to glb data.
    /// See [load_model](crate::load_model) or [load_model_legacy](crate::load_model_legacy) for loading files.
    ///
    /// The `model_name` is used to create resource names and should
    /// usually match the file name for [save](GlbFile::save) without the `.glb` extension.
    ///
    /// `flip_image_uvs` should only be set to `true` for Xenoblade X models.
    pub fn from_model(
        model_name: &str,
        roots: &[ModelRoot],
        animations: &[Animation],
        shader_textures: &ShaderTextures,
        flip_images_uvs: bool,
    ) -> Result<Self, CreateGltfError> {
        // TODO: Does this need a model name?
        GltfData::from_model(
            model_name,
            roots,
            animations,
            shader_textures,
            flip_images_uvs,
        )?
        .into_glb(model_name, flip_images_uvs)
    }

    /// Convert the Xenoblade map `roots` to glTF data.
    /// See [load_map](crate::load_map) for loading files.
    ///
    /// The `model_name` is used to create resource names and should
    /// usually match the file name for [save](GlbFile::save) without the `.glb` extension.
    ///
    /// `flip_images_uvs` should only be set to `true` for Xenoblade X maps.
    pub fn from_map(
        model_name: &str,
        roots: &[MapRoot],
        shader_textures: &ShaderTextures,
        flip_images_uvs: bool,
    ) -> Result<Self, CreateGltfError> {
        // TODO: Does this need a model name?
        GltfData::from_map(model_name, roots, shader_textures, flip_images_uvs)?
            .into_glb(model_name, flip_images_uvs)
    }

    /// Save the glb data to the specified `path`.
    ///
    /// # Examples
    ///
    /// ```rust no_run
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # use xc3_model::gltf::GlbFile;
    /// # let roots = Vec::new();
    /// let glb_file = GlbFile::from_model("model", &roots, &[], &Default::default(), false)?;
    /// glb_file.save("model.glb")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), SaveGltfError> {
        let json = serde_json::to_string(&self.root)?;
        let glb = gltf::binary::Glb {
            header: gltf::binary::Header {
                magic: *b"glTF",
                version: 2,
                length: (json.len().next_multiple_of(4) + self.buffer.len().next_multiple_of(4))
                    .try_into()
                    .expect("file size exceeds binary glTF limit"),
            },
            bin: Some(Cow::Borrowed(&self.buffer)),
            json: Cow::Owned(json.into_bytes()),
        };

        // TODO: is fully buffered faster?
        let writer = BufWriter::new(std::fs::File::create(path)?);
        // Assume to_writer handles aligning chunk sizes to 4 bytes.
        glb.to_writer(writer).expect("glTF binary output error");

        Ok(())
    }
}

#[allow(clippy::too_many_arguments)]
fn add_models(
    models: &crate::Models,
    group_buffers: &[crate::vertex::ModelBuffers],
    data: &mut GltfData,
    image_textures: &[crate::ImageTexture],
    root_index: usize,
    group_index: usize,
    models_index: usize,
    skin_index: Option<u32>,
    skeleton: Option<&crate::skeleton::Skeleton>,
    flip_uvs: bool,
) -> Result<u32, CreateGltfError> {
    let mut models_children = Vec::new();
    for (model_index, model) in models.models.iter().enumerate() {
        let mut children = Vec::new();

        let model_buffers = &group_buffers[model.model_buffers_index];

        for mesh in &model.meshes {
            // TODO: Make LOD selection configurable?
            // TODO: Add an option to export all material passes?
            let material = &models.materials[mesh.material_index];
            if models
                .lod_data
                .as_ref()
                .map(|d| d.is_base_lod(mesh.lod_item_index))
                .unwrap_or(true)
                && !material.name.ends_with("_outline")
                && !material.name.contains("_speff_")
            {
                // Lazy load vertex buffers since not all are unused.
                // TODO: How expensive is this clone?
                let vertex_buffer = data
                    .buffers
                    .insert_vertex_buffer(
                        &model_buffers.vertex_buffers[mesh.vertex_buffer_index],
                        root_index,
                        group_index,
                        model.model_buffers_index,
                        mesh.vertex_buffer_index,
                        flip_uvs,
                    )?
                    .clone();
                let mut attributes = vertex_buffer.attributes.clone();

                // Load skinning attributes separately to handle per mesh indexing.
                let weights_start_index = model_buffers
                    .weights
                    .as_ref()
                    .map(|w| {
                        w.weight_groups.weights_start_index(
                            mesh.flags2.into(),
                            mesh.lod_item_index,
                            material.pass_type,
                        )
                    })
                    .unwrap_or_default();

                if let Some(weight_group) = data.buffers.insert_weight_group(
                    model_buffers,
                    skeleton,
                    WeightGroupKey {
                        weights_start_index,
                        flags2: mesh.flags2.into(),
                        buffer: BufferKey {
                            root_index,
                            group_index,
                            buffers_index: model.model_buffers_index,
                            buffer_index: mesh.vertex_buffer_index,
                        },
                    },
                ) {
                    attributes.insert(weight_group.weights.0.clone(), weight_group.weights.1);
                    attributes.insert(weight_group.indices.0.clone(), weight_group.indices.1);
                }

                // Lazy load index buffers since not all are unused.
                let index_accessor = data.buffers.insert_index_buffer(
                    &model_buffers.index_buffers[mesh.index_buffer_index],
                    root_index,
                    group_index,
                    model.model_buffers_index,
                    mesh.index_buffer_index,
                )? as u32;

                // We lazy load meshes, so also lazy load materials to save space.
                let material_index = data.material_cache.insert(
                    material,
                    &mut data.texture_cache,
                    image_textures,
                    MaterialKey {
                        root_index,
                        group_index,
                        models_index,
                        material_index: mesh.material_index,
                    },
                );

                let targets = morph_targets(&vertex_buffer);
                // The first target is baked into vertices, so don't set weights.
                let weights = targets.as_ref().map(|targets| vec![0.0; targets.len()]);

                // TODO: is there a cleaner way of doing this?
                let mesh_extras = targets.as_ref().map(|_| {
                    Box::new(serde_json::value::RawValue::from_string(
                        serde_json::to_string(&BTreeMap::from([(
                            "targetNames",
                            &models.morph_controller_names,
                        )]))
                        .unwrap(),
                    ))
                    .unwrap()
                });

                let primitive = gltf::json::mesh::Primitive {
                    attributes,
                    extensions: Default::default(),
                    extras: Default::default(),
                    indices: Some(gltf::json::Index::new(index_accessor)),
                    material: Some(gltf::json::Index::new(material_index as u32)),
                    mode: Valid(gltf::json::mesh::Mode::Triangles),
                    targets,
                };

                // Assign one primitive per mesh to create distinct objects in applications.
                // In game meshes aren't named, so just use the material name.
                let mesh = gltf::json::Mesh {
                    extensions: Default::default(),
                    extras: mesh_extras,
                    name: Some(material.name.clone()),
                    primitives: vec![primitive],
                    weights,
                };
                let mesh_index = data.meshes.len() as u32;
                data.meshes.push(mesh);

                // Instancing is applied at the model level.
                // Instance meshes instead so each node has only one parent.
                for instance in &model.instances {
                    let child_index = data.add_node(gltf::json::Node {
                        matrix: if *instance == Mat4::IDENTITY {
                            None
                        } else {
                            Some(instance.to_cols_array())
                        },
                        mesh: Some(gltf::json::Index::new(mesh_index)),
                        skin: skin_index.map(gltf::json::Index::new),
                        ..default_node()
                    });

                    children.push(gltf::json::Index::new(child_index))
                }
            }
        }

        let model_node_index = data.add_node(gltf::json::Node {
            children: Some(children.clone()),
            name: Some(format!("model{model_index}")),
            ..default_node()
        });
        models_children.push(gltf::json::Index::new(model_node_index));
    }
    // TODO: Find a better way to organize character roots?
    let models_node_index = data.add_node(gltf::json::Node {
        children: Some(models_children),
        name: Some(format!("models{models_index}")),
        ..default_node()
    });
    Ok(models_node_index)
}

fn default_node() -> gltf::json::Node {
    gltf::json::Node {
        camera: None,
        children: None,
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
    skins: &mut Vec<gltf::json::Skin>,
    buffers: &mut Buffers,
    model_name: &str,
) -> Option<(u32, u32)> {
    skeleton.as_ref().map(|skeleton| {
        let bone_start_index = nodes.len() as u32;
        for (i, bone) in skeleton.bones.iter().enumerate() {
            let children = find_children(skeleton, i, bone_start_index);

            // Use TRS in case the bone node is the target of animation channels.
            let (translation, rotation, scale) = if bone.transform != Mat4::IDENTITY {
                let (s, r, t) = bone.transform.to_scale_rotation_translation();
                (
                    Some(t.to_array()),
                    Some(gltf::json::scene::UnitQuaternion(r.to_array())),
                    Some(s.to_array()),
                )
            } else {
                (None, None, None)
            };

            let joint_node = gltf::json::Node {
                children: if !children.is_empty() {
                    Some(children)
                } else {
                    None
                },
                translation,
                rotation,
                scale,
                name: Some(bone.name.clone()),
                ..default_node()
            };
            nodes.push(joint_node);
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
                (None, None),
                false,
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
            name: Some(model_name.to_string()),
            skeleton: None,
        };
        let skin_index = skins.len() as u32;
        skins.push(skin);

        (bone_start_index, skin_index)
    })
}

fn find_children(
    skeleton: &crate::skeleton::Skeleton,
    bone_index: usize,
    base_index: u32,
) -> Vec<gltf::json::Index<gltf::json::Node>> {
    // TODO: is is worth optimizing this lookup?
    skeleton
        .bones
        .iter()
        .enumerate()
        .filter_map(|(child_index, b)| {
            if b.parent_index == Some(bone_index) {
                Some(gltf::json::Index::new(child_index as u32 + base_index))
            } else {
                None
            }
        })
        .collect()
}

fn align_bytes(bytes: &mut Vec<u8>, align: usize) {
    let aligned = bytes.len().next_multiple_of(align);
    bytes.resize(aligned, 0u8);
}
