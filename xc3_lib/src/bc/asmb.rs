use crate::{parse_ptr64, parse_string_opt_ptr64, parse_string_ptr64};
use binrw::BinRead;
use xc3_write::{Xc3Write, Xc3WriteOffsets};

use super::{BcList, BcOffset};

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
#[br(magic(b"ASMB"))]
#[xc3(magic(b"ASMB"))]
pub struct Asmb {
    #[br(parse_with = parse_ptr64)]
    #[xc3(offset(u64))]
    pub inner: AsmbInner,
}

// TODO: How to select the version?
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub enum AsmbInner {
    V1(AsmbInnerV1),
    V2(AsmbInnerV2),
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct AsmbInnerV1 {
    pub unk1: u32,
    pub unk2: i32,
    // TODO: types?
    pub unk3: BcList<u8>,
    pub unk4: BcList<BcOffset<StateV1>>,
    pub unk5: BcList<u8>,
    pub unk6: BcList<AnimationV1>,

    // TODO: This doesn't always match the chr name?
    #[br(parse_with = parse_string_opt_ptr64)]
    #[xc3(offset(u64))]
    pub skeleton_file_name: Option<String>,

    pub unk8: BcList<KeyValueV1>,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct StateV1 {
    pub unk1: u32,
    pub unk2: i32,

    #[br(parse_with = parse_string_ptr64)]
    #[xc3(offset(u64))]
    pub name: String,

    pub unk3: [i32; 4],

    // TODO: types?
    pub children: BcList<BcOffset<StateTransitionV1>>,
    pub unk9: u64,
    pub unk10: BcList<StateKeyValueV1>,
    pub unk11: BcList<StateKeyValueV1>,
    pub unk12: BcList<u64>,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct AnimationV1 {
    #[br(parse_with = parse_string_ptr64)]
    #[xc3(offset(u64))]
    pub file_name: String,

    pub unk1: BcList<AnimationUnk1V1>,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct AnimationUnk1V1 {
    pub unk1: u32,
    pub unk2: i32,
}

// TODO: more fields?
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct StateTransitionV1 {
    #[br(parse_with = parse_string_ptr64)]
    #[xc3(offset(u64))]
    pub name: String,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct KeyValueV1 {
    #[br(parse_with = parse_string_ptr64)]
    #[xc3(offset(u64))]
    pub key: String,

    #[br(parse_with = parse_string_ptr64)]
    #[xc3(offset(u64))]
    pub value: String,
}

// TODO: size?
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct StateKeyValueV1 {
    #[br(parse_with = parse_string_ptr64)]
    #[xc3(offset(u64))]
    pub key: String,

    pub unk1: u32,
    pub unk2: i32,

    #[br(parse_with = parse_string_ptr64)]
    #[xc3(offset(u64))]
    pub value: String,

    pub unk4: u32,
    pub unk5: i32,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct AsmbInnerV2 {
    pub unk1: BcList<u64>,
    pub unk2: BcList<FsmGroupV2>,
    pub unk3: u64,        // 0?
    pub unk4: BcList<u8>, // TODO: type?
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct FsmGroupV2 {
    pub unk1: BcList<BcOffset<StateV2>>,
    pub unk2: BcList<BcOffset<StateTransitionV2>>,

    #[br(parse_with = parse_string_ptr64)]
    #[xc3(offset(u64))]
    pub unk3: String,

    #[br(parse_with = parse_string_ptr64)]
    #[xc3(offset(u64))]
    pub unk4: String,

    #[br(parse_with = parse_string_opt_ptr64)]
    #[xc3(offset(u64))]
    pub unk5: Option<String>,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct StateV2 {
    #[br(parse_with = parse_string_ptr64)]
    #[xc3(offset(u64))]
    pub name: String,

    /// Hash of [name](#structfield.name) using [murmur3](crate::hash::murmur3).
    pub name_hash: u32,
    pub unk3: u32,
    pub unk4: u32,
    pub unk5: u32,
    pub unk6: u32,
    pub unk7: u32,

    // TODO: types?
    pub children: BcList<BcOffset<StateTransitionV2>>,
    pub unk9: BcList<u8>,
    pub unk10: BcList<u8>,
    pub unk11: BcList<u8>,
    pub unk12: BcList<u8>,
    pub unk13: BcList<u8>,

    // TODO: only in xc3?
    #[br(parse_with = parse_string_ptr64)]
    #[xc3(offset(u64))]
    pub unk14: String,

    pub unk15: [f32; 8],
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct StateTransitionV2 {
    #[br(parse_with = parse_string_ptr64)]
    #[xc3(offset(u64))]
    pub name1: String,

    #[br(parse_with = parse_string_ptr64)]
    #[xc3(offset(u64))]
    pub name2: String,

    /// Hash of [name2](#structfield.name) using [murmur3](crate::hash::murmur3).
    pub name2_hash: u32,
    pub unk4: [f32; 4],
    pub unk5: i32,
    pub unk6: [f32; 2],
    pub unk7: u32,
    pub unk8: u32,
    pub unk9: [i16; 8],
    pub unk10: f32,
    pub unk11: u32,
    pub unk12: u32,
    pub unk13: i32,
}
