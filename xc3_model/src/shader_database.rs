//! Database for compiled shader metadata for more accurate rendering.
//!
//! In game shaders are precompiled and embedded in files like `.wismt`.
//! These types represent precomputed metadata like assignments to G-Buffer textures.
//! This is necessary for determining the usage of a texture like albedo or normal map
//! since the assignments are compiled into the shader code itself.
//! Shader database JSON files should be generated using the xc3_shader CLI tool.
//! Applications can use the generated database to avoid needing to generate this data at runtime.

use std::path::Path;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

// TODO: How much extra space does JSON take up?
// TODO: Is it worth having a human readable version if it's only accessed through libraries?
// TODO: Binary representation?
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct ShaderDatabase {
    /// The `.wimdo` file name without the extension and shader data for each file.
    pub files: IndexMap<String, Spch>,
    /// The `.wismhd` file name without the extension and shader data for each map.
    pub map_files: IndexMap<String, Map>,
}

impl ShaderDatabase {
    /// Loads and deserializes the JSON data from `path`.
    pub fn from_file<P: AsRef<Path>>(path: P) -> Self {
        let json = std::fs::read_to_string(path).unwrap();
        serde_json::from_str(&json).unwrap()
    }
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct Map {
    pub map_models: Vec<Spch>,
    pub prop_models: Vec<Spch>,
    pub env_models: Vec<Spch>,
}

/// The decompiled shader data for a single shader container file.
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct Spch {
    pub programs: Vec<ShaderProgram>,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct ShaderProgram {
    /// Some shaders have multiple NVSD sections, so the length may be greater than 1.
    pub shaders: Vec<Shader>,
}

// TODO: Document how to try sampler, constant, parameter in order.
/// The buffer elements, textures, and constants used to initialize each fragment output.
///
/// This assumes inputs are assigned directly to outputs without any modifications.
/// Fragment shaders typically only perform basic input and channel selection in practice.
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Shader {
    pub output_dependencies: IndexMap<String, Vec<String>>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct BufferParameter {
    pub buffer: String,
    pub uniform: String,
    pub index: usize,
    pub channel: char,
}

impl Shader {
    /// Returns the sampler and channel index of the first material sampler assigned to the output
    /// or `None` if the output does not use a sampler.
    ///
    /// For example, an assignment of `"s3.y"` results in a sampler index of `3` and a channel index of `1`.
    pub fn sampler_channel_index(
        &self,
        output_index: usize,
        channel: char,
    ) -> Option<(usize, usize)> {
        let output = format!("o{output_index}.{channel}");

        // Find the first material referenced sampler like "s0" or "s1".
        let (sampler_index, channels) =
            self.output_dependencies
                .get(&output)?
                .iter()
                .find_map(|sampler_name| {
                    let (sampler, channels) = sampler_name.split_once('.')?;
                    let sampler_index = material_sampler_index(sampler)?;

                    Some((sampler_index, channels))
                })?;

        // Textures may have multiple accessed channels like normal maps.
        // First check if the current channel is used.
        // TODO: Does this always work as intended?
        let c = if channels.contains(channel) {
            channel
        } else {
            channels.chars().next().unwrap()
        };
        let channel_index = "xyzw".find(c).unwrap();
        Some((sampler_index, channel_index))
    }

    /// Returns the float constant assigned directly to the output
    /// or `None` if the output does not use a constant.
    pub fn float_constant(&self, output_index: usize, channel: char) -> Option<f32> {
        let output = format!("o{output_index}.{channel}");

        // If a constant is assigned, it will be the only dependency.
        self.output_dependencies.get(&output)?.first()?.parse().ok()
    }

    /// Returns the uniform buffer parameter assigned directly to the output
    /// or `None` if the output does not use a parameter.
    pub fn buffer_parameter(&self, output_index: usize, channel: char) -> Option<BufferParameter> {
        let output = format!("o{output_index}.{channel}");

        // If a parameter is assigned, it will be the only dependency.
        let text = self.output_dependencies.get(&output)?.first()?;

        // Parse U_Mate_gWrkFl4[0].x into "U_Mate", "gWrkFl4", 0, 'x'.
        let (text, c) = text.split_once('.')?;
        let (buffer, text) = text.rsplit_once('_')?;
        let (uniform, index) = text.split_once('[')?;
        let (index, _) = index.rsplit_once(']')?;

        Some(BufferParameter {
            buffer: buffer.to_string(),
            uniform: uniform.to_string(),
            index: index.parse().ok()?,
            channel: c.chars().next().unwrap(),
        })
    }
}

fn material_sampler_index(sampler: &str) -> Option<usize> {
    // TODO: Just parse int?
    match sampler {
        "s0" => Some(0),
        "s1" => Some(1),
        "s2" => Some(2),
        "s3" => Some(3),
        "s4" => Some(4),
        "s5" => Some(5),
        "s6" => Some(6),
        "s7" => Some(7),
        "s8" => Some(8),
        "s9" => Some(9),
        // TODO: How to handle this case?
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn material_channel_assignment_empty() {
        let shader = Shader {
            output_dependencies: IndexMap::new(),
        };
        assert_eq!(None, shader.sampler_channel_index(0, 'x'));
    }

    #[test]
    fn material_channel_assignment_single_output_no_assignment() {
        let shader = Shader {
            output_dependencies: [("o0.x".to_string(), Vec::new())].into(),
        };
        assert_eq!(None, shader.sampler_channel_index(0, 'x'));
    }

    #[test]
    fn material_channel_assignment_multiple_output_assignment() {
        let shader = Shader {
            output_dependencies: [
                ("o0.x".to_string(), vec!["s0.y".to_string()]),
                (
                    "o0.y".to_string(),
                    vec!["tex.xyz".to_string(), "s2.z".to_string()],
                ),
                ("o1.x".to_string(), vec!["s3.xyz".to_string()]),
            ]
            .into(),
        };
        assert_eq!(Some((2, 2)), shader.sampler_channel_index(0, 'y'));
    }

    #[test]
    fn float_constant_multiple_assigments() {
        let shader = Shader {
            output_dependencies: [
                ("o0.x".to_string(), vec!["s0.y".to_string()]),
                (
                    "o0.y".to_string(),
                    vec!["tex.xyz".to_string(), "s2.z".to_string()],
                ),
                ("o1.z".to_string(), vec!["0.5".to_string()]),
            ]
            .into(),
        };
        assert_eq!(None, shader.float_constant(0, 'x'));
        assert_eq!(Some(0.5), shader.float_constant(1, 'z'));
    }

    #[test]
    fn buffer_parameter_multiple_assigments() {
        let shader = Shader {
            output_dependencies: [
                ("o0.x".to_string(), vec!["s0.y".to_string()]),
                (
                    "o0.y".to_string(),
                    vec!["tex.xyz".to_string(), "s2.z".to_string()],
                ),
                ("o1.z".to_string(), vec!["U_Mate_param[31].w".to_string()]),
            ]
            .into(),
        };
        assert_eq!(None, shader.buffer_parameter(0, 'x'));
        assert_eq!(
            Some(BufferParameter {
                buffer: "U_Mate".to_string(),
                uniform: "param".to_string(),
                index: 31,
                channel: 'w'
            }),
            shader.buffer_parameter(1, 'z')
        );
    }
}
