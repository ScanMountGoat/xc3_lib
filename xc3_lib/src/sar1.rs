//! Simple archive data in `.arc`, `.chr`, or `.mot` files.
//!
//! # File Paths
//! Xenoblade 1 `.mot` [Sar1] are in [Xbc1](crate::xbc1::Xbc1) archives.
//!
//! | Game | File Patterns |
//! | --- | --- |
//! | Xenoblade Chronicles 1 DE | `chr/{en,np,obj,pc,wp}/*.{chr,mot}` |
//! | Xenoblade Chronicles 2 | `event/mot/{bl,en,np,oj,pc,we,wp}/*.mot`, `model/{bl,en,np,oj,pc,we,wp}/*.{arc,mot}` |
//! | Xenoblade Chronicles 3 | `chr/{bt,ch,en,oj,wp}/*.{chr,mot}` |
use std::io::Cursor;

use crate::{
    hash::hash_str_crc, parse_count32_offset32, parse_offset32_count32, parse_ptr32,
    parse_string_ptr32,
};
use binrw::{binread, BinRead, BinReaderExt, BinResult, NullString};
use xc3_write::{write_full, Xc3Write, Xc3WriteOffsets};

/// A simple archive containing named entries.
#[derive(Debug, BinRead, Xc3Write)]
#[br(magic(b"1RAS"))]
#[xc3(magic(b"1RAS"))]
pub struct Sar1 {
    #[xc3(shared_offset)]
    pub file_size: u32,

    pub version: u32,

    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub entries: Vec<Entry>,

    #[xc3(shared_offset, align(64))]
    pub data_offset: u32,

    pub unk4: u32, // 0?
    pub unk5: u32, // 0?

    #[br(map = |x: NullString| x.to_string(), pad_size_to = 128)]
    #[xc3(pad_size_to(128))]
    pub name: String,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct Entry {
    #[br(parse_with = parse_offset32_count32)]
    #[xc3(offset_count(u32, u32), align(64))]
    pub entry_data: Vec<u8>,

    /// Hash of [name](#structfield.name) using [hash_str_crc].
    pub name_hash: u32,

    #[br(map = |x: NullString| x.to_string(), pad_size_to = 52)]
    #[xc3(pad_size_to(52))]
    pub name: String,
}

impl Entry {
    /// Write the bytes from `data` to a new [Entry].
    pub fn new<T>(name: String, data: &T) -> xc3_write::Xc3Result<Self>
    where
        T: Xc3Write + 'static,
        for<'a> T::Offsets<'a>: Xc3WriteOffsets,
    {
        let mut writer = Cursor::new(Vec::new());
        write_full(data, &mut writer, 0, &mut 0)?;

        Ok(Self::from_entry_data(name, writer.into_inner()))
    }

    /// Create a new [Entry] from `entry_data`.
    pub fn from_entry_data(name: String, entry_data: Vec<u8>) -> Self {
        Self {
            entry_data,
            name_hash: hash_str_crc(&name),
            name,
        }
    }

    // TODO: table of type and names
    /// Attempt to read an item from the bytes for this entry.
    pub fn read_data<T>(&self) -> BinResult<T>
    where
        for<'a> T: BinRead<Args<'a> = ()>,
    {
        Cursor::new(&self.entry_data).read_le()
    }
}

// character collision?
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
#[br(magic(b"CHCL"))]
#[xc3(magic(b"CHCL"))]
#[xc3(align_after(64))]
pub struct ChCl {
    pub version: u32,
    pub unk1: u32,

    #[br(parse_with = parse_ptr32)]
    #[xc3(offset(u32))]
    pub inner: ChClInner,

    // TODO: padding?
    pub unks: [u32; 10],
}

#[derive(Debug, BinRead, Xc3Write)]
pub struct ChClInner {
    #[br(parse_with = parse_offset32_count32)]
    #[xc3(offset_count(u32, u32))]
    pub unk1: Vec<[f32; 26]>,

    #[br(parse_with = parse_offset32_count32)]
    #[xc3(offset_count(u32, u32))]
    pub unk2: Vec<ChClUnk2>,

    #[br(parse_with = parse_offset32_count32)]
    #[xc3(offset_count(u32, u32), align(8))]
    pub unk3: Vec<u16>,

    #[br(parse_with = parse_offset32_count32)]
    #[xc3(offset_count(u32, u32), align(2))]
    pub unk4: Vec<u16>,

    #[br(parse_with = parse_offset32_count32)]
    #[xc3(offset_count(u32, u32), align(2))]
    pub unk5: Vec<u16>,

    #[br(parse_with = parse_offset32_count32)]
    #[xc3(offset_count(u32, u32), align(2))]
    pub unk6: Vec<u16>,

    #[br(parse_with = parse_offset32_count32)]
    #[xc3(offset_count(u32, u32))]
    pub unk7: Vec<ChClUnk7>,

    // TODO: padding?
    pub unks: [u32; 4],
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct ChClUnk2 {
    pub unk1: [[f32; 4]; 4],

    // TODO: bone names?
    #[br(parse_with = parse_string_ptr32)]
    #[xc3(offset(u32))]
    pub name: String,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct ChClUnk7 {
    pub unk1: [[f32; 4]; 3],
    // TODO: Pointer to Idcm?
}

#[binread]
#[derive(Debug, Xc3Write, Xc3WriteOffsets)]
#[br(stream = r)]
#[br(magic(b"IDCM"))]
#[xc3(base_offset)]
#[xc3(magic(b"IDCM"))]
pub struct Idcm {
    // Subtract the magic size.
    #[br(temp, try_calc = r.stream_position().map(|p| p - 4))]
    base_offset: u64,

    pub version: u32,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub unk1: Vec<[u32; 19]>,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub unk2: Vec<[u32; 3]>,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub unk3: Vec<u64>,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub unk4: Vec<[u32; 17]>,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub unk5: Vec<u32>,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub unk6: Vec<u32>,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub unk7: Vec<[u32; 4]>,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub unk8: Vec<[f32; 4]>,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub unk9: Vec<u32>,

    pub unk10: u64,
    // TODO: more fields
}

// TODO: Is the padding always aligned?
// "effpnt" or "effect" "point"?
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
#[br(magic(b"CSVB"))]
#[xc3(magic(b"CSVB"))]
#[xc3(align_after(64))]
pub struct Csvb {
    pub item_count: u16,
    pub unk_count: u16,
    pub unk_section_length: u32,
    pub string_section_length: u32,

    // TODO: Why do we need to divide here?
    #[br(count = unk_count / 8)]
    pub unks: Vec<u16>,

    #[br(count = item_count)]
    pub unk6: Vec<CvsbItem>,

    #[br(count = unk_section_length)]
    pub unk_section: Vec<u8>,

    #[br(count = string_section_length)]
    pub string_section: Vec<u8>,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct CvsbItem {
    // TODO: Offsets relative to start of string section.
    pub name1_offset: u16,
    pub name2_offset: u16,
    pub unk3: u32,
    pub unk4: u32,
    pub unk5: u32,
}

impl<'a> Xc3WriteOffsets for ChClInnerOffsets<'a> {
    fn write_offsets<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
    ) -> xc3_write::Xc3Result<()> {
        // Different order than field order.
        self.unk1.write_full(writer, base_offset, data_ptr)?;
        let unk2 = self.unk2.write_offset(writer, base_offset, data_ptr)?;
        if !self.unk7.data.is_empty() {
            self.unk7.write_full(writer, base_offset, data_ptr)?;
        }
        self.unk3.write_full(writer, base_offset, data_ptr)?;
        if !self.unk4.data.is_empty() {
            self.unk4.write_full(writer, base_offset, data_ptr)?;
        }
        self.unk5.write_full(writer, base_offset, data_ptr)?;
        self.unk6.write_full(writer, base_offset, data_ptr)?;

        // Strings appear at the end of the file.
        *data_ptr = data_ptr.next_multiple_of(4);
        for item in unk2.0 {
            item.name.write_full(writer, base_offset, data_ptr)?;
        }

        Ok(())
    }
}

impl<'a> Xc3WriteOffsets for Sar1Offsets<'a> {
    fn write_offsets<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
    ) -> xc3_write::Xc3Result<()> {
        // Make sure the data offset points to the first entry data.
        let entries = self.entries.write_offset(writer, base_offset, data_ptr)?;
        self.data_offset.write_full(writer, base_offset, data_ptr)?;
        for entry in entries.0 {
            entry.write_offsets(writer, base_offset, data_ptr)?;
        }

        // Align the file size to 2048.
        let padding = data_ptr.next_multiple_of(2048) - *data_ptr;
        vec![0u8; padding as usize].xc3_write(writer, data_ptr)?;
        self.file_size.write_full(writer, base_offset, data_ptr)?;

        Ok(())
    }
}
