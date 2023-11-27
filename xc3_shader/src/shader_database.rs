use std::path::Path;

use glsl_lang::{ast::TranslationUnit, parse::DefaultParse};
use log::error;
use rayon::prelude::*;
use xc3_model::shader_database::{Map, Shader, ShaderDatabase, ShaderProgram, Spch};

use crate::dependencies::input_dependencies;

fn shader_from_glsl(translation_unit: &TranslationUnit) -> Shader {
    // Get the textures used to initialize each fragment output channel.
    // Unused outputs will have an empty dependency list.
    Shader {
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

/// Find the texture dependencies for each fragment output channel.
pub fn create_shader_database(input: &str) -> ShaderDatabase {
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

    ShaderDatabase { files, map_files }
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
    // Only check the first shader for now.
    // TODO: What do additional shader entries do?
    let mut paths: Vec<_> = globwalk::GlobWalkerBuilder::from_patterns(folder, &["*FS0.glsl"])
        .build()
        .unwrap()
        .filter_map(|e| e.map(|e| e.path().to_owned()).ok())
        .collect();

    // Shaders are generated as "slct{program_index}_FS{i}.glsl".
    // Sort by {program_index} to process files in the right order.
    // TODO: Find a simpler way of doing this?
    paths.sort_by_cached_key(|p| extract_program_index(p));

    paths
        .par_iter()
        .filter_map(|path| {
            let source = std::fs::read_to_string(path).unwrap();
            // Only parse the source code once.
            // let modified_source = shader_source_no_extensions(source.to_string());
            match TranslationUnit::parse(&source) {
                Ok(translation_unit) => Some(
                    // TODO: Add FS0 and FS1 to the same program?
                    ShaderProgram {
                        shaders: vec![shader_from_glsl(&translation_unit)],
                    },
                ),
                Err(e) => {
                    error!("Error parsing {path:?}: {e}");
                    None
                }
            }
        })
        .collect()
}

fn extract_program_index(p: &Path) -> usize {
    let name = p.file_name().unwrap().to_string_lossy();
    let start = "slct".len();
    let end = name.find('_').unwrap();
    name[start..end].parse::<usize>().unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_program_index_multiple_digits() {
        assert_eq!(
            89,
            extract_program_index(Path::new("xc3_shader_dump/ch01027000/nvsd89_FS1.glsl"))
        )
    }
}
