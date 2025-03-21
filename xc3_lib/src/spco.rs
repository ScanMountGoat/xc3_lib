use std::io::{Cursor, Read, Seek, SeekFrom};

use crate::{
    get_bytes, parse_count32_offset32, parse_offset32_count32, parse_opt_ptr32, parse_ptr32,
    parse_string_ptr32, spch::Spch, xc3_write_binwrite_impl, StringOffset32,
};
use binrw::{args, binread, BinRead, BinReaderExt, BinResult, BinWrite};
use xc3_write::{Xc3Write, Xc3WriteOffsets};

// TODO: Add example code for extracting shaders.
/// .wishp, embedded in .wismt and .wimdo
#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, PartialEq, Clone)]
#[br(magic(b"OCPS"))]
#[xc3(magic(b"OCPS"))]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct Spco {
    // Subtract the magic size.
    #[br(temp, try_calc = r.stream_position().map(|p| p - 4))]
    base_offset: u64,

    pub version: u32, // 1001

    #[br(parse_with = parse_count32_offset32)]
    #[br(args { offset: base_offset, inner: base_offset })]
    #[xc3(count_offset(u32, u32))]
    pub items: Vec<SpcoItem>,

    pub padding: [u32; 4],
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct SpcoItem {
    pub unk1: u32,

    #[br(parse_with = parse_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub spch: Spch,

    // TODO: offset?
    pub unk2: u32,

    pub unk3: u32,
    pub unk4: u32,
    pub unk5: u32,
}
