use std::io::SeekFrom;

use crate::{parse_count_offset, parse_string_ptr32};
use binrw::{binread, BinRead, FilePtr32, NamedArgs, NullString};
use serde::Serialize;

// .chr files have skeletons?
// .mot files have animations?
#[binread]
#[derive(Debug, Serialize)]
#[br(magic(b"1RAS"))]
pub struct Sar1 {
    file_size: u32,
    version: u32,

    #[br(parse_with = parse_count_offset)]
    entries: Vec<Entry>,

    unk_offset: u32, // pointer to start of data?

    unk4: u32,
    unk5: u32,

    #[br(map = |x: NullString| x.to_string(), pad_size_to = 128)]
    name: String,
}

#[binread]
#[derive(Debug, Serialize)]
pub struct Entry {
    #[br(parse_with = FilePtr32::parse)]
    data: EntryData,
    data_size: u32,
    name_hash: u32, // TODO: CRC32C?
    #[br(map = |x: NullString| x.to_string(), pad_size_to = 52)]
    name: String,
    // TODO: padding after last element?
}

#[binread]
#[derive(Debug, Serialize)]
pub enum EntryData {
    Bc(Bc),
    ChCl(ChCl),
    Csvb(Csvb),
}

#[binread]
#[derive(Debug, Serialize)]
#[br(magic(b"BC\x00\x00"))]
#[br(stream = r)]
pub struct Bc {
    // Subtract the magic size.
    #[br(temp, try_calc = r.stream_position().map(|p| p - 4))]
    base_offset: u64,

    unk0: u16,
    block_count: u16,
    data_offset: u32,
    unk_offset: u32,
    unk1: u64,
    unk2: u64,

    #[br(args { base_offset })]
    data: BcData,
}

#[binread]
#[derive(Debug, Serialize)]
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
#[binread]
#[derive(Debug, Serialize)]
#[br(magic(b"SKDY"))]
pub struct Skdy {
    unk1: u32,
}

// animation?
#[binread]
#[derive(Debug, Serialize)]
#[br(magic(b"ANIM"))]
pub struct Anim {
    unk1: u32,
}

// TODO: Is there a cleaner way to handle base offsets?
// This pattern is used in a lot of files.
#[binread]
#[derive(Debug, Serialize)]
#[br(magic(b"SKEL"))]
#[br(import { base_offset: u64 })]
pub struct Skel {
    unks: [u32; 10],

    #[br(args { base_offset })]
    parents: SkelData<i16>,

    #[br(args { base_offset, inner: base_offset })]
    names: SkelData<BoneName>,

    #[br(args { base_offset })]
    transforms: SkelData<Transform>,

    // TODO: types?
    #[br(args { base_offset })]
    unk_table1: SkelData<u8>,
    #[br(args { base_offset })]
    unk_table2: SkelData<u8>,
    #[br(args { base_offset })]
    unk_table3: SkelData<u8>,
    #[br(args { base_offset })]
    unk_table4: SkelData<u8>,
    #[br(args { base_offset })]
    unk_table5: SkelData<u8>,
    // TODO: other fields?
}

#[binread]
#[derive(Debug, Serialize)]
#[br(import_raw(args: SkelDataArgs<T::Args<'_>>))]
pub struct SkelData<T>
where
    T: BinRead + 'static,
    for<'a> T::Args<'a>: Clone + Default,
{
    #[br(args { base_offset: args.base_offset, inner: args.inner })]
    items: Container<T>,
    unk1: i32,
}

#[derive(Clone, NamedArgs)]
pub struct SkelDataArgs<Inner: Default> {
    #[named_args(default = 0)]
    base_offset: u64,
    #[named_args(default = Inner::default())]
    inner: Inner,
}

#[binread]
#[derive(Debug, Serialize)]
pub struct Transform {
    position: [f32; 4],
    rotation_quaternion: [f32; 4],
    scale: [f32; 4],
}

#[binread]
#[derive(Debug, Serialize)]
#[br(import_raw(base_offset: u64))]
pub struct BoneName {
    #[br(parse_with = parse_string_ptr32, args_raw(base_offset))]
    #[br(pad_after = 12)]
    name: String,
}

#[binread]
#[derive(Debug, Serialize)]
#[br(magic(b"ASMB"))]
pub struct Asmb {
    unk1: u32,
}

// character collision?
#[binread]
#[derive(Debug, Serialize)]
#[br(magic(b"CHCL"))]
pub struct ChCl {
    unk1: u32,
}

// "effpnt" or "effect" "point"?
#[binread]
#[derive(Debug, Serialize)]
#[br(magic(b"CSVB"))]
pub struct Csvb {
    unk1: u32,
}

// TODO: Shared with mxmd just with a different pointer type.
/// A [u64] offset and [u32] count with an optional base offset.
#[derive(Clone, NamedArgs)]
struct ContainerArgs<Inner: Default> {
    #[named_args(default = 0)]
    base_offset: u64,
    #[named_args(default = Inner::default())]
    inner: Inner,
}

#[binread]
#[derive(Debug, Serialize)]
#[br(import_raw(args: ContainerArgs<T::Args<'_>>))]
#[serde(transparent)]
struct Container<T>
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
    elements: Vec<T>,
}
