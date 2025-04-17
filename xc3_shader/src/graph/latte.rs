use from_pest::{ConversionError, FromPest, Void};
use pest::{
    iterators::{Pair, Pairs},
    Parser, Span,
};
use pest_ast::FromPest;
use pest_derive::Parser;

use super::*;

// Grammar adapted from the cpp-peglib grammer used for decaf-emu:
// https://github.com/decaf-emu/decaf-emu/blob/master/tools/latte-assembler/resources/grammar.txt
#[derive(Parser)]
#[grammar = "graph/latte.pest"]
struct LatteParser;

fn parse_int(span: Span) -> usize {
    span.as_str().trim().parse().unwrap()
}

fn span_to_string(span: Span) -> String {
    span.as_str().to_string()
}

// TODO: simplify field names
#[derive(Debug, FromPest)]
#[pest_ast(rule(Rule::program))]
struct Program {
    instructions: Vec<Instruction>,
    end_of_program: EndOfProgram,
    eoi: EOI,
}

#[derive(Debug, FromPest)]
#[pest_ast(rule(Rule::instruction))]
enum Instruction {
    CfInst(CfInst),
    CfExpInst(CfExpInst),
    TexClause(TexClause),
    AluClause(AluClause),
}

#[derive(Debug, FromPest)]
#[pest_ast(rule(Rule::cf_inst))]
struct CfInst {
    inst_count: InstCount,
    cf_opcode: CfOpcode,
    cf_inst_properties: CfInstProperties,
}

#[derive(Debug, FromPest)]
#[pest_ast(rule(Rule::cf_opcode))]
struct CfOpcode;

#[derive(Debug, FromPest)]
#[pest_ast(rule(Rule::cf_inst_properties))]
struct CfInstProperties(Vec<CfInstProperty>);

#[derive(Debug)]
enum CfInstProperty {
    Burstcnt(Burstcnt),
    Unk(Rule),
}

// TODO: Is there a way to derive this?
impl<'pest> FromPest<'pest> for CfInstProperty {
    type Rule = Rule;

    type FatalError = Void;

    fn from_pest(
        pest: &mut Pairs<'pest, Self::Rule>,
    ) -> Result<Self, from_pest::ConversionError<Self::FatalError>> {
        // TODO: error type?
        let next = pest.peek().ok_or(ConversionError::NoMatch)?;
        match next.as_rule() {
            Rule::burstcnt => Burstcnt::from_pest(pest).map(Self::Burstcnt),
            _ => Ok(Self::Unk(next.as_rule())),
        }
    }
}

#[derive(Debug, FromPest)]
#[pest_ast(rule(Rule::burstcnt))]
struct Burstcnt(Number);

#[derive(Debug, FromPest)]
#[pest_ast(rule(Rule::cf_exp_inst))]
struct CfExpInst {
    inst_count: InstCount,
    exp_opcode: ExpOpcode,
    exp_target: ExpTarget,
    exp_src: ExpSrc,
    cf_inst_properties: CfInstProperties,
}

#[derive(Debug, FromPest)]
#[pest_ast(rule(Rule::exp_opcode))]
struct ExpOpcode(#[pest_ast(outer(with(span_to_string)))] String);

#[derive(Debug)]
enum ExpTarget {
    ExpPixTarget(ExpPixTarget),
    ExpPosTarget(ExpPosTarget),
    ExpParamTarget(ExpParamTarget),
}

// TODO: Is there a way to derive this?
impl<'pest> FromPest<'pest> for ExpTarget {
    type Rule = Rule;

    type FatalError = Void;

    fn from_pest(
        pest: &mut Pairs<'pest, Self::Rule>,
    ) -> Result<Self, from_pest::ConversionError<Self::FatalError>> {
        let next = pest.peek().unwrap();
        match next.as_rule() {
            Rule::exp_pix_target => ExpPixTarget::from_pest(pest).map(Self::ExpPixTarget),
            Rule::exp_pos_target => ExpPosTarget::from_pest(pest).map(Self::ExpPosTarget),
            Rule::exp_param_target => ExpParamTarget::from_pest(pest).map(Self::ExpParamTarget),
            _ => todo!(),
        }
    }
}

#[derive(Debug, FromPest)]
#[pest_ast(rule(Rule::exp_pix_target))]
struct ExpPixTarget(Number);

#[derive(Debug, FromPest)]
#[pest_ast(rule(Rule::exp_pos_target))]
struct ExpPosTarget(Number);

#[derive(Debug, FromPest)]
#[pest_ast(rule(Rule::exp_param_target))]
struct ExpParamTarget(Number);

#[derive(Debug, FromPest)]
#[pest_ast(rule(Rule::exp_src))]
struct ExpSrc {
    gpr: Gpr, // TODO: Gpr or GprRel
    four_comp_swizzle: FourCompSwizzle,
}

#[derive(Debug, FromPest)]
#[pest_ast(rule(Rule::tex_clause))]
struct TexClause {
    inst_count: InstCount,
}

#[derive(Debug, FromPest)]
#[pest_ast(rule(Rule::tex_inst))]
struct TexInst {
    inst_count: InstCount,
    tex_opcode: TexOpcode,
    tex_dst: TexDst,
    tex_src: TexSrc,
    tex_resource_id: TexResourceId,
    tex_sampler_id: TexSamplerId,
    tex_properties: TexProperties,
}

#[derive(Debug, FromPest)]
#[pest_ast(rule(Rule::tex_opcode))]
struct TexOpcode(#[pest_ast(outer(with(span_to_string)))] String);

#[derive(Debug, FromPest)]
#[pest_ast(rule(Rule::tex_resource_id))]
struct TexResourceId(#[pest_ast(outer(with(span_to_string)))] String);

#[derive(Debug, FromPest)]
#[pest_ast(rule(Rule::tex_sampler_id))]
struct TexSamplerId(#[pest_ast(outer(with(span_to_string)))] String);

#[derive(Debug, FromPest)]
#[pest_ast(rule(Rule::tex_dst))]
struct TexDst {
    gpr: Gpr,
    tex_rel: Option<TexRel>,
    four_comp_swizzle: FourCompSwizzle,
}

#[derive(Debug, FromPest)]
#[pest_ast(rule(Rule::tex_src))]
struct TexSrc {
    gpr: Gpr,
    tex_rel: Option<TexRel>,
    four_comp_swizzle: FourCompSwizzle,
}

#[derive(Debug, FromPest)]
#[pest_ast(rule(Rule::tex_rel))]
struct TexRel;

#[derive(Debug, FromPest)]
#[pest_ast(rule(Rule::tex_properties))]
struct TexProperties;

#[derive(Debug, FromPest)]
#[pest_ast(rule(Rule::alu_clause))]
struct AluClause {
    inst_count: InstCount,
    alu_clause_inst_type: AluClauseInstType,
    alu_clause_properties: AluClauseProperties,
    groups: Vec<AluGroup>,
}

#[derive(Debug, FromPest)]
#[pest_ast(rule(Rule::alu_clause_inst_type))]
struct AluClauseInstType;

#[derive(Debug, FromPest)]
#[pest_ast(rule(Rule::alu_clause_properties))]
struct AluClauseProperties(Vec<AluClauseProperty>);

#[derive(Debug)]
enum AluClauseProperty {
    Unk(Rule),
}

// TODO: Is there a way to derive this?
impl<'pest> FromPest<'pest> for AluClauseProperty {
    type Rule = Rule;

    type FatalError = Void;

    fn from_pest(
        pest: &mut Pairs<'pest, Self::Rule>,
    ) -> Result<Self, from_pest::ConversionError<Self::FatalError>> {
        // TODO: error type?
        let next = pest.next().ok_or(ConversionError::NoMatch)?;
        match next.as_rule() {
            Rule::addr
            | Rule::cnt
            | Rule::kcache0
            | Rule::kcache1
            | Rule::uses_waterfall
            | Rule::whole_quad_mode
            | Rule::no_barrier => Ok(Self::Unk(next.as_rule())),
            _ => Err(ConversionError::NoMatch),
        }
    }
}

#[derive(Debug, FromPest)]
#[pest_ast(rule(Rule::alu_group))]
struct AluGroup {
    inst_count: InstCount,
    scalars: Vec<AluScalar>,
}

#[derive(Debug)]
enum AluScalar {
    Scalar0(AluScalar0),
    Scalar1(AluScalar1),
    Scalar2(AluScalar2),
    Scalar3(AluScalar3),
}

// TODO: Is there a way to derive this?
impl<'pest> FromPest<'pest> for AluScalar {
    type Rule = Rule;

    type FatalError = Void;

    fn from_pest(
        pest: &mut Pairs<'pest, Self::Rule>,
    ) -> Result<Self, from_pest::ConversionError<Self::FatalError>> {
        let next = pest.peek().unwrap();
        match next.as_rule() {
            Rule::alu_scalar0 => AluScalar0::from_pest(pest).map(Self::Scalar0),
            Rule::alu_scalar1 => AluScalar1::from_pest(pest).map(Self::Scalar1),
            Rule::alu_scalar2 => AluScalar2::from_pest(pest).map(Self::Scalar2),
            Rule::alu_scalar3 => AluScalar3::from_pest(pest).map(Self::Scalar3),
            _ => todo!(),
        }
    }
}

#[derive(Debug, FromPest)]
#[pest_ast(rule(Rule::alu_scalar0))]
struct AluScalar0 {
    alu_unit: AluUnit,
    opcode: AluOpCode0,
    modifier: Option<AluOutputModifier>,
    dst: AluDst,
    properties: AluProperties,
}

#[derive(Debug, FromPest)]
#[pest_ast(rule(Rule::alu_scalar1))]
struct AluScalar1 {
    alu_unit: AluUnit,
    opcode: AluOpCode1,
    modifier: Option<AluOutputModifier>,
    dst: AluDst,
    src1: AluSrc,
    properties: AluProperties,
}

#[derive(Debug, FromPest)]
#[pest_ast(rule(Rule::alu_scalar2))]
struct AluScalar2 {
    alu_unit: AluUnit,
    opcode: AluOpCode2,
    modifier: Option<AluOutputModifier>,
    dst: AluDst,
    src1: AluSrc,
    src2: AluSrc,
    properties: AluProperties,
}

#[derive(Debug, FromPest)]
#[pest_ast(rule(Rule::alu_scalar3))]
struct AluScalar3 {
    alu_unit: AluUnit,
    opcode: AluOpCode3,
    dst: AluDst,
    src1: AluSrc,
    src2: AluSrc,
    src3: AluSrc,
    properties: AluProperties,
}

#[derive(Debug, FromPest)]
#[pest_ast(rule(Rule::write_mask))]
struct WriteMask(#[pest_ast(outer(with(span_to_string)))] String);

#[derive(Debug, FromPest)]
#[pest_ast(rule(Rule::alu_dst))]
struct AluDst(AluDstInner);

#[derive(Debug)]
enum AluDstInner {
    Value {
        gpr: Gpr,
        alu_rel: Option<AluRel>,
        one_comp_swizzle: Option<OneCompSwizzle>,
    },
    WriteMask(WriteMask),
}

// TODO: Is there a way to derive this?
impl<'pest> FromPest<'pest> for AluDstInner {
    type Rule = Rule;

    type FatalError = Void;

    fn from_pest(
        pest: &mut Pairs<'pest, Self::Rule>,
    ) -> Result<Self, from_pest::ConversionError<Self::FatalError>> {
        let next = pest.peek().unwrap();
        match next.as_rule() {
            Rule::write_mask => WriteMask::from_pest(pest).map(Self::WriteMask),
            Rule::gpr => Ok(Self::Value {
                gpr: Gpr::from_pest(pest)?,
                alu_rel: AluRel::from_pest(pest).ok(),
                one_comp_swizzle: OneCompSwizzle::from_pest(pest).ok(),
            }),
            _ => todo!(),
        }
    }
}

#[derive(Debug, FromPest)]
#[pest_ast(rule(Rule::alu_src))]
struct AluSrc {
    negate: Option<Negate>,
    src_value: AluSrcValue, // TODO: abs or src value
    alu_rel: Option<AluRel>,
    one_comp_swizzle: Option<OneCompSwizzle>,
}

#[derive(Debug, FromPest)]
#[pest_ast(rule(Rule::alu_src_value))]
struct AluSrcValue(AluSrcValueInner);

#[derive(Debug)]
enum AluSrcValueInner {
    Gpr(Gpr),
    ConstantCache0(ConstantCache0),
    ConstantCache1(ConstantCache1),
    ConstantFile(ConstantFile),
    Literal(Literal),
    PreviousScalar(PreviousScalar),
    PreviousVector(PreviousVector),
}

// TODO: Is there a way to derive this?
impl<'pest> FromPest<'pest> for AluSrcValueInner {
    type Rule = Rule;

    type FatalError = Void;

    fn from_pest(
        pest: &mut Pairs<'pest, Self::Rule>,
    ) -> Result<Self, from_pest::ConversionError<Self::FatalError>> {
        let next = pest.peek().unwrap();
        match next.as_rule() {
            Rule::gpr => Gpr::from_pest(pest).map(Self::Gpr),
            Rule::constant_cache0 => ConstantCache0::from_pest(pest).map(Self::ConstantCache0),
            Rule::constant_cache1 => ConstantCache1::from_pest(pest).map(Self::ConstantCache1),
            Rule::constant_file => ConstantFile::from_pest(pest).map(Self::ConstantFile),
            Rule::literal => Literal::from_pest(pest).map(Self::Literal),
            Rule::previous_scalar => PreviousScalar::from_pest(pest).map(Self::PreviousScalar),
            Rule::previous_vector => PreviousVector::from_pest(pest).map(Self::PreviousVector),
            _ => todo!(),
        }
    }
}

#[derive(Debug, FromPest)]
#[pest_ast(rule(Rule::literal))]
struct Literal(LiteralInner);

#[derive(Debug)]
enum LiteralInner {
    Hex(String),
    Float(String),
}

// TODO: Is there a way to derive this?
impl<'pest> FromPest<'pest> for LiteralInner {
    type Rule = Rule;

    type FatalError = Void;

    fn from_pest(
        pest: &mut Pairs<'pest, Self::Rule>,
    ) -> Result<Self, from_pest::ConversionError<Self::FatalError>> {
        let p1 = pest.next().unwrap();
        let p2 = pest.next();

        match (p1.as_rule(), p2.as_ref().map(|p| p.as_rule())) {
            (Rule::hex_number, None) => Ok(Self::Hex(p1.as_str().to_string())),
            (Rule::float, None) => Ok(Self::Float(p1.as_str().to_string())),
            (Rule::hex_number, Some(Rule::float)) => {
                Ok(Self::Float(p2.unwrap().as_str().to_string()))
            }
            _ => todo!(),
        }
    }
}

#[derive(Debug, FromPest)]
#[pest_ast(rule(Rule::constant_cache0))]
struct ConstantCache0(Number);

#[derive(Debug, FromPest)]
#[pest_ast(rule(Rule::constant_cache1))]
struct ConstantCache1(Number);

#[derive(Debug, FromPest)]
#[pest_ast(rule(Rule::constant_file))]
struct ConstantFile(Number);

#[derive(Debug, FromPest)]
#[pest_ast(rule(Rule::previous_scalar))]
struct PreviousScalar(Number);

#[derive(Debug, FromPest)]
#[pest_ast(rule(Rule::previous_vector))]
struct PreviousVector(Number);

#[derive(Debug, FromPest)]
#[pest_ast(rule(Rule::alu_abs_src_value))]
struct AluAbsSrcValue {
    value: AluSrcValue,
    one_comp_swizzle: Option<OneCompSwizzle>,
}

#[derive(Debug, FromPest)]
#[pest_ast(rule(Rule::alu_rel))]
struct AluRel;

#[derive(Debug, FromPest)]
#[pest_ast(rule(Rule::alu_unit))]
struct AluUnit(#[pest_ast(outer(with(span_to_string)))] String);

#[derive(Debug, FromPest)]
#[pest_ast(rule(Rule::negate))]
struct Negate;

#[derive(Debug, FromPest)]
#[pest_ast(rule(Rule::alu_output_modifier))]
struct AluOutputModifier(#[pest_ast(outer(with(span_to_string)))] String);

#[derive(Debug, FromPest)]
#[pest_ast(rule(Rule::alu_opcode0))]
struct AluOpCode0(#[pest_ast(outer(with(span_to_string)))] String);

#[derive(Debug, FromPest)]
#[pest_ast(rule(Rule::alu_opcode1))]
struct AluOpCode1(#[pest_ast(outer(with(span_to_string)))] String);

#[derive(Debug, FromPest)]
#[pest_ast(rule(Rule::alu_opcode2))]
struct AluOpCode2(#[pest_ast(outer(with(span_to_string)))] String);

#[derive(Debug, FromPest)]
#[pest_ast(rule(Rule::alu_opcode3))]
struct AluOpCode3(#[pest_ast(outer(with(span_to_string)))] String);

#[derive(Debug, FromPest)]
#[pest_ast(rule(Rule::alu_properties))]
struct AluProperties(Vec<AluProperty>);

#[derive(Debug)]
enum AluProperty {
    Unk(Rule),
}

// TODO: Is there a way to derive this?
impl<'pest> FromPest<'pest> for AluProperty {
    type Rule = Rule;

    type FatalError = Void;

    fn from_pest(
        pest: &mut Pairs<'pest, Self::Rule>,
    ) -> Result<Self, from_pest::ConversionError<Self::FatalError>> {
        // TODO: error type?
        let next = pest.next().ok_or(ConversionError::NoMatch)?;
        match next.as_rule() {
            Rule::bank_swizzle
            | Rule::update_exec_mask
            | Rule::update_pred
            | Rule::pred_sel
            | Rule::clamp => Ok(Self::Unk(next.as_rule())),
            _ => Err(ConversionError::NoMatch),
        }
    }
}

#[derive(Debug, FromPest)]
#[pest_ast(rule(Rule::inst_count))]
struct InstCount(Number);

#[derive(Debug, FromPest)]
#[pest_ast(rule(Rule::four_comp_swizzle))]
struct FourCompSwizzle(#[pest_ast(outer(with(span_to_string)))] String);

#[derive(Debug, FromPest)]
#[pest_ast(rule(Rule::one_comp_swizzle))]
struct OneCompSwizzle(#[pest_ast(outer(with(span_to_string)))] String);

#[derive(Debug, FromPest)]
#[pest_ast(rule(Rule::gpr))]
struct Gpr(Number);

#[derive(Debug, FromPest)]
#[pest_ast(rule(Rule::number))]
struct Number(#[pest_ast(outer(with(parse_int)))] usize);

#[derive(Debug, FromPest)]
#[pest_ast(rule(Rule::end_of_program))]
struct EndOfProgram;

#[derive(Debug, FromPest)]
#[pest_ast(rule(Rule::EOI))]
struct EOI;

impl std::fmt::Display for Gpr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "R{}", self.0 .0)
    }
}

impl std::fmt::Display for PreviousVector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PV{}", self.0 .0)
    }
}

impl std::fmt::Display for PreviousScalar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PS{}", self.0 .0)
    }
}

impl FourCompSwizzle {
    fn channels(&self) -> &str {
        self.0.trim_start_matches('.')
    }
}

impl OneCompSwizzle {
    fn channels(&self) -> &str {
        self.0.trim_start_matches('.')
    }
}

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

        let mut program = LatteParser::parse(Rule::program, &asm).unwrap();
        // TODO: use strongly typed API for parsing entire program.

        let mut nodes = Nodes::default();

        let program = program.next().unwrap();
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
    let exp = CfExpInst::from_pest(&mut Pairs::single(inst.clone())).unwrap();

    let inst_count = exp.inst_count.0 .0;

    let (target_name, target_index) = match exp.exp_target {
        ExpTarget::ExpPixTarget(t) => ("PIX", t.0 .0),
        ExpTarget::ExpPosTarget(t) => ("POS", t.0 .0),
        ExpTarget::ExpParamTarget(t) => ("PARAM", t.0 .0),
    };

    let source_name = "R";
    let source_index = exp.exp_src.gpr.0 .0;
    let channels = exp.exp_src.four_comp_swizzle.channels();

    let burst_count = exp
        .cf_inst_properties
        .0
        .iter()
        .find_map(|p| {
            if let CfInstProperty::Burstcnt(burstcnt) = p {
                Some(burstcnt.0 .0)
            } else {
                None
            }
        })
        .unwrap_or_default();

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

struct AluScalarData {
    alu_unit: char,
    op_code: String,
    output_modifier: Option<String>,
    output: Output,
    sources: Vec<Expr>,
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
            .map(|alu_scalar| {
                let scalar = AluScalar::from_pest(&mut Pairs::single(alu_scalar.clone())).unwrap();
                match scalar {
                    AluScalar::Scalar0(s) => {
                        let alu_unit = s.alu_unit.0.chars().next().unwrap();
                        AluScalarData {
                            alu_unit,
                            op_code: s.opcode.0,
                            output_modifier: s.modifier.map(|m| m.0),
                            output: alu_dst_output(s.dst, inst_count, alu_unit),
                            sources: Vec::new(),
                        }
                    }
                    AluScalar::Scalar1(s) => {
                        let alu_unit = s.alu_unit.0.chars().next().unwrap();
                        AluScalarData {
                            alu_unit,
                            op_code: s.opcode.0,
                            output_modifier: s.modifier.map(|m| m.0),
                            output: alu_dst_output(s.dst, inst_count, alu_unit),
                            sources: vec![alu_src_expr(s.src1, nodes)],
                        }
                    }
                    AluScalar::Scalar2(s) => {
                        let alu_unit = s.alu_unit.0.chars().next().unwrap();
                        AluScalarData {
                            alu_unit,
                            op_code: s.opcode.0,
                            output_modifier: s.modifier.map(|m| m.0),
                            output: alu_dst_output(s.dst, inst_count, alu_unit),
                            sources: vec![alu_src_expr(s.src1, nodes), alu_src_expr(s.src2, nodes)],
                        }
                    }
                    AluScalar::Scalar3(s) => {
                        let alu_unit = s.alu_unit.0.chars().next().unwrap();
                        AluScalarData {
                            alu_unit,
                            op_code: s.opcode.0,
                            output_modifier: None,
                            output: alu_dst_output(s.dst, inst_count, alu_unit),
                            sources: vec![
                                alu_src_expr(s.src1, nodes),
                                alu_src_expr(s.src2, nodes),
                                alu_src_expr(s.src3, nodes),
                            ],
                        }
                    }
                }
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
    scalars: &[AluScalarData],
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

fn add_scalar(scalar: AluScalarData, nodes: &mut Nodes, inst_count: usize) {
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
    scalar: &AluScalarData,
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

fn alu_dst_output(dst: AluDst, inst_count: usize, alu_unit: char) -> Output {
    match dst.0 {
        AluDstInner::Value {
            gpr,
            alu_rel,
            one_comp_swizzle,
        } => {
            let channel = one_comp_swizzle.and_then(|s| s.channels().chars().next());
            Output {
                name: gpr.to_string(),
                channel,
            }
        }
        AluDstInner::WriteMask(write_mask) => {
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
        }
    }
}

fn alu_output_modifier(modifier: &str, output: Output, node_index: usize) -> Node {
    let channel = output.channel;

    let (op, f) = match modifier {
        "/2" => (BinaryOp::Div, 2.0),
        "/4" => (BinaryOp::Div, 4.0),
        "*2" => (BinaryOp::Mul, 2.0),
        "*4" => (BinaryOp::Mul, 4.0),
        _ => panic!("unexpected modifier: {modifier}"),
    };

    Node {
        output,
        input: Expr::Binary(
            op,
            Box::new(Expr::Node {
                node_index,
                channel,
            }),
            Box::new(Expr::Float(f)),
        ),
    }
}

fn alu_src_expr(source: AluSrc, nodes: &Nodes) -> Expr {
    let negate = source.negate.is_some();

    let channel = source
        .one_comp_swizzle
        .and_then(|s| s.channels().chars().next());

    // Find a previous assignment that modifies the desired channel for variables.
    let expr = match source.src_value.0 {
        AluSrcValueInner::Gpr(gpr) => previous_assignment(&gpr.to_string(), channel, nodes),
        AluSrcValueInner::ConstantCache0(c0) => Expr::Parameter {
            name: "KC0".to_string(),
            field: None,
            index: Some(Box::new(Expr::Int(c0.0 .0 as i32))),
            channel,
        },
        AluSrcValueInner::ConstantCache1(c1) => Expr::Parameter {
            name: "KC1".to_string(),
            field: None,
            index: Some(Box::new(Expr::Int(c1.0 .0 as i32))),
            channel,
        },
        AluSrcValueInner::ConstantFile(constant_file) => todo!(),
        AluSrcValueInner::Literal(literal) => {
            // TODO: how to handle hex literals?
            match literal.0 {
                LiteralInner::Hex(hex) => todo!(),
                LiteralInner::Float(f) => Expr::Float(f.trim_end_matches('f').parse().unwrap()),
            }
        }
        AluSrcValueInner::PreviousScalar(s) => previous_assignment(&s.to_string(), channel, nodes),
        AluSrcValueInner::PreviousVector(v) => previous_assignment(&v.to_string(), channel, nodes),
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

fn tex_inst_node(tex_instruction: Pair<Rule>, nodes: &Nodes) -> Option<Vec<Node>> {
    let tex = TexInst::from_pest(&mut Pairs::single(tex_instruction)).ok()?;

    // TODO: Check that op code is SAMPLE?

    // TODO: Get the input names and channels.
    // TODO: register or mask?
    let output_name = tex.tex_dst.gpr.to_string();
    let output_channels = tex.tex_dst.four_comp_swizzle.channels();

    let texcoords = tex_src_coords(tex.tex_src, nodes)?;

    // TODO: make these rules not atomic and format similar to gpr?
    let texture = tex.tex_resource_id.0;
    let _sampler = tex.tex_sampler_id.0;

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
        // The output and input channels aren't always the same.
        // For example, ___x should assign src.x to dst.w.
        Some(
            output_channels
                .chars()
                .zip("xyzw".chars())
                .filter_map(|(c_in, c_out)| {
                    if c_in != '_' {
                        Some(Node {
                            output: Output {
                                name: output_name.clone(),
                                channel: Some(c_out),
                            },
                            input: Expr::Func {
                                name: "texture".to_string(),
                                args: vec![texture_name.clone(), texcoords.clone()],
                                channel: Some(c_in),
                            },
                        })
                    } else {
                        None
                    }
                })
                .collect(),
        )
    }
}

fn tex_src_coords(src: TexSrc, nodes: &Nodes) -> Option<Expr> {
    // TODO: Handle other cases from grammar.
    let gpr = src.gpr.to_string();

    // TODO: Handle write masks.
    let mut channels = src.four_comp_swizzle.channels().chars();

    // TODO: Also handle cube maps.
    Some(Expr::Func {
        name: "vec2".to_string(),
        args: vec![
            previous_assignment(&gpr, channels.next(), nodes),
            previous_assignment(&gpr, channels.next(), nodes),
        ],
        channel: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    use pretty_assertions::assert_eq;

    #[test]
    fn graph_from_asm_pc221115_frag_0() {
        // Elma's legs (visible on title screen).
        let asm = include_str!("../data/xcx/pc221115.0.frag.txt");
        let expected = include_str!("../data/xcx/pc221115.0.frag");

        // TODO: Figure out the expected nodes to test previous node references.
        // TODO: Test expected nodes on a handwritten example?
        let graph = Graph::from_latte_asm(asm);
        assert_eq!(expected, graph.to_glsl());
    }

    #[test]
    fn graph_from_asm_en020601_frag_0() {
        // Tree enemy.
        let asm = include_str!("../data/xcx/en020601.0.frag.txt");
        let expected = include_str!("../data/xcx/en020601.0.frag");

        let graph = Graph::from_latte_asm(asm);
        assert_eq!(expected, graph.to_glsl());
    }
}
