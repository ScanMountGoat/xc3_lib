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
            add_dependencies(&mut dependent_lines, &n.input, &self.nodes);
        }

        let max_depth = recursion_depth.unwrap_or(dependent_lines.len());
        dependent_lines
            .into_iter()
            .rev()
            .take(max_depth + 1)
            .rev()
            .collect()
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

fn add_dependencies(dependencies: &mut BTreeSet<usize>, input: &Expr, nodes: &[Node]) {
    // Recursively collect nodes that the given node depends on.
    match input {
        Expr::Node {
            node_index,
            channels,
        } => {
            // Avoid processing the subtree rooted at a line more than once.
            if dependencies.insert(*node_index) {
                add_dependencies(dependencies, &nodes[*node_index].input, nodes);
            }
        }
        Expr::Float(_) => (),
        Expr::Int(_) => (),
        Expr::Parameter {
            name,
            field,
            index,
            channels,
        } => {
            add_dependencies(dependencies, &index, nodes);
        }
        Expr::Global { name, channels } => (),
        Expr::Add(lh, rh) => {
            add_dependencies(dependencies, lh, nodes);
            add_dependencies(dependencies, rh, nodes);
        }
        Expr::Sub(lh, rh) => {
            add_dependencies(dependencies, lh, nodes);
            add_dependencies(dependencies, rh, nodes);
        }
        Expr::Mul(lh, rh) => {
            add_dependencies(dependencies, lh, nodes);
            add_dependencies(dependencies, rh, nodes);
        }
        Expr::Div(lh, rh) => {
            add_dependencies(dependencies, lh, nodes);
            add_dependencies(dependencies, rh, nodes);
        }
        Expr::LShift(lh, rh) => {
            add_dependencies(dependencies, lh, nodes);
            add_dependencies(dependencies, rh, nodes);
        }
        Expr::RShift(lh, rh) => {
            add_dependencies(dependencies, lh, nodes);
            add_dependencies(dependencies, rh, nodes);
        }
        Expr::Func { args, .. } => {
            for arg in args {
                add_dependencies(dependencies, arg, nodes);
            }
        }
    }
}
