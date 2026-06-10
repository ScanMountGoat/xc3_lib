use crate::{
    expr::{ExprCache, Operation, output_expr},
    graph::{Expr, Graph},
};

pub fn texture<Op>(e: &Expr, graph: &Graph, exprs: &mut ExprCache<Op>) -> Option<crate::expr::Value>
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
                let texcoords = texcoord_args(args, graph, exprs);

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

fn texcoord_args<Op>(args: &[usize], graph: &Graph, exprs: &mut ExprCache<Op>) -> Vec<usize>
where
    Op: Operation + std::hash::Hash + Eq + Default,
{
    // The arg0 should always be the texture name.
    // texture(arg0, vec2(arg1, arg2))
    if let Some(Expr::Func { args, .. }) = args.get(1).map(|a| &graph.exprs[*a])
        && args.len() == 2
    {
        args.iter()
            .map(|e| output_expr(&graph.exprs[*e], graph, exprs))
            .collect::<Vec<_>>()
    } else {
        // textureCube(arg0, arg1, arg2, arg3)
        args.iter()
            .skip(1)
            .map(|e| output_expr(&graph.exprs[*e], graph, exprs))
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
