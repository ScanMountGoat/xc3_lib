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

use super::{BinaryOp, Expr, Graph, Node};
use indoc::indoc;
use std::{collections::BTreeMap, ops::Deref};

impl Graph {
    /// Find the corresponding [Expr] in the graph for each [Expr::Global] in `query`
    /// or `None` if the graphs do not match.
    ///
    /// Variables in `query` function as placeholder variables and will match any input expression.
    /// This allows for extracting variable values from specific parts of the code.
    ///
    /// This uses a structural match that allows for differences in variable names
    /// and implements basic algebraic identities like `a*b == b*a`.
    pub fn query(&self, query: &Graph) -> Option<BTreeMap<String, &Expr>> {
        // TODO: Should this always be the last node?
        query_nodes(self.nodes.last()?, &self.nodes, &query.nodes)
    }
}

fn query_nodes_glsl<'a>(
    input_node: &'a Node,
    input: &'a [Node],
    query: &str,
) -> Option<BTreeMap<String, &'a Expr>> {
    let query = Graph::parse_glsl(&format!("void main() {{ {query} }}")).unwrap();
    query_nodes(input_node, input, &query.nodes)
}

fn query_nodes<'a>(
    input_node: &'a Node,
    input: &'a [Node],
    query: &[Node],
) -> Option<BTreeMap<String, &'a Expr>> {
    // Keep track of corresponding input exprs for global vars in query.
    let mut vars = BTreeMap::new();

    // TODO: Is this the right way to handle multiple nodes?

    let is_match = check_exprs(
        &query.last()?.input,
        &input_node.input,
        query,
        input,
        &mut vars,
    );

    is_match.then_some(vars)
}

fn check_exprs<'a>(
    query: &Expr,
    input: &'a Expr,
    query_nodes: &[Node],
    input_nodes: &'a [Node],
    vars: &mut BTreeMap<String, &'a Expr>,
) -> bool {
    match (query, input) {
        (Expr::Unary(op1, a1), Expr::Unary(op2, a2)) => {
            op1 == op2 && check_exprs(a1, a2, query_nodes, input_nodes, vars)
        }
        (Expr::Binary(op1, a1, b1), Expr::Binary(op2, a2, b2)) => {
            // TODO: commutativity for add, mul
            op1 == op2
                && check_exprs(a1, a2, query_nodes, input_nodes, vars)
                && check_exprs(b1, b2, query_nodes, input_nodes, vars)
        }
        (Expr::Ternary(a1, b1, c1), Expr::Ternary(a2, b2, c2)) => {
            check_exprs(a1, a2, query_nodes, input_nodes, vars)
                && check_exprs(b1, b2, query_nodes, input_nodes, vars)
                && check_exprs(c1, c2, query_nodes, input_nodes, vars)
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
                && args1.len() == args2.len()
                && args1
                    .iter()
                    .zip(args2)
                    .all(|(a1, a2)| check_exprs(a1, a2, query_nodes, input_nodes, vars))
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
            // TODO: is this the correct way to handle variables?
            c1 == c2
                && check_exprs(
                    &query_nodes[*n1].input,
                    &input_nodes[*n2].input,
                    query_nodes,
                    input_nodes,
                    vars,
                )
        }
        (Expr::Global { name, channel }, i) => {
            // TODO: What happens if the var is already in the map?
            // TODO: Also track channels?
            vars.insert(name.clone(), i);
            // TODO: Does this need to check that name usage is consistent for query and input?
            true
        }
        _ => query == input,
    }
}

pub fn assign_x<'a>(nodes: &'a [Node], node: &Node) -> Option<&'a Node> {
    match &node.input {
        Expr::Node { node_index, .. } => nodes.get(*node_index),
        _ => None,
    }
}

pub fn assign_x_recursive<'a>(nodes: &'a [Node], n: &'a Node) -> &'a Node {
    let mut node = n;
    while let Some(new_node) = assign_x(nodes, node) {
        node = new_node;
    }
    node
}

pub fn one_minus_x<'a>(nodes: &'a [Node], node: &'a Node) -> Option<&'a Expr> {
    let node = one_plus_x(nodes, node)?;
    zero_minus_x(node)
}

pub fn zero_minus_x(node: &Node) -> Option<&Expr> {
    match &node.input {
        Expr::Binary(BinaryOp::Sub, a, b) => match (a.deref(), b.deref()) {
            (Expr::Float(0.0), x) => Some(x),
            _ => None,
        },
        _ => None,
    }
}

pub fn one_plus_x<'a>(nodes: &'a [Node], node: &Node) -> Option<&'a Node> {
    // Addition is commutative.
    match &node.input {
        Expr::Binary(BinaryOp::Add, a, b) => match (a.deref(), b.deref()) {
            (Expr::Node { node_index, .. }, Expr::Float(1.0)) => nodes.get(*node_index),
            (Expr::Float(1.0), Expr::Node { node_index, .. }) => nodes.get(*node_index),
            _ => None,
        },
        _ => None,
    }
}

pub fn clamp_x_zero_one<'a>(nodes: &'a [Node], node: &Node) -> Option<&'a Node> {
    match &node.input {
        Expr::Func { name, args, .. } => {
            if name == "clamp" {
                match &args[..] {
                    [Expr::Node { node_index, .. }, Expr::Float(0.0), Expr::Float(1.0)] => {
                        nodes.get(*node_index)
                    }
                    _ => None,
                }
            } else {
                None
            }
        }
        _ => None,
    }
}

pub fn sqrt_x<'a>(nodes: &'a [Node], node: &Node) -> Option<&'a Node> {
    match &node.input {
        Expr::Func { name, args, .. } => {
            if name == "sqrt" {
                match &args[..] {
                    [Expr::Node { node_index, .. }] => nodes.get(*node_index),
                    _ => None,
                }
            } else {
                None
            }
        }
        _ => None,
    }
}

pub fn mix_a_b_ratio<'a>(
    nodes: &'a [Node],
    node: &'a Node,
) -> Option<(&'a Expr, &'a Expr, &'a Expr)> {
    // mix(a, b, ratio) = fma(b - a, ratio, a)
    // = ratio * b - ratio * a + a
    // = ratio * b + (1.0 - ratio) * a

    // TODO: Find a better way of handling both fma(a,b,c) and fma(b,a,c).
    // TODO: Should these functions take a callback to handle branching paths?
    // TODO: Some sort of query macro or function that takes code as input?
    let (x, y, a1) = fma_a_b_c(node)?;

    let (ratio, (b, a)) = node_expr(nodes, x)
        .and_then(|b_minus_a| Some((y, b_plus_neg_a(nodes, b_minus_a)?)))
        .or_else(|| {
            node_expr(nodes, y).and_then(|b_minus_a| Some((x, b_plus_neg_a(nodes, b_minus_a)?)))
        })?;

    if a != a1 {
        return None;
    }

    Some((a, b, ratio))
}

fn b_plus_neg_a<'a>(nodes: &'a [Node], b_minus_a: &'a Node) -> Option<(&'a Expr, &'a Expr)> {
    let (x, y) = match &b_minus_a.input {
        Expr::Binary(BinaryOp::Add, x, y) => Some((x.deref(), y.deref())),
        _ => None,
    }?;

    // Addition is commutative.
    node_expr(nodes, x)
        .and_then(|neg_a| Some((y, zero_minus_x(neg_a)?)))
        .or_else(|| node_expr(nodes, y).and_then(|neg_a| Some((x, zero_minus_x(neg_a)?))))
}

pub fn node_expr<'a>(nodes: &'a [Node], e: &Expr) -> Option<&'a Node> {
    if let Expr::Node { node_index, .. } = e {
        nodes.get(*node_index)
    } else {
        None
    }
}

pub fn dot3_a_b<'a>(nodes: &'a [Node], node: &'a Node) -> Option<([&'a Node; 3], [&'a Expr; 3])> {
    let query = indoc! {"
        float result = a1 * b1;
        float result = fma(a2, b2, result);
        float result = fma(a3, b3, result);
    "};
    let result = query_nodes_glsl(node, nodes, query)?;

    Some((
        [
            node_expr(nodes, result.get("a1")?)?,
            node_expr(nodes, result.get("a2")?)?,
            node_expr(nodes, result.get("a3")?)?,
        ],
        [result.get("b1")?, result.get("b2")?, result.get("b3")?],
    ))
}

pub fn fma_a_b_c(node: &Node) -> Option<(&Expr, &Expr, &Expr)> {
    match &node.input {
        Expr::Func { name, args, .. } => {
            if name == "fma" {
                match &args[..] {
                    [a, b, c] => Some((a, b, c)),
                    _ => None,
                }
            } else {
                None
            }
        }
        _ => None,
    }
}

pub fn fma_half_half<'a>(nodes: &'a [Node], node: &'a Node) -> Option<&'a Node> {
    let result = query_nodes_glsl(node, nodes, "float result = fma(x, 0.5, 0.5);")?;
    node_expr(nodes, result.get("x")?)
}

pub fn normalize<'a>(nodes: &'a [Node], node: &'a Node) -> Option<&'a Node> {
    let (x, length) = match &node.input {
        Expr::Binary(BinaryOp::Mul, a, b) => match (a.deref(), b.deref()) {
            (Expr::Node { node_index: a, .. }, Expr::Node { node_index: b, .. }) => {
                Some((nodes.get(*a)?, nodes.get(*b)?))
            }
            _ => None,
        },
        _ => None,
    }?;
    if !matches!(&length.input, Expr::Func { name, .. } if name == "inversesqrt") {
        return None;
    }

    Some(x)
}

#[cfg(test)]
mod tests {
    use super::*;
    use indoc::indoc;

    fn query_glsl(graph_glsl: &str, query_glsl: &str) -> Option<BTreeMap<String, Expr>> {
        let graph = Graph::parse_glsl(&format!("void main() {{ {graph_glsl} }}")).unwrap();
        let query = Graph::parse_glsl(&format!("void main() {{ {query_glsl} }}")).unwrap();
        // TODO: Check vars?
        graph
            .query(&query)
            .map(|v| v.into_iter().map(|(k, v)| (k, v.clone())).collect())
    }

    #[test]
    fn query_single_binary_expr() {
        // TODO: commutativity?
        assert!(query_glsl("float c = 1.0 * 2.0;", "float d = 1.0 * 2.0;").is_some());
    }

    #[test]
    fn query_single_binary_variable_expr() {
        // TODO: commutativity?
        assert!(query_glsl("float c = a * b;", "float d = b * c;").is_some());
    }

    #[test]
    fn query_single_binary_variable_expr_invalid_operand() {
        assert!(query_glsl("float c = a / 3.0;", "float d = b / 2.0;").is_none());
    }

    #[test]
    fn query_multiple_statements() {
        assert!(query_glsl(
            indoc! {"
                float a = 1.0;
                float a2 = a * 5.0;
                float b = texture(texture1, vec2(a2 + 2.0, 1.0)).x;
                float c = data[int(b)];
            "},
            indoc! {"
                float temp_4 = 1.0;
                float temp_5 = temp_4 * 5.0;
                float temp_6 = texture(texture1, vec2(temp_5 + 2.0, 1.0)).x;
                float temp_7 = data[int(temp_6)];
            "}
        )
        .is_some());
    }

    #[test]
    fn query_multiple_statements_missing_assignment() {
        assert!(query_glsl(
            indoc! {"
                float a = 1.0;
                float a2 = a * 5.0;
                float b = texture(texture1, vec2(a2 + 2.0, 1.0)).x;
                float c = data[int(b)];
            "},
            indoc! {"
                float temp_5 = 1.0 * 5.0;
                float temp_6 = texture(texture1, vec2(temp_5 + 2.0, 1.0)).x;
                float temp_7 = data[int(temp_6)];
            "}
        )
        .is_none());
    }
}
