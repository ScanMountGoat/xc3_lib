use crate::{
    expr::{output_expr, Operation, OutputExpr, Parameter, Texture, Value},
    graph::{Expr, Graph},
};

use indexmap::{IndexMap, IndexSet};

use xc3_model::shader_database::{
    AttributeDependency, BufferDependency, Dependency, TextureDependency,
};

pub fn texture_dependency<Op>(
    e: &Expr,
    graph: &Graph,
    exprs: &mut IndexSet<OutputExpr<Op>>,
    expr_to_index: &mut IndexMap<Expr, usize>,
) -> Option<Value>
where
    Op: Operation + std::hash::Hash + Eq + Default,
{
    if let Expr::Func {
        name,
        args,
        channel,
    } = e
    {
        if name.starts_with("texture") {
            if let Some(Expr::Global { name, .. }) = args.first().map(|a| &graph.exprs[*a]) {
                let texcoords = texcoord_args(args, graph, exprs, expr_to_index);

                Some(Value::Texture(Texture {
                    name: name.clone(),
                    channel: *channel,
                    texcoords,
                }))
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    }
}

fn texcoord_args<Op>(
    args: &[usize],
    graph: &Graph,
    exprs: &mut IndexSet<OutputExpr<Op>>,
    expr_to_index: &mut IndexMap<Expr, usize>,
) -> Vec<usize>
where
    Op: Operation + std::hash::Hash + Eq + Default,
{
    // The first arg is always the texture name.
    // texture(arg0, vec2(arg2, arg3, ...))
    if let Some(Expr::Func { args, .. }) = args.get(1).map(|a| &graph.exprs[*a]) {
        args.iter()
            .map(|e| output_expr(&graph.exprs[*e], graph, exprs, expr_to_index))
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    }
}

pub fn buffer_dependency(graph: &Graph, e: &Expr) -> Option<Parameter> {
    if let Expr::Parameter {
        name,
        field,
        index,
        channel,
    } = e
    {
        if let Some(Expr::Int(index)) = index.map(|i| &graph.exprs[i]) {
            Some(Parameter {
                name: name.clone(),
                field: field.clone().unwrap_or_default(),
                index: Some((*index).try_into().unwrap()),
                channel: *channel,
            })
        } else {
            Some(Parameter {
                name: name.clone(),
                field: field.clone().unwrap_or_default(),
                index: None,
                channel: *channel,
            })
        }
    } else {
        None
    }
}

pub fn latte_dependencies(source: &str, variable: &str, channel: Option<char>) -> String {
    Graph::from_latte_asm(source)
        .unwrap()
        .glsl_dependencies(variable, channel, None)
}

impl From<Value> for Dependency {
    fn from(value: Value) -> Self {
        match value {
            Value::Constant(f) => Self::Constant(f),
            Value::Parameter(parameter) => Self::Buffer(BufferDependency {
                name: parameter.name,
                field: parameter.field,
                index: parameter.index,
                channel: parameter.channel,
            }),
            Value::Texture(texture) => Self::Texture(TextureDependency {
                name: texture.name,
                channel: texture.channel,
                texcoords: texture.texcoords,
            }),
            Value::Attribute(attribute) => Self::Attribute(AttributeDependency {
                name: attribute.name,
                channel: attribute.channel,
            }),
        }
    }
}
