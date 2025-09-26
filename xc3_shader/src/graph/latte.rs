use log::error;
use peg::parser;
use smol_str::ToSmolStr;
use thiserror::Error;

use super::*;

/// Errors while converting latte shader assembly to a [Graph].
#[derive(Debug, Error)]
pub enum LatteError {
    #[error("error parsing assembly text")]
    Parse(#[from] peg::error::ParseError<peg::str::LineCol>),
}

// TODO: avoid unwrap
// TODO: avoid to_string() or clone()
// TODO: properly store properties
// Grammar adapted from the cpp-peglib grammer used for decaf-emu:
// https://github.com/decaf-emu/decaf-emu/blob/master/tools/latte-assembler/resources/grammar.txt
// Instruction details are available in the ISA:
// https://www.techpowerup.com/gpu-specs/docs/ati-r600-isa.pdf.
parser! {
grammar latte_parser() for str {
    pub rule program() -> Vec<Instruction> = _ i:(instruction() ** _) _ end_of_program() { i }
    rule instruction() -> Instruction
        = cf:cf_inst() { Instruction::CfInst(cf) }
        / cf_exp:cf_exp_inst() { Instruction::CfExpInst(cf_exp) }
        / clause:tex_clause() { Instruction::TexClause(clause) }
        / clause:alu_clause() { Instruction::AluClause(clause) }

    rule number() -> usize = n:$(['0'..='9']+) { n.parse().unwrap() }
    rule hex_number() -> &'input str = s:$("0x" ['0'..='9' | 'A'..='F' | 'a'..='f']+) { s }
    rule float() -> &'input str = s:$("-"? _ ['0'..='9']+ _ ("." ['0'..='9']+)? _ ("e-" ['0'..='9']+)? _ "f"?) { s }

    rule cf_inst() -> CfInst
        = inst_count:inst_count() _ opcode:cf_opcode() _ properties:cf_inst_properties() {
            CfInst {
                inst_count,
                opcode,
                properties
            }
        }
    rule cf_exp_inst() -> CfExpInst
        = inst_count:inst_count() _ opcode:exp_opcode() _ ":" _ target:exp_target() _ "," _ src:exp_src() _ properties:cf_inst_properties() {
            CfExpInst {
                inst_count,
                opcode: ExpOpcode(opcode.to_string()),
                target,
                src,
                properties
            }
        }
    rule tex_clause() -> TexClause
        = inst_count:inst_count() _ tex_clause_inst_type() _ ":" _ tex_clause_properties() _ instructions:(tex_or_fetch_inst() ** _) {
            TexClause {
                inst_count,
                inst_type: TexClauseInstType,
                properties: TexClauseProperties(Vec::new()),
                instructions
            }
        }
    rule alu_clause() -> AluClause
        = inst_count:inst_count() _ alu_clause_inst_type() _ ":" _ properties:alu_clause_properties() _ groups:(alu_group() ** _) {
            AluClause {
                inst_count,
                inst_type: AluClauseInstType,
                properties,
                groups
            }
        }
    rule end_of_program() = "END_OF_PROGRAM"

    rule inst_count() -> InstCount = n:number() { InstCount(n) }
    rule gpr() -> Gpr = "R" n:number() { Gpr(n) }
    rule gpr_rel() -> Gpr = "R[AL" _ "+" _ n:number() _ "]" { Gpr(n) }
    rule constant_file() -> usize = "C" n:number() { n }
    rule constant_cache0() -> usize = "KC0" _ "[" _ n:number() _ "]" { n }
    rule constant_cache1() -> usize = "KC1" _ "[" _ n:number() _ "]" { n }
    rule previous_scalar() -> usize = "PS" n:number() { n }
    rule previous_vector() -> usize = "PV" n:number() { n }
    rule one_comp_swizzle() -> char = "." c:$(['x' | 'y' | 'z' | 'w' | 'X' | 'Y' | 'Z' | 'W']) { c.chars().next().unwrap() }
    rule four_comp_swizzle() -> &'input str = "." s:$(['x' | 'y' | 'z' | 'w' | 'X' | 'Y' | 'Z' | 'W' | '0' | '1' | '_']+) { s }
    // TODO: always preserve hex?
    rule literal() -> Literal
        = n:hex_number() { Literal::Hex(n.to_string()) }
        / f:float() { Literal::Float(f.to_string()) }
        / "(" _ hex_number() _ "," _ f:float() _ ")" { Literal::Float(f.to_string()) }
    rule write_mask() -> &'input str = s:$("_"+) { s }
    rule negate() -> Negate = "-" { Negate }

    rule alu_clause_inst_type()
        = "ALU_PUSH_BEFORE"
        / "ALU_POP_AFTER"
        / "ALU_POP2_AFTER"
        / "ALU_EXT"
        / "ALU_CONTINUE"
        / "ALU_BREAK"
        / "ALU_ELSE_AFTER"
        / "ALU"
    rule tex_clause_inst_type() = "TEX_ACK" / "TEX"
    rule vtx_clause_inst_type() = "VTX_ACK" / "VTX_TC_ACK" / "VTX_TC" / "VTX"
    rule cf_opcode() -> CfOpcode
        = op:$("NOP"
        / "LOOP_START_NO_AL"
        / "LOOP_START_DX10"
        / "LOOP_START"
        / "LOOP_END"
        / "LOOP_CONTINUE"
        / "LOOP_BREAK"
        / "JUMP"
        / "PUSH_ELSE"
        / "PUSH"
        / "ELSE"
        / "POP_PUSH_ELSE"
        / "POP_PUSH"
        / "POP_JUMP"
        / "POP"
        / "CALL_FS"
        / "CALL"
        / "RETURN"
        / "EMIT_CUT_VERTEX"
        / "EMIT_VERTEX"
        / "CUT_VERTEX"
        / "KILL"
        / "WAIT_ACK"
        / "END_PROGRAM") { CfOpcode }

    rule tex_clause_properties() -> TexClauseProperties = props:(tex_clause_property() ** _) { TexClauseProperties(props) }
    rule tex_clause_property() -> TexClauseProperty
    = (addr() / cnt() / cf_const() / cnd() / whole_quad_mode() / no_barrier() / valid_pix()) {
        TexClauseProperty::Unk
    }
    rule alu_clause_properties() -> AluClauseProperties = props:(alu_clause_property() ** _) { AluClauseProperties(props) }
    rule alu_clause_property() -> AluClauseProperty
        = kc:kcache0() { AluClauseProperty::KCache0(kc) }
        / kc:kcache1() { AluClauseProperty::KCache1(kc) }
        / (addr() / cnt() / uses_waterfall() / whole_quad_mode() / no_barrier()) {
            AluClauseProperty::Unk
        }
    rule cf_inst_properties() -> CfInstProperties = props:(cf_inst_property() ** _) { CfInstProperties(props) }
    rule cf_inst_property() -> CfInstProperty
        = b:burstcnt() { CfInstProperty::Burstcnt(BurstCnt(b)) }
        / (addr()
        / cnt()
        / cf_const()
        / pop_cnt()
        / elem_size()
        / burstcnt()
        / kcache0()
        / kcache1()
        / uses_waterfall()
        / whole_quad_mode()
        / no_barrier()
        / valid_pix()
        / fail_jump_addr()
        / pass_jump_addr()) { CfInstProperty::Unk }
    rule burstcnt() -> usize = "BURSTCNT(" _ n:number() _ ")" { n }
    rule addr() -> usize = "ADDR(" _ n:number() _ ")" { n }
    rule cf_const() -> usize = "CF_CONST(" _ n:number() _ ")" { n }
    rule cnt() -> usize = "CNT(" _ n:number() _ ")" { n }
    rule cnd() = "CND(" _ ("ACTIVE" / "FALSE" / "BOOL" / "NOT_BOOL") _ ")"
    rule elem_size() -> usize = "ELEM_SIZE(" _ n:number() _ ")" { n }
    rule kcache0() -> ConstantBuffer
        = "KCACHE0" _ "(" _ "CB" _ n1:number() _ ":" _ n2:number() _ "-" n3:number() _ ")" {
            ConstantBuffer { index: n1, start_index: n2, end_index: n3 }
        }
    rule kcache1() -> ConstantBuffer
        = "KCACHE1" _ "(" _ "CB" _ n1:number() _ ":" _ n2:number() _ "-" n3:number() _ ")" {
            ConstantBuffer { index: n1, start_index: n2, end_index: n3 }
        }
    rule no_barrier() = "NO_BARRIER"
    rule pop_cnt() -> usize = "POP_CNT(" _ n:number() _ ")" { n }
    rule uses_waterfall() = "USES_WATERFALL"
    rule valid_pix() = "VALID_PIX"
    rule whole_quad_mode() = "WHOLE_QUAD_MODE" / "WHOLE_QUAD"
    rule fail_jump_addr() -> usize = "FAIL_JUMP_ADDR(" _ n:number() _ ")" { n }
    rule pass_jump_addr() -> usize = "PASS_JUMP_ADDR(" _ n:number() _ ")" { n }

    // TODO: Preserve gpr vs gpr_rel
    rule exp_src() -> ExpSrc = gpr:(gpr() / gpr_rel()) _ s:four_comp_swizzle()? { ExpSrc { gpr, swizzle: FourCompSwizzle(s.unwrap_or_default().to_string()) }}
    rule exp_opcode() -> &'input str = s:$("EXP_DONE" / "EXP") { s }
    rule exp_target() -> ExpTarget
        = i:exp_pix_target() { ExpTarget::Pix(ExpPixTarget(i)) }
        / i:exp_pos_target() { ExpTarget::Pos(ExpPosTarget(i)) }
        / i:exp_param_target() { ExpTarget::Param(ExpParamTarget(i)) }
    rule exp_pix_target() -> usize = "PIX" n:number() { n }
    rule exp_pos_target() -> usize = "POS" n:number() { n }
    rule exp_param_target() -> usize = "PARAM" n:number() { n }

    rule tex_inst() -> TexInst
        = inst_count:inst_count() _ opcode:tex_opcode() _ dst:tex_dst() _ "," _ src:tex_src() _ "," _ r:tex_resource_id() _ "," _ s:tex_sampler_id() _ tex_properties() {
            TexInst {
                inst_count,
                opcode: TexOpcode(opcode.to_string()),
                dst,
                src,
                resource_id: TexResourceId(r.to_string()),
                sampler_id: TexSamplerId(s.to_string()),
                properties: TexProperties
            }
        }
    rule tex_opcode() -> &'input str
        = s:$("VTX_FETCH"
        / "VTX_SEMANTIC"
        / "MEM"
        / "LD"
        / "GET_TEXTURE_INFO"
        / "GET_SAMPLE_INFO"
        / "GET_COMP_TEX_LOD"
        / "GET_GRADIENTS_H"
        / "GET_GRADIENTS_V"
        / "GET_LERP"
        / "KEEP_GRADIENTS"
        / "SET_GRADIENTS_H"
        / "SET_GRADIENTS_V"
        / "PASS"
        / "SET_CUBEMAP_INDEX"
        / "FETCH4"
        / "SAMPLE_C_G_LZ"
        / "SAMPLE_C_G_LB"
        / "SAMPLE_C_G_L"
        / "SAMPLE_C_G"
        / "SAMPLE_C_LZ"
        / "SAMPLE_C_LB"
        / "SAMPLE_C_L"
        / "SAMPLE_C"
        / "SAMPLE_G_LZ"
        / "SAMPLE_G_LB"
        / "SAMPLE_G_L"
        / "SAMPLE_G"
        / "SAMPLE_LZ"
        / "SAMPLE_LB"
        / "SAMPLE_L"
        / "SAMPLE"
        / "SET_TEXTURE_OFFSETS"
        / "GATHER4_C_O"
        / "GATHER4_O"
        / "GATHER4_C"
        / "GATHER4"
        / "GET_BUFFER_RESINFO") { s }
    rule tex_dst() -> TexDst
        = gpr:gpr() _ tex_rel:tex_rel()? _ s:four_comp_swizzle()? { TexDst { gpr, tex_rel, swizzle: FourCompSwizzle(s.unwrap_or_default().to_string()) }}
        / write_mask() { TexDst { gpr: todo!(), tex_rel: todo!(), swizzle: todo!() }}
    rule tex_src() -> TexSrc = gpr:gpr() _ tex_rel:tex_rel()? _ s:four_comp_swizzle()? { TexSrc { gpr, tex_rel, swizzle: FourCompSwizzle(s.unwrap_or_default().to_string()) }}
    rule tex_rel() -> TexRel = "[AL]" { TexRel }
    rule tex_resource_id() -> &'input str = s:$("t" number()) { s }
    rule tex_sampler_id() -> &'input str = s:$("s" number()) { s }
    rule tex_properties()  =  (alt_const() / bc_frac_mode() / denorm() / norm() / lod() / whole_quad_mode() / xoffset() / yoffset() / zoffset()) ** _
    rule alt_const() = "ALT_CONST"
    rule bc_frac_mode() = "BC_FRAC_MODE"
    rule denorm() = "DENORM(" _ ['x' | 'y' | 'z' | 'w' | 'X' | 'Y' | 'Z' | 'W']+ _ ")"
    rule norm() = "NORM(" _ ['x' | 'y' | 'z' | 'w' | 'X' | 'Y' | 'Z' | 'W']+ _ ")"
    rule lod() = "LOD(" _ float() _ ")"
    rule xoffset() = "XOFFSET(" _ float() _ ")"
    rule yoffset() = "YOFFSET(" _ float() _ ")"
    rule zoffset() = "ZOFFSET(" _ float() _ ")"

    // Fetch instructions are not part of the original grammar for some reason.
    rule tex_or_fetch_inst() -> TexInstOrFetchInst
        = f:fetch_inst() { TexInstOrFetchInst::Fetch(f) } / t:tex_inst() { TexInstOrFetchInst::Tex(t) }
    rule fetch_inst() -> FetchInst
        = inst_count:inst_count() _ "FETCH" _ dst:fetch_dst() _ "," _ src:fetch_src() _ "," _ buffer_id:fetch_buffer_id() _ properties:fetch_properties() {
            FetchInst {
                inst_count,
                dst,
                src,
                buffer_id,
                properties
            }
        }
    rule fetch_dst() -> FetchDst
        = gpr:gpr() _ s:four_comp_swizzle()? { FetchDst { gpr, swizzle: FourCompSwizzle(s.unwrap_or_default().to_string()) }}
    rule fetch_src() -> FetchSrc
        = gpr:gpr() _ swizzle:one_comp_swizzle()? { FetchSrc { gpr, swizzle }}
    rule fetch_buffer_id() -> usize = "b" _ n:number() { n }
    rule fetch_properties() -> Vec<FetchProperty>
        = (f:fetch_type() { FetchProperty::Type(f)}
        / m:fetch_mega() { FetchProperty::Mega(m) }
        / o:fetch_offset() { FetchProperty::Offset(o) }) ** _
    rule fetch_type() -> FetchType = "FETCH_TYPE(NO_INDEX_OFFSET)" { FetchType {} }
    rule fetch_mega() -> FetchMega = "MEGA(" _ n:number() _ ")" { FetchMega(n) }
    rule fetch_offset() -> FetchOffset = "OFFSET(" _ n:number() _ ")" { FetchOffset(n) }

    rule alu_group() -> AluGroup
        = inst_count:inst_count() _ scalars:(alu_scalar() ++ _) {
            AluGroup {
                inst_count,
                scalars
            }
        }
    rule alu_unit() -> &'input str = s:$("x" / "y" / "z" / "w" / "t") { s }
    rule alu_scalar() -> AluScalar
        = s:alu_scalar0() { AluScalar::Scalar0(s) }
        / s:alu_scalar1() { AluScalar::Scalar1(s) }
        / s:alu_scalar2() { AluScalar::Scalar2(s) }
        / s:alu_scalar3() { AluScalar::Scalar3(s) }
    rule alu_scalar0() -> AluScalar0
        = u:alu_unit() _ ":" _ op:alu_opcode0() _ m:alu_output_modifier()? _ dst:alu_dst() _ properties:alu_properties() {
            AluScalar0 {
                alu_unit: AluUnit(u.to_string()),
                opcode: AluOpCode0(op.to_string()),
                modifier: m.map(|m| AluOutputModifier(m.to_string())),
                dst,
                properties
            }
        }
    rule alu_scalar1() -> AluScalar1
        = u:alu_unit() _ ":" _ op:alu_opcode1() _ m:alu_output_modifier()? _ dst:alu_dst() _ "," _ src1:alu_src() _ properties:alu_properties() {
            AluScalar1 {
                alu_unit: AluUnit(u.to_string()),
                opcode: AluOpCode1(op.to_string()),
                modifier: m.map(|m| AluOutputModifier(m.to_string())),
                dst,
                src1,
                properties
            }
        }
    rule alu_scalar2() -> AluScalar2
        = u:alu_unit() _ ":" _ op:alu_opcode2() _ m:alu_output_modifier()? _ dst:alu_dst() _ "," _ src1:alu_src() _ "," _ src2:alu_src() _ properties:alu_properties() {
            AluScalar2 {
                alu_unit: AluUnit(u.to_string()),
                opcode: AluOpCode2(op.to_string()),
                modifier: m.map(|m| AluOutputModifier(m.to_string())),
                dst,
                src1,
                src2,
                properties
            }
        }
    rule alu_scalar3() -> AluScalar3
        = u:alu_unit() _ ":" _ op:alu_opcode3() _ dst:alu_dst() _ "," _ src1:alu_src() _ "," _ src2:alu_src() _ "," _ src3:alu_src() _ properties:alu_properties() {
            AluScalar3 {
                alu_unit: AluUnit(u.to_string()),
                opcode: AluOpCode3(op.to_string()),
                dst,
                src1,
                src2,
                src3,
                properties
            }
        }
    rule alu_opcode0() -> &'input str = s:$("NOP" / "SET_MODE" / "SET_CF_IDX0" / "SET_CF_IDX1") { s }
    rule alu_opcode1() -> &'input str
        = s:$("FLT64_TO_FLT32"
        / "FLT32_TO_FLT64"
        / "FREXP_64"
        / "FRACT"
        / "TRUNC"
        / "CEIL"
        / "RNDNE"
        / "FLOOR"
        / "MOVA_FLOOR"
        / "MOVA_INT"
        / "MOVA"
        / "MOV"
        / "EXP_IEEE"
        / "LOG_CLAMPED"
        / "LOG_IEEE"
        / "RECIP_CLAMPED"
        / "RECIP_IEEE"
        / "RECIP_UINT"
        / "RECIP_INT"
        / "RECIP_FF"
        / "RECIPSQRT_CLAMPED"
        / "RECIPSQRT_IEEE"
        / "RECIPSQRT_FF"
        / "SQRT_IEEE"
        / "FLT_TO_INT"
        / "INT_TO_FLT"
        / "UINT_TO_FLT"
        / "FLT_TO_UINT"
        / "SIN"
        / "COS"
        / "FRACT_64"
        / "SQRT_e"
        / "EXP_e"
        / "LOG_e"
        / "RSQ_e"
        / "RCP_e"
        / "LOG_sat") { s }
    rule alu_opcode2() -> &'input str
        = s:$("MULHI_INT24"
        / "MULLO_INT"
        / "MULHI_INT"
        / "MULLO_UINT"
        / "MULHI_UINT"
        / "MUL_INT24"
        / "MUL_IEEE"
        / "MUL_e"
        / "MUL_64"
        / "MUL"
        / "MAX_DX10"
        / "MAX_UINT"
        / "MAX_INT"
        / "MAX"
        / "MIN_DX10"
        / "MIN_UINT"
        / "MIN_INT"
        / "MIN"
        / "SETE_DX10"
        / "SETGT_DX10"
        / "SETGE_DX10"
        / "SETNE_DX10"
        / "ADD_INT"
        / "ADD_64"
        / "ADD"
        / "PRED_SETGT_PUSH_INT"
        / "PRED_SETGT_PUSH"
        / "PRED_SETGT_UINT"
        / "PRED_SETGT_INT"
        / "PRED_SETGT_64"
        / "PRED_SETGT"
        / "PRED_SETGE_PUSH_INT"
        / "PRED_SETGE_PUSH"
        / "PRED_SETGE_UINT"
        / "PRED_SETGE_INT"
        / "PRED_SETGE_64"
        / "PRED_SETGE"
        / "PRED_SETE_PUSH_INT"
        / "PRED_SETE_INT"
        / "PRED_SETE_PUSH"
        / "PRED_SETE_64"
        / "PRED_SETE"
        / "PRED_SETNE_PUSH_INT"
        / "PRED_SETNE_PUSH"
        / "PRED_SETNE_INT"
        / "PRED_SETNE"
        / "PRED_SETLT_PUSH_INT"
        / "PRED_SETLE_PUSH_INT"
        / "PRED_SET_INV"
        / "PRED_SET_POP"
        / "PRED_SET_CLR"
        / "PRED_SET_RESTORE"
        / "KILLGT"
        / "KILLGE"
        / "KILLNE"
        / "AND_INT"
        / "OR_INT"
        / "XOR_INT"
        / "NOT_INT"
        / "SUB_INT"
        / "SETE_INT"
        / "SETGT_INT"
        / "SETGE_INT"
        / "SETNE_INT"
        / "SETGT_UINT"
        / "SETGE_UINT"
        / "KILLGT_UINT"
        / "KILLGE_UINT"
        / "KILLE_INT"
        / "KILLGT_INT"
        / "KILLGE_INT"
        / "KILLNE_INT"
        / "DOT4_IEEE"
        / "DOT4_e"
        / "DOT4"
        / "CUBE"
        / "MAX4"
        / "GROUP_BARRIER"
        / "GROUP_SEQ_BEGIN"
        / "GROUP_SEQ_END"
        / "SET_LDS_SIZE"
        / "MOVA_GPR_INT"
        / "ASHR_INT"
        / "LSHR_INT"
        / "LSHL_INT"
        / "LDEXP_64"
        / "PREDGT"
        / "SETE"
        / "SETGT"
        / "SETGE"
        / "SETNE"
        / "KILLE") { s }
    rule alu_opcode3() -> &'input str
        = s:$("BFE_UINT"
        / "BFE_INT"
        / "BFI_INT"
        / "FMA"
        / "MULADD_64_D2"
        / "MULADD_64_M4"
        / "MULADD_64_M2"
        / "MULADD_64"
        / "MUL_LIT_D2"
        / "MUL_LIT_M4"
        / "MUL_LIT_M2"
        / "MUL_LIT"
        / "MULADD_IEEE_D2"
        / "MULADD_IEEE_M4"
        / "MULADD_IEEE_M2"
        / "MULADD_IEEE"
        / "MULADD_D2"
        / "MULADD_M4"
        / "MULADD_M2"
        / "MULADD_e"
        / "MULADD"
        / "CNDGE_INT"
        / "CNDGE"
        / "CNDGT_INT"
        / "CNDGT"
        / "CNDE_INT"
        / "CNDE") { s }
    rule alu_output_modifier() -> &'input str = s:$("*2" / "*4" / "/2" / "/4") { s }
    rule alu_rel() -> AluRel
        = (("[AR." _ ("x" / "y" / "z" / "w" / "t" / "X" / "Y" / "Z" / "W" / "T") _ "]") / "[AL]") {
            AluRel
        }
    rule alu_dst() -> AluDst
        = gpr:gpr() _ alu_rel:alu_rel()? _ swizzle:one_comp_swizzle()? {
            AluDst::Value {
                gpr,
                alu_rel,
                swizzle,
            }
        }
        / m:write_mask() { AluDst::WriteMask(WriteMask(m.to_string())) }
    rule alu_src() -> AluSrc
        = negate:negate()? _ value:alu_src_value_or_abs() _ alu_rel:alu_rel()? _ swizzle:one_comp_swizzle()? {
            AluSrc {
                negate,
                value,
                alu_rel,
                swizzle,
            }
        }
    rule alu_src_value_or_abs() -> AluSrcValueOrAbs
        = src:alu_abs_src_value() { AluSrcValueOrAbs::Abs(src) }
        / src:alu_src_value() { AluSrcValueOrAbs::Value(src) }
    rule alu_abs_src_value() -> AluAbsSrcValue
        = "|" _ value:alu_src_value() _ swizzle:one_comp_swizzle()? _ "|" {
            AluAbsSrcValue { value, swizzle }
        }
    rule alu_src_value() -> AluSrcValue
        = v:gpr() { AluSrcValue::Gpr(v) }
        / v:constant_cache0() { AluSrcValue::ConstantCache0(ConstantCache0(v)) }
        / v:constant_cache1() { AluSrcValue::ConstantCache1(ConstantCache1(v)) }
        / v:constant_file() { AluSrcValue::ConstantFile(ConstantFile(v)) }
        / v:literal() { AluSrcValue::Literal(v) }
        / v:previous_scalar() { AluSrcValue::PreviousScalar(PreviousScalar(v)) }
        / v:previous_vector() { AluSrcValue::PreviousVector(PreviousVector(v)) }
    rule alu_properties() -> Vec<AluProperty> = props:(alu_property() ** _) { props }
    rule alu_property() -> AluProperty
        = clamp() { AluProperty::Clamp }
        / (bank_swizzle() / update_exec_mask() / update_pred() / pred_sel()) { AluProperty::Unk }
    rule update_pred() = "UPDATE_PRED"
    rule pred_sel() = "PRED_SEL_OFF" / "PRED_SEL_ZERO" / "PRED_SEL_ONE"
    rule clamp() = "CLAMP"
    rule update_exec_mask() = "UPDATE_EXEC_MASK" _ ("(" _ execute_mask_op() _ ")")?
    rule execute_mask_op() = "DEACTIVATE" / "BREAK" / "CONTINUE" / "KILL"
    rule bank_swizzle() = "SCL_210" / "SCL_122" / "SCL_212" / "SCL_221" / "VEC_012" / "VEC_021" / "VEC_120" / "VEC_102" / "VEC_201" / "VEC_210"

    // Define a rule for whitespace.
    rule _ = [' ' | '\t' | '\r' | '\n']*
}}

#[derive(Debug)]
enum Instruction {
    CfInst(CfInst),
    CfExpInst(CfExpInst),
    TexClause(TexClause),
    AluClause(AluClause),
}

#[allow(dead_code)]
#[derive(Debug)]
struct CfInst {
    inst_count: InstCount,
    opcode: CfOpcode,
    properties: CfInstProperties,
}

#[derive(Debug)]
struct CfOpcode;

#[derive(Debug)]
struct CfInstProperties(Vec<CfInstProperty>);

#[derive(Debug)]
enum CfInstProperty {
    Burstcnt(BurstCnt),
    Unk,
}

#[derive(Debug)]
struct BurstCnt(usize);

#[allow(dead_code)]
#[derive(Debug)]
struct CfExpInst {
    inst_count: InstCount,
    opcode: ExpOpcode,
    target: ExpTarget,
    src: ExpSrc,
    properties: CfInstProperties,
}

#[allow(dead_code)]
#[derive(Debug)]
struct ExpOpcode(String);

#[derive(Debug)]
enum ExpTarget {
    Pix(ExpPixTarget),
    Pos(ExpPosTarget),
    Param(ExpParamTarget),
}

#[derive(Debug)]
struct ExpPixTarget(usize);

#[derive(Debug)]
struct ExpPosTarget(usize);

#[derive(Debug)]
struct ExpParamTarget(usize);

#[derive(Debug)]
struct ExpSrc {
    gpr: Gpr, // TODO: Gpr or GprRel
    swizzle: FourCompSwizzle,
}

#[allow(dead_code)]
#[derive(Debug)]
struct TexClause {
    inst_count: InstCount,
    inst_type: TexClauseInstType,
    properties: TexClauseProperties,
    instructions: Vec<TexInstOrFetchInst>,
}

#[derive(Debug)]
struct TexClauseInstType;

#[allow(dead_code)]
#[derive(Debug)]
struct TexClauseProperties(Vec<TexClauseProperty>);

#[derive(Debug)]
enum TexClauseProperty {
    Unk,
}

#[derive(Debug)]
enum TexInstOrFetchInst {
    Tex(TexInst),
    Fetch(FetchInst),
}

#[allow(dead_code)]
#[derive(Debug)]
struct TexInst {
    inst_count: InstCount,
    opcode: TexOpcode,
    dst: TexDst,
    src: TexSrc,
    resource_id: TexResourceId,
    sampler_id: TexSamplerId,
    properties: TexProperties,
}

#[allow(dead_code)]
#[derive(Debug)]
struct TexOpcode(String);

#[derive(Debug)]
struct TexResourceId(String);

#[allow(dead_code)]
#[derive(Debug)]
struct TexSamplerId(String);

#[allow(dead_code)]
#[derive(Debug)]
struct TexDst {
    gpr: Gpr,
    tex_rel: Option<TexRel>,
    swizzle: FourCompSwizzle,
}

#[allow(dead_code)]
#[derive(Debug)]
struct TexSrc {
    gpr: Gpr,
    tex_rel: Option<TexRel>,
    swizzle: FourCompSwizzle,
}

#[derive(Debug)]
struct TexRel;

#[derive(Debug)]
struct TexProperties;

#[allow(dead_code)]
#[derive(Debug)]
struct FetchInst {
    inst_count: InstCount,
    dst: FetchDst,
    src: FetchSrc,
    buffer_id: usize,
    properties: Vec<FetchProperty>,
}

#[derive(Debug)]
struct FetchDst {
    gpr: Gpr,
    swizzle: FourCompSwizzle,
}

#[derive(Debug)]
struct FetchSrc {
    gpr: Gpr,
    swizzle: Option<char>,
}

#[derive(Debug)]
struct FetchType {}

#[allow(dead_code)]
#[derive(Debug)]
struct FetchMega(usize);

#[allow(dead_code)]
#[derive(Debug)]
struct FetchOffset(usize);

#[allow(dead_code)]
#[derive(Debug)]
enum FetchProperty {
    Type(FetchType),
    Mega(FetchMega),
    Offset(FetchOffset),
}

#[allow(dead_code)]
#[derive(Debug)]
struct AluClause {
    inst_count: InstCount,
    inst_type: AluClauseInstType,
    properties: AluClauseProperties,
    groups: Vec<AluGroup>,
}

#[derive(Debug)]
struct AluClauseInstType;

#[derive(Debug)]
struct AluClauseProperties(Vec<AluClauseProperty>);

#[derive(Debug)]
enum AluClauseProperty {
    KCache0(ConstantBuffer),
    KCache1(ConstantBuffer),
    Unk,
}

#[derive(Debug)]
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

#[allow(dead_code)]
#[derive(Debug)]
struct AluScalar0 {
    alu_unit: AluUnit,
    opcode: AluOpCode0,
    modifier: Option<AluOutputModifier>,
    dst: AluDst,
    properties: Vec<AluProperty>,
}

#[allow(dead_code)]
#[derive(Debug)]
struct AluScalar1 {
    alu_unit: AluUnit,
    opcode: AluOpCode1,
    modifier: Option<AluOutputModifier>,
    dst: AluDst,
    src1: AluSrc,
    properties: Vec<AluProperty>,
}

#[allow(dead_code)]
#[derive(Debug)]
struct AluScalar2 {
    alu_unit: AluUnit,
    opcode: AluOpCode2,
    modifier: Option<AluOutputModifier>,
    dst: AluDst,
    src1: AluSrc,
    src2: AluSrc,
    properties: Vec<AluProperty>,
}

#[allow(dead_code)]
#[derive(Debug)]
struct AluScalar3 {
    alu_unit: AluUnit,
    opcode: AluOpCode3,
    dst: AluDst,
    src1: AluSrc,
    src2: AluSrc,
    src3: AluSrc,
    properties: Vec<AluProperty>,
}

#[allow(dead_code)]
#[derive(Debug)]
struct WriteMask(String);

#[allow(dead_code)]
#[derive(Debug)]
enum AluDst {
    Value {
        gpr: Gpr,
        alu_rel: Option<AluRel>,
        swizzle: Option<char>,
    },
    WriteMask(WriteMask),
}

#[allow(dead_code)]
#[derive(Debug)]
struct AluSrc {
    negate: Option<Negate>,
    value: AluSrcValueOrAbs,
    alu_rel: Option<AluRel>,
    swizzle: Option<char>,
}

#[derive(Debug)]
enum AluSrcValueOrAbs {
    Abs(AluAbsSrcValue),
    Value(AluSrcValue),
}

#[derive(Debug)]
enum AluSrcValue {
    Gpr(Gpr),
    ConstantCache0(ConstantCache0),
    ConstantCache1(ConstantCache1),
    ConstantFile(ConstantFile),
    Literal(Literal),
    PreviousScalar(PreviousScalar),
    PreviousVector(PreviousVector),
}

#[allow(dead_code)]
#[derive(Debug)]
enum Literal {
    Hex(String),
    Float(String),
}

#[derive(Debug)]
struct ConstantCache0(usize);

#[derive(Debug)]
struct ConstantCache1(usize);

#[derive(Debug)]
struct ConstantFile(usize);

#[derive(Debug)]
struct PreviousScalar(usize);

#[derive(Debug)]
struct PreviousVector(usize);

#[derive(Debug)]
struct AluAbsSrcValue {
    value: AluSrcValue,
    swizzle: Option<char>,
}

#[derive(Debug)]
struct AluRel;

#[derive(Debug)]
struct AluUnit(String);

#[derive(Debug)]
struct Negate;

#[derive(Debug)]
struct AluOutputModifier(String);

#[derive(Debug)]
struct AluOpCode0(String);

#[derive(Debug)]
struct AluOpCode1(String);

#[derive(Debug)]
struct AluOpCode2(String);

#[derive(Debug)]
struct AluOpCode3(String);

#[derive(Debug, PartialEq, Eq)]
enum AluProperty {
    Clamp,
    Unk,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
struct InstCount(usize);

#[derive(Debug)]
struct FourCompSwizzle(String);

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
struct Gpr(usize);

impl std::fmt::Display for InstCount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::fmt::Display for Gpr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "R{}", self.0)
    }
}

impl std::fmt::Display for PreviousVector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PV{}", self.0)
    }
}

impl std::fmt::Display for PreviousScalar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PS{}", self.0)
    }
}

#[derive(Default)]
struct Nodes {
    nodes: Vec<Node>,
    exprs: IndexSet<Expr>,
}

impl Nodes {
    fn node(&mut self, node: Node) -> usize {
        let index = self.nodes.len();
        self.nodes.push(node);
        index
    }

    fn set_float_node(&mut self, op: BinaryOp, scalar: &AluScalarData, output: Output) -> usize {
        let input = Expr::Ternary(
            self.binary_expr(op, scalar.sources[0].clone(), scalar.sources[1].clone()),
            self.expr(Expr::Float(1.0.into())),
            self.expr(Expr::Float(0.0.into())),
        );
        let node = Node {
            output,
            input: self.expr(input),
        };
        self.node(node)
    }

    fn set_float_dx10_node(
        &mut self,
        op: BinaryOp,
        scalar: &AluScalarData,
        output: Output,
    ) -> usize {
        let input = Expr::Ternary(
            self.binary_expr(op, scalar.sources[0].clone(), scalar.sources[1].clone()),
            self.unary_expr(UnaryOp::IntBitsToFloat, Expr::Int(-1)),
            self.unary_expr(UnaryOp::IntBitsToFloat, Expr::Int(0)),
        );
        let node = Node {
            output,
            input: self.expr(input),
        };
        self.node(node)
    }

    fn set_int_node(&mut self, op: BinaryOp, scalar: &AluScalarData, output: Output) -> usize {
        let src0 = self.float_to_int_expr(scalar.sources[0].clone());
        let src1 = self.float_to_int_expr(scalar.sources[1].clone());
        let input = Expr::Ternary(
            self.expr(Expr::Binary(op, src0, src1)),
            self.unary_expr(UnaryOp::IntBitsToFloat, Expr::Int(-1)),
            self.unary_expr(UnaryOp::IntBitsToFloat, Expr::Int(0)),
        );
        let node = Node {
            output,
            input: self.expr(input),
        };
        self.node(node)
    }

    fn cnd_float_node(&mut self, op: BinaryOp, scalar: &AluScalarData, output: Output) -> usize {
        let input = Expr::Ternary(
            self.binary_expr(op, scalar.sources[0].clone(), Expr::Float(0.0.into())),
            self.expr(scalar.sources[1].clone()),
            self.expr(scalar.sources[2].clone()),
        );
        let node = Node {
            output,
            input: self.expr(input),
        };
        self.node(node)
    }

    fn cnd_int_node(&mut self, op: BinaryOp, scalar: &AluScalarData, output: Output) -> usize {
        let src0 = self.float_to_int_expr(scalar.sources[0].clone());
        let cmp = self.expr(Expr::Int(0));
        let input = Expr::Ternary(
            self.expr(Expr::Binary(op, src0, cmp)),
            self.expr(scalar.sources[1].clone()),
            self.expr(scalar.sources[2].clone()),
        );
        let node = Node {
            output,
            input: self.expr(input),
        };
        self.node(node)
    }

    fn func_node(
        &mut self,
        func: &str,
        arg_count: usize,
        scalar: &AluScalarData,
        output: Output,
    ) -> usize {
        let args = scalar
            .sources
            .iter()
            .take(arg_count)
            .map(|a| self.expr(a.clone()))
            .collect();
        let input = self.func_expr(func, args);
        let node = Node { output, input };
        self.node(node)
    }

    fn binary_node(&mut self, op: BinaryOp, lh: Expr, rh: Expr, output: Output) -> usize {
        let input = self.binary_expr(op, lh, rh);
        let node = Node { output, input };
        self.node(node)
    }

    fn unary_node(&mut self, op: UnaryOp, e: Expr, output: Output) -> usize {
        let input = self.unary_expr(op, e);
        let node = Node { output, input };
        self.node(node)
    }

    fn expr(&mut self, expr: Expr) -> usize {
        self.exprs.insert_full(expr).0
    }

    fn unary_expr(&mut self, op: UnaryOp, e: Expr) -> usize {
        let result = Expr::Unary(op, self.expr(e));
        self.expr(result)
    }

    fn binary_expr(&mut self, op: BinaryOp, lh: Expr, rh: Expr) -> usize {
        let result = Expr::Binary(op, self.expr(lh), self.expr(rh));
        self.expr(result)
    }

    fn float_to_int_expr(&mut self, expr: Expr) -> usize {
        // Convert float literals directly to integers.
        let result = match expr {
            Expr::Float(f) => Expr::Int(f.to_bits() as i32),
            e => Expr::Unary(UnaryOp::FloatBitsToInt, self.expr(e)),
        };
        self.expr(result)
    }

    fn float_to_uint_expr(&mut self, expr: Expr) -> usize {
        // Convert float literals directly to integers.
        let result = match expr {
            Expr::Float(f) => Expr::Uint(f.to_bits()),
            e => Expr::Unary(UnaryOp::FloatBitsToUint, self.expr(e)),
        };
        self.expr(result)
    }

    fn clamp_expr(&mut self, e: Expr) -> usize {
        let arg0 = self.expr(e);
        let arg1 = self.expr(Expr::Float(0.0.into()));
        let arg2 = self.expr(Expr::Float(1.0.into()));
        self.func_expr("clamp", vec![arg0, arg1, arg2])
    }

    fn func_expr(&mut self, name: &str, args: Vec<usize>) -> usize {
        self.expr(Expr::Func {
            name: name.into(),
            args,
            channel: None,
        })
    }

    fn previous_assignment_expr(&mut self, value: &str, channel: Option<char>) -> usize {
        self.expr(previous_assignment(value, channel, self))
    }
}

impl Graph {
    pub fn from_latte_asm(asm: &str) -> Result<Self, LatteError> {
        if asm.is_empty() {
            return Ok(Graph::default());
        }

        let instructions = latte_parser::program(asm.trim())?;

        let mut nodes = Nodes::default();

        for instruction in instructions {
            match instruction {
                Instruction::CfInst(_inst) => (),
                Instruction::CfExpInst(inst) => add_exp_inst(inst, &mut nodes),
                Instruction::TexClause(inst) => add_tex_clause(inst, &mut nodes),
                Instruction::AluClause(inst) => add_alu_clause(inst, &mut nodes),
            }
        }

        Ok(Self {
            nodes: nodes.nodes,
            exprs: nodes.exprs.into_iter().collect(),
        })
    }
}

fn add_exp_inst(exp: CfExpInst, nodes: &mut Nodes) {
    let (target_name, target_index) = match exp.target {
        ExpTarget::Pix(t) => ("PIX", t.0),
        ExpTarget::Pos(t) => ("POS", t.0),
        ExpTarget::Param(t) => ("PARAM", t.0),
    };

    let source_name = "R";
    let source_index = exp.src.gpr.0;
    let channels = exp.src.swizzle.0;

    let burst_count = exp
        .properties
        .0
        .iter()
        .find_map(|p| {
            if let CfInstProperty::Burstcnt(burstcnt) = p {
                Some(burstcnt.0)
            } else {
                None
            }
        })
        .unwrap_or_default();

    // BURSTCNT assigns consecutive input and output registers.
    for i in 0..=burst_count {
        // Output names don't conflict with register names, so we don't need to backup any values.
        for c in channels.chars() {
            let node = Node {
                output: Output {
                    name: format!("{target_name}{}", target_index + i).into(),
                    channel: Some(c),
                },
                input: nodes.previous_assignment_expr(
                    &format!("{source_name}{}", source_index + i),
                    Some(c),
                ),
            };
            nodes.node(node);
        }
    }
}

fn add_tex_clause(clause: TexClause, nodes: &mut Nodes) {
    for tex_instruction in clause.instructions {
        match tex_instruction {
            TexInstOrFetchInst::Tex(tex_inst) => {
                let tex_nodes = tex_inst_node(tex_inst, nodes).unwrap();
                for node in tex_nodes {
                    nodes.node(node);
                }
            }
            TexInstOrFetchInst::Fetch(fetch_inst) => {
                let fetch_nodes = fetch_inst_node(fetch_inst, nodes).unwrap();
                for node in fetch_nodes {
                    nodes.node(node);
                }
            }
        }
    }
}

struct AluScalarData {
    alu_unit: char,
    op_code: String,
    output_modifier: Option<String>,
    properties: Vec<AluProperty>,
    output: Output,
    sources: Vec<Expr>,
}

#[allow(dead_code)]
#[derive(Debug)]
struct ConstantBuffer {
    index: usize,
    start_index: usize,
    end_index: usize,
}

fn add_alu_clause(clause: AluClause, nodes: &mut Nodes) {
    // It's not safe to assume that PV and PS values use a write mask instead of register output.
    // Check if values need to be backed up to use for later instructions.
    // TODO: Should this list be done for all clauses?
    let previous_vectors_scalars = used_previous_vectors_scalars(&clause.groups);

    let properties = &clause.properties;

    for group in clause.groups {
        let inst_count = group.inst_count;

        let scalars = get_scalar_data(properties, nodes, group, inst_count);

        let dot_node_index = dot_product_node_index(&scalars, inst_count, nodes);

        for scalar in scalars {
            let mut final_node_index = None;

            // Reduction instructions write a single result to all vector outputs.
            // TODO: Cube instructions are also reduction instructions.
            if scalar.op_code.starts_with("DOT4") {
                if let Some(node_index) = dot_node_index {
                    let node = Node {
                        output: scalar.output.clone(),
                        input: nodes.expr(Expr::Node {
                            node_index,
                            channel: None,
                        }),
                    };
                    final_node_index = Some(nodes.node(node));
                }
            } else {
                final_node_index = add_scalar(&scalar, nodes);
            }

            if let Some(node_index) = final_node_index {
                // TODO: Is there a better way to handle alu units with write masks?
                if !scalar.output.name.starts_with("PV") && !scalar.output.name.starts_with("PS") {
                    let (name, channel): (SmolStr, _) = match scalar.alu_unit {
                        c @ ('x' | 'y' | 'z' | 'w') => (format!("PV{inst_count}").into(), Some(c)),
                        't' => (format!("PS{inst_count}").into(), None),
                        _ => unreachable!(),
                    };
                    if previous_vectors_scalars.contains(&(name.clone(), channel)) {
                        let node = Node {
                            output: Output { name, channel },
                            input: nodes.expr(Expr::Node {
                                node_index,
                                channel: scalar.output.channel,
                            }),
                        };
                        nodes.node(node);
                    }
                }
            }
        }
    }
}

fn get_scalar_data(
    properties: &AluClauseProperties,
    nodes: &mut Nodes,
    group: AluGroup,
    inst_count: InstCount,
) -> Vec<AluScalarData> {
    // Ranges from constant buffers are mapped to constant cache KC0 and KC1.
    // These mappings persist for the duration of the ALU clause.
    let mut kc0_buffer = None;
    let mut kc1_buffer = None;
    for prop in &properties.0 {
        match prop {
            AluClauseProperty::KCache0(kc) => kc0_buffer = Some(kc),
            AluClauseProperty::KCache1(kc) => kc1_buffer = Some(kc),
            _ => (),
        }
    }

    // Backup values if the assigned value is read after being modified.
    let backup_gprs = backup_gprs(&group);

    for (i, channel) in &backup_gprs {
        let name = format!("R{i}");
        let node = Node {
            output: Output {
                name: format!("{name}_backup").into(),
                channel: *channel,
            },
            input: nodes.previous_assignment_expr(&name, *channel),
        };
        nodes.node(node);
    }

    group
        .scalars
        .into_iter()
        .map(|scalar| match scalar {
            AluScalar::Scalar0(s) => {
                let alu_unit = s.alu_unit.0.chars().next().unwrap();
                AluScalarData {
                    alu_unit,
                    op_code: s.opcode.0,
                    output_modifier: s.modifier.map(|m| m.0),
                    properties: s.properties,
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
                    properties: s.properties,
                    output: alu_dst_output(s.dst, inst_count, alu_unit),
                    sources: vec![alu_src_expr(
                        s.src1,
                        nodes,
                        kc0_buffer,
                        kc1_buffer,
                        &backup_gprs,
                    )],
                }
            }
            AluScalar::Scalar2(s) => {
                let alu_unit = s.alu_unit.0.chars().next().unwrap();
                AluScalarData {
                    alu_unit,
                    op_code: s.opcode.0,
                    output_modifier: s.modifier.map(|m| m.0),
                    properties: s.properties,
                    output: alu_dst_output(s.dst, inst_count, alu_unit),
                    sources: vec![
                        alu_src_expr(s.src1, nodes, kc0_buffer, kc1_buffer, &backup_gprs),
                        alu_src_expr(s.src2, nodes, kc0_buffer, kc1_buffer, &backup_gprs),
                    ],
                }
            }
            AluScalar::Scalar3(s) => {
                let alu_unit = s.alu_unit.0.chars().next().unwrap();
                AluScalarData {
                    alu_unit,
                    op_code: s.opcode.0,
                    output_modifier: None,
                    properties: s.properties,
                    output: alu_dst_output(s.dst, inst_count, alu_unit),
                    sources: vec![
                        alu_src_expr(s.src1, nodes, kc0_buffer, kc1_buffer, &backup_gprs),
                        alu_src_expr(s.src2, nodes, kc0_buffer, kc1_buffer, &backup_gprs),
                        alu_src_expr(s.src3, nodes, kc0_buffer, kc1_buffer, &backup_gprs),
                    ],
                }
            }
        })
        .collect()
}

fn backup_gprs(group: &AluGroup) -> BTreeSet<(usize, Option<char>)> {
    let mut write_gprs = BTreeSet::new();
    let mut backup_gprs = BTreeSet::new();
    for s in &group.scalars {
        match s {
            AluScalar::Scalar0(s) => {
                insert_write_gpr(&mut write_gprs, &s.dst);
            }
            AluScalar::Scalar1(s) => {
                insert_backup_gprs(&write_gprs, &mut backup_gprs, &s.src1);

                insert_write_gpr(&mut write_gprs, &s.dst);
            }
            AluScalar::Scalar2(s) => {
                insert_backup_gprs(&write_gprs, &mut backup_gprs, &s.src1);
                insert_backup_gprs(&write_gprs, &mut backup_gprs, &s.src2);

                insert_write_gpr(&mut write_gprs, &s.dst);
            }
            AluScalar::Scalar3(s) => {
                insert_backup_gprs(&write_gprs, &mut backup_gprs, &s.src1);
                insert_backup_gprs(&write_gprs, &mut backup_gprs, &s.src2);
                insert_backup_gprs(&write_gprs, &mut backup_gprs, &s.src3);

                insert_write_gpr(&mut write_gprs, &s.dst);
            }
        }
    }
    backup_gprs
}

fn insert_backup_gprs(
    write_gprs: &BTreeSet<(usize, Option<char>)>,
    backup_gprs: &mut BTreeSet<(usize, Option<char>)>,
    src: &AluSrc,
) {
    // Registers that are read after being written need to be backed up.
    // The reads should still use the old value for this ALU group.
    match &src.value {
        AluSrcValueOrAbs::Abs(v) => {
            if let AluSrcValue::Gpr(gpr) = &v.value {
                let key = (gpr.0, v.swizzle);
                if write_gprs.contains(&key) {
                    backup_gprs.insert(key);
                }
            }
        }
        AluSrcValueOrAbs::Value(v) => {
            if let AluSrcValue::Gpr(gpr) = v {
                let key = (gpr.0, src.swizzle);
                if write_gprs.contains(&key) {
                    backup_gprs.insert(key);
                }
            }
        }
    }
}

fn insert_write_gpr(write_gprs: &mut BTreeSet<(usize, Option<char>)>, dst: &AluDst) {
    match dst {
        AluDst::Value { gpr, swizzle, .. } => {
            write_gprs.insert((gpr.0, *swizzle));
        }
        AluDst::WriteMask(_) => (),
    }
}

fn used_previous_vectors_scalars(groups: &[AluGroup]) -> BTreeSet<(SmolStr, Option<char>)> {
    let mut values = BTreeSet::new();
    for group in groups {
        for s in &group.scalars {
            match s {
                AluScalar::Scalar0(_) => (),
                AluScalar::Scalar1(s) => {
                    insert_previous_vector_scalars(&mut values, &s.src1);
                }
                AluScalar::Scalar2(s) => {
                    insert_previous_vector_scalars(&mut values, &s.src1);
                    insert_previous_vector_scalars(&mut values, &s.src2);
                }
                AluScalar::Scalar3(s) => {
                    insert_previous_vector_scalars(&mut values, &s.src1);
                    insert_previous_vector_scalars(&mut values, &s.src2);
                    insert_previous_vector_scalars(&mut values, &s.src3);
                }
            }
        }
    }
    values
}

fn insert_previous_vector_scalars(values: &mut BTreeSet<(SmolStr, Option<char>)>, src: &AluSrc) {
    match &src.value {
        AluSrcValueOrAbs::Abs(v) => match &v.value {
            AluSrcValue::PreviousScalar(ps) => {
                values.insert((ps.to_string().into(), src.swizzle));
            }
            AluSrcValue::PreviousVector(pv) => {
                values.insert((pv.to_string().into(), src.swizzle));
            }
            _ => (),
        },
        AluSrcValueOrAbs::Value(v) => match v {
            AluSrcValue::PreviousScalar(ps) => {
                values.insert((ps.to_string().into(), src.swizzle));
            }
            AluSrcValue::PreviousVector(pv) => {
                values.insert((pv.to_string().into(), src.swizzle));
            }
            _ => (),
        },
    }
}

fn dot_product_node_index(
    scalars: &[AluScalarData],
    inst_count: InstCount,
    nodes: &mut Nodes,
) -> Option<usize> {
    let (dot4_a, dot4_b): (Vec<_>, Vec<_>) = scalars
        .iter()
        .filter_map(|s| {
            if s.op_code.starts_with("DOT4") {
                Some((
                    nodes.expr(s.sources[0].clone()),
                    nodes.expr(s.sources[1].clone()),
                ))
            } else {
                None
            }
        })
        .unzip();
    if !dot4_a.is_empty() && !dot4_b.is_empty() {
        let args = vec![
            nodes.func_expr("vec4", dot4_a),
            nodes.func_expr("vec4", dot4_b),
        ];
        let input = nodes.func_expr("dot", args);
        let temp_name: SmolStr = format!("temp{inst_count}").into();
        let node = Node {
            output: Output {
                name: temp_name.clone(),
                channel: None,
            },
            input,
        };
        let mut node_index = nodes.node(node);

        // Assume the clamp is applied to all xyzw scalars.
        if scalars.iter().any(|s| {
            s.op_code.starts_with("DOT4") && s.properties.iter().any(|p| p == &AluProperty::Clamp)
        }) {
            let input = nodes.clamp_expr(Expr::Node {
                node_index,
                channel: None,
            });
            node_index = nodes.node(Node {
                output: Output {
                    name: temp_name,
                    channel: None,
                },
                input,
            })
        }

        Some(node_index)
    } else {
        None
    }
}

fn add_scalar(scalar: &AluScalarData, nodes: &mut Nodes) -> Option<usize> {
    let output = scalar.output.clone();
    let node_index = match scalar.op_code.as_str() {
        // scalar1
        "MOV" => {
            let node = Node {
                output,
                input: nodes.expr(scalar.sources[0].clone()),
            };
            Some(nodes.node(node))
        }
        "FLOOR" => Some(nodes.func_node("floor", 1, scalar, output)),
        "SQRT_IEEE" => Some(nodes.func_node("sqrt", 1, scalar, output)),
        "RECIP_IEEE" => Some(nodes.binary_node(
            BinaryOp::Div,
            Expr::Float(1.0.into()),
            scalar.sources[0].clone(),
            output,
        )),
        "RECIP_FF" => {
            // TODO: Set result of +inf to +0 and -inf to -0.
            Some(nodes.binary_node(
                BinaryOp::Div,
                Expr::Float(1.0.into()),
                scalar.sources[0].clone(),
                output,
            ))
        }
        "RECIPSQRT_IEEE" => Some(nodes.func_node("inversesqrt", 1, scalar, output)),
        "RECIPSQRT_FF" => {
            // TODO: Set result of +inf to +0 and -inf to -0.
            Some(nodes.func_node("inversesqrt", 1, scalar, output))
        }
        "EXP_IEEE" => Some(nodes.func_node("exp2", 1, scalar, output)),
        "LOG_CLAMPED" => {
            // TODO: -inf to -max_float
            Some(nodes.func_node("log2", 1, scalar, output))
        }
        // scalar2
        "ADD" => Some(nodes.binary_node(
            BinaryOp::Add,
            scalar.sources[0].clone(),
            scalar.sources[1].clone(),
            output,
        )),
        "ADD_INT" => {
            let result = Expr::Binary(
                BinaryOp::Add,
                nodes.float_to_int_expr(scalar.sources[0].clone()),
                nodes.float_to_int_expr(scalar.sources[1].clone()),
            );
            let input = nodes.unary_expr(UnaryOp::IntBitsToFloat, result);
            let node = Node { output, input };
            Some(nodes.node(node))
        }
        "MIN" | "MIN_DX10" => Some(nodes.func_node("min", 2, scalar, output)),
        "MAX" | "MAX_DX10" => Some(nodes.func_node("max", 2, scalar, output)),
        // Scalar multiplication with floats.
        "MUL" | "MUL_IEEE" => Some(nodes.binary_node(
            BinaryOp::Mul,
            scalar.sources[0].clone(),
            scalar.sources[1].clone(),
            output,
        )),
        // Reduction instructions handled in a previous check.
        "DOT4" | "DOT4_IEEE" => unreachable!(),
        "MULLO_UINT" => {
            // Scalar multiplication with unsigned integers stored in the lower bits.
            let result = Expr::Binary(
                BinaryOp::Mul,
                nodes.float_to_uint_expr(scalar.sources[0].clone()),
                nodes.float_to_uint_expr(scalar.sources[1].clone()),
            );
            let input = nodes.unary_expr(UnaryOp::UintBitsToFloat, result);
            let node = Node { output, input };
            Some(nodes.node(node))
        }
        "MULLO_INT" => {
            // Scalar multiplication with signed integers stored in the lower bits.
            let result = Expr::Binary(
                BinaryOp::Mul,
                nodes.float_to_int_expr(scalar.sources[0].clone()),
                nodes.float_to_int_expr(scalar.sources[1].clone()),
            );
            let input = nodes.unary_expr(UnaryOp::IntBitsToFloat, result);
            let node = Node { output, input };
            Some(nodes.node(node))
        }
        // scalar3
        "MULADD" | "MULADD_IEEE" => Some(nodes.func_node("fma", 3, scalar, output)),
        "MULADD_M2" => {
            let node_index = nodes.func_node("fma", 3, scalar, output.clone());
            Some(nodes.binary_node(
                BinaryOp::Mul,
                Expr::Node {
                    node_index,
                    channel: scalar.output.channel,
                },
                Expr::Float(2.0.into()),
                output,
            ))
        }
        "MULADD_M4" => {
            let node_index = nodes.func_node("fma", 3, scalar, output.clone());
            Some(nodes.binary_node(
                BinaryOp::Mul,
                Expr::Node {
                    node_index,
                    channel: scalar.output.channel,
                },
                Expr::Float(4.0.into()),
                output,
            ))
        }
        "MULADD_D2" => {
            let node_index = nodes.func_node("fma", 3, scalar, output.clone());
            Some(nodes.binary_node(
                BinaryOp::Div,
                Expr::Node {
                    node_index,
                    channel: scalar.output.channel,
                },
                Expr::Float(2.0.into()),
                output,
            ))
        }
        "MULADD_D4" => {
            let node_index = nodes.func_node("fma", 3, scalar, output.clone());
            Some(nodes.binary_node(
                BinaryOp::Div,
                Expr::Node {
                    node_index,
                    channel: scalar.output.channel,
                },
                Expr::Float(4.0.into()),
                output,
            ))
        }
        "NOP" => None,
        "FLT_TO_INT" => {
            Some(nodes.unary_node(UnaryOp::FloatToInt, scalar.sources[0].clone(), output))
        }
        "FLT_TO_UINT" => {
            Some(nodes.unary_node(UnaryOp::FloatToUint, scalar.sources[0].clone(), output))
        }
        "INT_TO_FLT" => {
            Some(nodes.unary_node(UnaryOp::IntToFloat, scalar.sources[0].clone(), output))
        }
        "UINT_TO_FLT" => {
            Some(nodes.unary_node(UnaryOp::UintToFloat, scalar.sources[0].clone(), output))
        }
        "SIN" => Some(nodes.func_node("sin", 1, scalar, output)),
        "COS" => Some(nodes.func_node("cos", 1, scalar, output)),
        "FRACT" => Some(nodes.func_node("fract", 1, scalar, output)),
        "CUBE" => {
            // TODO: proper reduction instruction for cube maps
            let input = nodes.expr(Expr::Float(0.0.into()));
            let node = Node { output, input };
            Some(nodes.node(node))
        }
        // TODO: push/pop and predicates
        "PRED_SETGE" | "PRED_SETGT" => None,
        // Conditionals
        "KILLE_INT" => {
            // TODO: if src0 == src1 kill and set dst to 1.0 else set dst to 0.0
            None
        }
        // Floating-point set if equal.
        "SETE" => Some(nodes.set_float_node(BinaryOp::Equal, scalar, output)),
        // Integer set if equal.
        "SETE_INT" => Some(nodes.set_int_node(BinaryOp::Equal, scalar, output)),
        // Floating-point set if equal with an integer result.
        "SETE_DX10" => Some(nodes.set_float_dx10_node(BinaryOp::Equal, scalar, output)),
        // Floating-point set if not equal.
        "SETNE" => Some(nodes.set_float_node(BinaryOp::NotEqual, scalar, output)),
        // Integer set if not equal.
        "SETNE_INT" => Some(nodes.set_int_node(BinaryOp::NotEqual, scalar, output)),
        // Floating-point set if not equal with an integer result.
        "SETNE_DX10" => Some(nodes.set_float_dx10_node(BinaryOp::Equal, scalar, output)),
        // Floating-point set if greater than.
        "SETGT" => Some(nodes.set_float_node(BinaryOp::Greater, scalar, output)),
        // Integer set if greater than.
        "SETGT_INT" => Some(nodes.set_int_node(BinaryOp::Greater, scalar, output)),
        // Floating-point set if greater than with an integer result.
        "SETGT_DX10" => Some(nodes.set_float_dx10_node(BinaryOp::Greater, scalar, output)),
        // Floating-point set if greater than or equal.
        "SETGE" => Some(nodes.set_float_node(BinaryOp::GreaterEqual, scalar, output)),
        // Integer set if greater than or equal.
        "SETGE_INT" => Some(nodes.set_int_node(BinaryOp::GreaterEqual, scalar, output)),
        // Floating-point set if geq with an integer result.
        "SETGE_DX10" => Some(nodes.set_float_dx10_node(BinaryOp::GreaterEqual, scalar, output)),
        // Floating-point conditional move if equal 0.0.
        "CNDE" => Some(nodes.cnd_float_node(BinaryOp::Equal, scalar, output)),
        // Integer conditional move if equal 0.
        "CNDE_INT" => Some(nodes.cnd_int_node(BinaryOp::Equal, scalar, output)),
        // Floating-point conditional move if greater than 0.0.
        "CNDGT" => Some(nodes.cnd_float_node(BinaryOp::Greater, scalar, output)),
        // Integer conditional move if greater than 0.
        "CNDGT_INT" => Some(nodes.cnd_int_node(BinaryOp::Greater, scalar, output)),
        // Floating-point conditional move if greater than or equal 0.0.
        "CNDGE" => Some(nodes.cnd_float_node(BinaryOp::GreaterEqual, scalar, output)),
        // Integer conditional move if greater than or equal 0.
        "CNDGE_INT" => Some(nodes.cnd_int_node(BinaryOp::GreaterEqual, scalar, output)),
        opcode => {
            // TODO: Handle additional opcodes?
            error!("Unsupported opcode {opcode}");
            None
        }
    }?;

    let final_node_index = add_alu_output_modifiers(nodes, scalar, node_index);
    Some(final_node_index)
}

fn alu_dst_output(dst: AluDst, inst_count: InstCount, alu_unit: char) -> Output {
    match dst {
        AluDst::Value {
            gpr,
            alu_rel: _,
            swizzle,
        } => Output {
            name: gpr.to_smolstr(),
            channel: swizzle,
        },
        AluDst::WriteMask(_write_mask) => {
            match alu_unit {
                // ____ mask for xyzw writes to a previous vector "PV".
                c @ ('x' | 'y' | 'z' | 'w') => Output {
                    name: format!("PV{inst_count}").into(),
                    channel: Some(c),
                },
                // ____ mask for t writes to a previous scalar "PS".
                't' => Output {
                    name: format!("PS{inst_count}").into(),
                    channel: None,
                },
                _ => unreachable!(),
            }
        }
    }
}

fn add_alu_output_modifiers(nodes: &mut Nodes, scalar: &AluScalarData, node_index: usize) -> usize {
    let node_index = alu_output_modifier_scale(scalar, node_index, nodes).unwrap_or(node_index);

    if scalar.properties.iter().any(|p| p == &AluProperty::Clamp) {
        let input = nodes.clamp_expr(Expr::Node {
            node_index,
            channel: scalar.output.channel,
        });
        nodes.node(Node {
            output: scalar.output.clone(),
            input,
        })
    } else {
        node_index
    }
}

fn alu_output_modifier_scale(
    scalar: &AluScalarData,
    node_index: usize,
    nodes: &mut Nodes,
) -> Option<usize> {
    let modifier = scalar.output_modifier.as_ref()?.as_str();
    let lh = Expr::Node {
        node_index,
        channel: scalar.output.channel,
    };
    let output = scalar.output.clone();
    match modifier {
        "/2" => Some(nodes.binary_node(BinaryOp::Div, lh, Expr::Float(2.0.into()), output)),
        "/4" => Some(nodes.binary_node(BinaryOp::Div, lh, Expr::Float(4.0.into()), output)),
        "*2" => Some(nodes.binary_node(BinaryOp::Mul, lh, Expr::Float(2.0.into()), output)),
        "*4" => Some(nodes.binary_node(BinaryOp::Mul, lh, Expr::Float(4.0.into()), output)),
        _ => unreachable!(),
    }
}

fn alu_src_expr(
    source: AluSrc,
    nodes: &mut Nodes,
    kc0: Option<&ConstantBuffer>,
    kc1: Option<&ConstantBuffer>,
    backup_gprs: &BTreeSet<(usize, Option<char>)>,
) -> Expr {
    let negate = source.negate.is_some();

    let expr = match source.value {
        AluSrcValueOrAbs::Abs(abs_value) => {
            let channel = abs_value.swizzle;
            let arg = value_expr(nodes, channel, abs_value.value, kc0, kc1, backup_gprs);
            Expr::Func {
                name: "abs".into(),
                args: vec![nodes.expr(arg)],
                channel: None,
            }
        }
        AluSrcValueOrAbs::Value(value) => {
            let channel = source.swizzle;
            value_expr(nodes, channel, value, kc0, kc1, backup_gprs)
        }
    };

    if negate {
        if let Expr::Float(f) = expr {
            // Avoid an issue with -0.0 being equal to 0.0 when hashing ordered_float.
            Expr::Float(-f)
        } else {
            Expr::Unary(UnaryOp::Negate, nodes.expr(expr))
        }
    } else {
        expr
    }
}

fn value_expr(
    nodes: &mut Nodes,
    channel: Option<char>,
    value: AluSrcValue,
    kc0: Option<&ConstantBuffer>,
    kc1: Option<&ConstantBuffer>,
    backup_gprs: &BTreeSet<(usize, Option<char>)>,
) -> Expr {
    match value {
        AluSrcValue::Gpr(gpr) => {
            if backup_gprs.contains(&(gpr.0, channel)) {
                // Find the backed up value from before this ALU group.
                previous_assignment(&format!("{gpr}_backup"), channel, nodes)
            } else {
                previous_assignment(&gpr.to_string(), channel, nodes)
            }
        }
        AluSrcValue::ConstantCache0(c0) => constant_buffer(c0.0, channel, kc0, nodes),
        AluSrcValue::ConstantCache1(c1) => constant_buffer(c1.0, channel, kc1, nodes),
        AluSrcValue::ConstantFile(cf) => constant_file(cf, channel),
        AluSrcValue::Literal(l) => literal(l),
        AluSrcValue::PreviousScalar(s) => previous_assignment(&s.to_string(), channel, nodes),
        AluSrcValue::PreviousVector(v) => previous_assignment(&v.to_string(), channel, nodes),
    }
}

fn constant_file(cf: ConstantFile, channel: Option<char>) -> Expr {
    // TODO: how to handle constant file expressions?
    Expr::Global {
        name: format!("C{}", cf.0).into(),
        channel,
    }
}

fn literal(l: Literal) -> Expr {
    // TODO: how to handle hex literals?
    match l {
        Literal::Hex(_) => todo!(),
        Literal::Float(f) => Expr::Float(f.trim().trim_end_matches('f').parse().unwrap()),
    }
}

fn constant_buffer(
    index: usize,
    channel: Option<char>,
    constant_buffer: Option<&ConstantBuffer>,
    nodes: &mut Nodes,
) -> Expr {
    let cb = constant_buffer.as_ref().unwrap();
    Expr::Parameter {
        name: format!("CB{}", cb.index).into(),
        field: None,
        index: Some(nodes.expr(Expr::Int((index + cb.start_index) as i32))),
        channel,
    }
}

fn previous_assignment(value: &str, channel: Option<char>, nodes: &Nodes) -> Expr {
    // Find a previous assignment that modifies the desired channel for variables.
    // PV can also refer to an actual register if not all outputs were masked.
    nodes
        .nodes
        .iter()
        .rposition(|node| node.output.name == value && node.output.channel == channel)
        .map(|node_index| Expr::Node {
            node_index,
            channel,
        })
        .unwrap_or_else(|| {
            // TODO: This shouldn't happen if attributes are set properly.
            error!(
                "Unable to find previous assignment for {value}{}",
                channel.map(|c| format!(".{c}")).unwrap_or_default()
            );
            Expr::Global {
                name: value.into(),
                channel,
            }
        })
}

fn tex_inst_node(tex: TexInst, nodes: &mut Nodes) -> Option<Vec<Node>> {
    // TODO: Check that op code is SAMPLE?

    // TODO: Get the input names and channels.
    // TODO: register or mask?
    let output_name = tex.dst.gpr.to_smolstr();
    let output_channels = &tex.dst.swizzle.0;

    let texcoords = tex_src_coords(&tex, nodes)?;

    let texture_name = nodes.expr(Expr::Global {
        name: tex.resource_id.0.into(),
        channel: None,
    });

    // Convert vector swizzles to scalar operations to simplify analysis code.
    // The output and input channels aren't always the same.
    // For example, ___x should assign src.x to dst.w.
    Some(
        output_channels
            .chars()
            .zip("xyzw".chars())
            .filter_map(|(c_in, c_out)| {
                if c_in != '_' {
                    let input = Expr::Func {
                        name: "texture".into(),
                        args: vec![texture_name, texcoords],
                        channel: Some(c_in),
                    };
                    Some(Node {
                        output: Output {
                            name: output_name.clone(),
                            channel: Some(c_out),
                        },
                        input: nodes.expr(input),
                    })
                } else {
                    None
                }
            })
            .collect(),
    )
}

fn tex_src_coords(tex: &TexInst, nodes: &mut Nodes) -> Option<usize> {
    // TODO: Handle other cases from grammar.
    let gpr = tex.src.gpr.to_string();

    // TODO: Handle write masks.
    let args = if tex.dst.gpr == tex.src.gpr {
        // Backup values if input and output are the same when converting to scalar.
        // For example, R1.xyz = texture(s5, R1.xy) should use the previous value for R1.
        let node_indices: Vec<_> = tex
            .src
            .swizzle
            .0
            .chars()
            .take(2)
            .map(|c| {
                let node = Node {
                    output: Output {
                        name: format!("{gpr}_backup").into(),
                        channel: Some(c),
                    },
                    input: nodes.previous_assignment_expr(&gpr, Some(c)),
                };
                nodes.node(node)
            })
            .collect();

        let mut channels = tex.src.swizzle.0.chars();
        vec![
            nodes.expr(Expr::Node {
                node_index: node_indices[0],
                channel: channels.next(),
            }),
            nodes.expr(Expr::Node {
                node_index: node_indices[1],
                channel: channels.next(),
            }),
        ]
    } else {
        let mut channels = tex.src.swizzle.0.chars();

        vec![
            nodes.previous_assignment_expr(&gpr, channels.next()),
            nodes.previous_assignment_expr(&gpr, channels.next()),
        ]
    };

    // TODO: Also handle cube maps.
    Some(nodes.func_expr("vec2", args))
}

fn fetch_inst_node(tex: FetchInst, nodes: &mut Nodes) -> Option<Vec<Node>> {
    let output_name = tex.dst.gpr.to_smolstr();
    let output_channels = tex.dst.swizzle.0;

    let src_name = tex.src.gpr.to_smolstr();
    let src_channel = tex.src.swizzle;

    // TODO: Is this the correct way to calculate the buffer index?
    let cb_index = tex.buffer_id - 128;
    let cb_name: SmolStr = format!("CB{cb_index}").into();

    // TODO: How should the OFFSET property be used?
    let src_expr = previous_assignment(&src_name, src_channel, nodes);
    let src_index = nodes.float_to_uint_expr(src_expr);

    // Convert vector swizzles to scalar operations to simplify analysis code.
    Some(
        output_channels
            .chars()
            .zip("xyzw".chars())
            .filter_map(|(c_in, c_out)| {
                if c_in != '_' {
                    let input = Expr::Parameter {
                        name: cb_name.clone(),
                        field: None,
                        index: Some(src_index),
                        channel: Some(c_in),
                    };
                    Some(Node {
                        output: Output {
                            name: output_name.clone(),
                            channel: Some(c_out),
                        },
                        input: nodes.expr(input),
                    })
                } else {
                    None
                }
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    use insta::assert_snapshot;

    macro_rules! assert_glsl_snapshot {
        ($glsl:expr) => {
            let mut settings = insta::Settings::new();
            settings.set_prepend_module_to_snapshot(false);
            settings.set_omit_expression(true);
            settings.bind(|| {
                assert_snapshot!($glsl);
            });
        };
    }

    #[test]
    fn graph_from_asm_pc221115_frag_0() {
        // Elma's legs (visible on title screen).
        let asm = include_str!("../data/xcx/pc221115.0.frag.txt");
        let graph = Graph::from_latte_asm(asm).unwrap();
        assert_glsl_snapshot!(graph.to_glsl());
    }

    #[test]
    fn graph_from_asm_pc221115_vert_0() {
        // Elma's legs (visible on title screen).
        let asm = include_str!("../data/xcx/pc221115.0.vert.txt");
        let graph = Graph::from_latte_asm(asm).unwrap();
        assert_glsl_snapshot!(graph.to_glsl());
    }

    #[test]
    fn graph_from_asm_en020601_frag_0() {
        // Tree enemy.
        let asm = include_str!("../data/xcx/en020601.0.frag.txt");
        let graph = Graph::from_latte_asm(asm).unwrap();
        assert_glsl_snapshot!(graph.to_glsl());
    }
}
