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
    /// The operation performed on the inputs or `None` if assigned directly.
    pub operation: Option<Operation>,
    /// References to input values used in this assignment statement.
    pub inputs: Vec<Input>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Input {
    Node {
        node_index: usize,
        channels: String,
    },
    Constant(f32),
    Parameter {
        name: String,
        field: Option<String>,
        index: usize,
        channels: String,
    },
    Global {
        name: String,
        channels: String,
    },
}

#[derive(Debug, PartialEq, Clone)]
pub struct Output {
    name: String,
    channels: String,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Operation {
    Add,
    Sub,
    Mul,
    Div,
    Func(String),
}

// TODO: more strongly typed channel swizzles?
// TODO: use this instead of line dependencies
