use pest::{iterators::Pair, Parser};
use pest_derive::Parser;

use super::*;

// Each ALU group has 4 vector operations xyzw and a scalar operation t.
// TODO: Compare assembly and Cemu GLSL for Elma's legs (PC221115).
// TODO: unit tests for sample shaders to test all these cases
// TODO: The first registers are always input attributes?
impl Graph {
    pub fn from_latte_asm(asm: &str) -> Self {
        let program = LatteParser::parse(Rule::program, asm)
            .unwrap()
            .next()
            .unwrap();

        let mut nodes = Vec::new();
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

        Self { nodes }
    }
}

fn add_exp_inst(inst: Pair<Rule>, nodes: &mut Vec<Node>) {
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
        let node = Node {
            output: Output {
                name: format!("{target_name}{}", target_index + i),
                channels: channels.unwrap_or_default().to_string(),
            },
            input: Expr::Global {
                name: format!("{source_name}{}", source_index + i),
                channels: channels.unwrap_or_default().to_string(),
            },
        };
        nodes.push(node);
    }
}

fn add_tex_clause(inst: Pair<Rule>, nodes: &mut Vec<Node>) {
    let mut inner = inst.into_inner();
    let inst_count: usize = inner.next().unwrap().as_str().parse().unwrap();
    let _inst_type = inner.next().unwrap().as_str();
    let properties = inner.next().unwrap().as_str();
    for tex_instruction in inner {
        let node = tex_inst_node(tex_instruction).unwrap();
        nodes.push(node);
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
    fn from_pair(pair: Pair<Rule>, nodes: &[Node], inst_count: usize, source_count: usize) -> Self {
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

fn add_alu_clause(inst: Pair<Rule>, nodes: &mut Vec<Node>) {
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
                    nodes.push(node);
                }
            } else {
                add_scalar(scalar, nodes);
            }
        }
    }
}

fn dot_product_node_index(
    scalars: &[AluScalar],
    inst_count: usize,
    nodes: &mut Vec<Node>,
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
        let node_index = nodes.len();

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
        nodes.push(node);
        Some(node_index)
    } else {
        None
    }
}

fn add_scalar(scalar: AluScalar, nodes: &mut Vec<Node>) {
    let dst = scalar.output.name.clone();
    let output = scalar.output;
    match scalar.op_code.as_str() {
        // scalar1
        "MOV" => {
            let node = Node {
                output,
                input: scalar.sources[0].clone(),
            };
            nodes.push(node);
        }
        "FLOOR" => {
            let node = Node {
                output,
                input: Expr::Func {
                    name: "floor".to_string(),
                    args: vec![scalar.sources[0].clone()],
                    channels: String::new(),
                },
            };
            nodes.push(node);
        }
        "SQRT_IEEE" => {
            let node = Node {
                output,
                input: Expr::Func {
                    name: "sqrt".to_string(),
                    args: vec![scalar.sources[0].clone()],
                    channels: String::new(),
                },
            };
            nodes.push(node);
        }
        "RECIPSQRT_IEEE" => {
            let node = Node {
                output,
                input: Expr::Func {
                    name: "inversesqrt".to_string(),
                    args: vec![scalar.sources[0].clone()],
                    channels: String::new(),
                },
            };
            nodes.push(node);
        }
        "EXP_IEEE" => {
            let node = Node {
                output,
                input: Expr::Func {
                    name: "exp2".to_string(),
                    args: vec![scalar.sources[0].clone()],
                    channels: String::new(),
                },
            };
            nodes.push(node);
        }
        // scalar2
        "ADD" => {
            let node = Node {
                output,
                input: Expr::Add(
                    Box::new(scalar.sources[0].clone()),
                    Box::new(scalar.sources[1].clone()),
                ),
            };
            nodes.push(node);
        }
        "MAX" => {
            let node = Node {
                output,
                input: Expr::Func {
                    name: "max".to_string(),
                    args: vec![scalar.sources[0].clone(), scalar.sources[1].clone()],
                    channels: String::new(),
                },
            };
            nodes.push(node);
        }
        "MUL" => {
            let node = Node {
                output,
                input: Expr::Mul(
                    Box::new(scalar.sources[0].clone()),
                    Box::new(scalar.sources[1].clone()),
                ),
            };
            nodes.push(node);
        }
        "DOT4" | "DOT4_IEEE" => {
            // Handled in a previous check.
            unreachable!()
        }
        // scalar3
        "MULADD" => {
            let input = Expr::Func {
                name: "fma".to_string(),
                args: vec![
                    scalar.sources[0].clone(),
                    scalar.sources[1].clone(),
                    scalar.sources[2].clone(),
                ],
                channels: String::new(),
            };
            let node = Node { output, input };
            nodes.push(node);
        }
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
            let node_index = nodes.len();
            nodes.push(node);

            let node = Node {
                output,
                input: Expr::Div(
                    Box::new(Expr::Node {
                        node_index,
                        channels: String::new(),
                    }),
                    Box::new(Expr::Float(2.0)),
                ),
            };
            nodes.push(node);
        }
        _ => panic!("unexpected opcode: {}", scalar.op_code),
    };

    let node_index = nodes.len();
    if let Some(modifier) = scalar.output_modifier {
        let node = alu_output_modifier(&modifier, &dst, node_index);
        nodes.push(node);
    }
}

fn alu_dst_output(pair: Pair<Rule>, inst_count: usize, alu_unit: &str) -> Output {
    // ____ mask for xyzw writes to a previous vector "PV".
    // ____ mask for t writes to a previous scalar "PS".
    let text = pair.as_str();
    let name = if pair.into_inner().next().map(|p| p.as_rule()) == Some(Rule::write_mask) {
        match alu_unit {
            "x" => format!("PV{inst_count}.x"),
            "y" => format!("PV{inst_count}.y"),
            "z" => format!("PV{inst_count}.z"),
            "w" => format!("PV{inst_count}.w"),
            "t" => format!("PS{inst_count}"),
            _ => unreachable!(),
        }
    } else {
        text.to_string()
    };
    Output {
        name,
        channels: String::new(),
    }
}

fn alu_output_modifier(modifier: &str, dst: &str, node_index: usize) -> Node {
    match modifier {
        "/2" => Node {
            output: Output {
                name: dst.to_string(),
                channels: String::new(),
            },
            input: Expr::Div(
                Box::new(Expr::Node {
                    node_index,
                    channels: String::new(),
                }),
                Box::new(Expr::Float(2.0)),
            ),
        },
        "*2" => Node {
            output: Output {
                name: dst.to_string(),
                channels: String::new(),
            },
            input: Expr::Mul(
                Box::new(Expr::Node {
                    node_index,
                    channels: String::new(),
                }),
                Box::new(Expr::Float(2.0)),
            ),
        },
        _ => panic!("unexpected modifier: {modifier}"),
    }
}

fn alu_src_expr(source: Pair<Rule>, nodes: &[Node]) -> Expr {
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

    // TODO: abs value?
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
    let expr = nodes
        .iter()
        .rposition(|n| {
            n.output.name == value
                && (n.output.channels.is_empty()
                    || channels.chars().all(|c| n.output.channels.contains(c)))
        })
        .map(|node_index| Expr::Node {
            node_index,
            channels: channels.to_string(),
        })
        .unwrap_or(Expr::Global {
            name: value.to_string(),
            channels: channels.to_string(),
        });

    if negate {
        Expr::Negate(Box::new(expr))
    } else {
        expr
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

fn tex_inst_node(tex_instruction: Pair<Rule>) -> Option<Node> {
    let mut inner = tex_instruction.into_inner();
    // TODO: why does this have trailing white space?
    let inst_count = inner.next()?.as_str();

    // TODO: Check that this is SAMPLE?
    let op_code = inner.next()?.as_str();

    // TODO: Get the input names and channels.
    // TODO: register or mask?
    let dest = inner.next()?;
    let output = texture_inst_dest(dest)?;

    let src = inner.next()?;
    let texcoords = texture_inst_src(src)?;

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
            channels: String::new(),
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

fn texture_inst_src(dest: Pair<Rule>) -> Option<Expr> {
    // TODO: Handle other cases from grammar.
    let mut inner = dest.into_inner();
    let gpr = inner.next()?.as_str();
    if inner.peek().map(|p| p.as_rule()) == Some(Rule::tex_rel) {
        inner.next().unwrap();
    }
    let channels = comp_swizzle(inner);

    // TODO: Also handle cube maps.
    // TODO: Will these always use input attributes?
    Some(Expr::Func {
        name: "vec2".to_string(),
        args: vec![
            Expr::Global {
                name: gpr.to_string(),
                channels: channels.chars().next().unwrap().to_string(),
            },
            Expr::Global {
                name: gpr.to_string(),
                channels: channels.chars().nth(1).unwrap().to_string(),
            },
        ],
        channels: String::new(),
    })
}

// Grammar adapted from the cpp-peglib grammer used for decaf-emu:
// https://github.com/decaf-emu/decaf-emu/blob/master/tools/latte-assembler/resources/grammar.txt
// TODO: Double check that whitespace is handled the same using @ where appropriate
// TODO: comments?
// TODO: reduce repetition in grammar
#[derive(Parser)]
#[grammar = "graph/latte.pest"]
struct LatteParser;

#[cfg(test)]
mod tests {
    use super::*;

    use indoc::indoc;
    use pretty_assertions::assert_eq;

    // TODO ALso test the GLSL output for a known shader.
    #[test]
    fn graph_from_asm_dl019100_frag_0() {
        let asm = indoc! {"
            00 TEX: ADDR(160) CNT(5)

            0      SAMPLE          R2.xy__, R6.xy0x, t3, s3

            1      SAMPLE          R8.xyz_, R6.xy0x, t1, s1

            2      SAMPLE          R7.xxx_, R6.xy0x, t2, s2

            3      SAMPLE          R9.___x, R6.xy0x, t5, s5

            4      SAMPLE          R6.xyz_, R6.xy0x, t4, s4

            01 ALU: ADDR(32) CNT(92) KCACHE0(CB1:0-15)
            5   x: MULADD          R125.x, R2.x, (0x40000000, 2), -1.0f
                y: MULADD          R125.y, R2.y, (0x40000000, 2), -1.0f
                z: MOV             ____, 0.0f
                w: MUL             R124.w, R2.z, (0x41000000, 8)
                t: MOV             R6.w, 0.0f

            6   x: DOT4            ____, PV5.x, PV5.x
                y: DOT4            ____, PV5.y, PV5.y
                z: DOT4            ____, PV5.z, PV5.y
                w: DOT4            ____, (0x80000000, -0), 0.0f
                t: FLOOR           R125.z, PV5.w

            7   x: DOT4_IEEE       ____, R5.x, R5.x
                y: DOT4_IEEE       ____, R5.y, R5.y
                z: DOT4_IEEE       ____, R5.z, R5.z
                w: DOT4_IEEE       ____, (0x80000000, -0), 0.0f
                t: ADD             R127.z, -PV6.x, 1.0f

            8   x: DOT4_IEEE       ____, R3.x, R3.x
                y: DOT4_IEEE       R127.y, R3.y, R3.y
                z: DOT4_IEEE       ____, R3.z, R3.z
                w: DOT4_IEEE       ____, (0x80000000, -0), 0.0f
                t: RECIPSQRT_IEEE  ____, PV7.x SCL_210

            9   x: MUL             R127.x, R5.x, PS8
                y: MUL             R124.y, R125.z, (0x3B808081, 0.003921569)
                z: MUL             R127.z, R5.z, PS8 VEC_120
                w: MUL             R126.w, R5.y, PS8
                t: SQRT_IEEE       R127.w, R127.z SCL_210

            10  x: DOT4_IEEE       ____, R0.x, R0.x
                y: DOT4_IEEE       ____, R0.y, R0.y
                z: DOT4_IEEE       ____, R0.z, R0.z
                w: DOT4_IEEE       ____, (0x80000000, -0), 0.0f
                t: RECIPSQRT_IEEE  ____, R127.y SCL_210

            11  x: MUL             R126.x, R3.z, PS10
                y: MAX             ____, R127.w, 0.0f
                z: MUL             R126.z, R3.y, PS10
                w: MUL             R127.w, R3.x, PS10
                t: RECIPSQRT_IEEE  R125.w, PV10.x SCL_210

            12  x: MUL             ____, R127.x, PV11.y
                y: MUL             R127.y, R0.x, PS11 VEC_120
                z: MUL             ____, R126.w, PV11.y
                w: MUL             ____, R127.z, PV11.y
                t: MUL             R126.y, R0.y, PS11

            13  x: MULADD          R123.x, R126.z, R125.x, PV12.z
                y: MULADD          R123.y, R127.w, R125.x, PV12.x
                z: MUL             ____, R0.z, R125.w VEC_120
                w: MULADD          R123.w, R126.x, R125.x, PV12.w
                t: FLOOR           R125.x, R124.y

            14  x: MULADD          R126.x, R127.y, R125.y, PV13.y
                y: MULADD          R127.y, R126.y, R125.y, PV13.x VEC_210
                z: MULADD          R125.z, PV13.z, R125.y, PV13.w
                w: MOV             R3.w, KC0[1].w
                t: ADD             R3.x, R124.w, -R125.z

            15  x: DOT4_IEEE       ____, R4.x, R4.x
                y: DOT4_IEEE       ____, R4.y, R4.y
                z: DOT4_IEEE       ____, R4.z, R4.z
                w: DOT4_IEEE       ____, (0x80000000, -0), 0.0f
                t: ADD             R3.y, R124.y, -R125.x

            16  x: DOT4_IEEE       ____, R126.x, R126.x
                y: DOT4_IEEE       ____, R127.y, R127.y
                z: DOT4_IEEE       ____, R125.z, R125.z
                w: DOT4_IEEE       ____, (0x80000000, -0), 0.0f
                t: RECIPSQRT_IEEE  ____, PV15.x SCL_210

            17  x: MUL             R127.x, R4.x, PS16
                y: MUL             R124.y, R4.y, PS16
                z: MUL             R126.z, R4.z, PS16
                t: RECIPSQRT_IEEE  ____, PV16.x SCL_210

            18  x: MUL             R126.x, R126.x, PS17
                y: MUL             R127.y, R127.y, PS17
                z: MUL             ____, R125.z, PS17
                t: MUL             R3.z, R125.x, (0x3B808081, 0.003921569)

            19  x: DOT4            ____, -R127.x, PV18.x
                y: DOT4            ____, -R124.y, PV18.y
                z: DOT4            ____, -R126.z, PV18.z
                w: DOT4            ____, (0x80000000, -0), 0.0f
                t: ADD/2           ____, PV18.y, 1.0f

            20  x: ADD             ____, PV19.x, PV19.x
                y: ADD             R0.y, -PS19, 1.0f
                z: ADD/2           R0.z, R126.x, 1.0f
                w: MOV             R5.w, R126.x
                t: MOV             R5.y, R127.y

            21  z: MULADD_D2       R123.z, -PV20.x, R126.x, -R127.x
                w: MULADD_D2       R123.w, -PV20.x, R127.y, -R124.y

            22  x: ADD             R4.x, PV21.z, 0.5f
                y: ADD             R4.y, PV21.w, 0.5f

            02 TEX: ADDR(170) CNT(2) VALID_PIX

            23     SAMPLE          R4.xyz_, R4.xy0x, t6, s6

            24     SAMPLE          R0.xyzw, R0.zy0z, t0, s0

            03 ALU: ADDR(124) CNT(33) KCACHE0(CB1:0-15)
            25  x: MULADD_D2       R4.x, KC0[0].x, R0.x, 0.0f
                y: ADD             ____, -R8.z, R4.z
                z: ADD             ____, -R8.y, R4.y
                w: ADD             ____, -R8.x, R4.x VEC_021
                t: MULADD_D2       R4.y, KC0[0].y, R0.y, 0.0f VEC_021

            26  x: MULADD          R123.x, PV25.w, R7.x, R8.x
                z: MULADD          R123.z, PV25.y, R7.z, R8.z VEC_201
                w: MULADD          R123.w, PV25.z, R7.y, R8.y
                t: MULADD_D2       R4.z, KC0[0].z, R0.z, 0.0f VEC_021

            27  x: MUL             R9.x, R1.x, PV26.x
                y: MUL             R9.y, R1.y, PV26.w
                z: MUL             R9.z, R1.z, PV26.z
                w: MULADD_D2       R4.w, KC0[0].w, R0.w, 0.0f

            28  x: MOV             R14.x, R3.x
                y: MOV             R14.y, R3.y
                z: MOV             R14.z, R3.z
                w: MOV             R14.w, R3.w

            29  x: MOV             R13.x, R6.x
                y: MOV             R13.y, R6.y
                z: MOV             R13.z, R6.z
                w: MOV             R13.w, R6.w

            30  x: MOV             R11.x, R9.x
                y: MOV             R11.y, R9.y
                z: MOV             R11.z, R9.z
                w: MOV             R11.w, R9.w

            31  x: MOV             R10.x, R4.x
                y: MOV             R10.y, R4.y
                z: MOV             R10.z, R4.z
                w: MOV             R10.w, R4.w

            32  x: MOV             R12.x, R5.w
                y: MOV             R12.y, R5.y
                z: MOV             R12.z, R5.z
                w: MOV             R12.w, R5.z

            04 EXP_DONE: PIX0, R10.xyzw BURSTCNT(4)

            END_OF_PROGRAM
        "};

        let expected = indoc! {"
            R2.xy = texture(t3, vec2(R6.x, R6.y));
            R8.xyz = texture(t1, vec2(R6.x, R6.y));
            R7.xxx = texture(t2, vec2(R6.x, R6.y));
            R9.x = texture(t5, vec2(R6.x, R6.y));
            R6.xyz = texture(t4, vec2(R6.x, R6.y));
            R125.x = fma(R2.x, 2, -1.0);
            R125.y = fma(R2.y, 2, -1.0);
            PV5.z = 0.0;
            R124.w = R2.z * 8;
            R6.w = 0.0;
            temp6 = dot(vec4(PV5.x, PV5.y, PV5.z, -0), vec4(PV5.x, PV5.y, PV5.y, 0.0));
            PV6.x = temp6;
            PV6.y = temp6;
            PV6.z = temp6;
            PV6.w = temp6;
            R125.z = floor(PV5.w);
            temp7 = dot(vec4(R5.x, R5.y, R5.z, -0), vec4(R5.x, R5.y, R5.z, 0.0));
            PV7.x = temp7;
            PV7.y = temp7;
            PV7.z = temp7;
            PV7.w = temp7;
            R127.z = -PV6.x + 1.0;
            temp8 = dot(vec4(R3.x, R3.y, R3.z, -0), vec4(R3.x, R3.y, R3.z, 0.0));
            PV8.x = temp8;
            R127.y = temp8;
            PV8.z = temp8;
            PV8.w = temp8;
            PS8 = inversesqrt(PV7.x);
            R127.x = R5.x * PS8;
            R124.y = R125.z * 0.003921569;
            R127.z = R5.z * PS8;
            R126.w = R5.y * PS8;
            R127.w = sqrt(R127.z);
            temp10 = dot(vec4(R0.x, R0.y, R0.z, -0), vec4(R0.x, R0.y, R0.z, 0.0));
            PV10.x = temp10;
            PV10.y = temp10;
            PV10.z = temp10;
            PV10.w = temp10;
            PS10 = inversesqrt(R127.y);
            R126.x = R3.z * PS10;
            PV11.y = max(R127.w, 0.0);
            R126.z = R3.y * PS10;
            R127.w = R3.x * PS10;
            R125.w = inversesqrt(PV10.x);
            PV12.x = R127.x * PV11.y;
            R127.y = R0.x * PS11;
            PV12.z = R126.w * PV11.y;
            PV12.w = R127.z * PV11.y;
            R126.y = R0.y * PS11;
            R123.x = fma(R126.z, R125.x, PV12.z);
            R123.y = fma(R127.w, R125.x, PV12.x);
            PV13.z = R0.z * R125.w;
            R123.w = fma(R126.x, R125.x, PV12.w);
            R125.x = floor(R124.y);
            R126.x = fma(R127.y, R125.y, PV13.y);
            R127.y = fma(R126.y, R125.y, PV13.x);
            R125.z = fma(PV13.z, R125.y, PV13.w);
            R3.w = KC0[1].w;
            R3.x = R124.w + -R125.z;
            temp15 = dot(vec4(R4.x, R4.y, R4.z, -0), vec4(R4.x, R4.y, R4.z, 0.0));
            PV15.x = temp15;
            PV15.y = temp15;
            PV15.z = temp15;
            PV15.w = temp15;
            R3.y = R124.y + -R125.x;
            temp16 = dot(vec4(R126.x, R127.y, R125.z, -0), vec4(R126.x, R127.y, R125.z, 0.0));
            PV16.x = temp16;
            PV16.y = temp16;
            PV16.z = temp16;
            PV16.w = temp16;
            PS16 = inversesqrt(PV15.x);
            R127.x = R4.x * PS16;
            R124.y = R4.y * PS16;
            R126.z = R4.z * PS16;
            PS17 = inversesqrt(PV16.x);
            R126.x = R126.x * PS17;
            R127.y = R127.y * PS17;
            PV18.z = R125.z * PS17;
            R3.z = R125.x * 0.003921569;
            temp19 = dot(vec4(-R127.x, -R124.y, -R126.z, -0), vec4(PV18.x, PV18.y, PV18.z, 0.0));
            PV19.x = temp19;
            PV19.y = temp19;
            PV19.z = temp19;
            PV19.w = temp19;
            PS19 = PV18.y + 1.0;
            PS19 = PS19 / 2.0;
            PV20.x = PV19.x + PV19.x;
            R0.y = -PS19 + 1.0;
            R0.z = R126.x + 1.0;
            R0.z = R0.z / 2.0;
            R5.w = R126.x;
            R5.y = R127.y;
            R123.z = fma(-PV20.x, R126.x, -R127.x);
            R123.z = R123.z / 2.0;
            R123.w = fma(-PV20.x, R127.y, -R124.y);
            R123.w = R123.w / 2.0;
            R4.x = PV21.z + 0.5;
            R4.y = PV21.w + 0.5;
            R4.xyz = texture(t6, vec2(R4.x, R4.y));
            R0.xyzw = texture(t0, vec2(R0.z, R0.y));
            R4.x = fma(KC0[0].x, R0.x, 0.0);
            R4.x = R4.x / 2.0;
            PV25.y = -R8.z + R4.z;
            PV25.z = -R8.y + R4.y;
            PV25.w = -R8.x + R4.x;
            R4.y = fma(KC0[0].y, R0.y, 0.0);
            R4.y = R4.y / 2.0;
            R123.x = fma(PV25.w, R7.x, R8.x);
            R123.z = fma(PV25.y, R7.z, R8.z);
            R123.w = fma(PV25.z, R7.y, R8.y);
            R4.z = fma(KC0[0].z, R0.z, 0.0);
            R4.z = R4.z / 2.0;
            R9.x = R1.x * PV26.x;
            R9.y = R1.y * PV26.w;
            R9.z = R1.z * PV26.z;
            R4.w = fma(KC0[0].w, R0.w, 0.0);
            R4.w = R4.w / 2.0;
            R14.x = R3.x;
            R14.y = R3.y;
            R14.z = R3.z;
            R14.w = R3.w;
            R13.x = R6.x;
            R13.y = R6.y;
            R13.z = R6.z;
            R13.w = R6.w;
            R11.x = R9.x;
            R11.y = R9.y;
            R11.z = R9.z;
            R11.w = R9.w;
            R10.x = R4.x;
            R10.y = R4.y;
            R10.z = R4.z;
            R10.w = R4.w;
            R12.x = R5.w;
            R12.y = R5.y;
            R12.z = R5.z;
            R12.w = R5.z;
            PIX0.xyzw = R10.xyzw;
            PIX1.xyzw = R11.xyzw;
            PIX2.xyzw = R12.xyzw;
            PIX3.xyzw = R13.xyzw;
            PIX4.xyzw = R14.xyzw;
        "};

        // TODO: Figure out the expected nodes to test previous node references.
        let graph = Graph::from_latte_asm(asm);
        assert_eq!(expected, graph.to_glsl());
    }
}
