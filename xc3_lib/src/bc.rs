//! Animation and skeleton data in `.anm` or `.motstm_data` files or [Sar1](crate::sar1::Sar1) archives.
use std::collections::{BTreeMap, BTreeSet};

use crate::{
    parse_offset64_count32, parse_opt_ptr64, parse_ptr64, parse_string_opt_ptr64,
    parse_string_ptr64, xc3_write_binwrite_impl,
};
use binrw::{args, binread, BinRead, BinWrite};
use xc3_write::{round_up, VecOffsets, Xc3Write, Xc3WriteOffsets};

// TODO: is the 64 byte alignment on the sar1 entry size?
// TODO: Add class names from xenoblade 2 binary where appropriate.
// Assume the BC is at the beginning of the reader to simplify offsets.
#[binread]
#[derive(Debug, Xc3Write, Xc3WriteOffsets)]
#[br(magic(b"BC\x00\x00"))]
#[br(stream = r)]
#[xc3(magic(b"BC\x00\x00"))]
#[xc3(align_after(64))]
pub struct Bc {
    pub unk1: u32,
    // TODO: not always equal to the sar1 size?
    pub data_size: u32,
    pub address_count: u32,

    #[br(parse_with = parse_ptr64)]
    #[xc3(offset(u64))]
    pub data: BcData,

    // TODO: A list of offsets to data items?
    // TODO: relocatable addresses?
    #[br(parse_with = parse_ptr64)]
    #[br(args { inner: args! { count: address_count as usize}})]
    #[xc3(offset(u64), align(8, 0xff))]
    pub addresses: Vec<u64>,
}

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
    #[br(parse_with = parse_ptr64)]
    #[xc3(offset(u64))]
    pub dynamics: Dynamics,
}

// TODO: All names should be written at the end.
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct Dynamics {
    pub unk1: BcList<()>,
    pub unk2: u64,

    #[br(parse_with = parse_ptr64)]
    #[xc3(offset(u64))]
    pub unk3: DynamicsUnk1,

    // TODO: not always present?
    #[br(parse_with = parse_ptr64)]
    #[br(if(!unk3.unk1.elements.is_empty()))]
    #[xc3(offset(u64))]
    pub unk4: Option<DynamicsUnk2>,

    // TODO: not always present?
    #[br(parse_with = parse_ptr64)]
    #[br(if(!unk3.unk1.elements.is_empty()))]
    #[xc3(offset(u64))]
    pub unk5: Option<DynamicsUnk3>,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct DynamicsUnk1 {
    pub unk1: BcList<DynamicsUnk1Item>,
    // TODO: type?
    pub unk2: BcList<u8>,
    pub unk3: BcList<u8>,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct DynamicsUnk1Item {
    pub unk1: u32,
    pub unk2: i32,

    #[br(parse_with = parse_string_ptr64)]
    #[xc3(offset(u64))]
    pub name1: String,

    // TODO: Shared offset to string + 0xFF?
    #[br(parse_with = parse_string_ptr64)]
    #[xc3(offset(u64))]
    pub name2: String,
    pub unk4: u32,
    pub unk5: i32,

    pub unk6: [f32; 9],
    pub unk7: [i32; 3],
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct DynamicsUnk2 {
    pub unk1: BcList<DynamicsUnk2Item>,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct DynamicsUnk2Item {
    #[br(parse_with = parse_string_ptr64)]
    #[xc3(offset(u64))]
    pub name: String,

    pub unk1: BcList<DynamicsUnk2ItemUnk1>,
    pub unk2: BcList<[f32; 4]>,
    pub unk3: BcList<DynamicsUnk2ItemUnk3>,
    pub unk4: BcList<()>,
    pub unk5: BcList<()>,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct DynamicsUnk2ItemUnk1 {
    #[br(parse_with = parse_string_ptr64)]
    #[xc3(offset(u64))]
    pub name1: String,

    #[br(parse_with = parse_string_ptr64)]
    #[xc3(offset(u64))]
    pub name2: String,

    pub unk1: [f32; 7],
    pub unk2: u32,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct DynamicsUnk2ItemUnk3 {
    #[br(parse_with = parse_string_ptr64)]
    #[xc3(offset(u64))]
    pub name: String,

    pub unk1: [f32; 7],
    pub unk2: u32,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct DynamicsUnk3 {
    // TODO: points to string section?
    pub unk1: BcList<()>,
}

#[derive(Debug, BinRead, Xc3Write)]
#[br(magic(b"ANIM"))]
#[xc3(magic(b"ANIM"))]
pub struct Anim {
    #[br(parse_with = parse_ptr64)]
    #[xc3(offset(u64), align(8, 0xff))]
    pub binding: AnimationBinding,
}

#[binread]
#[derive(Debug, Xc3Write)]
#[br(stream = r)]
pub struct AnimationBinding {
    // Use temp fields to estimate the struct size.
    // These fields will be skipped when writing.
    // TODO: is there a better way to handle game specific differences?
    #[br(temp, try_calc = r.stream_position())]
    pub base_offset: u64,

    pub unk1: BcList<()>,
    pub unk2: u64, // 0?

    #[br(parse_with = parse_ptr64)]
    #[xc3(offset(u64))]
    pub animation: Animation,

    // Store the offset for the next field.
    #[br(temp, restore_position)]
    indices_offset: u64,

    /// The index of the track in [animation](#structfield.animation) for each bone
    /// or `-1` if no track is assigned.
    // TODO: chr bone ordering?
    // TODO: ordering can be changed by bone names or hashes?
    pub bone_track_indices: BcList<i16>,

    #[br(args { size: indices_offset - base_offset, animation_type: animation.animation_type })]
    pub inner: AnimationBindingInner,
}

// TODO: Is there a simpler way of doing this?
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
#[br(import { size: u64, animation_type: AnimationType })]
pub enum AnimationBindingInner {
    // XC2 has 60 total bytes.
    #[br(pre_assert(size == 60))]
    Unk1(AnimationBindingInner1),

    // XC1 and XC3 have 76 total bytes.
    #[br(pre_assert(size == 76))]
    Unk2(AnimationBindingInner2),

    // XC3 sometimes has 120 or 128 total bytes.
    #[br(pre_assert(size >= 120))]
    Unk3(#[br(args_raw(animation_type))] AnimationBindingInner3),
}

// 60 total bytes for xc2
#[derive(Debug, BinRead, Xc3Write)]
pub struct AnimationBindingInner1 {
    #[br(parse_with = parse_offset64_count32)]
    #[xc3(offset_count(u64, u32), align(8, 0xff))]
    pub extra_track_bindings: Vec<ExtraTrackAnimationBinding>,
}

#[derive(Debug, BinRead, Xc3Write)]
pub struct ExtraTrackAnimationBinding {
    #[br(parse_with = parse_opt_ptr64)]
    #[xc3(offset(u64))]
    pub extra_track_animation: Option<ExtraTrackAnimation>,

    // TODO: This can have 0 offset but nonzero count?
    // TODO: Is it worth preserving the count if the offset is 0?
    // TODO: Should this be ignored if extra_track_animation is None?
    #[br(parse_with = parse_offset64_count32)]
    #[xc3(offset_count(u64, u32), align(8, 0xff))]
    pub track_indices: Vec<i16>,
    pub unk1: i32,
}

#[derive(Debug, BinRead, Xc3Write)]
pub struct ExtraTrackAnimation {
    pub unk1: u64,

    #[br(parse_with = parse_string_ptr64)]
    #[xc3(offset(u64), align(8, 0xff))]
    pub name: String,

    pub animation_type: AnimationType,
    pub blend_mode: BlendMode,
    pub unk2: u8,
    pub unk3: u8,

    pub unk4: i32,

    // TODO: depends on type?
    pub values: BcList<f32>,
}

// 76 total bytes for xc1 or xc3
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct AnimationBindingInner2 {
    /// An alternative bone name list for
    /// [bone_track_indices](struct.AnimationBinding.html#structfield.bone_track_indices).
    pub bone_names: BcList<StringOffset>,

    // TODO: type?
    #[br(parse_with = parse_offset64_count32)]
    #[xc3(offset_count(u64, u32))]
    pub unk1: Vec<u32>,
}

// 120 total bytes for xc3
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
#[br(import_raw(animation_type: AnimationType))]
pub struct AnimationBindingInner3 {
    /// An alternative bone name list for
    /// [bone_track_indices](struct.AnimationBinding.html#structfield.bone_track_indices).
    pub bone_names: BcList<StringOffset>,

    #[br(args_raw(animation_type))]
    pub extra_track_data: ExtraTrackData,
}

#[derive(Debug, BinRead, Xc3Write)]
pub struct Animation {
    pub unk1: BcList<()>,
    pub unk_offset1: u64,

    #[br(parse_with = parse_string_ptr64)]
    #[xc3(offset(u64), align(8, 0xff))]
    pub name: String,

    pub animation_type: AnimationType,
    pub space_mode: SpaceMode,
    pub play_mode: PlayMode,
    pub blend_mode: BlendMode,
    pub frames_per_second: f32,
    pub seconds_per_frame: f32,
    pub frame_count: u32,

    // TODO: Add alignment customization to BcList?
    #[br(parse_with = parse_offset64_count32)]
    #[xc3(offset_count(u64, u32), align(8, 0xff))]
    pub notifies: Vec<AnimationNotify>,
    pub unk2: i32,

    #[br(parse_with = parse_opt_ptr64)]
    #[xc3(offset(u64), align(16, 0xff))]
    pub locomotion: Option<AnimationLocomotion>,

    #[br(args { animation_type })]
    pub data: AnimationData,
}

#[derive(Debug, BinRead, BinWrite, PartialEq, Eq, Clone, Copy)]
#[brw(repr(u8))]
pub enum SpaceMode {
    Local = 0,
    Model = 1,
}

#[derive(Debug, BinRead, BinWrite, PartialEq, Eq, Clone, Copy)]
#[brw(repr(u8))]
pub enum PlayMode {
    Loop = 0,
    Single = 1,
}

#[derive(Debug, BinRead, BinWrite, PartialEq, Eq, Clone, Copy)]
#[brw(repr(u8))]
pub enum BlendMode {
    Blend = 0,
    Add = 1,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct AnimationNotify {
    pub time: f32,
    pub unk2: i32,

    #[br(parse_with = parse_string_ptr64)]
    #[xc3(offset(u64))]
    pub unk3: String,

    #[br(parse_with = parse_string_ptr64)]
    #[xc3(offset(u64))]
    pub unk4: String,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct AnimationLocomotion {
    pub unk1: [u32; 4],
    pub seconds_per_frame: f32,
    pub unk2: i32,

    // TODO: type?
    #[br(parse_with = parse_offset64_count32)]
    #[xc3(offset_count(u64, u32), align(16, 0xff))]
    pub unk3: Vec<[u32; 4]>,
}

// TODO: Is this the right type?
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct StringOffset {
    #[br(parse_with = parse_string_opt_ptr64)]
    #[xc3(offset(u64))]
    pub name: Option<String>,
}

// TODO: is this only for XC3?
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
#[br(import_raw(animation_type: AnimationType))]
pub enum ExtraTrackData {
    #[br(pre_assert(animation_type == AnimationType::Uncompressed))]
    Uncompressed(UncompressedExtraData),

    #[br(pre_assert(animation_type == AnimationType::Cubic))]
    Cubic(CubicExtraData),

    // TODO: This has extra data?
    #[br(pre_assert(animation_type == AnimationType::Empty))]
    Empty,

    #[br(pre_assert(animation_type == AnimationType::PackedCubic))]
    PackedCubic(PackedCubicExtraData),
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct UncompressedExtraData {
    // TODO: type?
    pub unk1: BcList<u8>,

    #[br(parse_with = parse_opt_ptr64)]
    #[xc3(offset(u64))]
    pub motion: Option<UncompressedExtraDataMotion>,

    #[br(parse_with = parse_ptr64)]
    #[xc3(offset(u64))]
    pub unk3: UncompressedExtraDataUnk3,
}

// TODO: Default transform for a single bone at each frame?
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct UncompressedExtraDataMotion {
    pub translation: BcList<[f32; 4]>,
    pub rotation: BcList<[f32; 4]>,
    pub scale: BcList<[f32; 4]>,

    // length = frame count?
    // length * hashes length = transforms length?
    pub translation_indices: BcList<u16>,
    pub rotation_indices: BcList<u16>,
    pub scale_indices: BcList<u16>,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct UncompressedExtraDataUnk3 {
    // max of 254?
    // TODO: same length as hash indices?
    pub unk1: BcList<u8>,
    // TODO: assigns hashes to something?
    pub hash_indices: BcList<u16>,

    #[br(parse_with = parse_offset64_count32)]
    #[xc3(offset_count(u64, u32))]
    pub bone_name_hashes: Vec<u32>,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct CubicExtraData {
    // pointer to start of strings?
    #[br(parse_with = parse_string_ptr64)]
    #[xc3(offset(u64))]
    pub unk1: String,
    pub unk2: u32,
    pub unk3: i32,

    // TODO: root motion?
    pub unk4: u64,

    #[br(parse_with = parse_ptr64)]
    #[xc3(offset(u64))]
    pub data1: CubicExtraDataInner1,

    #[br(parse_with = parse_ptr64)]
    #[xc3(offset(u64))]
    pub data2: CubicExtraDataInner2,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct CubicExtraDataInner1 {
    // TODO: buffer?
    pub unk1: BcList<u8>,

    pub unk2: BcList<u16>,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct CubicExtraDataInner2 {
    // TODO: type?
    pub unk1: BcList<u8>, // ends with 0xFFFFFFFF?
    pub unk2: BcList<u8>,
}

// TODO: up to 64 bytes?
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct PackedCubicExtraData {
    // pointer to start of strings?
    #[br(parse_with = parse_string_ptr64)]
    #[xc3(offset(u64))]
    pub unk1: String,
    pub unk2: u32,
    pub unk3: i32,

    pub unk6: u32,
    pub unk7: u32,

    #[br(parse_with = parse_ptr64)]
    #[xc3(offset(u64))]
    pub data: PackedCubicExtraDataInner,

    pub unk_offset: u64,
    // TODO: optional padding?
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct PackedCubicExtraDataInner {
    // TODO: buffers?
    pub unk1: BcList<u8>,
    pub unk2: BcList<u8>,

    // The MurmurHash3 32-bit hash of the bone names.
    // TODO: type alias for hash?
    #[br(parse_with = parse_offset64_count32)]
    #[xc3(offset_count(u64, u32))]
    pub bone_name_hashes: Vec<u32>,
}

#[derive(Debug, BinRead, BinWrite, PartialEq, Eq, Clone, Copy)]
#[brw(repr(u8))]
pub enum AnimationType {
    Uncompressed = 0,
    Cubic = 1,
    Empty = 2,
    PackedCubic = 3,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
#[br(import { animation_type: AnimationType})]
pub enum AnimationData {
    #[br(pre_assert(animation_type == AnimationType::Uncompressed))]
    Uncompressed(Uncompressed),

    #[br(pre_assert(animation_type == AnimationType::Cubic))]
    Cubic(Cubic),

    #[br(pre_assert(animation_type == AnimationType::Empty))]
    Empty,

    #[br(pre_assert(animation_type == AnimationType::PackedCubic))]
    PackedCubic(PackedCubic),
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct Uncompressed {
    // TODO: Is every BcList aligned like this?
    #[br(parse_with = parse_offset64_count32)]
    #[xc3(offset_count(u64, u32), align(16, 0xff))]
    pub transforms: Vec<Transform>,
    pub unk1: i32,
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
    pub frame: f32,
    /// Coefficients `[a,b,c,d]` for `a*x^3 + b*x^2 + c*x + d` for frame index `x`.
    pub x: [f32; 4],
    /// Coefficients `[a,b,c,d]` for `a*x^3 + b*x^2 + c*x + d` for frame index `x`.
    pub y: [f32; 4],
    /// Coefficients `[a,b,c,d]` for `a*x^3 + b*x^2 + c*x + d` for frame index `x`.
    pub z: [f32; 4],
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct KeyFrameCubicQuaternion {
    pub frame: f32,
    /// Coefficients `[a,b,c,d]` for `a*x^3 + b*x^2 + c*x + d` for frame index `x`.
    pub x: [f32; 4],
    /// Coefficients `[a,b,c,d]` for `a*x^3 + b*x^2 + c*x + d` for frame index `x`.
    pub y: [f32; 4],
    /// Coefficients `[a,b,c,d]` for `a*x^3 + b*x^2 + c*x + d` for frame index `x`.
    pub z: [f32; 4],
    /// Coefficients `[a,b,c,d]` for `a*x^3 + b*x^2 + c*x + d` for frame index `x`.
    pub w: [f32; 4],
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct PackedCubic {
    // TODO: same length and ordering as bone indices and hashes?
    pub tracks: BcList<PackedCubicTrack>,

    /// Coefficients `[a,b,c,d]` for `a*x^3 + b*x^2 + c*x + d` for frame index `x`.
    pub vectors: BcList<[f32; 4]>,

    /// Coefficients `[a,b,c,d]` for `a*x^3 + b*x^2 + c*x + d` for frame index `x`.
    pub quaternions: BcList<[f32; 4]>,

    // TODO: Are these keyframe times?
    pub keyframes: BcList<u16>,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct PackedCubicTrack {
    pub translation: SubTrack,
    pub rotation: SubTrack,
    pub scale: SubTrack,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct SubTrack {
    /// Index into [keyframes](struct.PackedCubic.html#structfield.keyframes).
    pub keyframe_start_index: u32,
    /// Index into [vectors](struct.PackedCubic.html#structfield.vectors)
    /// or [quaternions](struct.PackedCubic.html#structfield.quaternions).
    pub curves_start_index: u32,
    /// Index into [keyframes](struct.PackedCubic.html#structfield.keyframes).
    pub keyframe_end_index: u32,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
#[br(magic(b"SKEL"))]
#[xc3(magic(b"SKEL"))]
pub struct Skel {
    #[br(parse_with = parse_ptr64)]
    #[xc3(offset(u64))]
    pub skeleton: Skeleton,
}

// TODO: variable size?
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct Skeleton {
    pub unk1: BcList<u8>,
    pub unk2: u64, // 0

    #[br(parse_with = parse_string_ptr64)]
    #[xc3(offset(u64))]
    pub root_bone_name: String,

    pub parent_indices: BcList<i16>,

    pub names: BcList<BoneName>,

    #[br(restore_position)]
    pub transforms_offset: u32,
    pub transforms: BcList<Transform>,

    pub extra_track_slots: BcList<SkeletonExtraTrackSlot>,

    // MT_ or mount bones?
    pub mt_indices: BcList<[i8; 8]>,
    pub mt_names: BcList<StringOffset>,
    pub mt_transforms: BcList<Transform>,

    pub labels: BcList<SkeletonLabel>,
    // TODO: 80 bytes of optional data not present for xc2?
    // TODO: These may only be pointed to by the offsets at the end of the file?
    // #[br(parse_with = parse_opt_ptr64)]
    // #[xc3(offset(u64))]
    // pub unk6: Option<SkeletonUnk6>,

    // #[br(parse_with = parse_opt_ptr64)]
    // #[xc3(offset(u64))]
    // pub unk7: Option<SkeletonUnk7>,

    // #[br(parse_with = parse_opt_ptr64)]
    // #[xc3(offset(u64))]
    // pub unk8: Option<SkeletonUnk8>,

    // #[br(parse_with = parse_opt_ptr64)]
    // #[xc3(offset(u64))]
    // pub unk9: Option<SkeletonUnk9>,

    // #[br(parse_with = parse_opt_ptr64)]
    // #[xc3(offset(u64))]
    // pub unk10: Option<SkeletonUnk10>,

    // #[br(parse_with = parse_opt_ptr64)]
    // #[xc3(offset(u64))]
    // pub unk11: Option<SkeletonUnk11>,

    // pub unk12: u64,
    // pub unk13: i64,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct SkeletonLabel {
    pub bone_type: u32, // enum?
    pub index: u16,     // incremented if type is the same?
    pub bone_index: u16,
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
    #[xc3(offset(u64))]
    pub name: String,

    // TODO: padding?
    pub unk: [u32; 2],
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct SkeletonExtraTrackSlot {
    #[br(parse_with = parse_string_ptr64)]
    #[xc3(offset(u64))]
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
    #[xc3(offset_count(u64, u32))]
    pub unk3: Vec<u32>,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct SkeletonUnk7 {
    pub unk1: BcList<u8>,
    pub unk2: BcList<u16>,

    // TODO: type?
    #[br(parse_with = parse_offset64_count32)]
    #[xc3(offset_count(u64, u32))]
    pub unk3: Vec<u32>,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct SkeletonUnk8 {
    #[br(parse_with = parse_offset64_count32)]
    #[xc3(offset_count(u64, u32))]
    pub unk1: Vec<u32>,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct SkeletonUnk9 {
    // TODO: type?
    pub unk1: BcList<[u32; 13]>,

    // TODO: type?
    #[br(parse_with = parse_offset64_count32)]
    #[xc3(offset_count(u64, u32))]
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
    #[xc3(offset_count(u64, u32))]
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
    #[xc3(offset_count(u64, u32))]
    pub elements: Vec<T>,

    // TODO: Does this field do anything?
    // #[br(assert(unk1 == -1))]
    pub unk1: i32,
}

xc3_write_binwrite_impl!(AnimationType, BlendMode, PlayMode, SpaceMode);

impl<'a> Xc3WriteOffsets for AnimOffsets<'a> {
    fn write_offsets<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
    ) -> xc3_write::Xc3Result<()> {
        // The binding points backwards to the animation.
        // This means the animation needs to be written first.
        let animation_position = *data_ptr;
        let animation = self.binding.data.animation.xc3_write(writer, data_ptr)?;
        animation
            .data
            .write_offsets(writer, base_offset, data_ptr)?;

        // TODO: Nicer way of writing this?
        let notifies = if !animation.notifies.data.is_empty() {
            Some(
                animation
                    .notifies
                    .write_offset(writer, base_offset, data_ptr)?,
            )
        } else {
            None
        };

        animation
            .locomotion
            .write_full(writer, base_offset, data_ptr)?;

        let binding = self.binding.write_offset(writer, base_offset, data_ptr)?;
        binding
            .bone_track_indices
            .write_offsets(writer, base_offset, data_ptr)?;
        match binding.inner {
            AnimationBindingInnerOffsets::Unk1(unk1) => {
                unk1.write_offsets(writer, base_offset, data_ptr)?
            }
            AnimationBindingInnerOffsets::Unk2(unk2) => {
                unk2.write_offsets(writer, base_offset, data_ptr)?
            }
            AnimationBindingInnerOffsets::Unk3(unk3) => {
                unk3.write_offsets(writer, base_offset, data_ptr)?
            }
        }

        binding.animation.set_offset(writer, animation_position)?;

        // TODO: This should also include the bone names from the extra data?
        // TODO: Create a StringSection that takes Offset<String> references?
        // The names are the last item before the addresses.
        // TODO: Avoid duplicating these strings?
        // TODO: Strings are stored in alphabetical order?
        let mut names = BTreeSet::new();
        names.insert(animation.name.data);
        if let Some(notifies) = &notifies {
            for n in &notifies.0 {
                names.insert(n.unk3.data);
                names.insert(n.unk4.data);
            }
        }
        // TODO: Cleaner way to handle alignment?
        let mut name_to_offset = BTreeMap::new();
        writer.seek(std::io::SeekFrom::Start(*data_ptr))?;
        let aligned = round_up(*data_ptr, 8);
        writer.write_all(&vec![0xff; (aligned - *data_ptr) as usize])?;

        for name in names {
            let offset = writer.stream_position()?;
            writer.write_all(name.as_bytes())?;
            writer.write_all(&[0u8])?;
            name_to_offset.insert(name, offset);
        }
        *data_ptr = (*data_ptr).max(writer.stream_position()?);

        animation
            .name
            .set_offset(writer, name_to_offset[animation.name.data])?;
        if let Some(notifies) = &notifies {
            for n in &notifies.0 {
                n.unk3.set_offset(writer, name_to_offset[n.unk3.data])?;
                n.unk4.set_offset(writer, name_to_offset[n.unk4.data])?;
            }
        }
        Ok(())
    }
}

// TODO: Add a skip(condition) attribute to derive this.
impl<'a> Xc3WriteOffsets for AnimationBindingInner1Offsets<'a> {
    fn write_offsets<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
    ) -> xc3_write::Xc3Result<()> {
        if !self.extra_track_bindings.data.is_empty() {
            self.extra_track_bindings
                .write_full(writer, base_offset, data_ptr)?;
        }
        Ok(())
    }
}

impl<'a> Xc3WriteOffsets for ExtraTrackAnimationBindingOffsets<'a> {
    fn write_offsets<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
    ) -> xc3_write::Xc3Result<()> {
        let animation = self
            .extra_track_animation
            .write_offset(writer, base_offset, data_ptr)?;

        if let Some(animation) = &animation {
            animation
                .values
                .write_offsets(writer, base_offset, data_ptr)?;
        }

        if !self.track_indices.data.is_empty() {
            self.track_indices
                .write_full(writer, base_offset, data_ptr)?;
        }

        // The name needs to be written at the end.
        if let Some(animation) = &animation {
            animation.name.write_full(writer, base_offset, data_ptr)?;
        }

        Ok(())
    }
}
