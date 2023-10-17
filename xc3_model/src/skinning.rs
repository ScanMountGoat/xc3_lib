//! Utilities for working with vertex skinning.
use glam::Vec4;
use log::error;

// Using a bone name allows using different skeleton hierarchies.
// wimdo and chr files use different ordering, for example.
// Consuming code can create their own mappings from names to indices.
#[derive(Debug, PartialEq)]
pub struct Influence {
    pub bone_name: String,
    pub weights: Vec<SkinWeight>,
}

#[derive(Debug, PartialEq)]
pub struct SkinWeight {
    pub vertex_index: u32,
    pub weight: f32,
}

/// Convert the per-vertex indices and weights to per bone influences.
/// The `weight_indices` represent the data from [crate::vertex::AttributeData::WeightIndex].
/// The `skeleton` defines the mapping from bone indices to bone names.
pub fn indices_weights_to_influences(
    weight_indices: &[u32],
    skin_weights: &[Vec4],
    bone_indices: &[[u8; 4]],
    bones: &[xc3_lib::mxmd::Bone],
) -> Vec<crate::skinning::Influence> {
    let mut influences: Vec<_> = bones
        .iter()
        .map(|b| Influence {
            bone_name: b.name.clone(),
            weights: Vec::new(),
        })
        .collect();

    // Weights and bone indices are shared among all buffers.
    // TODO: The actual lookup is more complex than this.
    // TODO: Handle weight groups and lods?
    for (vertex_index, index) in weight_indices.iter().enumerate() {
        for i in 0..4 {
            let bone_index = bone_indices[*index as usize][i] as usize;
            let weight = skin_weights[*index as usize][i];

            // Skip zero weights since they have no effect.
            if weight > 0.0 {
                // The vertex attributes use the bone order of the mxmd skeleton.
                influences[bone_index].weights.push(SkinWeight {
                    vertex_index: vertex_index as u32,
                    weight,
                });
            }
        }
    }

    influences
}

/// Convert the per-bone `influences` to per-vertex indices and weights.
/// The `bone_names` provide the mapping from bone names to bone indices.
/// Only the first 4 influences for each vertex will be included.
pub fn bone_indices_weights<S: AsRef<str>>(
    influences: &[Influence],
    vertex_count: usize,
    bone_names: &[S],
) -> (Vec<[u8; 4]>, Vec<Vec4>) {
    let mut influence_counts = vec![0; vertex_count];
    let mut indices = vec![[0u8; 4]; vertex_count];
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
                    indices[i][influence_counts[i]] = bone_index as u8;
                    weights[i][influence_counts[i]] = weight.weight;
                    influence_counts[i] += 1;
                }
            }
        } else {
            error!("Influence {:?} not found in skeleton.", influence.bone_name);
        }
    }

    (indices, weights)
}

#[cfg(test)]
mod tests {
    use super::*;

    use glam::vec4;

    #[test]
    fn bone_indices_weights_no_influences() {
        assert_eq!(
            (vec![[0u8; 4]; 3], vec![Vec4::ZERO; 3]),
            bone_indices_weights(&[], 3, &["a", "b", "c"])
        );
    }

    #[test]
    fn bone_indices_weights_multiple_influences() {
        assert_eq!(
            (
                vec![[2, 0, 0, 0], [0, 0, 0, 0], [0, 1, 0, 0]],
                vec![
                    vec4(0.2, 0.0, 0.0, 0.0),
                    vec4(0.0, 0.0, 0.0, 0.0),
                    vec4(0.11, 0.3, 0.0, 0.0)
                ]
            ),
            bone_indices_weights(
                &[
                    Influence {
                        bone_name: "a".to_string(),
                        weights: vec![
                            SkinWeight {
                                vertex_index: 0,
                                weight: 0.0
                            },
                            SkinWeight {
                                vertex_index: 2,
                                weight: 0.11
                            }
                        ]
                    },
                    Influence {
                        bone_name: "b".to_string(),
                        weights: vec![SkinWeight {
                            vertex_index: 0,
                            weight: 0.2
                        }]
                    },
                    Influence {
                        bone_name: "c".to_string(),
                        weights: vec![SkinWeight {
                            vertex_index: 2,
                            weight: 0.3
                        }]
                    },
                    Influence {
                        bone_name: "d".to_string(),
                        weights: vec![SkinWeight {
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

    fn bone(name: &str) -> xc3_lib::mxmd::Bone {
        xc3_lib::mxmd::Bone {
            name: name.to_string(),
            unk1: 0.0,
            unk_type: (0, 0),
            unk_index: 0,
            unk: [0; 2],
        }
    }

    #[test]
    fn bone_influences_empty() {
        assert!(indices_weights_to_influences(&[], &[], &[], &[]).is_empty());
    }

    #[test]
    fn bone_influences_zero_weights() {
        assert_eq!(
            vec![Influence {
                bone_name: "root".to_string(),
                weights: Vec::new()
            }],
            indices_weights_to_influences(
                &[0, 1],
                &[Vec4::ZERO, Vec4::ZERO],
                &[[0u8; 4], [0u8; 4]],
                &[bone("root")]
            )
        );
    }

    #[test]
    fn bone_influences_multiple_bones() {
        assert_eq!(
            vec![
                Influence {
                    bone_name: "A".to_string(),
                    weights: vec![SkinWeight {
                        vertex_index: 0,
                        weight: 0.2
                    }]
                },
                Influence {
                    bone_name: "B".to_string(),
                    weights: vec![
                        SkinWeight {
                            vertex_index: 0,
                            weight: 0.4
                        },
                        SkinWeight {
                            vertex_index: 1,
                            weight: 0.3
                        }
                    ]
                },
                Influence {
                    bone_name: "C".to_string(),
                    weights: vec![
                        SkinWeight {
                            vertex_index: 0,
                            weight: 0.1
                        },
                        SkinWeight {
                            vertex_index: 1,
                            weight: 0.7
                        }
                    ]
                },
                Influence {
                    bone_name: "D".to_string(),
                    weights: vec![SkinWeight {
                        vertex_index: 0,
                        weight: 0.3
                    }]
                }
            ],
            indices_weights_to_influences(
                &[0, 1],
                &[vec4(0.3, 0.4, 0.1, 0.2), vec4(0.7, 0.3, 0.0, 0.0)],
                &[[3, 1, 2, 0], [2, 1, 0, 0]],
                &[bone("A"), bone("B"), bone("C"), bone("D")]
            )
        );
    }
}
