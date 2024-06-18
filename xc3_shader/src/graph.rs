use crate::dependencies::AssignmentVisitor;
use glsl_lang::{
    ast::{
        DeclarationData, Expr, ExprData, FunIdentifierData, Identifier, InitializerData, Statement,
        StatementData, TranslationUnit,
    },
    parse::DefaultParse,
    transpiler::glsl::{show_expr, FormattingState},
    visitor::{Host, Visit, Visitor},
};

/// A directed graph of shader assignments and operations.
/// This normalizes identifiers and preserves only the data flow of the code.
/// Two graphs that perform the same operations will be isomorphic even if
/// the variable names change or unrelated code lines are inserted between statements.
#[derive(Debug, PartialEq, Clone)]
struct Graph {
    nodes: Vec<Node>,
}

/// A single assignment statement of the form `output = operation(inputs);`.
#[derive(Debug, PartialEq, Clone)]
struct Node {
    output: Output,
    /// The operation performed on the inputs or `None` if assigned directly.
    operation: Option<Operation>,
    /// References to input values used in this assignment statement.
    inputs: Vec<Input>,
}

#[derive(Debug, PartialEq, Clone)]
enum Input {
    Node {
        node_index: usize,
        channels: String,
    },
    Constant(f32),
    Parameter {
        name: String,
        index: usize,
        channels: String,
    },
    Global {
        name: String,
        channels: String,
    },
}

#[derive(Debug, PartialEq, Clone)]
struct Output {
    name: String,
    channels: String,
}

#[derive(Debug, PartialEq, Clone)]
enum Operation {
    Add,
    Sub,
    Mul,
    Div,
    Func(String),
}

// TODO: more strongly typed channel swizzles?
// TODO: use this instead of line dependencies
// TODO: display impl for GLSL

impl Graph {
    fn from_glsl(translation_unit: &TranslationUnit) -> Self {
        // Visit each assignment to establish data dependencies.
        // This converts the code to a directed acyclic graph (DAG).
        let mut visitor = AssignmentVisitor::default();
        translation_unit.visit(&mut visitor);

        // TODO: convert the visitor into nodes.
        let nodes = visitor
            .assignments
            .into_iter()
            .map(|a| {
                Node {
                    output: Output {
                        name: a.output_var,
                        channels: String::new(),
                    },
                    inputs: a
                        .input_last_assignments
                        .into_iter()
                        .map(|(i, c)| match i {
                            crate::dependencies::LastAssignment::LineNumber(l) => Input::Node {
                                node_index: l,
                                channels: c.unwrap_or_default(),
                            },
                            crate::dependencies::LastAssignment::Global(name) => Input::Global {
                                name,
                                channels: c.unwrap_or_default(),
                            },
                            crate::dependencies::LastAssignment::Constant(f) => Input::Constant(f),
                        })
                        .collect(),
                    operation: expr_operation(&a.assignment_input),
                }
            })
            .collect();

        Self { nodes }
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

    use indoc::indoc;

    // TODO: Test case for converting this graph to GLSL.
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
                        inputs: vec![Input::Global {
                            name: "fp_c9_data".to_string(),
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
}
