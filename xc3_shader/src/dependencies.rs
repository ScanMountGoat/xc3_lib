use crate::{
    graph::{
        query::{assign_x_recursive, query_nodes},
        Expr, Graph,
    },
    shader_database::Attributes,
};
use indoc::indoc;
use smol_str::SmolStr;
use std::sync::LazyLock;
use xc3_model::shader_database::{
    AttributeDependency, BufferDependency, Dependency, TexCoord, TexCoordParams, TextureDependency,
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
    // Assignments are in reverse order, so take only the first element.
    if let Some(i) = assignments.first() {
        match &graph.nodes[*i].input {
            Expr::Float(f) => dependencies.push(Dependency::Constant(*f)),
            Expr::Parameter {
                name,
                field,
                index,
                channel,
            } => {
                if let Some(Expr::Int(index)) = index.as_deref() {
                    dependencies.push(Dependency::Buffer(BufferDependency {
                        name: name.clone(),
                        field: field.clone().unwrap_or_default(),
                        index: Some((*index).try_into().unwrap()),
                        channel: *channel,
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
                                name: name.clone(),
                                channel: *channel,
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

pub fn texture_dependency(e: &Expr, graph: &Graph, attributes: &Attributes) -> Option<Dependency> {
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
                    name: name.clone(),
                    channel: *channel,
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
            // Detect common cases for transforming UV coordinates.
            texcoord_params(graph, e, attributes).or_else(|| {
                if let Expr::Node { node_index, .. } = e {
                    // Find the attribute used for this input.
                    // TODO: Is this a subset of the dependencies for the output variable?
                    let node_assignments = graph.node_dependencies_recursive(*node_index, None);

                    let (name, channel) =
                        texcoord_name_channel(&node_assignments, graph, attributes)?;

                    Some(TexCoord {
                        name: name.into(),
                        channel,
                        params: None,
                    })
                } else {
                    None
                }
            })
        })
        .collect()
}

pub fn texcoord_params(graph: &Graph, input: &Expr, attributes: &Attributes) -> Option<TexCoord> {
    // Detect operations from most specific to least specific.
    let (params, name, channel) = tex_parallax(graph, input, attributes)
        .or_else(|| tex_parallax_xcx_de(graph, input, attributes))
        .or_else(|| tex_matrix(graph, input))
        .or_else(|| scale_parameter(graph, input))?;

    Some(TexCoord {
        name,
        channel,
        params: Some(params),
    })
}

static SCALE_PARAMETER: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
        void main() {
            coord = coord;
            result = coord * scale;
            result = result;
        }
    "};
    Graph::parse_glsl(query).unwrap()
});

fn scale_parameter(graph: &Graph, expr: &Expr) -> Option<(TexCoordParams, SmolStr, Option<char>)> {
    // Detect simple multiplication by scale parameters.
    let result = query_nodes(expr, &graph.nodes, &SCALE_PARAMETER.nodes)?;

    let param = buffer_dependency(result.get("scale")?)?;

    let (coord, channel) = match result.get("coord")? {
        Expr::Global { name, channel } => Some((name.clone(), *channel)),
        _ => None,
    }?;

    Some((TexCoordParams::Scale(param), coord, channel))
}

static TEX_MATRIX: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
        void main() {
            u = coord.x;
            v = coord.y;
            result = u * param_x;
            result = fma(v, param_y, result);
            result = fma(0.0, param_z, result);
            result = result + param_w;
            result = result;
        }
    "};
    Graph::parse_glsl(query).unwrap()
});

fn tex_matrix(graph: &Graph, expr: &Expr) -> Option<(TexCoordParams, SmolStr, Option<char>)> {
    // TODO: Also check that the attribute name matches?
    // Detect matrix multiplication for the mat4x2 "gTexMat * vec4(u, v, 0.0, 1.0)".
    // U and V have the same pattern but use a different row of the matrix.
    let result = query_nodes(expr, &graph.nodes, &TEX_MATRIX.nodes)?;
    let x = result.get("param_x").copied().and_then(buffer_dependency)?;
    let y = result.get("param_y").copied().and_then(buffer_dependency)?;
    let z = result.get("param_z").copied().and_then(buffer_dependency)?;
    let w = result.get("param_w").copied().and_then(buffer_dependency)?;

    // TODO: How to differentiate between u and v?
    let (coord, channel) = match result.get("coord")? {
        Expr::Global { name, channel } => Some((name.clone(), *channel)),
        _ => None,
    }?;

    Some((TexCoordParams::Matrix([x, y, z, w]), coord, channel))
}

static TEX_PARALLAX_XC2: LazyLock<Graph> = LazyLock::new(|| {
    // uv = mix(mask, param, param_ratio) * 0.7 * (nrm.x * tan.xy - norm.y * bitan.xy) + vTex0.xy
    let query = indoc! {"
        void main() {
            coord = coord;
            mask = mask;
            nrm_result = fma(temp1, 0.7, temp2);
            neg_mask = 0.0 - mask;
            param_minus_mask = neg_mask + param;
            ratio = fma(param_minus_mask, param_ratio, mask);
            result = fma(ratio, nrm_result, coord);
        }
    "};
    Graph::parse_glsl(query).unwrap()
});

static TEX_PARALLAX_XC3: LazyLock<Graph> = LazyLock::new(|| {
    // uv = mix(param, mask, param_ratio) * 0.7 * (nrm.x * tan.xy - norm.y * bitan.xy) + vTex0.xy
    let query = indoc! {"
        void main() {
            coord = coord;
            mask = mask;
            nrm_result = fma(temp1, 0.7, temp2);
            neg_param = 0.0 - param;
            mask_minus_param = mask + neg_param;
            ratio = fma(mask_minus_param, param_ratio, param);
            result = fma(ratio, nrm_result, coord);
        }
    "};
    Graph::parse_glsl(query).unwrap()
});

static TEX_PARALLAX_XC3_2: LazyLock<Graph> = LazyLock::new(|| {
    // uv = mix(param, mask, param_ratio) * 0.7 * (nrm.x * tan.xy - norm.y * bitan.xy) + vTex0.xy
    let query = indoc! {"
        void main() {
            coord = coord;
            mask = mask;
            nrm_result = fma(temp1, 0.7, temp2);
            neg_param = 0.0 - param;
            mask_minus_param = mask + neg_param;
            ratio = fma(mask_minus_param, param_ratio, param);
            result = fma(ratio, nrm_result, coord);
            // Generated for some shaders.
            result = abs(result);
            result = result + -0.0;
        }
    "};
    Graph::parse_glsl(query).unwrap()
});

fn tex_parallax(
    graph: &Graph,
    expr: &Expr,
    attributes: &Attributes,
) -> Option<(TexCoordParams, SmolStr, Option<char>)> {
    let expr = assign_x_recursive(&graph.nodes, expr);

    // Some eye shaders use some form of parallax mapping.
    let (result, is_swapped) = query_nodes(expr, &graph.nodes, &TEX_PARALLAX_XC2.nodes)
        .map(|r| (r, false))
        .or_else(|| query_nodes(expr, &graph.nodes, &TEX_PARALLAX_XC3.nodes).map(|r| (r, true)))
        .or_else(|| {
            query_nodes(expr, &graph.nodes, &TEX_PARALLAX_XC3_2.nodes).map(|r| (r, true))
        })?;

    let mut mask_a = result.get("mask").copied().and_then(|e| {
        texture_dependency(e, graph, attributes)
            .or_else(|| buffer_dependency(e).map(Dependency::Buffer))
    })?;

    let mut mask_b = result.get("param").copied().and_then(|e| {
        texture_dependency(e, graph, attributes)
            .or_else(|| buffer_dependency(e).map(Dependency::Buffer))
    })?;

    if is_swapped {
        std::mem::swap(&mut mask_a, &mut mask_b);
    }

    let ratio = result.get("param_ratio").copied().and_then(|e| {
        texture_dependency(e, graph, attributes)
            .or_else(|| buffer_dependency(e).map(Dependency::Buffer))
    })?;

    let (coord, channel) = match result.get("coord")? {
        Expr::Global { name, channel } => Some((name.clone(), *channel)),
        _ => None,
    }?;

    Some((
        TexCoordParams::Parallax {
            mask_a,
            mask_b,
            ratio,
        },
        coord,
        channel,
    ))
}

static TEX_PARALLAX_XCX_DE: LazyLock<Graph> = LazyLock::new(|| {
    // uv = ratio * 0.7 * (nrm.x * tan.xy - norm.y * bitan.xy) + vTex0.xy
    let query = indoc! {"
        void main() {
            nrm_result = fma(temp1, 0.7, temp2);
            result = fma(nrm_result, ratio, coord);
        }
    "};
    Graph::parse_glsl(query).unwrap()
});

fn tex_parallax_xcx_de(
    graph: &Graph,
    expr: &Expr,
    attributes: &Attributes,
) -> Option<(TexCoordParams, SmolStr, Option<char>)> {
    let expr = assign_x_recursive(&graph.nodes, expr);

    // Some eye shaders use some form of parallax mapping.
    let result = query_nodes(expr, &graph.nodes, &TEX_PARALLAX_XCX_DE.nodes)?;

    let ratio = assign_x_recursive(&graph.nodes, result.get("ratio")?);
    let ratio = texture_dependency(ratio, graph, attributes)
        .or_else(|| buffer_dependency(ratio).map(Dependency::Buffer))?;

    let coord = assign_x_recursive(&graph.nodes, result.get("coord")?);
    let (coord, channel) = match coord {
        Expr::Global { name, channel } => Some((name.clone(), *channel)),
        _ => None,
    }?;

    Some((
        TexCoordParams::Parallax {
            mask_a: ratio,
            mask_b: Dependency::Constant(0.0.into()),
            ratio: Dependency::Constant(0.0.into()),
        },
        coord,
        channel,
    ))
}

fn texcoord_name_channel(
    node_assignments: &[usize],
    graph: &Graph,
    attributes: &Attributes,
) -> Option<(String, Option<char>)> {
    node_assignments.iter().find_map(|i| {
        // Check all exprs for binary ops, function args, etc.
        graph.nodes[*i]
            .input
            .exprs_recursive()
            .into_iter()
            .find_map(|e| {
                if let Expr::Global { name, channel } = e {
                    if attributes.input_locations.contains_left(name.as_str()) {
                        Some((name.to_string(), *channel))
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
                name: name.clone(),
                field: field.clone().unwrap_or_default(),
                index: Some((*index).try_into().unwrap()),
                channel: *channel,
            })
        } else {
            Some(BufferDependency {
                name: name.clone(),
                field: field.clone().unwrap_or_default(),
                index: None,
                channel: *channel,
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
                channel: Some('w'),
                texcoords: vec![
                    TexCoord {
                        name: "in_attr0".into(),
                        channel: Some('x'),
                        params: None
                    },
                    TexCoord {
                        name: "in_attr0".into(),
                        channel: Some('w'),
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
                channel: Some('x'),
                texcoords: vec![
                    TexCoord {
                        name: "in_attr4".into(),
                        channel: Some('x'),
                        params: Some(TexCoordParams::Matrix([
                            BufferDependency {
                                name: "U_Mate".into(),
                                field: "gTexMat".into(),
                                index: Some(0),
                                channel: Some('x'),
                            },
                            BufferDependency {
                                name: "U_Mate".into(),
                                field: "gTexMat".into(),
                                index: Some(0),
                                channel: Some('y'),
                            },
                            BufferDependency {
                                name: "U_Mate".into(),
                                field: "gTexMat".into(),
                                index: Some(0),
                                channel: Some('z'),
                            },
                            BufferDependency {
                                name: "U_Mate".into(),
                                field: "gTexMat".into(),
                                index: Some(0),
                                channel: Some('w'),
                            }
                        ]))
                    },
                    TexCoord {
                        name: "in_attr4".into(),
                        channel: Some('x'),
                        params: Some(TexCoordParams::Matrix([
                            BufferDependency {
                                name: "U_Mate".into(),
                                field: "gTexMat".into(),
                                index: Some(1),
                                channel: Some('x'),
                            },
                            BufferDependency {
                                name: "U_Mate".into(),
                                field: "gTexMat".into(),
                                index: Some(1),
                                channel: Some('y'),
                            },
                            BufferDependency {
                                name: "U_Mate".into(),
                                field: "gTexMat".into(),
                                index: Some(1),
                                channel: Some('z'),
                            },
                            BufferDependency {
                                name: "U_Mate".into(),
                                field: "gTexMat".into(),
                                index: Some(1),
                                channel: Some('w'),
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
                channel: Some('x'),
                texcoords: vec![
                    TexCoord {
                        name: "in_attr4".into(),
                        channel: Some('x'),
                        params: Some(TexCoordParams::Scale(BufferDependency {
                            name: "U_Mate".into(),
                            field: "gWrkFl4".into(),
                            index: Some(0),
                            channel: Some('z')
                        }))
                    },
                    TexCoord {
                        name: "in_attr4".into(),
                        channel: Some('y'),
                        params: Some(TexCoordParams::Scale(BufferDependency {
                            name: "U_Mate".into(),
                            field: "gWrkFl4".into(),
                            index: Some(0),
                            channel: Some('w')
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
                channel: Some('z'),
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
                    channel: Some('z'),
                    texcoords: Vec::new()
                }),
                Dependency::Texture(TextureDependency {
                    name: "texture1".into(),
                    channel: Some('w'),
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
                channel: Some('x'),
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
                channel: Some('w')
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
                channel: Some('y')
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
                channel: Some('x'),
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
                channel: Some('x'),
                texcoords: Vec::new()
            })],
            input_dependencies(&graph, &attributes, &assignments, &dependent_lines)
        );
    }
}
