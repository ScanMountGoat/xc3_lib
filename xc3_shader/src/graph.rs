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
    ) -> Vec<usize> {
        let mut dependent_lines = BTreeSet::new();
        if let Some((i, n)) = self
            .nodes
            .iter()
            .enumerate()
            .rfind(|(_, n)| n.output.name == variable && n.output.channels == channels)
        {
            dependent_lines.insert(i);

            // Follow data dependencies backwards to find all relevant lines.
            // add_dependencies(&mut dependent_lines, &n.input, &self.nodes);
            self.add_dependencies(n, &mut dependent_lines);
        }

        let max_depth = recursion_depth.unwrap_or(dependent_lines.len());
        dependent_lines
            .into_iter()
            .rev()
            .take(max_depth + 1)
            .rev()
            .collect()
    }

    fn add_dependencies(&self, n: &Node, dependent_lines: &mut BTreeSet<usize>) {
        for e in n.input.exprs_recursive() {
            if let Expr::Node { node_index, .. } = e {
                // Avoid processing the subtree rooted at a line more than once.
                if dependent_lines.insert(*node_index) {
                    self.add_dependencies(&self.nodes[*node_index], dependent_lines);
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
        for i in self.assignments_recursive(variable, channels, recursion_depth) {
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
