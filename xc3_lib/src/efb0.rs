//! Effects in .wiefb files.
//!
//! # File Paths
//!
//! | Game | Versions | File Patterns |
//! | --- | --- | --- |
//! | Xenoblade Chronicles 1 DE | |  |
//! | Xenoblade Chronicles 2 |  | `effect/**/*.wiefb` |
//! | Xenoblade Chronicles 3 |  |  |
use crate::{parse_opt_ptr32, xc3_write_binwrite_impl};
use binrw::{binread, BinRead, BinWrite, NullString};
use xc3_write::{Xc3Write, Xc3WriteOffsets};

// TODO: .wieab also has data?
#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(magic(b"efb0"))]
#[xc3(magic(b"efb0"))]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct Efb0 {
    // Subtract the magic size.
    #[br(temp, try_calc = r.stream_position().map(|p| p - 4))]
    base_offset: u64,

    pub version: (u16, u16),
    pub unk1: u32,

    // TODO: Why is this a linked list?
    #[br(parse_with = parse_opt_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub next_efb0: Option<Box<Efb0>>,

    pub unk2: u32,

    pub text: EfbString,
    // TODO: embedded mxmd, mibl, hcps?
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, BinWrite, PartialEq, Clone)]
pub struct EfbString(
    #[br(map = |x: NullString| x.to_string())]
    #[bw(map = |x: &String| NullString::from(x.as_str()))]
    #[brw(pad_size_to = 60)]
    String,
);

xc3_write_binwrite_impl!(EfbString);
