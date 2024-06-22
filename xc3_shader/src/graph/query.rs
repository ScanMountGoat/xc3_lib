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
