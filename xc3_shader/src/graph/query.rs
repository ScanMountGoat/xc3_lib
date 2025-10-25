//! Utilities for querying the structure of a graph.
//!
//! Identical shader code may have different variable names or whitespace
//! This is known in the literature as a "type-2 code clone".
//! These differences are especially common in decompiled code since
//! variable names are often based on how registers are allocated.
//!
//! A common solution is to normalize identifiers and whitespace before comparison.
//! This is possible using a graph representation that encodes data flow without variables.
//! Two code segments are considered equivalent if their graphs are isomorphic.
//! For example, the programs `a = 0; b = 1; c = a + b` and
//! `temp0 = 0; temp63 = 1; temp4 = temp0 + temp63` should have isomorphic graphs.
//!
//! The graph representation is chosen to allow structural matches
//! using node expr indices as edges in the graph.
//! Each node still stores its output variable name to facilitate debugging and searches.
//!
//! Extracting only the nodes that affect the output
//! with [assignments_recursive](super::Graph::assignments_recursive)
//! avoids needing to handle added or removed lines found in type-3 clones.

use crate::graph::UnaryOp;

use super::{BinaryOp, Expr, Graph};
use indexmap::IndexMap;
use indoc::indoc;
use ordered_float::OrderedFloat;
use smol_str::SmolStr;
use std::{collections::BTreeMap, sync::LazyLock};

impl Graph {
    /// Find the corresponding [Expr] in the graph for each [Expr::Global] in `query`
    /// or `None` if the graphs do not match.
    ///
    /// Variables in `query` function as placeholder variables and will match any input expression.
    /// This allows for extracting variable values from specific parts of the code.
    ///
    /// This uses a structural match that allows for differences in variable names
    /// and implements basic algebraic identities like `a*b == b*a`.
    pub fn query(&self, query: &Graph) -> Option<BTreeMap<SmolStr, &Expr>> {
        // TODO: Should this always be the last node?
        query_nodes(&self.exprs[self.nodes.last()?.input], self, query)
    }
}

/// A convenience method for [query_nodes] when the query is GLSL code.
///
/// Consider using [query_nodes] and initializing the query graph
/// ahead of time with [Graph::parse_glsl] if the query is used many times.
pub fn query_nodes_glsl<'a>(
    input: &'a Expr,
    input_graph: &'a Graph,
    query: &str,
) -> Option<BTreeMap<SmolStr, &'a Expr>> {
    // TODO: simplify both query and graph to a single expr?
    let query = Graph::parse_glsl(&format!("void main() {{ {query} }}")).unwrap();
    query_nodes(input, input_graph, &query)
}

/// Find the corresponding [Expr] in the graph for each [Expr::Global] in `query_nodes`
/// or `None` if the graphs do not match.
///
/// Variables in `query_nodes` function as placeholder variables and will match any input expression.
/// This allows for extracting variable values from specific parts of the code.
/// Unrelated nodes in the input will be ignored.
///
/// This uses a structural match that effectively checks if the query is a subgraph of the input
/// while allowing for differences in variable names basic algebraic identities like `a*b == b*a`.
pub fn query_nodes<'a>(
    input: &'a Expr,
    input_graph: &'a Graph,
    query_graph: &Graph,
) -> Option<BTreeMap<SmolStr, &'a Expr>> {
    // Keep track of corresponding input exprs for global vars in query.
    let mut vars = BTreeMap::new();

    // TODO: Is this the right way to handle multiple nodes?
    let is_match = check_exprs(
        query_graph.nodes.last()?.input,
        input_graph.exprs.iter().position(|e| e == input)?,
        query_graph,
        input_graph,
        &mut vars,
    );

    is_match.then_some(vars)
}

fn check_exprs<'a>(
    query: usize,
    input: usize,
    query_graph: &Graph,
    input_graph: &'a Graph,
    vars: &mut BTreeMap<SmolStr, &'a Expr>,
) -> bool {
    let mut check = |a, b| check_args(a, b, query_graph, input_graph, vars);

    match (&query_graph.exprs[query], &input_graph.exprs[input]) {
        (Expr::Binary(BinaryOp::Sub, a1, b1), Expr::Binary(BinaryOp::Add, a2, b2)) => {
            // a - b == a + (-b) == a + (0.0 - b)
            // TODO: Find a way to avoid repetition.
            match &input_graph.exprs[*b2] {
                Expr::Unary(UnaryOp::Negate, b2) => check(&[*a1, *b1], &[*a2, *b2]),
                Expr::Binary(BinaryOp::Sub, z, b2) => {
                    input_graph.exprs[*z] == Expr::Float(OrderedFloat(0.0))
                        && check(&[*a1, *b1], &[*a2, *b2])
                }
                _ => false,
            }
        }
        (Expr::Binary(BinaryOp::Add, a1, b1), Expr::Binary(BinaryOp::Sub, a2, b2)) => {
            // a + (-b) == a + (0.0 - b) == a - b
            match &query_graph.exprs[*b1] {
                Expr::Unary(UnaryOp::Negate, b1) => check(&[*a1, *b1], &[*a2, *b2]),
                Expr::Binary(BinaryOp::Sub, z, b1) => {
                    query_graph.exprs[*z] == Expr::Float(OrderedFloat(0.0))
                        && check(&[*a1, *b1], &[*a2, *b2])
                }
                _ => false,
            }
        }
        (Expr::Unary(op1, a1), Expr::Unary(op2, a2)) => op1 == op2 && check(&[*a1], &[*a2]),
        (Expr::Binary(op1, a1, b1), Expr::Binary(op2, a2, b2)) => {
            op1 == op2
                && if matches!(op1, BinaryOp::Add | BinaryOp::Mul) {
                    // commutativity
                    let q1 = &[*a1, *b1];
                    let q2 = &[*b1, *a1];
                    let i = &[*a2, *b2];
                    check(q1, i) || check(q2, i)
                } else {
                    check(&[*a1, *b1], &[*a2, *b2])
                }
        }
        (Expr::Ternary(a1, b1, c1), Expr::Ternary(a2, b2, c2)) => {
            check(&[*a1, *b1, *c1], &[*a2, *b2, *c2])
        }
        (
            Expr::Func {
                name: name1,
                args: args1,
                channel: channel1,
            },
            Expr::Func {
                name: name2,
                args: args2,
                channel: channel2,
            },
        ) => {
            name1 == name2
                && channel1 == channel2
                && if name1 == "fma" {
                    // commutativity of the mul part of fma
                    let q1 = &[args1[0], args1[1], args1[2]];
                    let q2 = &[args1[1], args1[0], args1[2]];
                    let i = &[args2[0], args2[1], args2[2]];
                    check(q1, i) || check(q2, i)
                } else if name1 == "max" || name1 == "min" {
                    // The order does not matter for max/min.
                    let q1 = &[args1[0], args1[1]];
                    let q2 = &[args1[1], args1[0]];
                    let i = &[args2[0], args2[1]];
                    check(q1, i) || check(q2, i)
                } else {
                    check(args1, args2)
                }
        }
        (
            Expr::Node {
                node_index: n1,
                channel: c1,
            },
            Expr::Node {
                node_index: n2,
                channel: c2,
            },
        ) => {
            check_channels(*c1, *c2)
                && check(
                    &[query_graph.nodes[*n1].input],
                    &[input_graph.nodes[*n2].input],
                )
        }
        (
            Expr::Parameter {
                name: n1,
                field: f1,
                index: i1,
                channel: c1,
            },
            Expr::Parameter {
                name: n2,
                field: f2,
                index: i2,
                channel: c2,
            },
        ) => {
            if let (Some(i1), Some(i2)) = (i1, i2) {
                n1 == n2 && f1 == f2 && c1 == c2 && check(&[*i1], &[*i2])
            } else {
                n1 == n2 && f1 == f2 && c1 == c2
            }
        }
        (Expr::Global { name, channel }, i) => {
            // TODO: What happens if the var is already in the map?
            // TODO: Special case to check name if query and input are both Expr::Global?
            vars.insert(name.clone(), i);

            check_channels(*channel, i.channel())
        }
        // TODO: Move this to simplification instead?
        (Expr::Unary(UnaryOp::Negate, a1), Expr::Binary(BinaryOp::Sub, a2, b2)) => {
            // 0.0 - x == -x
            input_graph.exprs[*a2] == Expr::Float(OrderedFloat(0.0)) && check(&[*a1], &[*b2])
        }
        (Expr::Binary(BinaryOp::Sub, a1, b1), Expr::Unary(UnaryOp::Negate, a2)) => {
            // 0.0 - x == -x
            query_graph.exprs[*a1] == Expr::Float(OrderedFloat(0.0)) && check(&[*b1], &[*a2])
        }
        (q, i) => q == i,
    }
}

fn check_channels(query: Option<char>, input: Option<char>) -> bool {
    // Treat unspecified channels as allowing all channels.
    query.is_none() || query == input
}

fn check_args<'a>(
    query: &[usize],
    input: &[usize],
    query_graph: &Graph,
    input_graph: &'a Graph,
    vars: &mut BTreeMap<SmolStr, &'a Expr>,
) -> bool {
    // Track values for query variables used in this expr.
    // fma(a, b, a) should not match fma(0.0, 1.0, 2.0).
    // fma(a, b, c) should still match fma(1.0, 1.0, 1.0).
    let mut local_vars = IndexMap::new();
    query.len() == input.len()
        && query.iter().zip(input).all(|(q, i)| {
            if let Expr::Global { name, .. } = &query_graph.exprs[*q]
                && let Some(i_prev) = local_vars.insert(name.clone(), i)
            {
                // TODO: Should this check equivalent exprs?
                if i_prev != i {
                    return false;
                }
            }
            check_exprs(*q, *i, query_graph, input_graph, vars)
        })
}

pub fn assign_x_recursive<'a>(graph: &'a Graph, expr: &'a Expr) -> &'a Expr {
    let mut node = expr;
    while let Some(new_node) = assign_x(graph, node) {
        node = new_node;
    }
    node
}

fn assign_x<'a>(graph: &'a Graph, expr: &Expr) -> Option<&'a Expr> {
    match expr {
        Expr::Node { node_index, .. } => {
            graph.nodes.get(*node_index).map(|n| &graph.exprs[n.input])
        }
        _ => None,
    }
}

static MIX_A_B_RATIO: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
        void main() {
            neg_a = 0.0 - a;
            b_minus_a = neg_a + b;
            result = fma(b_minus_a, ratio, a);
        }
    "};
    Graph::parse_glsl(query).unwrap()
});

pub fn mix_a_b_ratio<'a>(
    graph: &'a Graph,
    expr: &'a Expr,
) -> Option<(&'a Expr, &'a Expr, &'a Expr)> {
    let result = query_nodes(expr, graph, &MIX_A_B_RATIO)?;
    let a = result.get("a")?;
    let b = result.get("b")?;
    let ratio = result.get("ratio")?;
    Some((a, b, ratio))
}

pub fn node_expr<'a>(graph: &'a Graph, e: &Expr) -> Option<&'a Expr> {
    if let Expr::Node { node_index, .. } = e {
        graph.nodes.get(*node_index).map(|n| &graph.exprs[n.input])
    } else {
        None
    }
}

static DOT3_A_B: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
        void main() {
            result = a1 * b1;
            result = fma(a2, b2, result);
            result = fma(a3, b3, result);
        }
    "};
    Graph::parse_glsl(query).unwrap()
});

pub fn dot3_a_b<'a>(graph: &'a Graph, expr: &'a Expr) -> Option<([&'a Expr; 3], [&'a Expr; 3])> {
    let result = query_nodes(expr, graph, &DOT3_A_B)?;
    Some((
        [result.get("a1")?, result.get("a2")?, result.get("a3")?],
        [result.get("b1")?, result.get("b2")?, result.get("b3")?],
    ))
}

pub fn fma_a_b_c<'a>(graph: &'a Graph, expr: &'a Expr) -> Option<(&'a Expr, &'a Expr, &'a Expr)> {
    match expr {
        Expr::Func { name, args, .. } => {
            if name == "fma" {
                match &args[..] {
                    [a, b, c] => Some((&graph.exprs[*a], &graph.exprs[*b], &graph.exprs[*c])),
                    _ => None,
                }
            } else {
                None
            }
        }
        _ => None,
    }
}

static FMA_HALF_HALF: LazyLock<Graph> =
    LazyLock::new(|| Graph::parse_glsl("void main() { result = fma(x, 0.5, 0.5); }").unwrap());

pub fn fma_half_half<'a>(graph: &'a Graph, expr: &'a Expr) -> Option<&'a Expr> {
    let result = query_nodes(expr, graph, &FMA_HALF_HALF)?;
    node_expr(graph, result.get("x")?)
}

static NORMALIZE: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
        void main() {
            inv_length = inversesqrt(length);
            result = result * inv_length;
        }
    "};
    Graph::parse_glsl(query).unwrap()
});

pub fn normalize<'a>(graph: &'a Graph, expr: &'a Expr) -> Option<&'a Expr> {
    let result = query_nodes(expr, graph, &NORMALIZE)?;
    result.get("result").copied()
}

#[cfg(test)]
mod tests {
    use super::*;
    use indoc::indoc;

    fn query_glsl(graph_glsl: &str, query_glsl: &str) -> Option<BTreeMap<String, Expr>> {
        let graph = Graph::parse_glsl(&format!("void main() {{ {graph_glsl} }}")).unwrap();
        let query = Graph::parse_glsl(&format!("void main() {{ {query_glsl} }}")).unwrap();

        // TODO: Check vars?
        graph.query(&query).map(|v| {
            v.into_iter()
                .map(|(k, v)| (k.to_string(), v.clone()))
                .collect()
        })
    }

    fn query_glsl_simplified(graph_glsl: &str, query_glsl: &str) -> Option<BTreeMap<String, Expr>> {
        let graph = Graph::parse_glsl(&format!("void main() {{ {graph_glsl} }}")).unwrap();
        let query = Graph::parse_glsl(&format!("void main() {{ {query_glsl} }}")).unwrap();

        let graph = graph.simplify();
        let query = query.simplify();

        // TODO: Check vars?
        graph.query(&query).map(|v| {
            v.into_iter()
                .map(|(k, v)| (k.to_string(), v.clone()))
                .collect()
        })
    }

    #[test]
    fn query_single_binary_expr_mul() {
        assert!(query_glsl("c = 1.0 * 2.0;", "d = 1.0 * 2.0;").is_some());
        assert!(query_glsl("c = 1.0 * 2.0;", "d = 2.0 * 1.0;").is_some());
    }

    #[test]
    fn query_single_binary_expr_add() {
        assert!(query_glsl("c = 1.0 + 2.0;", "d = 1.0 + 2.0;").is_some());
        assert!(query_glsl("c = 1.0 + 2.0;", "d = 2.0 + 1.0;").is_some());
    }

    #[test]
    fn query_negate() {
        assert!(query_glsl("c = 0.0 - x;", "d = -x;").is_some());
        assert!(query_glsl("c = -x;", "d = 0.0 - x;").is_some());
    }

    #[test]
    fn query_subtract() {
        assert!(query_glsl("c = a - b;", "d = a + (-b);").is_some());
        assert!(query_glsl("c = a + (-b);", "d = a - b;").is_some());
        assert!(query_glsl("c = a - b;", "d = a + (0.0 -b);").is_some());
        assert!(query_glsl("c = a + (0.0 -b);", "d = a - b;").is_some());
    }

    #[test]
    fn query_min() {
        assert!(query_glsl("a = min(1.0, GLOBAL.x);", "d = min(b, GLOBAL.x);").is_some());
        assert!(query_glsl("a = min(GLOBAL.x, 1.0);", "d = min(b, GLOBAL.x);").is_some());
    }

    #[test]
    fn query_max() {
        assert!(query_glsl("a = max(1.0, GLOBAL.x);", "d = max(b, GLOBAL.x);").is_some());
        assert!(query_glsl("a = max(GLOBAL.x, 1.0);", "d = max(b, GLOBAL.x);").is_some());
    }

    #[test]
    fn query_single_binary_variable_expr() {
        assert!(query_glsl("c = a * b;", "d = b * c;").is_some());
    }

    #[test]
    fn query_single_binary_variable_expr_invalid_operand() {
        assert!(query_glsl("c = a / 3.0;", "d = b / 2.0;").is_none());
    }

    #[test]
    fn query_negation() {
        assert!(query_glsl("c = 1.0 * 2.0;", "d = 1.0 * 2.0;").is_some());
    }

    #[test]
    fn query_multiple_statements() {
        assert!(
            query_glsl(
                indoc! {"
                    a = 1.0;
                    a2 = a * 5.0;
                    b = texture(texture1, vec2(a2 + 2.0, 1.0)).x;
                    c = data[int(b)];
                "},
                indoc! {"
                    temp_4 = 1.0;
                    temp_5 = temp_4 * 5.0;
                    temp_6 = texture(texture1, vec2(temp_5 + 2.0, 1.0)).x;
                    temp_7 = data[int(temp_6)];
                "}
            )
            .is_some()
        );
    }

    #[test]
    fn query_parameters() {
        assert!(
            query_glsl(
                indoc! {"
                    PS32 = log2(R4.z);
                    PV33.w = KC0[2].w * PS32;
                    PS34 = exp2(PV33.w);
                "},
                indoc! {"
                    result = log2(a);
                    result = result * b;
                    result = exp2(result);
                "},
            )
            .is_some()
        );
    }

    #[test]
    fn query_multiple_statements_missing_assignment() {
        assert!(
            query_glsl(
                indoc! {"
                    a = 1.0;
                    a2 = a * 5.0;
                    b = texture(texture1, vec2(a2 + 2.0, 1.0)).x;
                    c = data[int(b)];
                "},
                indoc! {"
                    temp_5 = 1.0 * 5.0;
                    temp_6 = texture(texture1, vec2(temp_5 + 2.0, 1.0)).x;
                    temp_7 = data[int(temp_6)];
                "}
            )
            .is_none()
        );
    }

    #[test]
    fn query_repeated_args() {
        assert!(query_glsl("result = fma(1.0, 2.0, 3.0);", "result = fma(a, b, c);").is_some());
        assert!(query_glsl("result = fma(1.0, 2.0, 3.0);", "result = fma(a, b, a);").is_none());
        assert!(query_glsl("result = fma(1.0, 2.0, 1.0);", "result = fma(a, b, a);").is_some());
        assert!(query_glsl("result = fma(1.0, 1.0, 1.0);", "result = fma(a, b, c);").is_some());
    }

    #[test]
    fn query_repeated_args_multiple_assignments() {
        assert!(
            query_glsl(
                indoc! {"
                    temp0 = 4.0;
                    temp1 = temp0 + 3.0;
                    result = fma(temp1, 2.0, temp1);
                "},
                indoc! {"
                    a = 4.0;
                    b = a + 3.0;
                    result = fma(b, c, b);
                "}
            )
            .is_some()
        );
    }

    #[test]
    fn query_simplification() {
        let graph = indoc! {"
            color = texture(s0, vec2(0.0, 0.5));
            color2 = color;
            glossiness = color2.x;
            result = 0.0 - glossiness;
            result = 1.0 + result;
            result = clamp(result, 0.0, 1.0);
            result = sqrt(result);
            result = 0.0 - result;
            result = result + 1.0;
            result = result;
        "};

        assert!(
            query_glsl_simplified(
                graph,
                indoc! {"
                    result = 1.0 + (0.0 - texture(s0, vec2(0.0, 0.5)).x);
                    result = clamp(result, 0.0, 1.0);
                    result = 1.0 + (0.0 - sqrt(result));
                "}
            )
            .is_some()
        );

        assert!(
            query_glsl_simplified(
                graph,
                "result = 1.0 - sqrt(clamp(1.0 - texture(s0, vec2(0.0, 0.5)).x, 0.0, 1.0));"
            )
            .is_some()
        );
    }
}
