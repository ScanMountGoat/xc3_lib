use pest::{iterators::Pair, Parser};
use pest_derive::Parser;

use super::*;

#[derive(Default)]
struct Nodes {
    nodes: Vec<Node>,
    node_index_alu_unit_inst_count: Vec<(usize, String, usize)>,
}

impl Nodes {
    fn add_node(&mut self, node: Node, alu_unit: &str, inst_count: usize) -> usize {
        let index = self.nodes.len();
        self.nodes.push(node);
        self.node_index_alu_unit_inst_count
            .push((index, alu_unit.to_string(), inst_count));
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
                        let inst_count: usize = inner.next().unwrap().as_str().parse().unwrap();
                        let op_code = inner.next().unwrap().as_str();
                        for property in inner {}
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
    let op_code = inner.next().unwrap().as_str();

    let target = inner.next().unwrap();
    let (target_name, target_index) = exp_target(target);

    let source = inner.next().unwrap();
    let (source_name, source_index, channels) = exp_src(source).unwrap();

    let mut burst_count = 1;
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
        for c in channels.unwrap_or_default().chars() {
            let node = Node {
                output: Output {
                    name: format!("{target_name}{}", target_index + i),
                    channels: c.to_string(),
                },
                input: previous_assignment(
                    &format!("{source_name}{}", source_index + i),
                    &c.to_string(),
                    nodes,
                ),
            };
            nodes.add_node(node, "", inst_count);
        }
    }
}

fn add_tex_clause(inst: Pair<Rule>, nodes: &mut Nodes) {
    let mut inner = inst.into_inner();
    let inst_count: usize = inner.next().unwrap().as_str().parse().unwrap();
    let _inst_type = inner.next().unwrap().as_str();
    let properties = inner.next().unwrap().as_str();
    for tex_instruction in inner {
        let node = tex_inst_node(tex_instruction, nodes).unwrap();
        nodes.add_node(node, "", inst_count);
    }
}

struct AluScalar {
    alu_unit: String,
    op_code: String,
    output_modifier: Option<String>,
    output: Output,
    sources: Vec<Expr>,
}

impl AluScalar {
    fn from_pair(pair: Pair<Rule>, nodes: &Nodes, inst_count: usize, source_count: usize) -> Self {
        let mut inner = pair.into_inner();
        let alu_unit = inner.next().unwrap().as_str().to_string(); // xyzwt
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

        let output = alu_dst_output(inner.next().unwrap(), inst_count, &alu_unit);
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
    let inst_count: usize = inner.next().unwrap().as_str().parse().unwrap();
    let _inst_type = inner.next().unwrap().as_str();
    let properties = inner.next().unwrap().as_str();
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
                            channels: String::new(),
                        },
                    };
                    nodes.add_node(node, &scalar.alu_unit, inst_count);
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
                channels: String::new(),
            },
            input: Expr::Func {
                name: "dot".to_string(),
                args: vec![
                    Expr::Func {
                        name: "vec4".to_string(),
                        args: dot4_a,
                        channels: String::new(),
                    },
                    Expr::Func {
                        name: "vec4".to_string(),
                        args: dot4_b,
                        channels: String::new(),
                    },
                ],
                channels: String::new(),
            },
        };
        let node_index = nodes.add_node(node, "", inst_count);
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
            nodes.add_node(node, &scalar.alu_unit, inst_count)
        }
        "FLOOR" => add_func("floor", 1, &scalar, output, inst_count, nodes),
        "SQRT_IEEE" => add_func("sqrt", 1, &scalar, output, inst_count, nodes),
        "RECIP_IEEE" => {
            let node = Node {
                output,
                input: Expr::Div(
                    Box::new(Expr::Float(1.0)),
                    Box::new(scalar.sources[0].clone()),
                ),
            };
            nodes.add_node(node, &scalar.alu_unit, inst_count)
        }
        "RECIPSQRT_IEEE" => add_func("inversesqrt", 1, &scalar, output, inst_count, nodes),
        "EXP_IEEE" => add_func("exp2", 1, &scalar, output, inst_count, nodes),
        "LOG_CLAMPED" => add_func("log2", 1, &scalar, output, inst_count, nodes),
        // scalar2
        "ADD" => {
            let node = Node {
                output,
                input: Expr::Add(
                    Box::new(scalar.sources[0].clone()),
                    Box::new(scalar.sources[1].clone()),
                ),
            };
            nodes.add_node(node, &scalar.alu_unit, inst_count)
        }
        "MIN" | "MIN_DX10" => add_func("min", 2, &scalar, output, inst_count, nodes),
        "MAX" | "MAX_DX10" => add_func("max", 2, &scalar, output, inst_count, nodes),
        "MUL" | "MUL_IEEE" => {
            let node = Node {
                output,
                input: Expr::Mul(
                    Box::new(scalar.sources[0].clone()),
                    Box::new(scalar.sources[1].clone()),
                ),
            };
            nodes.add_node(node, &scalar.alu_unit, inst_count)
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
                channels: String::new(),
            };
            let node = Node {
                output: output.clone(),
                input,
            };
            let node_index = nodes.add_node(node, &scalar.alu_unit, inst_count);

            let node = Node {
                output,
                input: Expr::Div(
                    Box::new(Expr::Node {
                        node_index,
                        channels: scalar.output.channels.clone(),
                    }),
                    Box::new(Expr::Float(2.0)),
                ),
            };
            nodes.add_node(node, &scalar.alu_unit, inst_count)
        }
        "NOP" => 0,
        // TODO: Handle additional opcodes?
        _ => 0,
    };

    if let Some(modifier) = scalar.output_modifier {
        let node = alu_output_modifier(&modifier, scalar.output, node_index);
        nodes.add_node(node, &scalar.alu_unit, inst_count);
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
            channels: String::new(),
        },
    };
    nodes.add_node(node, &scalar.alu_unit, inst_count)
}

fn alu_dst_output(pair: Pair<Rule>, inst_count: usize, alu_unit: &str) -> Output {
    let mut inner = pair.into_inner();
    if inner.peek().map(|p| p.as_rule()) == Some(Rule::write_mask) {
        // ____ mask for xyzw writes to a previous vector "PV".
        // ____ mask for t writes to a previous scalar "PS".
        match alu_unit {
            "x" => Output {
                name: format!("PV{inst_count}"),
                channels: "x".to_string(),
            },
            "y" => Output {
                name: format!("PV{inst_count}"),
                channels: "y".to_string(),
            },
            "z" => Output {
                name: format!("PV{inst_count}"),
                channels: "z".to_string(),
            },
            "w" => Output {
                name: format!("PV{inst_count}"),
                channels: "w".to_string(),
            },
            "t" => Output {
                name: format!("PS{inst_count}"),
                channels: String::new(),
            },
            _ => unreachable!(),
        }
    } else {
        let gpr = inner.next().unwrap().as_str();
        if inner.peek().map(|p| p.as_rule()) == Some(Rule::tex_rel) {
            inner.next().unwrap();
        }
        let channels = comp_swizzle(inner);
        Output {
            name: gpr.to_string(),
            channels: channels.to_string(),
        }
    }
}

fn alu_output_modifier(modifier: &str, output: Output, node_index: usize) -> Node {
    let channels = output.channels.clone();
    match modifier {
        "/2" => Node {
            output,
            input: Expr::Div(
                Box::new(Expr::Node {
                    node_index,
                    channels,
                }),
                Box::new(Expr::Float(2.0)),
            ),
        },
        "/4" => Node {
            output,
            input: Expr::Div(
                Box::new(Expr::Node {
                    node_index,
                    channels,
                }),
                Box::new(Expr::Float(4.0)),
            ),
        },
        "*2" => Node {
            output,
            input: Expr::Mul(
                Box::new(Expr::Node {
                    node_index,
                    channels,
                }),
                Box::new(Expr::Float(2.0)),
            ),
        },
        "*4" => Node {
            output,
            input: Expr::Mul(
                Box::new(Expr::Node {
                    node_index,
                    channels,
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
    let value = match alu_src_value.as_rule() {
        Rule::literal => {
            let mut inner = alu_src_value.into_inner();
            let a = inner.next().unwrap();
            let b = inner.next();
            match (a.as_rule(), b.as_ref().map(|b| b.as_rule())) {
                (Rule::hex_number, None) => a.as_str(),
                (Rule::float, None) => a.as_str().trim_end_matches('f'),
                (Rule::hex_number, Some(Rule::float)) => {
                    // Extract the non hex portion from a float literal.
                    b.unwrap().as_str()
                }
                _ => unreachable!(),
            }
        }
        _ => alu_src_value.as_str(),
    };

    if inner.peek().map(|p| p.as_rule()) == Some(Rule::alu_rel) {
        inner.next().unwrap();
    }

    let channels = comp_swizzle(inner);

    // Find a previous assignment that modifies the desired channel.
    let expr = previous_assignment(value, channels, nodes);

    if negate {
        Expr::Negate(Box::new(expr))
    } else {
        expr
    }
}

fn previous_assignment(value: &str, channels: &str, nodes: &Nodes) -> Expr {
    // PV can also refer to an actual register if not all outputs were masked.
    if value.starts_with("PV") {
        let inst_count: usize = value.split_once("PV").unwrap().1.parse().unwrap();

        nodes
            .node_index_alu_unit_inst_count
            .iter()
            .find_map(|(n, alu, i)| {
                if *i == inst_count && alu == channels {
                    Some(Expr::Node {
                        node_index: *n,
                        channels: nodes.nodes[*n].output.channels.clone(),
                    })
                } else {
                    None
                }
            })
            .unwrap_or(Expr::Global {
                name: value.to_string(),
                channels: channels.to_string(),
            })
    } else if value.starts_with("PS") {
        let inst_count: usize = value.split_once("PS").unwrap().1.parse().unwrap();

        nodes
            .node_index_alu_unit_inst_count
            .iter()
            .find_map(|(n, alu, i)| {
                if *i == inst_count && alu == "t" {
                    Some(Expr::Node {
                        node_index: *n,
                        channels: nodes.nodes[*n].output.channels.clone(),
                    })
                } else {
                    None
                }
            })
            .unwrap_or(Expr::Global {
                name: value.to_string(),
                channels: channels.to_string(),
            })
    } else {
        nodes
            .nodes
            .iter()
            .rposition(|n| n.output.name == value && n.output.contains_channels(channels))
            .map(|node_index| Expr::Node {
                node_index,
                channels: channels.to_string(),
            })
            .unwrap_or(Expr::Global {
                name: value.to_string(),
                channels: channels.to_string(),
            })
    }
}

fn comp_swizzle(mut inner: pest::iterators::Pairs<Rule>) -> &str {
    inner
        .peek()
        .and_then(|p| {
            // Only advance the iterator if it's the expected type.
            if matches!(
                p.as_rule(),
                Rule::one_comp_swizzle | Rule::four_comp_swizzle
            ) {
                Some(inner.next().unwrap().as_str().trim_start_matches('.'))
            } else {
                None
            }
        })
        .unwrap_or_default()
}

fn exp_src(source: Pair<Rule>) -> Option<(&'static str, usize, Option<&str>)> {
    let mut source_inner = source.into_inner();
    let name_pair = source_inner.next()?;
    let name = match name_pair.as_rule() {
        Rule::gpr => "R",
        Rule::gpr_rel => todo!(),
        _ => unreachable!(),
    };
    let index = name_pair.into_inner().next()?.as_str().parse().unwrap();

    let channels = source_inner.next().and_then(|p| {
        if p.as_rule() == Rule::four_comp_swizzle {
            Some(p.as_str().trim_start_matches('.'))
        } else {
            None
        }
    });

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

fn tex_inst_node(tex_instruction: Pair<Rule>, nodes: &Nodes) -> Option<Node> {
    let mut inner = tex_instruction.into_inner();
    // TODO: why does this have trailing white space?
    let inst_count = inner.next()?.as_str();

    // TODO: Check that this is SAMPLE?
    let op_code = inner.next()?.as_str();

    // TODO: Get the input names and channels.
    // TODO: register or mask?
    let dest = inner.next()?;
    let output = texture_inst_dest(dest)?;

    let channels = output.channels.clone();

    let src = inner.next()?;
    let texcoords = texture_inst_src(src, nodes)?;

    let texture = inner.next()?.as_str();
    let sampler = inner.next()?.as_str();
    // TODO: always ignore properties?

    let texture_name = Expr::Global {
        name: texture.to_string(),
        channels: String::new(),
    };

    Some(Node {
        output,
        input: Expr::Func {
            name: "texture".to_string(),
            args: vec![texture_name, texcoords],
            channels,
        },
    })
}

fn texture_inst_dest(dest: Pair<Rule>) -> Option<Output> {
    // TODO: Handle other cases from grammar.
    let mut inner = dest.into_inner();
    let gpr = inner.next()?.as_str();
    if inner.peek().map(|p| p.as_rule()) == Some(Rule::tex_rel) {
        inner.next().unwrap();
    }
    let channels = comp_swizzle(inner);
    Some(Output {
        name: gpr.to_string(),
        channels: channels.trim_matches('_').to_string(),
    })
}

fn texture_inst_src(dest: Pair<Rule>, nodes: &Nodes) -> Option<Expr> {
    // TODO: Handle other cases from grammar.
    let mut inner = dest.into_inner();
    let gpr = inner.next()?.as_str();
    if inner.peek().map(|p| p.as_rule()) == Some(Rule::tex_rel) {
        inner.next().unwrap();
    }
    let channels = comp_swizzle(inner);

    // TODO: Also handle cube maps.
    Some(Expr::Func {
        name: "vec2".to_string(),
        args: vec![
            previous_assignment(gpr, &channels.chars().next().unwrap().to_string(), nodes),
            previous_assignment(gpr, &channels.chars().nth(1).unwrap().to_string(), nodes),
        ],
        channels: String::new(),
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

    use indoc::indoc;
    use pretty_assertions::assert_eq;

    #[test]
    fn graph_from_asm_pc221115_frag_0() {
        // Elma's legs (visible on title screen).
        let asm = indoc! {"
            00 TEX: ADDR(208) CNT(4)

            0      SAMPLE          R2.xy__, R6.xy0x, t3, s3

            1      SAMPLE          R8.xyz_, R6.xy0x, t2, s2

            2      SAMPLE          R7.xyz_, R6.xy0x, t1, s1

            3      SAMPLE          R6.xyz_, R6.xy0x, t4, s4

            01 ALU: ADDR(32) CNT(127) KCACHE0(CB1:0-15)
            4   x: MULADD          R125.x, R2.x, (0x40000000, 2), -1.0f
                y: MULADD          R126.y, R2.y, (0x40000000, 2), -1.0f
                z: MOV             ____, 0.0f
                w: MUL             R124.w, R2.z, (0x41000000, 8)
                t: SQRT_IEEE       ____, R5.w SCL_210

            5   x: DOT4            ____, PV4.x, PV4.x
                y: DOT4            ____, PV4.y, PV4.y
                z: DOT4            ____, PV4.z, PV4.y
                w: DOT4            ____, (0x80000000, -0), 0.0f
                t: ADD             R0.w, -PS4, 1.0f CLAMP

            6   x: DOT4_IEEE       ____, R5.x, R5.x
                y: DOT4_IEEE       ____, R5.y, R5.y
                z: DOT4_IEEE       ____, R5.z, R5.z
                w: DOT4_IEEE       ____, (0x80000000, -0), 0.0f
                t: ADD             R127.w, -PV5.x, 1.0f

            7   x: DOT4_IEEE       ____, R3.x, R3.x
                y: DOT4_IEEE       R127.y, R3.y, R3.y
                z: DOT4_IEEE       ____, R3.z, R3.z
                w: DOT4_IEEE       ____, (0x80000000, -0), 0.0f
                t: RECIPSQRT_IEEE  ____, PV6.x SCL_210

            8   x: MUL             R127.x, R5.x, PS7
                y: FLOOR           R125.y, R124.w
                z: MUL             R126.z, R5.z, PS7
                w: MUL             R127.w, R5.y, PS7
                t: SQRT_IEEE       R127.z, R127.w SCL_210

            9   x: DOT4_IEEE       ____, R0.x, R0.x
                y: DOT4_IEEE       ____, R0.y, R0.y
                z: DOT4_IEEE       ____, R0.z, R0.z
                w: DOT4_IEEE       ____, (0x80000000, -0), 0.0f
                t: RECIPSQRT_IEEE  ____, R127.y SCL_210

            10  x: MUL             R126.x, R3.z, PS9
                y: MAX             ____, R127.z, 0.0f VEC_120
                z: MUL             R127.z, R3.y, PS9
                w: MUL             R126.w, R3.x, PS9
                t: RECIPSQRT_IEEE  R125.w, PV9.x SCL_210

            11  x: MUL             ____, R126.z, PV10.y
                y: MUL             ____, R127.w, PV10.y
                z: MUL             R126.z, R0.x, PS10
                w: MUL             ____, R127.x, PV10.y VEC_120
                t: MUL             R127.y, R0.y, PS10

            12  x: MUL             ____, R0.z, R125.w
                y: MULADD          R123.y, R126.x, R125.x, PV11.x
                z: MULADD          R123.z, R126.w, R125.x, PV11.w
                w: MULADD          R123.w, R127.z, R125.x, PV11.y VEC_120
                t: MUL             R124.y, R125.y, (0x3B808081, 0.003921569)

            13  x: MULADD          R126.x, R126.z, R126.y, PV12.z
                y: MULADD          R127.y, R127.y, R126.y, PV12.w
                z: MULADD          R126.z, PV12.x, R126.y, PV12.y
                w: FLOOR           R126.w, PS12
                t: MOV             R2.w, 0.0f

            14  x: DOT4_IEEE       ____, R1.x, R1.x
                y: DOT4_IEEE       ____, R1.y, R1.y
                z: DOT4_IEEE       ____, R1.z, R1.z
                w: DOT4_IEEE       ____, (0x80000000, -0), 0.0f
                t: MOV             R6.w, KC0[1].x

            15  x: DOT4_IEEE       ____, R126.x, R126.x
                y: DOT4_IEEE       ____, R127.y, R127.y
                z: DOT4_IEEE       ____, R126.z, R126.z
                w: DOT4_IEEE       ____, (0x80000000, -0), 0.0f
                t: RECIPSQRT_IEEE  ____, PV14.x SCL_210

            16  x: MUL             R125.x, R1.x, PS15
                y: MUL             R126.y, R1.y, PS15
                z: MUL             R127.z, R1.z, PS15
                w: MOV             R5.w, R8.z VEC_120
                t: RECIPSQRT_IEEE  ____, PV15.x SCL_210

            17  x: MUL             R126.x, R126.x, PS16
                y: MUL             R127.y, R127.y, PS16
                z: MUL             R126.z, R126.z, PS16
                w: MUL_IEEE        ____, R4.z, R4.z VEC_120
                t: ADD             R5.x, R124.w, -R125.y

            18  x: DOT4            ____, R125.x, PV17.x
                y: DOT4            ____, R126.y, PV17.y
                z: DOT4            ____, R127.z, PV17.z
                w: DOT4            ____, (0x80000000, -0), 0.0f
                t: MULADD_IEEE     R122.x, R4.y, R4.y, PV17.w

            19  x: MULADD_IEEE     R123.x, R4.x, R4.x, PS18
                y: MUL             ____, PV18.x, R0.w VEC_021
                z: MUL             R5.z, R126.w, (0x3B808081, 0.003921569)
                t: ADD             R5.y, R124.y, -R126.w

            20  x: MULADD          R126.x, -R125.x, PV19.y, R126.x
                y: MULADD          R127.y, -R126.y, PV19.y, R127.y
                z: MULADD          R127.z, -R127.z, PV19.y, R126.z
                t: RECIPSQRT_IEEE  R126.w, PV19.x SCL_210

            21  x: DOT4_IEEE       ____, PV20.x, PV20.x
                y: DOT4_IEEE       ____, PV20.y, PV20.y
                z: DOT4_IEEE       ____, PV20.z, PV20.z
                w: DOT4_IEEE       ____, (0x80000000, -0), 0.0f
                t: MUL             R1.x, R4.x, PS20

            22  y: MUL             R1.y, R4.y, R126.w
                z: MUL             R126.z, R4.z, R126.w
                t: RECIPSQRT_IEEE  ____, PV21.x SCL_210

            23  x: MUL             R4.x, R126.x, PS22
                y: MUL             R4.y, R127.y, PS22
                z: MUL             R127.z, R127.z, PS22

            24  x: MOV             R9.x, PV23.x
                y: ADD/2           ____, PV23.y, 1.0f
                z: ADD/2           ____, PV23.x, 1.0f
                w: MOV             R9.w, PV23.y
                t: MUL             ____, -R126.z, PV23.z

            25  x: ADD             ____, -PV24.y, 1.0f
                y: MUL             ____, R126.z, R127.z
                w: MAX             ____, PV24.z, 0.0f
                t: MULADD          R122.x, -R1.y, R4.y, PS24

            26  x: MULADD          R123.x, -R1.x, R4.x, PS25
                z: MAX             ____, PV25.x, 0.0f
                w: MIN             R3.w, PV25.w, 1.0f
                t: MULADD          R122.x, R1.y, R4.y, PV25.y

            27  y: MIN             R3.y, PV26.z, 1.0f
                z: ADD             R4.z, PV26.x, PV26.x
                w: MULADD          R0.w, R1.x, R4.x, PS26

            28  x: MULADD_D2       R123.x, -R4.z, R4.x, -R1.x
                y: MAX_DX10        ____, R0.w, -R0.w
                w: MULADD_D2       R123.w, -R4.z, R4.y, -R1.y

            29  x: ADD             R1.x, PV28.x, 0.5f
                y: ADD             R1.y, PV28.w, 0.5f
                z: ADD             R4.z, -PV28.y, 1.0f CLAMP

            02 TEX: ADDR(216) CNT(2) VALID_PIX

            30     SAMPLE          R3.xyzw, R3.wy0w, t0, s0

            31     SAMPLE          R1.xyz_, R1.xy0x, t5, s5

            03 ALU: ADDR(159) CNT(40) KCACHE0(CB1:0-15)
            32  x: MULADD          R126.x, KC0[0].z, R3.z, 0.0f
                y: MULADD          R127.y, KC0[0].y, R3.y, 0.0f
                z: MULADD          R123.z, KC0[0].w, R3.w, 0.0f
                w: MULADD          R126.w, KC0[0].x, R3.x, 0.0f
                t: LOG_CLAMPED     ____, R4.z SCL_210

            33  x: MULADD          R2.x, R8.x, R1.x, R7.x
                y: MULADD          R2.y, R8.x, R1.y, R7.y
                z: MULADD          R2.z, R8.x, R1.z, R7.z
                w: MUL             ____, KC0[2].w, PS32
                t: MOV/2           R1.w, PV32.z

            34  t: EXP_IEEE        ____, PV33.w SCL_210

            35  x: MULADD          R123.x, KC0[2].x, PS34, R126.w
                z: MULADD          R123.z, KC0[2].y, PS34, R127.y
                w: MULADD          R123.w, KC0[2].z, PS34, R126.x

            36  x: MUL             ____, R8.y, PV35.z
                y: MUL             ____, R8.y, PV35.x
                z: MUL             ____, R8.y, PV35.w

            37  x: MOV/2           R1.x, PV36.y
                y: MOV/2           R1.y, PV36.x
                z: MOV/2           R1.z, PV36.z

            38  x: MOV             R14.x, R5.x
                y: MOV             R14.y, R5.y
                z: MOV             R14.z, R5.z
                w: MOV             R14.w, R5.w

            39  x: MOV             R13.x, R6.x
                y: MOV             R13.y, R6.y
                z: MOV             R13.z, R6.z
                w: MOV             R13.w, R6.w

            40  x: MOV             R11.x, R2.x
                y: MOV             R11.y, R2.y
                z: MOV             R11.z, R2.z
                w: MOV             R11.w, R2.w

            41  x: MOV             R10.x, R1.x
                y: MOV             R10.y, R1.y
                z: MOV             R10.z, R1.z
                w: MOV             R10.w, R1.w

            42  x: MOV             R12.x, R9.x
                y: MOV             R12.y, R9.w
                z: MOV             R12.z, R9.z
                w: MOV             R12.w, R9.z

            04 EXP_DONE: PIX0, R10.xyzw BURSTCNT(4)

            END_OF_PROGRAM
        "};

        let expected = indoc! {"
            R2.xy = texture(t3, vec2(R6.x, R6.y)).xy;
            R8.xyz = texture(t2, vec2(R6.x, R6.y)).xyz;
            R7.xyz = texture(t1, vec2(R6.x, R6.y)).xyz;
            R6.xyz = texture(t4, vec2(R6.x, R6.y)).xyz;
            R125.x = fma(R2.x, 2, -1.0);
            R126.y = fma(R2.y, 2, -1.0);
            PV4.z = 0.0;
            R124.w = R2.z * 8;
            PS4 = sqrt(R5.w);
            temp5 = dot(vec4(R125.x, R126.y, PV4.z, -0), vec4(R125.x, R126.y, R126.y, 0.0));
            PV5.x = temp5;
            PV5.y = temp5;
            PV5.z = temp5;
            PV5.w = temp5;
            R0.w = -PS4 + 1.0;
            temp6 = dot(vec4(R5.x, R5.y, R5.z, -0), vec4(R5.x, R5.y, R5.z, 0.0));
            PV6.x = temp6;
            PV6.y = temp6;
            PV6.z = temp6;
            PV6.w = temp6;
            R127.w = -PV5.x + 1.0;
            temp7 = dot(vec4(R3.x, R3.y, R3.z, -0), vec4(R3.x, R3.y, R3.z, 0.0));
            PV7.x = temp7;
            R127.y = temp7;
            PV7.z = temp7;
            PV7.w = temp7;
            PS7 = inversesqrt(PV6.x);
            R127.x = R5.x * PS7;
            R125.y = floor(R124.w);
            R126.z = R5.z * PS7;
            R127.w = R5.y * PS7;
            R127.z = sqrt(R127.w);
            temp9 = dot(vec4(R0.x, R0.y, R0.z, -0), vec4(R0.x, R0.y, R0.z, 0.0));
            PV9.x = temp9;
            PV9.y = temp9;
            PV9.z = temp9;
            PV9.w = temp9;
            PS9 = inversesqrt(R127.y);
            R126.x = R3.z * PS9;
            PV10.y = max(R127.z, 0.0);
            R127.z = R3.y * PS9;
            R126.w = R3.x * PS9;
            R125.w = inversesqrt(PV9.x);
            PV11.x = R126.z * PV10.y;
            PV11.y = R127.w * PV10.y;
            R126.z = R0.x * R125.w;
            PV11.w = R127.x * PV10.y;
            R127.y = R0.y * R125.w;
            PV12.x = R0.z * R125.w;
            R123.y = fma(R126.x, R125.x, PV11.x);
            R123.z = fma(R126.w, R125.x, PV11.w);
            R123.w = fma(R127.z, R125.x, PV11.y);
            R124.y = R125.y * 0.003921569;
            R126.x = fma(R126.z, R126.y, R123.z);
            R127.y = fma(R127.y, R126.y, R123.w);
            R126.z = fma(PV12.x, R126.y, R123.y);
            R126.w = floor(R124.y);
            R2.w = 0.0;
            temp14 = dot(vec4(R1.x, R1.y, R1.z, -0), vec4(R1.x, R1.y, R1.z, 0.0));
            PV14.x = temp14;
            PV14.y = temp14;
            PV14.z = temp14;
            PV14.w = temp14;
            R6.w = KC0[1].x;
            temp15 = dot(vec4(R126.x, R127.y, R126.z, -0), vec4(R126.x, R127.y, R126.z, 0.0));
            PV15.x = temp15;
            PV15.y = temp15;
            PV15.z = temp15;
            PV15.w = temp15;
            PS15 = inversesqrt(PV14.x);
            R125.x = R1.x * PS15;
            R126.y = R1.y * PS15;
            R127.z = R1.z * PS15;
            R5.w = R8.z;
            PS16 = inversesqrt(PV15.x);
            R126.x = R126.x * PS16;
            R127.y = R127.y * PS16;
            R126.z = R126.z * PS16;
            PV17.w = R4.z * R4.z;
            R5.x = R124.w + -R125.y;
            temp18 = dot(vec4(R125.x, R126.y, R127.z, -0), vec4(R126.x, R127.y, R126.z, 0.0));
            PV18.x = temp18;
            PV18.y = temp18;
            PV18.z = temp18;
            PV18.w = temp18;
            R122.x = fma(R4.y, R4.y, PV17.w);
            R123.x = fma(R4.x, R4.x, R122.x);
            PV19.y = PV18.x * R0.w;
            R5.z = R126.w * 0.003921569;
            R5.y = R124.y + -R126.w;
            R126.x = fma(-R125.x, PV19.y, R126.x);
            R127.y = fma(-R126.y, PV19.y, R127.y);
            R127.z = fma(-R127.z, PV19.y, R126.z);
            R126.w = inversesqrt(R123.x);
            temp21 = dot(vec4(R126.x, R127.y, R127.z, -0), vec4(R126.x, R127.y, R127.z, 0.0));
            PV21.x = temp21;
            PV21.y = temp21;
            PV21.z = temp21;
            PV21.w = temp21;
            R1.x = R4.x * R126.w;
            R1.y = R4.y * R126.w;
            R126.z = R4.z * R126.w;
            PS22 = inversesqrt(PV21.x);
            R4.x = R126.x * PS22;
            R4.y = R127.y * PS22;
            R127.z = R127.z * PS22;
            R9.x = R4.x;
            PV24.y = R4.y + 1.0;
            PV24.y = PV24.y / 2.0;
            PV24.z = R4.x + 1.0;
            PV24.z = PV24.z / 2.0;
            R9.w = R4.y;
            PS24 = -R126.z * R127.z;
            PV25.x = -PV24.y + 1.0;
            PV25.y = R126.z * R127.z;
            PV25.w = max(PV24.z, 0.0);
            R122.x = fma(-R1.y, R4.y, PS24);
            R123.x = fma(-R1.x, R4.x, R122.x);
            PV26.z = max(PV25.x, 0.0);
            R3.w = min(PV25.w, 1.0);
            R122.x = fma(R1.y, R4.y, PV25.y);
            R3.y = min(PV26.z, 1.0);
            R4.z = R123.x + R123.x;
            R0.w = fma(R1.x, R4.x, R122.x);
            R123.x = fma(-R4.z, R4.x, -R1.x);
            R123.x = R123.x / 2.0;
            PV28.y = max(R0.w, -R0.w);
            R123.w = fma(-R4.z, R4.y, -R1.y);
            R123.w = R123.w / 2.0;
            R1.x = R123.x + 0.5;
            R1.y = R123.w + 0.5;
            R4.z = -PV28.y + 1.0;
            R3.xyzw = texture(t0, vec2(R3.w, R3.y)).xyzw;
            R1.xyz = texture(t5, vec2(R1.x, R1.y)).xyz;
            R126.x = fma(KC0[0].z, R3.z, 0.0);
            R127.y = fma(KC0[0].y, R3.y, 0.0);
            R123.z = fma(KC0[0].w, R3.w, 0.0);
            R126.w = fma(KC0[0].x, R3.x, 0.0);
            PS32 = log2(R4.z);
            R2.x = fma(R8.x, R1.x, R7.x);
            R2.y = fma(R8.x, R1.y, R7.y);
            R2.z = fma(R8.x, R1.z, R7.z);
            PV33.w = KC0[2].w * PS32;
            R1.w = R123.z;
            R1.w = R1.w / 2.0;
            PS34 = exp2(PV33.w);
            R123.x = fma(KC0[2].x, PS34, R126.w);
            R123.z = fma(KC0[2].y, PS34, R127.y);
            R123.w = fma(KC0[2].z, PS34, R126.x);
            PV36.x = R8.y * R123.z;
            PV36.y = R8.y * R123.x;
            PV36.z = R8.y * R123.w;
            R1.x = PV36.y;
            R1.x = R1.x / 2.0;
            R1.y = PV36.x;
            R1.y = R1.y / 2.0;
            R1.z = PV36.z;
            R1.z = R1.z / 2.0;
            R14.x = R5.x;
            R14.y = R5.y;
            R14.z = R5.z;
            R14.w = R5.w;
            R13.x = R6.x;
            R13.y = R6.y;
            R13.z = R6.z;
            R13.w = R6.w;
            R11.x = R2.x;
            R11.y = R2.y;
            R11.z = R2.z;
            R11.w = R2.w;
            R10.x = R1.x;
            R10.y = R1.y;
            R10.z = R1.z;
            R10.w = R1.w;
            R12.x = R9.x;
            R12.y = R9.w;
            R12.z = R9.z;
            R12.w = R9.z;
            PIX0.x = R10.x;
            PIX0.y = R10.y;
            PIX0.z = R10.z;
            PIX0.w = R10.w;
            PIX1.x = R11.x;
            PIX1.y = R11.y;
            PIX1.z = R11.z;
            PIX1.w = R11.w;
            PIX2.x = R12.x;
            PIX2.y = R12.y;
            PIX2.z = R12.z;
            PIX2.w = R12.w;
            PIX3.x = R13.x;
            PIX3.y = R13.y;
            PIX3.z = R13.z;
            PIX3.w = R13.w;
            PIX4.x = R14.x;
            PIX4.y = R14.y;
            PIX4.z = R14.z;
            PIX4.w = R14.w;
        "};

        // TODO: Figure out the expected nodes to test previous node references.
        // TODO: Test expected nodes on a handwritten example?
        let graph = Graph::from_latte_asm(asm);
        assert_eq!(expected, graph.to_glsl());
    }
}
