use glsl::{parser::Parse, syntax::ShaderStage};
use indexmap::IndexMap;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::dependencies::texture_dependencies;

// TODO: Store a struct for the top level?
#[derive(Debug, Serialize, Deserialize)]
pub struct File {
    pub file: String,
    pub shaders: Vec<Shader>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Shader {
    pub name: String,
    pub output_dependencies: IndexMap<String, Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Output {
    pub x: Vec<String>,
    pub y: Vec<String>,
    pub z: Vec<String>,
    pub w: Vec<String>,
}

impl Shader {
    fn from_glsl(name: String, source: &str) -> Self {
        // Only parse the source code once.
        let translation_unit = &ShaderStage::parse(source).unwrap();

        // Get the textures used to initialize each fragment output channel.
        // Unused outputs will have an empty dependency list.
        Self {
            name,
            // IndexMap gives consistent ordering for attribute names.
            output_dependencies: (0..=5)
                .flat_map(|i| {
                    "xyzw".chars().map(move |c| {
                        let name = format!("out_attr{i}.{c}");
                        // Make ordering consistent across channels if possible.
                        let mut dependencies = texture_dependencies(translation_unit, &name);
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
    // TODO: process folders in parallel as well?
    let mut files: Vec<_> = std::fs::read_dir(input)
        .unwrap()
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
