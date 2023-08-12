use std::io::SeekFrom;

use crate::{parse_ptr32, parse_string_ptr32};
use binrw::{binread, file_ptr::FilePtrArgs, BinRead};

// Assume the BC is at the root of the reader to simplify offsets.
#[binread]
#[derive(Debug)]
#[br(magic(b"BC"))]
#[br(stream = r)]
pub struct Bc {
    pub unk_flags: u16,

    pub unk1: u32,
    pub data_size: u32,
    pub unk_count: u32,
    pub data_offset: u64, // TODO: offset for bcdata?
    pub unk_offset: u64,  // TODO: offset to u64s?

    pub data: BcData,
}

#[derive(BinRead, Debug)]
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

#[derive(BinRead, Debug)]
#[br(magic(b"ASMB"))]
pub struct Asmb {
    pub unk1: u32,
}

// skeleton dynamics?
#[derive(BinRead, Debug)]
#[br(magic(b"SKDY"))]
pub struct Skdy {
    pub unk1: u32,
}

#[derive(BinRead, Debug)]
#[br(magic(b"ANIM"))]
pub struct Anim {
    #[br(parse_with = parse_ptr32)]
    pub header: AnimHeader,

    pub unks_1: u32,
    pub unks_2: BcList<()>,
    pub unks_3: u32,
    pub unks_4: u32,
    #[br(parse_with = parse_string_ptr32)]
    pub name: String,
    pub unks_5: u32,

    pub animation_type: AnimationType,
    pub space_mode: u8,
    pub play_mode: u8,
    pub blend_mode: u8,
    pub frames_per_second: f32,
    pub seconds_per_frame: f32,
    pub frame_count: u32,
    pub unk1: BcList<()>,
    pub unk5: u64,

    #[br(args { animation_type })]
    pub data: AnimationData,
    // TODO: more fields?
}

#[derive(BinRead, Debug)]
pub struct AnimHeader {
    // TODO: More sar data?
    pub unk1: BcList<()>,
    pub unk2: [u32; 4],

    // TODO: Same length and ordering as hashes?
    // TODO: convert to indices in the mxmd skeleton based on hashes?
    // TODO: Are these always 0..N-1?
    // i.e are the hashes always unique?
    // TODO: same length and ordering as tracks?
    pub bone_indices: BcList<i16>,

    pub unk3: BcList<()>, // TODO: type?

    pub unk4: BcList<()>, // TODO: type?

    pub unk5: u32,
    pub unk6: u32,

    #[br(parse_with = parse_ptr32)]
    pub inner: AnimHeaderInner,
}

// TODO: animation type 1 doesn't have hashes, so indices aren't remapped?
#[derive(BinRead, Debug)]
pub struct AnimHeaderInner {
    // TODO: Types?
    pub unk1: BcList<()>,
    pub unk2: BcList<()>,
    /// The MurmurHash3 32-bit hash of the bone names.
    // TODO: type alias for this?
    pub hashes: BcList<u32>,
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
#[br(import { animation_type: AnimationType})]
pub enum AnimationData {
    #[br(pre_assert(animation_type == AnimationType::Unk0))]
    Unk0,

    #[br(pre_assert(animation_type == AnimationType::Unk1))]
    Cubic(Cubic),

    #[br(pre_assert(animation_type == AnimationType::Unk2))]
    Unk2,

    #[br(pre_assert(animation_type == AnimationType::PackedCubic))]
    PackedCubic(PackedCubic),
}

#[derive(BinRead, Debug)]
pub struct Cubic {
    pub tracks: BcList<CubicTrack>,
}

#[derive(BinRead, Debug)]
pub struct CubicTrack {
    pub translation: BcList<KeyFrameCubicVec3>,
    pub rotation: BcList<KeyFrameCubicQuaternion>,
    pub scale: BcList<KeyFrameCubicVec3>,
}

#[derive(BinRead, Debug)]
pub struct KeyFrameCubicVec3 {
    pub time: f32,
    pub x: [f32; 4],
    pub y: [f32; 4],
    pub z: [f32; 4],
}

#[derive(BinRead, Debug)]
pub struct KeyFrameCubicQuaternion {
    pub time: f32,
    pub x: [f32; 4],
    pub y: [f32; 4],
    pub z: [f32; 4],
    pub w: [f32; 4],
}

#[derive(BinRead, Debug)]
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

#[derive(BinRead, Debug)]
pub struct PackedCubicTrack {
    pub translation: SubTrack,
    pub rotation: SubTrack,
    pub scale: SubTrack,
}

#[derive(BinRead, Debug)]
pub struct SubTrack {
    // TODO: index into timings?
    pub time_start_index: u32,
    /// Starting index for the vector or quaternion values.
    pub curves_start_index: u32,
    // TODO: index into timings?
    pub time_end_index: u32,
}

#[derive(BinRead, Debug)]
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

#[derive(BinRead, Debug)]
pub struct Transform {
    pub translation: [f32; 4],
    pub rotation_quaternion: [f32; 4],
    pub scale: [f32; 4],
}

#[derive(BinRead, Debug)]
pub struct BoneName {
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
