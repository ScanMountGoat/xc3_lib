use std::path::Path;

use crate::vertex::{read_indices, read_vertices, Vertex};
use glam::Vec3;
use gltf::json::validation::Checked::Valid;
use xc3_lib::msrd::Msrd;

pub fn export_gltf<P: AsRef<Path>>(path: P, msrd: &Msrd) {
    let model_name = path
        .as_ref()
        .with_extension("")
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string();

    let model_data = msrd.extract_model_data();

    // TODO: Create nodes and meshes for each mesh in the mxmd.
    let vertices = read_vertices(&model_data.vertex_buffers[0], 0, &model_data);
    let vertex_bytes: &[u8] = bytemuck::cast_slice(&vertices);

    let indices = read_indices(&model_data, &model_data.index_buffers[0]);
    let index_bytes: &[u8] = bytemuck::cast_slice(&indices);

    let mut combined_buffer = vertex_bytes.to_vec();
    combined_buffer.extend_from_slice(index_bytes);

    let min: [f32; 3] = vertices
        .iter()
        .map(|v| v.position)
        .reduce(Vec3::min)
        .unwrap()
        .into();
    let max: [f32; 3] = vertices
        .iter()
        .map(|v| v.position)
        .reduce(Vec3::max)
        .unwrap()
        .into();

    let buffer_name = format!("{model_name}.buffer0.bin");
    let buffer = gltf::json::Buffer {
        byte_length: combined_buffer.len() as u32,
        extensions: Default::default(),
        extras: Default::default(),
        name: None,
        uri: Some(buffer_name.clone()),
    };

    // Place the indices after the vertices to use a single buffer.
    let buffer_view = gltf::json::buffer::View {
        buffer: gltf::json::Index::new(0),
        byte_length: buffer.byte_length,
        byte_offset: None,
        byte_stride: Some(std::mem::size_of::<Vertex>() as u32),
        extensions: Default::default(),
        extras: Default::default(),
        name: None,
        target: Some(Valid(gltf::json::buffer::Target::ArrayBuffer)),
    };
    let index_buffer_view = gltf::json::buffer::View {
        buffer: gltf::json::Index::new(0),
        byte_length: index_bytes.len() as u32,
        byte_offset: Some(vertex_bytes.len() as u32),
        byte_stride: None,
        extensions: Default::default(),
        extras: Default::default(),
        name: None,
        target: Some(Valid(gltf::json::buffer::Target::ElementArrayBuffer)),
    };

    let positions = gltf::json::Accessor {
        buffer_view: Some(gltf::json::Index::new(0)),
        byte_offset: 0,
        count: vertices.len() as u32,
        component_type: Valid(gltf::json::accessor::GenericComponentType(
            gltf::json::accessor::ComponentType::F32,
        )),
        extensions: Default::default(),
        extras: Default::default(),
        type_: Valid(gltf::json::accessor::Type::Vec3),
        min: Some(gltf::json::Value::from(Vec::from(min))),
        max: Some(gltf::json::Value::from(Vec::from(max))),
        name: None,
        normalized: false,
        sparse: None,
    };
    let indices = gltf::json::Accessor {
        buffer_view: Some(gltf::json::Index::new(1)),
        byte_offset: 0,
        count: indices.len() as u32,
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

    let primitive = gltf::json::mesh::Primitive {
        attributes: [(
            Valid(gltf::json::mesh::Semantic::Positions),
            gltf::json::Index::new(0),
        )]
        .into(),
        extensions: Default::default(),
        extras: Default::default(),
        indices: Some(gltf::json::Index::new(1)),
        material: None,
        mode: Valid(gltf::json::mesh::Mode::Triangles),
        targets: None,
    };

    let mesh = gltf::json::Mesh {
        extensions: Default::default(),
        extras: Default::default(),
        name: None,
        primitives: vec![primitive],
        weights: None,
    };

    let node = gltf::json::Node {
        camera: None,
        children: None,
        extensions: Default::default(),
        extras: Default::default(),
        matrix: None,
        mesh: Some(gltf::json::Index::new(0)),
        name: None,
        rotation: None,
        scale: None,
        translation: None,
        skin: None,
        weights: None,
    };

    let root = gltf::json::Root {
        accessors: vec![positions, indices],
        buffers: vec![buffer],
        buffer_views: vec![buffer_view, index_buffer_view],
        meshes: vec![mesh],
        nodes: vec![node],
        scenes: vec![gltf::json::Scene {
            extensions: Default::default(),
            extras: Default::default(),
            name: None,
            nodes: vec![gltf::json::Index::new(0)],
        }],
        ..Default::default()
    };

    // TODO: Make returning and writing the data separate functions.
    let writer = std::fs::File::create(path.as_ref()).unwrap();
    gltf::json::serialize::to_writer_pretty(writer, &root).unwrap();

    std::fs::write(path.as_ref().with_file_name(buffer_name), &combined_buffer).unwrap();
}
