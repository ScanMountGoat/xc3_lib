// TODO: make dependencies and annotation into a library?
use std::ops::Deref;

use glsl_lang::{ast::TranslationUnit, parse::DefaultParse};
use xc3_model::shader_database::{
    AttributeDependency, BufferDependency, Dependency, TexCoord, TextureDependency,
};

use crate::{
    annotation::shader_source_no_extensions,
    graph::{reduce_channels, Expr, Graph},
    shader_database::{find_attribute_locations, Attributes},
};

pub fn input_dependencies(translation_unit: &TranslationUnit, var: &str) -> Vec<Dependency> {
    // Find the most recent assignment for the output variable.
    let graph = Graph::from_glsl(translation_unit);

    let (variable, channels) = var.split_once('.').unwrap_or((var, ""));
    let node = graph
        .nodes
        .iter()
        .rfind(|n| n.output.name == variable && n.output.channels == channels);

    let attributes = find_attribute_locations(translation_unit);

    // TODO: Rework this to be cleaner and add more tests.
    let mut dependencies = texture_dependencies(&graph, &attributes, variable, channels);

    // Add anything directly assigned to the output variable.
    if let Some(node) = node {
        match &node.input {
            Expr::Float(f) => dependencies.push(Dependency::Constant((*f).into())),
            Expr::Parameter {
                name,
                field,
                index,
                channels,
            } => {
                if let Expr::Int(index) = index.deref() {
                    dependencies.push(Dependency::Buffer(BufferDependency {
                        name: name.to_string(),
                        field: field.clone().unwrap_or_default().to_string(),
                        index: (*index).try_into().unwrap(),
                        channels: channels.to_string(),
                    }))
                }
            }
            _ => (),
        }
    }

    // TODO: Depth not high enough for complex expressions involving attributes?
    // TODO: Query the graph for known functions instead of hard coding recursion depth.
    dependencies.extend(
        attribute_dependencies(&graph, variable, channels, &attributes, Some(1))
            .into_iter()
            .map(Dependency::Attribute),
    );

    dependencies
}

pub fn attribute_dependencies(
    graph: &Graph,
    variable: &str,
    channels: &str,
    attributes: &Attributes,
    recursion_depth: Option<usize>,
) -> Vec<AttributeDependency> {
    graph
        .assignments_recursive(variable, channels, recursion_depth)
        .into_iter()
        .filter_map(|(i, final_channels)| {
            // Check all exprs for binary ops, function args, etc.
            graph.nodes[i].input.exprs_recursive().iter().find_map(|e| {
                if let Expr::Global { name, .. } = e {
                    if attributes.input_locations.contains_left(name.as_str()) {
                        Some(AttributeDependency {
                            name: name.to_string(),
                            channels: final_channels.clone(),
                        })
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
        })
        .collect()
}

fn texture_dependencies(
    graph: &Graph,
    attributes: &Attributes,
    variable: &str,
    channels: &str,
) -> Vec<Dependency> {
    graph
        .assignments_recursive(variable, channels, None)
        .into_iter()
        .filter_map(|(i, final_channels)| {
            // Check all exprs for binary ops, function args, etc.
            graph.nodes[i]
                .input
                .exprs_recursive()
                .iter()
                .find_map(|e| texture_dependency(e, graph, attributes, &final_channels))
        })
        .collect()
}

fn texture_dependency(
    e: &Expr,
    graph: &Graph,
    attributes: &Attributes,
    final_channels: &str,
) -> Option<Dependency> {
    if let Expr::Func { name, args, .. } = e {
        if name.starts_with("texture") {
            if let Some(Expr::Global { name, .. }) = args.first() {
                let texcoords = texcoord_args(args, graph, attributes);

                Some(Dependency::Texture(TextureDependency {
                    name: name.to_string(),
                    channels: final_channels.to_string(),
                    texcoords,
                }))
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    }
}

fn texcoord_args(args: &[Expr], graph: &Graph, attributes: &Attributes) -> Vec<TexCoord> {
    // Search recursively to find texcoord variables.
    // The first arg is always the texture name.
    let texcoords: Vec<_> = args
        .iter()
        .skip(1)
        .flat_map(|a| a.exprs_recursive())
        .filter_map(|e| {
            if let Expr::Node { node_index, .. } = e {
                // Find the attribute used for this input.
                let node_assignments = graph.node_assignments_recursive(*node_index, None);
                let (name, channels) =
                    texcoord_name_channels(&node_assignments, graph, attributes)?;

                // TODO: use find_buffer_parameters?
                let params = node_assignments
                    .into_iter()
                    .filter_map(|(i, final_channels)| {
                        // Check all exprs for binary ops, function args, etc.
                        graph.nodes[i]
                            .input
                            .exprs_recursive()
                            .into_iter()
                            .find_map(|e| buffer_dependency(e, &final_channels))
                    })
                    .collect();

                Some(TexCoord {
                    name: name.clone(),
                    channels: channels.clone(),
                    params,
                })
            } else {
                None
            }
        })
        .collect();
    texcoords
}

fn texcoord_name_channels(
    node_assignments: &[(usize, String)],
    graph: &Graph,
    attributes: &Attributes,
) -> Option<(String, String)> {
    node_assignments.iter().find_map(|(i, final_channels)| {
        // Check all exprs for binary ops, function args, etc.
        graph.nodes[*i]
            .input
            .exprs_recursive()
            .into_iter()
            .find_map(|e| {
                if let Expr::Global { name, .. } = e {
                    if attributes.input_locations.contains_left(name.as_str()) {
                        Some((name.to_string(), final_channels.to_string()))
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
    })
}

pub fn glsl_dependencies(source: &str, var: &str) -> String {
    // TODO: Correctly handle if statements?
    let source = shader_source_no_extensions(source);
    let translation_unit = TranslationUnit::parse(source).unwrap();
    let (variable, channels) = var.split_once('.').unwrap_or((var, ""));

    Graph::from_glsl(&translation_unit).glsl_dependencies(variable, channels, None)
}

pub fn find_buffer_parameters(
    graph: &Graph,
    variable: &str,
    channels: &str,
) -> Vec<BufferDependency> {
    graph
        .assignments_recursive(variable, channels, None)
        .into_iter()
        .filter_map(|(i, final_channels)| {
            // Check all exprs for binary ops, function args, etc.
            graph.nodes[i]
                .input
                .exprs_recursive()
                .into_iter()
                .find_map(|e| buffer_dependency(e, &final_channels))
        })
        .collect()
}

fn buffer_dependency(e: &Expr, final_channels: &str) -> Option<BufferDependency> {
    if let Expr::Parameter {
        name,
        field,
        index,
        channels,
    } = e
    {
        if let Expr::Int(index) = index.deref() {
            Some(BufferDependency {
                name: name.to_string(),
                field: field.clone().unwrap_or_default().to_string(),
                index: (*index).try_into().unwrap(),
                channels: reduce_channels(channels, final_channels),
            })
        } else {
            None
        }
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use indoc::indoc;
    use pretty_assertions::assert_eq;
    use xc3_model::shader_database::AttributeDependency;

    #[test]
    fn line_dependencies_final_assignment() {
        let glsl = indoc! {"
            layout (binding = 9, std140) uniform fp_c9
            {
                vec4 fp_c9_data[0x1000];
            };

            layout(location = 0) in vec4 in_attr0;

            void main() 
            {
                float a = fp_c9_data[0].x;
                float b = 2.0;
                float c = a * b;
                float d = fma(a, b, c);
                d = d + 1.0;
                OUT_Color.x = c + d;
            }
        "};

        assert_eq!(
            indoc! {"
                a = fp_c9_data[0].x;
                b = 2;
                c = a * b;
                d = fma(a, b, c);
                d = d + 1;
                OUT_Color.x = c + d;
            "},
            glsl_dependencies(glsl, "OUT_Color.x")
        );
    }

    #[test]
    fn line_dependencies_intermediate_assignment() {
        let glsl = indoc! {"
            void main() 
            {
                float a = 1.0;
                float b = 2.0;
                float d = fma(a, b, -1.0);
                float c = 2 * b;
                d = d + 1.0;
                OUT_Color.x = c + d;
            }
        "};

        assert_eq!(
            indoc! {"
                b = 2;
                c = 2 * b;
            "},
            glsl_dependencies(glsl, "c")
        );
    }

    #[test]
    fn line_dependencies_type_casts() {
        let glsl = indoc! {"
            void main() 
            {
                float a = 0.0;
                uint b = uint(a) >> 2;
                float d = 3.0 + a;
                float c = data[int(b)];
            }
        "};

        assert_eq!(
            indoc! {"
                a = 0;
                b = uint(a) >> 2;
                c = data[int(b)];
            "},
            glsl_dependencies(glsl, "c")
        );
    }

    #[test]
    fn line_dependencies_missing() {
        let glsl = indoc! {"
            void main() 
            {
                float a = 0.0;
            }
        "};

        assert_eq!("", glsl_dependencies(glsl, "d"));
    }

    #[test]
    fn line_dependencies_textures() {
        let glsl = indoc! {"
            void main() 
            {
                float a = 1.0;
                float a2 = a * 5.0;
                float b = texture(texture1, vec2(a2 + 2.0, 1.0)).x;
                float c = data[int(b)];
            }
        "};

        assert_eq!(
            indoc! {"
                a = 1;
                a2 = a * 5;
                b = texture(texture1, vec2(a2 + 2, 1)).x;
                c = data[int(b)];
            "},
            glsl_dependencies(glsl, "c")
        );
    }

    #[test]
    fn input_dependencies_single_channel() {
        let glsl = indoc! {"
            layout(location = 0) in vec4 in_attr0;

            void main() 
            {
                float x = in_attr0.x;
                float y = in_attr0.w;
                float x2 = x;
                float y2 = y;
                float a = texture(texture1, vec2(x2, y2)).xw;
                float b = a.y * 2.0;
            }
        "};

        let tu = TranslationUnit::parse(glsl).unwrap();
        assert_eq!(
            vec![Dependency::Texture(TextureDependency {
                name: "texture1".to_string(),
                channels: "w".to_string(),
                texcoords: vec![
                    TexCoord {
                        name: "in_attr0".to_string(),
                        channels: "x".to_string(),
                        params: Vec::new()
                    },
                    TexCoord {
                        name: "in_attr0".to_string(),
                        channels: "w".to_string(),
                        params: Vec::new()
                    }
                ]
            })],
            input_dependencies(&tu, "b")
        );
    }

    #[test]
    fn input_dependencies_scale_tex_matrix() {
        // xeno3/chr/ch/ch01021013, shd0039.frag
        let glsl = indoc! {"
            layout(location = 4) in vec4 in_attr4;

            void main() 
            {
                temp_0 = in_attr4.x;
                temp_1 = in_attr4.y;
                temp_141 = temp_0 * U_Mate.gTexMat[0].x;
                temp_147 = temp_0 * U_Mate.gTexMat[1].x;
                temp_148 = fma(temp_1, U_Mate.gTexMat[0].y, temp_141);
                temp_151 = fma(temp_1, U_Mate.gTexMat[1].y, temp_147);
                temp_152 = fma(0., U_Mate.gTexMat[1].z, temp_151);
                temp_154 = fma(0., U_Mate.gTexMat[0].z, temp_148);
                temp_155 = temp_152 + U_Mate.gTexMat[1].w;
                temp_160 = temp_154 + U_Mate.gTexMat[0].w;
                temp_162 = texture(gTResidentTex05, vec2(temp_160, temp_155)).x;
                temp_163 = temp_162.x; 
            }
        "};

        // TODO: Handle the case where multiple attribute components are used?
        let tu = TranslationUnit::parse(glsl).unwrap();
        assert_eq!(
            vec![Dependency::Texture(TextureDependency {
                name: "gTResidentTex05".to_string(),
                channels: "x".to_string(),
                texcoords: vec![
                    TexCoord {
                        name: "in_attr4".to_string(),
                        channels: "x".to_string(),
                        params: vec![
                            BufferDependency {
                                name: "U_Mate".to_string(),
                                field: "gTexMat".to_string(),
                                index: 0,
                                channels: "x".to_string(),
                            },
                            BufferDependency {
                                name: "U_Mate".to_string(),
                                field: "gTexMat".to_string(),
                                index: 0,
                                channels: "y".to_string(),
                            },
                            BufferDependency {
                                name: "U_Mate".to_string(),
                                field: "gTexMat".to_string(),
                                index: 0,
                                channels: "z".to_string(),
                            },
                            BufferDependency {
                                name: "U_Mate".to_string(),
                                field: "gTexMat".to_string(),
                                index: 0,
                                channels: "w".to_string(),
                            },
                        ]
                    },
                    TexCoord {
                        name: "in_attr4".to_string(),
                        channels: "x".to_string(),
                        params: vec![
                            BufferDependency {
                                name: "U_Mate".to_string(),
                                field: "gTexMat".to_string(),
                                index: 1,
                                channels: "x".to_string(),
                            },
                            BufferDependency {
                                name: "U_Mate".to_string(),
                                field: "gTexMat".to_string(),
                                index: 1,
                                channels: "y".to_string(),
                            },
                            BufferDependency {
                                name: "U_Mate".to_string(),
                                field: "gTexMat".to_string(),
                                index: 1,
                                channels: "z".to_string(),
                            },
                            BufferDependency {
                                name: "U_Mate".to_string(),
                                field: "gTexMat".to_string(),
                                index: 1,
                                channels: "w".to_string(),
                            },
                        ]
                    }
                ]
            })],
            input_dependencies(&tu, "temp_163")
        );
    }

    #[test]
    fn input_dependencies_scale_parameter() {
        let glsl = indoc! {"
            layout(location = 4) in vec4 in_attr4;

            void main() 
            {
                temp_0 = in_attr4.x;
                temp_1 = in_attr4.y;
                test = 0.5;
                temp_121 = temp_1 * U_Mate.gWrkFl4[0].w;
                temp_157 = temp_0 * U_Mate.gWrkFl4[0].z;
                temp_169 = texture(gTResidentTex04, vec2(temp_157, temp_121)).xyz;
                temp_170 = temp_169.x; 
            }
        "};

        let tu = TranslationUnit::parse(glsl).unwrap();
        assert_eq!(
            vec![Dependency::Texture(TextureDependency {
                name: "gTResidentTex04".to_string(),
                channels: "x".to_string(),
                texcoords: vec![
                    TexCoord {
                        name: "in_attr4".to_string(),
                        channels: "x".to_string(),
                        params: vec![BufferDependency {
                            name: "U_Mate".to_string(),
                            field: "gWrkFl4".to_string(),
                            index: 0,
                            channels: "z".to_string()
                        }]
                    },
                    TexCoord {
                        name: "in_attr4".to_string(),
                        channels: "y".to_string(),
                        params: vec![BufferDependency {
                            name: "U_Mate".to_string(),
                            field: "gWrkFl4".to_string(),
                            index: 0,
                            channels: "w".to_string()
                        }]
                    }
                ]
            })],
            input_dependencies(&tu, "temp_170")
        );
    }

    #[test]
    fn input_dependencies_single_channel_scalar() {
        let glsl = indoc! {"
            void main() 
            {
                float t = 1.0;
                float a = texture(texture1, vec2(t)).z;
                float b = a * 2.0;
            }
        "};

        let tu = TranslationUnit::parse(glsl).unwrap();
        assert_eq!(
            vec![Dependency::Texture(TextureDependency {
                name: "texture1".to_string(),
                channels: "z".to_string(),
                texcoords: Vec::new()
            })],
            input_dependencies(&tu, "b")
        );
    }

    #[test]
    fn input_dependencies_multiple_channels() {
        let glsl = indoc! {"
            void main() 
            {
                float a = texture(texture1, vec2(1.0)).zw;
                float b = a.y + a.x;
            }
        "};

        let tu = TranslationUnit::parse(glsl).unwrap();
        assert_eq!(
            vec![
                Dependency::Texture(TextureDependency {
                    name: "texture1".to_string(),
                    channels: "w".to_string(),
                    texcoords: Vec::new()
                }),
                Dependency::Texture(TextureDependency {
                    name: "texture1".to_string(),
                    channels: "z".to_string(),
                    texcoords: Vec::new()
                })
            ],
            input_dependencies(&tu, "b")
        );
    }

    #[test]
    fn input_dependencies_buffers_constants_textures() {
        // Only handle parameters and constants assigned directly to outputs for now.
        // This also assumes buffers, constants, and textures are mutually exclusive.
        let glsl = indoc! {"
            void main() 
            {
                float a = texture(texture1, vec2(1.0)).x;
                out_attr1.x = a;
                out_attr1.y = U_Mate.data[1].w;
                out_attr1.z = uniform_data[3].y;
                out_attr1.w = 1.5;
            }
        "};

        let tu = TranslationUnit::parse(glsl).unwrap();
        assert_eq!(
            vec![Dependency::Texture(TextureDependency {
                name: "texture1".to_string(),
                channels: "x".to_string(),
                texcoords: Vec::new()
            })],
            input_dependencies(&tu, "out_attr1.x")
        );
        assert_eq!(
            vec![Dependency::Buffer(BufferDependency {
                name: "U_Mate".to_string(),
                field: "data".to_string(),
                index: 1,
                channels: "w".to_string()
            })],
            input_dependencies(&tu, "out_attr1.y")
        );
        assert_eq!(
            vec![Dependency::Buffer(BufferDependency {
                name: "uniform_data".to_string(),
                field: String::new(),
                index: 3,
                channels: "y".to_string()
            })],
            input_dependencies(&tu, "out_attr1.z")
        );
        assert_eq!(
            vec![Dependency::Constant(1.5.into())],
            input_dependencies(&tu, "out_attr1.w")
        );
    }

    #[test]
    fn input_dependencies_attribute() {
        let glsl = indoc! {"
            layout(location = 2) in vec4 in_attr2;

            void main() 
            {
                temp_0 = in_attr2.zwx;
                temp_1 = temp_0.xzy;
                out_attr1.x = 1.0;
                out_attr1.y = temp_1.y;
                out_attr1.z = uniform_data[3].y;
                out_attr1.w = 1.5;
            }
        "};

        // temp_0.zwx.xzy.y -> temp_0.zxw.y -> temp_0.x
        let tu = TranslationUnit::parse(glsl).unwrap();
        let graph = Graph::from_glsl(&tu);
        let attributes = find_attribute_locations(&tu);
        assert_eq!(
            vec![AttributeDependency {
                name: "in_attr2".to_string(),
                channels: "x".to_string(),
            }],
            attribute_dependencies(&graph, "out_attr1", "y", &attributes, None)
        );
    }

    #[test]
    fn find_vertex_texcoord_parameters() {
        let glsl = indoc! {"
            void main() {
                temp_62 = vTex0.x;
                temp_64 = vTex0.y;
                temp_119 = temp_62 * U_Mate.gWrkFl4[0].x;
                out_attr4.z = temp_119;
                out_attr4.x = temp_62;
                out_attr4.y = temp_64;
                temp_179 = temp_64 * U_Mate.gWrkFl4[0].y;
                out_attr4.w = temp_179;
            }
        "};

        let tu = TranslationUnit::parse(glsl).unwrap();
        let graph = Graph::from_glsl(&tu);
        assert!(find_buffer_parameters(&graph, "out_attr4", "x").is_empty());
        assert!(find_buffer_parameters(&graph, "out_attr4", "y").is_empty());
        assert_eq!(
            vec![BufferDependency {
                name: "U_Mate".to_string(),
                field: "gWrkFl4".to_string(),
                index: 0,
                channels: "x".to_string()
            }],
            find_buffer_parameters(&graph, "out_attr4", "z")
        );
        assert_eq!(
            vec![BufferDependency {
                name: "U_Mate".to_string(),
                field: "gWrkFl4".to_string(),
                index: 0,
                channels: "y".to_string()
            }],
            find_buffer_parameters(&graph, "out_attr4", "w")
        );
    }
}
