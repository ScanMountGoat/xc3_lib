use pest::{iterators::Pair, Parser};
use pest_derive::Parser;

use super::*;

#[derive(Default)]
struct Nodes {
    nodes: Vec<Node>,
    node_index_alu_unit_inst_count: Vec<(usize, Option<char>, usize)>,
}

impl Nodes {
    fn add_node(&mut self, node: Node, alu_unit: Option<char>, inst_count: usize) -> usize {
        let index = self.nodes.len();
        self.nodes.push(node);
        self.node_index_alu_unit_inst_count
            .push((index, alu_unit, inst_count));
        index
    }
}

// TODO: The first registers are always input attributes?
impl Graph {
    pub fn from_latte_asm(asm: &str) -> Self {
        // TODO: The FETCH instruction isn't part of the official grammar?
        let asm = asm
            .lines()
            .filter(|l| !l.contains("FETCH"))
            .collect::<Vec<_>>()
            .join("\n");
        if asm.is_empty() {
            return Graph::default();
        }

        let program = LatteParser::parse(Rule::program, &asm)
            .unwrap()
            .next()
            .unwrap();

        let mut nodes = Nodes::default();

        for pair in program.into_inner() {
            if pair.as_rule() == Rule::instruction {
                let inst = pair.into_inner().next().unwrap();
                match inst.as_rule() {
                    Rule::cf_inst => {
                        let mut inner = inst.into_inner();
                        let _inst_count: usize = inner.next().unwrap().as_str().parse().unwrap();
                        let _op_code = inner.next().unwrap().as_str();
                        for _property in inner {}
                    }
                    Rule::cf_exp_inst => add_exp_inst(inst, &mut nodes),
                    Rule::tex_clause => add_tex_clause(inst, &mut nodes),
                    Rule::alu_clause => add_alu_clause(inst, &mut nodes),
                    _ => (),
                }
            }
        }

        Self { nodes: nodes.nodes }
    }
}

fn add_exp_inst(inst: Pair<Rule>, nodes: &mut Nodes) {
    let mut inner = inst.into_inner();
    let inst_count: usize = inner.next().unwrap().as_str().parse().unwrap();
    let _op_code = inner.next().unwrap().as_str();

    let target = inner.next().unwrap();
    let (target_name, target_index) = exp_target(target);

    let source = inner.next().unwrap();
    let (source_name, source_index, channels) = exp_src(source).unwrap();

    let mut burst_count = 0;
    for property in inner {
        for inner in property.into_inner() {
            if inner.as_rule() == Rule::burstcnt {
                burst_count = inner.into_inner().next().unwrap().as_str().parse().unwrap();
            }
        }
    }

    // BURSTCNT assigns consecutive input and output registers.
    for i in 0..=burst_count {
        // TODO: use out_attr{i} for consistency with GLSL?
        for c in channels.chars() {
            let node = Node {
                output: Output {
                    name: format!("{target_name}{}", target_index + i),
                    channel: Some(c),
                },
                input: previous_assignment(
                    &format!("{source_name}{}", source_index + i),
                    Some(c),
                    nodes,
                ),
            };
            nodes.add_node(node, None, inst_count);
        }
    }
}

fn add_tex_clause(inst: Pair<Rule>, nodes: &mut Nodes) {
    let mut inner = inst.into_inner();
    let inst_count: usize = inner.next().unwrap().as_str().parse().unwrap();
    let _inst_type = inner.next().unwrap().as_str();
    let _properties = inner.next().unwrap().as_str();
    for tex_instruction in inner {
        let tex_nodes = tex_inst_node(tex_instruction, nodes).unwrap();
        for node in tex_nodes {
            nodes.add_node(node, None, inst_count);
        }
    }
}

struct AluScalar {
    alu_unit: char,
    op_code: String,
    output_modifier: Option<String>,
    output: Output,
    sources: Vec<Expr>,
}

impl AluScalar {
    fn from_pair(pair: Pair<Rule>, nodes: &Nodes, inst_count: usize, source_count: usize) -> Self {
        let mut inner = pair.into_inner();
        let alu_unit = inner.next().unwrap().as_str().chars().next().unwrap(); // xyzwt
        let op_code = inner.next().unwrap().as_str().to_string();

        // Optional modifier like /2 or *2
        let output_modifier = inner.peek().and_then(|p| {
            // Only advance the iterator if it's the expected type.
            if p.as_rule() == Rule::alu_output_modifier {
                Some(inner.next().unwrap().as_str().to_string())
            } else {
                None
            }
        });

        let output = alu_dst_output(inner.next().unwrap(), inst_count, alu_unit);
        let sources = inner
            .take(source_count)
            .map(|p| alu_src_expr(p, nodes))
            .collect();

        Self {
            alu_unit,
            op_code,
            output_modifier,
            output,
            sources,
        }
    }
}

fn add_alu_clause(inst: Pair<Rule>, nodes: &mut Nodes) {
    let mut inner = inst.into_inner();
    let _inst_count: usize = inner.next().unwrap().as_str().parse().unwrap();
    let _inst_type = inner.next().unwrap().as_str();
    let _properties = inner.next().unwrap().as_str();
    for group in inner {
        let mut inner = group.into_inner();
        let inst_count: usize = inner.next().unwrap().as_str().trim().parse().unwrap();

        let scalars: Vec<_> = inner
            .map(|alu_scalar| match alu_scalar.as_rule() {
                Rule::alu_scalar0 => AluScalar::from_pair(alu_scalar, nodes, inst_count, 0),
                Rule::alu_scalar1 => AluScalar::from_pair(alu_scalar, nodes, inst_count, 1),
                Rule::alu_scalar2 => AluScalar::from_pair(alu_scalar, nodes, inst_count, 2),
                Rule::alu_scalar3 => AluScalar::from_pair(alu_scalar, nodes, inst_count, 3),
                _ => unreachable!(),
            })
            .collect();

        let dot_node_index = dot_product_node_index(&scalars, inst_count, nodes);

        for scalar in scalars {
            if scalar.op_code.starts_with("DOT4") {
                // Dot products write the result to all vector components.
                if let Some(node_index) = dot_node_index {
                    let node = Node {
                        output: scalar.output,
                        input: Expr::Node {
                            node_index,
                            channel: None,
                        },
                    };
                    nodes.add_node(node, Some(scalar.alu_unit), inst_count);
                }
            } else {
                add_scalar(scalar, nodes, inst_count);
            }
        }
    }
}

fn dot_product_node_index(
    scalars: &[AluScalar],
    inst_count: usize,
    nodes: &mut Nodes,
) -> Option<usize> {
    let (dot4_a, dot4_b): (Vec<_>, Vec<_>) = scalars
        .iter()
        .filter_map(|s| {
            if s.op_code.starts_with("DOT4") {
                Some((s.sources[0].clone(), s.sources[1].clone()))
            } else {
                None
            }
        })
        .unzip();
    if !dot4_a.is_empty() && !dot4_b.is_empty() {
        let node = Node {
            output: Output {
                name: format!("temp{inst_count}"),
                channel: None,
            },
            input: Expr::Func {
                name: "dot".to_string(),
                args: vec![
                    Expr::Func {
                        name: "vec4".to_string(),
                        args: dot4_a,
                        channel: None,
                    },
                    Expr::Func {
                        name: "vec4".to_string(),
                        args: dot4_b,
                        channel: None,
                    },
                ],
                channel: None,
            },
        };
        let node_index = nodes.add_node(node, None, inst_count);
        Some(node_index)
    } else {
        None
    }
}

fn add_scalar(scalar: AluScalar, nodes: &mut Nodes, inst_count: usize) {
    let output = scalar.output.clone();
    let node_index = match scalar.op_code.as_str() {
        // scalar1
        "MOV" => {
            let node = Node {
                output,
                input: scalar.sources[0].clone(),
            };
            nodes.add_node(node, Some(scalar.alu_unit), inst_count)
        }
        "FLOOR" => add_func("floor", 1, &scalar, output, inst_count, nodes),
        "SQRT_IEEE" => add_func("sqrt", 1, &scalar, output, inst_count, nodes),
        "RECIP_IEEE" => {
            let node = Node {
                output,
                input: Expr::Binary(
                    BinaryOp::Div,
                    Box::new(Expr::Float(1.0)),
                    Box::new(scalar.sources[0].clone()),
                ),
            };
            nodes.add_node(node, Some(scalar.alu_unit), inst_count)
        }
        "RECIPSQRT_IEEE" => add_func("inversesqrt", 1, &scalar, output, inst_count, nodes),
        "EXP_IEEE" => add_func("exp2", 1, &scalar, output, inst_count, nodes),
        "LOG_CLAMPED" => add_func("log2", 1, &scalar, output, inst_count, nodes),
        // scalar2
        "ADD" => {
            let node = Node {
                output,
                input: Expr::Binary(
                    BinaryOp::Add,
                    Box::new(scalar.sources[0].clone()),
                    Box::new(scalar.sources[1].clone()),
                ),
            };
            nodes.add_node(node, Some(scalar.alu_unit), inst_count)
        }
        "MIN" | "MIN_DX10" => add_func("min", 2, &scalar, output, inst_count, nodes),
        "MAX" | "MAX_DX10" => add_func("max", 2, &scalar, output, inst_count, nodes),
        "MUL" | "MUL_IEEE" => {
            let node = Node {
                output,
                input: Expr::Binary(
                    BinaryOp::Mul,
                    Box::new(scalar.sources[0].clone()),
                    Box::new(scalar.sources[1].clone()),
                ),
            };
            nodes.add_node(node, Some(scalar.alu_unit), inst_count)
        }
        "DOT4" | "DOT4_IEEE" => {
            // Handled in a previous check.
            unreachable!()
        }
        // scalar3
        "MULADD" | "MULADD_IEEE" => add_func("fma", 3, &scalar, output, inst_count, nodes),
        "MULADD_D2" => {
            let input = Expr::Func {
                name: "fma".to_string(),
                args: vec![
                    scalar.sources[0].clone(),
                    scalar.sources[1].clone(),
                    scalar.sources[2].clone(),
                ],
                channel: None,
            };
            let node = Node {
                output: output.clone(),
                input,
            };
            let node_index = nodes.add_node(node, Some(scalar.alu_unit), inst_count);

            let node = Node {
                output,
                input: Expr::Binary(
                    BinaryOp::Div,
                    Box::new(Expr::Node {
                        node_index,
                        channel: scalar.output.channel,
                    }),
                    Box::new(Expr::Float(2.0)),
                ),
            };
            nodes.add_node(node, Some(scalar.alu_unit), inst_count)
        }
        "NOP" => 0,
        // TODO: Handle additional opcodes?
        _ => 0,
    };

    if let Some(modifier) = scalar.output_modifier {
        let node = alu_output_modifier(&modifier, scalar.output, node_index);
        nodes.add_node(node, Some(scalar.alu_unit), inst_count);
    }
}

fn add_func(
    func: &str,
    arg_count: usize,
    scalar: &AluScalar,
    output: Output,
    inst_count: usize,
    nodes: &mut Nodes,
) -> usize {
    let node = Node {
        output,
        input: Expr::Func {
            name: func.to_string(),
            args: (0..arg_count).map(|i| scalar.sources[i].clone()).collect(),
            channel: None,
        },
    };
    nodes.add_node(node, Some(scalar.alu_unit), inst_count)
}

fn alu_dst_output(pair: Pair<Rule>, inst_count: usize, alu_unit: char) -> Output {
    let mut inner = pair.into_inner();
    if inner.peek().map(|p| p.as_rule()) == Some(Rule::write_mask) {
        // ____ mask for xyzw writes to a previous vector "PV".
        // ____ mask for t writes to a previous scalar "PS".
        match alu_unit {
            'x' => Output {
                name: format!("PV{inst_count}"),
                channel: Some('x'),
            },
            'y' => Output {
                name: format!("PV{inst_count}"),
                channel: Some('y'),
            },
            'z' => Output {
                name: format!("PV{inst_count}"),
                channel: Some('z'),
            },
            'w' => Output {
                name: format!("PV{inst_count}"),
                channel: Some('w'),
            },
            't' => Output {
                name: format!("PS{inst_count}"),
                channel: None,
            },
            _ => unreachable!(),
        }
    } else {
        let gpr = inner.next().unwrap().as_str();
        if inner.peek().map(|p| p.as_rule()) == Some(Rule::tex_rel) {
            inner.next().unwrap();
        }
        let channel = one_comp_swizzle(inner);
        Output {
            name: gpr.to_string(),
            channel,
        }
    }
}

fn alu_output_modifier(modifier: &str, output: Output, node_index: usize) -> Node {
    let channel = output.channel;
    match modifier {
        "/2" => Node {
            output,
            input: Expr::Binary(
                BinaryOp::Div,
                Box::new(Expr::Node {
                    node_index,
                    channel,
                }),
                Box::new(Expr::Float(2.0)),
            ),
        },
        "/4" => Node {
            output,
            input: Expr::Binary(
                BinaryOp::Div,
                Box::new(Expr::Node {
                    node_index,
                    channel,
                }),
                Box::new(Expr::Float(4.0)),
            ),
        },
        "*2" => Node {
            output,
            input: Expr::Binary(
                BinaryOp::Mul,
                Box::new(Expr::Node {
                    node_index,
                    channel,
                }),
                Box::new(Expr::Float(2.0)),
            ),
        },
        "*4" => Node {
            output,
            input: Expr::Binary(
                BinaryOp::Mul,
                Box::new(Expr::Node {
                    node_index,
                    channel,
                }),
                Box::new(Expr::Float(4.0)),
            ),
        },
        _ => panic!("unexpected modifier: {modifier}"),
    }
}

fn alu_src_expr(source: Pair<Rule>, nodes: &Nodes) -> Expr {
    let mut inner = source.into_inner();
    let negate = inner
        .peek()
        .map(|p| {
            // Only advance the iterator if it's the expected type.
            if p.as_rule() == Rule::negate {
                inner.next().unwrap();
                true
            } else {
                false
            }
        })
        .unwrap_or_default();

    let value = inner.next().unwrap();
    let alu_src_value = value.into_inner().next().unwrap();

    if inner.peek().map(|p| p.as_rule()) == Some(Rule::alu_rel) {
        inner.next().unwrap();
    }

    let channel = one_comp_swizzle(inner);

    let expr = match alu_src_value.as_rule() {
        Rule::literal => {
            let mut inner = alu_src_value.into_inner();
            let a = inner.next().unwrap();
            let b = inner.next();
            let value = match (a.as_rule(), b.as_ref().map(|b| b.as_rule())) {
                (Rule::hex_number, None) => a.as_str().parse().unwrap(),
                (Rule::float, None) => a.as_str().trim_end_matches('f').parse().unwrap(),
                (Rule::hex_number, Some(Rule::float)) => {
                    // Extract the non hex portion from a float literal.
                    b.unwrap().as_str().parse().unwrap()
                }
                _ => unreachable!(),
            };
            Expr::Float(value)
        }
        Rule::constant_cache0 => {
            let mut inner = alu_src_value.into_inner();
            let number = inner.next().unwrap().as_str().parse().unwrap();
            Expr::Parameter {
                name: "KC0".to_string(),
                field: None,
                index: Box::new(Expr::Int(number)),
                channel,
            }
        }
        Rule::constant_cache1 => {
            let mut inner = alu_src_value.into_inner();
            let number = inner.next().unwrap().as_str().parse().unwrap();
            Expr::Parameter {
                name: "KC1".to_string(),
                field: None,
                index: Box::new(Expr::Int(number)),
                channel,
            }
        }
        _ => {
            // Find a previous assignment that modifies the desired channel.
            let name = alu_src_value.as_str();
            previous_assignment(name, channel, nodes)
        }
    };

    if negate {
        Expr::Unary(UnaryOp::Negate, Box::new(expr))
    } else {
        expr
    }
}

fn previous_assignment(value: &str, channel: Option<char>, nodes: &Nodes) -> Expr {
    // PV can also refer to an actual register if not all outputs were masked.
    if value.starts_with("PV") {
        let inst_count: usize = value.split_once("PV").unwrap().1.parse().unwrap();

        nodes
            .node_index_alu_unit_inst_count
            .iter()
            .find_map(|(n, alu, i)| {
                if *i == inst_count && *alu == channel {
                    Some(Expr::Node {
                        node_index: *n,
                        channel: nodes.nodes[*n].output.channel,
                    })
                } else {
                    None
                }
            })
            .unwrap_or(Expr::Global {
                name: value.to_string(),
                channel,
            })
    } else if value.starts_with("PS") {
        let inst_count: usize = value.split_once("PS").unwrap().1.parse().unwrap();

        nodes
            .node_index_alu_unit_inst_count
            .iter()
            .find_map(|(n, alu, i)| {
                if *i == inst_count && *alu == Some('t') {
                    Some(Expr::Node {
                        node_index: *n,
                        channel: nodes.nodes[*n].output.channel,
                    })
                } else {
                    None
                }
            })
            .unwrap_or(Expr::Global {
                name: value.to_string(),
                channel,
            })
    } else {
        nodes
            .nodes
            .iter()
            .rposition(|n| n.output.name == value && n.output.channel == channel)
            .map(|node_index| Expr::Node {
                node_index,
                channel,
            })
            .unwrap_or(Expr::Global {
                name: value.to_string(),
                channel,
            })
    }
}

fn one_comp_swizzle(mut inner: pest::iterators::Pairs<Rule>) -> Option<char> {
    inner.peek().and_then(|p| {
        // Only advance the iterator if it's the expected type.
        if matches!(p.as_rule(), Rule::one_comp_swizzle) {
            Some(
                inner
                    .next()
                    .unwrap()
                    .as_str()
                    .trim_start_matches('.')
                    .chars()
                    .next()
                    .unwrap(),
            )
        } else {
            None
        }
    })
}

fn four_comp_swizzle(mut inner: pest::iterators::Pairs<Rule>) -> &str {
    inner
        .peek()
        .and_then(|p| {
            // Only advance the iterator if it's the expected type.
            if matches!(p.as_rule(), Rule::four_comp_swizzle) {
                Some(inner.next().unwrap().as_str().trim_start_matches('.'))
            } else {
                None
            }
        })
        .unwrap_or_default()
}

fn exp_src(source: Pair<Rule>) -> Option<(&'static str, usize, &str)> {
    let mut inner = source.into_inner();
    let name_pair = inner.next()?;
    let name = match name_pair.as_rule() {
        Rule::gpr => "R",
        Rule::gpr_rel => todo!(),
        _ => unreachable!(),
    };
    let index = name_pair.into_inner().next()?.as_str().parse().unwrap();
    let channels = four_comp_swizzle(inner);

    Some((name, index, channels))
}

fn exp_target(target: Pair<Rule>) -> (&'static str, usize) {
    let target_name = match target.as_rule() {
        Rule::exp_pix_target => "PIX",
        Rule::exp_pos_target => "POS",

        Rule::exp_param_target => "PARAM",
        _ => unreachable!(),
    };
    let target_index: usize = target
        .into_inner()
        .next()
        .unwrap()
        .as_str()
        .parse()
        .unwrap();
    (target_name, target_index)
}

fn tex_inst_node(tex_instruction: Pair<Rule>, nodes: &Nodes) -> Option<Vec<Node>> {
    let mut inner = tex_instruction.into_inner();
    // TODO: why does this have trailing white space?
    let _inst_count = inner.next()?.as_str();

    // TODO: Check that this is SAMPLE?
    let _op_code = inner.next()?.as_str();

    // TODO: Get the input names and channels.
    // TODO: register or mask?
    let dest = inner.next()?;
    let (output_name, output_channels) = texture_inst_dest(dest)?;

    let src = inner.next()?;
    let texcoords = texture_inst_src(src, nodes)?;

    let texture = inner.next()?.as_str();
    let _sampler = inner.next()?.as_str();
    // TODO: always ignore properties?

    let texture_name = Expr::Global {
        name: texture.to_string(),
        channel: None,
    };

    if output_channels.is_empty() {
        Some(vec![Node {
            output: Output {
                name: output_name,
                channel: None,
            },
            input: Expr::Func {
                name: "texture".to_string(),
                args: vec![texture_name, texcoords],
                channel: None,
            },
        }])
    } else {
        // Convert vector swizzles to scalar operations to simplify analysis code.
        Some(
            output_channels
                .chars()
                .map(|c| Node {
                    output: Output {
                        name: output_name.clone(),
                        channel: Some(c),
                    },
                    input: Expr::Func {
                        name: "texture".to_string(),
                        args: vec![texture_name.clone(), texcoords.clone()],
                        channel: Some(c),
                    },
                })
                .collect(),
        )
    }
}

fn texture_inst_dest(dest: Pair<Rule>) -> Option<(String, String)> {
    // TODO: Handle other cases from grammar.
    let mut inner = dest.into_inner();
    let gpr = inner.next()?.as_str();
    if inner.peek().map(|p| p.as_rule()) == Some(Rule::tex_rel) {
        inner.next().unwrap();
    }
    let channels = four_comp_swizzle(inner);

    Some((gpr.to_string(), channels.trim_matches('_').to_string()))
}

fn texture_inst_src(dest: Pair<Rule>, nodes: &Nodes) -> Option<Expr> {
    // TODO: Handle other cases from grammar.
    let mut inner = dest.into_inner();
    let gpr = inner.next()?.as_str();
    if inner.peek().map(|p| p.as_rule()) == Some(Rule::tex_rel) {
        inner.next().unwrap();
    }
    let mut channels = four_comp_swizzle(inner).chars();

    // TODO: Also handle cube maps.
    Some(Expr::Func {
        name: "vec2".to_string(),
        args: vec![
            previous_assignment(gpr, channels.next(), nodes),
            previous_assignment(gpr, channels.next(), nodes),
        ],
        channel: None,
    })
}

// Grammar adapted from the cpp-peglib grammer used for decaf-emu:
// https://github.com/decaf-emu/decaf-emu/blob/master/tools/latte-assembler/resources/grammar.txt
#[derive(Parser)]
#[grammar = "graph/latte.pest"]
struct LatteParser;

#[cfg(test)]
mod tests {
    use super::*;

    use pretty_assertions::assert_eq;

    #[test]
    fn graph_from_asm_pc221115_frag_0() {
        // Elma's legs (visible on title screen).
        let asm = include_str!("../data/pc221115.0.frag.txt");
        let expected = include_str!("../data/pc221115.0.frag");

        // TODO: Figure out the expected nodes to test previous node references.
        // TODO: Test expected nodes on a handwritten example?
        let graph = Graph::from_latte_asm(asm);
        assert_eq!(expected, graph.to_glsl());
    }
}
