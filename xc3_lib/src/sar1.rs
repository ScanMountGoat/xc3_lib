//! Simple archive data in `.arc`, `.chr`, or `.mot` files.
//!
//! XC3: `chr/{ch,en,oj,wp}/*.{chr,mot}`
use std::io::Cursor;

use crate::{bc::Bc, eva::Eva, parse_count32_offset32, parse_offset32_count32, parse_ptr32};
use binrw::{binread, BinRead, BinReaderExt, BinResult, NullString};
use xc3_write::{Xc3Write, Xc3WriteOffsets};

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
#[br(magic(b"1RAS"))]
#[xc3(magic(b"1RAS"))]
#[xc3(align_after(2048))]
pub struct Sar1 {
    // TODO: calculate this when writing.
    pub file_size: u32,
    pub version: u32,

    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count32_offset32)]
    pub entries: Vec<Entry>,

    pub unk_offset: u32, // pointer to start of data?

    pub unk4: u32,
    pub unk5: u32,

    #[br(map = |x: NullString| x.to_string(), pad_size_to = 128)]
    #[xc3(pad_size_to(128))]
    pub name: String,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct Entry {
    #[br(parse_with = parse_offset32_count32)]
    #[xc3(offset32_count32, align(64))]
    pub entry_data: Vec<u8>,

    // TODO: CRC32C?
    // https://github.com/PredatorCZ/XenoLib/blob/master/source/sar.cpp
    pub name_hash: u32,

    #[br(map = |x: NullString| x.to_string(), pad_size_to = 52)]
    #[xc3(pad_size_to(52))]
    pub name: String,
}

// TODO: Is there a better way of expressing this?
impl Entry {
    pub fn read_data(&self) -> BinResult<EntryData> {
        Cursor::new(&self.entry_data).read_le()
    }
}

#[derive(Debug, BinRead)]
pub enum EntryData {
    Bc(Bc),
    ChCl(ChCl),
    Csvb(Csvb),
    Eva(Eva),
}

// character collision?
#[derive(Debug, BinRead)]
#[br(magic(b"CHCL"))]
pub struct ChCl {
    pub unk1: u32,
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
