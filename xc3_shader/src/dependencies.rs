use crate::{
    graph::{Expr, Graph},
    shader_database::output_expr,
};

use indexmap::{IndexMap, IndexSet};
use xc3_model::shader_database::{BufferDependency, Dependency, OutputExpr, TextureDependency};

pub fn texture_dependency(
    e: &Expr,
    graph: &Graph,
    exprs: &mut IndexSet<OutputExpr>,
    expr_to_index: &mut IndexMap<Expr, usize>,
) -> Option<Dependency> {
    if let Expr::Func {
        name,
        args,
        channel,
    } = e
    {
        if name.starts_with("texture") {
            if let Some(Expr::Global { name, .. }) = args.first() {
                let texcoords = texcoord_args(args, graph, exprs, expr_to_index);

                Some(Dependency::Texture(TextureDependency {
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

fn texcoord_args(
    args: &[Expr],
    graph: &Graph,
    exprs: &mut IndexSet<OutputExpr>,
    expr_to_index: &mut IndexMap<Expr, usize>,
) -> Vec<usize> {
    // Search recursively to find texcoord variables.
    // The first arg is always the texture name.
    // texture(arg0, vec2(arg2, arg3, ...))
    args.iter()
        .skip(1)
        .flat_map(|a| {
            a.exprs_recursive()
                .iter()
                .skip(1)
                .map(|e| output_expr(e, graph, exprs, expr_to_index))
                .collect::<Vec<_>>()
        })
        .collect()
}

pub fn buffer_dependency(e: &Expr) -> Option<BufferDependency> {
    if let Expr::Parameter {
        name,
        field,
        index,
        channel,
    } = e
    {
        if let Some(Expr::Int(index)) = index.as_deref() {
            Some(BufferDependency {
                name: name.clone(),
                field: field.clone().unwrap_or_default(),
                index: Some((*index).try_into().unwrap()),
                channel: *channel,
            })
        } else {
            Some(BufferDependency {
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
    Graph::from_latte_asm(source).glsl_dependencies(variable, channel, None)
}
