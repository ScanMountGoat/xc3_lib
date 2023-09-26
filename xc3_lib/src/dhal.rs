//! User interface [Mibl](crate::mibl::Mibl) images in `.wilay` files.
use crate::{parse_offset_count, parse_opt_ptr32};
use binrw::{binread, BinRead};

// TODO: LAGP files are similar?
// TODO: Dhal or Lahd?
/// .wilay images files for Xenoblade 2 and Xenoblade 3.
#[derive(BinRead, Debug)]
#[br(magic(b"LAHD"))]
pub struct Dhal {
    pub unk1: u32,
    pub unk2: u32,
    pub unk3: u32,
    pub unk4: u32,
    pub unk5: u32,
    pub unk6: u32,
    pub unk7: u32,
    pub unk8: u32,

    #[br(parse_with = parse_opt_ptr32)]
    pub textures: Option<Textures>,
    // TODO: more fields?
}

#[binread]
#[derive(Debug)]
#[br(stream = r)]
pub struct Textures {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    #[br(parse_with = parse_offset_count)]
    #[br(args { offset: base_offset, inner: base_offset })]
    pub textures: Vec<Texture>,
}

#[derive(BinRead, Debug)]
#[br(import_raw(base_offset: u64))]
pub struct Texture {
    pub unk1: u32,
    #[br(parse_with = parse_offset_count, offset = base_offset)]
    pub mibl_data: Vec<u8>,
}
