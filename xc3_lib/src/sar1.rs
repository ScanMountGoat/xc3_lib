use std::io::SeekFrom;

use crate::{parse_count_offset, parse_ptr32, parse_string_ptr32};
use binrw::{binread, file_ptr::FilePtrArgs, BinRead, NullString};

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

#[binread]
#[derive(Debug)]
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
#[derive(Debug)]
pub enum EntryData {
    Bc(Bc),
    ChCl(ChCl),
    Csvb(Csvb),
    Eva(Eva),
}

#[binread]
#[derive(Debug)]
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
    pub unk_offset: u64,  // TODO: offset to u64s?

    #[br(args { base_offset })]
    pub data: BcData,
}

#[derive(BinRead, Debug)]
#[br(import { base_offset: u64 })]
pub enum BcData {
    #[br(magic(2u32))]
    Skdy(Skdy),

    #[br(magic(4u32))]
    Anim(#[br(args_raw(base_offset))] Anim),

    #[br(magic(6u32))]
    Skel(#[br(args { base_offset })] Skel),

    #[br(magic(7u32))]
    Asmb(Asmb),
}

// skeleton dynamics?
#[derive(BinRead, Debug)]
#[br(magic(b"SKDY"))]
pub struct Skdy {
    pub unk1: u32,
}

// animation?
// TODO: animation binding?
#[derive(BinRead, Debug)]
#[br(magic(b"ANIM"))]
#[br(import_raw(base_offset: u64))]
pub struct Anim {
    pub unk1: [u32; 10],
    #[br(args_raw(base_offset))]
    pub animation: Animation,
}

#[derive(BinRead, Debug)]
#[br(import_raw(base_offset: u64))]
pub struct Animation {
    pub animation_type: AnimationType,
    pub space_mode: u8,
    pub play_mode: u8,
    pub blend_mode: u8,
    pub frames_per_second: f32,
    pub seconds_per_frame: f32,
    pub frame_count: u32,
    pub unk1: u32,
    pub unk2: u32,
    pub unk3: u32,
    pub unk4: i32,
    pub unk5: u64,

    // TODO: more fields?
    #[br(args { animation_type, base_offset })]
    pub data: AnimationData,
}

#[derive(BinRead, Debug, PartialEq, Eq, Clone, Copy)]
#[br(repr(u8))]
pub enum AnimationType {
    Unk0 = 0,
    Unk1 = 1,
    Unk2 = 2,
    PackedCubic = 3,
}

#[derive(BinRead, Debug)]
#[br(import { animation_type: AnimationType, base_offset: u64 })]
pub enum AnimationData {
    #[br(pre_assert(animation_type == AnimationType::Unk0))]
    Unk0,

    #[br(pre_assert(animation_type == AnimationType::Unk1))]
    Unk1,

    #[br(pre_assert(animation_type == AnimationType::Unk2))]
    Unk2,

    #[br(pre_assert(animation_type == AnimationType::PackedCubic))]
    PackedCubic(#[br(args_raw(base_offset))] PackedCubicData),
}

#[derive(BinRead, Debug)]
#[br(import_raw(base_offset: u64))]
pub struct PackedCubicData {
    #[br(offset = base_offset)]
    pub tracks: SarData<Track>,

    #[br(offset = base_offset)]
    pub translations: SarData<[f32; 4]>,

    #[br(offset = base_offset)]
    pub rotation_quaternions: SarData<[f32; 4]>,

    // TODO: Are these keyframe times?
    #[br(offset = base_offset)]
    pub timings: SarData<u16>,
}

#[derive(BinRead, Debug)]
pub struct Track {
    pub translaton: SubTrack,
    pub rotation: SubTrack,
    pub scale: SubTrack,
}

#[derive(BinRead, Debug)]
pub struct SubTrack {
    pub time_start_index: u32,
    pub curves_start_index: u32,
    pub time_end_index: u32,
}

#[derive(BinRead, Debug)]
#[br(magic(b"SKEL"))]
#[br(import { base_offset: u64 })]
pub struct Skel {
    pub unks: [u32; 10],

    #[br(offset = base_offset)]
    pub parents: SarData<i16>,

    #[br(args { offset: base_offset, inner: base_offset })]
    pub names: SarData<BoneName>,

    #[br(offset = base_offset)]
    pub transforms: SarData<Transform>,

    // TODO: types?
    #[br(offset = base_offset)]
    pub unk_table1: SarData<u8>,
    #[br(offset = base_offset)]
    pub unk_table2: SarData<u8>,
    #[br(offset = base_offset)]
    pub unk_table3: SarData<u8>,
    #[br(offset = base_offset)]
    pub unk_table4: SarData<u8>,
    #[br(offset = base_offset)]
    pub unk_table5: SarData<u8>,
    // TODO: other fields?
}

#[derive(BinRead, Debug)]
#[br(magic(b"eva\x00"))]
pub struct Eva {
    pub unk1: u32,
}

#[derive(BinRead, Debug)]
pub struct Transform {
    pub position: [f32; 4],
    pub rotation_quaternion: [f32; 4],
    pub scale: [f32; 4],
}

#[derive(BinRead, Debug)]
#[br(import_raw(base_offset: u64))]
pub struct BoneName {
    #[br(parse_with = parse_string_ptr32, offset = base_offset)]
    #[br(pad_after = 12)]
    pub name: String,
}

#[derive(BinRead, Debug)]
#[br(magic(b"ASMB"))]
pub struct Asmb {
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

#[binread]
#[derive(Debug)]
#[br(import_raw(args: FilePtrArgs<T::Args<'_>>))]
pub struct SarData<T>
where
    T: BinRead + 'static,
    for<'a> T::Args<'a>: Clone + Default,
{
    #[br(temp)]
    offset: u64,
    #[br(temp)]
    count: u32,

    // TODO: Use parse_with for this instead?
    #[br(args { count: count as usize, inner: args.inner })]
    #[br(seek_before = SeekFrom::Start(args.offset + offset as u64))]
    #[br(restore_position)]
    pub elements: Vec<T>,

    pub unk1: i32,
}
