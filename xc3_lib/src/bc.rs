//! Animation and skeleton data for [Sar1](crate::sar1::Sar1) archives.
use std::io::SeekFrom;

use crate::{parse_string_ptr32, parse_string_ptr64};
use binrw::{binread, file_ptr::FilePtrArgs, BinRead, FilePtr64};

// Assume the BC is at the root of the reader to simplify offsets.
#[binread]
#[derive(Debug)]
#[br(magic(b"BC\x00\x00"))]
#[br(stream = r)]
pub struct Bc {
    pub unk1: u32,
    pub data_size: u32,
    pub unk_count: u32,   // TODO: count for u64s?
    pub data_offset: u64, // TODO: offset for bcdata?
    pub unk_offset: u64,  // TODO: offset to u64s?
    pub data: BcData,
}

#[derive(Debug, BinRead)]
pub enum BcData {
    #[br(magic(2u32))]
    Skdy(Skdy),

    #[br(magic(4u32))]
    Anim(Anim),

    #[br(magic(6u32))]
    Skel(Skel),

    #[br(magic(7u32))]
    Asmb(Asmb),
}

#[derive(Debug, BinRead)]
#[br(magic(b"ASMB"))]
pub struct Asmb {
    pub unk1: u32,
}

// skeleton dynamics?
#[derive(Debug, BinRead)]
#[br(magic(b"SKDY"))]
pub struct Skdy {
    pub unk1: u32,
}

#[derive(Debug, BinRead)]
#[br(magic(b"ANIM"))]
pub struct Anim {
    #[br(parse_with = FilePtr64::parse)]
    pub binding: AnimationBinding,
}

#[derive(Debug, BinRead)]
pub struct Animation {
    pub unk1: BcList<()>,
    pub unk_offset1: u64,

    #[br(parse_with = parse_string_ptr64)]
    pub name: String,

    pub animation_type: AnimationType,
    pub space_mode: u8,
    pub play_mode: u8,
    pub blend_mode: u8,
    pub frames_per_second: f32,
    pub seconds_per_frame: f32,
    pub frame_count: u32,
    pub unk2: BcList<()>,
    pub unk3: u64,

    #[br(args { animation_type })]
    pub data: AnimationData,
}

#[derive(Debug, BinRead)]
pub struct AnimationBinding {
    // TODO: More data?
    pub unk1: BcList<()>,

    // u64?
    pub unk2: u64,

    // TODO: Avoid needing to match multiple times on animation type?
    #[br(parse_with = FilePtr64::parse)]
    pub animation: Animation,

    // TODO: Same length and ordering as hashes?
    // TODO: convert to indices in the mxmd skeleton based on hashes?
    // TODO: Are these always 0..N-1?
    // i.e are the hashes always unique?
    // TODO: same length and ordering as tracks?
    pub bone_indices: BcList<i16>,
    // TODO: extra track bindings?
    pub bone_names: BcList<StringOffset>,

    #[br(args_raw(animation.animation_type))]
    pub extra_track_animation: ExtraTrackAnimation,
}

// TODO: Is this the right type?
#[derive(Debug, BinRead)]
pub struct StringOffset {
    #[br(parse_with = parse_string_ptr64)]
    pub name: String,
}

#[derive(Debug, BinRead)]
#[br(import_raw(animation_type: AnimationType))]
pub struct ExtraTrackAnimation {
    #[br(parse_with = parse_string_ptr64)]
    pub unk1: String,
    pub unk2: u32,
    pub unk3: i32,
    pub unk6: u32,
    pub unk7: u32,

    #[br(parse_with = FilePtr64::parse)]
    #[br(args { inner: animation_type })]
    pub data: ExtraTrackAnimationData,

    pub unk_offset: u64,
}

#[derive(Debug, BinRead)]
#[br(import_raw(animation_type: AnimationType))]
pub enum ExtraTrackAnimationData {
    #[br(pre_assert(animation_type == AnimationType::Unk0))]
    Unk0,

    #[br(pre_assert(animation_type == AnimationType::Cubic))]
    Unk1,

    #[br(pre_assert(animation_type == AnimationType::Unk2))]
    Unk2,

    #[br(pre_assert(animation_type == AnimationType::PackedCubic))]
    PackedCubic(PackedCubicExtraData),
}

#[derive(Debug, BinRead)]
pub struct PackedCubicExtraData {
    // TODO: buffers?
    pub unk1: BcList<u8>,
    pub unk2: BcList<u8>,

    // The MurmurHash3 32-bit hash of the bone names.
    // TODO: type alias for hash?
    pub hashes: BcList<u32>,
}

#[derive(Debug, BinRead, PartialEq, Eq, Clone, Copy)]
#[br(repr(u8))]
pub enum AnimationType {
    Unk0 = 0,
    Cubic = 1,
    Unk2 = 2,
    PackedCubic = 3,
}

#[derive(Debug, BinRead)]
#[br(import { animation_type: AnimationType})]
pub enum AnimationData {
    #[br(pre_assert(animation_type == AnimationType::Unk0))]
    Unk0,

    #[br(pre_assert(animation_type == AnimationType::Cubic))]
    Cubic(Cubic),

    #[br(pre_assert(animation_type == AnimationType::Unk2))]
    Unk2,

    #[br(pre_assert(animation_type == AnimationType::PackedCubic))]
    PackedCubic(PackedCubic),
}

#[derive(Debug, BinRead)]
pub struct Cubic {
    pub tracks: BcList<CubicTrack>,
}

#[derive(Debug, BinRead)]
pub struct CubicTrack {
    pub translation: BcList<KeyFrameCubicVec3>,
    pub rotation: BcList<KeyFrameCubicQuaternion>,
    pub scale: BcList<KeyFrameCubicVec3>,
}

#[derive(Debug, BinRead)]
pub struct KeyFrameCubicVec3 {
    pub time: f32,
    pub x: [f32; 4],
    pub y: [f32; 4],
    pub z: [f32; 4],
}

#[derive(Debug, BinRead)]
pub struct KeyFrameCubicQuaternion {
    pub time: f32,
    pub x: [f32; 4],
    pub y: [f32; 4],
    pub z: [f32; 4],
    pub w: [f32; 4],
}

#[derive(Debug, BinRead)]
pub struct PackedCubic {
    // TODO: same length and ordering as bone indices and hashes?
    pub tracks: BcList<PackedCubicTrack>,

    // TODO: [a,b,c,d] for a*x^3 + b*x^2 + c*x + d?
    pub vectors: BcList<[f32; 4]>,

    // TODO: same equation as above?
    pub quaternions: BcList<[f32; 4]>,

    // TODO: Are these keyframe times?
    pub timings: BcList<u16>,
}

#[derive(Debug, BinRead)]
pub struct PackedCubicTrack {
    pub translation: SubTrack,
    pub rotation: SubTrack,
    pub scale: SubTrack,
}

#[derive(Debug, BinRead)]
pub struct SubTrack {
    // TODO: index into timings?
    pub time_start_index: u32,
    /// Starting index for the vector or quaternion values.
    pub curves_start_index: u32,
    // TODO: index into timings?
    pub time_end_index: u32,
}

#[derive(Debug, BinRead)]
#[br(magic(b"SKEL"))]
pub struct Skel {
    pub unks: [u32; 10],

    pub parents: BcList<i16>,
    pub names: BcList<BoneName>,
    pub transforms: BcList<Transform>,

    // TODO: types?
    pub unk_table1: BcList<u8>,
    pub unk_table2: BcList<u8>,
    pub unk_table3: BcList<u8>,
    pub unk_table4: BcList<u8>,
    pub unk_table5: BcList<u8>,
    // TODO: other fields?
}

#[derive(Debug, BinRead)]
pub struct Transform {
    pub translation: [f32; 4],
    pub rotation_quaternion: [f32; 4],
    pub scale: [f32; 4],
}

#[derive(Debug, BinRead)]
pub struct BoneName {
    // TODO: Is this a 64-bit pointer?
    #[br(parse_with = parse_string_ptr32)]
    #[br(pad_after = 12)]
    pub name: String,
}

#[binread]
#[derive(Debug)]
#[br(import_raw(args: FilePtrArgs<T::Args<'_>>))]
pub struct BcList<T>
where
    T: BinRead + 'static,
    for<'a> T::Args<'a>: Clone + Default,
{
    // TODO: parse_offset64_count?
    #[br(temp)]
    offset: u64,
    #[br(temp)]
    count: u32,

    // TODO: Use parse_with for this instead?
    // TODO: How to handle offset of 0?
    #[br(args { count: count as usize, inner: args.inner })]
    #[br(seek_before = SeekFrom::Start(args.offset + offset as u64))]
    #[br(restore_position)]
    pub elements: Vec<T>,

    pub unk1: i32,
}

/// Produce the 32-bit hash for a value like a bone name.
pub fn murmur3(bytes: &[u8]) -> u32 {
    murmur3::murmur3_32(&mut std::io::Cursor::new(bytes), 0).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_bones_murmur3() {
        // Check that wimdo bone name hashes match the mot hashes.
        // xeno3/chr/ch/ch01012013.wimdo
        // xeno3/chr/ch/ch01011000_battle.mot
        assert_eq!(0x47df19d5, murmur3("J_thumb_A_R".as_bytes()));
        assert_eq!(0xfd011736, murmur3("J_hip".as_bytes()));
    }
}
