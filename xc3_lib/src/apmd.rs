//! Model archive for character and map models in `.wimdo` files.
use std::io::Cursor;

use crate::{
    msmd::{Dlgt, Gibl, Nerd},
    mxmd::Mxmd,
    parse_offset32_count32, xc3_write_binwrite_impl,
};
use binrw::{BinRead, BinReaderExt, BinResult, BinWrite};
use xc3_write::{Xc3Write, Xc3WriteOffsets};

/// A packed model container with entries like [Mxmd] or [Gibl].
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(magic(b"DMPA"))]
#[xc3(magic(b"DMPA"))]
#[xc3(align_after(4096))]
pub struct Apmd {
    pub version: u32,
    #[br(parse_with = parse_offset32_count32)]
    #[xc3(offset_count(u32, u32))]
    pub entries: Vec<Entry>,
    pub unk2: u32, // entry count - 3?
    pub unk3: u32, // 0 or 1?
    // TODO: variable padding?
    pub unk: [u32; 8],
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct Entry {
    pub entry_type: EntryType,
    #[br(parse_with = parse_offset32_count32)]
    #[xc3(offset_count(u32, u32), align(4096))]
    pub entry_data: Vec<u8>,
}

#[derive(Debug, BinRead, BinWrite, PartialEq, Eq, Clone, Copy)]
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
    pub fn from_entry_data(data: EntryData) -> xc3_write::Xc3Result<Self> {
        // TODO: Create a to_bytes method?
        // TODO: Finish write support and test in xc3_test?
        let mut writer = Cursor::new(Vec::new());
        let entry_type = match data {
            EntryData::Mxmd(v) => {
                v.write(&mut writer)?;
                EntryType::Mxmd
            }
            EntryData::Dmis => EntryType::Dmis,
            EntryData::Dlgt(_) => EntryType::Dlgt,
            EntryData::Gibl(_) => EntryType::Gibl,
            EntryData::Nerd(_) => EntryType::Nerd,
            EntryData::Dlgt2(_) => EntryType::Dlgt2,
        };

        Ok(Self {
            entry_type,
            entry_data: writer.into_inner(),
        })
    }

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
