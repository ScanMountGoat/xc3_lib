use std::io::SeekFrom;

use crate::{parse_count_offset, parse_ptr32, parse_string_ptr32};
use binrw::{binread, BinRead, NamedArgs, NullString};
use serde::Serialize;

// .chr files have skeletons?
// .mot files have animations?
#[binread]
#[derive(Debug, Serialize)]
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

#[binread]
#[derive(Debug, Serialize)]
pub struct Entry {
    #[br(parse_with = parse_ptr32)]
    pub data: EntryData,
    pub data_size: u32,

    // TODO: CRC32C?
    // https://github.com/PredatorCZ/XenoLib/blob/master/source/sar.cpp
    pub name_hash: u32, 

    #[br(map = |x: NullString| x.to_string(), pad_size_to = 52)]
    pub name: String,
}

#[binread]
#[derive(Debug, Serialize)]
pub enum EntryData {
    Bc(Bc),
    ChCl(ChCl),
    Csvb(Csvb),
    Eva(Eva)
}

#[binread]
#[derive(Debug, Serialize)]
#[br(magic(b"BC"))]
#[br(stream = r)]
pub struct Bc {
    // Subtract the magic size.
    #[br(temp, try_calc = r.stream_position().map(|p| p - 2))]
    base_offset: u64,

    pub unk_flags: u16,

    pub unk1: u32,
    pub data_size: u32,
    pub unk_count: u32,
    pub data_offset: u64, // TODO: offset for bcdata?
    pub unk_offset: u64, // TODO: offset to u64s?

    #[br(args { base_offset })]
    pub data: BcData,
}

#[derive(BinRead, Debug, Serialize)]
#[br(import { base_offset: u64 })]
pub enum BcData {
    #[br(magic(2u32))]
    Skdy(Skdy),

    #[br(magic(4u32))]
    Anim(Anim),

    #[br(magic(6u32))]
    Skel(#[br(args { base_offset })] Skel),

    #[br(magic(7u32))]
    Asmb(Asmb),
}

// skeleton dynamics?
#[derive(BinRead, Debug, Serialize)]
#[br(magic(b"SKDY"))]
pub struct Skdy {
    pub unk1: u32,
}

// animation?
#[derive(BinRead, Debug, Serialize)]
#[br(magic(b"ANIM"))]
pub struct Anim {
    pub unk1: u32,
}

#[derive(BinRead, Debug, Serialize)]
#[br(magic(b"SKEL"))]
#[br(import { base_offset: u64 })]
pub struct Skel {
    pub unks: [u32; 10],

    #[br(args { base_offset })]
    pub parents: SkelData<i16>,

    #[br(args { base_offset, inner: base_offset })]
    pub names: SkelData<BoneName>,

    #[br(args { base_offset })]
    pub transforms: SkelData<Transform>,

    // TODO: types?
    #[br(args { base_offset })]
    pub unk_table1: SkelData<u8>,
    #[br(args { base_offset })]
    pub unk_table2: SkelData<u8>,
    #[br(args { base_offset })]
    pub unk_table3: SkelData<u8>,
    #[br(args { base_offset })]
    pub unk_table4: SkelData<u8>,
    #[br(args { base_offset })]
    pub unk_table5: SkelData<u8>,
    // TODO: other fields?
}

#[derive(BinRead, Debug, Serialize)]
#[br(magic(b"eva\x00"))]
pub struct Eva {
    pub unk1: u32,
}

#[derive(BinRead, Debug, Serialize)]
#[br(import_raw(args: SkelDataArgs<T::Args<'_>>))]
pub struct SkelData<T>
where
    T: BinRead + 'static,
    for<'a> T::Args<'a>: Clone + Default,
{
    // TODO: Use parse_with for this?
    #[br(args { base_offset: args.base_offset, inner: args.inner })]
    pub items: Container<T>,
    pub unk1: i32,
}

#[derive(Clone, NamedArgs)]
pub struct SkelDataArgs<Inner: Default> {
    #[named_args(default = 0)]
    base_offset: u64,
    #[named_args(default = Inner::default())]
    inner: Inner,
}

#[derive(BinRead, Debug, Serialize)]
pub struct Transform {
    pub position: [f32; 4],
    pub rotation_quaternion: [f32; 4],
    pub scale: [f32; 4],
}

#[derive(BinRead, Debug, Serialize)]
#[br(import_raw(base_offset: u64))]
pub struct BoneName {
    #[br(parse_with = parse_string_ptr32, offset = base_offset)]
    #[br(pad_after = 12)]
    pub name: String,
}

#[derive(BinRead, Debug, Serialize)]
#[br(magic(b"ASMB"))]
pub struct Asmb {
    pub unk1: u32,
}

// character collision?
#[derive(BinRead, Debug, Serialize)]
#[br(magic(b"CHCL"))]
pub struct ChCl {
    pub unk1: u32,
}

// "effpnt" or "effect" "point"?
#[derive(BinRead, Debug, Serialize)]
#[br(magic(b"CSVB"))]
pub struct Csvb {
    pub unk1: u32,
}

// TODO: Shared with mxmd just with a different pointer type.
/// A [u64] offset and [u32] count with an optional base offset.
#[derive(Clone, NamedArgs)]
pub struct ContainerArgs<Inner: Default> {
    #[named_args(default = 0)]
    base_offset: u64,
    #[named_args(default = Inner::default())]
    inner: Inner,
}

#[binread]
#[derive(Debug, Serialize)]
#[br(import_raw(args: ContainerArgs<T::Args<'_>>))]
#[serde(transparent)]
pub struct Container<T>
where
    T: BinRead + 'static,
    for<'a> <T as BinRead>::Args<'a>: Clone + Default,
{
    #[br(temp)]
    offset: u64,
    #[br(temp)]
    count: u32,

    #[br(args { count: count as usize, inner: args.inner })]
    #[br(seek_before = SeekFrom::Start(args.base_offset + offset as u64))]
    #[br(restore_position)]
    pub elements: Vec<T>,
}
