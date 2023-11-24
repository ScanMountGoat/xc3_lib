use std::{
    collections::BTreeMap,
    io::{Cursor, Seek, Write},
};

use crate::{vertex::AttributeData, ModelRoot};
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
type GltfAttribute = (
    gltf::json::validation::Checked<gltf::Semantic>,
    gltf::json::Index<gltf::json::Accessor>,
);

// gltf stores flat lists of attributes and accessors at the root level.
// Create mappings to properly differentiate models and groups.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct BufferKey {
    pub root_index: usize,
    pub group_index: usize,
    pub buffers_index: usize,
    /// Vertex or index buffer index.
    pub buffer_index: usize,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct WeightGroupKey {
    pub weight_group_index: Option<usize>,
    pub buffer: BufferKey,
}

// Combined vertex data for a gltf buffer.
#[derive(Default)]
pub struct Buffers {
    pub buffer_bytes: Vec<u8>,
    pub buffer_views: Vec<gltf::json::buffer::View>,
    pub accessors: Vec<gltf::json::Accessor>,

    pub vertex_buffer_attributes: BTreeMap<BufferKey, GltfAttributes>,
    pub morph_targets: BTreeMap<BufferKey, Vec<GltfAttributes>>,
    pub index_buffer_accessors: BTreeMap<BufferKey, usize>,
    pub weight_groups: BTreeMap<WeightGroupKey, WeightGroup>,
}

pub struct WeightGroup {
    pub weights: GltfAttribute,
    pub indices: GltfAttribute,
}

impl Buffers {
    pub fn new(roots: &[ModelRoot]) -> Self {
        let mut combined_buffers = Buffers::default();

        for (root_index, root) in roots.iter().enumerate() {
            for (group_index, group) in root.groups.iter().enumerate() {
                for (buffers_index, buffers) in group.buffers.iter().enumerate() {
                    // TODO: How to handle buffers shared between multiple skeletons?
                    combined_buffers.add_vertex_buffers(
                        buffers,
                        root_index,
                        group_index,
                        buffers_index,
                    );

                    // Place indices after the vertices to use a single buffer.
                    // TODO: Alignment?
                    combined_buffers.add_index_buffers(
                        buffers,
                        root_index,
                        group_index,
                        buffers_index,
                    );
                }
            }
        }

        combined_buffers
    }

    fn add_vertex_buffers(
        &mut self,
        buffers: &crate::ModelBuffers,
        root_index: usize,
        group_index: usize,
        buffers_index: usize,
    ) {
        for (i, vertex_buffer) in buffers.vertex_buffers.iter().enumerate() {
            // Assume the base morph target is already applied.
            let mut attributes = BTreeMap::new();
            self.add_attributes(&mut attributes, &vertex_buffer.attributes);

            // Morph targets have their own attribute data.
            if !vertex_buffer.morph_targets.is_empty() {
                let targets: Vec<_> = vertex_buffer
                    .morph_targets
                    .iter()
                    .map(|target| {
                        // Convert from a sparse to a dense representation.
                        let vertex_count = vertex_buffer.attributes[0].len();
                        let mut position_deltas = vec![Vec3::ZERO; vertex_count];
                        let mut normal_deltas = vec![Vec3::ZERO; vertex_count];
                        let mut tangent_deltas = vec![Vec3::ZERO; vertex_count];
                        for (i, vertex_index) in target.vertex_indices.iter().enumerate() {
                            position_deltas[*vertex_index as usize] = target.position_deltas[i];
                            normal_deltas[*vertex_index as usize] = target.normal_deltas[i].xyz();
                            tangent_deltas[*vertex_index as usize] = target.tangent_deltas[i].xyz();
                        }

                        // glTF morph targets are defined as a difference with the base target.
                        let mut attributes = attributes.clone();
                        self.insert_attribute_values(
                            &position_deltas,
                            gltf::Semantic::Positions,
                            gltf::json::accessor::Type::Vec3,
                            gltf::json::accessor::ComponentType::F32,
                            Some(Valid(Target::ArrayBuffer)),
                            &mut attributes,
                        );

                        // Normals and tangents also use deltas.
                        // These should use Vec3 to avoid displacing the sign in tangent.w.
                        self.insert_attribute_values(
                            &normal_deltas,
                            gltf::Semantic::Normals,
                            gltf::json::accessor::Type::Vec3,
                            gltf::json::accessor::ComponentType::F32,
                            Some(Valid(Target::ArrayBuffer)),
                            &mut attributes,
                        );

                        self.insert_attribute_values(
                            &tangent_deltas,
                            gltf::Semantic::Tangents,
                            gltf::json::accessor::Type::Vec3,
                            gltf::json::accessor::ComponentType::F32,
                            Some(Valid(Target::ArrayBuffer)),
                            &mut attributes,
                        );

                        attributes
                    })
                    .collect();

                self.morph_targets.insert(
                    BufferKey {
                        root_index,
                        group_index,
                        buffers_index,
                        buffer_index: i,
                    },
                    targets,
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

    pub fn get_weight_group_lazy(
        &mut self,
        buffers: &crate::ModelBuffers,
        skeleton: Option<&crate::Skeleton>,
        key: WeightGroupKey,
    ) -> Option<&WeightGroup> {
        if !self.weight_groups.contains_key(&key) {
            if let Some(skeleton) = skeleton {
                if let Some(weights) = &buffers.weights {
                    let vertex_buffer = &buffers.vertex_buffers[key.buffer.buffer_index];
                    if let Some(weight_indices) =
                        vertex_buffer.attributes.iter().find_map(|a| match a {
                            AttributeData::WeightIndex(indices) => Some(indices),
                            _ => None,
                        })
                    {
                        let weight_group = self.add_weight_group(
                            skeleton,
                            weights,
                            weight_indices,
                            key.weight_group_index,
                        );
                        self.weight_groups.insert(key, weight_group);
                    }
                }
            }
        }

        self.weight_groups.get(&key)
    }

    fn add_weight_group(
        &mut self,
        skeleton: &crate::Skeleton,
        weights: &crate::Weights,
        weight_indices: &[u32],
        weight_group_index: Option<usize>,
    ) -> WeightGroup {
        // The weights may be defined with a different bone ordering.
        let bone_names: Vec<_> = skeleton.bones.iter().map(|b| b.name.clone()).collect();
        let skin_weights = weights.skin_weights.reindex_bones(bone_names);

        // Each group has a different starting offset.
        // This needs to be applied during reindexing.
        // No offset is needed if no groups are assigned.
        let starting_index = weight_group_index
            .and_then(|i| weights.weight_groups.get(i).map(|g| g.input_start_index))
            .unwrap_or_default();
        let skin_weights = skin_weights.reindex(weight_indices, starting_index);

        let weights_accessor = self.add_values(
            &skin_weights.weights,
            gltf::json::accessor::Type::Vec4,
            gltf::json::accessor::ComponentType::F32,
            Some(Valid(Target::ArrayBuffer)),
        );
        let indices_accessor = self.add_values(
            &skin_weights.bone_indices,
            gltf::json::accessor::Type::Vec4,
            gltf::json::accessor::ComponentType::U8,
            Some(Valid(Target::ArrayBuffer)),
        );

        WeightGroup {
            weights: (Valid(gltf::Semantic::Weights(0)), weights_accessor),
            indices: (Valid(gltf::Semantic::Joints(0)), indices_accessor),
        }
    }

    fn add_attributes(
        &mut self,
        attributes: &mut GltfAttributes,
        buffer_attributes: &[AttributeData],
    ) {
        for attribute in buffer_attributes {
            match attribute {
                AttributeData::Position(values) => {
                    self.insert_attribute_values(
                        values,
                        gltf::Semantic::Positions,
                        gltf::json::accessor::Type::Vec3,
                        gltf::json::accessor::ComponentType::F32,
                        Some(Valid(Target::ArrayBuffer)),
                        attributes,
                    );
                }
                AttributeData::Normal(values) => {
                    // Not all applications will normalize the vertex normals.
                    // Use Vec3 instead of Vec4 since it's better supported.
                    let values: Vec<_> = values.iter().map(|v| v.xyz().normalize()).collect();
                    self.insert_attribute_values(
                        &values,
                        gltf::Semantic::Normals,
                        gltf::json::accessor::Type::Vec3,
                        gltf::json::accessor::ComponentType::F32,
                        Some(Valid(Target::ArrayBuffer)),
                        attributes,
                    );
                }
                AttributeData::Tangent(values) => {
                    // TODO: do these values need to be scaled/normalized?
                    self.insert_attribute_values(
                        values,
                        gltf::Semantic::Tangents,
                        gltf::json::accessor::Type::Vec4,
                        gltf::json::accessor::ComponentType::F32,
                        Some(Valid(Target::ArrayBuffer)),
                        attributes,
                    );
                }
                AttributeData::TexCoord0(values) => {
                    self.insert_attribute_values(
                        values,
                        gltf::Semantic::TexCoords(0),
                        gltf::json::accessor::Type::Vec2,
                        gltf::json::accessor::ComponentType::F32,
                        Some(Valid(Target::ArrayBuffer)),
                        attributes,
                    );
                }
                AttributeData::TexCoord1(values) => {
                    self.insert_attribute_values(
                        values,
                        gltf::Semantic::TexCoords(1),
                        gltf::json::accessor::Type::Vec2,
                        gltf::json::accessor::ComponentType::F32,
                        Some(Valid(Target::ArrayBuffer)),
                        attributes,
                    );
                }
                AttributeData::TexCoord2(values) => {
                    self.insert_attribute_values(
                        values,
                        gltf::Semantic::TexCoords(2),
                        gltf::json::accessor::Type::Vec2,
                        gltf::json::accessor::ComponentType::F32,
                        Some(Valid(Target::ArrayBuffer)),
                        attributes,
                    );
                }
                AttributeData::TexCoord3(values) => {
                    self.insert_attribute_values(
                        values,
                        gltf::Semantic::TexCoords(3),
                        gltf::json::accessor::Type::Vec2,
                        gltf::json::accessor::ComponentType::F32,
                        Some(Valid(Target::ArrayBuffer)),
                        attributes,
                    );
                }
                AttributeData::TexCoord4(values) => {
                    self.insert_attribute_values(
                        values,
                        gltf::Semantic::TexCoords(4),
                        gltf::json::accessor::Type::Vec2,
                        gltf::json::accessor::ComponentType::F32,
                        Some(Valid(Target::ArrayBuffer)),
                        attributes,
                    );
                }
                AttributeData::TexCoord5(values) => {
                    self.insert_attribute_values(
                        values,
                        gltf::Semantic::TexCoords(5),
                        gltf::json::accessor::Type::Vec2,
                        gltf::json::accessor::ComponentType::F32,
                        Some(Valid(Target::ArrayBuffer)),
                        attributes,
                    );
                }
                AttributeData::TexCoord6(values) => {
                    self.insert_attribute_values(
                        values,
                        gltf::Semantic::TexCoords(6),
                        gltf::json::accessor::Type::Vec2,
                        gltf::json::accessor::ComponentType::F32,
                        Some(Valid(Target::ArrayBuffer)),
                        attributes,
                    );
                }
                AttributeData::TexCoord7(values) => {
                    self.insert_attribute_values(
                        values,
                        gltf::Semantic::TexCoords(7),
                        gltf::json::accessor::Type::Vec2,
                        gltf::json::accessor::ComponentType::F32,
                        Some(Valid(Target::ArrayBuffer)),
                        attributes,
                    );
                }
                AttributeData::TexCoord8(values) => {
                    self.insert_attribute_values(
                        values,
                        gltf::Semantic::TexCoords(8),
                        gltf::json::accessor::Type::Vec2,
                        gltf::json::accessor::ComponentType::F32,
                        Some(Valid(Target::ArrayBuffer)),
                        attributes,
                    );
                }
                AttributeData::VertexColor(values) => {
                    // TODO: Vertex color isn't always an RGB multiplier?
                    // Use a custom attribute to avoid rendering issues.
                    self.insert_attribute_values(
                        values,
                        gltf::Semantic::Extras("_Color".to_string()),
                        gltf::json::accessor::Type::Vec4,
                        gltf::json::accessor::ComponentType::F32,
                        Some(Valid(Target::ArrayBuffer)),
                        attributes,
                    );
                }
                AttributeData::Blend(values) => {
                    // Used for color blending for some stages.
                    self.insert_attribute_values(
                        values,
                        gltf::Semantic::Extras("Blend".to_string()),
                        gltf::json::accessor::Type::Vec4,
                        gltf::json::accessor::ComponentType::F32,
                        Some(Valid(Target::ArrayBuffer)),
                        attributes,
                    );
                }
                // Skin weights are handled separately.
                AttributeData::WeightIndex(_) => (),
                AttributeData::SkinWeights(_) => (),
                AttributeData::BoneIndices(_) => (),
            }
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
                byte_offset: Some(0),
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

    pub fn insert_attribute_values<T: WriteBytes>(
        &mut self,
        values: &[T],
        semantic: gltf::Semantic,
        components: gltf::json::accessor::Type,
        component_type: gltf::json::accessor::ComponentType,
        target: Option<Checked<Target>>,
        attributes: &mut GltfAttributes,
    ) {
        // Attributes should be non empty.
        if !values.is_empty() {
            let index = self.add_values(values, components, component_type, target);

            // Assume the buffer has only one of each attribute semantic.
            attributes.insert(Valid(semantic), index);
        }
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
            byte_offset: Some(0),
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
