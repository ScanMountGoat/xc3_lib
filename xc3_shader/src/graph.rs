use std::collections::{BTreeMap, BTreeSet};

use indexmap::IndexSet;
use ordered_float::OrderedFloat;
use smol_str::SmolStr;

#[cfg(feature = "glsl")]
pub mod glsl;
#[cfg(feature = "latte")]
pub mod latte;

#[cfg(feature = "glsl")]
pub mod query;

/// A directed graph of shader assignments and input expressions to simplify analysis.
#[derive(Debug, PartialEq, Clone, Default)]
pub struct Graph {
    pub nodes: Vec<Node>,
    /// Unique [Expr] used for the input values for each [Node].
    pub exprs: Vec<Expr>,
}

/// A single assignment statement of the form `output = operation(inputs);`.
#[derive(Debug, PartialEq, Clone)]
pub struct Node {
    pub output: Output,
    /// Index into [exprs](struct.Graph.html#structfield.exprs) value assigned in this assignment statement.
    pub input: usize,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum Expr {
    /// A value assigned in a previous node.
    Node {
        /// Index into [nodes](struct.Graph.html#structfield.nodes).
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
        index: Option<usize>,
        channel: Option<char>,
    },
    /// A global identifier like `in_attr0.x`.
    Global {
        name: SmolStr,
        channel: Option<char>,
    },
    Unary(UnaryOp, usize),
    Binary(BinaryOp, usize, usize),
    Ternary(usize, usize, usize),
    Func {
        name: SmolStr,
        args: Vec<usize>,
        channel: Option<char>,
    },
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum UnaryOp {
    Negate,
    Not,
    Complement,
    /// Reinterpret the u32 input as a float.
    UintBitsToFloat,
    /// Reinterpret the float input as an u32.
    FloatBitsToUint,
    /// Reinterpret the i32 input as a float.
    IntBitsToFloat,
    /// Reinterpret the float input as an i32.
    FloatBitsToInt,
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
        if let Some(n) = self.nodes.get(node_index) {
            if dependent_lines.insert(node_index) {
                self.add_dependencies(n.input, &mut dependent_lines);
            }
        }

        let max_depth = recursion_depth.unwrap_or(dependent_lines.len());
        dependent_lines
            .into_iter()
            .rev()
            .take(max_depth + 1)
            .rev()
            .collect()
    }

    fn add_dependencies(&self, expr_index: usize, dependent_lines: &mut BTreeSet<usize>) {
        match &self.exprs[expr_index] {
            Expr::Node { node_index, .. } => {
                if let Some(n) = self.nodes.get(*node_index) {
                    // Avoid processing the subtree rooted at a line more than once.
                    if dependent_lines.insert(*node_index) {
                        self.add_dependencies(n.input, dependent_lines)
                    }
                }
            }
            Expr::Parameter { index: Some(i), .. } => {
                self.add_dependencies(*i, dependent_lines);
            }
            Expr::Global { .. } => (),
            Expr::Unary(.., a) => {
                self.add_dependencies(*a, dependent_lines);
            }
            Expr::Binary(_, a, b) => {
                for i in [a, b] {
                    self.add_dependencies(*i, dependent_lines);
                }
            }
            Expr::Ternary(a, b, c) => {
                for i in [a, b, c] {
                    self.add_dependencies(*i, dependent_lines);
                }
            }
            Expr::Func { args, .. } => {
                for arg in args {
                    self.add_dependencies(*arg, dependent_lines);
                }
            }
            _ => (),
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
                if let Expr::Node { node_index, .. } = &self.exprs[n.input] {
                    self.add_assignments(*node_index, dependent_lines);
                }
            }
        }
    }

    /// Return the GLSL for each line from [Self::assignments_recursive].
    #[cfg(feature = "glsl")]
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

        // TODO: Simplify the entire graph to reuse calculations.

        // TODO: Remove unused exprs and reindex?
        let mut exprs = self.exprs.iter().cloned().collect();

        let input = self.simplify_expr(node.input, &mut simplified, &mut exprs);
        let nodes = vec![Node {
            output: node.output.clone(),
            input: exprs.insert_full(input).0,
        }];

        Self {
            nodes,
            exprs: exprs.into_iter().collect(),
        }
    }

    // TODO: Does this correctly rebuild a new simplified graph in all cases?
    fn simplify_expr(
        &self,
        input: usize,
        simplified: &mut BTreeMap<usize, Expr>,
        exprs: &mut IndexSet<Expr>,
    ) -> Expr {
        // Recursively simplify an expression.
        // TODO: perform other simplifications?
        if let Some(expr) = simplified.get(&input) {
            expr.clone()
        } else {
            // TODO: avoid clone
            let result = match &exprs[input].clone() {
                Expr::Node {
                    node_index,
                    channel,
                } => {
                    // Simplify assignments using variable substitution.
                    let mut expr =
                        self.simplify_expr(self.nodes[*node_index].input, simplified, exprs);
                    // TODO: Is this the right way to apply channels?
                    if expr.channel().is_none() {
                        expr.set_channel(*channel);
                    }
                    expr
                }
                Expr::Unary(UnaryOp::Negate, e) => {
                    let e = self.simplify_expr(*e, simplified, exprs);

                    if let Expr::Float(f) = e {
                        // -(f) == -f
                        Expr::Float(-f)
                    } else {
                        Expr::Unary(UnaryOp::Negate, exprs.insert_full(e).0)
                    }
                }
                Expr::Unary(op, e) => {
                    let new_e = self.simplify_expr(*e, simplified, exprs);
                    Expr::Unary(*op, exprs.insert_full(new_e).0)
                }
                Expr::Binary(BinaryOp::Sub, a, b) => {
                    let a = self.simplify_expr(*a, simplified, exprs);
                    let b = self.simplify_expr(*b, simplified, exprs);

                    // TODO: a - -b == a + b
                    if let Expr::Float(OrderedFloat(0.0)) = a {
                        // 0.0 - b == -b
                        let new_b = Expr::Unary(UnaryOp::Negate, exprs.insert_full(b).0);
                        self.simplify_expr(exprs.insert_full(new_b).0, simplified, exprs)
                    } else {
                        Expr::Binary(
                            BinaryOp::Sub,
                            exprs.insert_full(a).0,
                            exprs.insert_full(b).0,
                        )
                    }
                }
                Expr::Binary(BinaryOp::Add, a, b) => {
                    let a = self.simplify_expr(*a, simplified, exprs);
                    let b = self.simplify_expr(*b, simplified, exprs);

                    if let Expr::Float(OrderedFloat(0.0)) = a {
                        // 0.0 + b == b
                        b
                    } else if let Expr::Float(OrderedFloat(0.0)) = b {
                        // a + 0.0 == a
                        a
                    } else if let Expr::Unary(UnaryOp::Negate, a) = a {
                        // -a + b == b - a
                        Expr::Binary(BinaryOp::Sub, exprs.insert_full(b).0, a)
                    } else if let Expr::Unary(UnaryOp::Negate, b) = b {
                        // a + -b == a - b
                        Expr::Binary(BinaryOp::Sub, exprs.insert_full(a).0, b)
                    } else {
                        Expr::Binary(
                            BinaryOp::Add,
                            exprs.insert_full(a).0,
                            exprs.insert_full(b).0,
                        )
                    }
                }
                Expr::Binary(op, a, b) => {
                    let new_a = self.simplify_expr(*a, simplified, exprs);
                    let new_b = self.simplify_expr(*b, simplified, exprs);
                    Expr::Binary(*op, exprs.insert_full(new_a).0, exprs.insert_full(new_b).0)
                }
                Expr::Ternary(a, b, c) => {
                    let new_a = self.simplify_expr(*a, simplified, exprs);
                    let new_b = self.simplify_expr(*b, simplified, exprs);
                    let new_c = self.simplify_expr(*c, simplified, exprs);
                    Expr::Ternary(
                        exprs.insert_full(new_a).0,
                        exprs.insert_full(new_b).0,
                        exprs.insert_full(new_c).0,
                    )
                }
                Expr::Func {
                    name,
                    args,
                    channel,
                } => Expr::Func {
                    name: name.clone(),
                    args: args
                        .iter()
                        .map(|arg| {
                            let new_arg = self.simplify_expr(*arg, simplified, exprs);
                            exprs.insert_full(new_arg).0
                        })
                        .collect(),
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
                    index: index.map(|i| {
                        let new_index = self.simplify_expr(i, simplified, exprs);
                        exprs.insert_full(new_index).0
                    }),
                    channel: *channel,
                },
                i => i.clone(),
            };
            simplified.insert(input, result.clone());
            result
        }
    }
}

impl Expr {
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
