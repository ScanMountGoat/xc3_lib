use std::{collections::BTreeMap, path::Path};

use crate::{
    texture::load_textures,
    vertex::{read_indices, read_vertex_buffers, AttributeData},
};
use glam::Vec4Swizzles;
use gltf::json::validation::Checked::Valid;
use xc3_lib::{msrd::Msrd, mxmd::Mxmd};
use xc3_shader::gbuffer_database::GBufferDatabase;

type GltfAttributes = BTreeMap<
    gltf::json::validation::Checked<gltf::Semantic>,
    gltf::json::Index<gltf::json::Accessor>,
>;

/// Data associated with a [VertexData](xc3_lib::vertex::VertexData).
struct Buffers {
    buffer: gltf::json::Buffer,
    buffer_bytes: Vec<u8>,
    buffer_views: Vec<gltf::json::buffer::View>,
    accessors: Vec<gltf::json::Accessor>,

    vertex_buffer_attributes: Vec<GltfAttributes>,
    // Mapping from buffer index to accessor index.
    index_buffer_accessors: Vec<usize>,
}

// TODO: Take xc3_model types directly?
pub fn export_gltf<P: AsRef<Path>>(
    path: P,
    mxmd: &Mxmd,
    msrd: &Msrd,
    model_name: &str,
    m_tex_folder: &Path,
    h_tex_folder: &Path,
    database: &GBufferDatabase,
) {
    let mibls = load_textures(msrd, mxmd, m_tex_folder, h_tex_folder);
    // TODO: Is it worth giving images their in game names?
    let mut png_images: Vec<_> = mibls
        .iter()
        .map(|texture| {
            // Convert to PNG since DDS is not well supported.
            let dds = texture.to_dds().unwrap();
            image_dds::image_from_dds(&dds, 0).unwrap()
        })
        .collect();

    let textures = (0..mibls.len())
        .map(|i| gltf::json::Texture {
            name: None,
            sampler: None,
            source: gltf::json::Index::new(i as u32),
            extensions: None,
            extras: Default::default(),
        })
        .collect();

    // TODO: These need to be made while creating
    let images = (0..mibls.len())
        .map(|i| gltf::json::Image {
            buffer_view: None,
            mime_type: None,
            name: None,
            uri: Some(format!("model{i}.png")),
            extensions: None,
            extras: Default::default(),
        })
        .collect();

    let materials: Vec<_> = mxmd
        .materials
        .materials
        .iter()
        .map(|material| {
            let program_index = material.shader_programs[0].program_index as usize;
            let programs = database.files.get(model_name).map(|f| &f.programs);
            let shader = programs
                .and_then(|programs| programs.get(program_index))
                .map(|program| &program.shaders[0]);

            // TODO: A proper solution will construct each channel individually.
            // Assume the texture is used for all channels for now.
            // TODO: Create consts for the gbuffer texture indices?
            let albedo_index = texture_index(shader, material, 0, 'x');
            let normal_index = texture_index(shader, material, 2, 'x');

            // Reconstruct the normal map Z channel.
            // TODO: Cache already processed textures?
            // TODO: Cache by program index, usage (albedo vs normal), input samplers and channels?
            // TODO: handle the case where each channel has a different resolution?
            if let Some(index) = normal_index {
                for pixel in png_images[index as usize].pixels_mut() {
                    // x^y + y^2 + z^2 = 1 for unit vectors.
                    let x = (pixel[0] as f32 / 255.0) * 2.0 - 1.0;
                    let y = (pixel[1] as f32 / 255.0) * 2.0 - 1.0;
                    let z = 1.0 - x * x - y * y;
                    pixel[2] = (z * 255.0) as u8;
                }
            }

            gltf::json::Material {
                name: Some(material.name.clone()),
                pbr_metallic_roughness: gltf::json::material::PbrMetallicRoughness {
                    base_color_texture: albedo_index.map(|i| gltf::json::texture::Info {
                        index: gltf::json::Index::new(i),
                        tex_coord: 0,
                        extensions: None,
                        extras: Default::default(),
                    }),
                    metallic_factor: gltf::json::material::StrengthFactor(0.0),
                    roughness_factor: gltf::json::material::StrengthFactor(0.5),
                    // TODO: metalness in B channel and roughness in G channel?
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
        vertex_buffer_attributes,
        index_buffer_accessors,
    } = create_buffers(vertex_data, buffer_name.clone());

    // TODO: select by LOD and skip outline meshes?
    let meshes: Vec<_> = mxmd
        .models
        .models
        .iter()
        .flat_map(|model| {
            model.meshes.iter().map(|mesh| {
                let attributes =
                    vertex_buffer_attributes[mesh.vertex_buffer_index as usize].clone();

                let index_accessor =
                    index_buffer_accessors[mesh.index_buffer_index as usize] as u32;

                let primitive = gltf::json::mesh::Primitive {
                    // TODO: Store this with the buffers?
                    attributes,
                    extensions: Default::default(),
                    extras: Default::default(),
                    indices: Some(gltf::json::Index::new(index_accessor)),
                    material: Some(gltf::json::Index::new(mesh.material_index as u32)),
                    mode: Valid(gltf::json::mesh::Mode::Triangles),
                    targets: None,
                };

                // Assign one primitive per mesh to create distinct objects in applications.
                // In game meshes aren't named, so just use the material name.
                gltf::json::Mesh {
                    extensions: Default::default(),
                    extras: Default::default(),
                    name: materials[mesh.material_index as usize].name.clone(),
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
        textures,
        images,
        ..Default::default()
    };

    // TODO: Make returning and writing the data separate functions.
    let writer = std::fs::File::create(path.as_ref()).unwrap();
    gltf::json::serialize::to_writer_pretty(writer, &root).unwrap();

    std::fs::write(path.as_ref().with_file_name(buffer_name), buffer_bytes).unwrap();

    for (i, image) in png_images.iter().enumerate() {
        image.save(format!("model{i}.png")).unwrap();
    }
}

fn texture_index(
    shader: Option<&xc3_shader::gbuffer_database::Shader>,
    material: &xc3_lib::mxmd::Material,
    gbuffer_index: usize,
    channel: char,
) -> Option<u32> {
    // Find the sampler from the material.
    let (sampler_index, _) = shader?.material_channel_assignment(gbuffer_index, channel)?;

    // Find the texture referenced by this sampler.
    material
        .textures
        .get(sampler_index as usize)
        .map(|t| t.texture_index as u32)
}

fn create_buffers(vertex_data: xc3_lib::vertex::VertexData, buffer_name: String) -> Buffers {
    let mut buffer_bytes = Vec::new();
    let mut buffer_views = Vec::new();
    let mut accessors = Vec::new();
    let mut vertex_buffer_attributes = Vec::new();
    let mut index_buffer_accessors = Vec::new();

    // TODO: Handle the weight buffers separately?
    let vertex_buffers = read_vertex_buffers(&vertex_data);

    for vertex_buffer in vertex_buffers {
        let mut attributes = BTreeMap::new();
        for attribute in &vertex_buffer.attributes {
            match attribute {
                AttributeData::Position(values) => {
                    add_attribute_values(
                        values,
                        gltf::Semantic::Positions,
                        gltf::json::accessor::Type::Vec3,
                        &mut buffer_bytes,
                        &mut buffer_views,
                        &mut attributes,
                        &mut accessors,
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
                        &mut buffer_bytes,
                        &mut buffer_views,
                        &mut attributes,
                        &mut accessors,
                    );
                }
                AttributeData::Tangent(values) => {
                    // TODO: do these values need to be scaled/normalized?
                    add_attribute_values(
                        values,
                        gltf::Semantic::Tangents,
                        gltf::json::accessor::Type::Vec4,
                        &mut buffer_bytes,
                        &mut buffer_views,
                        &mut attributes,
                        &mut accessors,
                    );
                }
                AttributeData::Uv1(values) => {
                    add_attribute_values(
                        values,
                        gltf::Semantic::TexCoords(0),
                        gltf::json::accessor::Type::Vec2,
                        &mut buffer_bytes,
                        &mut buffer_views,
                        &mut attributes,
                        &mut accessors,
                    );
                }
                AttributeData::Uv2(values) => {
                    add_attribute_values(
                        values,
                        gltf::Semantic::TexCoords(1),
                        gltf::json::accessor::Type::Vec2,
                        &mut buffer_bytes,
                        &mut buffer_views,
                        &mut attributes,
                        &mut accessors,
                    );
                }
                AttributeData::VertexColor(values) => {
                    add_attribute_values(
                        values,
                        gltf::Semantic::Colors(0),
                        gltf::json::accessor::Type::Vec4,
                        &mut buffer_bytes,
                        &mut buffer_views,
                        &mut attributes,
                        &mut accessors,
                    );
                }
                AttributeData::WeightIndex(_) => (),
            }
        }

        vertex_buffer_attributes.push(attributes);
    }

    // Place indices after the vertices to use a single buffer.
    // TODO: Alignment?
    for index_buffer in &vertex_data.index_buffers {
        let indices = read_indices(index_buffer, &vertex_data.buffer);
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
        vertex_buffer_attributes,
        index_buffer_accessors,
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
    // TODO: Make this a generic function?
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
