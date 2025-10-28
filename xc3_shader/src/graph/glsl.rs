use std::collections::BTreeMap;
use std::fmt::Write;

use bimap::BiBTreeMap;
use glsl_lang::{
    ast::{
        DeclarationData, ExprData, FunIdentifierData, InitializerData, LayoutQualifierSpecData,
        SingleDeclaration, Statement, StatementData, StorageQualifierData, TranslationUnit,
        TypeQualifierSpecData,
    },
    parse::DefaultParse,
    transpiler::glsl::{FormattingState, show_expr, show_type_specifier},
    visitor::{Host, Visit, Visitor},
};
use log::error;
use smol_str::ToSmolStr;

use crate::database::remove_attribute_transforms;

use super::*;

#[derive(Debug, Default)]
struct AssignmentVisitor {
    assignments: Vec<AssignmentDependency>,

    exprs: IndexSet<Expr>,

    // Cache the last line where each variable was assigned.
    last_assignment_index: BTreeMap<Output, usize>,
}

#[derive(Debug, Clone)]
struct AssignmentDependency {
    output: Output,
    input: usize,
}

impl AssignmentVisitor {
    fn add_assignment(
        &mut self,
        output_name: SmolStr,
        output_channels: &str,
        assignment_input: &glsl_lang::ast::Expr,
    ) {
        let inputs = input_expr(
            assignment_input,
            &self.last_assignment_index,
            &mut self.exprs,
        );
        let mut channels = if output_channels.is_empty() && inputs.len() > 1 {
            "xyzw".chars()
        } else {
            output_channels.chars()
        };

        // Convert vector swizzles to scalar operations to simplify analysis code.
        for input in inputs {
            let assignment = AssignmentDependency {
                output: Output {
                    name: output_name.clone(),
                    channel: channels.next(),
                },
                input: self.exprs.insert_full(input).0,
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
                        ExprData::Variable(id) => (id.0.clone(), ""),
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
                            (text.to_smolstr(), "")
                        }
                        ExprData::FunCall(_, _) => todo!(),
                        ExprData::Dot(e, channel) => {
                            if let ExprData::Variable(id) = &e.content {
                                (id.0.clone(), channel.as_str())
                            } else {
                                todo!()
                            }
                        }
                        ExprData::PostInc(_) => todo!(),
                        ExprData::PostDec(_) => todo!(),
                        ExprData::Comma(_, _) => todo!(),
                    };

                    self.add_assignment(output_name, output_channels, rh);
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
                        self.add_assignment(output, "", init);
                    }
                }

                Visit::Children
            }
            _ => Visit::Children,
        }
    }

    fn visit_selection_statement(&mut self, _: &glsl_lang::ast::SelectionStatement) -> Visit {
        // TODO: How to properly handle if statements in graph?
        Visit::Parent
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

        Self {
            nodes,
            exprs: visitor.exprs.into_iter().collect(),
        }
    }

    /// Convert  GLSL into a graph representation.
    pub fn parse_glsl(glsl: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let tu = TranslationUnit::parse(glsl)?;
        Ok(Graph::from_glsl(&tu))
    }

    /// Pretty print the graph as GLSL code with an assignment line for each node.
    /// The output may not be valid GLSL and should only be used for debugging.
    pub fn to_glsl(&self) -> String {
        let mut output = String::new();
        for node in &self.nodes {
            self.write_node_glsl(&mut output, node);
        }
        output
    }

    pub(crate) fn write_node_glsl(&self, output: &mut String, node: &Node) {
        let channels = channel_swizzle(node.output.channel);
        write!(output, "{}{} = ", node.output.name, channels).unwrap();
        self.write_expr_glsl(output, node.input);
        write!(output, ";\n").unwrap();
    }

    pub fn write_expr_glsl(&self, output: &mut String, input: usize) {
        // TODO: Add parentheses for nested expressions.
        // TODO: write the strings instead for faster performance?
        // write ( write inner write )?
        match &self.exprs[input] {
            Expr::Node {
                node_index,
                channel,
            } => write!(
                output,
                "{}{}",
                self.nodes[*node_index].output.name,
                channel_swizzle(*channel)
            )
            .unwrap(),
            Expr::Float(f) => write!(output, "{f:?}").unwrap(),
            Expr::Int(i) => write!(output, "{}", i).unwrap(),
            Expr::Uint(u) => write!(output, "{}", u).unwrap(),
            Expr::Bool(b) => write!(output, "{}", b).unwrap(),
            Expr::Parameter {
                name,
                field,
                index,
                channel,
            } => {
                write!(output, "{name}",).unwrap();
                if let Some(f) = field {
                    write!(output, ".{f}").unwrap();
                }
                if let Some(i) = index {
                    write!(output, "[").unwrap();
                    self.write_expr_glsl(output, *i);
                    write!(output, "]").unwrap();
                }
                write!(output, "{}", channel_swizzle(*channel)).unwrap();
            }
            Expr::Global { name, channel } => {
                write!(output, "{name}{}", channel_swizzle(*channel)).unwrap()
            }
            Expr::Unary(op, a) => self.write_unary_glsl(output, *op, *a),
            Expr::Binary(op, a, b) => self.write_binary_glsl(output, *op, *a, *b),
            Expr::Ternary(a, b, c) => {
                self.write_expr_glsl(output, *a);
                write!(output, " ? ").unwrap();
                self.write_expr_glsl(output, *b);
                write!(output, " : ").unwrap();
                self.write_expr_glsl(output, *c);
            }
            Expr::Func {
                name,
                args,
                channel,
            } => {
                write!(output, "{name}(").unwrap();
                if let Some((last, args)) = args.split_last() {
                    for a in args {
                        self.write_expr_glsl(output, *a);
                        write!(output, ", ").unwrap();
                    }
                    self.write_expr_glsl(output, *last);
                }
                write!(output, "){}", channel_swizzle(*channel)).unwrap();
            }
        }
    }

    fn write_unary_glsl(&self, output: &mut String, op: UnaryOp, a: usize) {
        match op {
            UnaryOp::Negate => {
                write!(output, "-").unwrap();
                self.write_expr_glsl(output, a);
            }
            UnaryOp::Not => write!(output, "!").unwrap(),
            UnaryOp::Complement => {
                write!(output, "~").unwrap();
                self.write_expr_glsl(output, a);
                write!(output, ")").unwrap();
            }
            UnaryOp::IntBitsToFloat => {
                write!(output, "intBitsToFloat(").unwrap();
                self.write_expr_glsl(output, a);
                write!(output, ")").unwrap();
            }
            UnaryOp::FloatBitsToInt => {
                write!(output, "floatBitsToInt(").unwrap();
                self.write_expr_glsl(output, a);
                write!(output, ")").unwrap();
            }
            UnaryOp::UintBitsToFloat => {
                write!(output, "uintBitsToFloat(").unwrap();
                self.write_expr_glsl(output, a);
                write!(output, ")").unwrap();
            }
            UnaryOp::FloatBitsToUint => {
                write!(output, "floatBitsToUint(").unwrap();
                self.write_expr_glsl(output, a);
                write!(output, ")").unwrap();
            }
            UnaryOp::IntToFloat => {
                write!(output, "float(").unwrap();
                self.write_expr_glsl(output, a);
                write!(output, ")").unwrap();
            }
            UnaryOp::UintToFloat => {
                write!(output, "float(").unwrap();
                self.write_expr_glsl(output, a);
                write!(output, ")").unwrap();
            }
            UnaryOp::FloatToInt => {
                write!(output, "int(").unwrap();
                self.write_expr_glsl(output, a);
                write!(output, ")").unwrap();
            }
            UnaryOp::FloatToUint => {
                write!(output, "uint(").unwrap();
                self.write_expr_glsl(output, a);
                write!(output, ")").unwrap();
            }
        }
    }

    fn write_binary_glsl(&self, output: &mut String, op: BinaryOp, a: usize, b: usize) {
        let op = match op {
            BinaryOp::Add => "+",
            BinaryOp::Sub => "-",
            BinaryOp::Mul => "*",
            BinaryOp::Div => "/",
            BinaryOp::LeftShift => "<<",
            BinaryOp::RightShift => ">>",
            BinaryOp::BitOr => "|",
            BinaryOp::BitXor => "^",
            BinaryOp::BitAnd => "&",
            BinaryOp::Equal => "==",
            BinaryOp::NotEqual => "!=",
            BinaryOp::Less => "<",
            BinaryOp::Greater => ">",
            BinaryOp::LessEqual => "<=",
            BinaryOp::GreaterEqual => ">=",
            BinaryOp::Or => "||",
            BinaryOp::And => "&&",
        };
        self.write_expr_glsl(output, a);
        write!(output, " {op} ").unwrap();
        self.write_expr_glsl(output, b);
    }
}

pub fn glsl_dependencies(source: &str, variable: &str, channel: Option<char>) -> String {
    let source = shader_source_no_extensions(source);
    let translation_unit = TranslationUnit::parse(source).unwrap();
    Graph::from_glsl(&translation_unit).glsl_dependencies(variable, channel, None)
}

pub fn shader_source_no_extensions(glsl: &str) -> &str {
    // TODO: Find a better way to skip unsupported extensions.
    glsl.find("#pragma").map(|i| &glsl[i..]).unwrap_or(glsl)
}

fn channel_swizzle(channel: Option<char>) -> String {
    channel.map(|c| format!(".{c}")).unwrap_or_default()
}

fn input_expr(
    expr: &glsl_lang::ast::Expr,
    last_assignment_index: &BTreeMap<Output, usize>,
    exprs: &mut IndexSet<Expr>,
) -> Vec<Expr> {
    // Collect any variables used in an expression.
    // Code like fma(a, b, c) should return [a, b, c].
    if let ExprData::Dot(e, channel) = &expr.content {
        // Track the channels accessed by expressions like "value.rgb".
        channel
            .as_str()
            .chars()
            .map(|c| input_expr_inner(e, last_assignment_index, exprs, Some(c)))
            .collect()
    } else {
        vec![input_expr_inner(expr, last_assignment_index, exprs, None)]
    }
}

fn input_expr_inner(
    expr: &glsl_lang::ast::Expr,
    last_assignment_index: &BTreeMap<Output, usize>,
    exprs: &mut IndexSet<Expr>,
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
                    name: i.0.clone(),
                    channel,
                })
                .or_else(|| {
                    last_assignment_index.get(&Output {
                        name: i.0.clone(),
                        channel: None,
                    })
                }) {
                Some(i) => Expr::Node {
                    node_index: *i,
                    channel,
                },
                None => Expr::Global {
                    name: i.0.clone(),
                    channel,
                },
            }
        }
        ExprData::IntConst(i) => Expr::Int(*i),
        ExprData::UIntConst(u) => Expr::Uint(*u),
        ExprData::BoolConst(b) => Expr::Bool(*b),
        ExprData::FloatConst(f) => Expr::Float((*f).into()),
        ExprData::DoubleConst(_) => todo!(),
        ExprData::Unary(op, e) => {
            let a = input_expr_inner(e, last_assignment_index, exprs, channel);
            let a = exprs.insert_full(a).0;
            let op = match op.content {
                glsl_lang::ast::UnaryOpData::Inc => todo!(),
                glsl_lang::ast::UnaryOpData::Dec => todo!(),
                glsl_lang::ast::UnaryOpData::Add => todo!(),
                glsl_lang::ast::UnaryOpData::Minus => UnaryOp::Negate,
                glsl_lang::ast::UnaryOpData::Not => UnaryOp::Not,
                glsl_lang::ast::UnaryOpData::Complement => UnaryOp::Complement,
            };
            Expr::Unary(op, a)
        }
        ExprData::Binary(op, lh, rh) => {
            let a = input_expr_inner(lh, last_assignment_index, exprs, None);
            let a = exprs.insert_full(a).0;

            let b = input_expr_inner(rh, last_assignment_index, exprs, None);
            let b = exprs.insert_full(b).0;

            let op = match &op.content {
                // TODO: Fill in remaining ops.
                glsl_lang::ast::BinaryOpData::Or => BinaryOp::Or,
                glsl_lang::ast::BinaryOpData::Xor => todo!(),
                glsl_lang::ast::BinaryOpData::And => BinaryOp::And,
                glsl_lang::ast::BinaryOpData::BitOr => BinaryOp::BitOr,
                glsl_lang::ast::BinaryOpData::BitXor => BinaryOp::BitXor,
                glsl_lang::ast::BinaryOpData::BitAnd => BinaryOp::BitAnd,
                glsl_lang::ast::BinaryOpData::Equal => BinaryOp::Equal,
                glsl_lang::ast::BinaryOpData::NonEqual => BinaryOp::NotEqual,
                glsl_lang::ast::BinaryOpData::Lt => BinaryOp::Less,
                glsl_lang::ast::BinaryOpData::Gt => BinaryOp::Greater,
                glsl_lang::ast::BinaryOpData::Lte => BinaryOp::LessEqual,
                glsl_lang::ast::BinaryOpData::Gte => BinaryOp::GreaterEqual,
                glsl_lang::ast::BinaryOpData::LShift => BinaryOp::LeftShift,
                glsl_lang::ast::BinaryOpData::RShift => BinaryOp::RightShift,
                glsl_lang::ast::BinaryOpData::Add => BinaryOp::Add,
                glsl_lang::ast::BinaryOpData::Sub => BinaryOp::Sub,
                glsl_lang::ast::BinaryOpData::Mult => BinaryOp::Mul,
                glsl_lang::ast::BinaryOpData::Div => BinaryOp::Div,
                glsl_lang::ast::BinaryOpData::Mod => todo!(),
            };
            Expr::Binary(op, a, b)
        }
        ExprData::Ternary(a, b, c) => {
            let a = input_expr_inner(a, last_assignment_index, exprs, None);
            let a = exprs.insert_full(a).0;

            let b = input_expr_inner(b, last_assignment_index, exprs, None);
            let b = exprs.insert_full(b).0;

            let c = input_expr_inner(c, last_assignment_index, exprs, None);
            let c = exprs.insert_full(c).0;

            Expr::Ternary(a, b, c)
        }
        ExprData::Assignment(_, _, _) => todo!(),
        ExprData::Bracket(e, specifier) => {
            let (name, field) = match &e.as_ref().content {
                ExprData::Variable(id) => {
                    // buffer[index].x
                    (id.0.clone(), None)
                }
                ExprData::Dot(e, field) => {
                    if let ExprData::Variable(id) = &e.content {
                        // buffer.field[index].x
                        (id.0.clone(), Some(field.0.clone()))
                    } else {
                        todo!()
                    }
                }
                ExprData::Bracket(e2, specifier2) => {
                    if let ExprData::Dot(e, field) = &e2.content {
                        if let ExprData::Variable(id) = &e.content {
                            // TODO: Add proper support for multiple brackets in the graph itself?
                            // buffer.field[index2][index]
                            let mut index2 = String::new();
                            show_expr(&mut index2, specifier2, &mut FormattingState::default())
                                .unwrap();

                            (id.0.clone(), Some(format!("{field}[{index2}]").into()))
                        } else {
                            todo!()
                        }
                    } else {
                        todo!()
                    }
                }
                _ => {
                    let mut text = String::new();
                    show_expr(&mut text, e, &mut FormattingState::default()).unwrap();
                    error!("Unsupported bracket expr {text:?}");
                    (text.into(), None)
                }
            };

            let index = input_expr_inner(specifier, last_assignment_index, exprs, None);

            Expr::Parameter {
                name,
                field,
                index: Some(exprs.insert_full(index).0),
                channel,
            }
        }
        ExprData::FunCall(id, es) => {
            let name = match &id.content {
                FunIdentifierData::Expr(expr) => {
                    if let ExprData::Variable(id) = &expr.content {
                        // A normal function like "fma" or "texture".
                        id.0.clone()
                    } else {
                        todo!()
                    }
                }
                FunIdentifierData::TypeSpecifier(ty) => {
                    // A type cast like "int(temp_0)".
                    let mut name = String::new();
                    show_type_specifier(&mut name, ty, &mut FormattingState::default()).unwrap();
                    name.into()
                }
            };

            // The function call channels don't affect its arguments.
            let args = es
                .iter()
                .map(|e| {
                    let arg = input_expr_inner(e, last_assignment_index, exprs, None);
                    exprs.insert_full(arg).0
                })
                .collect();

            Expr::Func {
                name,
                args,
                channel,
            }
        }
        ExprData::Dot(e, rh) => {
            // Track the channels accessed by expressions like "value.rgb".
            if rh.as_str().len() == 1 {
                input_expr_inner(e, last_assignment_index, exprs, rh.as_str().chars().next())
            } else if !rh.as_str().chars().all(|c| "xyzw".contains(c)) {
                let name = match &e.as_ref().content {
                    ExprData::Variable(id) => id.0.clone(),
                    _ => todo!(),
                };

                // Handle params like U_Mate.gAlInf.w.
                Expr::Parameter {
                    name,
                    field: Some(rh.0.clone()),
                    index: None,
                    channel,
                }
            } else {
                // TODO: how to handle values with multiple channels like a.xyz * b.wzy?
                // TODO: These should already be split up into multiple scalar operations?
                let mut text = String::new();
                show_expr(&mut text, e, &mut FormattingState::default()).unwrap();
                panic!("{text}.{rh}\n")
            }
        }
        ExprData::PostInc(e) => input_expr_inner(e, last_assignment_index, exprs, channel),
        ExprData::PostDec(e) => input_expr_inner(e, last_assignment_index, exprs, channel),
        ExprData::Comma(_, _) => todo!(),
    }
}

#[derive(Debug, Default)]
struct AttributeVisitor {
    attributes: Attributes,
}

#[derive(Debug, Default, PartialEq)]
pub struct Attributes {
    pub input_locations: BiBTreeMap<SmolStr, i32>,
    pub output_locations: BiBTreeMap<SmolStr, i32>,
}

impl Visitor for AttributeVisitor {
    fn visit_single_declaration(&mut self, declaration: &SingleDeclaration) -> Visit {
        if let Some(name) = &declaration.name
            && let Some(qualifier) = &declaration.ty.content.qualifier
        {
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
                        if let Some(id) = layout.content.ids.first()
                            && let LayoutQualifierSpecData::Identifier(key, value) = &id.content
                            && key.0 == "location"
                            && let Some(ExprData::IntConst(i)) = value.as_ref().map(|v| &v.content)
                        {
                            location = Some(*i);
                        }
                    }
                    _ => (),
                }
            }

            if let (Some(is_input), Some(location)) = (is_input, location) {
                if is_input {
                    self.attributes
                        .input_locations
                        .insert(name.0.clone(), location);
                } else {
                    self.attributes
                        .output_locations
                        .insert(name.0.clone(), location);
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

pub fn merge_vertex_fragment(
    vert: Graph,
    vert_attributes: &Attributes,
    frag: Graph,
    frag_attributes: &Attributes,
) -> Graph {
    let mut exprs: IndexSet<_> = vert.exprs.iter().cloned().collect();
    let mut graph = vert;

    // Use an offset to make sure fragment node references are preserved properly.
    let start = graph.nodes.len();
    for n in &frag.nodes {
        let input =
            fragment_input_to_vertex_output(&graph, vert_attributes, &frag, frag_attributes, n)
                .map(|e| exprs.insert_full(e).0)
                .unwrap_or_else(|| reindex_node_expr(&frag, &mut exprs, n.input, start));

        graph.nodes.push(Node {
            output: n.output.clone(),
            input,
        });
    }

    graph.exprs = exprs.into_iter().collect();

    graph
}

fn fragment_input_to_vertex_output(
    vert: &Graph,
    vert_attributes: &Attributes,
    frag: &Graph,
    frag_attributes: &Attributes,
    new_node: &Node,
) -> Option<Expr> {
    if let Expr::Global { name, channel } = &frag.exprs[new_node.input] {
        // Convert a fragment input like "in_attr4" to its vertex output like "out_attr4".
        if let Some(fragment_location) = frag_attributes.input_locations.get_by_left(name.as_str())
            && let Some(vertex_output_name) = vert_attributes
                .output_locations
                .get_by_right(fragment_location)
        {
            // This will search vertex nodes first even if a fragment output has the same name.
            if let Some(node) = vert
                .nodes
                .iter()
                .find(|n| &n.output.name == vertex_output_name && n.output.channel == *channel)
            {
                // Remove attribute skinning if present, so queries can detect globals like "vNormal.x".
                // TODO: Make this configurable.
                let expr = remove_attribute_transforms(vert, &vert.exprs[node.input]);
                return Some(expr);
            }
        }
    }

    None
}

fn reindex_node_expr(
    old_graph: &Graph,
    exprs: &mut IndexSet<Expr>,
    input: usize,
    start_index: usize,
) -> usize {
    // Recursively shift node indices to match their new position.
    let new_expr = match &old_graph.exprs[input] {
        Expr::Node {
            node_index,
            channel,
        } => Expr::Node {
            node_index: *node_index + start_index,
            channel: *channel,
        },
        Expr::Parameter {
            name,
            field,
            index,
            channel,
        } => Expr::Parameter {
            name: name.clone(),
            field: field.clone(),
            index: index.map(|i| reindex_node_expr(old_graph, exprs, i, start_index)),
            channel: *channel,
        },
        Expr::Unary(op, a) => {
            Expr::Unary(*op, reindex_node_expr(old_graph, exprs, *a, start_index))
        }
        Expr::Binary(op, lh, rh) => Expr::Binary(
            *op,
            reindex_node_expr(old_graph, exprs, *lh, start_index),
            reindex_node_expr(old_graph, exprs, *rh, start_index),
        ),
        Expr::Ternary(a, b, c) => Expr::Ternary(
            reindex_node_expr(old_graph, exprs, *a, start_index),
            reindex_node_expr(old_graph, exprs, *b, start_index),
            reindex_node_expr(old_graph, exprs, *c, start_index),
        ),
        Expr::Func {
            name,
            args,
            channel,
        } => Expr::Func {
            name: name.clone(),
            args: args
                .iter()
                .map(|a| reindex_node_expr(old_graph, exprs, *a, start_index))
                .collect(),
            channel: *channel,
        },
        e => e.clone(),
    };
    exprs.insert_full(new_expr).0
}

#[cfg(test)]
mod tests {
    use super::*;

    use glsl_lang::parse::DefaultParse;
    use indoc::indoc;
    use pretty_assertions::assert_eq;

    #[test]
    fn graph_glsl_basic() {
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

        let graph = Graph {
            nodes: vec![
                Node {
                    output: Output {
                        name: "a".into(),
                        channel: None,
                    },
                    input: 1,
                },
                Node {
                    output: Output {
                        name: "b".into(),
                        channel: None,
                    },
                    input: 2,
                },
                Node {
                    output: Output {
                        name: "c".into(),
                        channel: None,
                    },
                    input: 5,
                },
                Node {
                    output: Output {
                        name: "d".into(),
                        channel: None,
                    },
                    input: 7,
                },
                Node {
                    output: Output {
                        name: "d".into(),
                        channel: None,
                    },
                    input: 10,
                },
                Node {
                    output: Output {
                        name: "OUT_Color".into(),
                        channel: Some('x'),
                    },
                    input: 12,
                },
            ],
            exprs: vec![
                Expr::Int(0),
                Expr::Parameter {
                    name: "fp_c9_data".into(),
                    field: None,
                    index: Some(0),
                    channel: Some('x'),
                },
                Expr::Global {
                    name: "in_attr0".into(),
                    channel: Some('z'),
                },
                Expr::Node {
                    node_index: 0,
                    channel: None,
                },
                Expr::Node {
                    node_index: 1,
                    channel: None,
                },
                Expr::Binary(BinaryOp::Mul, 3, 4),
                Expr::Node {
                    node_index: 2,
                    channel: None,
                },
                Expr::Func {
                    name: "fma".into(),
                    args: vec![3, 4, 6],
                    channel: None,
                },
                Expr::Node {
                    node_index: 3,
                    channel: None,
                },
                Expr::Float(1.0.into()),
                Expr::Binary(BinaryOp::Add, 8, 9),
                Expr::Node {
                    node_index: 4,
                    channel: None,
                },
                Expr::Binary(BinaryOp::Sub, 6, 11),
            ],
        };
        assert_eq!(graph, Graph::from_glsl(&tu));

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
    fn graph_glsl_textures() {
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

        let graph = Graph {
            nodes: vec![
                Node {
                    output: Output {
                        name: "a".into(),
                        channel: None,
                    },
                    input: 0,
                },
                Node {
                    output: Output {
                        name: "a2".into(),
                        channel: None,
                    },
                    input: 3,
                },
                Node {
                    output: Output {
                        name: "b".into(),
                        channel: None,
                    },
                    input: 9,
                },
                Node {
                    output: Output {
                        name: "c".into(),
                        channel: None,
                    },
                    input: 12,
                },
            ],
            exprs: vec![
                Expr::Float(1.0.into()),
                Expr::Node {
                    node_index: 0,
                    channel: None,
                },
                Expr::Float(5.0.into()),
                Expr::Binary(BinaryOp::Mul, 1, 2),
                Expr::Global {
                    name: "texture1".into(),
                    channel: None,
                },
                Expr::Node {
                    node_index: 1,
                    channel: None,
                },
                Expr::Float(2.0.into()),
                Expr::Binary(BinaryOp::Add, 5, 6),
                Expr::Func {
                    name: "vec2".into(),
                    args: vec![7, 0],
                    channel: None,
                },
                Expr::Func {
                    name: "texture".into(),
                    args: vec![4, 8],
                    channel: Some('x'),
                },
                Expr::Node {
                    node_index: 2,
                    channel: None,
                },
                Expr::Func {
                    name: "int".into(),
                    args: vec![10],
                    channel: None,
                },
                Expr::Parameter {
                    name: "data".into(),
                    field: None,
                    index: Some(11),
                    channel: None,
                },
            ],
        };
        assert_eq!(graph, Graph::from_glsl(&tu));

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
    fn graph_glsl_vector_registers() {
        let glsl = indoc! {"
            void main() {
                R12.w = R9.z;
                PIX2.w = R12.w;
            }
        "};
        let tu = TranslationUnit::parse(glsl).unwrap();

        let graph = Graph {
            nodes: vec![
                Node {
                    output: Output {
                        name: "R12".into(),
                        channel: Some('w'),
                    },
                    input: 0,
                },
                Node {
                    output: Output {
                        name: "PIX2".into(),
                        channel: Some('w'),
                    },
                    input: 1,
                },
            ],
            exprs: vec![
                Expr::Global {
                    name: "R9".into(),
                    channel: Some('z'),
                },
                Expr::Node {
                    node_index: 0,
                    channel: Some('w'),
                },
            ],
        };
        assert_eq!(graph, Graph::from_glsl(&tu));

        assert_eq!(
            indoc! {"
                R12.w = R9.z;
                PIX2.w = R12.w;
            "},
            graph.to_glsl()
        );
    }

    #[test]
    fn graph_glsl_parameters() {
        let glsl = indoc! {"
            void main() {
                f0 = U_BILL.data[int(temp_4)][temp_5];
                f1 = U_POST.data[int(temp_206)];
                f2 = U_Mate.gMatCol.x;
                f3 = U_Mate.gWrkCol[1].w;
            }
        "};
        let tu = TranslationUnit::parse(glsl).unwrap();

        let graph = Graph {
            nodes: vec![
                Node {
                    output: Output {
                        name: "f0".into(),
                        channel: None,
                    },
                    input: 1,
                },
                Node {
                    output: Output {
                        name: "f1".into(),
                        channel: None,
                    },
                    input: 4,
                },
                Node {
                    output: Output {
                        name: "f2".into(),
                        channel: None,
                    },
                    input: 5,
                },
                Node {
                    output: Output {
                        name: "f3".into(),
                        channel: None,
                    },
                    input: 7,
                },
            ],
            exprs: vec![
                Expr::Global {
                    name: "temp_5".into(),
                    channel: None,
                },
                Expr::Parameter {
                    name: "U_BILL".into(),
                    field: Some("data[int(temp_4)]".into()),
                    index: Some(0),
                    channel: None,
                },
                Expr::Global {
                    name: "temp_206".into(),
                    channel: None,
                },
                Expr::Func {
                    name: "int".into(),
                    args: vec![2],
                    channel: None,
                },
                Expr::Parameter {
                    name: "U_POST".into(),
                    field: Some("data".into()),
                    index: Some(3),
                    channel: None,
                },
                Expr::Parameter {
                    name: "U_Mate".into(),
                    field: Some("gMatCol".into()),
                    index: None,
                    channel: Some('x'),
                },
                Expr::Int(1),
                Expr::Parameter {
                    name: "U_Mate".into(),
                    field: Some("gWrkCol".into()),
                    index: Some(6),
                    channel: Some('w'),
                },
            ],
        };
        assert_eq!(graph, Graph::from_glsl(&tu));

        assert_eq!(
            indoc! {"
                f0 = U_BILL.data[int(temp_4)][temp_5];
                f1 = U_POST.data[int(temp_206)];
                f2 = U_Mate.gMatCol.x;
                f3 = U_Mate.gWrkCol[1].w;
            "},
            graph.to_glsl()
        );
    }

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
                b = 2.0;
                c = a * b;
                d = fma(a, b, c);
                d = d + 1.0;
                OUT_Color.x = c + d;
            "},
            glsl_dependencies(glsl, "OUT_Color", Some('x'))
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
                b = 2.0;
                c = 2 * b;
            "},
            glsl_dependencies(glsl, "c", None)
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
                a = 0.0;
                b = uint(a) >> 2;
                c = data[int(b)];
            "},
            glsl_dependencies(glsl, "c", None)
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

        assert_eq!("", glsl_dependencies(glsl, "d", None));
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
                a = 1.0;
                a2 = a * 5.0;
                b = texture(texture1, vec2(a2 + 2.0, 1.0)).x;
                c = data[int(b)];
            "},
            glsl_dependencies(glsl, "c", None)
        );
    }

    #[test]
    fn line_dependencies_parameters() {
        let glsl = indoc! {"
            void main() {
                temp_0 = U_Static.gDitVal.w * U_Mate.gAlInf.z;
                temp_1 = floor(temp_0);
                temp_2 = temp_1 * U_Static.gDitVal.z;
                temp_3 = in_attr4.w;
                temp_4 = in_attr4.x;
                temp_5 = in_attr4.y;
                temp_6 = 1. / temp_3;
                temp_7 = temp_4 * temp_6;
                temp_8 = temp_5 * temp_6;
                temp_9 = fma(temp_7, 0.5, 0.5);
                temp_10 = fma(temp_8, -0.5, 0.5);
                temp_11 = temp_9 * U_Static.gDitVal.x;
                temp_12 = temp_10 * U_Static.gDitVal.y;
                temp_13 = floor(temp_11);
                temp_14 = floor(temp_12);
                temp_15 = 0. - temp_13;
                temp_16 = temp_11 + temp_15;
                temp_17 = 0. - temp_14;
                temp_18 = temp_12 + temp_17;
                temp_19 = fma(temp_16, U_Static.gDitVal.z, temp_2);
                temp_20 = texture(texDither, vec2(temp_19, temp_18)).x;
                temp_21 = in_attr2.x;
                temp_22 = in_attr2.y;
                temp_23 = temp_20 <= U_Mate.gAlInf.y;
                if (temp_23) {
                    discard;
                }
            }
        "};

        assert_eq!(
            indoc! {"
                temp_0 = U_Static.gDitVal.w * U_Mate.gAlInf.z;
                temp_1 = floor(temp_0);
                temp_2 = temp_1 * U_Static.gDitVal.z;
                temp_3 = in_attr4.w;
                temp_4 = in_attr4.x;
                temp_5 = in_attr4.y;
                temp_6 = 1.0 / temp_3;
                temp_7 = temp_4 * temp_6;
                temp_8 = temp_5 * temp_6;
                temp_9 = fma(temp_7, 0.5, 0.5);
                temp_10 = fma(temp_8, -0.5, 0.5);
                temp_11 = temp_9 * U_Static.gDitVal.x;
                temp_12 = temp_10 * U_Static.gDitVal.y;
                temp_13 = floor(temp_11);
                temp_14 = floor(temp_12);
                temp_15 = 0.0 - temp_13;
                temp_16 = temp_11 + temp_15;
                temp_17 = 0.0 - temp_14;
                temp_18 = temp_12 + temp_17;
                temp_19 = fma(temp_16, U_Static.gDitVal.z, temp_2);
                temp_20 = texture(texDither, vec2(temp_19, temp_18)).x;
            "},
            glsl_dependencies(glsl, "temp_20", None)
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
                    ("in_attr0".to_smolstr(), 0),
                    ("in_attr1".to_smolstr(), 4),
                    ("in_attr2".to_smolstr(), 3)
                ]
                .into_iter()
                .collect(),
                output_locations: [
                    ("out_attr0".to_smolstr(), 3),
                    ("out_attr1".to_smolstr(), 5),
                    ("out_attr2".to_smolstr(), 7)
                ]
                .into_iter()
                .collect(),
            },
            find_attribute_locations(&tu)
        );
    }
}
