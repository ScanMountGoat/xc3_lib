use std::ops::Deref;

use indoc::{formatdoc, indoc};
use xc3_model::shader_database::{
    AttributeDependency, BufferDependency, Dependency, TexCoord, TexCoordParams, TextureDependency,
};

use crate::{
    graph::{query::query_nodes_glsl, BinaryOp, Expr, Graph},
    shader_database::Attributes,
};

pub fn input_dependencies(
    graph: &Graph,
    attributes: &Attributes,
    assignments: &[usize],
    dependent_lines: &[usize],
) -> Vec<Dependency> {
    // TODO: Rework this to be cleaner and add more tests.
    let mut dependencies = texture_dependencies(graph, attributes, dependent_lines);

    // Add anything assigned directly to the output.
    for i in assignments {
        match &graph.nodes[*i].input {
            Expr::Float(f) => dependencies.push(Dependency::Constant((*f).into())),
            Expr::Parameter {
                name,
                field,
                index,
                channel,
            } => {
                if let Some(Expr::Int(index)) = index.as_deref() {
                    dependencies.push(Dependency::Buffer(BufferDependency {
                        name: name.into(),
                        field: field.clone().unwrap_or_default().into(),
                        index: Some((*index).try_into().unwrap()),
                        channels: channel.map(|c| c.to_string().into()).unwrap_or_default(),
                    }))
                }
            }
            _ => (),
        }
    }

    // TODO: Depth not high enough for complex expressions involving attributes?
    // TODO: Query the graph for known functions instead of hard coding recursion depth.
    dependencies.extend(
        attribute_dependencies(graph, dependent_lines, attributes, Some(1))
            .into_iter()
            .map(Dependency::Attribute),
    );

    dependencies
}

pub fn attribute_dependencies(
    graph: &Graph,
    dependent_lines: &[usize],
    attributes: &Attributes,
    recursion_depth: Option<usize>,
) -> Vec<AttributeDependency> {
    // Limit the recursion depth.
    let max_depth = recursion_depth.unwrap_or(dependent_lines.len());
    let dependent_lines: Vec<_> = dependent_lines
        .iter()
        .rev()
        .take(max_depth + 1)
        .rev()
        .collect();

    dependent_lines
        .into_iter()
        .filter_map(|i| {
            // Check all exprs for binary ops, function args, etc.
            graph.nodes[*i]
                .input
                .exprs_recursive()
                .iter()
                .find_map(|e| {
                    if let Expr::Global { name, channel } = e {
                        if attributes.input_locations.contains_left(name.as_str()) {
                            Some(AttributeDependency {
                                name: name.into(),
                                channels: channel.map(|c| c.to_string().into()).unwrap_or_default(),
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
    dependent_lines: &[usize],
) -> Vec<Dependency> {
    dependent_lines
        .iter()
        .filter_map(|i| {
            // Check all exprs for binary ops, function args, etc.
            graph.nodes[*i]
                .input
                .exprs_recursive()
                .iter()
                .find_map(|e| texture_dependency(e, graph, attributes))
        })
        .collect()
}

fn texture_dependency(e: &Expr, graph: &Graph, attributes: &Attributes) -> Option<Dependency> {
    if let Expr::Func {
        name,
        args,
        channel,
    } = e
    {
        if name.starts_with("texture") {
            if let Some(Expr::Global { name, .. }) = args.first() {
                let texcoords = texcoord_args(args, graph, attributes);

                Some(Dependency::Texture(TextureDependency {
                    name: name.into(),
                    channels: channel.map(|c| c.to_string().into()).unwrap_or_default(),
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
    args.iter()
        .skip(1)
        .flat_map(|a| a.exprs_recursive())
        .filter_map(|e| {
            if let Expr::Node { node_index, .. } = e {
                // Find the attribute used for this input.
                // TODO: Is this a subset of the dependencies for the output variable?
                let node_assignments = graph.node_dependencies_recursive(*node_index, None);
                let (name, channels) =
                    texcoord_name_channels(&node_assignments, graph, attributes)?;

                // Detect common cases for transforming UV coordinates.
                // TODO: This should also potentially modify the channels.
                let params = texcoord_params(graph, *node_index, attributes);

                Some(TexCoord {
                    name: name.into(),
                    channels: channels.into(),
                    params,
                })
            } else {
                None
            }
        })
        .collect()
}

pub fn texcoord_params(
    graph: &Graph,
    node_index: usize,
    attributes: &Attributes,
) -> Option<TexCoordParams> {
    let node = graph.nodes.get(node_index)?;
    scale_parameter(&node.input)
        .map(TexCoordParams::Scale)
        .or_else(|| tex_matrix(graph, &node.input).map(TexCoordParams::Matrix))
        .or_else(|| {
            tex_parallax(graph, &node.input, attributes).map(|(mask, param, param_ratio)| {
                TexCoordParams::Parallax {
                    mask,
                    param,
                    param_ratio,
                }
            })
        })
}

pub fn scale_parameter(expr: &Expr) -> Option<BufferDependency> {
    // Detect simple multiplication by scale parameters.
    // TODO: Also check that the attribute name matches?
    // temp_0 = vTex0.x
    // temp_1 = temp_0 * scale_param
    // temp_2 = temp_1
    match expr {
        Expr::Binary(BinaryOp::Mul, a, b) => match (a.deref(), b.deref()) {
            (Expr::Node { .. }, e) => buffer_dependency(e),
            (e, Expr::Node { .. }) => buffer_dependency(e),
            _ => None,
        },
        _ => None,
    }
}

pub fn tex_matrix(graph: &Graph, expr: &Expr) -> Option<[BufferDependency; 4]> {
    // TODO: Also check that the attribute name matches?
    // Detect matrix multiplication for the mat4x2 "gTexMat * vec4(u, v, 0.0, 1.0)".
    // U and V have the same pattern but use a different row of the matrix.
    let query = indoc! {"
        u = tex_coord.x;
        v = tex_coord.y;
        result = u * param_x;
        result = fma(v, param_y, result);
        result = fma(0.0, param_z, result);
        result = result + param_w;
    "};
    let result = query_nodes_glsl(expr, &graph.nodes, query)?;
    let x = result.get("param_x").copied().and_then(buffer_dependency)?;
    let y = result.get("param_y").copied().and_then(buffer_dependency)?;
    let z = result.get("param_z").copied().and_then(buffer_dependency)?;
    let w = result.get("param_w").copied().and_then(buffer_dependency)?;
    // TODO: Also detect UV texcoord names?
    Some([x, y, z, w])
}

pub fn tex_parallax(
    graph: &Graph,
    expr: &Expr,
    attributes: &Attributes,
) -> Option<(Dependency, BufferDependency, BufferDependency)> {
    // Some eye shaders use some form of parallax mapping.
    // uv = mix(mask, param, param_ratio) * 0.7 * (nrm.x * tan.xy - norm.y * bitan.xy) + vTex0.xy
    let query_xc2 = indoc! {"
        mask = mask;
        nrm_result = fma(temp, 0.7, temp);
        neg_mask = 0.0 - mask;
        param_minus_mask = neg_mask + param;
        ratio = fma(param_minus_mask, param_ratio, mask);
        result = fma(ratio, nrm_result, coord);
    "};

    // uv = mix(param, mask, param_ratio) * 0.7 * (nrm.x * tan.xy - norm.y * bitan.xy) + vTex0.xy
    // TODO: how to indicate the swapping of the param and mask in the mix function?
    // TODO: Also return the uv attribute and channel?
    let query_xc3 = indoc! {"
        mask = mask;
        nrm_result = fma(temp, 0.7, temp);
        neg_param = 0.0 - param;
        mask_minus_param = mask + neg_param;
        ratio = fma(mask_minus_param, param_ratio, param);
        result = fma(ratio, nrm_result, coord);
    "};
    let query_xc3_2 = formatdoc! {"
        {query_xc3}
        result = abs(result);
        result = result + -0.0;
    "};
    let result = query_nodes_glsl(expr, &graph.nodes, query_xc2)
        .or_else(|| query_nodes_glsl(expr, &graph.nodes, &query_xc3))
        .or_else(|| query_nodes_glsl(expr, &graph.nodes, &query_xc3_2))?;

    let mask = result
        .get("mask")
        .copied()
        .and_then(|e| texture_dependency(e, graph, attributes))?;
    let param = result.get("param").copied().and_then(buffer_dependency)?;
    let param_ratio = result
        .get("param_ratio")
        .copied()
        .and_then(buffer_dependency)?;

    Some((mask, param, param_ratio))
}

fn texcoord_name_channels(
    node_assignments: &[usize],
    graph: &Graph,
    attributes: &Attributes,
) -> Option<(String, String)> {
    node_assignments.iter().find_map(|i| {
        // Check all exprs for binary ops, function args, etc.
        graph.nodes[*i]
            .input
            .exprs_recursive()
            .into_iter()
            .find_map(|e| {
                if let Expr::Global { name, channel } = e {
                    if attributes.input_locations.contains_left(name.as_str()) {
                        Some((
                            name.to_string(),
                            channel.map(|c| c.to_string()).unwrap_or_default(),
                        ))
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
    })
}

pub fn latte_dependencies(source: &str, variable: &str, channel: Option<char>) -> String {
    Graph::from_latte_asm(source).glsl_dependencies(variable, channel, None)
}

pub fn buffer_dependency(e: &Expr) -> Option<BufferDependency> {
    if let Expr::Parameter {
        name,
        field,
        index,
        channel,
    } = e
    {
        if let Some(Expr::Int(index)) = index.as_deref() {
            Some(BufferDependency {
                name: name.into(),
                field: field.clone().unwrap_or_default().into(),
                index: Some((*index).try_into().unwrap()),
                channels: channel.map(|c| c.to_string().into()).unwrap_or_default(),
            })
        } else {
            Some(BufferDependency {
                name: name.into(),
                field: field.clone().unwrap_or_default().into(),
                index: None,
                channels: channel.map(|c| c.to_string().into()).unwrap_or_default(),
            })
        }
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::shader_database::find_attribute_locations;
    use glsl_lang::{ast::TranslationUnit, parse::DefaultParse};
    use indoc::indoc;
    use pretty_assertions::assert_eq;
    use xc3_model::shader_database::AttributeDependency;

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
        let graph = Graph::from_glsl(&tu);
        let attributes = find_attribute_locations(&tu);
        let assignments = graph.assignments_recursive("b", None, None);
        let dependent_lines = graph.dependencies_recursive("b", None, None);

        assert_eq!(
            vec![Dependency::Texture(TextureDependency {
                name: "texture1".into(),
                channels: "w".into(),
                texcoords: vec![
                    TexCoord {
                        name: "in_attr0".into(),
                        channels: "x".into(),
                        params: None
                    },
                    TexCoord {
                        name: "in_attr0".into(),
                        channels: "w".into(),
                        params: None
                    }
                ]
            })],
            input_dependencies(&graph, &attributes, &assignments, &dependent_lines)
        );
    }

    #[test]
    fn input_dependencies_tex_matrix() {
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
        let graph = Graph::from_glsl(&tu);
        let attributes = find_attribute_locations(&tu);
        let assignments = graph.assignments_recursive("temp_163", None, None);
        let dependent_lines = graph.dependencies_recursive("temp_163", None, None);

        assert_eq!(
            vec![Dependency::Texture(TextureDependency {
                name: "gTResidentTex05".into(),
                channels: "x".into(),
                texcoords: vec![
                    TexCoord {
                        name: "in_attr4".into(),
                        channels: "x".into(),
                        params: Some(TexCoordParams::Matrix([
                            BufferDependency {
                                name: "U_Mate".into(),
                                field: "gTexMat".into(),
                                index: Some(0),
                                channels: "x".into(),
                            },
                            BufferDependency {
                                name: "U_Mate".into(),
                                field: "gTexMat".into(),
                                index: Some(0),
                                channels: "y".into(),
                            },
                            BufferDependency {
                                name: "U_Mate".into(),
                                field: "gTexMat".into(),
                                index: Some(0),
                                channels: "z".into(),
                            },
                            BufferDependency {
                                name: "U_Mate".into(),
                                field: "gTexMat".into(),
                                index: Some(0),
                                channels: "w".into(),
                            }
                        ]))
                    },
                    TexCoord {
                        name: "in_attr4".into(),
                        channels: "x".into(),
                        params: Some(TexCoordParams::Matrix([
                            BufferDependency {
                                name: "U_Mate".into(),
                                field: "gTexMat".into(),
                                index: Some(1),
                                channels: "x".into(),
                            },
                            BufferDependency {
                                name: "U_Mate".into(),
                                field: "gTexMat".into(),
                                index: Some(1),
                                channels: "y".into(),
                            },
                            BufferDependency {
                                name: "U_Mate".into(),
                                field: "gTexMat".into(),
                                index: Some(1),
                                channels: "z".into(),
                            },
                            BufferDependency {
                                name: "U_Mate".into(),
                                field: "gTexMat".into(),
                                index: Some(1),
                                channels: "w".into(),
                            }
                        ]))
                    }
                ]
            })],
            input_dependencies(&graph, &attributes, &assignments, &dependent_lines)
        );
    }

    #[test]
    fn input_dependencies_scale_parameters() {
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
        let graph = Graph::from_glsl(&tu);
        let attributes = find_attribute_locations(&tu);
        let assignments = graph.assignments_recursive("temp_170", None, None);
        let dependent_lines = graph.dependencies_recursive("temp_170", None, None);

        assert_eq!(
            vec![Dependency::Texture(TextureDependency {
                name: "gTResidentTex04".into(),
                channels: "x".into(),
                texcoords: vec![
                    TexCoord {
                        name: "in_attr4".into(),
                        channels: "x".into(),
                        params: Some(TexCoordParams::Scale(BufferDependency {
                            name: "U_Mate".into(),
                            field: "gWrkFl4".into(),
                            index: Some(0),
                            channels: "z".into()
                        }))
                    },
                    TexCoord {
                        name: "in_attr4".into(),
                        channels: "y".into(),
                        params: Some(TexCoordParams::Scale(BufferDependency {
                            name: "U_Mate".into(),
                            field: "gWrkFl4".into(),
                            index: Some(0),
                            channels: "w".into()
                        }))
                    }
                ]
            })],
            input_dependencies(&graph, &attributes, &assignments, &dependent_lines)
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
        let graph = Graph::from_glsl(&tu);
        let attributes = find_attribute_locations(&tu);
        let assignments = graph.assignments_recursive("b", None, None);
        let dependent_lines = graph.dependencies_recursive("b", None, None);

        assert_eq!(
            vec![Dependency::Texture(TextureDependency {
                name: "texture1".into(),
                channels: "z".into(),
                texcoords: Vec::new()
            })],
            input_dependencies(&graph, &attributes, &assignments, &dependent_lines)
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
        let graph = Graph::from_glsl(&tu);
        let attributes = find_attribute_locations(&tu);
        let assignments = graph.assignments_recursive("b", None, None);
        let dependent_lines = graph.dependencies_recursive("b", None, None);

        assert_eq!(
            vec![
                Dependency::Texture(TextureDependency {
                    name: "texture1".into(),
                    channels: "z".into(),
                    texcoords: Vec::new()
                }),
                Dependency::Texture(TextureDependency {
                    name: "texture1".into(),
                    channels: "w".into(),
                    texcoords: Vec::new()
                })
            ],
            input_dependencies(&graph, &attributes, &assignments, &dependent_lines)
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
        let graph = Graph::from_glsl(&tu);
        let attributes = find_attribute_locations(&tu);

        assert_eq!(
            vec![Dependency::Texture(TextureDependency {
                name: "texture1".into(),
                channels: "x".into(),
                texcoords: Vec::new()
            })],
            input_dependencies(
                &graph,
                &attributes,
                &graph.assignments_recursive("out_attr1", Some('x'), None),
                &graph.dependencies_recursive("out_attr1", Some('x'), None)
            )
        );
        assert_eq!(
            vec![Dependency::Buffer(BufferDependency {
                name: "U_Mate".into(),
                field: "data".into(),
                index: Some(1),
                channels: "w".into()
            })],
            input_dependencies(
                &graph,
                &attributes,
                &graph.assignments_recursive("out_attr1", Some('y'), None),
                &graph.dependencies_recursive("out_attr1", Some('y'), None)
            )
        );
        assert_eq!(
            vec![Dependency::Buffer(BufferDependency {
                name: "uniform_data".into(),
                field: Default::default(),
                index: Some(3),
                channels: "y".into()
            })],
            input_dependencies(
                &graph,
                &attributes,
                &graph.assignments_recursive("out_attr1", Some('z'), None),
                &graph.dependencies_recursive("out_attr1", Some('z'), None)
            )
        );
        assert_eq!(
            vec![Dependency::Constant(1.5.into())],
            input_dependencies(
                &graph,
                &attributes,
                &graph.assignments_recursive("out_attr1", Some('w'), None),
                &graph.dependencies_recursive("out_attr1", Some('w'), None)
            )
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
        let dependent_lines = graph.dependencies_recursive("out_attr1", Some('y'), None);

        assert_eq!(
            vec![AttributeDependency {
                name: "in_attr2".into(),
                channels: "x".into(),
            }],
            attribute_dependencies(&graph, &dependent_lines, &attributes, None)
        );
    }

    #[test]
    fn input_dependencies_vector_registers() {
        let glsl = indoc! {"
            void main() {
                R9.z = texture(tex, vec2(0.0)).x;
                R12.w = R9.z;
                PIX2.w = R12.w;
            }
        "};

        let tu = TranslationUnit::parse(glsl).unwrap();
        let graph = Graph::from_glsl(&tu);
        let attributes = find_attribute_locations(&tu);
        let assignments = graph.assignments_recursive("PIX2", Some('w'), None);
        let dependent_lines = graph.dependencies_recursive("PIX2", Some('w'), None);

        assert_eq!(
            vec![Dependency::Texture(TextureDependency {
                name: "tex".into(),
                channels: "x".into(),
                texcoords: Vec::new()
            })],
            input_dependencies(&graph, &attributes, &assignments, &dependent_lines)
        );
    }
}
