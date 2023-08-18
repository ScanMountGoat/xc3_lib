use binrw::{BinRead, BinResult};

use crate::{parse_count_offset, parse_offset_count, write::Xc3Write};

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
    // TODO: Support alignment constants.
    #[br(parse_with = parse_offset_count)]
    #[xc3(offset_count, align(4096))]
    pub mibl_data: Vec<u8>,
    pub unk1: u32,
    // TODO: padding?
    pub unks: [u32; 4],
}

// TODO: This can just be derived?
pub fn write_ltpc<W: std::io::Write + std::io::Seek>(ltpc: &Ltpc, writer: &mut W) -> BinResult<()> {
    let mut data_ptr = 0;

    let root = ltpc.write(writer, &mut data_ptr)?;
    let textures = root.textures.write_offset(writer, 0, &mut data_ptr)?;
    for texture in textures {
        texture.mibl_data.write_offset(writer, 0, &mut data_ptr)?;
    }

    Ok(())
}
