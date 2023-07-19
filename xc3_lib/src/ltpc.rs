use binrw::{BinRead, BinResult};

use crate::{
    parse_count_offset, parse_offset_count,
    write::{round_up, Xc3Write},
};

/// `monolib/shader/filterlut.wiltp` for Xenoblade 3.
#[derive(BinRead, Xc3Write, Debug)]
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

#[derive(BinRead, Xc3Write, Debug)]
pub struct Texture {
    #[br(parse_with = parse_offset_count)]
    #[xc3(offset_count)]
    pub mibl_data: Vec<u8>,
    pub unk1: u32,
    // TODO: padding?
    pub unks: [u32; 4],
}

// TODO: This can just be derived?
pub fn write_ltpc<W: std::io::Write + std::io::Seek>(root: &Ltpc, writer: &mut W) -> BinResult<()> {
    let mut data_ptr = 0;

    let root_offsets = root.write(writer, &mut data_ptr)?;
    let textures_offsets = root_offsets
        .textures
        .write_offset(writer, 0, &mut data_ptr)?;
    for offsets in textures_offsets {
        // TODO: Add alignment customization to derive?
        data_ptr = round_up(data_ptr, 4096);
        offsets.mibl_data.write_offset(writer, 0, &mut data_ptr)?;
    }

    Ok(())
}
