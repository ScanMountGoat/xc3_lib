use std::collections::BTreeSet;

mod glsl;
mod latte;
pub mod query;

/// A directed graph of shader assignments and input expressions to simplify analysis.
#[derive(Debug, PartialEq, Clone)]
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
        channels: String,
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
        channels: String,
    },
    /// A global identifier like `in_attr0.x`.
    Global {
        name: String,
        channels: String,
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
        channels: String,
    },
}

#[derive(Debug, PartialEq, Clone, Eq, PartialOrd, Ord)]
pub struct Output {
    pub name: String,
    pub channels: String,
}

// TODO: more strongly typed channel swizzles?
// TODO: use this instead of line dependencies

impl Graph {
    /// Return the indices of dependent nodes for `variable` and `channels`
    /// starting from the last assignment.
    pub fn assignments_recursive(
        &self,
        variable: &str,
        channels: &str,
        recursion_depth: Option<usize>,
    ) -> Vec<(usize, String)> {
        if let Some(i) = self
            .nodes
            .iter()
            .rposition(|n| n.output.name == variable && n.output.channels == channels)
        {
            self.node_assignments_recursive(i, recursion_depth)
        } else {
            Vec::new()
        }
    }

    // TODO: can this also track channels?
    /// Return the indices of dependent nodes for `node`
    /// starting from the last assignment.
    pub fn node_assignments_recursive(
        &self,
        node_index: usize,
        recursion_depth: Option<usize>,
    ) -> Vec<(usize, String)> {
        let mut dependent_lines = BTreeSet::new();

        // Follow data dependencies backwards to find all relevant lines.
        self.add_dependencies(node_index, &mut dependent_lines, String::new());

        // TODO: return type for accumulated channels.
        let max_depth = recursion_depth.unwrap_or(dependent_lines.len());
        dependent_lines
            .into_iter()
            .rev()
            .take(max_depth + 1)
            .rev()
            .collect()
    }

    // TODO: only Node references impact channels recursively?
    // TODO: function calls or parameters don't impact channels of arguments?
    fn add_dependencies(
        &self,
        node_index: usize,
        dependent_lines: &mut BTreeSet<(usize, String)>,
        previous_channels: String,
    ) {
        if let Some(n) = self.nodes.get(node_index) {
            // The final channels should always include all node channels.
            let channels =
                reduce_channels(n.input.channels().unwrap_or_default(), &previous_channels);

            // Channels don't apply to function arguments or buffer indices.
            let should_pass_channels = !matches!(
                n.input,
                Expr::Func { .. } | Expr::Parameter { .. } | Expr::Global { .. }
            );

            // Avoid processing the subtree rooted at a line more than once.
            if dependent_lines.insert((node_index, channels)) {
                for e in n.input.exprs_recursive() {
                    if let Expr::Node {
                        node_index,
                        channels,
                    } = e
                    {
                        let new_channels = if should_pass_channels {
                            reduce_channels(channels, &previous_channels)
                        } else {
                            String::new()
                        };
                        self.add_dependencies(*node_index, dependent_lines, new_channels);
                    }
                }
            }
        }
    }

    /// Return the GLSL for each line from [Self::assignments_recursive].
    pub fn glsl_dependencies(
        &self,
        variable: &str,
        channels: &str,
        recursion_depth: Option<usize>,
    ) -> String {
        let mut output = String::new();
        let mut visited = BTreeSet::new();
        for (i, _) in self.assignments_recursive(variable, channels, recursion_depth) {
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

    pub fn channels(&self) -> Option<&str> {
        match self {
            Expr::Node { channels, .. } => Some(channels),
            Expr::Parameter { channels, .. } => Some(channels),
            Expr::Global { channels, .. } => Some(channels),
            Expr::Func { channels, .. } => Some(channels),
            _ => None,
        }
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

// TODO: Test cases for this.
pub fn reduce_channels(inner: &str, outer: &str) -> String {
    if inner.is_empty() {
        // Reduce ".xyz" -> "xyz".
        outer.to_string()
    } else if outer.is_empty() {
        // Reduce "xyz." -> "xyz".
        inner.to_string()
    } else if inner == outer {
        // TODO: Why is this case happening?
        inner.to_string()
    } else {
        // TODO: handle errors
        // Reduce "xyz.zyx" -> "zyx".
        let channel_index = |c: char| "xyzw".find(c).unwrap();
        // TODO: handle errors
        outer
            .chars()
            .map(|c| {
                inner
                    .chars()
                    .nth(channel_index(c))
                    .unwrap_or_else(|| panic!("{inner}.{outer}"))
            })
            .collect()
    }
}
