use crate::parse_offset_count;
use binrw::BinRead;

/// `chr/oj/oj03010100.wimdo` or `map/*.wimdo` for Xenoblade 3.
#[derive(BinRead, Debug)]
#[br(magic(b"DMPA"))]
pub struct Apmd {
    pub version: u32,
    #[br(parse_with = parse_offset_count)]
    pub entries: Vec<Entry>,
    pub unk2: u32,
    pub unk3: u32,
    // TODO: padding?
    pub unk: [u32; 8],
}

#[derive(BinRead, Debug)]
pub struct Entry {
    pub entry_type: EntryType,
    pub offset: u32,
    pub length: u32,
}

#[derive(BinRead, Debug)]
#[br(repr(u32))]
pub enum EntryType {
    Mxmd = 0,
    Dmis = 1,
    Dlgt = 3,
    Gibl = 4,
    Nerd = 5,
    Dlgt2 = 6,
}
