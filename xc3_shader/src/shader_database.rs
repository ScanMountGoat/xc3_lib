use std::path::Path;

use bimap::BiBTreeMap;
use glsl_lang::{
    ast::{
        ExprData, LayoutQualifierSpecData, SingleDeclaration, StorageQualifierData,
        TranslationUnit, TypeQualifierSpecData,
    },
    parse::DefaultParse,
    visitor::{Host, Visit, Visitor},
};
use log::error;
use rayon::prelude::*;
use xc3_model::shader_database::{Dependency, Map, Shader, ShaderDatabase, ShaderProgram, Spch};

use crate::{
    annotation::shader_source_no_extensions,
    dependencies::{
        attribute_dependencies, find_buffer_parameters, input_dependencies, line_dependencies,
    },
};

fn shader_from_glsl(vertex: Option<&TranslationUnit>, fragment: &TranslationUnit) -> Shader {
    // Get the textures used to initialize each fragment output channel.
    // Unused outputs will have an empty dependency list.
    Shader {
        // IndexMap gives consistent ordering for attribute names.
        output_dependencies: (0..=5)
            .flat_map(|i| {
                "xyzw".chars().map(move |c| {
                    // TODO: Handle cases with multiple operations before assignment?
                    // TODO: Avoid calling dependency functions more than once to improve performance.

                    let name = format!("out_attr{i}.{c}");
                    let mut dependencies = input_dependencies(fragment, &name);

                    if let Some(vertex) = vertex {
                        // Add texture parameters used for the corresponding vertex output.
                        // Most shaders apply UV transforms in the vertex shader.
                        apply_vertex_texcoord_params(vertex, fragment, &mut dependencies);

                        apply_attribute_names(vertex, fragment, &mut dependencies);
                    }

                    // Simplify the output name to save space.
                    let output_name = format!("o{i}.{c}");
                    (output_name, dependencies)
                })
            })
            .filter(|(_, dependencies)| !dependencies.is_empty())
            .collect(),
    }
}

fn apply_vertex_texcoord_params(
    vertex: &TranslationUnit,
    fragment: &TranslationUnit,
    dependencies: &mut [Dependency],
) {
    let vertex_attributes = find_attribute_locations(vertex);
    let fragment_attributes = find_attribute_locations(fragment);

    for dependency in dependencies {
        if let Dependency::Texture(texture) = dependency {
            if let Some(texcoord) = &mut texture.texcoord {
                // Convert a fragment input like "in_attr4" to its vertex output like "vTex0".
                if let Some(fragment_location) = fragment_attributes
                    .input_locations
                    .get_by_left(&texcoord.name)
                {
                    if let Some(vertex_output_name) = vertex_attributes
                        .output_locations
                        .get_by_right(fragment_location)
                    {
                        if let Some(actual_name) = find_texcoord_input_name(
                            texcoord,
                            vertex,
                            vertex_output_name,
                            &vertex_attributes,
                        ) {
                            texcoord.name = actual_name;
                        }

                        // Preserve the channel ordering here.
                        for c in texcoord.channels.chars() {
                            let output = format!("{vertex_output_name}.{c}");
                            let vertex_params = find_buffer_parameters(vertex, &output);
                            texcoord.params.extend(vertex_params);
                        }
                        // Remove any duplicates shared by multiple channels.
                        texcoord.params.sort();
                        texcoord.params.dedup();
                    }
                }
            }
        }
    }
}

// TODO: Share code with texcoord function.
fn apply_attribute_names(
    vertex: &TranslationUnit,
    fragment: &TranslationUnit,
    dependencies: &mut [Dependency],
) {
    let vertex_attributes = find_attribute_locations(vertex);
    let fragment_attributes = find_attribute_locations(fragment);

    for dependency in dependencies {
        if let Dependency::Attribute(attribute) = dependency {
            // Convert a fragment input like "in_attr4" to its vertex output like "vTex0".
            if let Some(fragment_location) = fragment_attributes
                .input_locations
                .get_by_left(&attribute.name)
            {
                if let Some(vertex_output_name) = vertex_attributes
                    .output_locations
                    .get_by_right(fragment_location)
                {
                    for c in attribute.channels.chars() {
                        let output = format!("{vertex_output_name}.{c}");
                        if let Some(vertex_dependencies) = line_dependencies(vertex, &output) {
                            if let Some(input_attribute) = attribute_dependencies(
                                &vertex_dependencies,
                                &vertex_attributes,
                                None,
                            )
                            .first()
                            {
                                attribute.name = input_attribute.name.clone();
                            }
                        }
                    }
                }
            }
        }
    }
}

fn find_texcoord_input_name(
    texcoord: &xc3_model::shader_database::TexCoord,
    vertex: &TranslationUnit,
    vertex_output_name: &str,
    vertex_attributes: &Attributes,
) -> Option<String> {
    // Assume only one texcoord attribute is used for all components.
    let c = texcoord.channels.chars().next()?;
    let output = format!("{vertex_output_name}.{c}");
    let vertex_dependencies = line_dependencies(&vertex, &output)?;
    attribute_dependencies(&vertex_dependencies, vertex_attributes, None)
        .first()
        .map(|a| a.name.clone())
}

/// Find the texture dependencies for each fragment output channel.
pub fn create_shader_database(input: &str) -> ShaderDatabase {
    // Sort to make the output consistent.
    let mut folders: Vec<_> = std::fs::read_dir(input)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    folders.sort();

    let files = folders
        .par_iter()
        .filter_map(|folder| {
            // TODO: Find a better way to detect maps.
            if !folder.join("map").exists() {
                let programs = create_shader_programs(folder);

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
    // TODO: What do additional nvsd shader entries do?
    let mut paths: Vec<_> = globwalk::GlobWalkerBuilder::from_patterns(folder, &["*nvsd0*.frag"])
        .build()
        .unwrap()
        .filter_map(|e| e.map(|e| e.path().to_owned()).ok())
        .collect();

    // Shaders are generated as "slct{program_index}_nvsd{i}_{name}.glsl".
    // Sort by {program_index} to process files in the right order.
    paths.sort_by_cached_key(|p| extract_program_index(p));

    paths
        .par_iter()
        .filter_map(|path| {
            // TODO: Should the vertex shader be mandatory?
            let vertex_source = std::fs::read_to_string(path.with_extension("vert")).ok();
            let vertex = vertex_source.and_then(|s| {
                let source = shader_source_no_extensions(&s);
                match TranslationUnit::parse(source) {
                    Ok(vertex) => Some(vertex),
                    Err(e) => {
                        error!("Error parsing {path:?}: {e}");
                        None
                    }
                }
            });

            let frag_source = std::fs::read_to_string(path).unwrap();
            let frag_source = shader_source_no_extensions(&frag_source);
            match TranslationUnit::parse(frag_source) {
                Ok(fragment) => Some(ShaderProgram {
                    shaders: vec![shader_from_glsl(vertex.as_ref(), &fragment)],
                }),
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

// TODO: module for this?
#[derive(Debug, Default)]
struct AttributeVisitor {
    attributes: Attributes,
}

#[derive(Debug, Default, PartialEq)]
pub struct Attributes {
    pub input_locations: BiBTreeMap<String, i32>,
    pub output_locations: BiBTreeMap<String, i32>,
}

impl Visitor for AttributeVisitor {
    fn visit_single_declaration(&mut self, declaration: &SingleDeclaration) -> Visit {
        if let Some(name) = &declaration.name {
            if let Some(qualifier) = &declaration.ty.content.qualifier {
                let mut is_input = None;
                let mut location = None;

                for q in &qualifier.qualifiers {
                    match &q.content {
                        TypeQualifierSpecData::Storage(storage) => match &storage.content {
                            StorageQualifierData::In => {
                                is_input = Some(true);
                            }
                            StorageQualifierData::Out => {
                                is_input = Some(false);
                            }
                            _ => (),
                        },
                        TypeQualifierSpecData::Layout(layout) => {
                            if let Some(id) = layout.content.ids.first() {
                                if let LayoutQualifierSpecData::Identifier(key, value) = &id.content
                                {
                                    if key.0 == "location" {
                                        if let Some(ExprData::IntConst(i)) =
                                            value.as_ref().map(|v| &v.content)
                                        {
                                            location = Some(*i);
                                        }
                                    }
                                }
                            }
                        }
                        _ => (),
                    }
                }

                if let (Some(is_input), Some(location)) = (is_input, location) {
                    if is_input {
                        self.attributes
                            .input_locations
                            .insert(name.0.to_string(), location);
                    } else {
                        self.attributes
                            .output_locations
                            .insert(name.0.to_string(), location);
                    }
                }
            }
        }

        Visit::Children
    }
}

pub fn find_attribute_locations(translation_unit: &TranslationUnit) -> Attributes {
    let mut visitor = AttributeVisitor::default();
    translation_unit.visit(&mut visitor);
    visitor.attributes
}

#[cfg(test)]
mod tests {
    use super::*;

    use indoc::indoc;
    use xc3_model::shader_database::{
        AttributeDependency, BufferDependency, TexCoord, TextureDependency,
    };

    #[test]
    fn extract_program_index_multiple_digits() {
        assert_eq!(
            89,
            extract_program_index(Path::new(
                "xc3_shader_dump/ch01027000/slct89_nvsd0_shd0089.frag"
            ))
        );
        assert_eq!(
            89,
            extract_program_index(Path::new("xc3_shader_dump/ch01027000/slct89_nvsd1.frag"))
        );
    }

    #[test]
    fn find_attribute_locations_outputs() {
        let glsl = indoc! {"
            layout(location = 0) in vec4 in_attr0;
            layout(location = 4) in vec4 in_attr1;
            layout(location = 3) in vec4 in_attr2;

            layout(location = 3) out vec4 out_attr0;
            layout(location = 5) out vec4 out_attr1;
            layout(location = 7) out vec4 out_attr2;

            void main() {}
        "};

        let tu = TranslationUnit::parse(glsl).unwrap();
        assert_eq!(
            Attributes {
                input_locations: [
                    ("in_attr0".to_string(), 0),
                    ("in_attr1".to_string(), 4),
                    ("in_attr2".to_string(), 3)
                ]
                .into_iter()
                .collect(),
                output_locations: [
                    ("out_attr0".to_string(), 3),
                    ("out_attr1".to_string(), 5),
                    ("out_attr2".to_string(), 7)
                ]
                .into_iter()
                .collect(),
            },
            find_attribute_locations(&tu)
        );
    }

    #[test]
    fn shader_from_vertex_fragment() {
        let glsl = indoc! {"
            layout(location = 0) in vec4 pos;
            layout(location = 4) in vec4 tex0;
            layout(location = 3) in vec4 color;

            layout(location = 3) out vec4 out_attr0;
            layout(location = 5) out vec4 out_attr1;
            layout(location = 7) out vec4 out_attr2;

            void main() 
            {
                float temp_0 = tex0.z;
                float temp_1 = tex0.w;
                float temp_2 = temp_0 * U_Mate.gWrkFl4[0].x;
                float temp_3 = temp_1 * U_Mate.gWrkFl4[0].y;
                out_attr0.x = 1.0;
                out_attr0.y = 2.0;
                out_attr0.z = 3.0;
                out_attr0.w = 4.0;
                out_attr1.x = temp_2;
                out_attr1.y = temp_3;
                out_attr1.z = 0.0;
                out_attr1.w = 0.0;
                out_attr2.x = 1.0;
                out_attr2.y = 2.0;
                out_attr2.z = 3.0;
                out_attr2.w = 4.0;
            }
        "};
        let vertex = TranslationUnit::parse(glsl).unwrap();

        let glsl = indoc! {"
            layout(location = 3) in vec4 in_attr0;
            layout(location = 5) in vec4 in_attr1;
            layout(location = 7) in vec4 in_attr2;

            layout(location = 0) out vec4 out_attr0;
            layout(location = 1) out vec4 out_attr1;
            layout(location = 2) out vec4 out_attr2;

            void main() 
            {
                float temp_0 = in_attr1.x;
                float temp_1 = in_attr1.y;
                float a = texture(texture1, vec2(temp_0, temp_1)).x;
                out_attr1.x = a;
                out_attr1.y = U_Mate.data[1].w;
                out_attr1.z = uniform_data[3].y;
                out_attr1.w = temp_1;
            }
        "};

        let fragment = TranslationUnit::parse(glsl).unwrap();

        let shader = shader_from_glsl(Some(&vertex), &fragment);
        assert_eq!(
            Shader {
                output_dependencies: [
                    (
                        "o1.x".to_string(),
                        vec![Dependency::Texture(TextureDependency {
                            name: "texture1".to_string(),
                            channels: "x".to_string(),
                            texcoord: Some(TexCoord {
                                name: "tex0".to_string(),
                                channels: "xy".to_string(),
                                params: vec![
                                    BufferDependency {
                                        name: "U_Mate".to_string(),
                                        field: "gWrkFl4".to_string(),
                                        index: 0,
                                        channels: "x".to_string()
                                    },
                                    BufferDependency {
                                        name: "U_Mate".to_string(),
                                        field: "gWrkFl4".to_string(),
                                        index: 0,
                                        channels: "y".to_string()
                                    }
                                ]
                            })
                        }),]
                    ),
                    (
                        "o1.y".to_string(),
                        vec![Dependency::Buffer(BufferDependency {
                            name: "U_Mate".to_string(),
                            field: "data".to_string(),
                            index: 1,
                            channels: "w".to_string()
                        })],
                    ),
                    (
                        "o1.z".to_string(),
                        vec![Dependency::Buffer(BufferDependency {
                            name: "uniform_data".to_string(),
                            field: "".to_string(),
                            index: 3,
                            channels: "y".to_string()
                        })]
                    ),
                    (
                        "o1.w".to_string(),
                        vec![Dependency::Attribute(AttributeDependency {
                            name: "tex0".to_string(),
                            channels: "y".to_string()
                        })]
                    )
                ]
                .into()
            },
            shader
        );
    }
}
