//! User interface [Mibl](crate::mibl::Mibl) images in `.wilay` files.

//! # File Paths
//! Xenoblade 1 `.wilay` [Dhal] are in [Xbc1](crate::xbc1::Xbc1) archives.
//!
//! | Game | Versions | File Patterns |
//! | --- | --- |
//! | Xenoblade Chronicles 1 DE | 10001, 10003 | `menu/image/*.wilay` |
//! | Xenoblade Chronicles 2 | 10001 | `menu/image/*.wilay` |
//! | Xenoblade Chronicles 3 | 10003 | `menu/image/*.wilay` |
use std::io::Cursor;

use crate::{
    parse_count32_offset32, parse_offset32_count32, parse_opt_ptr32, parse_ptr32,
    xc3_write_binwrite_impl,
};
use binrw::{args, binread, BinRead, BinWrite};
use xc3_write::{Xc3Write, Xc3WriteOffsets};

// TODO: LAGP files are similar?
// TODO: LAPS files are similar?
#[derive(Debug, BinRead, Xc3Write)]
#[br(magic(b"LAHD"))]
#[xc3(magic(b"LAHD"))]
pub struct Dhal {
    // TODO: enum?
    pub version: u32,

    // TODO: changes remaining fields?
    pub unk0: Unk0,

    // TODO: alignment is sometimes 16?
    #[br(parse_with = parse_ptr32)]
    #[xc3(offset(u32))]
    pub unk1: Unk1,

    #[br(parse_with = parse_opt_ptr32)]
    #[xc3(offset(u32), align(2))]
    pub unk2: Option<Unk2>,

    #[br(parse_with = parse_opt_ptr32)]
    #[xc3(offset(u32), align(2))]
    pub unk3: Option<Unk3>,

    #[br(parse_with = parse_opt_ptr32)]
    #[br(args { inner: version })]
    #[xc3(offset(u32), align(2))]
    pub unk4: Option<Unk4>,

    #[br(parse_with = parse_opt_ptr32)]
    #[xc3(offset(u32), align(2))]
    pub unk5: Option<Unk5>,

    #[br(parse_with = parse_opt_ptr32)]
    #[xc3(offset(u32), align(2))]
    pub unk6: Option<Unk6>,

    #[br(parse_with = parse_opt_ptr32)]
    #[xc3(offset(u32), align(2))]
    pub textures: Option<Textures>,

    // array?
    pub unks_2: u32,

    #[br(parse_with = parse_opt_ptr32)]
    #[xc3(offset(u32), align(2))]
    pub unk7: Option<Unk7>,

    // TODO: more fields?
    pub unks1: [u32; 5],

    #[br(parse_with = parse_opt_ptr32)]
    #[xc3(offset(u32))]
    pub uncompressed_textures: Option<UncompressedTextures>,

    // TODO: padding?
    pub unk: [u32; 7],

    #[br(if(version > 10001))]
    pub unks2: Option<[u32; 3]>,
}

// TODO: Is this actually flags?
#[derive(Debug, BinRead, BinWrite, Clone, Copy, PartialEq, Eq, Hash)]
#[brw(repr(u32))]
pub enum Unk0 {
    Unk0 = 0,     // images?
    Unk1 = 1,     // images?
    Unk3 = 3,     // images?
    Unk17 = 17,   // ???
    Unk32 = 32,   // strings?
    Unk129 = 129, // vol?
}

#[binread]
#[derive(Debug, Xc3Write, Xc3WriteOffsets)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct Unk1 {
    pub unk1: u32,
    pub unk2: u32,
    pub unk3: u32,
    pub unk4: u32,
    pub unk5: f32,
    pub unk6: f32,
    pub unk7: u32,
    pub unk8: f32,
    pub unk9: f32,
    pub unk10: f32,
    pub unk11: f32,

    // TODO: padding?
    pub unk: [u32; 4],
}

#[binread]
#[derive(Debug, Xc3Write, Xc3WriteOffsets)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct Unk2 {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub unk1: Vec<[u32; 3]>,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub unk2: Vec<[u32; 2]>,

    // TODO: type?
    // TODO: Some lagp files don't have enough bytes?
    // TODO: params with f32, f32, ..., 0xffffffff?
    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub unk3: Vec<u8>,

    // TODO: padding?
    pub unk: [u32; 4],
}

#[binread]
#[derive(Debug, Xc3Write, Xc3WriteOffsets)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct Unk3 {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    // TODO: This type is sometimes larger?
    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub unk1: Vec<[u32; 7]>,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub unk2: Vec<[u32; 4]>,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub unk3: Vec<[u16; 3]>,

    // TODO: padding?
    pub unk: [u32; 4],
}

#[binread]
#[derive(Debug, Xc3Write)]
#[br(stream = r)]
#[br(import_raw(version: u32))]
#[xc3(base_offset)]
pub struct Unk4 {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    pub unk1: u32, // 0

    #[br(parse_with = parse_offset32_count32)]
    #[br(args { offset: base_offset, inner: base_offset })]
    #[xc3(offset_count(u32, u32), align(2))]
    pub unk2: Vec<Unk4Unk2>,

    pub unk4: u32, // pointer before strings?
    pub unk5: u32, // pointer to string offsets?
    pub unk6: u32, // 0?
    pub unk7: u32, // pointer before strings?

    // TODO: Is this the right check?
    #[br(if(version > 10001))]
    #[br(args_raw(base_offset))]
    pub extra: Option<Unk4Extra>,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
#[br(import_raw(base_offset: u64))]
pub struct Unk4Extra {
    #[br(parse_with = parse_opt_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub unk1: Option<[u32; 4]>,

    // TODO: padding?
    pub unk: [u32; 4],
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
#[br(import_raw(base_offset: u64))]
pub struct Unk4Unk2 {
    #[br(parse_with = parse_count32_offset32, offset = base_offset)]
    #[xc3(count_offset(u32, u32), align(2))]
    pub unk1: Vec<u32>,

    // TODO: Just store offsets to calculate counts for now?
    // TODO: Count can be 44?
    #[br(parse_with = parse_opt_ptr32, offset = base_offset)]
    #[xc3(offset(u32), align(2))]
    pub unk3: Option<[f32; 2]>,

    // TODO: count depends on unk1?
    #[br(parse_with = parse_opt_ptr32)]
    #[br(args { offset: base_offset, inner: args! { count: unk1.len().saturating_sub(1).max(1) }})]
    #[xc3(offset(u32), align(2))]
    pub unk4: Option<Vec<[u32; 2]>>,

    pub unk5: u32,
    pub unk6: u32, // 0?

    // TODO: count depends on unk1?
    #[br(parse_with = parse_opt_ptr32)]
    #[br(args { offset: base_offset, inner: args! { count: unk1.len().next_multiple_of(4) / 4 }})]
    #[xc3(offset(u32), align(2))]
    pub unk7: Option<Vec<u32>>,

    #[br(parse_with = parse_opt_ptr32)]
    #[br(args { offset: base_offset, inner: args! { count: unk1.len() }})]
    #[xc3(offset(u32), align(2))]
    pub unk8: Option<Vec<u8>>,

    // TODO: not always 0?
    pub unk9: u32,  // 0
    pub unk10: u32, // 0

    // #[br(parse_with = parse_opt_ptr32, offset = base_offset)]
    // #[xc3(offset(u32), align(2))]
    pub unk11: u32, //Option<[u32; 2]>,

    pub unk12: u32, // 0
    // TODO: not padding?
    pub unk13: [u32; 4],
}

#[binread]
#[derive(Debug, Xc3Write, Xc3WriteOffsets)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct Unk4Unk7 {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    // TODO: strings?
    // TODO: size and type?
    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub unk1: Vec<[i32; 5]>,

    // TODO: padding?
    pub unk: [u32; 4],
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct Unk5 {
    pub unk1: u32,
    pub unk2: u32,
    pub unk3: u32,
    pub unk4: u32,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct Unk6 {
    pub unk1: u32,
    pub unk2: u32,
    pub unk3: u32,
}

#[binread]
#[derive(Debug, Xc3Write, Xc3WriteOffsets)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct Unk7 {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    pub count: u32,

    pub unk1: [u32; 3],
    pub unk2: [f32; 2],

    #[br(parse_with = parse_ptr32)]
    #[br(args { offset: base_offset, inner: args! { count: count as usize }})]
    #[xc3(offset(u32), align(2))]
    pub items: Vec<Unk7Item>,

    // TODO: padding?
    pub unk: [u32; 4],
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct Unk7Item {
    pub unk1: [f32; 6],
    pub unk2: u16,
    pub unk3: u16,
}

#[binread]
#[derive(Debug, Xc3Write, Xc3WriteOffsets)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct Textures {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    #[br(parse_with = parse_offset32_count32)]
    #[br(args { offset: base_offset, inner: base_offset })]
    #[xc3(offset_count(u32, u32), align(2))]
    pub textures: Vec<Texture>,

    // TODO: padding?
    pub unk: [u32; 4],
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
#[br(import_raw(base_offset: u64))]
pub struct Texture {
    // TODO: 1000, 1001, 1002?
    pub unk1: u32,
    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32), align(4096))]
    pub mibl_data: Vec<u8>,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct UncompressedTextures {
    // TODO: does this always use base offset 0?
    #[br(parse_with = parse_offset32_count32)]
    #[xc3(offset_count(u32, u32))]
    pub textures: Vec<UncompressedTexture>,

    // TODO: padding?
    pub unk: [u32; 4],
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct UncompressedTexture {
    // TODO: always JFIF?
    /// JFIF/JPEG image file data commonly saved with the `.jfif` or `.jpeg` extension.
    #[br(parse_with = parse_offset32_count32)]
    #[xc3(offset_count(u32, u32))]
    pub jpeg_data: Vec<u8>,

    pub unk3: u32,
    pub unk4: u32,
}

impl UncompressedTexture {
    /// Decode the JPEG/JFIF data to an RGB image.
    pub fn to_image(
        &self,
    ) -> Result<image_dds::image::RgbImage, image_dds::image::error::ImageError> {
        let mut reader = image_dds::image::io::Reader::new(Cursor::new(&self.jpeg_data));
        reader.set_format(image_dds::image::ImageFormat::Jpeg);
        Ok(reader.decode()?.into_rgb8())
    }
}

xc3_write_binwrite_impl!(Unk0);

impl<'a> Xc3WriteOffsets for DhalOffsets<'a> {
    fn write_offsets<W: std::io::prelude::Write + std::io::prelude::Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
    ) -> xc3_write::Xc3Result<()> {
        // Different order than field order.
        self.unk1.write_full(writer, base_offset, data_ptr)?;
        self.unk3.write_full(writer, base_offset, data_ptr)?;
        self.unk7.write_full(writer, base_offset, data_ptr)?;
        self.unk4.write_full(writer, base_offset, data_ptr)?;
        self.unk5.write_full(writer, base_offset, data_ptr)?;
        self.unk6.write_full(writer, base_offset, data_ptr)?;
        self.unk2.write_full(writer, base_offset, data_ptr)?;
        self.textures.write_full(writer, base_offset, data_ptr)?;
        self.uncompressed_textures
            .write_full(writer, base_offset, data_ptr)?;
        Ok(())
    }
}

impl<'a> Xc3WriteOffsets for Unk4Offsets<'a> {
    fn write_offsets<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        _base_offset: u64,
        data_ptr: &mut u64,
    ) -> xc3_write::Xc3Result<()> {
        // Different order than field order.
        let base_offset = self.base_offset;

        let unk2s = self.unk2.write_offset(writer, base_offset, data_ptr)?;
        for unk2 in &unk2s.0 {
            unk2.unk1.write_full(writer, base_offset, data_ptr)?;
        }
        for unk2 in &unk2s.0 {
            unk2.unk3.write_full(writer, base_offset, data_ptr)?;
            unk2.unk4.write_full(writer, base_offset, data_ptr)?;
            unk2.unk7.write_full(writer, base_offset, data_ptr)?;
            unk2.unk8.write_full(writer, base_offset, data_ptr)?;
        }

        self.extra.write_offsets(writer, base_offset, data_ptr)?;
        Ok(())
    }
}
