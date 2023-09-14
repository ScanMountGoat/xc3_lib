use std::io::Cursor;

use crate::write::{Xc3Write, Xc3WriteFull};
use crate::{
    msmd::{Dlgt, Gibl, Nerd},
    mxmd::Mxmd,
    parse_offset_count,
    write::round_up,
};
use binrw::{BinRead, BinReaderExt, BinWrite};

/// A packed model container with entries like [Mxmd](crate::mxmd::Mxmd) or [Gibl](crate::msmd::Gibl).
#[derive(Debug, BinRead, Xc3Write)]
#[br(magic(b"DMPA"))]
#[xc3(magic(b"DMPA"))]
pub struct Apmd {
    pub version: u32,
    #[br(parse_with = parse_offset_count)]
    #[xc3(offset_count)]
    pub entries: Vec<Entry>,
    pub unk2: u32,
    pub unk3: u32,
    // TODO: padding?
    pub unk: [u32; 7],
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteFull)]
pub struct Entry {
    pub entry_type: EntryType,
    #[br(parse_with = parse_offset_count)]
    #[xc3(offset_count, align(4096))]
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
    pub fn read_data(&self) -> EntryData {
        // TODO: Avoid unwrap.
        let mut reader = Cursor::new(&self.entry_data);
        match self.entry_type {
            EntryType::Mxmd => EntryData::Mxmd(reader.read_le().unwrap()),
            EntryType::Dmis => EntryData::Dmis,
            EntryType::Dlgt => EntryData::Dlgt(reader.read_le().unwrap()),
            EntryType::Gibl => EntryData::Gibl(reader.read_le().unwrap()),
            EntryType::Nerd => EntryData::Nerd(reader.read_le().unwrap()),
            EntryType::Dlgt2 => EntryData::Dlgt2(reader.read_le().unwrap()),
        }
    }
}

impl<'a> Xc3WriteFull for ApmdOffsets<'a> {
    fn write_full<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
    ) -> binrw::BinResult<()> {
        self.entries
            .write_offset_full(writer, base_offset, data_ptr)?;
        // TODO: Handle this in the derive?
        let length = writer.stream_position()?;
        let padding = round_up(length, 4096) - length;
        writer.write_all(&vec![0u8; padding as usize])?;
        Ok(())
    }
}
