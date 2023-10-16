//! Camera animations images in `.eva` files or embedded in `.mot` files.
use crate::{parse_offset32_count32, parse_ptr32};
use binrw::{binread, BinRead};
use xc3_write::{Xc3Write, Xc3WriteOffsets};

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
#[br(magic(b"eva\x00"))]
#[xc3(magic(b"eva\x00"))]
pub struct Eva {
    pub unk1: u32,
    pub item_count: u32,
    pub unk3: u32, // frame count?

    #[br(count = item_count)]
    pub items: Vec<EvaItem1>,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct EvaItem1 {
    // TODO: flags?
    pub unk1: u32,

    #[br(parse_with = parse_ptr32)]
    #[xc3(offset32)]
    pub item2: EvaItem2,
}

#[binread]
#[derive(Debug, Xc3Write, Xc3WriteOffsets)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct EvaItem2 {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    // TODO: type?
    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset32)]
    pub items: Vec<u8>,

    pub float_count: u32,
    #[br(args { count: float_count as usize })]
    pub floats: Vec<f32>,
}
