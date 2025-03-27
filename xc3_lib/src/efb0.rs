//! Effects in .wiefb files.
//!
//! # File Paths
//!
//! | Game | Versions | File Patterns |
//! | --- | --- | --- |
//! | Xenoblade 1 DE | |  |
//! | Xenoblade 2 | | `effect/**/*.wiefb` |
//! | Xenoblade 3 |  |  |
use crate::{parse_opt_ptr32, xc3_write_binwrite_impl};
use binrw::{binread, BinRead, BinWrite, NullString};
use xc3_write::{Xc3Write, Xc3WriteOffsets};

// TODO: .wieab also has data?
/// `ptlib::ParticleManager::parse_efs` in the Xenoblade 2 binary.
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

    // TODO: not present for bl200101_12_deathblow_00?
    // TODO: in game parser checks if type is 0x61 first?
    pub version: (u16, u16), // 2, 1

    // TODO: flags for data
    // 0x61 for start
    // 0x1 for data?
    // 0x50 for ???
    pub unk1: u32,

    // TODO: Why is this a linked list?
    #[br(parse_with = parse_opt_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub next_efb0: Option<Box<Efb0>>,

    // TODO: Difference between unk2 and next unk2 is next efb0 offset?
    pub unk2: u32,

    pub text: EfbString,
    // TODO: embedded data
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, BinWrite, PartialEq, Clone)]
pub struct EfbString(
    #[br(map = |x: NullString| x.to_string())]
    #[bw(map = |x: &String| NullString::from(x.as_str()))]
    #[brw(pad_size_to = 60)]
    String,
);

// TODO: Only present for type 0x1
/// `FxHeader` in Xenoblade 2 binary.
#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct FxHeader {
    #[br(temp, try_calc = r.stream_position())]
    _base_offset: u64,

    // TODO: Offsets relative to data start?
    // TODO: This repeats?
    pub unk1: u32,
    pub unk2: u32,
    pub unk3: i32,
    pub unk4: u32, // offset?
}

xc3_write_binwrite_impl!(EfbString);
