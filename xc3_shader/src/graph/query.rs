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
        query_nodes(&self.nodes.last()?.input, &self.nodes, &query.nodes)
    }
}

pub fn query_nodes_glsl<'a>(
    input: &'a Expr,
    input_nodes: &'a [Node],
    query: &str,
) -> Option<BTreeMap<String, &'a Expr>> {
    let query = Graph::parse_glsl(&format!("void main() {{ {query} }}")).unwrap();
    query_nodes(input, input_nodes, &query.nodes)
}

fn query_nodes<'a>(
    input: &'a Expr,
    input_nodes: &'a [Node],
    query_nodes: &[Node],
) -> Option<BTreeMap<String, &'a Expr>> {
    // Keep track of corresponding input exprs for global vars in query.
    let mut vars = BTreeMap::new();

    // TODO: Is this the right way to handle multiple nodes?

    let is_match = check_exprs(
        &query_nodes.last()?.input,
        input,
        query_nodes,
        input_nodes,
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
    let mut check = |a, b| check_exprs(a, b, query_nodes, input_nodes, vars);

    match (query, input) {
        (Expr::Binary(BinaryOp::Sub, a1, b1), Expr::Binary(BinaryOp::Add, a2, b2)) => {
            // a - b == a + (-b) == a + (0.0 - b)
            // TODO: Find a way to avoid repetition.
            match b2.deref() {
                Expr::Unary(UnaryOp::Negate, b2) => check(a1, a2) && check(b1, b2),
                Expr::Binary(BinaryOp::Sub, z, b2) => {
                    **z == Expr::Float(0.0) && check(a1, a2) && check(b1, b2)
                }
                _ => false,
            }
        }
        (Expr::Binary(BinaryOp::Add, a1, b1), Expr::Binary(BinaryOp::Sub, a2, b2)) => {
            // a + (-b) == a + (0.0 - b) == a - b
            match b1.deref() {
                Expr::Unary(UnaryOp::Negate, b1) => check(a1, a2) && check(b1, b2),
                Expr::Binary(BinaryOp::Sub, z, b1) => {
                    **z == Expr::Float(0.0) && check(a1, a2) && check(b1, b2)
                }
                _ => false,
            }
        }
        (Expr::Unary(op1, a1), Expr::Unary(op2, a2)) => op1 == op2 && check(a1, a2),
        (Expr::Binary(op1, a1, b1), Expr::Binary(op2, a2, b2)) => {
            op1 == op2
                && if matches!(op1, BinaryOp::Add | BinaryOp::Mul) {
                    // commutativity
                    check(a1, a2) && check(b1, b2) || check(a1, b2) && check(b1, a2)
                } else {
                    check(a1, a2) && check(b1, b2)
                }
        }
        (Expr::Ternary(a1, b1, c1), Expr::Ternary(a2, b2, c2)) => {
            check(a1, a2) && check(b1, b2) && check(c1, c2)
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
                && if name1 == "fma" {
                    // commutativity of the mul part of fma
                    check(&args1[0], &args2[0])
                        && check(&args1[1], &args2[1])
                        && check(&args1[2], &args2[2])
                        || check(&args1[1], &args2[0])
                            && check(&args1[0], &args2[1])
                            && check(&args1[2], &args2[2])
                } else {
                    args1.iter().zip(args2).all(|(a1, a2)| check(a1, a2))
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
        ) => c1 == c2 && check(&query_nodes[*n1].input, &input_nodes[*n2].input),
        // TODO: Should this always be fully recursive?
        // ex: fma(x, 0.5, 0.5) == fma(y, 0.5, 0.5) doesn't need to keep following x or y
        // (
        //     Expr::Node {
        //         node_index,
        //         channel: _,
        //     },
        //     b,
        // ) => {
        //     // Recursively eliminate assignments from query.
        //     // TODO: How to handle channels?
        //     check(&query_nodes[*node_index].input, b)
        // }
        // (
        //     a,
        //     Expr::Node {
        //         node_index,
        //         channel: _,
        //     },
        // ) => {
        //     // Recursively eliminate assignments from input.
        //     // TODO: How to handle channels?
        //     check(a, &input_nodes[*node_index].input)
        // }
        (Expr::Global { name, channel: _ }, i) => {
            // TODO: What happens if the var is already in the map?
            // TODO: Also track channels?
            vars.insert(name.clone(), i);
            // TODO: Does this need to check that name usage is consistent for query and input?
            true
        }
        (Expr::Unary(UnaryOp::Negate, a1), Expr::Binary(BinaryOp::Sub, a2, b2)) => {
            // 0.0 - x == -x
            **a2 == Expr::Float(0.0) && check(a1, b2)
        }
        (Expr::Binary(BinaryOp::Sub, a1, b1), Expr::Unary(UnaryOp::Negate, a2)) => {
            // 0.0 - x == -x
            **a1 == Expr::Float(0.0) && check(b1, a2)
        }
        _ => query == input,
    }
}

pub fn assign_x<'a>(nodes: &'a [Node], expr: &Expr) -> Option<&'a Expr> {
    match expr {
        Expr::Node { node_index, .. } => nodes.get(*node_index).map(|n| &n.input),
        _ => None,
    }
}

pub fn assign_x_recursive<'a>(nodes: &'a [Node], expr: &'a Expr) -> &'a Expr {
    let mut node = expr;
    while let Some(new_node) = assign_x(nodes, node) {
        node = new_node;
    }
    node
}

pub fn zero_minus_x(expr: &Expr) -> Option<&Expr> {
    match expr {
        Expr::Binary(BinaryOp::Sub, a, b) => match (a.deref(), b.deref()) {
            (Expr::Float(0.0), x) => Some(x),
            _ => None,
        },
        _ => None,
    }
}

pub fn mix_a_b_ratio<'a>(
    nodes: &'a [Node],
    expr: &'a Expr,
) -> Option<(&'a Expr, &'a Expr, &'a Expr)> {
    // mix(a, b, ratio) = fma(b - a, ratio, a)
    // = ratio * b - ratio * a + a
    // = ratio * b + (1.0 - ratio) * a

    // TODO: Find a better way of handling both fma(a,b,c) and fma(b,a,c).
    // TODO: Should these functions take a callback to handle branching paths?
    // TODO: Some sort of query macro or function that takes code as input?
    let (x, y, a1) = fma_a_b_c(expr)?;

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

fn b_plus_neg_a<'a>(nodes: &'a [Node], b_minus_a: &'a Expr) -> Option<(&'a Expr, &'a Expr)> {
    let (x, y) = match b_minus_a {
        Expr::Binary(BinaryOp::Add, x, y) => Some((x.deref(), y.deref())),
        _ => None,
    }?;

    // Addition is commutative.
    node_expr(nodes, x)
        .and_then(|neg_a| Some((y, zero_minus_x(neg_a)?)))
        .or_else(|| node_expr(nodes, y).and_then(|neg_a| Some((x, zero_minus_x(neg_a)?))))
}

pub fn node_expr<'a>(nodes: &'a [Node], e: &Expr) -> Option<&'a Expr> {
    if let Expr::Node { node_index, .. } = e {
        nodes.get(*node_index).map(|n| &n.input)
    } else {
        None
    }
}

pub fn dot3_a_b<'a>(nodes: &'a [Node], expr: &'a Expr) -> Option<([&'a Expr; 3], [&'a Expr; 3])> {
    let query = indoc! {"
        result = a1 * b1;
        result = fma(a2, b2, result);
        result = fma(a3, b3, result);
    "};
    let result = query_nodes_glsl(expr, nodes, query)?;

    Some((
        [result.get("a1")?, result.get("a2")?, result.get("a3")?],
        [result.get("b1")?, result.get("b2")?, result.get("b3")?],
    ))
}

pub fn fma_a_b_c(expr: &Expr) -> Option<(&Expr, &Expr, &Expr)> {
    match expr {
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

pub fn fma_half_half<'a>(nodes: &'a [Node], expr: &'a Expr) -> Option<&'a Expr> {
    let result = query_nodes_glsl(expr, nodes, "result = fma(x, 0.5, 0.5);")?;
    node_expr(nodes, result.get("x")?)
}

pub fn normalize<'a>(nodes: &'a [Node], expr: &'a Expr) -> Option<&'a Node> {
    let (x, length) = match &expr {
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
        assert!(query_glsl(
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
        .is_some());
    }

    #[test]
    fn query_multiple_statements_missing_assignment() {
        assert!(query_glsl(
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
        .is_none());
    }

    #[test]
    fn query_simplification() {
        assert!(query_glsl(
            indoc! {"
                result = 0.0 - glossiness;
                result = 1.0 + result;
                result = fma(result, result, temp);
                result = clamp(result, 0.0, 1.0);
                result = sqrt(result);
                result = 0.0 - result;
                result = result + 1.0;
                result = result;
            "},
            indoc! {"
                result = 1.0 - glossiness;
                result = fma(result, result, temp);
                result = clamp(result, 0.0, 1.0);
                result = 1.0 - sqrt(result);
            "}
        )
        .is_some());
    }
}
