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
    rule one_comp_swizzle() -> &'input str = s:$("." ['x' | 'y' | 'z' | 'w' | 'X' | 'Y' | 'Z' | 'W']) { s }
    rule four_comp_swizzle() -> &'input str = s:$("." ['x' | 'y' | 'z' | 'w' | 'X' | 'Y' | 'Z' | 'W' | '0' | '1' | '_']+) { s }
    // TODO: always preserve hex?
    rule literal() -> Literal
        = n:hex_number() { Literal(LiteralInner::Hex(n.to_string())) }
        / f:float() { Literal(LiteralInner::Float(f.to_string())) }
        / "(" _ hex_number() _ "," _ f:float() _ ")" { Literal(LiteralInner::Float(f.to_string())) }
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
    rule kcache0() -> KCache0
        = "KCACHE0" _ "(" _ "CB" _ n1:number() _ ":" _ n2:number() _ "-" n3:number() _ ")" {
            KCache0 { constant_buffer: n1, start_index: n2, end_index: n3 }
        }
    rule kcache1() -> KCache1
        = "KCACHE1" _ "(" _ "CB" _ n1:number() _ ":" _ n2:number() _ "-" n3:number() _ ")" {
            KCache1 { constant_buffer: n1, start_index: n2, end_index: n3 }
        }
    rule no_barrier() = "NO_BARRIER"
    rule pop_cnt() -> usize = "POPCNT(" _ n:number() _ ")" { n }
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
        = inst_count:inst_count() _ "FETCH" _ dst:fetch_dst() _ "," _ src:fetch_src() _ "," _ buffer_id:fetch_buffer_id() _ fetch_properties() {
            FetchInst {
                inst_count,
                dst,
                src,
                buffer_id,
                properties: FetchProperties(Vec::new())
            }
        }
    rule fetch_dst() -> FetchDst
        = gpr:gpr() _ s:four_comp_swizzle()? { FetchDst { gpr, swizzle: FourCompSwizzle(s.unwrap_or_default().to_string()) }}
    rule fetch_src() -> FetchSrc
        = gpr:gpr() _ s:one_comp_swizzle()? { FetchSrc { gpr, swizzle: OneCompSwizzle(s.unwrap_or_default().to_string()) }}
    rule fetch_buffer_id() -> usize = "b" _ n:number() { n }
    rule fetch_properties() = (fetch_type() / fetch_mega() / fetch_offset()) ** _
    rule fetch_type() = "FETCH_TYPE(NO_INDEX_OFFSET)"
    rule fetch_mega() = "MEGA(" _ number() _ ")"
    rule fetch_offset() = "OFFSET(" _ number() _ ")"

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
        = u:alu_unit() _ ":" _ op:alu_opcode0() _ m:alu_output_modifier()? _ dst:alu_dst() _ alu_properties() {
            AluScalar0 {
                alu_unit: AluUnit(u.to_string()),
                opcode: AluOpCode0(op.to_string()),
                modifier: m.map(|m| AluOutputModifier(m.to_string())),
                dst,
                properties: AluProperties(Vec::new())
            }
        }
    rule alu_scalar1() -> AluScalar1
        = u:alu_unit() _ ":" _ op:alu_opcode1() _ m:alu_output_modifier()? _ dst:alu_dst() _ "," _ src1:alu_src() _ alu_properties() {
            AluScalar1 {
                alu_unit: AluUnit(u.to_string()),
                opcode: AluOpCode1(op.to_string()),
                modifier: m.map(|m| AluOutputModifier(m.to_string())),
                dst,
                src1,
                properties: AluProperties(Vec::new())
            }
        }
    rule alu_scalar2() -> AluScalar2
        = u:alu_unit() _ ":" _ op:alu_opcode2() _ m:alu_output_modifier()? _ dst:alu_dst() _ "," _ src1:alu_src() _ "," _ src2:alu_src() _ alu_properties() {
            AluScalar2 {
                alu_unit: AluUnit(u.to_string()),
                opcode: AluOpCode2(op.to_string()),
                modifier: m.map(|m| AluOutputModifier(m.to_string())),
                dst,
                src1,
                src2,
                properties: AluProperties(Vec::new())
            }
        }
    rule alu_scalar3() -> AluScalar3
        = u:alu_unit() _ ":" _ op:alu_opcode3() _ dst:alu_dst() _ "," _ src1:alu_src() _ "," _ src2:alu_src() _ "," _ src3:alu_src() _ alu_properties() {
            AluScalar3 {
                alu_unit: AluUnit(u.to_string()),
                opcode: AluOpCode3(op.to_string()),
                dst,
                src1,
                src2,
                src3,
                properties: AluProperties(Vec::new())
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
        = gpr:gpr() _ alu_rel:alu_rel()? _ s:one_comp_swizzle()? {
            AluDst::Value {
                gpr,
                alu_rel,
                swizzle: s.map(|s| OneCompSwizzle(s.to_string()))
            }
        }
        / m:write_mask() { AluDst::WriteMask(WriteMask(m.to_string())) }
    rule alu_src() -> AluSrc
        = negate:negate()? _ value:alu_src_value_or_abs() _ alu_rel:alu_rel()? _ s:one_comp_swizzle()? {
            AluSrc {
                negate,
                value,
                alu_rel,
                swizzle: s.map(|s| OneCompSwizzle(s.to_string()))
            }
        }
    rule alu_src_value_or_abs() -> AluSrcValueOrAbs
        = src:alu_abs_src_value() { AluSrcValueOrAbs::Abs(src) }
        / src:alu_src_value() { AluSrcValueOrAbs::Value(src) }
    rule alu_abs_src_value() -> AluAbsSrcValue
        = "/" _ value:alu_src_value() _ s:one_comp_swizzle()? _ "/" {
            AluAbsSrcValue { value, swizzle: s.map(|s| OneCompSwizzle(s.to_string())) }
        }
    rule alu_src_value() -> AluSrcValue
        = v:gpr() { AluSrcValue::Gpr(v) }
        / v:constant_cache0() { AluSrcValue::ConstantCache0(ConstantCache0(v)) }
        / v:constant_cache1() { AluSrcValue::ConstantCache1(ConstantCache1(v)) }
        / v:constant_file() { AluSrcValue::ConstantFile(ConstantFile(v)) }
        / v:literal() { AluSrcValue::Literal(v) }
        / v:previous_scalar() { AluSrcValue::PreviousScalar(PreviousScalar(v)) }
        / v:previous_vector() { AluSrcValue::PreviousVector(PreviousVector(v)) }
    rule alu_properties() = (bank_swizzle() / update_exec_mask() / update_pred() / pred_sel() / clamp()) ** _
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

#[derive(Debug)]
struct CfExpInst {
    inst_count: InstCount,
    opcode: ExpOpcode,
    target: ExpTarget,
    src: ExpSrc,
    properties: CfInstProperties,
}

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

#[derive(Debug)]
struct TexClause {
    inst_count: InstCount,
    inst_type: TexClauseInstType,
    properties: TexClauseProperties,
    instructions: Vec<TexInstOrFetchInst>,
}

#[derive(Debug)]
struct TexClauseInstType;

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

#[derive(Debug)]
struct TexOpcode(String);

#[derive(Debug)]
struct TexResourceId(String);

#[derive(Debug)]
struct TexSamplerId(String);

#[derive(Debug)]
struct TexDst {
    gpr: Gpr,
    tex_rel: Option<TexRel>,
    swizzle: FourCompSwizzle,
}

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

#[derive(Debug)]
struct FetchInst {
    inst_count: InstCount,
    dst: FetchDst,
    src: FetchSrc,
    buffer_id: usize,
    properties: FetchProperties,
}

#[derive(Debug)]
struct FetchDst {
    gpr: Gpr,
    swizzle: FourCompSwizzle,
}

#[derive(Debug)]
struct FetchSrc {
    gpr: Gpr,
    swizzle: OneCompSwizzle,
}

#[derive(Debug)]
struct FetchBufferId {
    id: usize,
}

#[derive(Debug)]
struct FetchType {}

#[derive(Debug)]
struct FetchMega {
    id: usize,
}

#[derive(Debug)]
struct FetchOffset {
    id: usize,
}

#[derive(Debug)]
struct FetchProperties(Vec<FetchProperty>);

#[derive(Debug)]
enum FetchProperty {
    Type(FetchType),
    Mega(FetchMega),
    Offset(FetchOffset),
}

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
    KCache0(KCache0),
    KCache1(KCache1),
    Unk,
}

#[derive(Debug)]
struct KCache0 {
    constant_buffer: usize,
    start_index: usize,
    end_index: usize,
}

#[derive(Debug)]
struct KCache1 {
    constant_buffer: usize,
    start_index: usize,
    end_index: usize,
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

#[derive(Debug)]
struct AluScalar0 {
    alu_unit: AluUnit,
    opcode: AluOpCode0,
    modifier: Option<AluOutputModifier>,
    dst: AluDst,
    properties: AluProperties,
}

#[derive(Debug)]
struct AluScalar1 {
    alu_unit: AluUnit,
    opcode: AluOpCode1,
    modifier: Option<AluOutputModifier>,
    dst: AluDst,
    src1: AluSrc,
    properties: AluProperties,
}

#[derive(Debug)]
struct AluScalar2 {
    alu_unit: AluUnit,
    opcode: AluOpCode2,
    modifier: Option<AluOutputModifier>,
    dst: AluDst,
    src1: AluSrc,
    src2: AluSrc,
    properties: AluProperties,
}

#[derive(Debug)]
struct AluScalar3 {
    alu_unit: AluUnit,
    opcode: AluOpCode3,
    dst: AluDst,
    src1: AluSrc,
    src2: AluSrc,
    src3: AluSrc,
    properties: AluProperties,
}

#[derive(Debug)]
struct WriteMask(String);

#[derive(Debug)]
enum AluDst {
    Value {
        gpr: Gpr,
        alu_rel: Option<AluRel>,
        swizzle: Option<OneCompSwizzle>,
    },
    WriteMask(WriteMask),
}

#[derive(Debug)]
struct AluSrc {
    negate: Option<Negate>,
    value: AluSrcValueOrAbs,
    alu_rel: Option<AluRel>,
    swizzle: Option<OneCompSwizzle>,
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

#[derive(Debug)]
struct Literal(LiteralInner);

#[derive(Debug)]
enum LiteralInner {
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
    swizzle: Option<OneCompSwizzle>,
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

#[derive(Debug)]
struct AluProperties(Vec<AluProperty>);

#[derive(Debug)]
enum AluProperty {
    Unk,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
struct InstCount(usize);

#[derive(Debug)]
struct FourCompSwizzle(String);

#[derive(Debug)]
struct OneCompSwizzle(String);

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
struct Gpr(usize);

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
    node_info: Vec<NodeInfo>,
    exprs: IndexSet<Expr>,
}

struct NodeInfo {
    index: usize,
    alu_unit: Option<char>,
    inst_count: InstCount,
}

impl Nodes {
    fn node(&mut self, node: Node, alu_unit: Option<char>, inst_count: InstCount) -> usize {
        let index = self.nodes.len();
        self.nodes.push(node);
        self.node_info.push(NodeInfo {
            index,
            alu_unit,
            inst_count,
        });
        index
    }

    fn set_float_node(
        &mut self,
        op: BinaryOp,
        scalar: &AluScalarData,
        output: Output,
        inst_count: InstCount,
    ) -> usize {
        let input = Expr::Ternary(
            self.binary_expr(op, scalar.sources[0].clone(), scalar.sources[1].clone()),
            self.expr(Expr::Float(1.0.into())),
            self.expr(Expr::Float(0.0.into())),
        );
        let node = Node {
            output,
            input: self.expr(input),
        };
        self.node(node, Some(scalar.alu_unit), inst_count)
    }

    fn set_float_dx10_node(
        &mut self,
        op: BinaryOp,
        scalar: &AluScalarData,
        output: Output,
        inst_count: InstCount,
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
        self.node(node, Some(scalar.alu_unit), inst_count)
    }

    fn set_int_node(
        &mut self,
        op: BinaryOp,
        scalar: &AluScalarData,
        output: Output,
        inst_count: InstCount,
    ) -> usize {
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
        self.node(node, Some(scalar.alu_unit), inst_count)
    }

    fn cnd_float_node(
        &mut self,
        op: BinaryOp,
        scalar: &AluScalarData,
        output: Output,
        inst_count: InstCount,
    ) -> usize {
        let input = Expr::Ternary(
            self.binary_expr(op, scalar.sources[0].clone(), Expr::Float(0.0.into())),
            self.expr(scalar.sources[1].clone()),
            self.expr(scalar.sources[2].clone()),
        );
        let node = Node {
            output,
            input: self.expr(input),
        };
        self.node(node, Some(scalar.alu_unit), inst_count)
    }

    fn cnd_int_node(
        &mut self,
        op: BinaryOp,
        scalar: &AluScalarData,
        output: Output,
        inst_count: InstCount,
    ) -> usize {
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
        self.node(node, Some(scalar.alu_unit), inst_count)
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

    fn add_float_to_uint(&mut self, expr: Expr) -> usize {
        // Convert float literals directly to integers.
        let result = match expr {
            Expr::Float(f) => Expr::Uint(f.to_bits()),
            e => Expr::Unary(UnaryOp::FloatBitsToUint, self.expr(e)),
        };
        self.expr(result)
    }

    fn add_func(
        &mut self,
        func: &str,
        arg_count: usize,
        scalar: &AluScalarData,
        output: Output,
        inst_count: InstCount,
    ) -> usize {
        let input = Expr::Func {
            name: func.into(),
            args: scalar
                .sources
                .iter()
                .take(arg_count)
                .map(|a| self.expr(a.clone()))
                .collect(),
            channel: None,
        };
        let node = Node {
            output,
            input: self.expr(input),
        };
        self.node(node, Some(scalar.alu_unit), inst_count)
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
    let inst_count = exp.inst_count;

    let (target_name, target_index) = match exp.target {
        ExpTarget::Pix(t) => ("PIX", t.0),
        ExpTarget::Pos(t) => ("POS", t.0),
        ExpTarget::Param(t) => ("PARAM", t.0),
    };

    let source_name = "R";
    let source_index = exp.src.gpr.0;
    let channels = exp.src.swizzle.channels();

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
                input: nodes.expr(previous_assignment(
                    &format!("{source_name}{}", source_index + i),
                    Some(c),
                    nodes,
                    inst_count,
                )),
            };
            nodes.node(node, None, inst_count);
        }
    }
}

fn add_tex_clause(clause: TexClause, nodes: &mut Nodes) {
    for tex_instruction in clause.instructions {
        match tex_instruction {
            TexInstOrFetchInst::Tex(tex_inst) => {
                let tex_nodes = tex_inst_node(tex_inst, nodes).unwrap();
                for node in tex_nodes {
                    nodes.node(node, None, clause.inst_count);
                }
            }
            TexInstOrFetchInst::Fetch(fetch_inst) => {
                let fetch_nodes = fetch_inst_node(fetch_inst, nodes).unwrap();
                for node in fetch_nodes {
                    nodes.node(node, None, clause.inst_count);
                }
            }
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

struct ConstantBuffer {
    index: usize,
    start_index: usize,
    end_index: usize,
}

fn add_alu_clause(clause: AluClause, nodes: &mut Nodes) {
    for group in clause.groups {
        let inst_count = group.inst_count;

        // Ranges from constant buffers are mapped to constant cache KC0 and KC1.
        // These mappings persist for the duration of the ALU clause.
        let mut kc0_buffer = None;
        let mut kc1_buffer = None;
        for prop in &clause.properties.0 {
            match prop {
                AluClauseProperty::KCache0(kc) => {
                    kc0_buffer = Some(ConstantBuffer {
                        index: kc.constant_buffer,
                        start_index: kc.start_index,
                        end_index: kc.end_index,
                    })
                }
                AluClauseProperty::KCache1(kc) => {
                    kc1_buffer = Some(ConstantBuffer {
                        index: kc.constant_buffer,
                        start_index: kc.start_index,
                        end_index: kc.end_index,
                    })
                }
                _ => (),
            }
        }

        // TODO: backup values if assigned value is used for another channel
        let scalars: Vec<_> = group
            .scalars
            .into_iter()
            .map(|scalar| match scalar {
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
                        sources: vec![alu_src_expr(
                            s.src1,
                            nodes,
                            &kc0_buffer,
                            &kc1_buffer,
                            inst_count,
                        )],
                    }
                }
                AluScalar::Scalar2(s) => {
                    let alu_unit = s.alu_unit.0.chars().next().unwrap();
                    AluScalarData {
                        alu_unit,
                        op_code: s.opcode.0,
                        output_modifier: s.modifier.map(|m| m.0),
                        output: alu_dst_output(s.dst, inst_count, alu_unit),
                        sources: vec![
                            alu_src_expr(s.src1, nodes, &kc0_buffer, &kc1_buffer, inst_count),
                            alu_src_expr(s.src2, nodes, &kc0_buffer, &kc1_buffer, inst_count),
                        ],
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
                            alu_src_expr(s.src1, nodes, &kc0_buffer, &kc1_buffer, inst_count),
                            alu_src_expr(s.src2, nodes, &kc0_buffer, &kc1_buffer, inst_count),
                            alu_src_expr(s.src3, nodes, &kc0_buffer, &kc1_buffer, inst_count),
                        ],
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
                        input: nodes.expr(Expr::Node {
                            node_index,
                            channel: None,
                        }),
                    };
                    nodes.node(node, Some(scalar.alu_unit), inst_count);
                }
            } else {
                add_scalar(scalar, nodes, inst_count);
            }
        }
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
        let input = Expr::Func {
            name: "dot".into(),
            args: vec![
                nodes.expr(Expr::Func {
                    name: "vec4".into(),
                    args: dot4_a,
                    channel: None,
                }),
                nodes.expr(Expr::Func {
                    name: "vec4".into(),
                    args: dot4_b,
                    channel: None,
                }),
            ],
            channel: None,
        };
        let node = Node {
            output: Output {
                name: format!("temp{}", inst_count.0).into(),
                channel: None,
            },
            input: nodes.expr(input),
        };
        let node_index = nodes.node(node, None, inst_count);
        Some(node_index)
    } else {
        None
    }
}

fn add_scalar(scalar: AluScalarData, nodes: &mut Nodes, inst_count: InstCount) {
    let output = scalar.output.clone();
    let node_index = match scalar.op_code.as_str() {
        // scalar1
        "MOV" => {
            let node = Node {
                output,
                input: nodes.expr(scalar.sources[0].clone()),
            };
            Some(nodes.node(node, Some(scalar.alu_unit), inst_count))
        }
        "FLOOR" => Some(nodes.add_func("floor", 1, &scalar, output, inst_count)),
        "SQRT_IEEE" => Some(nodes.add_func("sqrt", 1, &scalar, output, inst_count)),
        "RECIP_IEEE" => {
            let input = nodes.binary_expr(
                BinaryOp::Div,
                Expr::Float(1.0.into()),
                scalar.sources[0].clone(),
            );
            let node = Node { output, input };
            Some(nodes.node(node, Some(scalar.alu_unit), inst_count))
        }
        "RECIP_FF" => {
            // TODO: Set result of +inf to +0 and -inf to -0.
            let input = nodes.binary_expr(
                BinaryOp::Div,
                Expr::Float(1.0.into()),
                scalar.sources[0].clone(),
            );
            let node = Node { output, input };
            Some(nodes.node(node, Some(scalar.alu_unit), inst_count))
        }
        "RECIPSQRT_IEEE" => Some(nodes.add_func("inversesqrt", 1, &scalar, output, inst_count)),
        "RECIPSQRT_FF" => {
            // TODO: Set result of +inf to +0 and -inf to -0.
            Some(nodes.add_func("inversesqrt", 1, &scalar, output, inst_count))
        }
        "EXP_IEEE" => Some(nodes.add_func("exp2", 1, &scalar, output, inst_count)),
        "LOG_CLAMPED" => Some(nodes.add_func("log2", 1, &scalar, output, inst_count)),
        // scalar2
        "ADD" => {
            let input = nodes.binary_expr(
                BinaryOp::Add,
                scalar.sources[0].clone(),
                scalar.sources[1].clone(),
            );
            let node = Node { output, input };
            Some(nodes.node(node, Some(scalar.alu_unit), inst_count))
        }
        "ADD_INT" => {
            let result = Expr::Binary(
                BinaryOp::Add,
                nodes.float_to_int_expr(scalar.sources[0].clone()),
                nodes.float_to_int_expr(scalar.sources[1].clone()),
            );
            let input = nodes.unary_expr(UnaryOp::IntBitsToFloat, result);
            let node = Node { output, input };
            Some(nodes.node(node, Some(scalar.alu_unit), inst_count))
        }
        "MIN" | "MIN_DX10" => Some(nodes.add_func("min", 2, &scalar, output, inst_count)),
        "MAX" | "MAX_DX10" => Some(nodes.add_func("max", 2, &scalar, output, inst_count)),
        "MUL" | "MUL_IEEE" => {
            // Scalar multiplication with floats.
            let input = nodes.binary_expr(
                BinaryOp::Mul,
                scalar.sources[0].clone(),
                scalar.sources[1].clone(),
            );
            let node = Node { output, input };
            Some(nodes.node(node, Some(scalar.alu_unit), inst_count))
        }
        "DOT4" | "DOT4_IEEE" => {
            // Handled in a previous check.
            unreachable!()
        }
        "MULLO_UINT" => {
            // Scalar multiplication with unsigned integers stored in the lower bits.
            let result = Expr::Binary(
                BinaryOp::Mul,
                nodes.add_float_to_uint(scalar.sources[0].clone()),
                nodes.add_float_to_uint(scalar.sources[1].clone()),
            );
            let input = nodes.unary_expr(UnaryOp::UintBitsToFloat, result);
            let node = Node { output, input };
            Some(nodes.node(node, Some(scalar.alu_unit), inst_count))
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
            Some(nodes.node(node, Some(scalar.alu_unit), inst_count))
        }
        // scalar3
        "MULADD" | "MULADD_IEEE" => Some(nodes.add_func("fma", 3, &scalar, output, inst_count)),
        "MULADD_M2" => {
            let node_index = nodes.add_func("fma", 3, &scalar, output.clone(), inst_count);
            let input = nodes.binary_expr(
                BinaryOp::Mul,
                Expr::Node {
                    node_index,
                    channel: scalar.output.channel,
                },
                Expr::Float(2.0.into()),
            );
            let node = Node { output, input };
            Some(nodes.node(node, Some(scalar.alu_unit), inst_count))
        }
        "MULADD_M4" => {
            let node_index = nodes.add_func("fma", 3, &scalar, output.clone(), inst_count);
            let input = nodes.binary_expr(
                BinaryOp::Mul,
                Expr::Node {
                    node_index,
                    channel: scalar.output.channel,
                },
                Expr::Float(4.0.into()),
            );
            let node = Node { output, input };
            Some(nodes.node(node, Some(scalar.alu_unit), inst_count))
        }
        "MULADD_D2" => {
            let node_index = nodes.add_func("fma", 3, &scalar, output.clone(), inst_count);
            let input = nodes.binary_expr(
                BinaryOp::Div,
                Expr::Node {
                    node_index,
                    channel: scalar.output.channel,
                },
                Expr::Float(2.0.into()),
            );
            let node = Node { output, input };
            Some(nodes.node(node, Some(scalar.alu_unit), inst_count))
        }
        "MULADD_D4" => {
            let node_index = nodes.add_func("fma", 3, &scalar, output.clone(), inst_count);
            let input = nodes.binary_expr(
                BinaryOp::Div,
                Expr::Node {
                    node_index,
                    channel: scalar.output.channel,
                },
                Expr::Float(4.0.into()),
            );
            let node = Node { output, input };
            Some(nodes.node(node, Some(scalar.alu_unit), inst_count))
        }
        "NOP" => None,
        "FLT_TO_INT" => {
            let input = nodes.unary_expr(UnaryOp::FloatToInt, scalar.sources[0].clone());
            let node = Node { output, input };
            Some(nodes.node(node, Some(scalar.alu_unit), inst_count))
        }
        "FLT_TO_UINT" => {
            let input = nodes.unary_expr(UnaryOp::FloatToUint, scalar.sources[0].clone());
            let node = Node { output, input };
            Some(nodes.node(node, Some(scalar.alu_unit), inst_count))
        }
        "INT_TO_FLT" => {
            let input = nodes.unary_expr(UnaryOp::IntToFloat, scalar.sources[0].clone());
            let node = Node { output, input };
            Some(nodes.node(node, Some(scalar.alu_unit), inst_count))
        }
        "UINT_TO_FLT" => {
            let input = nodes.unary_expr(UnaryOp::UintToFloat, scalar.sources[0].clone());
            let node = Node { output, input };
            Some(nodes.node(node, Some(scalar.alu_unit), inst_count))
        }
        "SIN" => Some(nodes.add_func("sin", 1, &scalar, output, inst_count)),
        "COS" => Some(nodes.add_func("cos", 1, &scalar, output, inst_count)),
        "FRACT" => Some(nodes.add_func("fract", 1, &scalar, output, inst_count)),
        "CUBE" => None, // TODO: Cube maps
        // TODO: push/pop and predicates
        "PRED_SETGE" | "PRED_SETGT" => None,
        // Conditionals
        "KILLE_INT" => {
            // TODO: if src0 == src1 kill and set dst to 1.0 else set dst to 0.0
            None
        }
        "SETE" => {
            // Floating-point set if equal.
            Some(nodes.set_float_node(BinaryOp::Equal, &scalar, output, inst_count))
        }
        "SETE_INT" => {
            // Integer set if equal.
            Some(nodes.set_int_node(BinaryOp::Equal, &scalar, output, inst_count))
        }
        "SETE_DX10" => {
            // Floating-point set if equal with an integer result.
            Some(nodes.set_float_dx10_node(BinaryOp::Equal, &scalar, output, inst_count))
        }
        "SETNE" => {
            // Floating-point set if not equal.
            Some(nodes.set_float_node(BinaryOp::NotEqual, &scalar, output, inst_count))
        }
        "SETNE_INT" => {
            // Integer set if not equal.
            Some(nodes.set_int_node(BinaryOp::NotEqual, &scalar, output, inst_count))
        }
        "SETNE_DX10" => {
            // Floating-point set if not equal with an integer result.
            Some(nodes.set_float_dx10_node(BinaryOp::Equal, &scalar, output, inst_count))
        }
        "SETGT" => {
            // Floating-point set if greater than.
            Some(nodes.set_float_node(BinaryOp::Greater, &scalar, output, inst_count))
        }
        "SETGT_INT" => {
            // Integer set if greater than.
            Some(nodes.set_int_node(BinaryOp::Greater, &scalar, output, inst_count))
        }
        "SETGT_DX10" => {
            // Floating-point set if greater than with an integer result.
            Some(nodes.set_float_dx10_node(BinaryOp::Greater, &scalar, output, inst_count))
        }
        "SETGE" => {
            // Floating-point set if greater than or equal.
            Some(nodes.set_float_node(BinaryOp::GreaterEqual, &scalar, output, inst_count))
        }
        "SETGE_INT" => {
            // Integer set if greater than or equal.
            Some(nodes.set_int_node(BinaryOp::GreaterEqual, &scalar, output, inst_count))
        }
        "SETGE_DX10" => {
            // Floating-point set if geq with an integer result.
            Some(nodes.set_float_dx10_node(BinaryOp::GreaterEqual, &scalar, output, inst_count))
        }
        "CNDE" => {
            // Floating-point conditional move if equal 0.0.
            Some(nodes.cnd_float_node(BinaryOp::Equal, &scalar, output, inst_count))
        }
        "CNDE_INT" => {
            // Integer conditional move if equal 0.
            Some(nodes.cnd_int_node(BinaryOp::Equal, &scalar, output, inst_count))
        }
        "CNDGT" => {
            // Floating-point conditional move if greater than 0.0.
            Some(nodes.cnd_float_node(BinaryOp::Greater, &scalar, output, inst_count))
        }
        "CNDGT_INT" => {
            // Integer conditional move if greater than 0.
            Some(nodes.cnd_int_node(BinaryOp::Greater, &scalar, output, inst_count))
        }
        "CNDGE" => {
            // Floating-point conditional move if greater than or equal 0.0.
            Some(nodes.cnd_float_node(BinaryOp::GreaterEqual, &scalar, output, inst_count))
        }
        "CNDGE_INT" => {
            // Integer conditional move if greater than or equal 0.
            Some(nodes.cnd_int_node(BinaryOp::GreaterEqual, &scalar, output, inst_count))
        }
        opcode => {
            // TODO: Handle additional opcodes?
            error!("Unsupported opcode {opcode}");
            None
        }
    };

    if let Some(modifier) = scalar.output_modifier {
        if let Some(node_index) = node_index {
            let node = alu_output_modifier(&modifier, scalar.output, node_index, nodes);
            nodes.node(node, Some(scalar.alu_unit), inst_count);
        }
    }
}

fn alu_dst_output(dst: AluDst, inst_count: InstCount, alu_unit: char) -> Output {
    match dst {
        AluDst::Value {
            gpr,
            alu_rel: _,
            swizzle: one_comp_swizzle,
        } => {
            let channel = one_comp_swizzle.and_then(|s| s.channels().chars().next());
            Output {
                name: gpr.to_smolstr(),
                channel,
            }
        }
        AluDst::WriteMask(_write_mask) => {
            // ____ mask for xyzw writes to a previous vector "PV".
            // ____ mask for t writes to a previous scalar "PS".
            match alu_unit {
                'x' => Output {
                    name: format!("PV{}", inst_count.0).into(),
                    channel: Some('x'),
                },
                'y' => Output {
                    name: format!("PV{}", inst_count.0).into(),
                    channel: Some('y'),
                },
                'z' => Output {
                    name: format!("PV{}", inst_count.0).into(),
                    channel: Some('z'),
                },
                'w' => Output {
                    name: format!("PV{}", inst_count.0).into(),
                    channel: Some('w'),
                },
                't' => Output {
                    name: format!("PS{}", inst_count.0).into(),
                    channel: None,
                },
                _ => unreachable!(),
            }
        }
    }
}

fn alu_output_modifier(
    modifier: &str,
    output: Output,
    node_index: usize,
    nodes: &mut Nodes,
) -> Node {
    let (op, f) = match modifier {
        "/2" => (BinaryOp::Div, 2.0),
        "/4" => (BinaryOp::Div, 4.0),
        "*2" => (BinaryOp::Mul, 2.0),
        "*4" => (BinaryOp::Mul, 4.0),
        _ => panic!("unexpected modifier: {modifier}"),
    };

    let input = nodes.binary_expr(
        op,
        Expr::Node {
            node_index,
            channel: output.channel,
        },
        Expr::Float(f.into()),
    );
    Node { output, input }
}

fn alu_src_expr(
    source: AluSrc,
    nodes: &mut Nodes,
    kc0: &Option<ConstantBuffer>,
    kc1: &Option<ConstantBuffer>,
    inst_count: InstCount,
) -> Expr {
    let negate = source.negate.is_some();

    let channel = source.swizzle.and_then(|s| s.channels().chars().next());

    let expr = match source.value {
        AluSrcValueOrAbs::Abs(abs_value) => {
            let arg = value_expr(nodes, channel, abs_value.value, kc0, kc1, inst_count);
            Expr::Func {
                name: "abs".into(),
                args: vec![nodes.expr(arg)],
                channel: abs_value.swizzle.and_then(|s| s.channels().chars().next()),
            }
        }
        AluSrcValueOrAbs::Value(value) => value_expr(nodes, channel, value, kc0, kc1, inst_count),
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
    kc0: &Option<ConstantBuffer>,
    kc1: &Option<ConstantBuffer>,
    inst_count: InstCount,
) -> Expr {
    match value {
        AluSrcValue::Gpr(gpr) => previous_assignment(&gpr.to_string(), channel, nodes, inst_count),
        AluSrcValue::ConstantCache0(c0) => constant_buffer(c0.0, channel, kc0, nodes),
        AluSrcValue::ConstantCache1(c1) => constant_buffer(c1.0, channel, kc1, nodes),
        AluSrcValue::ConstantFile(cf) => constant_file(cf, channel),
        AluSrcValue::Literal(l) => literal(l),
        AluSrcValue::PreviousScalar(s) => {
            previous_assignment(&s.to_string(), channel, nodes, inst_count)
        }
        AluSrcValue::PreviousVector(v) => {
            previous_assignment(&v.to_string(), channel, nodes, inst_count)
        }
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
    match l.0 {
        LiteralInner::Hex(_) => todo!(),
        LiteralInner::Float(f) => Expr::Float(f.trim().trim_end_matches('f').parse().unwrap()),
    }
}

fn constant_buffer(
    index: usize,
    channel: Option<char>,
    constant_buffer: &Option<ConstantBuffer>,
    nodes: &mut Nodes,
) -> Expr {
    Expr::Parameter {
        name: format!("CB{}", constant_buffer.as_ref().unwrap().index).into(),
        field: None,
        index: Some(nodes.expr(Expr::Int(
            (index + constant_buffer.as_ref().unwrap().start_index) as i32,
        ))),
        channel,
    }
}

fn previous_assignment(
    value: &str,
    channel: Option<char>,
    nodes: &Nodes,
    inst_count: InstCount,
) -> Expr {
    // Find a previous assignment that modifies the desired channel for variables.
    // PV can also refer to an actual register if not all outputs were masked.
    if value.starts_with("PV") {
        let inst_count: usize = value.split_once("PV").unwrap().1.parse().unwrap();
        find_node(nodes, InstCount(inst_count), channel, value)
    } else if value.starts_with("PS") {
        let inst_count: usize = value.split_once("PS").unwrap().1.parse().unwrap();
        find_node(nodes, InstCount(inst_count), Some('t'), value)
    } else {
        // Search starting from before the current clause.
        nodes
            .nodes
            .iter()
            .zip(&nodes.node_info)
            .rposition(|(node, info)| {
                node.output.name == value
                    && node.output.channel == channel
                    && info.inst_count != inst_count
            })
            .map(|node_index| Expr::Node {
                node_index,
                channel,
            })
            .unwrap_or(Expr::Global {
                name: value.into(),
                channel,
            })
    }
}

fn find_node(nodes: &Nodes, inst_count: InstCount, channel: Option<char>, value: &str) -> Expr {
    nodes
        .node_info
        .iter()
        .find_map(|n| {
            if n.inst_count == inst_count && n.alu_unit == channel {
                Some(Expr::Node {
                    node_index: n.index,
                    channel: nodes.nodes[n.index].output.channel,
                })
            } else {
                None
            }
        })
        .unwrap_or(Expr::Global {
            name: value.into(),
            channel,
        })
}

fn tex_inst_node(tex: TexInst, nodes: &mut Nodes) -> Option<Vec<Node>> {
    let inst_count = tex.inst_count;

    // TODO: Check that op code is SAMPLE?

    // TODO: Get the input names and channels.
    // TODO: register or mask?
    let output_name = tex.dst.gpr.to_smolstr();
    let output_channels = tex.dst.swizzle.channels();

    let texcoords = tex_src_coords(&tex, nodes, inst_count)?;
    let texcoords = nodes.expr(texcoords);

    // TODO: make these rules not atomic and format similar to gpr?
    let texture = tex.resource_id.0;
    let _sampler = tex.sampler_id.0;

    let texture_name = nodes.expr(Expr::Global {
        name: texture.into(),
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

fn tex_src_coords(tex: &TexInst, nodes: &mut Nodes, inst_count: InstCount) -> Option<Expr> {
    // TODO: Handle other cases from grammar.
    let gpr = tex.src.gpr.to_string();

    // TODO: Handle write masks.
    let args = if tex.dst.gpr == tex.src.gpr {
        // Backup values if input and output are the same when converting to scalar.
        // For example, R1.xyz = texture(s5, R1.xy) should use the previous value for R1.
        let node_indices: Vec<_> = tex
            .src
            .swizzle
            .channels()
            .chars()
            .take(2)
            .map(|c| {
                let input = nodes.expr(previous_assignment(&gpr, Some(c), nodes, inst_count));
                let node = Node {
                    output: Output {
                        name: format!("{gpr}_backup").into(),
                        channel: Some(c),
                    },
                    input,
                };
                nodes.node(node, None, tex.inst_count)
            })
            .collect();

        let mut channels = tex.src.swizzle.channels().chars();
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
        let mut channels = tex.src.swizzle.channels().chars();

        vec![
            nodes.expr(previous_assignment(
                &gpr,
                channels.next(),
                nodes,
                inst_count,
            )),
            nodes.expr(previous_assignment(
                &gpr,
                channels.next(),
                nodes,
                inst_count,
            )),
        ]
    };

    // TODO: Also handle cube maps.
    Some(Expr::Func {
        name: "vec2".into(),
        args,
        channel: None,
    })
}

fn fetch_inst_node(tex: FetchInst, nodes: &mut Nodes) -> Option<Vec<Node>> {
    let inst_count = tex.inst_count;

    let output_name = tex.dst.gpr.to_smolstr();
    let output_channels = tex.dst.swizzle.channels();

    let src_name = tex.src.gpr.to_smolstr();
    let src_channels = tex.src.swizzle.channels();

    // TODO: Is this the correct way to calculate the buffer index?
    let cb_index = tex.buffer_id - 128;
    let cb_name: SmolStr = format!("CB{cb_index}").into();

    // TODO: How should the OFFSET property be used?
    let src_expr = previous_assignment(&src_name, src_channels.chars().next(), nodes, inst_count);
    let src_index = nodes.add_float_to_uint(src_expr);

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

    use pretty_assertions::assert_eq;

    #[test]
    fn graph_from_asm_pc221115_frag_0() {
        // Elma's legs (visible on title screen).
        let asm = include_str!("../data/xcx/pc221115.0.frag.txt");
        let expected = include_str!("../data/xcx/pc221115.0.frag");

        // TODO: Figure out the expected nodes to test previous node references.
        // TODO: Test expected nodes on a handwritten example?
        let graph = Graph::from_latte_asm(asm).unwrap();
        assert_eq!(expected, graph.to_glsl());
    }

    #[test]
    fn graph_from_asm_pc221115_vert_0() {
        // Elma's legs (visible on title screen).
        let asm = include_str!("../data/xcx/pc221115.0.vert.txt");
        let expected = include_str!("../data/xcx/pc221115.0.vert");

        // TODO: Figure out the expected nodes to test previous node references.
        // TODO: Test expected nodes on a handwritten example?
        let graph = Graph::from_latte_asm(asm).unwrap();
        assert_eq!(expected, graph.to_glsl());
    }

    #[test]
    fn graph_from_asm_en020601_frag_0() {
        // Tree enemy.
        let asm = include_str!("../data/xcx/en020601.0.frag.txt");
        let expected = include_str!("../data/xcx/en020601.0.frag");

        let graph = Graph::from_latte_asm(asm).unwrap();
        assert_eq!(expected, graph.to_glsl());
    }
}
