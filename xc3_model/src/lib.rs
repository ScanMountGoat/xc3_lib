//! # xc3_model
//! xc3_model provides high level data access for the files that make up a model.

use glam::Mat4;
use texture::ImageTexture;
use vertex::{read_index_buffers, read_vertex_buffers, AttributeData};

pub mod gltf;
pub mod texture;
pub mod vertex;

#[derive(Debug)]
pub struct ModelGroup {
    pub models: Vec<Model>,
    pub materials: Vec<Material>,
    pub textures: Vec<ImageTexture>,
}

// Start using this for xc3_wgpu loading?
// TODO: create a map module for loading Vec<Model> from props, maps, etc?
// TODO: Handle materials later?
#[derive(Debug)]
pub struct Model {
    pub meshes: Vec<Mesh>,
    pub vertex_buffers: Vec<VertexBuffer>,
    pub index_buffers: Vec<IndexBuffer>,
    pub instances: Vec<Mat4>,
}

#[derive(Debug)]

pub struct Mesh {
    pub vertex_buffer_index: usize,
    pub index_buffer_index: usize,
    pub material_index: usize,
}

#[derive(Debug)]

pub struct Material {
    name: String,
    // TODO: What to store here?
}

#[derive(Debug)]
pub struct VertexBuffer {
    pub attributes: Vec<AttributeData>,
}

#[derive(Debug)]
pub struct IndexBuffer {
    // TODO: support u32?
    pub indices: Vec<u16>,
}

impl Model {
    pub fn from_model(
        model: &xc3_lib::mxmd::Model,
        vertex_data: &xc3_lib::vertex::VertexData,
        instances: Vec<Mat4>,
    ) -> Self {
        let meshes = model
            .meshes
            .iter()
            .map(|mesh| Mesh {
                vertex_buffer_index: mesh.vertex_buffer_index as usize,
                index_buffer_index: mesh.index_buffer_index as usize,
                material_index: mesh.material_index as usize,
            })
            .collect();

        let vertex_buffers = read_vertex_buffers(vertex_data);
        let index_buffers = read_index_buffers(vertex_data);

        Self {
            meshes,
            vertex_buffers,
            index_buffers,
            instances,
        }
    }
}
