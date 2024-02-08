//! Texture container of [Mibl](crate::mibl::Mibl) images in `.wiltp` files.
use binrw::BinRead;

use crate::{parse_count32_offset32, parse_offset32_count32};
use xc3_write::{Xc3Write, Xc3WriteOffsets};

/// `monolib/shader/filterlut.wiltp` for Xenoblade 3.
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(magic(b"LTPC"))]
#[xc3(magic(b"LTPC"))]
pub struct Ltpc {
    pub version: u32,

    /// A collection of typically 3D texture files.
    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub textures: Vec<Texture>,

    // TODO: padding?
    pub unk: [u32; 6],
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct Texture {
    // TODO: Support alignment constants.
    #[br(parse_with = parse_offset32_count32)]
    #[xc3(offset_count(u32, u32), align(4096))]
    pub mibl_data: Vec<u8>,
    pub unk1: u32,
    // TODO: padding?
    pub unks: [u32; 4],
}
