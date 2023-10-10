//! Simple archive data in `.chr` or `.mot` files.
//!
//! XC3: `chr/{ch,en,oj,wp}/*.{chr,mot}`
use std::io::Cursor;

use crate::{bc::Bc, parse_count32_offset32, parse_offset32_count32};
use binrw::{binread, BinRead, BinReaderExt, BinResult, NullString};
use xc3_write::{Xc3Write, Xc3WriteOffsets};

#[binread]
#[derive(Debug, Xc3Write, Xc3WriteOffsets)]
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

#[binread]
#[derive(Debug, Xc3Write, Xc3WriteOffsets)]
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

#[binread]
#[derive(Debug)]
pub enum EntryData {
    Bc(Bc),
    ChCl(ChCl),
    Csvb(Csvb),
    Eva(Eva),
}

#[derive(BinRead, Debug)]
#[br(magic(b"eva\x00"))]
pub struct Eva {
    pub unk1: u32,
}

// character collision?
#[derive(BinRead, Debug)]
#[br(magic(b"CHCL"))]
pub struct ChCl {
    pub unk1: u32,
}

// "effpnt" or "effect" "point"?
#[derive(BinRead, Debug)]
#[br(magic(b"CSVB"))]
pub struct Csvb {
    pub unk1: u32,
}
