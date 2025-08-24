use ordered_float::OrderedFloat;
use smol_str::SmolStr;

use crate::graph::{Expr, Node};

pub struct ProgramOutputs<Op> {
    pub outputs: Vec<usize>,
    pub exprs: Vec<OutputExpr<Op>>,
}

/// A tree of computations with [Value] for the leaf values.
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum OutputExpr<Op> {
    /// A single value.
    Value(Value),
    /// An operation applied to one or more [OutputExpr].
    Func {
        /// The operation this function performs.
        op: Op,
        /// Indices into [exprs](struct.ProgramOutputs.html#structfield.exprs) for the function argument list `[arg0, arg1, ...]`.
        args: Vec<usize>,
    },
}

/// A single access to a constant or global resource like a texture.
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum Value {
    Constant(OrderedFloat<f32>),
    Parameter(Parameter),
    Texture(Texture),
    Attribute(Attribute),
}

/// A single buffer access like `UniformBuffer.field[0].y` or `UniformBuffer.field.y` in GLSL.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
pub struct Parameter {
    pub name: SmolStr,
    pub field: SmolStr,
    pub index: Option<usize>,
    pub channel: Option<char>,
}

/// A single texture access like `texture(s0, tex0.xy).rgb` in GLSL.
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct Texture {
    pub name: SmolStr,
    pub channel: Option<char>,
    /// Indices into [exprs](struct.ProgramOutputs.html#structfield.exprs)
    /// for texture coordinate values used for the texture function call.
    pub texcoords: Vec<usize>,
}

/// A single input attribute like `in_attr0.x` in GLSL.
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct Attribute {
    pub name: SmolStr,
    pub channel: Option<char>,
}

pub trait Operation: Sized {
    fn query_operation_args<'a>(nodes: &'a [Node], expr: &'a Expr)
        -> Option<(Self, Vec<&'a Expr>)>;
}

// TODO: generic function for converting graph to OutputExpr if Op: Operation
