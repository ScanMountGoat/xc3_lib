use std::path::Path;

use glsl_lang::{ast::TranslationUnit, parse::DefaultParse};
use log::error;
use rayon::prelude::*;
use xc3_model::shader_database::{Map, Shader, ShaderDatabase, ShaderProgram, Spch};

use crate::{annotation::shader_source_no_extensions, dependencies::input_dependencies};

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
    // Sort to make the output consistent.
    let mut folders: Vec<_> = std::fs::read_dir(input)
        .unwrap()
        .into_iter()
        .map(|e| e.unwrap().path())
        .collect();
    folders.sort();

    let files = folders
        .par_iter()
        .filter_map(|folder| {
            // TODO: Find a better way to detect maps.
            if !folder.join("map").exists() {
                let programs = create_shader_programs(&folder);

                let file = folder.file_name().unwrap().to_string_lossy().to_string();
                Some((file, Spch { programs }))
            } else {
                None
            }
        })
        .collect();

    let map_files = folders
        .par_iter()
        .filter_map(|folder| {
            // TODO: Find a better way to detect maps.
            if folder.join("map").exists() {
                let map_models = create_map_spchs(&folder.join("map"));
                let prop_models = create_map_spchs(&folder.join("prop"));
                let env_models = create_map_spchs(&folder.join("env"));

                let file = folder.file_name().unwrap().to_string_lossy().to_string();
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
    if let Ok(dir) = std::fs::read_dir(folder) {
        // Folders are generated like "ma01a/prop/4".
        // Sort by index to process files in the right order.
        let mut paths: Vec<_> = dir.into_iter().map(|e| e.unwrap().path()).collect();
        paths.sort_by_cached_key(|p| extract_folder_index(p));

        paths
            .into_iter()
            .map(|path| Spch {
                programs: create_shader_programs(&path),
            })
            .collect()
    } else {
        Vec::new()
    }
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
    paths.sort_by_cached_key(|p| extract_program_index(p));

    paths
        .par_iter()
        .filter_map(|path| {
            let source = std::fs::read_to_string(path).unwrap();
            let modified_source = shader_source_no_extensions(&source);
            match TranslationUnit::parse(&modified_source) {
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

fn extract_folder_index(p: &Path) -> usize {
    let name = p.file_name().unwrap().to_string_lossy();
    name.parse::<usize>().unwrap()
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
