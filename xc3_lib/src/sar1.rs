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
    hash::hash_str_crc, idcm::Idcm, parse_count32_offset32, parse_offset32_count32,
    parse_opt_ptr32, parse_ptr32, parse_string_ptr32,
};
use binrw::{binread, BinRead, BinReaderExt, BinResult, NullString};
use xc3_write::{write_full, Xc3Write, Xc3WriteOffsets};

/// A simple archive containing named entries.
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, PartialEq, Clone)]
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

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
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
        for<'a> T::Offsets<'a>: Xc3WriteOffsets<Args = ()>,
    {
        let mut writer = Cursor::new(Vec::new());
        write_full(data, &mut writer, 0, &mut 0, xc3_write::Endian::Little, ())?;

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
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(magic(b"CHCL"))]
#[xc3(magic(b"CHCL"))]
#[xc3(align_after(64))]
pub struct ChCl {
    pub version: u32, // 10002
    pub unk1: u32,

    #[br(parse_with = parse_ptr32)]
    #[xc3(offset(u32))]
    pub inner: ChClInner,

    // TODO: padding?
    pub unks: [u32; 10],
}

#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, PartialEq, Clone)]
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

    // TODO: Find a nicer way to express this.
    #[br(temp, restore_position)]
    unk7_offset_count: [u32; 2],

    #[br(parse_with = parse_opt_ptr32)]
    #[br(args { inner: unk7_offset_count[1] as usize })]
    #[xc3(offset(u32))]
    pub unk7: Option<ChClUnk7>,

    // TODO: add offset_inner_count to xc3?
    #[xc3(shared_offset)]
    pub unk7_count: u32,

    // TODO: padding?
    pub unks: [u32; 4],
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct ChClUnk2 {
    pub unk1: [[f32; 4]; 4],

    // TODO: bone names?
    #[br(parse_with = parse_string_ptr32)]
    #[xc3(offset(u32))]
    pub name: String,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(count: usize))]
pub struct ChClUnk7 {
    #[br(count = count)]
    pub unk1: Vec<[[f32; 4]; 3]>,
    #[br(count = count)]
    pub unk2: Vec<ChClUnk7Item>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct ChClUnk7Item {
    pub unk1: f32,
    #[br(parse_with = parse_ptr32)]
    #[xc3(offset(u32))]
    pub idcm: Idcm,

    // TODO: padding?
    pub unk: [u32; 3],
}

// TODO: Is the padding always aligned?
// "effpnt" or "effect" "point"?
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(magic(b"CSVB"))]
#[xc3(magic(b"CSVB"))]
#[xc3(align_after(64))]
pub struct Csvb {
    pub item_count: u16,
    pub unk_count: u16,
    pub unk_section_length: u32,
    pub string_section_length: u32,

    // TODO: Why do we need to divide here?
    #[br(count = unk_count as usize / 8)]
    pub unks: Vec<u16>,

    #[br(count = item_count as usize)]
    pub unk6: Vec<CvsbItem>,

    #[br(count = unk_section_length as usize)]
    pub unk_section: Vec<u8>,

    #[br(count = string_section_length as usize)]
    pub string_section: Vec<u8>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct CvsbItem {
    // TODO: Offsets relative to start of string section.
    pub name1_offset: u16,
    pub name2_offset: u16,
    pub unk3: u32,
    pub unk4: u32,
    pub unk5: u32,
}

impl<'a> Xc3WriteOffsets for ChClInnerOffsets<'a> {
    type Args = ();

    fn write_offsets<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
        endian: xc3_write::Endian,
        _args: Self::Args,
    ) -> xc3_write::Xc3Result<()> {
        // Different order than field order.
        self.unk1
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        let unk2 = self.unk2.write(writer, base_offset, data_ptr, endian)?;
        let unk7 = self.unk7.write(writer, base_offset, data_ptr, endian)?;
        self.unk3
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        if !self.unk4.data.is_empty() {
            self.unk4
                .write_full(writer, base_offset, data_ptr, endian, ())?;
        }
        self.unk5
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.unk6
            .write_full(writer, base_offset, data_ptr, endian, ())?;

        // Strings appear at the end of the file.
        *data_ptr = data_ptr.next_multiple_of(4);
        for item in unk2.0 {
            item.name
                .write_full(writer, base_offset, data_ptr, endian, ())?;
        }

        unk7.write_offsets(writer, base_offset, data_ptr, endian, ())?;

        // Assume both lists have the same length.
        // TODO: Find a nicer way of expressing this.
        self.unk7_count.set_offset(
            writer,
            self.unk7
                .data
                .as_ref()
                .map(|d| d.unk1.len())
                .unwrap_or_default() as u64,
            endian,
        )?;

        Ok(())
    }
}

impl<'a> Xc3WriteOffsets for Sar1Offsets<'a> {
    type Args = ();

    fn write_offsets<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
        endian: xc3_write::Endian,
        _args: Self::Args,
    ) -> xc3_write::Xc3Result<()> {
        // Make sure the data offset points to the first entry data.
        let entries = self.entries.write(writer, base_offset, data_ptr, endian)?;
        self.data_offset
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        for entry in entries.0 {
            entry.write_offsets(writer, base_offset, data_ptr, endian, ())?;
        }

        // Align the file size to 2048.
        let padding = data_ptr.next_multiple_of(2048) - *data_ptr;
        vec![0u8; padding as usize].xc3_write(writer, endian)?;
        *data_ptr = (*data_ptr).max(writer.stream_position()?);

        self.file_size.set_offset(writer, *data_ptr, endian)?;

        Ok(())
    }
}
