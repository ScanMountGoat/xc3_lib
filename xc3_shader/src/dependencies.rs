use crate::{
    expr::{Operation, OutputExpr, output_expr},
    graph::{Expr, Graph},
};

// Faster than the default hash implementation.
type IndexSet<T> = indexmap::IndexSet<T, ahash::RandomState>;
type IndexMap<K, V> = indexmap::IndexMap<K, V, ahash::RandomState>;

pub fn texture<Op>(
    e: &Expr,
    graph: &Graph,
    exprs: &mut IndexSet<OutputExpr<Op>>,
    expr_to_index: &mut IndexMap<Expr, usize>,
) -> Option<crate::expr::Value>
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

                Some(crate::expr::Value::Texture(crate::expr::Texture {
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
    // The arg0 should always be the texture name.
    // texture(arg0, vec2(arg1, arg2))
    if let Some(Expr::Func { args, .. }) = args.get(1).map(|a| &graph.exprs[*a])
        && args.len() == 2
    {
        args.iter()
            .map(|e| output_expr(&graph.exprs[*e], graph, exprs, expr_to_index))
            .collect::<Vec<_>>()
    } else {
        // textureCube(arg0, arg1, arg2, arg3)
        args.iter()
            .skip(1)
            .map(|e| output_expr(&graph.exprs[*e], graph, exprs, expr_to_index))
            .collect::<Vec<_>>()
    }
}

pub fn parameter(graph: &Graph, e: &Expr) -> Option<crate::expr::Parameter> {
    if let Expr::Parameter {
        name,
        field,
        index,
        channel,
    } = e
    {
        if let Some(Expr::Int(index)) = index.map(|i| &graph.exprs[i]) {
            Some(crate::expr::Parameter {
                name: name.clone(),
                field: field.clone().unwrap_or_default(),
                index: Some((*index).try_into().unwrap()),
                channel: *channel,
            })
        } else {
            Some(crate::expr::Parameter {
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

#[cfg(feature = "xc3")]
impl From<crate::expr::Value> for xc3_model::shader_database::Value {
    fn from(value: crate::expr::Value) -> Self {
        match value {
            crate::expr::Value::Int(i) => Self::Int(i),
            crate::expr::Value::Float(f) => Self::Float(f),
            crate::expr::Value::Parameter(parameter) => {
                Self::Parameter(xc3_model::shader_database::Parameter {
                    name: parameter.name,
                    field: parameter.field,
                    index: parameter.index,
                    channel: parameter.channel,
                })
            }
            crate::expr::Value::Texture(texture) => {
                Self::Texture(xc3_model::shader_database::Texture {
                    name: texture.name,
                    channel: texture.channel,
                    texcoords: texture.texcoords,
                })
            }
            crate::expr::Value::Attribute(attribute) => {
                Self::Attribute(xc3_model::shader_database::Attribute {
                    name: attribute.name,
                    channel: attribute.channel,
                })
            }
        }
    }
}
