use std::borrow::Cow;

use ordered_float::OrderedFloat;
use smol_str::SmolStr;

use crate::{
    dependencies::{buffer_dependency, texture_dependency},
    graph::{Expr, Graph, UnaryOp},
};

// Faster than the default hash implementation.
type IndexSet<T> = indexmap::IndexSet<T, ahash::RandomState>;
type IndexMap<K, V> = indexmap::IndexMap<K, V, ahash::RandomState>;

// TODO: Use this for all outputs?
pub struct ProgramOutputs<Op> {
    pub outputs: Vec<usize>,
    pub exprs: Vec<OutputExpr<Op>>,
}

/// An expression tree with [Value] for the leaf nodes.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum Value {
    Int(i32),
    Float(OrderedFloat<f32>),
    Parameter(Parameter),
    Texture(Texture),
    Attribute(Attribute),
}

/// A single buffer access like `UniformBuffer.field[0].y` or `UniformBuffer.field.y` in GLSL.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
pub struct Parameter {
    pub name: SmolStr,
    pub field: SmolStr,
    pub index: Option<usize>,
    pub channel: Option<char>,
}

/// A single texture access like `texture(s0, tex0.xy).rgb` in GLSL.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct Texture {
    pub name: SmolStr,
    pub channel: Option<char>,
    /// Indices into [exprs](struct.ProgramOutputs.html#structfield.exprs)
    /// for texture coordinate values used for the texture function call.
    pub texcoords: Vec<usize>,
}

/// A single input attribute like `in_attr0.x` in GLSL.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct Attribute {
    pub name: SmolStr,
    pub channel: Option<char>,
}

/// A set of operations like `fma` or matrix multiplication that can be detected from a [Graph].
pub trait Operation: Sized {
    /// Detect operations and their arguments from most specific to least specific.
    fn query_operation_args<'a>(graph: &'a Graph, expr: &'a Expr) -> Option<(Self, Vec<&'a Expr>)>;

    /// Potentially modify the expr before detecting [OutputExpr::Func] or [OutputExpr::Value].
    fn preprocess_expr<'a>(graph: &'a Graph, expr: &'a Expr) -> Cow<'a, Expr>;

    /// Potentially modify the expr before detecting [OutputExpr::Value].
    fn preprocess_value_expr<'a>(graph: &'a Graph, expr: &'a Expr) -> Cow<'a, Expr>;
}

/// Convert `graph` to an expression tree using the [Operation] implementation for `Op`.
pub fn output_expr<Op>(
    expr: &Expr,
    graph: &Graph,
    exprs: &mut IndexSet<OutputExpr<Op>>,
    expr_to_index: &mut IndexMap<Expr, usize>,
) -> usize
where
    Op: Operation + std::hash::Hash + Eq + Default,
{
    // Cache graph input expressions to avoid processing nodes more than once while recursing.
    match expr_to_index.get(expr) {
        Some(i) => *i,
        None => {
            let original_expr = expr.clone();

            let expr = Op::preprocess_expr(graph, expr);
            let output = output_expr_inner(&expr, graph, exprs, expr_to_index);

            let index = exprs.insert_full(output).0;
            expr_to_index.insert(original_expr, index);

            index
        }
    }
}

fn output_expr_inner<Op>(
    expr: &Expr,
    graph: &Graph,
    exprs: &mut IndexSet<OutputExpr<Op>>,
    expr_to_index: &mut IndexMap<Expr, usize>,
) -> OutputExpr<Op>
where
    Op: Operation + std::hash::Hash + Eq + Default,
{
    if let Some(value) = extract_value(expr, graph, exprs, expr_to_index) {
        // The base case is a single value.
        OutputExpr::Value(value)
    } else {
        // Detect operations from most specific to least specific.
        // This results in fewer operations in many cases.
        if let Some((op, args)) = Op::query_operation_args(graph, expr) {
            // Insert values that this operation depends on first.
            let args: Vec<_> = args
                .into_iter()
                .map(|arg| output_expr(arg, graph, exprs, expr_to_index))
                .collect();
            OutputExpr::Func { op, args }
        } else {
            // TODO: log unsupported expr?
            OutputExpr::Func {
                op: Op::default(),
                args: Vec::new(),
            }
        }
    }
}

fn extract_value<Op>(
    expr: &Expr,
    graph: &Graph,
    exprs: &mut IndexSet<OutputExpr<Op>>,
    expr_to_index: &mut IndexMap<Expr, usize>,
) -> Option<crate::expr::Value>
where
    Op: Operation + std::hash::Hash + Eq + Default,
{
    let expr = Op::preprocess_expr(graph, expr);
    dependency_expr(&expr, graph, exprs, expr_to_index)
}

fn dependency_expr<Op>(
    e: &Expr,
    graph: &Graph,
    exprs: &mut IndexSet<OutputExpr<Op>>,
    expr_to_index: &mut IndexMap<Expr, usize>,
) -> Option<crate::expr::Value>
where
    Op: Operation + std::hash::Hash + Eq + Default,
{
    texture_dependency(e, graph, exprs, expr_to_index).or_else(|| {
        buffer_dependency(graph, e)
            .map(crate::expr::Value::Parameter)
            .or_else(|| match e {
                Expr::Unary(UnaryOp::Negate, e) => match &graph.exprs[*e] {
                    Expr::Float(f) => Some(crate::expr::Value::Float(-f)),
                    Expr::Int(i) => Some(crate::expr::Value::Int(-i)),
                    _ => None,
                },
                Expr::Float(f) => Some(crate::expr::Value::Float(*f)),
                Expr::Int(i) => Some(crate::expr::Value::Int(*i)),
                Expr::Global { name, channel } => {
                    // TODO: Also check if this matches a vertex input name?
                    Some(crate::expr::Value::Attribute(crate::expr::Attribute {
                        name: name.clone(),
                        channel: *channel,
                    }))
                }
                _ => None,
            })
    })
}

impl<Op> std::fmt::Display for OutputExpr<Op>
where
    Op: std::fmt::Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputExpr::Value(d) => write!(f, "{d}"),
            OutputExpr::Func { op, args } => {
                let args: Vec<_> = args.iter().map(|a| format!("var{a}")).collect();
                write!(f, "{op}({})", args.join(", "))
            }
        }
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Int(i) => write!(f, "{i:?}"),
            Value::Float(c) => write!(f, "{c:?}"),
            Value::Parameter(p) => write!(f, "{p}"),
            Value::Texture(t) => write!(f, "{t}"),
            Value::Attribute(a) => write!(f, "{a}"),
        }
    }
}

impl std::fmt::Display for Parameter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}{}{}{}",
            self.name,
            if !self.field.is_empty() {
                format!(".{}", self.field)
            } else {
                String::new()
            },
            self.index.map(|i| format!("[{i}]")).unwrap_or_default(),
            channels(self.channel)
        )
    }
}

impl std::fmt::Display for Texture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let args: Vec<_> = self.texcoords.iter().map(|t| format!("var{t}")).collect();
        write!(
            f,
            "Texture({}, {}){}",
            &self.name,
            args.join(", "),
            channels(self.channel)
        )
    }
}

impl std::fmt::Display for Attribute {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", self.name, channels(self.channel))
    }
}

fn channels(c: Option<char>) -> String {
    c.map(|c| format!(".{c}")).unwrap_or_default()
}
