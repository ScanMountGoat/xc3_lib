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

use super::{BinaryOp, Expr, Node};
use std::ops::Deref;

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
    // result = a1 * b1;
    // result = fma(a2, b2, result);
    // result = fma(a3, b3, result);
    let (a3, b3, x) = fma_a_b_c(node)?;
    let x = node_expr(nodes, x)?;
    let a3 = node_expr(nodes, a3)?;

    let (a2, b2, x) = fma_a_b_c(x)?;
    let x = node_expr(nodes, x)?;
    let a2 = node_expr(nodes, a2)?;

    let (a1, b1) = match &x.input {
        Expr::Binary(BinaryOp::Mul, x, y) => match (x.deref(), y.deref()) {
            (Expr::Node { node_index: x, .. }, y) => Some((nodes.get(*x)?, y)),
            _ => None,
        },
        _ => None,
    }?;
    Some(([a1, a2, a3], [b1, b2, b3]))
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
    match &node.input {
        Expr::Func { name, args, .. } => {
            if name == "fma" {
                match &args[..] {
                    [Expr::Node { node_index, .. }, Expr::Float(f1), Expr::Float(f2)] => {
                        if *f1 == 0.5 && *f2 == 0.5 {
                            nodes.get(*node_index)
                        } else {
                            None
                        }
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
