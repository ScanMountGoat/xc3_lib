use std::collections::BTreeSet;

mod glsl;
mod latte;
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

// TODO: SmolStr?
#[derive(Debug, PartialEq, Clone)]
pub enum Expr {
    /// A value assigned in a previous node.
    Node {
        node_index: usize,
        channel: Option<char>,
    },
    /// A float constant like `1.0`.
    Float(f32),
    /// An integer constant like `-1`.
    Int(i32),
    /// An unsigned integer constant like `1`.
    Uint(u32),
    /// An boolean constant like `true`.
    Bool(bool),
    /// A buffer access like `name.field[index].x` or `name[index].x`.
    Parameter {
        name: String,
        field: Option<String>,
        index: Box<Expr>,
        channel: Option<char>,
    },
    /// A global identifier like `in_attr0.x`.
    Global {
        name: String,
        channel: Option<char>,
    },
    Add(Box<Expr>, Box<Expr>),
    Sub(Box<Expr>, Box<Expr>),
    Mul(Box<Expr>, Box<Expr>),
    Div(Box<Expr>, Box<Expr>),
    LeftShift(Box<Expr>, Box<Expr>),
    RightShift(Box<Expr>, Box<Expr>),
    BitOr(Box<Expr>, Box<Expr>),
    BitXor(Box<Expr>, Box<Expr>),
    BitAnd(Box<Expr>, Box<Expr>),
    Equal(Box<Expr>, Box<Expr>),
    NotEqual(Box<Expr>, Box<Expr>),
    Less(Box<Expr>, Box<Expr>),
    Greater(Box<Expr>, Box<Expr>),
    LessEqual(Box<Expr>, Box<Expr>),
    GreaterEqual(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
    And(Box<Expr>, Box<Expr>),
    Negate(Box<Expr>),
    Not(Box<Expr>),
    Complement(Box<Expr>),
    Ternary(Box<Expr>, Box<Expr>, Box<Expr>),
    Func {
        name: String,
        args: Vec<Expr>,
        channel: Option<char>,
    },
}

#[derive(Debug, PartialEq, Clone, Eq, PartialOrd, Ord)]
pub struct Output {
    /// The name of the output variable like `out` in `out.x = in`
    pub name: String,
    // TODO: use a char index instead?
    /// The channel to assign to like `x` in `out.x = in`.
    /// Multiple channel assignments need to be split into multiple scalar assignments.
    pub channel: Option<char>,
}

// TODO: more strongly typed channel swizzles?
// TODO: use this instead of line dependencies

impl Graph {
    /// Return the indices of dependent nodes for `variable` and `channels`
    /// starting from the last assignment.
    pub fn dependencies_recursive(
        &self,
        variable: &str,
        channel: Option<char>,
        recursion_depth: Option<usize>,
    ) -> Vec<usize> {
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

        // TODO: return type for accumulated channels.
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
                for e in n.input.exprs_recursive() {
                    if let Expr::Node { node_index, .. } = e {
                        self.add_dependencies(*node_index, dependent_lines);
                    }
                }
            }
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

        // TODO: return type for accumulated channels.
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
}

// TODO: Turn this into an iterator or visitor that doesn't allocate?
impl Expr {
    /// Flatten all expressions recursively.
    pub fn exprs_recursive(&self) -> Vec<&Expr> {
        let mut exprs = Vec::new();
        add_exprs(&mut exprs, self);
        exprs
    }
}

fn add_exprs<'a>(exprs: &mut Vec<&'a Expr>, input: &'a Expr) {
    // Recursively collect exprs.
    exprs.push(input);
    match input {
        Expr::Node { .. } => (),
        Expr::Float(_) => (),
        Expr::Int(_) => (),
        Expr::Uint(_) => (),
        Expr::Bool(_) => (),
        Expr::Parameter { index, .. } => {
            add_exprs(exprs, index);
        }
        Expr::Global { .. } => (),
        Expr::Add(lh, rh) => {
            add_exprs(exprs, lh);
            add_exprs(exprs, rh);
        }
        Expr::Sub(lh, rh) => {
            add_exprs(exprs, lh);
            add_exprs(exprs, rh);
        }
        Expr::Mul(lh, rh) => {
            add_exprs(exprs, lh);
            add_exprs(exprs, rh);
        }
        Expr::Div(lh, rh) => {
            add_exprs(exprs, lh);
            add_exprs(exprs, rh);
        }
        Expr::LeftShift(lh, rh) => {
            add_exprs(exprs, lh);
            add_exprs(exprs, rh);
        }
        Expr::RightShift(lh, rh) => {
            add_exprs(exprs, lh);
            add_exprs(exprs, rh);
        }
        Expr::BitOr(lh, rh) => {
            add_exprs(exprs, lh);
            add_exprs(exprs, rh);
        }
        Expr::BitXor(lh, rh) => {
            add_exprs(exprs, lh);
            add_exprs(exprs, rh);
        }
        Expr::BitAnd(lh, rh) => {
            add_exprs(exprs, lh);
            add_exprs(exprs, rh);
        }
        Expr::Equal(lh, rh) => {
            add_exprs(exprs, lh);
            add_exprs(exprs, rh);
        }
        Expr::NotEqual(lh, rh) => {
            add_exprs(exprs, lh);
            add_exprs(exprs, rh);
        }
        Expr::Less(lh, rh) => {
            add_exprs(exprs, lh);
            add_exprs(exprs, rh);
        }
        Expr::Greater(lh, rh) => {
            add_exprs(exprs, lh);
            add_exprs(exprs, rh);
        }
        Expr::LessEqual(lh, rh) => {
            add_exprs(exprs, lh);
            add_exprs(exprs, rh);
        }
        Expr::GreaterEqual(lh, rh) => {
            add_exprs(exprs, lh);
            add_exprs(exprs, rh);
        }
        Expr::Or(lh, rh) => {
            add_exprs(exprs, lh);
            add_exprs(exprs, rh);
        }
        Expr::And(lh, rh) => {
            add_exprs(exprs, lh);
            add_exprs(exprs, rh);
        }
        Expr::Ternary(a, b, c) => {
            add_exprs(exprs, a);
            add_exprs(exprs, b);
            add_exprs(exprs, c);
        }
        Expr::Negate(a) => {
            add_exprs(exprs, a);
        }
        Expr::Not(a) => {
            add_exprs(exprs, a);
        }
        Expr::Complement(a) => {
            add_exprs(exprs, a);
        }
        Expr::Func { args, .. } => {
            for arg in args {
                add_exprs(exprs, arg);
            }
        }
    }
}
