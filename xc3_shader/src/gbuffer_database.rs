use std::path::Path;

use glsl_lang::{ast::TranslationUnit, parse::DefaultParse};
use indexmap::IndexMap;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::dependencies::input_dependencies;

// TODO: How much extra space does JSON take up?
// TODO: Is it worth having a human readable version if it's only accessed through libraries?
// TODO: Binary representation?
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct GBufferDatabase {
    /// The `.wismt` file name without the extension and shader data for each file.
    pub files: IndexMap<String, Spch>,
    // TODO: Put maps here?
    pub map_files: IndexMap<String, Map>,
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
    // TODO: Is it worth including the program name?
    /// Some shaders have multiple NVSD sections, so the length may be greater than 1.
    pub shaders: Vec<Shader>,
}

/// The buffer elements, textures, and constants used to initialize each fragment output.
///
/// This assumes inputs are assigned directly to outputs without any modifications.
/// Fragment shaders typically only perform basic input and channel selection in practice.
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Shader {
    pub output_dependencies: IndexMap<String, Vec<String>>,
}

impl Shader {
    fn from_glsl(source: &str) -> Self {
        // TODO: Find a better way to skip unsupported extensions.
        let modified_source = source.get(source.find("#pragma").unwrap()..).unwrap();
        // Only parse the source code once.
        let translation_unit = &TranslationUnit::parse(modified_source).unwrap();

        // Get the textures used to initialize each fragment output channel.
        // Unused outputs will have an empty dependency list.
        Self {
            // IndexMap gives consistent ordering for attribute names.
            output_dependencies: (0..=5)
                .flat_map(|i| {
                    "xyzw".chars().map(move |c| {
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

                        // Simplify the output name to save space.
                        let output_name = format!("o{i}.{c}");
                        (output_name, dependencies)
                    })
                })
                .filter(|(_, dependencies)| !dependencies.is_empty())
                .collect(),
        }
    }

    /// Find the sampler and channel index of the first material sampler assigned to the output.
    ///
    /// For example, an assignment of `"s3.y"` results in a sampler index of `3` and a channel index of `1`.
    pub fn sampler_channel_index(&self, output_index: usize, channel: char) -> Option<(u32, u32)> {
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
        let channel_index = "xyzw".find(c).unwrap() as u32;
        Some((sampler_index, channel_index))
    }
}

/// Find the texture dependencies for each fragment output channel.
pub fn create_shader_database(input: &str) -> GBufferDatabase {
    let files = std::fs::read_dir(input)
        .unwrap()
        .par_bridge()
        .filter_map(|entry| {
            let path = entry.unwrap().path();
            // TODO: Find a better way to detect maps.
            if !path.join("map").exists() {
                let programs = create_shader_programs(&path);

                let file = path.file_name().unwrap().to_string_lossy().to_string();
                Some((file, Spch { programs }))
            } else {
                None
            }
        })
        .collect();

    let map_files = std::fs::read_dir(input)
        .unwrap()
        .par_bridge()
        .filter_map(|entry| {
            let path = entry.unwrap().path();
            // TODO: Find a better way to detect maps.
            if path.join("map").exists() {
                let map_models = create_map_spchs(&path.join("map"));
                let prop_models = create_map_spchs(&path.join("prop"));
                let env_models = create_map_spchs(&path.join("env"));

                let file = path.file_name().unwrap().to_string_lossy().to_string();
                Some((
                    file,
                    Map {
                        map_models,
                        prop_models,
                        env_models,
                    },
                ))
            } else {
                None
            }
        })
        .collect();

    GBufferDatabase { files, map_files }
}

fn create_map_spchs(folder: &Path) -> Vec<Spch> {
    // TODO: Not all maps have env or prop models?
    std::fs::read_dir(folder)
        .map(|dir| {
            dir.into_iter()
                .map(|entry| Spch {
                    programs: create_shader_programs(&entry.unwrap().path()),
                })
                .collect()
        })
        .unwrap_or_default()
}

fn create_shader_programs(folder: &Path) -> Vec<ShaderProgram> {
    // Shaders are generated as nvsd{program_index}.
    // Sort by file name to process files in the right order.
    let mut paths: Vec<_> = globwalk::GlobWalkerBuilder::from_patterns(folder, &["*FS0.glsl"])
        .build()
        .unwrap()
        .filter_map(|e| e.map(|e| e.path().to_owned()).ok())
        .collect();
    paths.sort();

    paths
        .par_iter()
        .map(|path| {
            let source = std::fs::read_to_string(path).unwrap();

            // TODO: Add FS0 and FS1 to the same program?
            ShaderProgram {
                shaders: vec![Shader::from_glsl(&source)],
            }
        })
        .collect()
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
}
