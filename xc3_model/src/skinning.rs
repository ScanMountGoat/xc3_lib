use glam::Vec4;

use crate::vertex::AttributeData;

// Using a bone name allows using different skeleton hierarchies.
// wimdo and chr files use different ordering, for example.
// Consuming code can create their own mappings from names to indices.
#[derive(Debug)]
pub struct Influence {
    pub bone_name: String,
    pub weights: Vec<SkinWeight>,
}

#[derive(Debug)]
pub struct SkinWeight {
    pub vertex_index: u32,
    pub weight: f32,
}

// TODO: test this
/// Convert the per-vertex indices and weights to per bone influences.
/// The `skeleton` defines the mapping from bone indices to bone names.
pub fn create_influences(
    attributes: &[AttributeData],
    skin_weights: &[Vec4],
    bone_indices: &[[u8; 4]],
    skeleton: &xc3_lib::mxmd::Skeleton,
) -> Vec<crate::skinning::Influence> {
    let mut influences: Vec<_> = skeleton
        .bones
        .iter()
        .map(|b| Influence {
            bone_name: b.name.clone(),
            weights: Vec::new(),
        })
        .collect();

    // Weights and bone indices are shared among all buffers.
    // TODO: The actual lookup is more complex than this.
    // TODO: Handle weight groups and lods?
    if let Some(indices) = attributes.iter().find_map(|a| match a {
        AttributeData::WeightIndex(indices) => Some(indices),
        _ => None,
    }) {
        for (vertex_index, index) in indices.iter().enumerate() {
            for i in 0..4 {
                let bone_index = bone_indices[*index as usize][i] as usize;
                let weight = skin_weights[*index as usize][i];

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

// TODO: Use this for gltf and xc3_wgpu
/// Convert the per-bone `influences` to per-vertex indices and weights.
/// The `bone_names` provide the mapping from bone names to bone indices.
/// Only the first 4 influences for each vertex will be included.
pub fn create_bone_indices_weights<S: AsRef<str>>(
    influences: &[Influence],
    vertex_count: usize,
    bone_names: &[S],
) -> (Vec<[u8; 4]>, Vec<Vec4>) {
    // TODO: reverse the mapping
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
                if influence_counts[i] < 4 {
                    indices[i][influence_counts[i]] = bone_index as u8;
                    weights[i][influence_counts[i]] = weight.weight;
                    influence_counts[i] += 1;
                }
            }
        }
    }

    (indices, weights)
}

// TODO: indices/weights -> influences
// TODO influences -> indices/weights
// TODO: Test cases?

#[cfg(test)]
mod tests {
    #[test]
    fn test_weights() {}
}
