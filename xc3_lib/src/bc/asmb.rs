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

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct AsmbInner {
    pub unk1: BcList<u64>,
    pub unk2: BcList<AsmbUnk2>,
    pub unk3: u64,        // 0?
    pub unk4: BcList<u8>, // TODO: type?
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct AsmbUnk2 {
    pub unk1: BcList<BcOffset<AsmbUnk2Unk1>>,
    pub unk2: BcList<BcOffset<AsmbUnk2Unk1Unk8>>,

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
pub struct AsmbUnk2Unk1 {
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
    pub unk8: BcList<BcOffset<AsmbUnk2Unk1Unk8>>,
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
pub struct AsmbUnk2Unk1Unk8 {
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
