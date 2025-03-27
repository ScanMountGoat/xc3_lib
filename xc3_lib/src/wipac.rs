//! Effects in .wipac files.
//!
//! # File Paths
//!
//! | Game | Versions | File Patterns |
//! | --- | --- | --- |
//! | Xenoblade 1 DE | |  |
//! | Xenoblade 2 |  |  |
//! | Xenoblade 3 |  | `effect/**/*.wipac`  |
use crate::{parse_ptr32, xc3_write_binwrite_impl, Offset32};
use binrw::{binread, BinRead, BinWrite, NullString};
use xc3_write::{Xc3Write, Xc3WriteOffsets};

// TODO: Come up with a better name
// TODO: implement proper write support
#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(magic(b"ARC\x00"))]
#[xc3(magic(b"ARC\x00"))]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct Wipac {
    pub unk1: u32,
    pub unk2: u32,
    pub unk3: u32,

    // FirehawkBinarys
    #[br(map(|x: NullString| x.to_string()))]
    #[br(pad_size_to = 32)]
    pub unk4: String,

    #[br(count = unk2, align_after = 16)]
    pub unk5: Vec<Offset32<Unk>>,

    #[br(count = unk2)]
    pub unk6: Vec<Efxa>,
}

#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct Unk {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    #[br(parse_with = parse_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub cmp: Cmp,

    pub unk2: u32,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(magic(b"cmp\x00"))]
#[xc3(magic(b"cmp\x00"))]
pub struct Cmp {
    pub cmp_type: CmpType,
    pub compressed_size: u32,
    pub decompressed_size: u32,

    #[br(count = compressed_size)]
    pub compressed_stream: Vec<u8>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(magic(b"efxa"))]
#[xc3(magic(b"efxa"))]
pub struct Efxa {
    pub unk1: u32,
    pub unk2: u32,
    pub unk3: u32,
    pub unk4: u32,

    #[br(map(|x: NullString| x.to_string()))]
    #[br(pad_size_to = 44)]
    pub unk5: String,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, BinWrite, Debug, Clone, Copy, PartialEq, Eq)]
pub enum CmpType {
    #[brw(magic(b"zlib"))]
    Zlib,
}

xc3_write_binwrite_impl!(CmpType);
