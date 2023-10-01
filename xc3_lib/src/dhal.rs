//! User interface [Mibl](crate::mibl::Mibl) images in `.wilay` files.
use crate::{parse_count_offset, parse_offset_count, parse_opt_ptr32, parse_ptr32};
use binrw::{binread, BinRead};
use xc3_write::{Xc3Write, Xc3WriteOffsets};

// TODO: LAGP files are similar?
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
#[br(magic(b"LAHD"))]
#[xc3(magic(b"LAHD"))]
pub struct Dhal {
    pub version: u32,

    #[br(parse_with = parse_count_offset)]
    #[xc3(count32_offset32)]
    pub unk1: Vec<[f32; 15]>,

    // TODO: always 0?
    pub unk2: u32,

    #[br(parse_with = parse_ptr32)]
    #[xc3(offset32)]
    pub unk3: Unk3,

    // TODO: more offsets?
    #[br(parse_with = parse_ptr32)]
    #[xc3(offset32)]
    pub unk4: Unk4,

    #[br(parse_with = parse_ptr32)]
    #[xc3(offset32)]
    pub unk5: [u32; 4],

    #[br(parse_with = parse_ptr32)]
    #[xc3(offset32)]
    pub unk6: [u32; 3],

    #[br(parse_with = parse_opt_ptr32)]
    #[xc3(offset32)]
    pub textures: Option<Textures>,

    // TODO: more fields?
    pub unks1: [u32; 7],

    // TODO: padding?
    pub unk: [u32; 10],
}

#[binread]
#[derive(Debug, Xc3Write, Xc3WriteOffsets)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct Unk3 {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    #[br(parse_with = parse_offset_count, offset = base_offset)]
    #[xc3(offset32_count32)]
    pub unk1: Vec<[u32; 7]>,

    #[br(parse_with = parse_offset_count, offset = base_offset)]
    #[xc3(offset32_count32)]
    pub unk2: Vec<[u32; 4]>,

    #[br(parse_with = parse_offset_count, offset = base_offset)]
    #[xc3(offset32_count32)]
    pub unk3: Vec<[u32; 5]>,

    // TODO: padding?
    pub unk: [u32; 4],
}

#[binread]
#[derive(Debug, Xc3Write, Xc3WriteOffsets)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct Unk4 {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    pub unk1: u32,
    pub unk2: u32,
    pub unk3: u32,
    pub unk4: u32,
    pub unk5: u32,
    pub unk6: u32,
    pub unk7: u32,
    pub unk8: u32,

    // TODO: padding?
    pub unk: [u32; 4],
}

#[binread]
#[derive(Debug, Xc3Write, Xc3WriteOffsets)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct Textures {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    #[br(parse_with = parse_offset_count)]
    #[br(args { offset: base_offset, inner: base_offset })]
    #[xc3(offset32_count32)]
    pub textures: Vec<Texture>,

    // TODO: padding?
    pub unk: [u32; 4],
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
#[br(import_raw(base_offset: u64))]
pub struct Texture {
    pub unk1: u32,
    #[br(parse_with = parse_offset_count, offset = base_offset)]
    #[xc3(offset32_count32, align(4096))]
    pub mibl_data: Vec<u8>,
}
