use pest::{iterators::Pair, Parser};
use pest_derive::Parser;

use super::*;

// TODO: Compare assembly and Cemu GLSL for Elma's legs (PC221115).
// TODO: PV6 is a temp value (____ mask) written to in instruction count 6?
// TODO: exp_done with brstcnt assigns sequential registers to sequential outputs
// TODO: mov/2 a b is equivalent to a = b / 2
// TODO: unit tests for sample shaders to test all these cases
// TODO: MULADD_D2 is fma and then divide by 2
// TODO: The first registers are always input attributes?
impl Graph {
    pub fn from_latte_asm(asm: &str) -> Self {
        let program = LatteParser::parse(Rule::program, asm)
            .unwrap()
            .next()
            .unwrap();

        // TODO: Convert rules into a graph.
        // TODO: Convert vector to 4 scalar instructions.
        // TODO: Handle burstcnt outputs.
        // TODO: How to handle masks?
        let mut nodes = Vec::new();
        for pair in program.into_inner() {
            if pair.as_rule() == Rule::instruction {
                let inst = pair.into_inner().next().unwrap();
                match inst.as_rule() {
                    // TODO: functions that return option to clean this up
                    Rule::cf_inst => {
                        let mut inner = inst.into_inner();
                        let inst_count: usize = inner.next().unwrap().as_str().parse().unwrap();
                        let op_code = inner.next().unwrap().as_str();
                        for property in inner {}
                    }
                    Rule::cf_exp_inst => {
                        let mut inner = inst.into_inner();
                        let inst_count: usize = inner.next().unwrap().as_str().parse().unwrap();
                        let op_code = inner.next().unwrap().as_str();

                        let target = inner.next().unwrap();
                        let (target_name, target_index) = exp_target(target);

                        let source = inner.next().unwrap().as_str();

                        // TODO: track source register range and output range
                        let mut burst_count = 1;
                        for property in inner {
                            for inner in property.into_inner() {
                                if inner.as_rule() == Rule::burstcnt {
                                    burst_count = inner
                                        .into_inner()
                                        .next()
                                        .unwrap()
                                        .as_str()
                                        .parse()
                                        .unwrap();
                                }
                            }
                        }

                        // BURSTCNT assigns consecutive input and output registers.
                        for i in 0..burst_count {
                            // TODO: Track previous node assignments for source.
                            // TODO: use out_attr{i} for consistency with GLSL?
                            let node = Node {
                                output: Output {
                                    name: format!("{target_name}{}", target_index + i),
                                    channels: String::new(),
                                },
                                input: Expr::Node {
                                    node_index: 0,
                                    channels: String::new(),
                                },
                            };
                            nodes.push(node);
                        }
                    }
                    Rule::tex_clause => {
                        let mut inner = inst.into_inner();
                        let inst_count: usize = inner.next().unwrap().as_str().parse().unwrap();
                        let _inst_type = inner.next().unwrap().as_str();
                        let properties = inner.next().unwrap().as_str();
                        for tex_instruction in inner {
                            let node = tex_inst_node(tex_instruction).unwrap();
                            nodes.push(node);
                        }
                    }
                    Rule::alu_clause => {
                        let mut inner = inst.into_inner();
                        let inst_count: usize = inner.next().unwrap().as_str().parse().unwrap();
                        let _inst_type = inner.next().unwrap().as_str();
                        let properties = inner.next().unwrap().as_str();
                        for group in inner {}
                    }
                    _ => (),
                }
            }
        }

        Self { nodes }
    }
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

    // TODO: Generate a "texture" function node.
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
    let swizzle = inner.next()?.as_str();
    Some(Output {
        name: gpr.to_string(),
        channels: String::new(),
    })
}

fn texture_inst_src(dest: Pair<Rule>) -> Option<Expr> {
    // TODO: Handle other cases from grammar.
    let mut inner = dest.into_inner();
    let gpr = inner.next()?.as_str();
    let swizzle = inner.next()?.as_str();

    // TODO: Also handle cube maps.
    // TODO: Will these always use input attributes?
    Some(Expr::Func {
        name: "vec2".to_string(),
        args: vec![
            Expr::Global {
                name: gpr.to_string(),
                channels: String::new(),
            },
            Expr::Global {
                name: gpr.to_string(),
                channels: String::new(),
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

        // TODO: Figure out the expected nodes.
        assert_eq!(Graph { nodes: vec![] }, Graph::from_latte_asm(asm));
    }
}
