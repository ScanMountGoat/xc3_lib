//! Cutscene data in `.evpa` files.
//!
//! # File Paths
//! | Game | Versions | File Patterns |
//! | --- | --- | --- |
//! | Xenoblade 1 DE | | |
//! | Xenoblade 2 |  | `event/evpa/jp/*.evpa` |
//! | Xenoblade 3 |  | |
//! | Xenoblade X DE | | |
use crate::parse_offset32_count32;
use binrw::{BinRead, NullString};
use xc3_write::{Xc3Write, Xc3WriteOffsets};

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(magic(b"EVPA"))]
#[xc3(magic(b"EVPA"))]
#[xc3(align_after(4096))]
pub struct Evpa {
    pub entry_count: u32,
    pub unk2: u32,
    pub unk3: u32,

    #[br(count = entry_count)]
    pub entries: Vec<EvpaEntry>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct EvpaEntry {
    #[br(parse_with = parse_offset32_count32)]
    #[xc3(offset_count(u32, u32), align(4096))]
    pub entry_data: Vec<u8>,

    pub unk2: u32,
    pub unk3: u32,

    #[br(map = |x: NullString| x.to_string(), pad_size_to = 128)]
    #[xc3(pad_size_to(128))]
    pub name: String,
}
