use std::{ops::Deref, path::Path};

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
use xc3_model::shader_database::{
    BufferDependency, Dependency, Map, Shader, ShaderDatabase, ShaderProgram, Spch,
};

use crate::{
    annotation::shader_source_no_extensions,
    dependencies::{
        attribute_dependencies, buffer_dependency, find_buffer_parameters, input_dependencies,
    },
    graph::{
        query::{clamp_x_zero_one, one_minus_x, one_plus_x, sqrt_x},
        Expr, Graph,
    },
};

fn shader_from_glsl(vertex: Option<&TranslationUnit>, fragment: &TranslationUnit) -> Shader {
    let output_dependencies = (0..=5)
        .flat_map(|i| {
            "xyzw".chars().map(move |c| {
                // TODO: Handle cases with multiple operations before assignment?
                // TODO: Avoid calling dependency functions more than once to improve performance.

                // TODO: Split this?
                let name = format!("out_attr{i}.{c}");
                let mut dependencies = input_dependencies(fragment, &name);

                if let Some(vertex) = vertex {
                    // Add texture parameters used for the corresponding vertex output.
                    // Most shaders apply UV transforms in the vertex shader.
                    apply_vertex_texcoord_params(vertex, fragment, &mut dependencies);

                    apply_attribute_names(vertex, fragment, &mut dependencies);
                }

                // TODO: set o1.y to account for specular AA if the graph matches.
                // We only need the glossiness input for now.
                // TODO: is specular AA ever used with textures as input?
                if i == 1 && c == 'y' {
                    if let Some(param) = apply_geometric_specular_aa(fragment) {
                        dependencies = vec![Dependency::Buffer(param)];
                    }
                }

                // Simplify the output name to save space.
                let output_name = format!("o{i}.{c}");
                (output_name, dependencies)
            })
        })
        .filter(|(_, dependencies)| !dependencies.is_empty())
        .collect();

    Shader {
        // IndexMap gives consistent ordering for attribute names.
        output_dependencies,
    }
}

fn apply_geometric_specular_aa(fragment: &TranslationUnit) -> Option<BufferDependency> {
    // TODO: Avoid constructing the graph more than once.
    // Extract the glossiness input from the following expression.
    // glossiness = 1.0 - sqrt(clamp((1.0 - glossiness)^2 + kernelRoughness2 0.0, 1.0))
    let graph = Graph::from_glsl(&fragment);
    let node_index = graph
        .nodes
        .iter()
        .rposition(|n| n.output.name == "out_attr1" && n.output.channels == "y")?;
    let nodes: Vec<_> = graph
        .node_assignments_recursive(node_index, None)
        .into_iter()
        .map(|(i, _)| &graph.nodes[i])
        .collect();
    // TODO: Define query function or macro?
    let node = match &nodes.last()?.input {
        Expr::Node { node_index, .. } => graph.nodes.get(*node_index),
        _ => None,
    }?;
    let node = one_minus_x(&graph.nodes, node)?;
    let node = sqrt_x(&graph.nodes, node)?;
    let node = clamp_x_zero_one(&graph.nodes, node)?;
    let node = match &node.input {
        Expr::Func { name, args, .. } => {
            if name == "fma" {
                match &args[..] {
                    [Expr::Node { node_index: i1, .. }, Expr::Node { node_index: i2, .. }, Expr::Node { .. }] => {
                        if i1 == i2 {
                            graph.nodes.get(*i1)
                        } else {
                            None
                        }
                    }
                    _ => None,
                }
            } else {
                None
            }
        }
        _ => None,
    }?;
    let node = one_plus_x(&graph.nodes, node)?;
    // TODO: Will this final node ever not be a parameter?
    // TODO: Add an option to get the expr itself?
    match &node.input {
        Expr::Sub(a, b) => match (a.deref(), b.deref()) {
            (Expr::Float(0.0), e) => buffer_dependency(e, ""),
            _ => None,
        },
        _ => None,
    }
}

fn apply_vertex_texcoord_params(
    vertex: &TranslationUnit,
    fragment: &TranslationUnit,
    dependencies: &mut [Dependency],
) {
    let vertex_attributes = find_attribute_locations(vertex);
    let fragment_attributes = find_attribute_locations(fragment);

    let vertex_graph = Graph::from_glsl(vertex);

    for dependency in dependencies {
        if let Dependency::Texture(texture) = dependency {
            for texcoord in &mut texture.texcoords {
                // Convert a fragment input like "in_attr4" to its vertex output like "vTex0".
                if let Some(fragment_location) = fragment_attributes
                    .input_locations
                    .get_by_left(&texcoord.name)
                {
                    if let Some(vertex_output_name) = vertex_attributes
                        .output_locations
                        .get_by_right(fragment_location)
                    {
                        // Preserve the channel ordering here.
                        for c in texcoord.channels.chars() {
                            let vertex_params = find_buffer_parameters(
                                &vertex_graph,
                                vertex_output_name,
                                &c.to_string(),
                            );
                            texcoord.params.extend(vertex_params);
                        }
                        // Remove any duplicates.
                        texcoord.params.sort();
                        texcoord.params.dedup();

                        // Also fix channels since the zw output may just be scaled vTex0.xy.
                        if let Some((actual_name, actual_channels)) =
                            find_texcoord_input_name_channels(
                                texcoord,
                                vertex,
                                vertex_output_name,
                                &vertex_attributes,
                            )
                        {
                            texcoord.name = actual_name;
                            texcoord.channels = actual_channels;
                        }
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
                        let graph = Graph::from_glsl(vertex);
                        if let Some(input_attribute) = attribute_dependencies(
                            &graph,
                            vertex_output_name,
                            &c.to_string(),
                            &vertex_attributes,
                            None,
                        )
                        .first()
                        {
                            attribute.name.clone_from(&input_attribute.name);
                        }
                    }
                }
            }
        }
    }
}

fn find_texcoord_input_name_channels(
    texcoord: &xc3_model::shader_database::TexCoord,
    vertex: &TranslationUnit,
    vertex_output_name: &str,
    vertex_attributes: &Attributes,
) -> Option<(String, String)> {
    // We only need to look up one output per texcoord.
    let c = texcoord.channels.chars().next()?;

    let graph = Graph::from_glsl(vertex);
    attribute_dependencies(
        &graph,
        vertex_output_name,
        &c.to_string(),
        vertex_attributes,
        None,
    )
    .first()
    .map(|a| (a.name.clone(), a.channels.clone()))
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
    use pretty_assertions::assert_eq;
    use xc3_model::shader_database::{BufferDependency, TexCoord, TextureDependency};

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
    fn shader_from_vertex_fragment_pyra_body() {
        // Test trimmed shaders from Pyra's metallic chest material.
        // xeno2/bl/bl000101, "ho_BL_TS2", shd0022.vert
        let glsl = indoc! {"
            layout(location = 0) in vec4 vPos;
            layout(location = 1) in vec4 nWgtIdx;
            layout(location = 2) in vec4 vTex0;
            layout(location = 3) in vec4 vColor;
            layout(location = 4) in vec4 vNormal;
            layout(location = 5) in vec4 vTan;
            layout(location = 0) out vec4 out_attr0;
            layout(location = 1) out vec4 out_attr1;
            layout(location = 2) out vec4 out_attr2;
            layout(location = 3) out vec4 out_attr3;
            layout(location = 4) out vec4 out_attr4;
            layout(location = 5) out vec4 out_attr5;
            layout(location = 6) out vec4 out_attr6;
            layout(location = 7) out vec4 out_attr7;
            layout(location = 8) out vec4 out_attr8;

            void main() 
            {
                temp_62 = vTex0.x;
                temp_64 = vTex0.y;
                temp_119 = temp_62 * U_Mate.gWrkFl4[0].x;
                temp_179 = temp_64 * U_Mate.gWrkFl4[0].y;
                out_attr4.x = temp_62;
                out_attr4.y = temp_64;
                out_attr4.z = temp_119;
                out_attr4.w = temp_179;
            }
        "};
        let vertex = TranslationUnit::parse(glsl).unwrap();

        // xeno2/bl/bl000101, "ho_BL_TS2", shd0022.frag
        let glsl = indoc! {"
            layout(location = 0) in vec4 in_attr0;
            layout(location = 1) in vec4 in_attr1;
            layout(location = 2) in vec4 in_attr2;
            layout(location = 3) in vec4 in_attr3;
            layout(location = 4) in vec4 in_attr4;
            layout(location = 5) in vec4 in_attr5;
            layout(location = 6) in vec4 in_attr6;
            layout(location = 7) in vec4 in_attr7;
            layout(location = 0) out vec4 out_attr0;
            layout(location = 1) out vec4 out_attr1;
            layout(location = 2) out vec4 out_attr2;
            layout(location = 3) out vec4 out_attr3;
            layout(location = 4) out vec4 out_attr4;
            layout(location = 5) out vec4 out_attr5;

            void main() 
            {
                temp_1 = in_attr4.x;
                temp_2 = in_attr4.y;
                temp_15 = in_attr4.z;
                temp_16 = in_attr4.w;
                temp_17 = texture(s5, vec2(temp_15, temp_16)).xyz;
                temp_18 = temp_17.x;
                temp_19 = temp_17.y;
                temp_20 = temp_17.z;
                temp_21 = texture(s4, vec2(temp_1, temp_2)).xy;
                temp_22 = temp_21.x;
                temp_23 = temp_21.y;
                out_attr1.x = temp_23;
                out_attr1.y = U_Mate.gWrkFl4[2].x;
                out_attr1.z = U_Mate.gWrkFl4[1].y;
                out_attr1.w = 0.07098039;
                out_attr5.x = temp_18;
                out_attr5.y = temp_19;
                out_attr5.z = temp_20;
                out_attr5.w = 0.;
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
                            name: "s4".to_string(),
                            channels: "y".to_string(),
                            texcoords: vec![
                                TexCoord {
                                    name: "vTex0".to_string(),
                                    channels: "x".to_string(),
                                    params: Vec::new(),
                                },
                                TexCoord {
                                    name: "vTex0".to_string(),
                                    channels: "y".to_string(),
                                    params: Vec::new(),
                                },
                            ]
                        })]
                    ),
                    (
                        "o1.y".to_string(),
                        vec![Dependency::Buffer(BufferDependency {
                            name: "U_Mate".to_string(),
                            field: "gWrkFl4".to_string(),
                            index: 2,
                            channels: "x".to_string(),
                        })]
                    ),
                    (
                        "o1.z".to_string(),
                        vec![Dependency::Buffer(BufferDependency {
                            name: "U_Mate".to_string(),
                            field: "gWrkFl4".to_string(),
                            index: 1,
                            channels: "y".to_string(),
                        })]
                    ),
                    (
                        "o1.w".to_string(),
                        vec![Dependency::Constant(0.07098039.into())]
                    ),
                    (
                        "o5.x".to_string(),
                        vec![Dependency::Texture(TextureDependency {
                            name: "s5".to_string(),
                            channels: "x".to_string(),
                            texcoords: vec![
                                TexCoord {
                                    name: "vTex0".to_string(),
                                    channels: "x".to_string(),
                                    params: vec![BufferDependency {
                                        name: "U_Mate".to_string(),
                                        field: "gWrkFl4".to_string(),
                                        index: 0,
                                        channels: "x".to_string(),
                                    }]
                                },
                                TexCoord {
                                    name: "vTex0".to_string(),
                                    channels: "y".to_string(),
                                    params: vec![BufferDependency {
                                        name: "U_Mate".to_string(),
                                        field: "gWrkFl4".to_string(),
                                        index: 0,
                                        channels: "y".to_string(),
                                    }]
                                },
                            ],
                        })]
                    ),
                    (
                        "o5.y".to_string(),
                        vec![Dependency::Texture(TextureDependency {
                            name: "s5".to_string(),
                            channels: "y".to_string(),
                            texcoords: vec![
                                TexCoord {
                                    name: "vTex0".to_string(),
                                    channels: "x".to_string(),
                                    params: vec![BufferDependency {
                                        name: "U_Mate".to_string(),
                                        field: "gWrkFl4".to_string(),
                                        index: 0,
                                        channels: "x".to_string(),
                                    }]
                                },
                                TexCoord {
                                    name: "vTex0".to_string(),
                                    channels: "y".to_string(),
                                    params: vec![BufferDependency {
                                        name: "U_Mate".to_string(),
                                        field: "gWrkFl4".to_string(),
                                        index: 0,
                                        channels: "y".to_string(),
                                    }]
                                },
                            ],
                        })]
                    ),
                    (
                        "o5.z".to_string(),
                        vec![Dependency::Texture(TextureDependency {
                            name: "s5".to_string(),
                            channels: "z".to_string(),
                            texcoords: vec![
                                TexCoord {
                                    name: "vTex0".to_string(),
                                    channels: "x".to_string(),
                                    params: vec![BufferDependency {
                                        name: "U_Mate".to_string(),
                                        field: "gWrkFl4".to_string(),
                                        index: 0,
                                        channels: "x".to_string(),
                                    }]
                                },
                                TexCoord {
                                    name: "vTex0".to_string(),
                                    channels: "y".to_string(),
                                    params: vec![BufferDependency {
                                        name: "U_Mate".to_string(),
                                        field: "gWrkFl4".to_string(),
                                        index: 0,
                                        channels: "y".to_string(),
                                    }]
                                },
                            ],
                        })]
                    ),
                    ("o5.w".to_string(), vec![Dependency::Constant(0.0.into())]),
                ]
                .into()
            },
            shader
        );
    }

    #[test]
    fn shader_from_vertex_fragment_mio_skirt() {
        // xeno3/chr/ch/ch11021013, "body_skert2", shd0028.frag
        let glsl = indoc! {"
            void main() {
                temp_0 = in_attr3.x;
                temp_1 = in_attr3.y;
                temp_2 = in_attr3.z;
                temp_3 = in_attr3.w;
                temp_8 = texture(s2, vec2(temp_0, temp_1)).xy;
                temp_8 = texture(s2, vec2(temp_0, temp_1)).xy;
                temp_9 = temp_8.x;
                temp_10 = temp_8.y;
                temp_11 = texture(gTResidentTex09, vec2(temp_2, temp_3)).xy;
                temp_11 = texture(gTResidentTex09, vec2(temp_2, temp_3)).xy;
                temp_12 = temp_11.x;
                temp_13 = temp_11.y;
                temp_26 = fma(temp_9, 2.0, 1.0039216);
                temp_27 = fma(temp_10, 2.0, 1.0039216);
                temp_28 = temp_26 * temp_26;
                temp_29 = fma(temp_12, 2.0, 1.0039216);
                temp_30 = fma(temp_13, 2.0, 1.0039216);
                temp_31 = 0.0 + temp_27;
                temp_32 = fma(temp_27, temp_27, temp_28);
                temp_33 = temp_29 * temp_29;
                temp_34 = 0.0 - temp_32;
                temp_35 = temp_34 + 1.0;
                temp_36 = fma(temp_30, temp_30, temp_33);
                temp_37 = sqrt(temp_35);
                temp_38 = 0.0 + temp_26;
                temp_40 = 0.0 - temp_36;
                temp_41 = temp_40 + 1.0;
                temp_42 = 0.0 - temp_38;
                temp_43 = temp_29 * temp_42;
                temp_44 = sqrt(temp_41);
                temp_47 = max(0.0, temp_37);
                temp_48 = 0.0 - temp_31;
                temp_49 = fma(temp_30, temp_48, temp_43);
                temp_51 = temp_47 + 1.0;
                temp_52 = max(0.0, temp_44);
                temp_53 = in_attr1.y;
                temp_54 = 0.0 - temp_51;
                temp_55 = temp_29 * temp_54;
                temp_56 = in_attr1.x;
                temp_57 = fma(temp_52, temp_51, temp_49);
                temp_58 = temp_52 * temp_51;
                temp_59 = 0.0 - temp_55;
                temp_60 = fma(temp_38, temp_57, temp_59);
                temp_61 = 0.0 - temp_51;
                temp_62 = temp_30 * temp_61;
                temp_63 = in_attr1.z;
                temp_64 = 0.0 - temp_58;
                temp_65 = fma(temp_51, temp_57, temp_64);
                temp_66 = 0.0 - temp_62;
                temp_67 = fma(temp_31, temp_57, temp_66);
                temp_68 = temp_56 * temp_56;
                temp_69 = fma(temp_53, temp_53, temp_68);
                temp_70 = temp_60 * temp_60;
                temp_71 = fma(temp_63, temp_63, temp_69);
                temp_72 = fma(temp_67, temp_67, temp_70);
                temp_73 = inversesqrt(temp_71);
                temp_74 = fma(temp_65, temp_65, temp_72);
                temp_75 = in_attr0.x;
                temp_76 = temp_56 * temp_73;
                temp_77 = inversesqrt(temp_74);
                temp_78 = temp_53 * temp_73;
                temp_79 = temp_63 * temp_73;
                temp_80 = 0.0 - temp_26;
                temp_81 = fma(temp_60, temp_77, temp_80);
                temp_82 = 0.0 - temp_27;
                temp_83 = fma(temp_67, temp_77, temp_82);
                temp_84 = in_attr0.y;
                temp_85 = 0.0 - temp_47;
                temp_86 = fma(temp_65, temp_77, temp_85);
                temp_87 = in_attr0.z;
                temp_88 = fma(temp_81, U_Mate.gWrkFl4[1].z, temp_26);
                temp_89 = fma(temp_83, U_Mate.gWrkFl4[1].z, temp_27);
                temp_90 = fma(temp_86, U_Mate.gWrkFl4[1].z, temp_47);
                temp_91 = temp_75 * temp_75;
                temp_92 = temp_88 * temp_88;
                temp_93 = fma(temp_89, temp_89, temp_92);
                temp_94 = fma(temp_84, temp_84, temp_91);
                temp_95 = fma(temp_87, temp_87, temp_94);
                temp_96 = fma(temp_90, temp_90, temp_93);
                temp_97 = inversesqrt(temp_95);
                temp_100 = inversesqrt(temp_96);
                temp_105 = temp_75 * temp_97;
                temp_106 = temp_84 * temp_97;
                temp_107 = temp_87 * temp_97;
                temp_108 = in_attr2.x;
                temp_109 = temp_88 * temp_100;
                temp_110 = temp_90 * temp_100;
                temp_111 = temp_89 * temp_100;
                temp_112 = in_attr2.y;
                temp_114 = temp_109 * temp_76;
                temp_115 = temp_109 * temp_78;
                temp_116 = in_attr2.z;
                temp_117 = temp_109 * temp_79;
                temp_118 = fma(temp_110, temp_105, temp_114);
                temp_120 = fma(temp_110, temp_106, temp_115);
                temp_121 = fma(temp_110, temp_107, temp_117);
                temp_122 = temp_108 * temp_108;
                temp_123 = fma(temp_112, temp_112, temp_122);
                temp_125 = fma(temp_116, temp_116, temp_123);
                temp_126 = inversesqrt(temp_125);
                temp_127 = temp_108 * temp_126;
                temp_128 = temp_112 * temp_126;
                temp_129 = temp_116 * temp_126;
                temp_130 = fma(temp_111, temp_127, temp_118);
                temp_132 = fma(temp_111, temp_128, temp_120);
                temp_133 = fma(temp_111, temp_129, temp_121);
                temp_134 = 0.0 - U_Mate.gWrkFl4[2].y;
                temp_135 = 1.0 + temp_134;
                temp_136 = temp_130 * temp_130;
                temp_137 = fma(temp_132, temp_132, temp_136);
                temp_138 = fma(temp_133, temp_133, temp_137);
                temp_140 = inversesqrt(temp_138);
                temp_143 = temp_130 * temp_140;
                temp_144 = temp_132 * temp_140;
                temp_145 = temp_133 * temp_140;
                temp_149 = 0.0 - temp_143;
                temp_150 = temp_149 + 0.0;
                temp_146 = temp_150;
                temp_151 = 0.0 - temp_144;
                temp_152 = temp_151 + 0.0;
                temp_147 = temp_152;
                temp_153 = 0.0 - temp_145;
                temp_154 = temp_153 + 0.0;
                temp_148 = temp_154;
                temp_155 = dFdy(temp_146);
                temp_156 = dFdy(temp_147);
                temp_157 = 1.0 * temp_155;
                temp_158 = dFdy(temp_148);
                temp_159 = 1.0 * temp_156;
                temp_160 = dFdx(temp_146);
                temp_161 = temp_157 * temp_157;
                temp_162 = 1.0 * temp_158;
                temp_164 = dFdx(temp_147);
                temp_165 = temp_160 * temp_160;
                temp_166 = fma(temp_159, temp_159, temp_161);
                temp_168 = fma(temp_164, temp_164, temp_165);
                temp_170 = fma(temp_162, temp_162, temp_166);
                temp_174 = dFdx(temp_148);
                temp_176 = fma(temp_174, temp_174, temp_168);
                temp_177 = temp_176 + temp_170;
                temp_182 = temp_177 * 0.5;
                temp_183 = min(temp_182, fp_c1.data[0].y);
                temp_184 = fma(temp_135, temp_135, temp_183);
                temp_185 = clamp(temp_184, 0.0, 1.0);
                temp_186 = sqrt(temp_185);
                temp_192 = 0.0 - temp_186;
                temp_193 = temp_192 + 1.0;
                out_attr1.y = temp_193;
            }
        "};

        // The pcmdo calcGeometricSpecularAA function compiles to the expression
        // glossiness = 1.0 - sqrt(clamp((1.0 - glossiness)^2 + kernelRoughness2 0.0, 1.0))
        // Consuming applications only care about the glossiness input.
        // This also avoids considering normal maps as a dependency.
        let fragment = TranslationUnit::parse(glsl).unwrap();
        let shader = shader_from_glsl(None, &fragment);
        assert_eq!(
            Shader {
                output_dependencies: [(
                    "o1.y".to_string(),
                    vec![Dependency::Buffer(BufferDependency {
                        name: "U_Mate".to_string(),
                        field: "gWrkFl4".to_string(),
                        index: 2,
                        channels: "y".to_string()
                    })]
                ),]
                .into()
            },
            shader
        );
    }
}
