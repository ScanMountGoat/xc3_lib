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

    // TODO: temp?
    #[br(restore_position)]
    #[xc3(skip)]
    pub offset: u32,

    // TODO: alignment is sometimes 16?
    #[br(parse_with = parse_ptr32)]
    #[xc3(offset(u32))]
    pub unk1: Unk1,

    // TODO: alignment isn't always 2 for all types?
    #[br(parse_with = parse_opt_ptr32)]
    #[xc3(offset(u32), align(2))]
    pub unk2: Option<Unk2>,

    #[br(parse_with = parse_opt_ptr32)]
    #[xc3(offset(u32), align(2))]
    pub unk3: Option<Unk3>,

    // TODO: align 16 for xc3?
    #[br(parse_with = parse_opt_ptr32)]
    #[br(args { inner: args! { offset, version } })]
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

    pub unks_3: u32,

    #[br(parse_with = parse_opt_ptr32)]
    #[xc3(offset(u32))]
    pub unk8: Option<Unk8>,
    pub unk8_1: u32, // count?

    // TODO: more fields?
    pub unks1: u32,

    #[br(parse_with = parse_opt_ptr32)]
    #[xc3(offset(u32), align(2))]
    pub unk9: Option<Unk9>,

    #[br(parse_with = parse_opt_ptr32)]
    #[xc3(offset(u32))]
    pub uncompressed_textures: Option<UncompressedTextures>,

    // TODO: padding?
    pub unk: [u32; 7],

    // TODO: 4 more bytes of padding for xc3?
    #[br(if(offset >= 108))]
    pub unks2: Option<[u32; 2]>,

    #[br(if(offset >= 112))]
    pub unks3: Option<u32>,
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

    // TODO: Describes sections of buffer?
    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32), align(2))]
    pub unk1: Vec<Unk2Unk1>,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32), align(2))]
    pub unk2: Vec<Unk2Unk2>,

    // TODO: Infer the length somehow?
    // TODO: params with f32, f32, ..., 0xffffffff?
    // TODO: what determines the remaining data count?
    #[br(parse_with = parse_ptr32)]
    #[br(args { offset: base_offset, inner: args! { count: buffer_size(&unk1, &unk2) }})]
    #[xc3(offset(u32), align(4096))]
    pub buffer: Vec<u8>,

    pub unk4: u32, // 4096?

    // TODO: padding?
    pub unk: [u32; 4],
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct Unk2Unk1 {
    // TODO: array of [u32; 5]?
    pub data_offset: u32,
    pub count: u32,
    pub unk: u32,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct Unk2Unk2 {
    // TODO: array of u16?
    pub data_offset: u32,
    pub count: u32,
}

#[binread]
#[derive(Debug, Xc3Write, Xc3WriteOffsets)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct Unk3 {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    #[br(parse_with = parse_offset32_count32)]
    #[br(args { offset: base_offset, inner: base_offset})]
    #[xc3(offset_count(u32, u32))]
    pub unk1: Vec<Unk3Unk1>,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub unk2: Vec<[u32; 4]>,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub unk3: Vec<[u16; 3]>,

    // TODO: padding?
    pub unk: [u32; 4],
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
#[br(import_raw(base_offset: u64))]
pub struct Unk3Unk1 {
    pub unk1: (u16, u16),

    #[br(parse_with = parse_opt_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub unk2: Option<[u32; 3]>,

    pub unk3: u32,
    pub unk4: u32,
    pub unk5: u32,
    pub unk6: u32,
    pub unk7: (u16, u16),
}

#[binread]
#[derive(Debug, Xc3Write)]
#[br(stream = r)]
#[br(import { version: u32, offset: u32 })]
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

    // TODO: Should xc3 be treated as a separate format?
    #[br(if(offset >= 112))]
    pub unk: Option<[u32; 3]>,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
#[br(import_raw(base_offset: u64))]
pub struct Unk4Extra {
    #[br(parse_with = parse_opt_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub unk1: Option<[u32; 4]>,

    // TODO: padding?
    pub unk: u32,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
#[br(import_raw(base_offset: u64))]
pub struct Unk4Unk2 {
    #[br(parse_with = parse_count32_offset32, offset = base_offset)]
    #[xc3(count_offset(u32, u32), align(2))]
    pub unk1: Vec<u32>,

    #[br(parse_with = parse_opt_ptr32, offset = base_offset)]
    #[xc3(offset(u32), align(2))]
    pub unk3: Option<[f32; 2]>,

    #[br(restore_position)]
    #[xc3(skip)]
    _temp: [u32; 4],

    // TODO: count depends on unk1 length?
    #[br(parse_with = parse_opt_ptr32)]
    #[br(args {
        offset: base_offset,
        inner: args! { count: (_temp[3] - _temp[0]) as usize / 8 }
    })]
    #[xc3(offset(u32), align(2))]
    pub unk4: Option<Vec<[u32; 2]>>,

    pub unk5: u32, // 0?
    pub unk6: u32, // 0?

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

    pub unk11: u32,

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

#[binread]
#[derive(Debug, Xc3Write, Xc3WriteOffsets)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct Unk5 {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32), align(2))]
    pub unk1: Vec<[u32; 2]>,
}

#[binread]
#[derive(Debug, Xc3Write, Xc3WriteOffsets)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct Unk6 {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32), align(2))]
    pub unk1: Vec<u32>,
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
pub struct Unk8 {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    pub unk1: u32,

    #[br(parse_with = parse_offset32_count32)]
    #[br(args { offset: base_offset, inner: base_offset})]
    #[xc3(offset_count(u32, u32))]
    pub unk2: Vec<Unk8Item>,

    // TODO: padding?
    pub unk: [u32; 12],
}

// TODO: pointers to strings?
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
#[br(import_raw(base_offset: u64))]
pub struct Unk8Item {
    pub unk1: u32,
    pub unk2: u32,
    pub index: u32,

    // TODO: string or ints + string?
    #[br(parse_with = parse_ptr32)]
    #[br(args { offset: base_offset, inner: args! { count: if unk2 == 0 { 8 } else { 16} }})]
    #[xc3(offset(u32), align(2))]
    pub data: Vec<u8>,
    pub unk5: u32, // TODO: data type?

    pub unk6: u32,
    pub unk7: u32,
    // TODO: padding?
    pub unk: [u32; 4],
}

#[binread]
#[derive(Debug, Xc3Write, Xc3WriteOffsets)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct Unk9 {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32), align(2))]
    pub unk1: Vec<Unk9Item>,

    // TODO: padding?
    pub unk: [u32; 4],
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct Unk9Item {
    pub unk1: [i32; 5],
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
        self.unk9.write_full(writer, base_offset, data_ptr)?;
        self.unk5.write_full(writer, base_offset, data_ptr)?;
        self.unk6.write_full(writer, base_offset, data_ptr)?;
        self.unk8.write_full(writer, base_offset, data_ptr)?;
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

        let unk2s = self.unk2.write(writer, base_offset, data_ptr)?;
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

fn buffer_size(unk1: &[Unk2Unk1], unk2: &[Unk2Unk2]) -> usize {
    // Assume data is tightly packed and starts from 0.
    // TODO: extra data?
    unk1.iter().map(|u| u.count as usize * 20).sum::<usize>()
        + unk2.iter().map(|u| u.count as usize * 2).sum::<usize>()
}
