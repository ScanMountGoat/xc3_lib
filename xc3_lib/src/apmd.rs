//! Model archive for character and map models in `.wimdo` files.
use std::io::Cursor;

use crate::{
    msmd::{Dlgt, Gibl, Nerd},
    mxmd::Mxmd,
    parse_offset32_count32,
};
use binrw::{BinRead, BinReaderExt, BinResult, BinWrite};
use xc3_write::{xc3_write_binwrite_impl, Xc3Write, Xc3WriteOffsets};

/// A packed model container with entries like [Mxmd] or [Gibl].
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
#[br(magic(b"DMPA"))]
#[xc3(magic(b"DMPA"))]
#[xc3(align_after(4096))]
pub struct Apmd {
    pub version: u32,
    #[br(parse_with = parse_offset32_count32)]
    #[xc3(offset_count(u32, u32))]
    pub entries: Vec<Entry>,
    pub unk2: u32,
    pub unk3: u32,
    // TODO: padding?
    pub unk: [u32; 7],
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct Entry {
    pub entry_type: EntryType,
    #[br(parse_with = parse_offset32_count32)]
    #[xc3(offset_count(u32, u32), align(4096))]
    pub entry_data: Vec<u8>,
}

#[derive(Debug, BinRead, BinWrite)]
#[brw(repr(u32))]
pub enum EntryType {
    Mxmd = 0,
    Dmis = 1,
    Dlgt = 3,
    Gibl = 4,
    Nerd = 5,
    Dlgt2 = 6,
}

#[derive(Debug)]
pub enum EntryData {
    Mxmd(Mxmd),
    Dmis,
    Dlgt(Dlgt),
    Gibl(Gibl),
    Nerd(Nerd),
    Dlgt2(Dlgt),
}

impl Entry {
    pub fn read_data(&self) -> BinResult<EntryData> {
        let mut reader = Cursor::new(&self.entry_data);
        match self.entry_type {
            EntryType::Mxmd => Ok(EntryData::Mxmd(reader.read_le()?)),
            EntryType::Dmis => Ok(EntryData::Dmis),
            EntryType::Dlgt => Ok(EntryData::Dlgt(reader.read_le()?)),
            EntryType::Gibl => Ok(EntryData::Gibl(reader.read_le()?)),
            EntryType::Nerd => Ok(EntryData::Nerd(reader.read_le()?)),
            EntryType::Dlgt2 => Ok(EntryData::Dlgt2(reader.read_le()?)),
        }
    }
}

xc3_write_binwrite_impl!(EntryType);
