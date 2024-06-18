use std::collections::BTreeMap;

use glsl_lang::{
    ast::{
        DeclarationData, Expr, ExprData, FunIdentifierData, InitializerData, Statement,
        StatementData, TranslationUnit,
    },
    transpiler::glsl::{show_expr, FormattingState},
    visitor::{Host, Visit, Visitor},
};

use super::*;

#[derive(Debug, Default)]
struct AssignmentVisitor {
    assignments: Vec<AssignmentDependency>,

    // Cache the last line where each variable was assigned.
    last_assignment_index: BTreeMap<String, usize>,
}

#[derive(Debug, Clone)]
struct AssignmentDependency {
    output_var: String,
    assignment_input: Expr,
    inputs: Vec<Input>,
}

impl AssignmentVisitor {
    fn add_assignment(&mut self, output: String, input: &Expr) {
        let mut inputs = Vec::new();
        add_inputs(input, &mut inputs, &self.last_assignment_index, None);

        let assignment = AssignmentDependency {
            output_var: output,
            inputs,
            assignment_input: input.clone(),
        };
        // The visitor doesn't track line numbers.
        // We only need to look up the assignments, so use the index instead.
        self.last_assignment_index
            .insert(assignment.output_var.clone(), self.assignments.len());
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
                    let mut output = String::new();
                    show_expr(&mut output, lh, &mut FormattingState::default()).unwrap();

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
                        self.add_assignment(output.to_string(), init);
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

        // TODO: convert the visitor into nodes.
        let nodes = visitor
            .assignments
            .into_iter()
            .map(|a| Node {
                output: Output {
                    name: a.output_var.clone(),
                    channels: String::new(),
                },
                inputs: a.inputs,
                operation: expr_operation(&a.assignment_input),
            })
            .collect();
        Self { nodes }
    }

    pub fn to_glsl(&self) -> String {
        let mut output = String::new();
        for node in &self.nodes {
            let input_expr = match &node.operation {
                Some(op) => match op {
                    Operation::Add => format!(
                        "{} + {}",
                        self.input_glsl(&node.inputs[0]),
                        self.input_glsl(&node.inputs[1])
                    ),
                    Operation::Sub => format!(
                        "{} - {}",
                        self.input_glsl(&node.inputs[0]),
                        self.input_glsl(&node.inputs[1])
                    ),
                    Operation::Mul => format!(
                        "{} * {}",
                        self.input_glsl(&node.inputs[0]),
                        self.input_glsl(&node.inputs[1])
                    ),
                    Operation::Div => format!(
                        "{} / {}",
                        self.input_glsl(&node.inputs[0]),
                        self.input_glsl(&node.inputs[1])
                    ),
                    Operation::Func(f) => format!(
                        "{f}({})",
                        node.inputs
                            .iter()
                            .map(|i| self.input_glsl(i))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                },
                None => self.input_glsl(&node.inputs[0]),
            };
            let channels = channel_display(&node.output.channels);
            output += &format!("{}{} = {input_expr};\n", node.output.name, channels);
        }
        output
    }

    fn input_glsl(&self, input: &Input) -> String {
        match input {
            Input::Node {
                node_index,
                channels,
            } => format!(
                "{}{}",
                self.nodes[*node_index].output.name,
                channel_display(channels)
            ),
            Input::Constant(f) => f.to_string(),
            Input::Parameter {
                name,
                field,
                index,
                channels,
            } => match field {
                Some(field) => format!("{name}.{field}[{index}]{}", channel_display(channels)),
                None => format!("{name}[{index}]{}", channel_display(channels)),
            },
            Input::Global { name, channels } => format!("{name}{}", channel_display(channels)),
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

// TODO: module for glsl?
fn add_inputs(
    expr: &Expr,
    inputs: &mut Vec<Input>,
    last_assignment_index: &BTreeMap<String, usize>,
    channel: Option<&str>,
) {
    // Collect any variables used in an expression.
    // Code like fma(a, b, c) should return [a, b, c].
    // TODO: Include constants?
    match &expr.content {
        ExprData::Variable(i) => {
            // The base case is a single variable like temp_01.
            // Also handle values like buffer or texture names.
            let input = match last_assignment_index.get(i.content.0.as_str()) {
                Some(i) => Input::Node {
                    node_index: *i,
                    channels: channel.unwrap_or_default().to_string(),
                },
                None => Input::Global {
                    name: i.content.0.to_string(),
                    channels: channel.unwrap_or_default().to_string(),
                },
            };
            inputs.push(input);
        }
        ExprData::IntConst(_) => (),
        ExprData::UIntConst(_) => (),
        ExprData::BoolConst(_) => (),
        ExprData::FloatConst(f) => {
            inputs.push(Input::Constant(*f));
        }
        ExprData::DoubleConst(_) => (),
        ExprData::Unary(_, e) => add_inputs(e, inputs, last_assignment_index, channel),
        ExprData::Binary(_, lh, rh) => {
            add_inputs(lh, inputs, last_assignment_index, channel);
            add_inputs(rh, inputs, last_assignment_index, channel);
        }
        ExprData::Ternary(a, b, c) => {
            add_inputs(a, inputs, last_assignment_index, channel);
            add_inputs(b, inputs, last_assignment_index, channel);
            add_inputs(c, inputs, last_assignment_index, channel);
        }
        ExprData::Assignment(_, _, _) => todo!(),
        ExprData::Bracket(e, specifier) => {
            // TODO: Expressions like array[index] depend on index.
            // TODO: Do we also need to depend on array itself?
            // add_inputs(e, inputs, channel);
            // add_inputs(specifier, inputs, channel);

            if let ExprData::IntConst(index) = &specifier.content {
                match &e.as_ref().content {
                    ExprData::Variable(id) => {
                        // buffer[index].x
                        inputs.push(Input::Parameter {
                            name: id.content.to_string(),
                            field: None,
                            index: *index as usize,
                            channels: channel.unwrap_or_default().to_string(),
                        });
                    }
                    ExprData::Dot(e, field) => {
                        if let ExprData::Variable(id) = &e.content {
                            // buffer.field[index].x
                            inputs.push(Input::Parameter {
                                name: id.content.to_string(),
                                field: Some(field.0.to_string()),
                                index: *index as usize,
                                channels: channel.unwrap_or_default().to_string(),
                            });
                        }
                    }
                    _ => (),
                }
            }
        }
        ExprData::FunCall(_, es) => {
            for e in es {
                add_inputs(e, inputs, last_assignment_index, channel);
            }
        }
        ExprData::Dot(e, channel) => {
            // Track the channels accessed by expressions like "value.rgb".
            // TODO: Detect buffer parameters?
            add_inputs(
                e,
                inputs,
                last_assignment_index,
                Some(channel.content.0.as_str()),
            )
        }
        ExprData::PostInc(e) => add_inputs(e, inputs, last_assignment_index, channel),
        ExprData::PostDec(e) => add_inputs(e, inputs, last_assignment_index, channel),
        ExprData::Comma(_, _) => todo!(),
    }
}

fn expr_operation(expr: &Expr) -> Option<Operation> {
    match &expr.content {
        ExprData::Variable(_) => None,
        ExprData::IntConst(_) => None,
        ExprData::UIntConst(_) => None,
        ExprData::BoolConst(_) => None,
        ExprData::FloatConst(_) => None,
        ExprData::DoubleConst(_) => None,
        ExprData::Unary(_, _) => todo!(),
        ExprData::Binary(op, _, _) => match op.content {
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
            glsl_lang::ast::BinaryOpData::LShift => todo!(),
            glsl_lang::ast::BinaryOpData::RShift => todo!(),
            glsl_lang::ast::BinaryOpData::Add => Some(Operation::Add),
            glsl_lang::ast::BinaryOpData::Sub => Some(Operation::Sub),
            glsl_lang::ast::BinaryOpData::Mult => Some(Operation::Mul),
            glsl_lang::ast::BinaryOpData::Div => Some(Operation::Div),
            glsl_lang::ast::BinaryOpData::Mod => todo!(),
        },
        ExprData::Ternary(_, _, _) => todo!(),
        ExprData::Assignment(_, _, _) => todo!(),
        ExprData::Bracket(_, _) => None,
        ExprData::FunCall(id, _) => {
            if let FunIdentifierData::Expr(expr) = &id.content {
                if let ExprData::Variable(id) = &expr.content {
                    return Some(Operation::Func(id.content.0.to_string()));
                }
            }
            todo!()
        }
        ExprData::Dot(e, _) => expr_operation(e),
        ExprData::PostInc(_) => todo!(),
        ExprData::PostDec(_) => todo!(),
        ExprData::Comma(_, _) => todo!(),
    }
}

// TODO: test converting to and from GLSL
#[cfg(test)]
mod tests {
    use super::*;

    use glsl_lang::parse::DefaultParse;
    use indoc::indoc;

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
                            channels: String::new()
                        },
                        operation: None,
                        inputs: vec![Input::Parameter {
                            name: "fp_c9_data".to_string(),
                            field: None,
                            index: 0,
                            channels: "x".to_string()
                        }],
                    },
                    Node {
                        output: Output {
                            name: "b".to_string(),
                            channels: String::new()
                        },
                        operation: None,
                        inputs: vec![Input::Global {
                            name: "in_attr0".to_string(),
                            channels: "z".to_string()
                        }],
                    },
                    Node {
                        output: Output {
                            name: "c".to_string(),
                            channels: String::new()
                        },
                        operation: Some(Operation::Mul),
                        inputs: vec![
                            Input::Node {
                                node_index: 0,
                                channels: String::new()
                            },
                            Input::Node {
                                node_index: 1,
                                channels: String::new()
                            }
                        ],
                    },
                    Node {
                        output: Output {
                            name: "d".to_string(),
                            channels: String::new()
                        },
                        operation: Some(Operation::Func("fma".to_string())),
                        inputs: vec![
                            Input::Node {
                                node_index: 0,
                                channels: String::new()
                            },
                            Input::Node {
                                node_index: 1,
                                channels: String::new()
                            },
                            Input::Node {
                                node_index: 2,
                                channels: String::new()
                            }
                        ],
                    },
                    Node {
                        output: Output {
                            name: "d".to_string(),
                            channels: String::new()
                        },
                        operation: Some(Operation::Add),
                        inputs: vec![
                            Input::Node {
                                node_index: 3,
                                channels: String::new()
                            },
                            Input::Constant(1.0)
                        ],
                    },
                    Node {
                        output: Output {
                            name: "OUT_Color.x".to_string(),
                            channels: String::new()
                        },
                        operation: Some(Operation::Sub),
                        inputs: vec![
                            Input::Node {
                                node_index: 2,
                                channels: String::new()
                            },
                            Input::Node {
                                node_index: 4,
                                channels: String::new()
                            }
                        ],
                    }
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
                    operation: None,
                    inputs: vec![Input::Parameter {
                        name: "fp_c9_data".to_string(),
                        field: None,
                        index: 0,
                        channels: "x".to_string(),
                    }],
                },
                Node {
                    output: Output {
                        name: "b".to_string(),
                        channels: String::new(),
                    },
                    operation: None,
                    inputs: vec![Input::Global {
                        name: "in_attr0".to_string(),
                        channels: "z".to_string(),
                    }],
                },
                Node {
                    output: Output {
                        name: "c".to_string(),
                        channels: String::new(),
                    },
                    operation: Some(Operation::Mul),
                    inputs: vec![
                        Input::Node {
                            node_index: 0,
                            channels: String::new(),
                        },
                        Input::Node {
                            node_index: 1,
                            channels: String::new(),
                        },
                    ],
                },
                Node {
                    output: Output {
                        name: "d".to_string(),
                        channels: String::new(),
                    },
                    operation: Some(Operation::Func("fma".to_string())),
                    inputs: vec![
                        Input::Node {
                            node_index: 0,
                            channels: String::new(),
                        },
                        Input::Node {
                            node_index: 1,
                            channels: String::new(),
                        },
                        Input::Node {
                            node_index: 2,
                            channels: String::new(),
                        },
                    ],
                },
                Node {
                    output: Output {
                        name: "d".to_string(),
                        channels: String::new(),
                    },
                    operation: Some(Operation::Add),
                    inputs: vec![
                        Input::Node {
                            node_index: 3,
                            channels: String::new(),
                        },
                        Input::Constant(1.0),
                    ],
                },
                Node {
                    output: Output {
                        name: "OUT_Color.x".to_string(),
                        channels: String::new(),
                    },
                    operation: Some(Operation::Sub),
                    inputs: vec![
                        Input::Node {
                            node_index: 2,
                            channels: String::new(),
                        },
                        Input::Node {
                            node_index: 4,
                            channels: String::new(),
                        },
                    ],
                },
            ],
        };
        pretty_assertions::assert_eq!(
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
}
