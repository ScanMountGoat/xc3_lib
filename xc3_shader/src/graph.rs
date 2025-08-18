use std::collections::{BTreeMap, BTreeSet};

use ordered_float::OrderedFloat;
use smol_str::SmolStr;

pub mod glsl;
#[cfg(feature = "xc3")]
pub mod latte;
pub mod query;

/// A directed graph of shader assignments and input expressions to simplify analysis.
#[derive(Debug, PartialEq, Clone, Default)]
pub struct Graph {
    pub nodes: Vec<Node>,
}

/// A single assignment statement of the form `output = operation(inputs);`.
#[derive(Debug, PartialEq, Clone)]
pub struct Node {
    pub output: Output,
    /// The value assigned in this assignment statement.
    pub input: Expr,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum Expr {
    /// A value assigned in a previous node.
    Node {
        node_index: usize,
        channel: Option<char>,
    },
    /// A float constant like `1.0`.
    Float(OrderedFloat<f32>),
    /// An integer constant like `-1`.
    Int(i32),
    /// An unsigned integer constant like `1`.
    Uint(u32),
    /// An boolean constant like `true`.
    Bool(bool),
    /// A parameter access like `name.field[index].x`, `name[index].x`, or `name.field.x`.
    Parameter {
        name: SmolStr,
        field: Option<SmolStr>,
        index: Option<Box<Expr>>,
        channel: Option<char>,
    },
    /// A global identifier like `in_attr0.x`.
    Global {
        name: SmolStr,
        channel: Option<char>,
    },
    Unary(UnaryOp, Box<Expr>),
    Binary(BinaryOp, Box<Expr>, Box<Expr>),
    Ternary(Box<Expr>, Box<Expr>, Box<Expr>),
    Func {
        name: SmolStr,
        args: Vec<Expr>,
        channel: Option<char>,
    },
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum UnaryOp {
    Negate,
    Not,
    Complement,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    LeftShift,
    RightShift,
    BitOr,
    BitXor,
    BitAnd,
    Equal,
    NotEqual,
    Less,
    Greater,
    LessEqual,
    GreaterEqual,
    Or,
    And,
}

#[derive(Debug, PartialEq, Clone, Eq, PartialOrd, Ord)]
pub struct Output {
    /// The name of the output variable like `out` in `out.x = in`
    pub name: SmolStr,
    // TODO: use a char index instead?
    /// The channel to assign to like `x` in `out.x = in`.
    /// Multiple channel assignments need to be split into multiple scalar assignments.
    pub channel: Option<char>,
}

impl Graph {
    /// Return the indices of dependent nodes for `variable` and `channels`
    /// starting from the last assignment.
    pub fn dependencies_recursive(
        &self,
        variable: &str,
        channel: Option<char>,
        recursion_depth: Option<usize>,
    ) -> Vec<usize> {
        // Find the most recent assignment for the output variable.
        if let Some(i) = self
            .nodes
            .iter()
            .rposition(|n| n.output.name == variable && n.output.channel == channel)
        {
            self.node_dependencies_recursive(i, recursion_depth)
        } else {
            Vec::new()
        }
    }

    /// Return the indices of dependent nodes for `node`
    /// starting from the last assignment.
    pub fn node_dependencies_recursive(
        &self,
        node_index: usize,
        recursion_depth: Option<usize>,
    ) -> Vec<usize> {
        let mut dependent_lines = BTreeSet::new();

        // Follow data dependencies backwards to find all relevant lines.
        self.add_dependencies(node_index, &mut dependent_lines);

        let max_depth = recursion_depth.unwrap_or(dependent_lines.len());
        dependent_lines
            .into_iter()
            .rev()
            .take(max_depth + 1)
            .rev()
            .collect()
    }

    fn add_dependencies(&self, node_index: usize, dependent_lines: &mut BTreeSet<usize>) {
        if let Some(n) = self.nodes.get(node_index) {
            // Avoid processing the subtree rooted at a line more than once.
            if dependent_lines.insert(node_index) {
                n.input.visit_exprs(&mut |e| {
                    if let Expr::Node { node_index, .. } = e {
                        self.add_dependencies(*node_index, dependent_lines);
                    }
                });
            }
        }
    }

    /// Return the indices of dependent nodes for `node`
    /// starting from the last assignment.
    ///
    /// Unlike [Self::dependencies_recursive],
    /// this only considers direct assignment chains like
    /// `a = b; c = a;` and does not recurse into operands or arguments.
    pub fn assignments_recursive(
        &self,
        variable: &str,
        channel: Option<char>,
        recursion_depth: Option<usize>,
    ) -> Vec<usize> {
        // Find the most recent assignment for the output variable.
        if let Some(i) = self
            .nodes
            .iter()
            .rposition(|n| n.output.name == variable && n.output.channel == channel)
        {
            self.node_assignments_recursive(i, recursion_depth)
        } else {
            Vec::new()
        }
    }

    /// Return the indices of dependent nodes for `node`
    /// starting from the last assignment.
    ///
    /// Unlike [Self::node_dependencies_recursive],
    /// this only considers direct assignment chains like
    /// `a = b; c = a;` and does not recurse into operands or arguments.
    pub fn node_assignments_recursive(
        &self,
        node_index: usize,
        recursion_depth: Option<usize>,
    ) -> Vec<usize> {
        let mut dependent_lines = BTreeSet::new();

        // Follow data dependencies backwards to find all relevant lines.
        self.add_assignments(node_index, &mut dependent_lines);

        let max_depth = recursion_depth.unwrap_or(dependent_lines.len());
        dependent_lines
            .into_iter()
            .rev()
            .take(max_depth + 1)
            .rev()
            .collect()
    }

    fn add_assignments(&self, node_index: usize, dependent_lines: &mut BTreeSet<usize>) {
        if let Some(n) = self.nodes.get(node_index) {
            // Avoid processing the subtree rooted at a line more than once.
            if dependent_lines.insert(node_index) {
                if let Expr::Node { node_index, .. } = n.input {
                    self.add_assignments(node_index, dependent_lines);
                }
            }
        }
    }

    /// Return the GLSL for each line from [Self::assignments_recursive].
    pub fn glsl_dependencies(
        &self,
        variable: &str,
        channel: Option<char>,
        recursion_depth: Option<usize>,
    ) -> String {
        let mut output = String::new();
        let mut visited = BTreeSet::new();
        for i in self.dependencies_recursive(variable, channel, recursion_depth) {
            // Some nodes may be repeated with different tracked channels.
            if visited.insert(i) {
                output += &self.node_to_glsl(&self.nodes[i]);
            }
        }
        output
    }

    /// Simplify the `node` using variable substitution to eliminate assignments
    /// and other algebraic identities.
    pub fn simplify(&self, node: &Node) -> Self {
        let mut simplified = BTreeMap::new();

        let nodes = vec![Node {
            output: node.output.clone(),
            input: simplify(&node.input, &self.nodes, &mut simplified),
        }];

        Self { nodes }
    }
}

fn simplify(input: &Expr, nodes: &[Node], simplified: &mut BTreeMap<usize, Expr>) -> Expr {
    // Recursively simplify an expression.
    // TODO: perform other simplifications?
    match input {
        Expr::Node {
            node_index,
            channel,
        } => {
            // Simplify assignments using variable substitution.
            if let Some(expr) = simplified.get(node_index) {
                expr.clone()
            } else {
                let mut expr = simplify(&nodes[*node_index].input, nodes, simplified);
                // TODO: Is this the right way to apply channels?
                if expr.channel().is_none() {
                    expr.set_channel(*channel);
                }
                simplified.insert(*node_index, expr.clone());
                expr
            }
        }
        Expr::Unary(UnaryOp::Negate, e) => {
            let e = simplify(e, nodes, simplified);

            if let Expr::Float(f) = e {
                // -(f) == -f
                Expr::Float(-f)
            } else {
                Expr::Unary(UnaryOp::Negate, Box::new(e))
            }
        }
        Expr::Unary(op, e) => Expr::Unary(*op, Box::new(simplify(e, nodes, simplified))),
        Expr::Binary(BinaryOp::Sub, a, b) => {
            let a = simplify(a, nodes, simplified);
            let b = simplify(b, nodes, simplified);

            // TODO: a - -b == a + b
            if let Expr::Float(OrderedFloat(0.0)) = a {
                // 0.0 - b == -b
                simplify(
                    &Expr::Unary(UnaryOp::Negate, Box::new(b)),
                    nodes,
                    simplified,
                )
            } else {
                Expr::Binary(BinaryOp::Sub, Box::new(a), Box::new(b))
            }
        }
        Expr::Binary(BinaryOp::Add, a, b) => {
            let a = simplify(a, nodes, simplified);
            let b = simplify(b, nodes, simplified);

            if let Expr::Float(OrderedFloat(0.0)) = a {
                // 0.0 + b == b
                b
            } else if let Expr::Float(OrderedFloat(0.0)) = b {
                // a + 0.0 == a
                a
            } else if let Expr::Unary(UnaryOp::Negate, a) = a {
                // -a + b == b - a
                Expr::Binary(BinaryOp::Sub, Box::new(b), a)
            } else if let Expr::Unary(UnaryOp::Negate, b) = b {
                // a + -b == a - b
                Expr::Binary(BinaryOp::Sub, Box::new(a), b)
            } else {
                Expr::Binary(BinaryOp::Add, Box::new(a), Box::new(b))
            }
        }
        Expr::Binary(op, a, b) => Expr::Binary(
            *op,
            Box::new(simplify(a, nodes, simplified)),
            Box::new(simplify(b, nodes, simplified)),
        ),
        Expr::Ternary(a, b, c) => Expr::Ternary(
            Box::new(simplify(a, nodes, simplified)),
            Box::new(simplify(b, nodes, simplified)),
            Box::new(simplify(c, nodes, simplified)),
        ),
        Expr::Func {
            name,
            args,
            channel,
        } => Expr::Func {
            name: name.clone(),
            args: args
                .iter()
                .map(|a| simplify(a, nodes, simplified))
                .collect(),
            channel: *channel,
        },
        i => i.clone(),
    }
}

impl Expr {
    /// Apply `visit` to the current expression recursively.
    pub fn visit_exprs<F: FnMut(&Expr)>(&self, visit: &mut F) {
        visit(self);
        match self {
            Expr::Node { .. } => (),
            Expr::Float(_) => (),
            Expr::Int(_) => (),
            Expr::Uint(_) => (),
            Expr::Bool(_) => (),
            Expr::Parameter { index, .. } => {
                if let Some(index) = index {
                    index.visit_exprs(visit);
                }
            }
            Expr::Global { .. } => (),
            Expr::Unary(_, a) => {
                a.visit_exprs(visit);
            }
            Expr::Binary(_, lh, rh) => {
                lh.visit_exprs(visit);
                rh.visit_exprs(visit);
            }
            Expr::Ternary(a, b, c) => {
                a.visit_exprs(visit);
                b.visit_exprs(visit);
                c.visit_exprs(visit);
            }
            Expr::Func { args, .. } => {
                for arg in args {
                    arg.visit_exprs(visit);
                }
            }
        }
    }

    /// Apply `visit` to the current expression recursively.
    pub fn visit_exprs_mut<F: FnMut(&mut Expr)>(&mut self, visit: &mut F) {
        visit(self);
        match self {
            Expr::Node { .. } => (),
            Expr::Float(_) => (),
            Expr::Int(_) => (),
            Expr::Uint(_) => (),
            Expr::Bool(_) => (),
            Expr::Parameter { index, .. } => {
                if let Some(index) = index {
                    index.visit_exprs_mut(visit);
                }
            }
            Expr::Global { .. } => (),
            Expr::Unary(_, a) => {
                a.visit_exprs_mut(visit);
            }
            Expr::Binary(_, lh, rh) => {
                lh.visit_exprs_mut(visit);
                rh.visit_exprs_mut(visit);
            }
            Expr::Ternary(a, b, c) => {
                a.visit_exprs_mut(visit);
                b.visit_exprs_mut(visit);
                c.visit_exprs_mut(visit);
            }
            Expr::Func { args, .. } => {
                for arg in args {
                    arg.visit_exprs_mut(visit);
                }
            }
        }
    }

    pub fn channel(&self) -> Option<char> {
        match self {
            Expr::Node { channel, .. } => *channel,
            Expr::Float(_) => None,
            Expr::Int(_) => None,
            Expr::Uint(_) => None,
            Expr::Bool(_) => None,
            Expr::Parameter { channel, .. } => *channel,
            Expr::Global { channel, .. } => *channel,
            Expr::Unary(_, _) => None,
            Expr::Binary(_, _, _) => None,
            Expr::Ternary(_, _, _) => None,
            Expr::Func { channel, .. } => *channel,
        }
    }

    pub fn set_channel(&mut self, c: Option<char>) {
        match self {
            Expr::Node { channel, .. } => *channel = c,
            Expr::Parameter { channel, .. } => *channel = c,
            Expr::Global { channel, .. } => *channel = c,
            Expr::Func { channel, .. } => *channel = c,
            _ => (),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use indoc::indoc;
    use pretty_assertions::assert_eq;

    #[test]
    fn simplify_statements() {
        let glsl = indoc! {"
            void main() {       
                color = texture(s0, vec2(0.0, 0.5));
                color2 = color;
                glossiness = color2.x;
                result = 0.0 - glossiness;
                result = 1.0 + result;
                result = clamp(result, 0.0, 1.0);
                result = sqrt(result);
                result = 0.0 - result;
                result = result + 1.0;
                result = result;
            }
        "};
        let graph = Graph::parse_glsl(glsl).unwrap();

        assert_eq!(
            "result = 1.0 - sqrt(clamp(1.0 - texture(s0, vec2(0.0, 0.5)).x, 0.0, 1.0));\n",
            graph.simplify(graph.nodes.last().unwrap()).to_glsl()
        );
    }
}
