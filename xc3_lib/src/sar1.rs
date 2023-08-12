use std::io::Cursor;

use crate::{bc::Bc, parse_count_offset, parse_offset_count};
use binrw::{binread, BinRead, NullString, BinResult, BinReaderExt};

// .chr files have skeletons?
// .mot files have animations?
#[binread]
#[derive(Debug)]
#[br(magic(b"1RAS"))]
pub struct Sar1 {
    pub file_size: u32,
    pub version: u32,

    #[br(parse_with = parse_count_offset)]
    pub entries: Vec<Entry>,

    pub unk_offset: u32, // pointer to start of data?

    pub unk4: u32,
    pub unk5: u32,

    #[br(map = |x: NullString| x.to_string(), pad_size_to = 128)]
    pub name: String,
}

// TODO: Parse this in a separate step?
// This would simplify base offsets for BC data.
#[binread]
#[derive(Debug)]
pub struct Entry {
    #[br(parse_with = parse_offset_count)]
    pub entry_data: Vec<u8>,

    // TODO: CRC32C?
    // https://github.com/PredatorCZ/XenoLib/blob/master/source/sar.cpp
    pub name_hash: u32,

    #[br(map = |x: NullString| x.to_string(), pad_size_to = 52)]
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
