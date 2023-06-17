use std::path::Path;

use crate::vertex::{read_indices, read_vertices, Vertex};
use gltf::json::validation::Checked::Valid;
use xc3_lib::{msrd::Msrd, mxmd::Mxmd};

/// Data associated with a [VertexData](xc3_lib::vertex::VertexData).
struct Buffers {
    buffer: gltf::json::Buffer,
    buffer_bytes: Vec<u8>,
    buffer_views: Vec<gltf::json::buffer::View>,
    accessors: Vec<gltf::json::Accessor>,

    // Mapping from buffer indices to accessor indices.
    vertex_buffer_accessors: Vec<VertexAccessors>,
    index_buffer_accessors: Vec<usize>,
}

struct VertexAccessors {
    position_index: usize,
    normal_index: usize,
    uv1_index: usize,
}

// TODO: Take models, materials, and vertex data directly?
pub fn export_gltf<P: AsRef<Path>>(path: P, mxmd: &Mxmd, msrd: &Msrd) {
    let materials = mxmd
        .materials
        .materials
        .elements
        .iter()
        .map(|material|
            // TODO: Assign textures using gbuffer database.
            // TODO: Can texture assignment code be shared with xc3_wgpu?
            // TODO: database -> shader -> sampler name -> sampler index -> texture file name.
            // TODO: Automatically handle channels by decoding to PNG?
            gltf::json::Material {
            name: Some(material.name.clone()),
            pbr_metallic_roughness: gltf::json::material::PbrMetallicRoughness {
                ..Default::default()
            },
            normal_texture: None,
            ..Default::default()
        })
        .collect();

    let model_name = path
        .as_ref()
        .with_extension("")
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string();

    let vertex_data = msrd.extract_vertex_data();

    // TODO: Create nodes and meshes for each mesh in the mxmd.
    let buffer_name = format!("{model_name}.buffer0.bin");

    let Buffers {
        buffer,
        buffer_bytes,
        buffer_views,
        accessors,
        vertex_buffer_accessors,
        index_buffer_accessors,
    } = create_buffers(vertex_data, buffer_name.clone());

    // TODO: select by LOD and skip outline meshes?
    let meshes: Vec<_> = mxmd
        .models
        .models
        .elements
        .iter()
        .flat_map(|model| {
            model.meshes.iter().map(|mesh| {
                let vertex_accessors = &vertex_buffer_accessors[mesh.vertex_buffer_index as usize];

                let index_accessor =
                    index_buffer_accessors[mesh.index_buffer_index as usize] as u32;

                let primitive = gltf::json::mesh::Primitive {
                    attributes: [
                        (
                            Valid(gltf::json::mesh::Semantic::Positions),
                            gltf::json::Index::new(vertex_accessors.position_index as u32),
                        ),
                        (
                            Valid(gltf::json::mesh::Semantic::Normals),
                            gltf::json::Index::new(vertex_accessors.normal_index as u32),
                        ),
                        (
                            Valid(gltf::json::mesh::Semantic::TexCoords(0)),
                            gltf::json::Index::new(vertex_accessors.uv1_index as u32),
                        ),
                    ]
                    .into(),
                    extensions: Default::default(),
                    extras: Default::default(),
                    indices: Some(gltf::json::Index::new(index_accessor)),
                    material: Some(gltf::json::Index::new(mesh.material_index as u32)),
                    mode: Valid(gltf::json::mesh::Mode::Triangles),
                    targets: None,
                };

                // Assign one primitive per mesh to create distinct objects in applications.
                gltf::json::Mesh {
                    extensions: Default::default(),
                    extras: Default::default(),
                    name: None,
                    primitives: vec![primitive],
                    weights: None,
                }
            })
        })
        .collect();

    // TODO: Instance transforms for stages?
    let nodes: Vec<_> = (0..meshes.len())
        .map(|i| {
            // Assume one gltf node per gltf mesh for now.
            gltf::json::Node {
                camera: None,
                children: None,
                extensions: Default::default(),
                extras: Default::default(),
                matrix: None,
                mesh: Some(gltf::json::Index::new(i as u32)),
                name: None,
                rotation: None,
                scale: None,
                translation: None,
                skin: None,
                weights: None,
            }
        })
        .collect();

    // TODO: Should all nodes be used like this?
    let scene_nodes = (0..nodes.len())
        .map(|i| gltf::json::Index::new(i as u32))
        .collect();

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
        ..Default::default()
    };

    // TODO: Make returning and writing the data separate functions.
    let writer = std::fs::File::create(path.as_ref()).unwrap();
    gltf::json::serialize::to_writer_pretty(writer, &root).unwrap();

    std::fs::write(path.as_ref().with_file_name(buffer_name), buffer_bytes).unwrap();
}

fn create_buffers(vertex_data: xc3_lib::vertex::VertexData, buffer_name: String) -> Buffers {
    let mut buffer_bytes = Vec::new();
    let mut buffer_views = Vec::new();
    let mut accessors = Vec::new();
    let mut vertex_buffer_accessors = Vec::new();
    let mut index_buffer_accessors = Vec::new();

    // TODO: Handle the weight buffers separately?
    for (i, vertex_buffer) in vertex_data.vertex_buffers.iter().enumerate() {
        let vertices = read_vertices(vertex_buffer, i, &vertex_data);
        let vertex_bytes: &[u8] = bytemuck::cast_slice(&vertices);

        // Assume everything uses the same buffer for now.
        // TODO: Stride can be greater than length?
        let view = gltf::json::buffer::View {
            buffer: gltf::json::Index::new(0),
            byte_length: vertex_bytes.len() as u32,
            byte_offset: Some(buffer_bytes.len() as u32),
            byte_stride: Some(std::mem::size_of::<Vertex>() as u32),
            extensions: Default::default(),
            extras: Default::default(),
            name: None,
            target: Some(Valid(gltf::json::buffer::Target::ArrayBuffer)),
        };

        // Vertices are already in a unified format, so the offsets are known.
        // TODO: use memoffset here?
        let positions = gltf::json::Accessor {
            buffer_view: Some(gltf::json::Index::new(buffer_views.len() as u32)),
            byte_offset: 0,
            count: vertices.len() as u32,
            component_type: Valid(gltf::json::accessor::GenericComponentType(
                gltf::json::accessor::ComponentType::F32,
            )),
            extensions: Default::default(),
            extras: Default::default(),
            type_: Valid(gltf::json::accessor::Type::Vec3),
            min: None,
            max: None,
            name: None,
            normalized: false,
            sparse: None,
        };
        let position_index = accessors.len();
        accessors.push(positions);

        let normals = gltf::json::Accessor {
            buffer_view: Some(gltf::json::Index::new(buffer_views.len() as u32)),
            byte_offset: 32,
            count: vertices.len() as u32,
            component_type: Valid(gltf::json::accessor::GenericComponentType(
                gltf::json::accessor::ComponentType::F32,
            )),
            extensions: Default::default(),
            extras: Default::default(),
            type_: Valid(gltf::json::accessor::Type::Vec3),
            min: None,
            max: None,
            name: None,
            normalized: false,
            sparse: None,
        };
        let normal_index = accessors.len();
        accessors.push(normals);

        let uv1 = gltf::json::Accessor {
            buffer_view: Some(gltf::json::Index::new(buffer_views.len() as u32)),
            byte_offset: 64,
            count: vertices.len() as u32,
            component_type: Valid(gltf::json::accessor::GenericComponentType(
                gltf::json::accessor::ComponentType::F32,
            )),
            extensions: Default::default(),
            extras: Default::default(),
            type_: Valid(gltf::json::accessor::Type::Vec2),
            min: None,
            max: None,
            name: None,
            normalized: false,
            sparse: None,
        };
        let uv1_index = accessors.len();
        accessors.push(uv1);

        vertex_buffer_accessors.push(VertexAccessors {
            position_index,
            normal_index,
            uv1_index,
        });

        buffer_views.push(view);
        buffer_bytes.extend_from_slice(vertex_bytes);
    }

    // Place indices after the vertices to use a single buffer.
    // TODO: Alignment?
    for index_buffer in &vertex_data.index_buffers {
        let indices = read_indices(&vertex_data, index_buffer);
        let index_bytes: &[u8] = bytemuck::cast_slice(&indices);

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
        index_buffer_accessors.push(accessors.len());

        accessors.push(indices);
        buffer_views.push(view);
        buffer_bytes.extend_from_slice(index_bytes);
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
        vertex_buffer_accessors,
        index_buffer_accessors,
    }
}
