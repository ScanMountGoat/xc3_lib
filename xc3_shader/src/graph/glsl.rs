use std::collections::BTreeMap;

use glsl_lang::{
    ast::{
        DeclarationData, ExprData, FunIdentifierData, InitializerData, Statement, StatementData,
        TranslationUnit, TypeSpecifierData,
    },
    transpiler::glsl::{show_type_specifier, FormattingState},
    visitor::{Host, Visit, Visitor},
};

use super::*;

#[derive(Debug, Default)]
struct AssignmentVisitor {
    assignments: Vec<AssignmentDependency>,

    // Cache the last line where each variable was assigned.
    last_assignment_index: BTreeMap<Output, usize>,
}

#[derive(Debug, Clone)]
struct AssignmentDependency {
    output: Output,
    input: Expr,
}

impl AssignmentVisitor {
    fn add_assignment(&mut self, output: Output, assignment_input: &glsl_lang::ast::Expr) {
        let input = input_expr(assignment_input, &self.last_assignment_index, None);

        let assignment = AssignmentDependency {
            output: output.clone(),
            input,
        };
        // The visitor doesn't track line numbers.
        // We only need to look up the assignments, so use the index instead.
        self.last_assignment_index
            .insert(output, self.assignments.len());
        self.assignments.push(assignment);
    }
}

impl Visitor for AssignmentVisitor {
    fn visit_statement(&mut self, statement: &Statement) -> Visit {
        match &statement.content {
            StatementData::Expression(expr) => {
                if let Some(ExprData::Assignment(lh, _, rh)) =
                    expr.content.0.as_ref().map(|c| &c.content)
                {
                    let output = match &lh.content {
                        ExprData::Variable(id) => Output {
                            name: id.to_string(),
                            channels: String::new(),
                        },
                        ExprData::IntConst(_) => todo!(),
                        ExprData::UIntConst(_) => todo!(),
                        ExprData::BoolConst(_) => todo!(),
                        ExprData::FloatConst(_) => todo!(),
                        ExprData::DoubleConst(_) => todo!(),
                        ExprData::Unary(_, _) => todo!(),
                        ExprData::Binary(_, _, _) => todo!(),
                        ExprData::Ternary(_, _, _) => todo!(),
                        ExprData::Assignment(_, _, _) => todo!(),
                        ExprData::Bracket(_, _) => todo!(),
                        ExprData::FunCall(_, _) => todo!(),
                        ExprData::Dot(e, channel) => {
                            if let ExprData::Variable(id) = &e.content {
                                Output {
                                    name: id.to_string(),
                                    channels: channel.to_string(),
                                }
                            } else {
                                todo!()
                            }
                        }
                        ExprData::PostInc(_) => todo!(),
                        ExprData::PostDec(_) => todo!(),
                        ExprData::Comma(_, _) => todo!(),
                    };

                    self.add_assignment(output, rh);
                }

                Visit::Children
            }
            StatementData::Declaration(decl) => {
                if let DeclarationData::InitDeclaratorList(l) = &decl.content {
                    // TODO: is it worth handling complex initializers?
                    if let Some(InitializerData::Simple(init)) =
                        l.head.initializer.as_ref().map(|c| &c.content)
                    {
                        let output = l.head.name.as_ref().unwrap().0.clone();
                        self.add_assignment(
                            Output {
                                name: output.to_string(),
                                channels: String::new(),
                            },
                            init,
                        );
                    }
                }

                Visit::Children
            }
            _ => Visit::Children,
        }
    }
}

impl Graph {
    pub fn from_glsl(translation_unit: &TranslationUnit) -> Self {
        // Visit each assignment to establish data dependencies.
        // This converts the code to a directed acyclic graph (DAG).
        let mut visitor = AssignmentVisitor::default();
        translation_unit.visit(&mut visitor);

        let nodes = visitor
            .assignments
            .into_iter()
            .map(|a| Node {
                output: a.output,
                input: a.input,
            })
            .collect();

        Self { nodes }
    }

    pub fn to_glsl(&self) -> String {
        let mut output = String::new();
        for node in &self.nodes {
            output += &self.node_to_glsl(node);
        }
        output
    }

    pub(crate) fn node_to_glsl(&self, node: &Node) -> String {
        let input_expr = self.expr_to_glsl(&node.input);
        let channels = channel_display(&node.output.channels);
        format!("{}{} = {input_expr};\n", node.output.name, channels)
    }

    fn expr_to_glsl(&self, input: &Expr) -> String {
        match input {
            Expr::Node {
                node_index,
                channels,
            } => format!(
                "{}{}",
                self.nodes[*node_index].output.name,
                channel_display(channels)
            ),
            Expr::Float(f) => f.to_string(),
            Expr::Int(i) => i.to_string(),
            Expr::Parameter {
                name,
                field,
                index,
                channels,
            } => match field {
                Some(field) => format!(
                    "{name}.{field}[{}]{}",
                    self.expr_to_glsl(index),
                    channel_display(channels)
                ),
                None => format!(
                    "{name}[{}]{}",
                    self.expr_to_glsl(index),
                    channel_display(channels)
                ),
            },
            Expr::Global { name, channels } => format!("{name}{}", channel_display(channels)),
            Expr::Add(a, b) => format!("{} + {}", self.expr_to_glsl(a), self.expr_to_glsl(b)),
            Expr::Sub(a, b) => format!("{} - {}", self.expr_to_glsl(a), self.expr_to_glsl(b)),
            Expr::Mul(a, b) => format!("{} * {}", self.expr_to_glsl(a), self.expr_to_glsl(b)),
            Expr::Div(a, b) => format!("{} / {}", self.expr_to_glsl(a), self.expr_to_glsl(b)),
            Expr::LShift(a, b) => format!("{} << {}", self.expr_to_glsl(a), self.expr_to_glsl(b)),
            Expr::RShift(a, b) => format!("{} >> {}", self.expr_to_glsl(a), self.expr_to_glsl(b)),
            Expr::Func {
                name,
                args,
                channels,
            } => format!(
                "{name}({}){}",
                args.iter()
                    .map(|a| self.expr_to_glsl(a))
                    .collect::<Vec<_>>()
                    .join(", "),
                channel_display(channels)
            ),
        }
    }
}

fn channel_display(channels: &str) -> String {
    if channels.is_empty() {
        String::new()
    } else {
        ".".to_string() + channels
    }
}

fn input_expr(
    expr: &glsl_lang::ast::Expr,
    last_assignment_index: &BTreeMap<Output, usize>,
    channel: Option<&str>,
) -> Expr {
    // Collect any variables used in an expression.
    // Code like fma(a, b, c) should return [a, b, c].
    // TODO: Include constants?
    match &expr.content {
        ExprData::Variable(i) => {
            // The base case is a single variable like temp_01.
            // Also handle values like buffer or texture names.
            match last_assignment_index.get(&Output {
                name: i.content.0.as_str().to_string(),
                channels: channel.unwrap_or_default().to_string(),
            }) {
                Some(i) => Expr::Node {
                    node_index: *i,
                    channels: channel.unwrap_or_default().to_string(),
                },
                None => Expr::Global {
                    name: i.content.0.to_string(),
                    channels: channel.unwrap_or_default().to_string(),
                },
            }
        }
        ExprData::IntConst(i) => Expr::Int(*i),
        ExprData::UIntConst(_) => todo!(),
        ExprData::BoolConst(_) => todo!(),
        ExprData::FloatConst(f) => Expr::Float(*f),
        ExprData::DoubleConst(_) => todo!(),
        ExprData::Unary(_, e) => input_expr(e, last_assignment_index, channel),
        ExprData::Binary(op, lh, rh) => {
            let a = Box::new(input_expr(lh, last_assignment_index, channel));
            let b = Box::new(input_expr(rh, last_assignment_index, channel));
            match &op.content {
                glsl_lang::ast::BinaryOpData::Or => todo!(),
                glsl_lang::ast::BinaryOpData::Xor => todo!(),
                glsl_lang::ast::BinaryOpData::And => todo!(),
                glsl_lang::ast::BinaryOpData::BitOr => todo!(),
                glsl_lang::ast::BinaryOpData::BitXor => todo!(),
                glsl_lang::ast::BinaryOpData::BitAnd => todo!(),
                glsl_lang::ast::BinaryOpData::Equal => todo!(),
                glsl_lang::ast::BinaryOpData::NonEqual => todo!(),
                glsl_lang::ast::BinaryOpData::Lt => todo!(),
                glsl_lang::ast::BinaryOpData::Gt => todo!(),
                glsl_lang::ast::BinaryOpData::Lte => todo!(),
                glsl_lang::ast::BinaryOpData::Gte => todo!(),
                glsl_lang::ast::BinaryOpData::LShift => Expr::LShift(a, b),
                glsl_lang::ast::BinaryOpData::RShift => Expr::RShift(a, b),
                glsl_lang::ast::BinaryOpData::Add => Expr::Add(a, b),
                glsl_lang::ast::BinaryOpData::Sub => Expr::Sub(a, b),
                glsl_lang::ast::BinaryOpData::Mult => Expr::Mul(a, b),
                glsl_lang::ast::BinaryOpData::Div => Expr::Div(a, b),
                glsl_lang::ast::BinaryOpData::Mod => todo!(),
            }
        }
        ExprData::Ternary(a, b, c) => {
            todo!()
        }
        ExprData::Assignment(_, _, _) => todo!(),
        ExprData::Bracket(e, specifier) => {
            let (name, field) = match &e.as_ref().content {
                ExprData::Variable(id) => {
                    // buffer[index].x
                    (id.content.to_string(), None)
                }
                ExprData::Dot(e, field) => {
                    if let ExprData::Variable(id) = &e.content {
                        // buffer.field[index].x
                        (id.content.to_string(), Some(field.0.to_string()))
                    } else {
                        todo!()
                    }
                }
                _ => todo!(),
            };

            let index = Box::new(input_expr(specifier, last_assignment_index, channel));

            Expr::Parameter {
                name,
                field,
                index,
                channels: channel.unwrap_or_default().to_string(),
            }
        }
        ExprData::FunCall(id, es) => {
            let name = match &id.content {
                FunIdentifierData::Expr(expr) => {
                    if let ExprData::Variable(id) = &expr.content {
                        // A normal function like "fma" or "texture".
                        id.to_string()
                    } else {
                        todo!()
                    }
                }
                FunIdentifierData::TypeSpecifier(ty) => {
                    // A type cast like "int(temp_0)".
                    let mut name = String::new();
                    show_type_specifier(&mut name, ty, &mut FormattingState::default()).unwrap();
                    name
                }
            };

            let args = es
                .iter()
                .map(|e| input_expr(e, last_assignment_index, None))
                .collect();

            Expr::Func {
                name,
                args,
                channels: channel.unwrap_or_default().to_string(),
            }
        }
        ExprData::Dot(e, channel) => {
            // Track the channels accessed by expressions like "value.rgb".
            // TODO: Detect buffer parameters?
            input_expr(e, last_assignment_index, Some(channel.content.0.as_str()))
        }
        ExprData::PostInc(e) => input_expr(e, last_assignment_index, channel),
        ExprData::PostDec(e) => input_expr(e, last_assignment_index, channel),
        ExprData::Comma(_, _) => todo!(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use glsl_lang::parse::DefaultParse;
    use indoc::indoc;
    use pretty_assertions::assert_eq;

    #[test]
    fn graph_from_glsl() {
        let glsl = indoc! {"
            layout (binding = 9, std140) uniform fp_c9
            {
                vec4 fp_c9_data[0x1000];
            };

            void main() 
            {
                float a = fp_c9_data[0].x;
                float b = in_attr0.z;
                float c = a * b;
                float d = fma(a, b, c);
                d = d + 1.0;
                OUT_Color.x = c - d;
            }
        "};
        let tu = TranslationUnit::parse(glsl).unwrap();

        assert_eq!(
            Graph {
                nodes: vec![
                    Node {
                        output: Output {
                            name: "a".to_string(),
                            channels: String::new(),
                        },
                        input: Expr::Parameter {
                            name: "fp_c9_data".to_string(),
                            field: None,
                            index: Box::new(Expr::Int(0)),
                            channels: "x".to_string(),
                        },
                    },
                    Node {
                        output: Output {
                            name: "b".to_string(),
                            channels: String::new(),
                        },
                        input: Expr::Global {
                            name: "in_attr0".to_string(),
                            channels: "z".to_string(),
                        },
                    },
                    Node {
                        output: Output {
                            name: "c".to_string(),
                            channels: String::new(),
                        },
                        input: Expr::Mul(
                            Box::new(Expr::Node {
                                node_index: 0,
                                channels: String::new(),
                            }),
                            Box::new(Expr::Node {
                                node_index: 1,
                                channels: String::new(),
                            }),
                        ),
                    },
                    Node {
                        output: Output {
                            name: "d".to_string(),
                            channels: String::new(),
                        },
                        input: Expr::Func {
                            name: "fma".to_string(),
                            args: vec![
                                Expr::Node {
                                    node_index: 0,
                                    channels: String::new(),
                                },
                                Expr::Node {
                                    node_index: 1,
                                    channels: String::new(),
                                },
                                Expr::Node {
                                    node_index: 2,
                                    channels: String::new(),
                                },
                            ],
                            channels: String::new()
                        },
                    },
                    Node {
                        output: Output {
                            name: "d".to_string(),
                            channels: String::new(),
                        },
                        input: Expr::Add(
                            Box::new(Expr::Node {
                                node_index: 3,
                                channels: String::new(),
                            }),
                            Box::new(Expr::Float(1.0)),
                        ),
                    },
                    Node {
                        output: Output {
                            name: "OUT_Color".to_string(),
                            channels: "x".to_string(),
                        },
                        input: Expr::Sub(
                            Box::new(Expr::Node {
                                node_index: 2,
                                channels: String::new(),
                            }),
                            Box::new(Expr::Node {
                                node_index: 4,
                                channels: String::new(),
                            }),
                        ),
                    },
                ]
            },
            Graph::from_glsl(&tu)
        );
    }

    #[test]
    fn graph_to_glsl() {
        let graph = Graph {
            nodes: vec![
                Node {
                    output: Output {
                        name: "a".to_string(),
                        channels: String::new(),
                    },
                    input: Expr::Parameter {
                        name: "fp_c9_data".to_string(),
                        field: None,
                        index: Box::new(Expr::Int(0)),
                        channels: "x".to_string(),
                    },
                },
                Node {
                    output: Output {
                        name: "b".to_string(),
                        channels: String::new(),
                    },
                    input: Expr::Global {
                        name: "in_attr0".to_string(),
                        channels: "z".to_string(),
                    },
                },
                Node {
                    output: Output {
                        name: "c".to_string(),
                        channels: String::new(),
                    },
                    input: Expr::Mul(
                        Box::new(Expr::Node {
                            node_index: 0,
                            channels: String::new(),
                        }),
                        Box::new(Expr::Node {
                            node_index: 1,
                            channels: String::new(),
                        }),
                    ),
                },
                Node {
                    output: Output {
                        name: "d".to_string(),
                        channels: String::new(),
                    },
                    input: Expr::Func {
                        name: "fma".to_string(),
                        args: vec![
                            Expr::Node {
                                node_index: 0,
                                channels: String::new(),
                            },
                            Expr::Node {
                                node_index: 1,
                                channels: String::new(),
                            },
                            Expr::Node {
                                node_index: 2,
                                channels: String::new(),
                            },
                        ],
                        channels: String::new(),
                    },
                },
                Node {
                    output: Output {
                        name: "d".to_string(),
                        channels: String::new(),
                    },
                    input: Expr::Add(
                        Box::new(Expr::Node {
                            node_index: 3,
                            channels: String::new(),
                        }),
                        Box::new(Expr::Float(1.0)),
                    ),
                },
                Node {
                    output: Output {
                        name: "OUT_Color".to_string(),
                        channels: "x".to_string(),
                    },
                    input: Expr::Sub(
                        Box::new(Expr::Node {
                            node_index: 2,
                            channels: String::new(),
                        }),
                        Box::new(Expr::Node {
                            node_index: 4,
                            channels: String::new(),
                        }),
                    ),
                },
            ],
        };
        assert_eq!(
            indoc! {"
                a = fp_c9_data[0].x;
                b = in_attr0.z;
                c = a * b;
                d = fma(a, b, c);
                d = d + 1;
                OUT_Color.x = c - d;
            "},
            graph.to_glsl()
        );
    }

    // TODO: Split into two test cases.
    #[test]
    fn graph_textures_to_and_from_glsl() {
        // Test some more varied syntax.
        let glsl = indoc! {"
            void main() 
            {
                float a = 1.0;
                float a2 = a * 5.0;
                float b = texture(texture1, vec2(a2 + 2.0, 1.0)).x;
                float c = data[int(b)];
            }
        "};
        let tu = TranslationUnit::parse(glsl).unwrap();
        let graph = Graph::from_glsl(&tu);
        assert_eq!(
            indoc! {"
                a = 1;
                a2 = a * 5;
                b = texture(texture1, vec2(a2 + 2, 1)).x;
                c = data[int(b)];
            "},
            graph.to_glsl()
        );
    }
}
