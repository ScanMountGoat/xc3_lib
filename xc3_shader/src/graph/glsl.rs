use std::collections::BTreeMap;

use glsl_lang::{
    ast::{
        DeclarationData, ExprData, FunIdentifierData, InitializerData, Statement, StatementData,
        TranslationUnit,
    },
    transpiler::glsl::{show_expr, show_type_specifier, FormattingState},
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
    fn add_assignment(
        &mut self,
        output_name: &str,
        output_channels: &str,
        assignment_input: &glsl_lang::ast::Expr,
    ) {
        let inputs = input_expr(assignment_input, &self.last_assignment_index);
        let mut channels = if output_channels.is_empty() && inputs.len() > 1 {
            "xyzw".chars()
        } else {
            output_channels.chars()
        };

        // Convert vector swizzles to scalar operations to simplify analysis code.
        for input in inputs {
            let assignment = AssignmentDependency {
                output: Output {
                    name: output_name.to_string(),
                    channel: channels.next(),
                },
                input,
            };
            // The visitor doesn't track line numbers.
            // We only need to look up the assignments, so use the index instead.
            self.last_assignment_index
                .insert(assignment.output.clone(), self.assignments.len());
            self.assignments.push(assignment);
        }
    }
}

impl Visitor for AssignmentVisitor {
    fn visit_statement(&mut self, statement: &Statement) -> Visit {
        match &statement.content {
            StatementData::Expression(expr) => {
                if let Some(ExprData::Assignment(lh, _, rh)) =
                    expr.content.0.as_ref().map(|c| &c.content)
                {
                    let (output_name, output_channels) = match &lh.content {
                        ExprData::Variable(id) => (id.to_string(), ""),
                        ExprData::IntConst(_) => todo!(),
                        ExprData::UIntConst(_) => todo!(),
                        ExprData::BoolConst(_) => todo!(),
                        ExprData::FloatConst(_) => todo!(),
                        ExprData::DoubleConst(_) => todo!(),
                        ExprData::Unary(_, _) => todo!(),
                        ExprData::Binary(_, _, _) => todo!(),
                        ExprData::Ternary(_, _, _) => todo!(),
                        ExprData::Assignment(_, _, _) => todo!(),
                        ExprData::Bracket(_, _) => {
                            // TODO: Better support for assigning to array elements?
                            let mut text = String::new();
                            show_expr(&mut text, lh, &mut FormattingState::default()).unwrap();
                            (text, "")
                        }
                        ExprData::FunCall(_, _) => todo!(),
                        ExprData::Dot(e, channel) => {
                            if let ExprData::Variable(id) = &e.content {
                                (id.to_string(), channel.as_str())
                            } else {
                                todo!()
                            }
                        }
                        ExprData::PostInc(_) => todo!(),
                        ExprData::PostDec(_) => todo!(),
                        ExprData::Comma(_, _) => todo!(),
                    };

                    self.add_assignment(&output_name, output_channels, rh);
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
                        self.add_assignment(&output, "", init);
                    }
                }

                Visit::Children
            }
            _ => Visit::Children,
        }
    }
}

impl Graph {
    /// Convert parsed GLSL into a graph representation.
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

    /// Pretty print the graph as GLSL code with an assignment line for each node.
    /// The output may not be valid GLSL and should only be used for debugging.
    pub fn to_glsl(&self) -> String {
        let mut output = String::new();
        for node in &self.nodes {
            output += &self.node_to_glsl(node);
        }
        output
    }

    pub(crate) fn node_to_glsl(&self, node: &Node) -> String {
        let input_expr = self.expr_to_glsl(&node.input);
        let channels = channel_swizzle(node.output.channel);
        format!("{}{} = {input_expr};\n", node.output.name, channels)
    }

    fn expr_to_glsl(&self, input: &Expr) -> String {
        match input {
            Expr::Node {
                node_index,
                channel,
            } => format!(
                "{}{}",
                self.nodes[*node_index].output.name,
                channel_swizzle(*channel)
            ),
            Expr::Float(f) => format!("{f:?}"),
            Expr::Int(i) => i.to_string(),
            Expr::Uint(u) => u.to_string(),
            Expr::Bool(b) => b.to_string(),
            Expr::Parameter {
                name,
                field,
                index,
                channel,
            } => match field {
                Some(field) => format!(
                    "{name}.{field}[{}]{}",
                    self.expr_to_glsl(index),
                    channel_swizzle(*channel)
                ),
                None => format!(
                    "{name}[{}]{}",
                    self.expr_to_glsl(index),
                    channel_swizzle(*channel)
                ),
            },
            Expr::Global { name, channel } => format!("{name}{}", channel_swizzle(*channel)),
            Expr::Add(a, b) => self.binary_to_glsl(a, "+", b),
            Expr::Sub(a, b) => self.binary_to_glsl(a, "-", b),
            Expr::Mul(a, b) => self.binary_to_glsl(a, "*", b),
            Expr::Div(a, b) => self.binary_to_glsl(a, "/", b),
            Expr::LeftShift(a, b) => self.binary_to_glsl(a, "<<", b),
            Expr::RightShift(a, b) => self.binary_to_glsl(a, ">>", b),
            Expr::BitOr(a, b) => self.binary_to_glsl(a, "|", b),
            Expr::BitXor(a, b) => self.binary_to_glsl(a, "^", b),
            Expr::BitAnd(a, b) => self.binary_to_glsl(a, "&", b),
            Expr::Equal(a, b) => self.binary_to_glsl(a, "==", b),
            Expr::NotEqual(a, b) => self.binary_to_glsl(a, "!=", b),
            Expr::Less(a, b) => self.binary_to_glsl(a, "<", b),
            Expr::Greater(a, b) => self.binary_to_glsl(a, ">", b),
            Expr::LessEqual(a, b) => self.binary_to_glsl(a, "<=", b),
            Expr::GreaterEqual(a, b) => self.binary_to_glsl(a, ">=", b),
            Expr::Or(a, b) => self.binary_to_glsl(a, "||", b),
            Expr::And(a, b) => self.binary_to_glsl(a, "&&", b),
            Expr::Negate(a) => self.unary_to_glsl("-", a),
            Expr::Not(a) => self.unary_to_glsl("!", a),
            Expr::Complement(a) => self.unary_to_glsl("~", a),
            Expr::Ternary(a, b, c) => format!(
                "{} ? {} : {}",
                self.expr_to_glsl(a),
                self.expr_to_glsl(b),
                self.expr_to_glsl(c)
            ),
            Expr::Func {
                name,
                args,
                channel,
            } => format!(
                "{name}({}){}",
                args.iter()
                    .map(|a| self.expr_to_glsl(a))
                    .collect::<Vec<_>>()
                    .join(", "),
                channel_swizzle(*channel)
            ),
        }
    }

    fn unary_to_glsl(&self, op: &str, a: &Expr) -> String {
        format!("{op}{}", self.expr_to_glsl(a))
    }

    fn binary_to_glsl(&self, a: &Expr, op: &str, b: &Expr) -> String {
        format!("{} {op} {}", self.expr_to_glsl(a), self.expr_to_glsl(b))
    }
}

fn channel_swizzle(channel: Option<char>) -> String {
    channel.map(|c| format!(".{c}")).unwrap_or_default()
}

fn input_expr(
    expr: &glsl_lang::ast::Expr,
    last_assignment_index: &BTreeMap<Output, usize>,
) -> Vec<Expr> {
    // Collect any variables used in an expression.
    // Code like fma(a, b, c) should return [a, b, c].
    if let ExprData::Dot(e, channel) = &expr.content {
        // Track the channels accessed by expressions like "value.rgb".
        channel
            .as_str()
            .chars()
            .map(|c| input_expr_inner(e, last_assignment_index, Some(c)))
            .collect()
    } else {
        vec![input_expr_inner(expr, last_assignment_index, None)]
    }
}

fn input_expr_inner(
    expr: &glsl_lang::ast::Expr,
    last_assignment_index: &BTreeMap<Output, usize>,
    channel: Option<char>,
) -> Expr {
    // Collect any variables used in an expression.
    // Code like fma(a, b, c) should return [a, b, c].
    match &expr.content {
        ExprData::Variable(i) => {
            // The base case is a single variable like temp_01.
            // Also handle values like buffer or texture names.
            // The previous assignment may not always have a channel.
            match last_assignment_index
                .get(&Output {
                    name: i.to_string(),
                    channel,
                })
                .or_else(|| {
                    last_assignment_index.get(&Output {
                        name: i.to_string(),
                        channel: None,
                    })
                }) {
                Some(i) => Expr::Node {
                    node_index: *i,
                    channel,
                },
                None => Expr::Global {
                    name: i.to_string(),
                    channel,
                },
            }
        }
        ExprData::IntConst(i) => Expr::Int(*i),
        ExprData::UIntConst(u) => Expr::Uint(*u),
        ExprData::BoolConst(b) => Expr::Bool(*b),
        ExprData::FloatConst(f) => Expr::Float(*f),
        ExprData::DoubleConst(_) => todo!(),
        ExprData::Unary(op, e) => {
            let a = Box::new(input_expr_inner(e, last_assignment_index, channel));
            match op.content {
                glsl_lang::ast::UnaryOpData::Inc => todo!(),
                glsl_lang::ast::UnaryOpData::Dec => todo!(),
                glsl_lang::ast::UnaryOpData::Add => todo!(),
                glsl_lang::ast::UnaryOpData::Minus => Expr::Negate(a),
                glsl_lang::ast::UnaryOpData::Not => Expr::Not(a),
                glsl_lang::ast::UnaryOpData::Complement => Expr::Complement(a),
            }
        }
        ExprData::Binary(op, lh, rh) => {
            let a = Box::new(input_expr_inner(lh, last_assignment_index, None));
            let b = Box::new(input_expr_inner(rh, last_assignment_index, None));
            match &op.content {
                // TODO: Fill in remaining ops.
                glsl_lang::ast::BinaryOpData::Or => Expr::Or(a, b),
                glsl_lang::ast::BinaryOpData::Xor => todo!(),
                glsl_lang::ast::BinaryOpData::And => Expr::And(a, b),
                glsl_lang::ast::BinaryOpData::BitOr => Expr::BitOr(a, b),
                glsl_lang::ast::BinaryOpData::BitXor => Expr::BitXor(a, b),
                glsl_lang::ast::BinaryOpData::BitAnd => Expr::BitAnd(a, b),
                glsl_lang::ast::BinaryOpData::Equal => Expr::Equal(a, b),
                glsl_lang::ast::BinaryOpData::NonEqual => Expr::NotEqual(a, b),
                glsl_lang::ast::BinaryOpData::Lt => Expr::Less(a, b),
                glsl_lang::ast::BinaryOpData::Gt => Expr::Greater(a, b),
                glsl_lang::ast::BinaryOpData::Lte => Expr::LessEqual(a, b),
                glsl_lang::ast::BinaryOpData::Gte => Expr::GreaterEqual(a, b),
                glsl_lang::ast::BinaryOpData::LShift => Expr::LeftShift(a, b),
                glsl_lang::ast::BinaryOpData::RShift => Expr::RightShift(a, b),
                glsl_lang::ast::BinaryOpData::Add => Expr::Add(a, b),
                glsl_lang::ast::BinaryOpData::Sub => Expr::Sub(a, b),
                glsl_lang::ast::BinaryOpData::Mult => Expr::Mul(a, b),
                glsl_lang::ast::BinaryOpData::Div => Expr::Div(a, b),
                glsl_lang::ast::BinaryOpData::Mod => todo!(),
            }
        }
        ExprData::Ternary(a, b, c) => {
            let a = Box::new(input_expr_inner(a, last_assignment_index, None));
            let b = Box::new(input_expr_inner(b, last_assignment_index, None));
            let c = Box::new(input_expr_inner(c, last_assignment_index, None));
            Expr::Ternary(a, b, c)
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
                        (id.content.to_string(), Some(field.to_string()))
                    } else {
                        todo!()
                    }
                }
                _ => {
                    // TODO: Better support for nested brackets like "U_BILL.data[int(temp_4)][temp_5];"
                    let mut text = String::new();
                    show_expr(&mut text, e, &mut FormattingState::default()).unwrap();
                    (text, None)
                }
            };

            let index = Box::new(input_expr_inner(specifier, last_assignment_index, None));

            Expr::Parameter {
                name,
                field,
                index,
                channel,
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

            // The function call channels don't affect its arguments.
            let args = es
                .iter()
                .map(|e| input_expr_inner(e, last_assignment_index, None))
                .collect();

            Expr::Func {
                name,
                args,
                channel,
            }
        }
        ExprData::Dot(e, channel) => {
            // Track the channels accessed by expressions like "value.rgb".
            if channel.as_str().len() == 1 {
                input_expr_inner(e, last_assignment_index, channel.as_str().chars().next())
            } else if !channel.as_str().chars().all(|c| "xyzw".contains(c)) {
                // TODO: Is there a better way to handle float params like U_Mate.gAlInf?
                let mut text = String::new();
                show_expr(&mut text, e, &mut FormattingState::default()).unwrap();
                Expr::Global {
                    name: text,
                    channel: None,
                }
            } else {
                // TODO: how to handle values with multiple channels like a.xyz * b.wzy?
                let mut text = String::new();
                show_expr(&mut text, e, &mut FormattingState::default()).unwrap();
                panic!("{}.{}\n", text, channel)
            }
        }
        ExprData::PostInc(e) => input_expr_inner(e, last_assignment_index, channel),
        ExprData::PostDec(e) => input_expr_inner(e, last_assignment_index, channel),
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
                            channel: None,
                        },
                        input: Expr::Parameter {
                            name: "fp_c9_data".to_string(),
                            field: None,
                            index: Box::new(Expr::Int(0)),
                            channel: Some('x'),
                        },
                    },
                    Node {
                        output: Output {
                            name: "b".to_string(),
                            channel: None,
                        },
                        input: Expr::Global {
                            name: "in_attr0".to_string(),
                            channel: Some('z'),
                        },
                    },
                    Node {
                        output: Output {
                            name: "c".to_string(),
                            channel: None,
                        },
                        input: Expr::Mul(
                            Box::new(Expr::Node {
                                node_index: 0,
                                channel: None,
                            }),
                            Box::new(Expr::Node {
                                node_index: 1,
                                channel: None,
                            }),
                        ),
                    },
                    Node {
                        output: Output {
                            name: "d".to_string(),
                            channel: None,
                        },
                        input: Expr::Func {
                            name: "fma".to_string(),
                            args: vec![
                                Expr::Node {
                                    node_index: 0,
                                    channel: None,
                                },
                                Expr::Node {
                                    node_index: 1,
                                    channel: None,
                                },
                                Expr::Node {
                                    node_index: 2,
                                    channel: None,
                                },
                            ],
                            channel: None
                        },
                    },
                    Node {
                        output: Output {
                            name: "d".to_string(),
                            channel: None,
                        },
                        input: Expr::Add(
                            Box::new(Expr::Node {
                                node_index: 3,
                                channel: None,
                            }),
                            Box::new(Expr::Float(1.0)),
                        ),
                    },
                    Node {
                        output: Output {
                            name: "OUT_Color".to_string(),
                            channel: Some('x'),
                        },
                        input: Expr::Sub(
                            Box::new(Expr::Node {
                                node_index: 2,
                                channel: None,
                            }),
                            Box::new(Expr::Node {
                                node_index: 4,
                                channel: None,
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
                        channel: None,
                    },
                    input: Expr::Parameter {
                        name: "fp_c9_data".to_string(),
                        field: None,
                        index: Box::new(Expr::Int(0)),
                        channel: Some('x'),
                    },
                },
                Node {
                    output: Output {
                        name: "b".to_string(),
                        channel: None,
                    },
                    input: Expr::Global {
                        name: "in_attr0".to_string(),
                        channel: Some('z'),
                    },
                },
                Node {
                    output: Output {
                        name: "c".to_string(),
                        channel: None,
                    },
                    input: Expr::Mul(
                        Box::new(Expr::Node {
                            node_index: 0,
                            channel: None,
                        }),
                        Box::new(Expr::Node {
                            node_index: 1,
                            channel: None,
                        }),
                    ),
                },
                Node {
                    output: Output {
                        name: "d".to_string(),
                        channel: None,
                    },
                    input: Expr::Func {
                        name: "fma".to_string(),
                        args: vec![
                            Expr::Node {
                                node_index: 0,
                                channel: None,
                            },
                            Expr::Node {
                                node_index: 1,
                                channel: None,
                            },
                            Expr::Node {
                                node_index: 2,
                                channel: None,
                            },
                        ],
                        channel: None,
                    },
                },
                Node {
                    output: Output {
                        name: "d".to_string(),
                        channel: None,
                    },
                    input: Expr::Add(
                        Box::new(Expr::Node {
                            node_index: 3,
                            channel: None,
                        }),
                        Box::new(Expr::Float(1.0)),
                    ),
                },
                Node {
                    output: Output {
                        name: "OUT_Color".to_string(),
                        channel: Some('x'),
                    },
                    input: Expr::Sub(
                        Box::new(Expr::Node {
                            node_index: 2,
                            channel: None,
                        }),
                        Box::new(Expr::Node {
                            node_index: 4,
                            channel: None,
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
                d = d + 1.0;
                OUT_Color.x = c - d;
            "},
            graph.to_glsl()
        );
    }

    #[test]
    fn graph_to_glsl_textures() {
        // Test some more varied syntax.
        let graph = Graph {
            nodes: vec![
                Node {
                    output: Output {
                        name: "a".to_string(),
                        channel: None,
                    },
                    input: Expr::Float(1.0),
                },
                Node {
                    output: Output {
                        name: "a2".to_string(),
                        channel: None,
                    },
                    input: Expr::Mul(
                        Box::new(Expr::Node {
                            node_index: 0,
                            channel: None,
                        }),
                        Box::new(Expr::Float(5.0)),
                    ),
                },
                Node {
                    output: Output {
                        name: "b".to_string(),
                        channel: None,
                    },
                    input: Expr::Func {
                        name: "texture".to_string(),
                        args: vec![
                            Expr::Global {
                                name: "texture1".to_string(),
                                channel: None,
                            },
                            Expr::Func {
                                name: "vec2".to_string(),
                                args: vec![
                                    Expr::Add(
                                        Box::new(Expr::Node {
                                            node_index: 1,
                                            channel: None,
                                        }),
                                        Box::new(Expr::Float(2.0)),
                                    ),
                                    Expr::Float(1.0),
                                ],
                                channel: None,
                            },
                        ],
                        channel: Some('x'),
                    },
                },
                Node {
                    output: Output {
                        name: "c".to_string(),
                        channel: None,
                    },
                    input: Expr::Parameter {
                        name: "data".to_string(),
                        field: None,
                        index: Box::new(Expr::Func {
                            name: "int".to_string(),
                            args: vec![Expr::Node {
                                node_index: 2,
                                channel: None,
                            }],
                            channel: None,
                        }),
                        channel: None,
                    },
                },
            ],
        };
        assert_eq!(
            indoc! {"
                a = 1.0;
                a2 = a * 5.0;
                b = texture(texture1, vec2(a2 + 2.0, 1.0)).x;
                c = data[int(b)];
            "},
            graph.to_glsl()
        );
    }

    #[test]
    fn graph_from_glsl_textures() {
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
        assert_eq!(
            Graph {
                nodes: vec![
                    Node {
                        output: Output {
                            name: "a".to_string(),
                            channel: None,
                        },
                        input: Expr::Float(1.0,),
                    },
                    Node {
                        output: Output {
                            name: "a2".to_string(),
                            channel: None,
                        },
                        input: Expr::Mul(
                            Box::new(Expr::Node {
                                node_index: 0,
                                channel: None,
                            }),
                            Box::new(Expr::Float(5.0,)),
                        ),
                    },
                    Node {
                        output: Output {
                            name: "b".to_string(),
                            channel: None,
                        },
                        input: Expr::Func {
                            name: "texture".to_string(),
                            args: vec![
                                Expr::Global {
                                    name: "texture1".to_string(),
                                    channel: None,
                                },
                                Expr::Func {
                                    name: "vec2".to_string(),
                                    args: vec![
                                        Expr::Add(
                                            Box::new(Expr::Node {
                                                node_index: 1,
                                                channel: None,
                                            }),
                                            Box::new(Expr::Float(2.0,)),
                                        ),
                                        Expr::Float(1.0,),
                                    ],
                                    channel: None,
                                },
                            ],
                            channel: Some('x'),
                        },
                    },
                    Node {
                        output: Output {
                            name: "c".to_string(),
                            channel: None,
                        },
                        input: Expr::Parameter {
                            name: "data".to_string(),
                            field: None,
                            index: Box::new(Expr::Func {
                                name: "int".to_string(),
                                args: vec![Expr::Node {
                                    node_index: 2,
                                    channel: None,
                                }],
                                channel: None,
                            }),
                            channel: None,
                        },
                    },
                ]
            },
            Graph::from_glsl(&tu)
        );
    }

    #[test]
    fn graph_from_glsl_vector_registers() {
        let glsl = indoc! {"
            void main() {
                R12.w = R9.z;
                PIX2.w = R12.w;
            }
        "};
        let tu = TranslationUnit::parse(glsl).unwrap();
        assert_eq!(
            Graph {
                nodes: vec![
                    Node {
                        output: Output {
                            name: "R12".to_string(),
                            channel: Some('w'),
                        },
                        input: Expr::Global {
                            name: "R9".to_string(),
                            channel: Some('z'),
                        },
                    },
                    Node {
                        output: Output {
                            name: "PIX2".to_string(),
                            channel: Some('w'),
                        },
                        input: Expr::Node {
                            node_index: 0,
                            channel: Some('w'),
                        },
                    },
                ],
            },
            Graph::from_glsl(&tu)
        );
    }
}
