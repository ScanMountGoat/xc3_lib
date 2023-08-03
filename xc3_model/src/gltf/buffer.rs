use std::{
    collections::BTreeMap,
    io::{Cursor, Seek, Write},
};

use crate::{skinning::bone_indices_weights, vertex::AttributeData, ModelRoot};
use binrw::BinWrite;
use glam::{Mat4, Vec2, Vec3, Vec4, Vec4Swizzles};
use gltf::{
    buffer::Target,
    json::validation::Checked::{self, Valid},
};

type GltfAttributes = BTreeMap<
    gltf::json::validation::Checked<gltf::Semantic>,
    gltf::json::Index<gltf::json::Accessor>,
>;

// gltf stores flat lists of attributes and accessors at the root level.
// Create mappings to properly differentiate models and groups.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct BufferKey {
    pub root_index: usize,
    pub group_index: usize,
    pub buffers_index: usize,
    pub buffer_index: usize,
}

// Combined vertex data for a gltf buffer.
pub struct Buffers {
    pub buffer_bytes: Vec<u8>,
    pub buffer_views: Vec<gltf::json::buffer::View>,
    pub accessors: Vec<gltf::json::Accessor>,
    // Map group and model specific indices to flattened indices.
    pub vertex_buffer_attributes: BTreeMap<BufferKey, GltfAttributes>,
    pub index_buffer_accessors: BTreeMap<BufferKey, usize>,
}

impl Buffers {
    pub fn new(roots: &[ModelRoot]) -> Self {
        let mut combined_buffers = Buffers {
            buffer_bytes: Vec::new(),
            buffer_views: Vec::new(),
            accessors: Vec::new(),
            vertex_buffer_attributes: BTreeMap::new(),
            index_buffer_accessors: BTreeMap::new(),
        };

        for (root_index, root) in roots.iter().enumerate() {
            for (group_index, group) in root.groups.iter().enumerate() {
                for (buffers_index, buffers) in group.buffers.iter().enumerate() {
                    // TODO: How to handle buffers shared between multiple skeletons?
                    combined_buffers.add_vertex_buffers(
                        buffers,
                        group.models.first().and_then(|m| m.skeleton.as_ref()),
                        root_index,
                        group_index,
                        buffers_index,
                    );

                    // Place indices after the vertices to use a single buffer.
                    // TODO: Alignment?
                    combined_buffers.add_index_buffers(buffers, root_index, group_index, buffers_index);
                }
            }
        }

        combined_buffers
    }

    fn add_vertex_buffers(
        &mut self,
        buffers: &crate::ModelBuffers,
        skeleton: Option<&crate::skeleton::Skeleton>,
        root_index: usize,
        group_index: usize,
        buffers_index: usize,
    ) {
        for (i, vertex_buffer) in buffers.vertex_buffers.iter().enumerate() {
            let mut attributes = BTreeMap::new();
            for attribute in &vertex_buffer.attributes {
                match attribute {
                    AttributeData::Position(values) => {
                        self.add_attribute_values(
                            values,
                            gltf::Semantic::Positions,
                            gltf::json::accessor::Type::Vec3,
                            gltf::json::accessor::ComponentType::F32,
                            Some(Valid(Target::ArrayBuffer)),
                            &mut attributes,
                        );
                    }
                    AttributeData::Normal(values) => {
                        // Not all applications will normalize the vertex normals.
                        // Use Vec3 instead of Vec4 since it's better supported.
                        let values: Vec<_> = values.iter().map(|v| v.xyz().normalize()).collect();
                        self.add_attribute_values(
                            &values,
                            gltf::Semantic::Normals,
                            gltf::json::accessor::Type::Vec3,
                            gltf::json::accessor::ComponentType::F32,
                            Some(Valid(Target::ArrayBuffer)),
                            &mut attributes,
                        );
                    }
                    AttributeData::Tangent(values) => {
                        // TODO: do these values need to be scaled/normalized?
                        self.add_attribute_values(
                            values,
                            gltf::Semantic::Tangents,
                            gltf::json::accessor::Type::Vec4,
                            gltf::json::accessor::ComponentType::F32,
                            Some(Valid(Target::ArrayBuffer)),
                            &mut attributes,
                        );
                    }
                    AttributeData::Uv1(values) => {
                        self.add_attribute_values(
                            values,
                            gltf::Semantic::TexCoords(0),
                            gltf::json::accessor::Type::Vec2,
                            gltf::json::accessor::ComponentType::F32,
                            Some(Valid(Target::ArrayBuffer)),
                            &mut attributes,
                        );
                    }
                    AttributeData::Uv2(values) => {
                        self.add_attribute_values(
                            values,
                            gltf::Semantic::TexCoords(1),
                            gltf::json::accessor::Type::Vec2,
                            gltf::json::accessor::ComponentType::F32,
                            Some(Valid(Target::ArrayBuffer)),
                            &mut attributes,
                        );
                    }
                    AttributeData::VertexColor(values) => {
                        self.add_attribute_values(
                            values,
                            gltf::Semantic::Colors(0),
                            gltf::json::accessor::Type::Vec4,
                            gltf::json::accessor::ComponentType::F32,
                            Some(Valid(Target::ArrayBuffer)),
                            &mut attributes,
                        );
                    }
                    // Skin weights are handled separately.
                    // TODO: remove these attributes?
                    AttributeData::WeightIndex(_) => (),
                    AttributeData::SkinWeights(_) => (),
                    AttributeData::BoneIndices(_) => (),
                }
            }

            if let Some(skeleton) = skeleton {
                // TODO: Avoid collect?
                let bone_names: Vec<_> = skeleton.bones.iter().map(|b| &b.name).collect();

                let (indices, weights) = bone_indices_weights(
                    &vertex_buffer.influences,
                    vertex_buffer.attributes[0].len(),
                    &bone_names,
                );

                self.add_attribute_values(
                    &weights,
                    gltf::Semantic::Weights(0),
                    gltf::json::accessor::Type::Vec4,
                    gltf::json::accessor::ComponentType::F32,
                    Some(Valid(Target::ArrayBuffer)),
                    &mut attributes,
                );
                self.add_attribute_values(
                    &indices,
                    gltf::Semantic::Joints(0),
                    gltf::json::accessor::Type::Vec4,
                    gltf::json::accessor::ComponentType::U8,
                    Some(Valid(Target::ArrayBuffer)),
                    &mut attributes,
                );
            }

            self.vertex_buffer_attributes.insert(
                BufferKey {
                    root_index,
                    group_index,
                    buffers_index,
                    buffer_index: i,
                },
                attributes,
            );
        }
    }

    fn add_index_buffers(
        &mut self,
        buffers: &crate::ModelBuffers,
        root_index: usize,
        group_index: usize,
        buffers_index: usize,
    ) {
        for (i, index_buffer) in buffers.index_buffers.iter().enumerate() {
            let index_bytes = write_bytes(&index_buffer.indices);

            // Assume everything uses the same buffer for now.
            let view = gltf::json::buffer::View {
                buffer: gltf::json::Index::new(0),
                byte_length: index_bytes.len() as u32,
                byte_offset: Some(self.buffer_bytes.len() as u32),
                byte_stride: None,
                extensions: Default::default(),
                extras: Default::default(),
                name: None,
                target: Some(Valid(gltf::json::buffer::Target::ElementArrayBuffer)),
            };

            let indices = gltf::json::Accessor {
                buffer_view: Some(gltf::json::Index::new(self.buffer_views.len() as u32)),
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
            self.index_buffer_accessors.insert(
                BufferKey {
                    root_index,
                    group_index,
                    buffers_index,
                    buffer_index: i,
                },
                self.accessors.len(),
            );

            self.accessors.push(indices);
            self.buffer_views.push(view);
            self.buffer_bytes.extend_from_slice(&index_bytes);
        }
    }

    pub fn add_attribute_values<T: WriteBytes>(
        &mut self,
        values: &[T],
        semantic: gltf::Semantic,
        components: gltf::json::accessor::Type,
        component_type: gltf::json::accessor::ComponentType,
        target: Option<Checked<Target>>,
        attributes: &mut GltfAttributes,
    ) {
        let index = self.add_values(values, components, component_type, target);

        // Assume the buffer has only one of each attribute semantic.
        attributes.insert(Valid(semantic), index);
    }

    pub fn add_values<T: WriteBytes>(
        &mut self,
        values: &[T],
        components: gltf::json::accessor::Type,
        component_type: gltf::json::accessor::ComponentType,
        target: Option<Checked<Target>>,
    ) -> gltf::json::Index<gltf::json::Accessor> {
        let attribute_bytes = write_bytes(values);

        // Assume everything uses the same buffer for now.
        // Each attribute is in its own section and thus has its own view.
        let view = gltf::json::buffer::View {
            buffer: gltf::json::Index::new(0),
            byte_length: attribute_bytes.len() as u32,
            byte_offset: Some(self.buffer_bytes.len() as u32),
            byte_stride: Some(std::mem::size_of::<T>() as u32),
            extensions: Default::default(),
            extras: Default::default(),
            name: None,
            target,
        };
        self.buffer_bytes.extend_from_slice(&attribute_bytes);

        // TODO: min/max for positions.
        let accessor = gltf::json::Accessor {
            buffer_view: Some(gltf::json::Index::new(self.buffer_views.len() as u32)),
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

        let index = gltf::json::Index::new(self.accessors.len() as u32);

        self.accessors.push(accessor);
        self.buffer_views.push(view);

        index
    }
}

// gltf requires little endian for byte buffers.
// Create a trait instead of using bytemuck.
pub trait WriteBytes {
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
