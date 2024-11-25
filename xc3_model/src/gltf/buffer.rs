use std::{
    collections::BTreeMap,
    io::{Cursor, Seek, Write},
};

use crate::vertex::AttributeData;
use binrw::{BinResult, BinWrite};
use glam::{Mat4, Vec2, Vec3, Vec4, Vec4Swizzles};
use gltf::{
    buffer::Target,
    json::validation::Checked::{self, Valid},
};

use super::align_bytes;

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

// TODO: Use the start index to adjust the buffer offset instead?
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct WeightGroupKey {
    pub weights_start_index: usize,
    pub flags2: u32,
    pub buffer: BufferKey,
}

// Combined vertex data for a gltf buffer.
#[derive(Default, Clone)]
pub struct Buffers {
    pub buffer_bytes: Vec<u8>,
    pub buffer_views: Vec<gltf::json::buffer::View>,
    pub accessors: Vec<gltf::json::Accessor>,

    pub vertex_buffers: BTreeMap<BufferKey, VertexBuffer>,
    pub index_buffer_accessors: BTreeMap<BufferKey, usize>,
    pub weight_groups: BTreeMap<WeightGroupKey, WeightGroup>,
}

// TODO: Also store weights here?
#[derive(Clone)]
pub struct VertexBuffer {
    pub attributes: GltfAttributes,
    pub morph_targets: Vec<GltfAttributes>,
}

#[derive(Clone)]
pub struct WeightGroup {
    pub weights: GltfAttribute,
    pub indices: GltfAttribute,
}

impl Buffers {
    pub fn insert_vertex_buffer(
        &mut self,
        vertex_buffer: &crate::vertex::VertexBuffer,
        root_index: usize,
        group_index: usize,
        buffers_index: usize,
        buffer_index: usize,
        flip_uvs: bool,
    ) -> BinResult<VertexBuffer> {
        let key = BufferKey {
            root_index,
            group_index,
            buffers_index,
            buffer_index,
        };
        match self.vertex_buffers.get(&key) {
            Some(buffer) => Ok(buffer.clone()),
            None => {
                let buffer = self.insert_vertex_buffer_inner(vertex_buffer, flip_uvs)?;
                self.vertex_buffers.insert(key, buffer.clone());
                Ok(buffer)
            }
        }
    }

    fn insert_vertex_buffer_inner(
        &mut self,
        vertex_buffer: &crate::vertex::VertexBuffer,
        flip_uvs: bool,
    ) -> Result<VertexBuffer, binrw::Error> {
        let mut attributes = self.write_attributes(&vertex_buffer.attributes, flip_uvs)?;

        // Apply attributes from the base blend target if present.
        let blend_attributes =
            self.write_attributes(&vertex_buffer.morph_blend_target, flip_uvs)?;
        attributes.extend(blend_attributes);

        let morph_targets = self.insert_morph_targets(vertex_buffer, &attributes)?;

        Ok(VertexBuffer {
            attributes,
            morph_targets,
        })
    }

    fn insert_morph_targets(
        &mut self,
        vertex_buffer: &crate::vertex::VertexBuffer,
        attributes: &GltfAttributes,
    ) -> Result<Vec<GltfAttributes>, binrw::Error> {
        if !vertex_buffer.morph_targets.is_empty() {
            let base_normals = vertex_buffer
                .morph_blend_target
                .iter()
                .find_map(|a| {
                    if let AttributeData::Normal4(v) = a {
                        Some(v)
                    } else {
                        None
                    }
                })
                .unwrap();
            let base_tangents = vertex_buffer
                .morph_blend_target
                .iter()
                .find_map(|a| {
                    if let AttributeData::Tangent2(v) = a {
                        Some(v)
                    } else {
                        None
                    }
                })
                .unwrap();

            vertex_buffer
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

                        let normal = base_normals[*vertex_index as usize].xyz() * 2.0 - 1.0;
                        normal_deltas[*vertex_index as usize] = target.normals[i].xyz() - normal;

                        let tangent = base_tangents[*vertex_index as usize].xyz() * 2.0 - 1.0;
                        tangent_deltas[*vertex_index as usize] = target.tangents[i].xyz() - tangent;
                    }

                    // glTF morph targets are defined as a difference with the base target.
                    let mut attributes = attributes.clone();
                    self.insert_positions(&position_deltas, &mut attributes)?;

                    // Normals and tangents also use deltas.
                    // These should use Vec3 to avoid displacing the sign in tangent.w.
                    self.insert_vec3(&normal_deltas, gltf::Semantic::Normals, &mut attributes)?;
                    self.insert_vec3(&tangent_deltas, gltf::Semantic::Tangents, &mut attributes)?;

                    Ok(attributes)
                })
                .collect()
        } else {
            Ok(Vec::new())
        }
    }

    pub fn insert_weight_group(
        &mut self,
        buffers: &crate::ModelBuffers,
        skeleton: Option<&crate::Skeleton>,
        key: WeightGroupKey,
    ) -> Option<WeightGroup> {
        match self.weight_groups.get(&key) {
            Some(group) => Some(group.clone()),
            None => {
                let weight_group = self.insert_weight_group_inner(skeleton, buffers, key)?;
                self.weight_groups.insert(key, weight_group.clone());
                Some(weight_group)
            }
        }
    }

    fn insert_weight_group_inner(
        &mut self,
        skeleton: Option<&crate::Skeleton>,
        buffers: &crate::vertex::ModelBuffers,
        key: WeightGroupKey,
    ) -> Option<WeightGroup> {
        let vertex_buffer = &buffers.vertex_buffers[key.buffer.buffer_index];
        let weight_indices = vertex_buffer.attributes.iter().find_map(|a| match a {
            AttributeData::WeightIndex(indices) => Some(indices),
            AttributeData::WeightIndex2(indices) => Some(indices),
            _ => None,
        })?;

        let weight_group = self
            .add_weight_group(
                skeleton?,
                buffers.weights.as_ref()?,
                weight_indices,
                key.flags2,
                key.weights_start_index,
            )
            .unwrap();
        Some(weight_group)
    }

    fn add_weight_group(
        &mut self,
        skeleton: &crate::Skeleton,
        weights: &crate::skinning::Weights,
        weight_indices: &[[u16; 2]],
        flags2: u32,
        weights_start_index: usize,
    ) -> BinResult<WeightGroup> {
        let skin_weights = weights.weight_buffer(flags2).unwrap();

        // The weights may be defined with a different bone ordering.
        let bone_names: Vec<_> = skeleton.bones.iter().map(|b| b.name.clone()).collect();
        let skin_weights = skin_weights.reindex_bones(bone_names);

        // Each group has a different starting offset.
        // This needs to be applied during reindexing.
        // No offset is needed if no groups are assigned.
        let skin_weights = skin_weights.reindex(weight_indices, weights_start_index as u32);

        let weights_accessor = self.add_values(
            &skin_weights.weights,
            gltf::json::accessor::Type::Vec4,
            gltf::json::accessor::ComponentType::F32,
            Some(Valid(Target::ArrayBuffer)),
            (None, None),
            true,
        )?;
        let indices_accessor = self.add_values(
            &skin_weights.bone_indices,
            gltf::json::accessor::Type::Vec4,
            gltf::json::accessor::ComponentType::U8,
            Some(Valid(Target::ArrayBuffer)),
            (None, None),
            true,
        )?;

        Ok(WeightGroup {
            weights: (Valid(gltf::Semantic::Weights(0)), weights_accessor),
            indices: (Valid(gltf::Semantic::Joints(0)), indices_accessor),
        })
    }

    fn write_attributes(
        &mut self,
        buffer_attributes: &[AttributeData],
        flip_uvs: bool,
    ) -> BinResult<GltfAttributes> {
        let mut attributes = GltfAttributes::new();

        for attribute in buffer_attributes {
            match attribute {
                AttributeData::Position(values) => {
                    self.insert_positions(values, &mut attributes)?;
                }
                AttributeData::Normal(values) => {
                    // Not all applications will normalize the vertex normals.
                    // Use Vec3 instead of Vec4 since it's better supported.
                    let values: Vec<_> =
                        values.iter().map(|v| v.xyz().normalize_or_zero()).collect();
                    self.insert_vec3(&values, gltf::Semantic::Normals, &mut attributes)?;
                }
                AttributeData::Normal2(values) => {
                    // Not all applications will normalize the vertex normals.
                    // Use Vec3 instead of Vec4 since it's better supported.
                    let values: Vec<_> =
                        values.iter().map(|v| v.xyz().normalize_or_zero()).collect();
                    self.insert_vec3(&values, gltf::Semantic::Normals, &mut attributes)?;
                }
                AttributeData::Tangent(values) => {
                    // Not all applications will normalize the vertex tangents.
                    let values: Vec<_> = values
                        .iter()
                        .map(|v| v.xyz().normalize_or_zero().extend(v.w))
                        .collect();
                    self.insert_vec4(&values, gltf::Semantic::Tangents, &mut attributes)?;
                }
                AttributeData::TexCoord0(values) => {
                    self.insert_uvs(values, 0, &mut attributes, flip_uvs)?;
                }
                AttributeData::TexCoord1(values) => {
                    self.insert_uvs(values, 1, &mut attributes, flip_uvs)?;
                }
                AttributeData::TexCoord2(values) => {
                    self.insert_uvs(values, 2, &mut attributes, flip_uvs)?;
                }
                AttributeData::TexCoord3(values) => {
                    self.insert_uvs(values, 3, &mut attributes, flip_uvs)?;
                }
                AttributeData::TexCoord4(values) => {
                    self.insert_uvs(values, 4, &mut attributes, flip_uvs)?;
                }
                AttributeData::TexCoord5(values) => {
                    self.insert_uvs(values, 5, &mut attributes, flip_uvs)?;
                }
                AttributeData::TexCoord6(values) => {
                    self.insert_uvs(values, 6, &mut attributes, flip_uvs)?;
                }
                AttributeData::TexCoord7(values) => {
                    self.insert_uvs(values, 7, &mut attributes, flip_uvs)?;
                }
                AttributeData::TexCoord8(values) => {
                    self.insert_uvs(values, 8, &mut attributes, flip_uvs)?;
                }
                AttributeData::VertexColor(values) => {
                    // TODO: Vertex color isn't always an RGB multiplier?
                    // Use a custom attribute to avoid rendering issues.
                    self.insert_vec4(
                        values,
                        gltf::Semantic::Extras("VertexColor".to_string()),
                        &mut attributes,
                    )?;
                }
                AttributeData::Blend(values) => {
                    // Used for color blending for some stages.
                    self.insert_vec4(
                        values,
                        gltf::Semantic::Extras("Blend".to_string()),
                        &mut attributes,
                    )?;
                }
                AttributeData::ValInf(_) => (),
                AttributeData::Unk15(_) => (),
                AttributeData::Unk16(_) => (),
                AttributeData::Unk18(_) => (),
                AttributeData::Unk24(_) => (),
                AttributeData::Unk25(_) => (),
                AttributeData::Unk26(_) => (),
                AttributeData::Unk30(_) => (),
                AttributeData::Unk31(_) => (),
                AttributeData::Flow(_) => (),
                AttributeData::Normal3(_) => (),
                AttributeData::VertexColor3(_) => (),
                // Skin weights are handled separately.
                AttributeData::WeightIndex(_) => (),
                AttributeData::WeightIndex2(_) => (),
                AttributeData::SkinWeights(_) => (),
                AttributeData::SkinWeights2(_) => (),
                AttributeData::BoneIndices(_) => (),
                AttributeData::BoneIndices2(_) => (),
                // Also handle base morph attributes.
                AttributeData::Position2(values) => {
                    self.insert_positions(values, &mut attributes)?;
                }
                AttributeData::Normal4(values) => {
                    // Not all applications will normalize the vertex normals.
                    // Use Vec3 instead of Vec4 since it's better supported.
                    let values: Vec<_> = values
                        .iter()
                        .map(|v| (v.xyz()).normalize_or_zero())
                        .collect();
                    self.insert_vec3(&values, gltf::Semantic::Normals, &mut attributes)?;
                }
                AttributeData::OldPosition(_) => (),
                AttributeData::Tangent2(values) => {
                    // Not all applications will normalize the vertex tangents.
                    let values: Vec<_> = values
                        .iter()
                        .map(|v| (v.xyz()).normalize_or_zero().extend(v.w))
                        .collect();
                    self.insert_vec4(&values, gltf::Semantic::Tangents, &mut attributes)?;
                }
            }
        }
        Ok(attributes)
    }

    pub fn insert_index_buffer(
        &mut self,
        index_buffer: &crate::vertex::IndexBuffer,
        root_index: usize,
        group_index: usize,
        buffers_index: usize,
        buffer_index: usize,
    ) -> BinResult<usize> {
        let key = BufferKey {
            root_index,
            group_index,
            buffers_index,
            buffer_index,
        };
        if !self.index_buffer_accessors.contains_key(&key) {
            let index_bytes = write_bytes(&index_buffer.indices)?;

            // The offset must be a multiple of the component data type.
            align_bytes(&mut self.buffer_bytes, std::mem::size_of::<u16>());

            // Assume everything uses the same buffer for now.
            let view = gltf::json::buffer::View {
                buffer: gltf::json::Index::new(0),
                byte_length: index_bytes.len().into(),
                byte_offset: Some(self.buffer_bytes.len().into()),
                byte_stride: None,
                extensions: Default::default(),
                extras: Default::default(),
                name: None,
                target: Some(Valid(gltf::json::buffer::Target::ElementArrayBuffer)),
            };

            let indices = gltf::json::Accessor {
                buffer_view: Some(gltf::json::Index::new(self.buffer_views.len() as u32)),
                byte_offset: Some(gltf::json::validation::USize64(0)),
                count: index_buffer.indices.len().into(),
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
                    buffer_index,
                },
                self.accessors.len(),
            );

            self.accessors.push(indices);
            self.buffer_views.push(view);
            self.buffer_bytes.extend_from_slice(&index_bytes);
        }

        Ok(*self.index_buffer_accessors.get(&key).unwrap())
    }

    fn insert_positions(
        &mut self,
        values: &[Vec3],
        attributes: &mut GltfAttributes,
    ) -> BinResult<()> {
        // Attributes should be non empty.
        if !values.is_empty() {
            // Only the position attribute requires min/max.
            let min_max = positions_min_max(values);

            let index = self.add_values(
                values,
                gltf::json::accessor::Type::Vec3,
                gltf::json::accessor::ComponentType::F32,
                Some(Valid(Target::ArrayBuffer)),
                min_max,
                true,
            )?;

            // Assume the buffer has only one of each attribute semantic.
            attributes.insert(Valid(gltf::Semantic::Positions), index);
        }
        Ok(())
    }

    fn insert_uvs(
        &mut self,
        values: &[Vec2],
        index: u32,
        attributes: &mut GltfAttributes,
        flip_vertical: bool,
    ) -> BinResult<()> {
        if flip_vertical {
            let mut values = values.to_vec();
            for v in &mut values {
                v.y = 1.0 - v.y;
            }
            self.insert_vec2(&values, gltf::Semantic::TexCoords(index), attributes)
        } else {
            self.insert_vec2(values, gltf::Semantic::TexCoords(index), attributes)
        }
    }

    fn insert_vec2(
        &mut self,
        values: &[Vec2],
        semantic: gltf::Semantic,
        attributes: &mut GltfAttributes,
    ) -> BinResult<()> {
        self.insert_attribute_values(
            values,
            semantic,
            gltf::json::accessor::Type::Vec2,
            gltf::json::accessor::ComponentType::F32,
            Some(Valid(Target::ArrayBuffer)),
            attributes,
        )
    }

    fn insert_vec3(
        &mut self,
        values: &[Vec3],
        semantic: gltf::Semantic,
        attributes: &mut GltfAttributes,
    ) -> BinResult<()> {
        self.insert_attribute_values(
            values,
            semantic,
            gltf::json::accessor::Type::Vec3,
            gltf::json::accessor::ComponentType::F32,
            Some(Valid(Target::ArrayBuffer)),
            attributes,
        )
    }

    fn insert_vec4(
        &mut self,
        values: &[Vec4],
        semantic: gltf::Semantic,
        attributes: &mut GltfAttributes,
    ) -> BinResult<()> {
        self.insert_attribute_values(
            values,
            semantic,
            gltf::json::accessor::Type::Vec4,
            gltf::json::accessor::ComponentType::F32,
            Some(Valid(Target::ArrayBuffer)),
            attributes,
        )
    }

    fn insert_attribute_values<T: WriteBytes>(
        &mut self,
        values: &[T],
        semantic: gltf::Semantic,
        components: gltf::json::accessor::Type,
        component_type: gltf::json::accessor::ComponentType,
        target: Option<Checked<Target>>,
        attributes: &mut GltfAttributes,
    ) -> BinResult<()> {
        // Attributes should be non empty.
        if !values.is_empty() {
            let index = self.add_values(
                values,
                components,
                component_type,
                target,
                (None, None),
                true,
            )?;

            // Assume the buffer has only one of each attribute semantic.
            attributes.insert(Valid(semantic), index);
        }
        Ok(())
    }

    pub fn add_values<T: WriteBytes>(
        &mut self,
        values: &[T],
        components: gltf::json::accessor::Type,
        component_type: gltf::json::accessor::ComponentType,
        target: Option<Checked<Target>>,
        min_max: (Option<gltf_json::Value>, Option<gltf_json::Value>),
        byte_stride: bool,
    ) -> BinResult<gltf::json::Index<gltf::json::Accessor>> {
        let attribute_bytes = write_bytes(values)?;

        // The offset must be a multiple of the component data type.
        align_bytes(&mut self.buffer_bytes, std::mem::size_of::<T>());

        // Assume everything uses the same buffer for now.
        // Each attribute is in its own section and thus has its own view.
        let view = gltf::json::buffer::View {
            buffer: gltf::json::Index::new(0),
            byte_length: attribute_bytes.len().into(),
            byte_offset: Some(self.buffer_bytes.len().into()),
            byte_stride: byte_stride
                .then_some(gltf::json::buffer::Stride(std::mem::size_of::<T>())),
            extensions: Default::default(),
            extras: Default::default(),
            name: None,
            target,
        };
        self.buffer_bytes.extend_from_slice(&attribute_bytes);

        let (min, max) = min_max;

        let accessor = gltf::json::Accessor {
            buffer_view: Some(gltf::json::Index::new(self.buffer_views.len() as u32)),
            byte_offset: Some(gltf::json::validation::USize64(0)),
            count: values.len().into(),
            component_type: Valid(gltf::json::accessor::GenericComponentType(component_type)),
            extensions: Default::default(),
            extras: Default::default(),
            type_: Valid(components),
            min,
            max,
            name: None,
            normalized: false,
            sparse: None,
        };

        let index = gltf::json::Index::new(self.accessors.len() as u32);

        self.accessors.push(accessor);
        self.buffer_views.push(view);

        Ok(index)
    }
}

fn positions_min_max(values: &[Vec3]) -> (Option<gltf_json::Value>, Option<gltf_json::Value>) {
    let min = values.iter().copied().reduce(Vec3::min);
    let max = values.iter().copied().reduce(Vec3::max);

    if let (Some(min), Some(max)) = (min, max) {
        (
            Some(serde_json::json!([min.x, min.y, min.z])),
            Some(serde_json::json!([max.x, max.y, max.z])),
        )
    } else {
        (None, None)
    }
}

// gltf requires little endian for byte buffers.
// Create a trait instead of using bytemuck.
pub trait WriteBytes {
    fn write<W: Write + Seek>(&self, writer: &mut W) -> BinResult<()>;
}

impl WriteBytes for u16 {
    fn write<W: Write + Seek>(&self, writer: &mut W) -> BinResult<()> {
        self.write_le(writer)
    }
}

impl WriteBytes for [u8; 4] {
    fn write<W: Write + Seek>(&self, writer: &mut W) -> BinResult<()> {
        self.write_le(writer)
    }
}

impl WriteBytes for Vec2 {
    fn write<W: Write + Seek>(&self, writer: &mut W) -> BinResult<()> {
        self.to_array().write_le(writer)
    }
}

impl WriteBytes for Vec3 {
    fn write<W: Write + Seek>(&self, writer: &mut W) -> BinResult<()> {
        self.to_array().write_le(writer)
    }
}

impl WriteBytes for Vec4 {
    fn write<W: Write + Seek>(&self, writer: &mut W) -> BinResult<()> {
        self.to_array().write_le(writer)
    }
}

impl WriteBytes for Mat4 {
    fn write<W: Write + Seek>(&self, writer: &mut W) -> BinResult<()> {
        self.to_cols_array().write_le(writer)
    }
}

fn write_bytes<T: WriteBytes>(values: &[T]) -> BinResult<Vec<u8>> {
    let mut writer = Cursor::new(Vec::new());
    for v in values {
        v.write(&mut writer)?;
    }
    Ok(writer.into_inner())
}
