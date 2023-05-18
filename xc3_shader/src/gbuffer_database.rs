use glsl::{parser::Parse, syntax::ShaderStage};
use indexmap::IndexMap;
use rayon::prelude::*;
use serde::Serialize;

use crate::dependencies::texture_dependencies;

#[derive(Debug, Serialize)]
pub struct File {
    file: String,
    shaders: Vec<Shader>,
}

#[derive(Debug, Serialize)]
pub struct Shader {
    name: String,
    output_dependencies: IndexMap<String, Vec<String>>,
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

#[derive(Debug, Serialize)]
pub struct Output {
    x: Vec<String>,
    y: Vec<String>,
    z: Vec<String>,
    w: Vec<String>,
}

/// Find the texture dependencies for each fragment output channel.
pub fn create_shader_database(input: &str) -> Vec<File> {
    // TODO: process folders in parallel as well?
    let mut files = Vec::new();
    for entry in std::fs::read_dir(input).unwrap() {
        let path = entry.unwrap().path();

        // Process all fragment shaders.
        let mut shaders: Vec<_> = globwalk::GlobWalkerBuilder::from_patterns(&path, &["*FS*.glsl"])
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

        let file = File {
            file: path.file_name().unwrap().to_string_lossy().to_string(),
            shaders,
        };
        files.push(file);
    }
    files.sort_by(|a, b| a.file.partial_cmp(&b.file).unwrap());

    files
}
