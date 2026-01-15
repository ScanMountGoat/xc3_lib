//! Utilities for working with vertex skinning.
use glam::{Vec3, Vec4};
use log::error;
use xc3_lib::{mxmd::RenderPassType, vertex::WeightLod};

/// See [Skinning](xc3_lib::mxmd::Skinning).
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Clone)]
pub struct Skinning {
    pub bones: Vec<Bone>,
}

/// See [Bone](xc3_lib::mxmd::Skinning).
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Clone)]
pub struct Bone {
    pub name: String,
    pub bounds: Option<BoneBounds>,
    pub constraint: Option<BoneConstraint>,
    pub no_camera_overlap: bool,
}

/// See [BoneBounds](xc3_lib::mxmd::BoneBounds).
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Clone)]
pub struct BoneBounds {
    pub center: Vec3,
    pub size: Vec3,
    pub radius: f32,
}

/// See [BoneConstraint](xc3_lib::mxmd::BoneConstraint).
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Clone)]
pub struct BoneConstraint {
    pub fixed_offset: Vec3,
    pub max_distance: f32,
    pub constraint_type: BoneConstraintType,
    /// The index of the parent [Bone] in [bones](struct.Skinning.html#structfield.bones)
    /// or `None` if this is a root bone.
    pub parent_index: Option<usize>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum BoneConstraintType {
    FixedOffset,
    Distance,
}

// TODO: come up with a better name?
/// See [Weights](xc3_lib::vertex::Weights).
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Clone)]
pub struct Weights {
    /// Attributes for buffers containing skin weights.
    /// Xenoblade X models may have more than one weight buffer.
    pub weight_buffers: Vec<SkinWeights>,

    // TODO: Is this the best way to represent this information?
    // TODO: Avoid storing game specific data here?
    // TODO: Is it possible to rebuild equivalent weights for in game models?
    pub weight_groups: WeightGroups,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Clone)]
pub enum WeightGroups {
    Legacy {
        /// Same as the indices in [VertexData](xc3_lib::mxmd::legacy::VertexData).
        weight_buffer_indices: [Option<usize>; 6],
    },
    Groups {
        weight_groups: Vec<xc3_lib::vertex::WeightGroup>,
        weight_lods: Vec<xc3_lib::vertex::WeightLod>,
    },
}

impl Weights {
    /// Calculate the weights buffer for the given flags.
    ///
    /// For some legacy models in Xenoblade this will combine two buffers.
    /// Non legacy models will only ever use a single buffer.
    pub fn weight_buffer(&self, flags2: u32) -> Option<SkinWeights> {
        match self.weight_groups {
            WeightGroups::Legacy {
                weight_buffer_indices,
            } => match flags2 & 0xff {
                1 => {
                    // TODO: Why is this check necessary?
                    if weight_buffer_indices[4].unwrap_or_default() > 0 {
                        self.concatenate_buffers(weight_buffer_indices, 4, 0)
                    } else {
                        self.concatenate_buffers(weight_buffer_indices, 0, 4)
                    }
                }
                2 | 64 => self
                    .weight_buffers
                    .get(weight_buffer_indices[1].unwrap_or_default())
                    .cloned(),
                8 => self.concatenate_buffers(weight_buffer_indices, 3, 4),
                0x21 => self
                    .weight_buffers
                    .get(weight_buffer_indices[4].unwrap_or_default())
                    .cloned(),
                _ => self.weight_buffers.first().cloned(),
            },
            WeightGroups::Groups { .. } => self.weight_buffers.first().cloned(),
        }
    }

    fn concatenate_buffers(
        &self,
        weight_buffer_indices: [Option<usize>; 6],
        i0: usize,
        i1: usize,
    ) -> Option<SkinWeights> {
        let mut b0 = self
            .weight_buffers
            .get(weight_buffer_indices[i0].unwrap_or_default())?
            .clone();
        let b1 = self
            .weight_buffers
            .get(weight_buffer_indices[i1].unwrap_or_default())?;
        b0.bone_indices.extend_from_slice(&b1.bone_indices);
        b0.weights.extend_from_slice(&b1.weights);
        Some(b0)
    }

    // TODO: Fully recreate all data and return Self?
    /// Initialize all weight data to use a single shared weight buffer.
    pub fn update_weights(&mut self, combined_weights: SkinWeights) {
        match &mut self.weight_groups {
            WeightGroups::Legacy {
                weight_buffer_indices,
            } => {
                // Point all meshes to the first weight buffer.
                *weight_buffer_indices = [Some(0); 6];
            }
            WeightGroups::Groups { weight_groups, .. } => {
                // TODO: Will making each group the same account for mesh.flags2?
                // TODO: Recreate this from scratch based on lod count?
                // TODO: What to do for the pass indices?
                for group in weight_groups {
                    // TODO: Is it ok for these ranges to all overlap?
                    group.output_start_index = 0;
                    group.input_start_index = 0;
                    group.count = combined_weights.bone_indices.len() as u32;
                    group.max_influences = 4; // TODO: calculate this?
                }
            }
        }
        self.weight_buffers = vec![combined_weights];
    }
}

impl WeightGroups {
    /// The offset to add to [crate::vertex::AttributeData::WeightIndex]
    /// when selecting [crate::vertex::AttributeData::BoneIndices] and [crate::vertex::AttributeData::SkinWeights].
    ///
    /// Preskinned matrices starting from the input index are written to the output index.
    /// This means the final index value is `weight_index = nWgtIndex + input_start - output_start`.
    /// Equivalent bone indices and weights are simply `indices[weight_index]` and `weights[weight_index]`.
    /// A mesh has only one assigned weight group, so this is sufficient to recreate the in game behavior
    /// without any complex precomputation of skinning matrices.
    pub fn weights_start_index(
        &self,
        flags2: u32,
        lod_item_index: Option<usize>,
        unk_type: xc3_lib::mxmd::RenderPassType,
    ) -> usize {
        match self {
            WeightGroups::Legacy { .. } => 0,
            WeightGroups::Groups {
                weight_groups,
                weight_lods,
            } => {
                // TODO: Error if none?
                let group_index = weight_group_index(weight_lods, flags2, lod_item_index, unk_type);
                weight_groups
                    .get(group_index)
                    .map(|group| (group.input_start_index - group.output_start_index) as usize)
                    .unwrap_or_default()
            }
        }
    }
}

fn weight_group_index(
    weight_lods: &[WeightLod],
    skin_flags: u32,
    lod_item_index: Option<usize>,
    unk_type: RenderPassType,
) -> usize {
    if !weight_lods.is_empty() {
        // TODO: Should this check skin flags?
        // TODO: Is lod actually some sort of flags?
        // TODO: Return none if skin_flags == 64?
        let lod_index = lod_item_index.unwrap_or_default();

        // TODO: More mesh lods than weight lods for models with multiple lod groups?
        // TODO: Should this use the index in the LodItem?
        // weight_lod_index = lod_data.items[lod].index
        let weight_lod = &weight_lods[lod_index % weight_lods.len()];

        let pass_index = weight_pass_index(unk_type, skin_flags);
        weight_lod.group_indices_plus_one[pass_index].saturating_sub(1) as usize
    } else {
        // TODO: How to handle the empty case?
        0
    }
}

// TODO: Should this be the pass from flags2 instead?
fn weight_pass_index(unk_type: RenderPassType, flags2: u32) -> usize {
    // TODO: skin_flags & 0xF has a max value of group_indices.len() - 1?
    // TODO: bit mask?
    // TODO: Test possible values by checking mesh flags and pass types in xc3_test?
    // TODO: Compare this with non zero entries in group indices?
    // TODO: Assume all weight groups are assigned to at least one mesh?
    // TODO: get unique parameters for this function for each wimdo?

    // TODO: Find a way to determine the group selected in game?
    // TODO: Test unique parameter combination using a modified weight group?
    // TODO: Detect if vertices move in game?
    let mut pass_index = match unk_type {
        RenderPassType::Unk0 => 0,
        RenderPassType::Unk1 => 1,
        RenderPassType::Unk2 => todo!(),
        RenderPassType::Unk3 => todo!(),
        RenderPassType::Unk5 => todo!(),
        RenderPassType::Unk6 => todo!(),
        RenderPassType::Unk7 => 3, // TODO: also 4?
        RenderPassType::Unk8 => todo!(),
        RenderPassType::Unk9 => todo!(),
    };
    if flags2 == 64 {
        pass_index = 4;
    }
    // TODO: Some pass index values don't get returned like 5,6?
    pass_index
}

// Using a bone name allows using different skeleton hierarchies.
// wimdo and chr files use different ordering, for example.
// Consuming code can create their own mappings from names to indices.
#[derive(Debug, PartialEq)]
pub struct Influence {
    pub bone_name: String,
    pub weights: Vec<VertexWeight>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VertexWeight {
    pub vertex_index: u32,
    pub weight: f32,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Clone)]
pub struct SkinWeights {
    pub bone_indices: Vec<[u8; 4]>,
    pub weights: Vec<Vec4>,
}

impl SkinWeights {
    // TODO: tests for this?
    /// Reindex the weights and indices using [WeightIndex](xc3_lib::vertex::DataType::WeightIndex) values.
    /// The `weight_group_input_start_index` should use the value from the mesh's weight group.
    pub fn reindex(
        &self,
        weight_indices: &[[u16; 2]],
        weight_group_input_start_index: u32,
    ) -> Self {
        let mut weights = Vec::new();
        let mut bone_indices = Vec::new();
        for i in weight_indices {
            let index = i[0] as usize + weight_group_input_start_index as usize;
            // TODO: Why is this sometimes out of bounds.
            if index < self.weights.len() && index < self.bone_indices.len() {
                weights.push(self.weights[index]);
                bone_indices.push(self.bone_indices[index]);
            }
        }
        Self {
            bone_indices,
            weights,
        }
    }

    // TODO: How should this handle of out range indices?
    /// Convert the per-vertex indices and weights to per bone influences used by `weight_indices`.
    ///
    /// The `weight_indices` represent the data from [crate::vertex::AttributeData::WeightIndex].
    /// The `skeleton` defines the mapping from bone indices to bone names.
    ///
    /// The `bone_names` should match the skinning bone list used for these skin weights.
    ///
    /// This assumes the weight group starting index has already been applied
    /// using a method like [Self::reindex].
    pub fn to_influences(
        &self,
        weight_indices: &[[u16; 2]],
        bone_names: &[String],
    ) -> Vec<crate::skinning::Influence> {
        let mut influences: Vec<_> = bone_names
            .iter()
            .map(|bone_name| Influence {
                bone_name: bone_name.clone(),
                weights: Vec::new(),
            })
            .collect();

        // The weights buffer contains both the bone indices and weights.
        // Vertex buffers only contain an index into the weights buffer.
        // TODO: The actual lookup is more complex than this.
        // TODO: Handle weight groups and lods?
        for (vertex_index, weight_index) in weight_indices.iter().enumerate() {
            let weight_index = weight_index[0] as usize;
            for i in 0..4 {
                // The weight index selects an entry in the weights buffer.
                let bone_index = self.bone_indices[weight_index][i] as usize;
                let weight = self.weights[weight_index][i];

                // Skip zero weights since they have no effect.
                if weight > 0.0 {
                    // The vertex attributes use the bone order of the mxmd skeleton.
                    influences[bone_index].weights.push(VertexWeight {
                        vertex_index: vertex_index as u32,
                        weight,
                    });
                }
            }
        }

        influences.retain(|i| !i.weights.is_empty());

        influences
    }

    // TODO: Remove the names parameter and add a modify names method?
    /// Convert the per-bone `influences` to per-vertex indices and weights.
    /// The `bone_names` provide the mapping from bone names to bone indices.
    /// Only the first 4 influences for each vertex will be included.
    pub fn from_influences<S: AsRef<str>>(
        influences: &[Influence],
        vertex_count: usize,
        bone_names: &[S],
    ) -> Self {
        let mut influence_counts = vec![0; vertex_count];
        let mut bone_indices = vec![[0u8; 4]; vertex_count];
        let mut weights = vec![Vec4::ZERO; vertex_count];

        // Assign up to 4 influences to each vertex.
        for influence in influences {
            if let Some(bone_index) = bone_names
                .iter()
                .position(|n| n.as_ref() == influence.bone_name)
            {
                for weight in &influence.weights {
                    let i = weight.vertex_index as usize;
                    // Ignore empty weights since they have no effect.
                    if influence_counts[i] < 4 && weight.weight > 0.0 {
                        bone_indices[i][influence_counts[i]] = bone_index as u8;
                        weights[i][influence_counts[i]] = weight.weight;
                        influence_counts[i] += 1;
                    }
                }
            } else {
                // TODO: This can result in bone names not working?
                error!("Influence {:?} not found in skeleton.", influence.bone_name);
            }
        }

        // In game weights are in ascending order by weight.
        // This can also reduce the number of unique weight values.
        for (is, ws) in bone_indices.iter_mut().zip(weights.iter_mut()) {
            let mut permutation = [0, 1, 2, 3];
            permutation.sort_by_key(|i| ordered_float::OrderedFloat::from(-ws[*i]));

            *is = permutation.map(|i| is[i]);
            *ws = permutation.map(|i| ws[i]).into();
        }

        Self {
            bone_indices,
            weights,
        }
    }

    // TODO: Tests for this
    /// Add unique bone indices and weights from `influences`
    /// and return the weight indices for `vertex_count` many vertices.
    ///
    /// The `bone_names` should match the skinning bone list used for these skin weights.
    pub fn add_influences<S>(
        &mut self,
        influences: &[Influence],
        vertex_count: usize,
        bone_names: &[S],
    ) -> Vec<[u16; 2]>
    where
        S: AsRef<str>,
    {
        let new_weights = SkinWeights::from_influences(influences, vertex_count, bone_names);

        // Add unique indices and weights from each buffer.
        // TODO: Xenoblade 2 has 384000 SSBO bytes / sizeof(mat3x4) = 8000 unique elements.
        // TODO: Xenoblade 3 has 576560 SSBO bytes / sizeof(mat3x4) = 12011 unique elements.
        // TODO: Make this not O(N^2) with key ([u8; 4], Vec4)
        new_weights
            .bone_indices
            .iter()
            .zip(new_weights.weights.iter())
            .map(|(bone_indices, bone_weights)| {
                match self
                    .bone_indices
                    .iter()
                    .zip(self.weights.iter())
                    .position(|(i2, w2)| i2 == bone_indices && w2 == bone_weights)
                {
                    Some(index) => [index as u16, 0],
                    None => {
                        let new_index = self.bone_indices.len();
                        self.bone_indices.push(*bone_indices);
                        self.weights.push(*bone_weights);
                        [new_index as u16, 0]
                    }
                }
            })
            .collect()
    }
}

pub(crate) fn create_skinning(skinning: &xc3_lib::mxmd::Skinning) -> Skinning {
    Skinning {
        bones: skinning
            .bones
            .iter()
            .map(|b| Bone {
                name: b.name.clone(),
                bounds: skinning.bounds.as_ref().and_then(|bounds| {
                    if b.flags.bounds_offset() {
                        bounds
                            .get(b.bounds_index as usize)
                            .map(|bounds| BoneBounds {
                                center: [bounds.center[0], bounds.center[1], bounds.center[2]]
                                    .into(),
                                size: [bounds.size[0], bounds.size[1], bounds.size[2]].into(),
                                radius: b.bounds_radius,
                            })
                    } else {
                        None
                    }
                }),
                constraint: skinning.constraints.as_ref().and_then(|constraints| {
                    if b.flags.fixed_offset_constraint() || b.flags.distance_constraint() {
                        constraints
                            .get(b.constraint_index as usize)
                            .map(|constraint| {
                                BoneConstraint {
                                    fixed_offset: constraint.fixed_offset.into(),
                                    max_distance: constraint.max_distance,
                                    constraint_type: if b.flags.distance_constraint() {
                                        BoneConstraintType::Distance
                                    } else {
                                        BoneConstraintType::FixedOffset
                                    },
                                    // TODO: how to detect root bones?
                                    parent_index: Some(b.parent_index as usize),
                                }
                            })
                    } else {
                        None
                    }
                }),
                no_camera_overlap: b.flags.no_camera_overlap(),
            })
            .collect(),
    }
}

// TODO: Test using a different bone name list.
#[cfg(test)]
mod tests {
    use super::*;

    use glam::vec4;

    #[test]
    fn bone_indices_weights_no_influences() {
        assert_eq!(
            SkinWeights {
                bone_indices: vec![[0u8; 4]; 3],
                weights: vec![Vec4::ZERO; 3],
            },
            SkinWeights::from_influences(&[], 3, &["a", "b", "c"])
        );
    }

    #[test]
    fn bone_indices_weights_multiple_influences() {
        assert_eq!(
            SkinWeights {
                bone_indices: vec![[2, 0, 0, 0], [0, 0, 0, 0], [1, 0, 0, 0]],
                weights: vec![
                    vec4(0.2, 0.0, 0.0, 0.0),
                    vec4(0.0, 0.0, 0.0, 0.0),
                    vec4(0.3, 0.11, 0.0, 0.0)
                ],
            },
            SkinWeights::from_influences(
                &[
                    Influence {
                        bone_name: "a".to_string(),
                        weights: vec![
                            VertexWeight {
                                vertex_index: 0,
                                weight: 0.0
                            },
                            VertexWeight {
                                vertex_index: 2,
                                weight: 0.11
                            }
                        ]
                    },
                    Influence {
                        bone_name: "b".to_string(),
                        weights: vec![VertexWeight {
                            vertex_index: 0,
                            weight: 0.2
                        }]
                    },
                    Influence {
                        bone_name: "c".to_string(),
                        weights: vec![VertexWeight {
                            vertex_index: 2,
                            weight: 0.3
                        }]
                    },
                    Influence {
                        bone_name: "d".to_string(),
                        weights: vec![VertexWeight {
                            vertex_index: 1,
                            weight: 0.4
                        }]
                    }
                ],
                3,
                &["a", "c", "b"]
            )
        );
    }

    #[test]
    fn bone_influences_empty() {
        assert!(
            SkinWeights {
                bone_indices: Vec::new(),
                weights: Vec::new(),
            }
            .to_influences(&[], &[])
            .is_empty()
        );
    }

    #[test]
    fn bone_influences_zero_weights() {
        assert!(
            SkinWeights {
                bone_indices: vec![[0u8; 4], [0u8; 4]],
                weights: vec![Vec4::ZERO, Vec4::ZERO],
            }
            .to_influences(&[[0, 0], [1, 0]], &["root".to_string()])
            .is_empty()
        );
    }

    #[test]
    fn bone_influences_reindex_weights() {
        assert_eq!(
            vec![Influence {
                bone_name: "root".to_string(),
                weights: vec![
                    VertexWeight {
                        vertex_index: 0,
                        weight: 0.75
                    };
                    4
                ]
            }],
            SkinWeights {
                bone_indices: vec![[0u8; 4], [0u8; 4]],
                weights: vec![Vec4::splat(0.5), Vec4::splat(0.75)],
            }
            .to_influences(&[[1, 0]], &["root".to_string()])
        );
    }

    #[test]
    fn bone_influences_multiple_bones() {
        assert_eq!(
            vec![
                Influence {
                    bone_name: "D".to_string(),
                    weights: vec![VertexWeight {
                        vertex_index: 0,
                        weight: 0.2
                    }]
                },
                Influence {
                    bone_name: "C".to_string(),
                    weights: vec![
                        VertexWeight {
                            vertex_index: 0,
                            weight: 0.4
                        },
                        VertexWeight {
                            vertex_index: 1,
                            weight: 0.3
                        }
                    ]
                },
                Influence {
                    bone_name: "B".to_string(),
                    weights: vec![
                        VertexWeight {
                            vertex_index: 0,
                            weight: 0.1
                        },
                        VertexWeight {
                            vertex_index: 1,
                            weight: 0.7
                        }
                    ]
                },
                Influence {
                    bone_name: "A".to_string(),
                    weights: vec![VertexWeight {
                        vertex_index: 0,
                        weight: 0.3
                    }]
                },
            ],
            SkinWeights {
                bone_indices: vec![[3, 1, 2, 0], [2, 1, 0, 0]],
                weights: vec![vec4(0.3, 0.4, 0.1, 0.2), vec4(0.7, 0.3, 0.0, 0.0)],
            }
            .to_influences(
                &[[0, 0], [1, 0]],
                &[
                    "D".to_string(),
                    "C".to_string(),
                    "B".to_string(),
                    "A".to_string(),
                    "unused".to_string()
                ]
            )
        );
    }

    #[test]
    fn weight_group_index_pc082402_fiora() {
        // xeno1/chr/pc/pc082402.wimdo
        let weight_lods = [WeightLod {
            group_indices_plus_one: [1, 0, 0, 2, 0, 0, 0, 0, 0],
        }];
        assert_eq!(
            0,
            weight_group_index(&weight_lods, 16385, None, RenderPassType::Unk0)
        );
        assert_eq!(
            1,
            weight_group_index(&weight_lods, 16392, None, RenderPassType::Unk7)
        );
    }

    #[test]
    fn weight_group_index_bl301501_ursula() {
        // xeno2/model/bl/bl301501.wimdo
        let weight_lods = [
            WeightLod {
                group_indices_plus_one: [1, 2, 0, 0, 0, 0, 0, 0, 0],
            },
            WeightLod {
                group_indices_plus_one: [3, 4, 0, 0, 0, 0, 0, 0, 0],
            },
            WeightLod {
                group_indices_plus_one: [5, 6, 0, 0, 0, 0, 0, 0, 0],
            },
        ];
        assert_eq!(
            0,
            weight_group_index(&weight_lods, 16385, Some(0), RenderPassType::Unk0)
        );
        assert_eq!(
            0,
            weight_group_index(&weight_lods, 1, Some(0), RenderPassType::Unk0)
        );
        assert_eq!(
            3,
            weight_group_index(&weight_lods, 2, Some(1), RenderPassType::Unk1)
        );
        assert_eq!(
            5,
            weight_group_index(&weight_lods, 2, Some(2), RenderPassType::Unk1)
        );
    }

    #[test]
    fn weight_group_index_ch01011023_noah() {
        // xeno3/chr/ch/ch01011023.wimdo
        let weight_lods = [
            WeightLod {
                group_indices_plus_one: [4, 0, 0, 3, 0, 1, 2, 0, 0],
            },
            WeightLod {
                group_indices_plus_one: [7, 0, 0, 6, 0, 5, 0, 0, 0],
            },
            WeightLod {
                group_indices_plus_one: [10, 0, 0, 9, 0, 8, 0, 0, 0],
            },
        ];
        assert_eq!(
            0,
            weight_group_index(&weight_lods, 64, Some(0), RenderPassType::Unk0)
        );
        assert_eq!(
            6,
            weight_group_index(&weight_lods, 16400, Some(1), RenderPassType::Unk0)
        );
    }

    #[test]
    fn add_influences_from_empty() {
        let influences = vec![
            Influence {
                bone_name: "D".to_string(),
                weights: vec![VertexWeight {
                    vertex_index: 0,
                    weight: 0.2,
                }],
            },
            Influence {
                bone_name: "C".to_string(),
                weights: vec![
                    VertexWeight {
                        vertex_index: 0,
                        weight: 0.4,
                    },
                    VertexWeight {
                        vertex_index: 1,
                        weight: 0.3,
                    },
                ],
            },
            Influence {
                bone_name: "B".to_string(),
                weights: vec![
                    VertexWeight {
                        vertex_index: 0,
                        weight: 0.1,
                    },
                    VertexWeight {
                        vertex_index: 1,
                        weight: 0.7,
                    },
                ],
            },
            Influence {
                bone_name: "A".to_string(),
                weights: vec![VertexWeight {
                    vertex_index: 0,
                    weight: 0.3,
                }],
            },
        ];
        let mut skin_weights = SkinWeights {
            bone_indices: Vec::new(),
            weights: Vec::new(),
        };
        skin_weights.add_influences(
            &influences,
            2,
            &[
                "D".to_string(),
                "C".to_string(),
                "B".to_string(),
                "A".to_string(),
                "unused".to_string(),
            ],
        );

        assert_eq!(
            SkinWeights {
                bone_indices: vec![[1, 3, 0, 2], [2, 1, 0, 0]],
                weights: vec![vec4(0.4, 0.3, 0.2, 0.1), vec4(0.7, 0.3, 0.0, 0.0)],
            },
            skin_weights
        );
    }
}
