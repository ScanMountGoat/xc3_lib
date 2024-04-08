//! User interface data in `.bmn` files.
//!
//! # File Paths
//!
//! | Game | Versions | File Patterns |
//! | --- | --- | --- |
//! | Xenoblade Chronicles X | | `menu/**/*.bmn` |
use crate::{parse_offset32_count32, parse_opt_ptr32};
use binrw::{binread, BinRead};

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, PartialEq, Clone)]
#[br(magic(b"BMN\x20"))]
pub struct Bmn {
    pub unk1: u32,
    pub unk2: u32,
    pub unk3: u32,
    pub unk4: u32,
    pub unk5: u32,
    pub unk6: u32,
    pub unk7: u32,
    pub unk8: u32,
    pub unk9: u32,
    pub unk10: u32,
    pub unk11: u32,
    pub unk12: u32,
    pub unk13: u32,
    pub unk14: u32,
    pub unk15: u32,
    #[br(parse_with = parse_opt_ptr32)]
    pub unk16: Option<Unk16>,
    pub unk17: u32,
    pub unk18: u32,
}

#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Clone)]
#[br(stream = r)]
pub struct Unk16 {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    pub textures: Vec<Unk16Texture>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, PartialEq, Clone)]
pub struct Unk16Texture {
    #[br(parse_with = parse_offset32_count32)]
    pub mtxt_data: Vec<u8>,
}
