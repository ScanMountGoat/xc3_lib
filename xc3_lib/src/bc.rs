//! Animation and skeleton data for [Sar1](crate::sar1::Sar1) archives.
use crate::{parse_offset64_count32, parse_ptr64, parse_string_ptr64};
use binrw::{args, binread, BinRead, BinWrite};
use xc3_write::{xc3_write_binwrite_impl, VecOffsets, Xc3Write, Xc3WriteOffsets};

// Assume the BC is at the beginning of the reader to simplify offsets.
#[binread]
#[derive(Debug, Xc3Write, Xc3WriteOffsets)]
#[br(magic(b"BC\x00\x00"))]
#[br(stream = r)]
#[xc3(magic(b"BC\x00\x00"))]
pub struct Bc {
    pub unk1: u32,
    pub data_size: u32, // TODO: bc data size?
    pub unk_count: u32,

    #[br(parse_with = parse_ptr64)]
    #[xc3(offset64)]
    pub data: BcData,

    // TODO: A list of offsets to data items?
    #[br(parse_with = parse_ptr64)]
    #[br(args { inner: args! { count: unk_count as usize}})]
    #[xc3(offset64)]
    pub unks: Vec<u64>,
}

// TODO: variant level magic?
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub enum BcData {
    #[br(magic(2u32))]
    #[xc3(magic(2u32))]
    Skdy(Skdy),

    #[br(magic(4u32))]
    #[xc3(magic(4u32))]
    Anim(Anim),

    #[br(magic(6u32))]
    #[xc3(magic(6u32))]
    Skel(Skel),

    #[br(magic(7u32))]
    #[xc3(magic(7u32))]
    Asmb(Asmb),
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
#[br(magic(b"ASMB"))]
#[xc3(magic(b"ASMB"))]
pub struct Asmb {
    pub unk1: u32,
}

// skeleton dynamics?
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
#[br(magic(b"SKDY"))]
#[xc3(magic(b"SKDY"))]
pub struct Skdy {
    pub unk1: u32,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
#[br(magic(b"ANIM"))]
#[xc3(magic(b"ANIM"))]
pub struct Anim {
    #[br(parse_with = parse_ptr64)]
    #[xc3(offset64)]
    pub binding: AnimationBinding,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct AnimationBinding {
    // TODO: More data?
    pub unk1: BcList<()>,

    // u64?
    pub unk2: u64,

    // TODO: Avoid needing to match multiple times on animation type?
    #[br(parse_with = parse_ptr64)]
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

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct Animation {
    pub unk1: BcList<()>,
    pub unk_offset1: u64,

    #[br(parse_with = parse_string_ptr64)]
    #[xc3(offset64)]
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

// TODO: Is this the right type?
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct StringOffset {
    #[br(parse_with = parse_string_ptr64)]
    #[xc3(offset64)]
    pub name: String,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
#[br(import_raw(animation_type: AnimationType))]
pub struct ExtraTrackAnimation {
    #[br(parse_with = parse_string_ptr64)]
    #[xc3(offset64)]
    pub unk1: String,

    pub unk2: u32,
    pub unk3: i32,
    pub unk6: u32,
    pub unk7: u32,

    #[br(parse_with = parse_ptr64)]
    #[br(args { inner: animation_type })]
    pub data: ExtraTrackAnimationData,

    pub unk_offset: u64,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
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

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct PackedCubicExtraData {
    // TODO: buffers?
    pub unk1: BcList<u8>,
    pub unk2: BcList<u8>,

    // The MurmurHash3 32-bit hash of the bone names.
    // TODO: type alias for hash?
    pub hashes: BcList<u32>,
}

#[derive(Debug, BinRead, BinWrite, PartialEq, Eq, Clone, Copy)]
#[brw(repr(u8))]
pub enum AnimationType {
    Unk0 = 0,
    Cubic = 1,
    Unk2 = 2,
    PackedCubic = 3,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
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

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct Cubic {
    pub tracks: BcList<CubicTrack>,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct CubicTrack {
    pub translation: BcList<KeyFrameCubicVec3>,
    pub rotation: BcList<KeyFrameCubicQuaternion>,
    pub scale: BcList<KeyFrameCubicVec3>,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct KeyFrameCubicVec3 {
    pub time: f32,
    pub x: [f32; 4],
    pub y: [f32; 4],
    pub z: [f32; 4],
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct KeyFrameCubicQuaternion {
    pub time: f32,
    pub x: [f32; 4],
    pub y: [f32; 4],
    pub z: [f32; 4],
    pub w: [f32; 4],
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
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

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct PackedCubicTrack {
    pub translation: SubTrack,
    pub rotation: SubTrack,
    pub scale: SubTrack,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct SubTrack {
    // TODO: index into timings?
    pub time_start_index: u32,
    /// Starting index for the vector or quaternion values.
    pub curves_start_index: u32,
    // TODO: index into timings?
    pub time_end_index: u32,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
#[br(magic(b"SKEL"))]
#[xc3(magic(b"SKEL"))]
pub struct Skel {
    #[br(parse_with = parse_ptr64)]
    #[xc3(offset64)]
    pub skeleton: Skeleton,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct Skeleton {
    pub unk1: BcList<u8>,

    pub unk2: u64, // 0

    #[br(parse_with = parse_string_ptr64)]
    #[xc3(offset64)]
    pub root_bone_name: String,

    pub parent_indices: BcList<i16>,
    pub names: BcList<BoneName>,
    pub transforms: BcList<Transform>,

    pub unk_table1: BcList<SkeletonUnk1>,
    pub unk_table2: BcList<u64>,
    pub unk_table3: BcList<StringOffset>,
    pub unk_table4: BcList<[[f32; 4]; 3]>,
    pub unk_table5: BcList<u64>,

    #[br(parse_with = parse_ptr64)]
    #[xc3(offset64)]
    pub unk6: SkeletonUnk6,

    #[br(parse_with = parse_ptr64)]
    #[xc3(offset64)]
    pub unk7: SkeletonUnk7,

    #[br(parse_with = parse_ptr64)]
    #[xc3(offset64)]
    pub unk8: SkeletonUnk8,

    #[br(parse_with = parse_ptr64)]
    #[xc3(offset64)]
    pub unk9: SkeletonUnk9,

    #[br(parse_with = parse_ptr64)]
    #[xc3(offset64)]
    pub unk10: SkeletonUnk10,

    #[br(parse_with = parse_ptr64)]
    #[xc3(offset64)]
    pub unk11: SkeletonUnk11,

    pub unk12: u64,
    pub unk13: i64,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct Transform {
    pub translation: [f32; 4],
    pub rotation_quaternion: [f32; 4],
    pub scale: [f32; 4],
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct BoneName {
    #[br(parse_with = parse_string_ptr64)]
    #[xc3(offset64)]
    pub name: String,

    // TODO: padding?
    pub unk: [u32; 2],
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct SkeletonUnk1 {
    #[br(parse_with = parse_string_ptr64)]
    #[xc3(offset64)]
    pub unk1: String,

    pub unk2: BcList<StringOffset>,
    pub unk3: BcList<f32>,
    pub unk4: BcList<[f32; 2]>,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct SkeletonUnk6 {
    pub unk1: BcList<u8>,
    pub unk2: BcList<u16>,

    #[br(parse_with = parse_offset64_count32)]
    #[xc3(offset64_count32)]
    pub unk3: Vec<u32>,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct SkeletonUnk7 {
    pub unk1: BcList<u8>,
    pub unk2: BcList<u16>,

    // TODO: type?
    #[br(parse_with = parse_offset64_count32)]
    #[xc3(offset64_count32)]
    pub unk3: Vec<u32>,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct SkeletonUnk8 {
    #[br(parse_with = parse_offset64_count32)]
    #[xc3(offset64_count32)]
    pub unk1: Vec<u32>,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct SkeletonUnk9 {
    // TODO: type?
    pub unk1: BcList<[u32; 13]>,

    // TODO: type?
    #[br(parse_with = parse_offset64_count32)]
    #[xc3(offset64_count32)]
    pub unk2: Vec<u32>,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct SkeletonUnk10 {
    // TODO: type?
    pub unk1: [u32; 8],
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct SkeletonUnk11 {
    #[br(parse_with = parse_offset64_count32)]
    #[xc3(offset64_count32)]
    pub unk1: Vec<u8>,
}

#[binread]
#[derive(Debug, Xc3Write, Xc3WriteOffsets)]
pub struct BcList<T>
where
    T: BinRead + Xc3Write + 'static,
    for<'a> T: BinRead<Args<'a> = ()>,
    for<'a> VecOffsets<<T as Xc3Write>::Offsets<'a>>: Xc3WriteOffsets,
{
    #[br(parse_with = parse_offset64_count32)]
    #[xc3(offset64_count32)]
    pub elements: Vec<T>,

    // TODO: Does this field do anything?
    pub unk1: i32,
}

/// Produce the 32-bit hash for a value like a bone name.
pub fn murmur3(bytes: &[u8]) -> u32 {
    murmur3::murmur3_32(&mut std::io::Cursor::new(bytes), 0).unwrap()
}

xc3_write_binwrite_impl!(AnimationType);

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
