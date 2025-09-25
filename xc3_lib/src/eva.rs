//! Camera animations in `.eva` files or embedded in `.mot` files.
use crate::parse_ptr32;
use binrw::{BinRead, binread};
use xc3_write::{Xc3Write, Xc3WriteOffsets};

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(magic(b"eva\x00"))]
#[xc3(magic(b"eva\x00"))]
pub struct Eva {
    pub unk1: u32,
    pub item_count: u32,
    pub frame_count: u32, // frame count?

    #[br(count = item_count)]
    pub items: Vec<EvaItem1>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct EvaItem1 {
    // TODO: flags?
    pub unk1: u32,

    #[br(parse_with = parse_ptr32)]
    #[xc3(offset(u32))]
    pub item2: EvaItem2,
}

#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct EvaItem2 {
    #[br(temp, try_calc = r.stream_position())]
    _base_offset: u64,

    // TODO: The float array isn't always present?
    // TODO: type?
    // #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    // #[xc3(offset_count(u32, u32))]
    // pub items: Vec<u8>,
    pub unk1: u32, // TODO: offset to next EvaItem2?
    pub unk2: u32,

    pub frame_count: u32,
    // TODO: What controls if there is a float array here?
    // #[br(args { count: float_count as usize })]
    // pub floats: Vec<f32>,
}
