//! User interface [Mibl](crate::mibl::Mibl) images in `.wilay` files.
//!
//! # File Paths
//! Xenoblade 1 `.wilay` [Dhal] are in [Xbc1](crate::xbc1::Xbc1) archives.
//!
//! | Game | Versions | File Patterns |
//! | --- | --- | --- |
//! | Xenoblade 1 DE | 10001, 10003 | `menu/image/*.wilay` |
//! | Xenoblade 2 | 10001 | `menu/image/*.wilay` |
//! | Xenoblade 3 | 10003 | `menu/image/*.wilay` |
//! | Xenoblade X DE | 10003 | `ui/image/*.wilay` |
use std::collections::HashMap;

use crate::{
    parse_count32_offset32, parse_offset, parse_offset32_count32, parse_opt_ptr32, parse_ptr32,
    parse_string_ptr32, xc3_write_binwrite_impl,
};
use binrw::{args, binread, BinRead, BinWrite, NullString};
use indexmap::IndexMap;
use xc3_write::{Xc3Write, Xc3WriteOffsets};

// TODO: LAGP files are similar?
// TODO: LAPS files are similar?
#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, PartialEq, Clone)]
#[br(magic(b"LAHD"))]
#[xc3(magic(b"LAHD"))]
pub struct Dhal {
    // TODO: enum?
    #[xc3(save_position)]
    pub version: u32,

    // TODO: changes remaining fields?
    pub unk0: Unk0,

    #[br(temp, restore_position)]
    offsets: [u32; 15],

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

    // TODO: Pass in offsets that come after this for buffer size estimation?
    // TODO: align 16 for xc3?
    #[br(parse_with = parse_opt_ptr32)]
    #[br(args { inner: args! { offset: offsets[0], version } })]
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
    #[br(if(offsets[0] >= 108))]
    pub unks2: Option<[u32; 2]>,

    #[br(if(offsets[0] >= 112))]
    pub unks3: Option<u32>,
}

// TODO: Is this actually flags?
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, BinWrite, PartialEq, Eq, Clone, Copy, Hash)]
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
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
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
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct Unk2 {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    // TODO: Describes sections of buffer?
    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32), align(1))]
    pub unk1: Vec<Unk2Unk1>,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32), align(1))]
    pub unk2: Vec<Unk2Unk2>,

    // TODO: Infer the length somehow?
    // TODO: params with f32, f32, ..., 0xffffffff?
    // TODO: what determines the remaining data count?
    #[br(parse_with = parse_ptr32)]
    #[br(args { offset: base_offset, inner: args! { count: unk2_buffer_size(&unk1, &unk2) }})]
    #[xc3(offset(u32), align(4096))]
    pub buffer: Vec<u8>,

    pub unk4: u32, // 4096?

    // TODO: padding?
    pub unk: [u32; 4],
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct Unk2Unk1 {
    // TODO: array of (f32, f32, i32)?
    pub data_offset: u32,
    pub count: u32,
    pub unk: u32,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct Unk2Unk2 {
    // TODO: array of u16?
    pub data_offset: u32,
    pub count: u32,
}

#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
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

#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct Unk3Unk1 {
    pub unk1: (u16, u16),

    // TODO: Find a better way of expressing this count.
    #[br(temp, restore_position)]
    temp: [u16; 12],

    #[br(parse_with = parse_opt_ptr32)]
    #[br(args { offset: base_offset, inner: args! { count: temp[11] as usize * 2 } })]
    #[xc3(offset(u32))]
    pub unk2: Option<Vec<u16>>,

    pub unk3: u32,
    pub unk4: u32,
    pub unk5: u32,
    pub unk6: u32,
    pub unk7: u16,
    pub count: u16, // TODO: count for unk2 above?
}

#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, PartialEq, Clone)]
#[br(stream = r)]
#[br(import { version: u32, offset: u32 })]
#[xc3(base_offset)]
pub struct Unk4 {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    pub unk1: u32, // 0

    // TODO: Describes buffer?
    #[br(parse_with = parse_offset32_count32)]
    #[br(args { offset: base_offset, inner: base_offset })]
    #[xc3(offset_count(u32, u32), align(2))]
    pub unk2: Vec<Unk4Unk2>,

    #[br(parse_with = parse_opt_ptr32)]
    #[br(args { offset: base_offset, inner: unk2.iter().map(|u| u.unk_index + 1).max().unwrap_or_default() })]
    #[xc3(offset(u32))]
    pub unk4: Option<Unk4Unk4>,

    // TODO: Better way to determine this count?
    #[br(temp, restore_position)]
    #[br(parse_with = parse_opt_ptr32, offset = base_offset)]
    first_offset: Option<u32>,

    #[br(temp, restore_position)]
    unk5_offset: u32,

    // TODO: find a better way to determine the length.
    #[br(parse_with = parse_opt_ptr32)]
    #[br(args {
        offset: base_offset,
        inner: args! {
            count: (first_offset.unwrap_or(unk5_offset) - unk5_offset) as usize / 8,
            inner: base_offset
        }
    })]
    #[xc3(offset(u32))]
    pub unk5: Option<Vec<Unk4Unk5>>, // items?

    // TODO: Not all values are floats?
    #[br(parse_with = parse_count32_offset32, offset = base_offset)]
    #[xc3(count_offset(u32, u32), align(64))]
    pub unk6: Vec<[[f32; 4]; 8]>,

    // TODO: Is this the right check?
    #[br(if(version > 10001))]
    #[br(args_raw(base_offset))]
    pub extra: Option<Unk4Extra>,

    // TODO: Should xc3 be treated as a separate format?
    #[br(if(offset >= 112))]
    pub unk: Option<[u32; 3]>,

    // TODO: Find a cleaner way of preserving data.
    #[br(parse_with = parse_offset)]
    #[br(args {
        offset: base_offset + unk4_buffer_offset(&unk2),
        inner: args! { count: unk4_buffer_size(&unk2)}
    })]
    #[xc3(save_position, skip)]
    pub buffer: Vec<u8>,
}

// TODO: shared section for string keys and values?
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct Unk4Unk5 {
    #[br(parse_with = parse_string_ptr32, offset = base_offset)]
    #[xc3(offset(u32), align(1))]
    pub key: String,

    #[br(parse_with = parse_ptr32, offset = base_offset)]
    #[xc3(offset(u32), align(1))]
    pub value: Unk4Unk5Value,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Eq, Clone, Hash)]
pub struct Unk4Unk5Value {
    pub value_type: Unk4Unk5ValueType,
    #[br(args_raw(value_type))]
    pub value_data: Unk4Unk5ValueData,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Eq, Clone, Hash)]
#[br(import_raw(ty: Unk4Unk5ValueType))]
pub enum Unk4Unk5ValueData {
    #[br(pre_assert(ty == Unk4Unk5ValueType::Unk0))]
    Unk0(u32),

    #[br(pre_assert(ty == Unk4Unk5ValueType::Unk1))]
    Unk1(u64),

    #[br(pre_assert(ty == Unk4Unk5ValueType::Unk2))]
    Unk2(#[br(map(|x: NullString| x.to_string()))] String),
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, BinWrite, PartialEq, Eq, Clone, Copy, Hash)]
#[brw(repr(u8))]
pub enum Unk4Unk5ValueType {
    Unk0 = 0,
    Unk1 = 1,
    Unk2 = 2,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct Unk4Extra {
    // TODO: type?
    #[br(parse_with = parse_opt_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub unk1: Option<[u32; 4]>,

    // TODO: padding?
    pub unk: u32,
}

#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(_base_offset: u64))]
pub struct Unk4Unk2 {
    // TODO: count for u32?
    pub unk1: u32,

    // TODO: u32?
    pub unk2: u32,

    // TODO: floats?
    pub unk3: u32,

    pub unk4: u32,
    pub unk5: u32,
    pub unk6: u32,
    pub unk7: u32,
    pub unk8: u32,
    pub unk9: u32,
    pub unk10: u32,

    pub unk11: u32,
    pub unk12: u32, // 0
    pub unk13: u16,
    pub unk14: u16,
    pub unk15: u32,
    pub unk16: u32,
    pub unk17: u16,
    pub unk_index: u16, // TODO: index into Unk4Unk4?
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(stream = r)]
#[xc3(base_offset)]
#[br(import_raw(count: u16))]
pub struct Unk4Unk4 {
    #[br(count = count)]
    pub unk2: Vec<(u16, u16)>,
}

#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
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
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
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
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
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

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct Unk7Item {
    pub unk1: [f32; 6],
    pub unk2: u16,
    pub unk3: u16,
}

#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
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
#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct Unk8Item {
    pub unk1: u32,
    pub unk2: u32, // TODO: type or flags?
    pub index: u32,

    #[br(temp, restore_position)]
    offset_counts: [u32; 3],

    // TODO: string or ints + string?
    #[br(parse_with = parse_ptr32)]
    #[br(args { offset: base_offset, inner: args! { unk5: offset_counts[1], unk2 } })]
    #[xc3(offset(u32))]
    pub item: Unk8ItemInner,

    // TODO: number of u32?
    pub unk5: u32,

    // TODO: total size?
    pub unk6: u32,

    pub unk7: f32,

    // TODO: padding?
    pub unk: [u32; 4],
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import { unk5: u32, unk2: u32 })]
pub struct Unk8ItemInner {
    #[br(count = unk5)]
    pub unk1: Vec<u32>,

    #[br(if(unk2 == 65536))]
    pub unk2: Option<(u16, u16)>,

    // TODO: string?
    #[br(map(|s: NullString| s.to_string()))]
    pub unk3: String,
}

#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
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

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct Unk9Item {
    pub unk1: [i32; 5],
}

#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
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

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct Texture {
    // TODO: 1000, 1001, 1002?
    pub unk1: u32,
    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32), align(4096))]
    pub mibl_data: Vec<u8>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct UncompressedTextures {
    // TODO: does this always use base offset 0?
    #[br(parse_with = parse_offset32_count32)]
    #[xc3(offset_count(u32, u32))]
    pub textures: Vec<UncompressedTexture>,

    // TODO: padding?
    pub unk: [u32; 4],
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
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
    #[cfg(feature = "image")]
    pub fn to_image(&self) -> Result<image::RgbImage, image::error::ImageError> {
        let mut reader = image::ImageReader::new(std::io::Cursor::new(&self.jpeg_data));
        reader.set_format(image::ImageFormat::Jpeg);
        Ok(reader.decode()?.into_rgb8())
    }
}

xc3_write_binwrite_impl!(Unk0, Unk4Unk5ValueType);

#[derive(Default)]
struct Unk4KeyValueSection {
    // Keys and values both share a single data section.
    // Preserve insertion order to match the order in the file.
    value_to_offsets: IndexMap<Unk4Data, Vec<u64>>,
}

#[derive(Xc3Write, PartialEq, Eq, Hash)]
enum Unk4Data {
    Key(String),
    Value(Unk4Unk5Value),
}

impl Unk4KeyValueSection {
    fn insert_key(&mut self, offset: &xc3_write::Offset<'_, u32, String>) {
        self.value_to_offsets
            .entry(Unk4Data::Key(offset.data.clone()))
            .or_default()
            .push(offset.position);
    }

    fn insert_value(&mut self, offset: &xc3_write::Offset<'_, u32, Unk4Unk5Value>) {
        self.value_to_offsets
            .entry(Unk4Data::Value(offset.data.clone()))
            .or_default()
            .push(offset.position);
    }

    fn write<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
        endian: xc3_write::Endian,
    ) -> xc3_write::Xc3Result<()> {
        // Write all the keys and values.
        let mut value_to_position = HashMap::new();
        writer.seek(std::io::SeekFrom::Start(*data_ptr))?;

        for (value, _) in &self.value_to_offsets {
            let offset = writer.stream_position()?;
            value.xc3_write(writer, endian)?;
            value_to_position.insert(value, offset);
        }
        *data_ptr = (*data_ptr).max(writer.stream_position()?);

        // Update offsets.
        for (value, offsets) in &self.value_to_offsets {
            for offset in offsets {
                let position = value_to_position[value];
                let final_offset = position - base_offset;
                // Assume all pointers are 4 bytes.
                writer.seek(std::io::SeekFrom::Start(*offset))?;
                (final_offset as u32).xc3_write(writer, endian)?;
            }
        }

        Ok(())
    }
}

impl Xc3WriteOffsets for DhalOffsets<'_> {
    type Args = ();

    fn write_offsets<W: std::io::prelude::Write + std::io::prelude::Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
        endian: xc3_write::Endian,
        _args: Self::Args,
    ) -> xc3_write::Xc3Result<()> {
        // Different order than field order.
        // Different versions have different layouts.
        self.unk1
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.unk3
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        if *self.version.data <= 10001 {
            self.unk7
                .write_full(writer, base_offset, data_ptr, endian, ())?;
        }
        self.unk4
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.unk9
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.unk5
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.unk6
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        if *self.version.data > 10001 {
            self.unk7
                .write_full(writer, base_offset, data_ptr, endian, ())?;
        }
        self.unk8
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.unk2
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.textures
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.uncompressed_textures
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        Ok(())
    }
}

impl Xc3WriteOffsets for Unk4Offsets<'_> {
    type Args = ();

    fn write_offsets<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        _base_offset: u64,
        data_ptr: &mut u64,
        endian: xc3_write::Endian,
        _args: Self::Args,
    ) -> xc3_write::Xc3Result<()> {
        // Different order than field order.
        let base_offset = self.base_offset;

        self.unk2
            .write_full(writer, base_offset, data_ptr, endian, ())?;

        // TODO: Figure out the fields stored in this buffer.
        writer.write_all(self.buffer.data)?;
        *data_ptr = (*data_ptr).max(writer.stream_position()?);

        self.extra
            .write_offsets(writer, base_offset, data_ptr, endian, ())?;
        self.unk6
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.unk4
            .write_full(writer, base_offset, data_ptr, endian, ())?;

        // Only unique keys and values are stored in this section.
        let mut value_section = Unk4KeyValueSection::default();
        if let Some(unk5) = self.unk5.write(writer, base_offset, data_ptr, endian)? {
            for offsets in unk5.0 {
                value_section.insert_key(&offsets.key);
                value_section.insert_value(&offsets.value);
            }
        }
        value_section.write(writer, base_offset, data_ptr, endian)?;

        Ok(())
    }
}

fn unk2_buffer_size(unk1: &[Unk2Unk1], unk2: &[Unk2Unk2]) -> usize {
    // Assume data starts from 0.
    // TODO: extra padding bytes?
    // TODO: Some items overlap?
    let unk1_size = unk1.iter().map(|u| u.count as usize * 12).sum::<usize>();
    let unk2_size = unk2
        .iter()
        .map(|u| u.data_offset as usize + u.count as usize * 2)
        .max()
        .unwrap_or_default();
    unk1_size.max(unk2_size)
}

fn unk4_buffer_offset(unk2: &[Unk4Unk2]) -> u64 {
    // TODO: These items have offsets into the buffer relative to base struct start?
    unk2.iter()
        .flat_map(|u| {
            [
                u.unk2, u.unk3, u.unk4, u.unk5, u.unk6, u.unk7, u.unk8, u.unk9, u.unk10,
            ]
        })
        .filter(|u| *u != 0)
        .min()
        .unwrap_or_default() as u64
}

fn unk4_buffer_size(unk2: &[Unk4Unk2]) -> usize {
    // Add what appears to be a count for byte indices.
    let max_offset = unk2
        .iter()
        .map(|u| {
            u.unk1
                + [
                    u.unk2, u.unk3, u.unk4, u.unk5, u.unk6, u.unk7, u.unk8, u.unk9, u.unk10,
                ]
                .into_iter()
                .max()
                .unwrap_or_default()
        })
        .max()
        .unwrap_or_default();

    // Assume data starts from offset 0.
    let base_offset = unk4_buffer_offset(unk2);
    (max_offset as usize).saturating_sub(base_offset as usize)
}
