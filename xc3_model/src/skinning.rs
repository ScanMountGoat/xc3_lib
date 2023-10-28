//! Utilities for working with vertex skinning.
use glam::Vec4;
use log::error;

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

#[derive(Debug, PartialEq, Clone)]
pub struct SkinWeights {
    pub bone_indices: Vec<[u8; 4]>,
    pub weights: Vec<Vec4>,
    /// The name list for the indices in [bone_indices](#structfield.bone_indices).
    pub bone_names: Vec<String>,
}

impl SkinWeights {
    // TODO: tests for this?
    /// Reindex bone indices to match the ordering defined in the new bone list.
    pub fn reindex_bones(&self, bone_names: Vec<String>) -> Self {
        let bone_indices = self
            .bone_indices
            .iter()
            .map(|indices| {
                indices.map(|i| {
                    let name = &self.bone_names[i as usize];
                    // TODO: Return an error if a bone is missing?
                    bone_names
                        .iter()
                        .position(|n| n == name)
                        .map(|i| i as u8)
                        .unwrap()
                })
            })
            .collect();

        Self {
            bone_indices,
            weights: self.weights.clone(),
            bone_names,
        }
    }

    // TODO: tests for this?
    /// Reindex the weights and indices using [WeightIndex](xc3_lib::vertex::DataType::WeightIndex) values.
    /// The `weight_group_input_start_index` should use the value from the mesh's weight group.
    pub fn reindex(&self, weight_indices: &[u32], weight_group_input_start_index: u32) -> Self {
        let mut weights = Vec::new();
        let mut bone_indices = Vec::new();
        for i in weight_indices {
            let index = *i as usize + weight_group_input_start_index as usize;
            weights.push(self.weights[index]);
            bone_indices.push(self.bone_indices[index]);
        }
        Self {
            bone_indices,
            weights,
            bone_names: self.bone_names.clone(),
        }
    }
}

impl SkinWeights {
    /// Convert the per-vertex indices and weights to per bone influences.
    /// The `weight_indices` represent the data from [crate::vertex::AttributeData::WeightIndex].
    /// The `skeleton` defines the mapping from bone indices to bone names.
    pub fn to_influences(&self, weight_indices: &[u32]) -> Vec<crate::skinning::Influence> {
        let mut influences: Vec<_> = self
            .bone_names
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
            for i in 0..4 {
                // The weight index selects an entry in the weights buffer.
                let bone_index = self.bone_indices[*weight_index as usize][i] as usize;
                let weight = self.weights[*weight_index as usize][i];

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
                error!("Influence {:?} not found in skeleton.", influence.bone_name);
            }
        }

        Self {
            bone_indices,
            weights,
            bone_names: bone_names.iter().map(|n| n.as_ref().to_string()).collect(),
        }
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
                bone_names: vec!["a".to_string(), "b".to_string(), "c".to_string()]
            },
            SkinWeights::from_influences(&[], 3, &["a", "b", "c"])
        );
    }

    #[test]
    fn bone_indices_weights_multiple_influences() {
        assert_eq!(
            SkinWeights {
                bone_indices: vec![[2, 0, 0, 0], [0, 0, 0, 0], [0, 1, 0, 0]],
                weights: vec![
                    vec4(0.2, 0.0, 0.0, 0.0),
                    vec4(0.0, 0.0, 0.0, 0.0),
                    vec4(0.11, 0.3, 0.0, 0.0)
                ],
                bone_names: vec!["a".to_string(), "c".to_string(), "b".to_string()]
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
        assert!(SkinWeights {
            bone_indices: Vec::new(),
            weights: Vec::new(),
            bone_names: Vec::new(),
        }
        .to_influences(&[])
        .is_empty());
    }

    #[test]
    fn bone_influences_zero_weights() {
        assert_eq!(
            vec![Influence {
                bone_name: "root".to_string(),
                weights: Vec::new()
            }],
            SkinWeights {
                bone_indices: vec![[0u8; 4], [0u8; 4]],
                weights: vec![Vec4::ZERO, Vec4::ZERO],
                bone_names: vec!["root".to_string()]
            }
            .to_influences(&[0, 1])
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
                bone_names: vec!["root".to_string()]
            }
            .to_influences(&[1])
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
                }
            ],
            SkinWeights {
                bone_indices: vec![[3, 1, 2, 0], [2, 1, 0, 0]],
                weights: vec![vec4(0.3, 0.4, 0.1, 0.2), vec4(0.7, 0.3, 0.0, 0.0)],
                bone_names: vec![
                    "D".to_string(),
                    "C".to_string(),
                    "B".to_string(),
                    "A".to_string(),
                ]
            }
            .to_influences(&[0, 1])
        );
    }
}
