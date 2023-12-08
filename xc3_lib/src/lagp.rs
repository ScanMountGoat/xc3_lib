//! User interface [Mibl](crate::mibl::Mibl) images in `.wilay` files.
//!
//! # File Paths
//! Xenoblade 1 and some Xenoblade 3 `.wilay` [Lagp] are in [Xbc1](crate::xbc1::Xbc1) archives.
//!
//! | Game | File Patterns |
//! | --- | --- |
//! | Xenoblade Chronicles 1 DE | `menu/image/*.wilay` |
//! | Xenoblade Chronicles 2 |  |
//! | Xenoblade Chronicles 3 | `menu/image/*.wilay` |
use crate::{dhal::Textures, parse_opt_ptr32};
use binrw::BinRead;
use xc3_write::{Xc3Write, Xc3WriteOffsets};

// TODO: How much of this is shared with LAHD?
// TODO: Is this used for xc2?
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
#[br(magic(b"LAGP"))]
#[xc3(magic(b"LAGP"))]
pub struct Lagp {
    pub version: u32, // 10003
    pub unk1: u32,    // 0
    pub unk2: u32,
    pub unk3: u32,
    pub unk4: u32,
    pub unk5: u32,
    pub unk6: u32,
    pub unk7: u32,

    #[br(parse_with = parse_opt_ptr32)]
    #[xc3(offset(u32))]
    pub textures: Option<Textures>,

    pub unk9: u32,
    pub unk10: u32,
    pub unk11: u32,
    pub unk12: u32,
    pub unk13: u32,
    pub unk14: u32,
    // TODO: padding?
    pub unk: [u32; 11],
}
