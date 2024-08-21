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

use super::{Expr, Node};
use std::ops::Deref;

pub fn assign_x<'a>(nodes: &'a [Node], node: &Node) -> Option<&'a Node> {
    match &node.input {
        Expr::Node { node_index, .. } => nodes.get(*node_index),
        _ => None,
    }
}

pub fn one_minus_x<'a>(nodes: &'a [Node], node: &Node) -> Option<&'a Node> {
    let node = one_plus_x(nodes, node)?;
    zero_minus_x(nodes, node)
}

pub fn zero_minus_x<'a>(nodes: &'a [Node], node: &Node) -> Option<&'a Node> {
    match &node.input {
        Expr::Sub(a, b) => match (a.deref(), b.deref()) {
            (Expr::Float(0.0), Expr::Node { node_index, .. }) => nodes.get(*node_index),
            _ => None,
        },
        _ => None,
    }
}

pub fn one_plus_x<'a>(nodes: &'a [Node], node: &Node) -> Option<&'a Node> {
    // Addition is commutative.
    match &node.input {
        Expr::Add(a, b) => match (a.deref(), b.deref()) {
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
) -> Option<(&'a Node, &'a Node, &'a Expr)> {
    // mix(a, b, ratio) = fma(b - a, ratio, a)
    // = ratio * b - ratio * a + a
    // = ratio * b + (1.0 - ratio) * a
    let (b_minus_a, ratio, a1) = match &node.input {
        Expr::Func { name, args, .. } => {
            if name == "fma" {
                match &args[..] {
                    [Expr::Node {
                        node_index: b_minus_a,
                        ..
                    }, ratio, Expr::Node { node_index: a, .. }] => {
                        Some((nodes.get(*b_minus_a)?, ratio, a))
                    }
                    _ => None,
                }
            } else {
                None
            }
        }
        _ => None,
    }?;
    let (b, neg_a) = match &b_minus_a.input {
        Expr::Add(a, b) => match (a.deref(), b.deref()) {
            (
                Expr::Node {
                    node_index: neg_b, ..
                },
                Expr::Node { node_index: a, .. },
            ) => Some((nodes.get(*a)?, nodes.get(*neg_b)?)),
            _ => None,
        },
        _ => None,
    }?;
    let a = zero_minus_x(nodes, neg_a)?;
    if a != nodes.get(*a1)? {
        return None;
    }
    Some((a, b, ratio))
}

pub fn dot3_a_b<'a>(nodes: &'a [Node], node: &'a Node) -> Option<([&'a Node; 3], [&'a Expr; 3])> {
    // result = a1 * b1;
    // result = fma(a2, b2, result);
    // result = fma(a3, b3, result);
    let (a3, b3, x) = fma_a_b_c(nodes, node)?;
    let (a2, b2, x) = fma_a_b_c(nodes, x)?;
    let (a1, b1) = match &x.input {
        Expr::Mul(x, y) => match (x.deref(), y.deref()) {
            (Expr::Node { node_index: x, .. }, y) => Some((nodes.get(*x)?, y)),
            _ => None,
        },
        _ => None,
    }?;
    Some(([a1, a2, a3], [b1, b2, b3]))
}

fn fma_a_b_c<'a>(nodes: &'a [Node], node: &'a Node) -> Option<(&'a Node, &'a Expr, &'a Node)> {
    match &node.input {
        Expr::Func { name, args, .. } => {
            if name == "fma" {
                match &args[..] {
                    [Expr::Node { node_index: a3, .. }, b3, Expr::Node { node_index, .. }] => {
                        Some((nodes.get(*a3)?, b3, nodes.get(*node_index)?))
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
        Expr::Mul(a, b) => match (a.deref(), b.deref()) {
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
