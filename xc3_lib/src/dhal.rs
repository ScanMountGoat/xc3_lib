//! User interface [Mibl](crate::mibl::Mibl) images in `.wilay` files.
use crate::{parse_offset_count, parse_opt_ptr32, parse_ptr32};
use binrw::{binread, BinRead};
use xc3_write::{Xc3Write, Xc3WriteOffsets};

// TODO: LAGP files are similar?
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
#[br(magic(b"LAHD"))]
#[xc3(magic(b"LAHD"))]
pub struct Dhal {
    pub version: u32,

    // TODO: 0 or 1 depending on type of next field?
    pub unk0: u32,

    #[br(parse_with = parse_ptr32)]
    #[xc3(offset32)]
    pub unk1: [f32; 15],

    // TODO: always 0?
    pub unk2: u32,

    #[br(parse_with = parse_opt_ptr32)]
    #[xc3(offset32)]
    pub unk3: Option<Unk3>,

    // TODO: more offsets?
    #[br(parse_with = parse_opt_ptr32)]
    #[xc3(offset32)]
    pub unk4: Option<Unk4>,

    #[br(parse_with = parse_opt_ptr32)]
    #[xc3(offset32)]
    pub unk5: Option<[u32; 4]>,

    #[br(parse_with = parse_opt_ptr32)]
    #[xc3(offset32)]
    pub unk6: Option<[u32; 3]>,

    #[br(parse_with = parse_opt_ptr32)]
    #[xc3(offset32)]
    pub textures: Option<Textures>,

    // TODO: more fields?
    pub unks1: [u32; 7],

    #[br(parse_with = parse_opt_ptr32)]
    #[xc3(offset32)]
    pub uncompressed_textures: Option<UncompressedTextures>,

    // TODO: padding?
    pub unk: [u32; 9],
}

#[binread]
#[derive(Debug, Xc3Write, Xc3WriteOffsets)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct Unk3 {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    #[br(parse_with = parse_offset_count, offset = base_offset)]
    #[xc3(offset32_count32)]
    pub unk1: Vec<[u32; 7]>,

    #[br(parse_with = parse_offset_count, offset = base_offset)]
    #[xc3(offset32_count32)]
    pub unk2: Vec<[u32; 4]>,

    #[br(parse_with = parse_offset_count, offset = base_offset)]
    #[xc3(offset32_count32)]
    pub unk3: Vec<[u32; 5]>,

    // TODO: padding?
    pub unk: [u32; 4],
}

#[binread]
#[derive(Debug, Xc3Write)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct Unk4 {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    pub unk1: u32, // 0

    #[br(parse_with = parse_offset_count)]
    #[br(args { offset: base_offset, inner: base_offset })]
    #[xc3(offset32_count32)]
    pub unk2: Vec<Unk4Unk2>,

    pub unk4: u32,
    pub unk5: u32,
    pub unk6: u32,

    #[br(parse_with = parse_ptr32, offset = base_offset)]
    #[xc3(offset32)]
    pub unk7: Unk4Unk7,

    #[br(parse_with = parse_ptr32, offset = base_offset)]
    #[xc3(offset32)]
    pub unk8: [u32; 4],

    // TODO: padding?
    pub unk: [u32; 4],
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
#[br(import_raw(base_offset: u64))]
pub struct Unk4Unk2 {
    // TODO: more offsets
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
    pub unk16: u32,
}

#[binread]
#[derive(Debug, Xc3Write, Xc3WriteOffsets)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct Unk4Unk7 {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    // TODO: size and type?
    #[br(parse_with = parse_offset_count, offset = base_offset)]
    #[xc3(offset32_count32)]
    pub unk1: Vec<[i32; 5]>,

    // TODO: padding?
    pub unk: [u32; 4],
}

#[binread]
#[derive(Debug, Xc3Write, Xc3WriteOffsets)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct Textures {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    #[br(parse_with = parse_offset_count)]
    #[br(args { offset: base_offset, inner: base_offset })]
    #[xc3(offset32_count32)]
    pub textures: Vec<Texture>,

    // TODO: padding?
    pub unk: [u32; 4],
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
#[br(import_raw(base_offset: u64))]
pub struct Texture {
    pub unk1: u32,
    #[br(parse_with = parse_offset_count, offset = base_offset)]
    #[xc3(offset32_count32, align(4096))]
    pub mibl_data: Vec<u8>,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct UncompressedTextures {
    // TODO: does this always use base offset 0?
    #[br(parse_with = parse_offset_count)]
    #[xc3(offset32_count32)]
    pub textures: Vec<UncompressedTexture>,

    // TODO: padding?
    pub unk: [u32; 4],
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct UncompressedTexture {
    // TODO: always JFIF?
    #[br(parse_with = parse_offset_count)]
    #[xc3(offset32_count32)]
    pub jfif_data: Vec<u8>,

    pub unk3: u32,
    pub unk4: u32,
}

impl<'a> Xc3WriteOffsets for Unk4Offsets<'a> {
    fn write_offsets<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        _base_offset: u64,
        data_ptr: &mut u64,
    ) -> binrw::BinResult<()> {
        // Different order than field order.
        let base_offset = self.base_offset;
        self.unk2.write_full(writer, base_offset, data_ptr)?;
        self.unk8.write_full(writer, base_offset, data_ptr)?;
        self.unk7.write_full(writer, base_offset, data_ptr)?;
        Ok(())
    }
}
