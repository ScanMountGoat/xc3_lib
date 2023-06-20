use glsl::{parser::Parse, syntax::ShaderStage};
use indexmap::IndexMap;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::dependencies::input_dependencies;

// TODO: How much extra space does JSON take up?
// TODO: Is it worth having a human readable version if it's only accessed through libraries?
// TODO: Binary representation?
#[derive(Debug, Serialize, Deserialize)]
pub struct GBufferDatabase {
    /// The `.wismt` file name without the extension and shader data for each file.
    pub files: IndexMap<String, File>,
}

/// The decompiled shader data for a single `.wismt` model file.
#[derive(Debug, Serialize, Deserialize)]
pub struct File {
    pub shaders: IndexMap<String, Shader>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Shader {
    // TODO: Should dependencies be more strongly typed?
    // It seems redundant to do string -> struct -> string on save.
    // Applications will always want to parse this anyway.
    // TODO: Add strings as an optional export option?
    /// The buffer elements, textures, and constants used to initialize each fragment output.
    ///
    /// This assumes inputs are assigned directly to outputs without any modifications.
    /// Fragment shaders typically only perform basic input and channel selection in practice.
    pub output_dependencies: IndexMap<String, Vec<String>>,
}

impl Shader {
    fn from_glsl(source: &str) -> Self {
        // Only parse the source code once.
        // TODO: Will naga's glsl frontend be faster or easier to use?
        let translation_unit = &ShaderStage::parse(source).unwrap();

        // Get the textures used to initialize each fragment output channel.
        // Unused outputs will have an empty dependency list.
        Self {
            // IndexMap gives consistent ordering for attribute names.
            output_dependencies: (0..=5)
                .flat_map(|i| {
                    "xyzw".chars().map(move |c| {
                        // TODO: Handle cases like "out_attr1.w = 0.00823529344;"
                        // TODO: Handle cases like "out_attr1.z = fp_c4_data[0].z;"
                        // TODO: Handle cases with vertex color assignments.
                        // TODO: Handle cases with multiple operations before assignment?
                        // TODO: Tests for the above?
                        let name = format!("out_attr{i}.{c}");
                        // Make ordering consistent across channels if possible.
                        let mut dependencies: Vec<_> = input_dependencies(translation_unit, &name)
                            .into_iter()
                            .map(|d| d.to_string())
                            .collect();
                        dependencies.sort();

                        (name, dependencies)
                    })
                })
                .filter(|(_, dependencies)| !dependencies.is_empty())
                .collect(),
        }
    }

    /// Find the sampler and channel index of the first material sampler like "s0" or "s3" assigned to the output.
    pub fn material_channel_assignment(
        &self,
        output_index: usize,
        channel: char,
    ) -> Option<(u32, u32)> {
        let output = format!("out_attr{output_index}.{channel}");

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
        let channel_index = "xyzw".find(c).unwrap() as u32;
        Some((sampler_index, channel_index))
    }
}

/// Find the texture dependencies for each fragment output channel.
pub fn create_shader_database(input: &str) -> GBufferDatabase {
    // TODO: BTreeMap to sort?
    let files = std::fs::read_dir(input)
        .unwrap()
        .par_bridge()
        .map(|entry| {
            let path = entry.unwrap().path();

            // Process all fragment shaders.
            let shaders = globwalk::GlobWalkerBuilder::from_patterns(&path, &["*FS*.glsl"])
                .build()
                .unwrap()
                .par_bridge()
                .map(|entry| {
                    // TODO: Add FS0 and FS1 to the same parent entry?
                    // TODO: Add shaders in order by index for easier access using mxmd data?
                    let path = entry.as_ref().unwrap().path();
                    let name = path.file_name().unwrap().to_string_lossy().to_string();
                    let source = std::fs::read_to_string(path).unwrap();
                    (name, Shader::from_glsl(&source))
                })
                .collect();

            let file = path.file_name().unwrap().to_string_lossy().to_string();
            (file, File { shaders })
        })
        .collect();

    GBufferDatabase { files }
}

fn material_sampler_index(sampler: &str) -> Option<u32> {
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
        assert_eq!(None, shader.material_channel_assignment(0, 'x'));
    }

    #[test]
    fn material_channel_assignment_single_output_no_assignment() {
        let shader = Shader {
            output_dependencies: [("out_attr0.x".to_string(), Vec::new())].into(),
        };
        assert_eq!(None, shader.material_channel_assignment(0, 'x'));
    }

    #[test]
    fn material_channel_assignment_multiple_output_assignment() {
        let shader = Shader {
            output_dependencies: [
                ("out_attr0.x".to_string(), vec!["s0.y".to_string()]),
                (
                    "out_attr0.y".to_string(),
                    vec!["tex.xyz".to_string(), "s2.z".to_string()],
                ),
                ("out_attr1.x".to_string(), vec!["s3.xyz".to_string()]),
            ]
            .into(),
        };
        assert_eq!(Some((2, 2)), shader.material_channel_assignment(0, 'y'));
    }
}
