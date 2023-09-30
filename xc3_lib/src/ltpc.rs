//! Texture container of [Mibl](crate::mibl::Mibl) images in `.wiltp` files.
use binrw::BinRead;

use crate::{
    parse_count_offset, parse_offset_count,
    write::{Xc3Write, Xc3WriteOffsets},
};

/// `monolib/shader/filterlut.wiltp` for Xenoblade 3.
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
#[br(magic(b"LTPC"))]
#[xc3(magic(b"LTPC"))]
pub struct Ltpc {
    pub version: u32,

    /// A collection of typically 3D texture files.
    #[br(parse_with = parse_count_offset)]
    #[xc3(count32_offset32)]
    pub textures: Vec<Texture>,

    // TODO: padding?
    pub unk: [u32; 6],
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct Texture {
    // TODO: Support alignment constants.
    #[br(parse_with = parse_offset_count)]
    #[xc3(offset32_count32, align(4096))]
    pub mibl_data: Vec<u8>,
    pub unk1: u32,
    // TODO: padding?
    pub unks: [u32; 4],
}
