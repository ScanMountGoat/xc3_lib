use crate::{
    graph::{Expr, Graph},
    shader_database::{output_expr, Attributes},
};

use indexmap::{IndexMap, IndexSet};
use xc3_model::shader_database::{
    AttributeDependency, BufferDependency, Dependency, OutputExpr, TextureDependency,
};

pub fn input_dependencies(
    graph: &Graph,
    attributes: &Attributes,
    assignments: &[usize],
    dependent_lines: &[usize],
    exprs: &mut IndexSet<OutputExpr>,
    expr_to_index: &mut IndexMap<Expr, usize>,
) -> Vec<Dependency> {
    // TODO: Rework this to be cleaner and add more tests.
    let mut dependencies =
        texture_dependencies(graph, attributes, dependent_lines, exprs, expr_to_index);

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
    exprs: &mut IndexSet<OutputExpr>,
    expr_to_index: &mut IndexMap<Expr, usize>,
) -> Vec<Dependency> {
    dependent_lines
        .iter()
        .filter_map(|i| {
            // Check all exprs for binary ops, function args, etc.
            graph.nodes[*i]
                .input
                .exprs_recursive()
                .iter()
                .find_map(|e| texture_dependency(e, graph, attributes, exprs, expr_to_index))
        })
        .collect()
}

pub fn texture_dependency(
    e: &Expr,
    graph: &Graph,
    attributes: &Attributes,
    exprs: &mut IndexSet<OutputExpr>,
    expr_to_index: &mut IndexMap<Expr, usize>,
) -> Option<Dependency> {
    if let Expr::Func {
        name,
        args,
        channel,
    } = e
    {
        if name.starts_with("texture") {
            if let Some(Expr::Global { name, .. }) = args.first() {
                let texcoords = texcoord_args(args, graph, attributes, exprs, expr_to_index);

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

fn texcoord_args(
    args: &[Expr],
    graph: &Graph,
    attributes: &Attributes,
    exprs: &mut IndexSet<OutputExpr>,
    expr_to_index: &mut IndexMap<Expr, usize>,
) -> Vec<usize> {
    // Search recursively to find texcoord variables.
    // The first arg is always the texture name.
    // texture(arg0, vec2(arg2, arg3, ...))
    args.iter()
        .skip(1)
        .flat_map(|a| {
            a.exprs_recursive()
                .iter()
                .skip(1)
                .map(|e| output_expr(e, graph, attributes, exprs, expr_to_index))
                .collect::<Vec<_>>()
        })
        .collect()
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
    use xc3_model::shader_database::{AttributeDependency, Operation};

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

        // assert_eq!(
        //     vec![Dependency::Texture(TextureDependency {
        //         name: "texture1".into(),
        //         channel: Some('w'),
        //         texcoords: vec![
        //             OutputExpr::Value(Dependency::Attribute(AttributeDependency {
        //                 name: "in_attr0".into(),
        //                 channel: Some('x')
        //             })),
        //             OutputExpr::Value(Dependency::Attribute(AttributeDependency {
        //                 name: "in_attr0".into(),
        //                 channel: Some('w')
        //             }))
        //         ]
        //     })],
        //     input_dependencies(&graph, &attributes, &assignments, &dependent_lines)
        // );
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

        // assert_eq!(
        //     vec![Dependency::Texture(TextureDependency {
        //         name: "gTResidentTex05".into(),
        //         channel: Some('x'),
        //         texcoords: vec![
        //             OutputExpr::Func {
        //                 op: Operation::TexMatrix,
        //                 args: vec![
        //                     OutputExpr::Value(Dependency::Attribute(AttributeDependency {
        //                         name: "in_attr4".into(),
        //                         channel: Some('x')
        //                     })),
        //                     OutputExpr::Value(Dependency::Attribute(AttributeDependency {
        //                         name: "in_attr4".into(),
        //                         channel: Some('y')
        //                     })),
        //                     OutputExpr::Value(Dependency::Buffer(BufferDependency {
        //                         name: "U_Mate".into(),
        //                         field: "gTexMat".into(),
        //                         index: Some(0),
        //                         channel: Some('x'),
        //                     })),
        //                     OutputExpr::Value(Dependency::Buffer(BufferDependency {
        //                         name: "U_Mate".into(),
        //                         field: "gTexMat".into(),
        //                         index: Some(0),
        //                         channel: Some('y'),
        //                     })),
        //                     OutputExpr::Value(Dependency::Buffer(BufferDependency {
        //                         name: "U_Mate".into(),
        //                         field: "gTexMat".into(),
        //                         index: Some(0),
        //                         channel: Some('z'),
        //                     })),
        //                     OutputExpr::Value(Dependency::Buffer(BufferDependency {
        //                         name: "U_Mate".into(),
        //                         field: "gTexMat".into(),
        //                         index: Some(0),
        //                         channel: Some('w'),
        //                     }))
        //                 ]
        //             },
        //             OutputExpr::Func {
        //                 op: Operation::TexMatrix,
        //                 args: vec![
        //                     OutputExpr::Value(Dependency::Attribute(AttributeDependency {
        //                         name: "in_attr4".into(),
        //                         channel: Some('x')
        //                     })),
        //                     OutputExpr::Value(Dependency::Attribute(AttributeDependency {
        //                         name: "in_attr4".into(),
        //                         channel: Some('y')
        //                     })),
        //                     OutputExpr::Value(Dependency::Buffer(BufferDependency {
        //                         name: "U_Mate".into(),
        //                         field: "gTexMat".into(),
        //                         index: Some(1),
        //                         channel: Some('x'),
        //                     })),
        //                     OutputExpr::Value(Dependency::Buffer(BufferDependency {
        //                         name: "U_Mate".into(),
        //                         field: "gTexMat".into(),
        //                         index: Some(1),
        //                         channel: Some('y'),
        //                     })),
        //                     OutputExpr::Value(Dependency::Buffer(BufferDependency {
        //                         name: "U_Mate".into(),
        //                         field: "gTexMat".into(),
        //                         index: Some(1),
        //                         channel: Some('z'),
        //                     })),
        //                     OutputExpr::Value(Dependency::Buffer(BufferDependency {
        //                         name: "U_Mate".into(),
        //                         field: "gTexMat".into(),
        //                         index: Some(1),
        //                         channel: Some('w'),
        //                     }))
        //                 ]
        //             }
        //         ]
        //     })],
        //     input_dependencies(&graph, &attributes, &assignments, &dependent_lines)
        // );
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

        // assert_eq!(
        //     vec![Dependency::Texture(TextureDependency {
        //         name: "gTResidentTex04".into(),
        //         channel: Some('x'),
        //         texcoords: vec![
        //             OutputExpr::Func {
        //                 op: Operation::Mul,
        //                 args: vec![
        //                     OutputExpr::Value(Dependency::Attribute(AttributeDependency {
        //                         name: "in_attr4".into(),
        //                         channel: Some('x')
        //                     })),
        //                     OutputExpr::Value(Dependency::Buffer(BufferDependency {
        //                         name: "U_Mate".into(),
        //                         field: "gWrkFl4".into(),
        //                         index: Some(0),
        //                         channel: Some('z')
        //                     }))
        //                 ]
        //             },
        //             OutputExpr::Func {
        //                 op: Operation::Mul,
        //                 args: vec![
        //                     OutputExpr::Value(Dependency::Attribute(AttributeDependency {
        //                         name: "in_attr4".into(),
        //                         channel: Some('y')
        //                     })),
        //                     OutputExpr::Value(Dependency::Buffer(BufferDependency {
        //                         name: "U_Mate".into(),
        //                         field: "gWrkFl4".into(),
        //                         index: Some(0),
        //                         channel: Some('w')
        //                     }))
        //                 ]
        //             }
        //         ]
        //     })],
        //     input_dependencies(&graph, &attributes, &assignments, &dependent_lines)
        // );
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

        // assert_eq!(
        //     vec![Dependency::Texture(TextureDependency {
        //         name: "texture1".into(),
        //         channel: Some('z'),
        //         texcoords: vec![OutputExpr::Value(Dependency::Constant(1.0.into()))]
        //     })],
        //     input_dependencies(&graph, &attributes, &assignments, &dependent_lines)
        // );
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

        // assert_eq!(
        //     vec![
        //         Dependency::Texture(TextureDependency {
        //             name: "texture1".into(),
        //             channel: Some('z'),
        //             texcoords: vec![OutputExpr::Value(Dependency::Constant(1.0.into()))]
        //         }),
        //         Dependency::Texture(TextureDependency {
        //             name: "texture1".into(),
        //             channel: Some('w'),
        //             texcoords: vec![OutputExpr::Value(Dependency::Constant(1.0.into()))]
        //         })
        //     ],
        //     input_dependencies(&graph, &attributes, &assignments, &dependent_lines)
        // );
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

        // assert_eq!(
        //     vec![Dependency::Texture(TextureDependency {
        //         name: "texture1".into(),
        //         channel: Some('x'),
        //         texcoords: vec![OutputExpr::Value(Dependency::Constant(1.0.into()))]
        //     })],
        //     input_dependencies(
        //         &graph,
        //         &attributes,
        //         &graph.assignments_recursive("out_attr1", Some('x'), None),
        //         &graph.dependencies_recursive("out_attr1", Some('x'), None)
        //     )
        // );
        // assert_eq!(
        //     vec![Dependency::Buffer(BufferDependency {
        //         name: "U_Mate".into(),
        //         field: "data".into(),
        //         index: Some(1),
        //         channel: Some('w')
        //     })],
        //     input_dependencies(
        //         &graph,
        //         &attributes,
        //         &graph.assignments_recursive("out_attr1", Some('y'), None),
        //         &graph.dependencies_recursive("out_attr1", Some('y'), None)
        //     )
        // );
        // assert_eq!(
        //     vec![Dependency::Buffer(BufferDependency {
        //         name: "uniform_data".into(),
        //         field: Default::default(),
        //         index: Some(3),
        //         channel: Some('y')
        //     })],
        //     input_dependencies(
        //         &graph,
        //         &attributes,
        //         &graph.assignments_recursive("out_attr1", Some('z'), None),
        //         &graph.dependencies_recursive("out_attr1", Some('z'), None)
        //     )
        // );
        // assert_eq!(
        //     vec![Dependency::Constant(1.5.into())],
        //     input_dependencies(
        //         &graph,
        //         &attributes,
        //         &graph.assignments_recursive("out_attr1", Some('w'), None),
        //         &graph.dependencies_recursive("out_attr1", Some('w'), None)
        //     )
        // );
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

        // assert_eq!(
        //     vec![Dependency::Texture(TextureDependency {
        //         name: "tex".into(),
        //         channel: Some('x'),
        //         texcoords: vec![OutputExpr::Value(Dependency::Constant(0.0.into()))]
        //     })],
        //     input_dependencies(&graph, &attributes, &assignments, &dependent_lines)
        // );
    }
}
