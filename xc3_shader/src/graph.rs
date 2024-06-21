use std::collections::BTreeSet;

mod glsl;

/// A directed graph of shader assignments and operations.
/// This normalizes identifiers and preserves only the data flow of the code.
/// Two graphs that perform the same operations will be isomorphic even if
/// the variable names change or unrelated code lines are inserted between statements.
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
    LShift(Box<Expr>, Box<Expr>),
    RShift(Box<Expr>, Box<Expr>),
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
            // TODO: This isn't right either?
            let should_pass_channels = !matches!(
                n.input,
                Expr::Func { .. } | Expr::Parameter { .. } | Expr::Global { .. }
            );
            // dbg!(node_index, should_pass_channels);

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
        for (i, _) in self.assignments_recursive(variable, channels, recursion_depth) {
            output += &self.node_to_glsl(&self.nodes[i]);
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
            Expr::Float(_) => None,
            Expr::Int(_) => None,
            Expr::Parameter { channels, .. } => Some(channels),
            Expr::Global { channels, .. } => Some(channels),
            Expr::Add(_, _) => None,
            Expr::Sub(_, _) => None,
            Expr::Mul(_, _) => None,
            Expr::Div(_, _) => None,
            Expr::LShift(_, _) => None,
            Expr::RShift(_, _) => None,
            Expr::Func { channels, .. } => Some(channels),
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
        Expr::LShift(lh, rh) => {
            add_exprs(exprs, lh);
            add_exprs(exprs, rh);
        }
        Expr::RShift(lh, rh) => {
            add_exprs(exprs, lh);
            add_exprs(exprs, rh);
        }
        Expr::Func { args, .. } => {
            for arg in args {
                add_exprs(exprs, arg);
            }
        }
    }
}

// TODO: Test cases for this.
fn reduce_channels(inner: &str, outer: &str) -> String {
    if inner.is_empty() {
        // Reduce ".xyz" -> "xyz".
        outer.to_string()
    } else if outer.is_empty() {
        // Reduce "xyz." -> "xyz".
        inner.to_string()
    } else {
        // TODO: handle errors
        // Reduce "xyz.zyx" -> "zyx".
        let channel_index = |c2: char| {
            ['x', 'y', 'z', 'w']
                .iter()
                .position(|c1| *c1 == c2)
                .unwrap()
        };
        outer
            .chars()
            .map(|c| inner.chars().nth(channel_index(c)).unwrap())
            .collect()
    }
}
