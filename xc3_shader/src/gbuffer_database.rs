use glsl::{parser::Parse, syntax::ShaderStage};
use indexmap::IndexMap;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::dependencies::input_dependencies;

// TODO: How much extra space does this take up?
// TODO: Is it worth having a human readable version if it's only accessed through libraries?
// TODO: Binary representation?
// TODO: Store a struct for the top level?
#[derive(Debug, Serialize, Deserialize)]
pub struct File {
    pub file: String,
    pub shaders: Vec<Shader>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Shader {
    pub name: String,
    // TODO: Should dependencies be more strongly typed?
    // It seems redundant to do string -> struct -> string on save.
    // Applications will always want to parse this anyway.
    // TODO: Add strings as an optional export option?
    pub output_dependencies: IndexMap<String, Vec<String>>,
}

impl Shader {
    fn from_glsl(name: String, source: &str) -> Self {
        // Only parse the source code once.
        // TODO: Will naga's glsl frontend be faster or easier to use?
        let translation_unit = &ShaderStage::parse(source).unwrap();

        // Get the textures used to initialize each fragment output channel.
        // Unused outputs will have an empty dependency list.
        Self {
            name,
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
                .collect(),
        }
    }
}

/// Find the texture dependencies for each fragment output channel.
pub fn create_shader_database(input: &str) -> Vec<File> {
    let mut files: Vec<_> = std::fs::read_dir(input)
        .unwrap()
        .par_bridge()
        .map(|entry| {
            let path = entry.unwrap().path();

            // Process all fragment shaders.
            let mut shaders: Vec<_> =
                globwalk::GlobWalkerBuilder::from_patterns(&path, &["*FS*.glsl"])
                    .build()
                    .unwrap()
                    .par_bridge()
                    .map(|entry| {
                        let path = entry.as_ref().unwrap().path();
                        let name = path.file_name().unwrap().to_string_lossy().to_string();
                        let source = std::fs::read_to_string(path).unwrap();
                        Shader::from_glsl(name, &source)
                    })
                    .collect();
            shaders.sort_by(|a, b| a.name.partial_cmp(&b.name).unwrap());

            File {
                file: path.file_name().unwrap().to_string_lossy().to_string(),
                shaders,
            }
        })
        .collect();
    files.sort_by(|a, b| a.file.partial_cmp(&b.file).unwrap());

    files
}
