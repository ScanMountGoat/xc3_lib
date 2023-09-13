use binrw::BinRead;

use crate::{
    parse_count_offset, parse_offset_count,
    write::{Xc3Write, Xc3WriteFull},
};

/// `monolib/shader/filterlut.wiltp` for Xenoblade 3.
#[derive(Debug, BinRead, Xc3Write, Xc3WriteFull)]
#[br(magic(b"LTPC"))]
#[xc3(magic(b"LTPC"))]
pub struct Ltpc {
    pub version: u32,

    /// A collection of typically 3D texture files.
    #[br(parse_with = parse_count_offset)]
    #[xc3(count_offset)]
    pub textures: Vec<Texture>,

    // TODO: padding?
    pub unk: [u32; 6],
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteFull)]
pub struct Texture {
    // TODO: Support alignment constants.
    #[br(parse_with = parse_offset_count)]
    #[xc3(offset_count, align(4096))]
    pub mibl_data: Vec<u8>,
    pub unk1: u32,
    // TODO: padding?
    pub unks: [u32; 4],
}
