use binrw::{args, binread, FilePtr32, NullString};

#[binread]
#[derive(Debug)]
#[br(magic(b"1RAS"))]
pub struct Sar1 {
    file_size: u32,
    version: u32,

    #[br(temp)]
    count: u32,

    #[br(parse_with = FilePtr32::parse)]
    #[br(args { inner: args!(count: count as usize) })]
    entries: Vec<Entry>,

    unk_offset: u32, // pointer to start of data?

    unk4: u32,
    unk5: u32,

    #[br(map = |x: NullString| x.to_string(), pad_size_to = 128)]
    name: String,
}

#[binread]
#[derive(Debug)]
pub struct Entry {
    #[br(parse_with = FilePtr32::parse)]
    bc: Bc,
    data_size: u32,
    name_hash: u32, // TODO: CRC32C?
    #[br(map = |x: NullString| x.to_string(), pad_size_to = 52)]
    name: String,
    // TODO: padding after last element?
}

#[binread]
#[derive(Debug)]
#[br(magic(b"BC\x00\x00"))]
pub struct Bc {
    unk0: u16,
    block_count: u16,
    data_offset: u32,
    unk_offset: u32,
    unk1: u64,
    unk2: u64,
    data_type: u32, // 4 anim, 7 ASMB
    #[br(args { data_type })]
    data: Data,
}

#[binread]
#[derive(Debug)]
#[br(import { data_type: u32 })]
pub enum Data {
    #[br(pre_assert(data_type == 4))]
    Anim(Anim),
    #[br(pre_assert(data_type == 7))]
    Asmb(Asmb),
}

#[binread]
#[derive(Debug)]
#[br(magic(b"ANIM"))]
pub struct Anim {}

#[binread]
#[derive(Debug)]
#[br(magic(b"ASMB"))]
pub struct Asmb {}
