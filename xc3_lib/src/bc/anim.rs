use crate::{
    parse_offset64_count32, parse_opt_ptr64, parse_ptr64, parse_string_ptr64,
    xc3_write_binwrite_impl,
};
use binrw::{binread, BinRead, BinWrite};
use xc3_write::{Xc3Write, Xc3WriteOffsets};

use super::{BcList, BcList2, StringOffset, StringSection, Transform};

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, PartialEq, Clone)]
#[br(magic(b"ANIM"))]
#[xc3(magic(b"ANIM"))]
pub struct Anim {
    #[br(parse_with = parse_ptr64)]
    #[xc3(offset(u64), align(8, 0xff))]
    pub binding: AnimationBinding,
}

#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, PartialEq, Clone)]
#[br(stream = r)]
pub struct AnimationBinding {
    // Use temp fields to estimate the struct size.
    // These fields will be skipped when writing.
    // TODO: is there a better way to handle game specific differences?
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

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
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, PartialEq, Clone)]
#[br(import { size: u64, animation_type: AnimationType })]
pub enum AnimationBindingInner {
    // XC2 has 60 total bytes.
    #[br(pre_assert(size == 60))]
    Unk1(AnimationBindingInner1),

    // XC1 and XC3 have 76 total bytes.
    #[br(pre_assert(size == 76))]
    Unk2(AnimationBindingInner2),

    // XC3 sometimes has 120 or 128 total bytes.
    #[br(pre_assert(size == 120))]
    Unk3(#[br(args_raw(animation_type))] AnimationBindingInner3),

    #[br(pre_assert(size == 128))]
    Unk4(#[br(args_raw(animation_type))] AnimationBindingInner4),
}

// 60 total bytes for xc2
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, PartialEq, Clone)]
pub struct AnimationBindingInner1 {
    #[br(parse_with = parse_offset64_count32)]
    #[xc3(offset_count(u64, u32), align(8, 0xff))]
    pub extra_track_bindings: Vec<ExtraTrackAnimationBinding>,
}

// TODO: Is this always used for morph targets?
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, PartialEq, Clone)]
pub struct ExtraTrackAnimationBinding {
    #[br(parse_with = parse_opt_ptr64)]
    #[xc3(offset(u64))]
    pub extra_track_animation: Option<ExtraTrackAnimation>,

    // TODO: Same count as ModelUnk1 items?
    // TODO: Assigns values in extra_track_animation to ModelUnk1Items?

    // TODO: Should this be ignored if extra_track_animation is None?
    pub track_indices: BcList2<i16>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, PartialEq, Clone)]
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
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, PartialEq, Clone)]
pub struct AnimationBindingInner2 {
    /// An alternative bone name list for
    /// [bone_track_indices](struct.AnimationBinding.html#structfield.bone_track_indices).
    #[br(parse_with = parse_offset64_count32)]
    #[xc3(offset_count(u64, u32), align(8, 0xff))]
    pub bone_names: Vec<StringOffset>,
    pub unk2: i32,

    #[br(parse_with = parse_offset64_count32)]
    #[xc3(offset_count(u64, u32))]
    pub extra_track_bindings: Vec<ExtraTrackAnimationBinding>,
}

// 120 or 128 total bytes for xc3
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, PartialEq, Clone)]
#[br(import_raw(animation_type: AnimationType))]
pub struct AnimationBindingInner3 {
    /// An alternative bone name list for
    /// [bone_track_indices](struct.AnimationBinding.html#structfield.bone_track_indices).
    #[br(parse_with = parse_offset64_count32)]
    #[xc3(offset_count(u64, u32), align(8, 0xff))]
    pub bone_names: Vec<StringOffset>,
    pub unk2: i32,

    #[br(args_raw(animation_type))]
    pub extra_track_data: ExtraTrackData,
}

// 128 total bytes for xc3
// TODO: Is it worth making a whole separate type for this?
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, PartialEq, Clone)]
#[br(import_raw(animation_type: AnimationType))]
pub struct AnimationBindingInner4 {
    /// An alternative bone name list for
    /// [bone_track_indices](struct.AnimationBinding.html#structfield.bone_track_indices).
    #[br(parse_with = parse_offset64_count32)]
    #[xc3(offset_count(u64, u32), align(8, 0xff))]
    pub bone_names: Vec<StringOffset>,
    pub unk2: i32,

    #[br(args_raw(animation_type))]
    pub extra_track_data: ExtraTrackData,

    pub unk1: u64,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, PartialEq, Clone)]
pub struct Animation {
    pub unk1: BcList<()>,
    pub unk_offset1: u64,

    #[br(parse_with = parse_string_ptr64)]
    #[xc3(offset(u64), align(8, 0xff))]
    pub name: String,

    pub animation_type: AnimationType,
    /// The space for transforms in [data](#structfield.data).
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
    pub unk2: i32, // -1

    #[br(parse_with = parse_opt_ptr64)]
    #[xc3(offset(u64), align(16, 0xff))]
    pub locomotion: Option<AnimationLocomotion>,

    #[br(args { animation_type })]
    pub data: AnimationData,
}

/// The space for how to convert transforms to model space. Usually [SpaceMode::Local].
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, BinWrite, PartialEq, Eq, Clone, Copy)]
#[brw(repr(u8))]
pub enum SpaceMode {
    /// Transforms are relative to the parent bone's accumulated transform.
    /// ```text
    /// model_transform = parent.model_transform * transform;
    /// ```
    Local = 0,
    /// Transforms are the accumulated transform.
    /// This avoids needing to accumulate transform matrices recursively for animated bones.
    /// ```text
    /// model_transform = transform;
    /// ```
    Model = 1,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, BinWrite, PartialEq, Eq, Clone, Copy)]
#[brw(repr(u8))]
pub enum PlayMode {
    Loop = 0,
    Single = 1,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, BinWrite, PartialEq, Eq, Clone, Copy)]
#[brw(repr(u8))]
pub enum BlendMode {
    Blend = 0,
    Add = 1,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
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

/// Animation for the root bone.
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct AnimationLocomotion {
    pub unk1: [u32; 4],
    pub seconds_per_frame: f32,
    pub unk2: i32,

    #[br(parse_with = parse_offset64_count32)]
    #[xc3(offset_count(u64, u32), align(16, 0xff))]
    pub translation: Vec<[f32; 4]>,
}

// TODO: is this only for XC3?
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
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

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, PartialEq, Clone)]
pub struct UncompressedExtraData {
    // TODO: type?
    #[br(parse_with = parse_offset64_count32)]
    #[xc3(offset_count(u64, u32), align(8, 0xff))]
    pub unk1: Vec<u8>,
    pub unk2: i32, // -1

    #[br(parse_with = parse_opt_ptr64)]
    #[xc3(offset(u64))]
    pub motion: Option<UncompressedExtraDataMotion>,

    #[br(parse_with = parse_ptr64)]
    #[xc3(offset(u64), align(8, 0xff))]
    pub hashes: TrackHashes,

    pub unk4: [u32; 6],
}

// TODO: Default transform for a single bone at each frame?
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
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

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, PartialEq, Clone)]
pub struct CubicExtraData {
    // TODO: type?
    #[br(parse_with = parse_offset64_count32)]
    #[xc3(offset_count(u64, u32), align(8, 0xff))]
    pub unk1: Vec<u8>,
    pub unk2: i32, // -1

    // TODO: root motion?
    pub unk4: u64, // TODO: always 0?

    #[br(parse_with = parse_ptr64)]
    #[xc3(offset(u64), align(8, 0xff))]
    pub data1: CubicExtraDataInner1,

    #[br(parse_with = parse_ptr64)]
    #[xc3(offset(u64), align(8, 0xff))]
    pub data2: CubicExtraDataInner2,

    // TODO: padding?
    pub unk: [u32; 4],
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct CubicExtraDataInner1 {
    // TODO: buffer?
    pub unk1: BcList<u8>,

    #[br(parse_with = parse_offset64_count32)]
    #[xc3(offset_count(u64, u32), align(8, 0xff))]
    pub unk2: Vec<u16>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct CubicExtraDataInner2 {
    pub unk1: BcList<u8>,

    #[br(parse_with = parse_offset64_count32)]
    #[xc3(offset_count(u64, u32), align(8, 0xff))]
    pub unk2: Vec<u16>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, PartialEq, Clone)]
pub struct PackedCubicExtraData {
    #[br(parse_with = parse_offset64_count32)]
    #[xc3(offset_count(u64, u32), align(8))]
    pub extra_track_bindings: Vec<ExtraTrackAnimationBinding>,
    pub unk2: i32, // -1

    pub unk6: u32,
    pub unk7: u32,

    #[br(parse_with = parse_ptr64)]
    #[xc3(offset(u64), align(8, 0xff))]
    pub hashes: TrackHashes,

    #[br(parse_with = parse_opt_ptr64)]
    #[xc3(offset(u64), align(8, 0xff))]
    pub unk_offset1: Option<PackedCubicExtraDataUnk1>,

    #[br(parse_with = parse_opt_ptr64)]
    #[xc3(offset(u64), align(8, 0xff))]
    pub unk_offset2: Option<PackedCubicExtraDataUnk2>,

    #[br(parse_with = parse_opt_ptr64)]
    #[xc3(offset(u64), align(8, 0xff))]
    pub unk_offset3: Option<PackedCubicExtraDataUnk3>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct PackedCubicExtraDataUnk1 {
    pub unk1: BcList<u8>,
    pub unk2: BcList<u16>,
    #[br(parse_with = parse_offset64_count32)]
    #[xc3(offset_count(u64, u32))]
    pub unk3: Vec<u32>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct PackedCubicExtraDataUnk2 {
    #[br(parse_with = parse_offset64_count32)]
    #[xc3(offset_count(u64, u32))]
    pub items: Vec<f32>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct PackedCubicExtraDataUnk3 {
    #[br(parse_with = parse_offset64_count32)]
    #[xc3(offset_count(u64, u32), align(16, 0xff))]
    pub items1: Vec<[f32; 4]>,
    pub unk1: i32, // -1

    #[br(parse_with = parse_offset64_count32)]
    #[xc3(offset_count(u64, u32))]
    pub items2: Vec<i16>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct TrackHashes {
    // TODO: buffers?
    pub unk1: BcList<u8>,

    #[br(parse_with = parse_offset64_count32)]
    #[xc3(offset_count(u64, u32), align(8, 0xff))]
    pub unk2: Vec<u16>,
    pub unk3: i32, // -1

    /// Hash of bone names using [murmur3](crate::hash::murmur3).
    // TODO: type alias for hash?
    #[br(parse_with = parse_offset64_count32)]
    #[xc3(offset_count(u64, u32), align(8, 0xff))]
    pub bone_name_hashes: Vec<u32>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, BinWrite, PartialEq, Eq, Clone, Copy)]
#[brw(repr(u8))]
pub enum AnimationType {
    Uncompressed = 0,
    Cubic = 1,
    Empty = 2,
    PackedCubic = 3,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
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

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct Uncompressed {
    // TODO: Is every BcList aligned like this?
    #[br(parse_with = parse_offset64_count32)]
    #[xc3(offset_count(u64, u32), align(16, 0xff))]
    pub transforms: Vec<Transform>,
    pub unk1: i32, // -1
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct Cubic {
    pub tracks: BcList<CubicTrack>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct CubicTrack {
    pub translation: BcList<KeyFrameCubicVec3>,
    pub rotation: BcList<KeyFrameCubicQuaternion>,
    pub scale: BcList<KeyFrameCubicVec3>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct KeyFrameCubicVec3 {
    pub frame: f32,
    /// Coefficients `[a,b,c,d]` for `a*x^3 + b*x^2 + c*x + d` for frame index `x`.
    pub x: [f32; 4],
    /// Coefficients `[a,b,c,d]` for `a*x^3 + b*x^2 + c*x + d` for frame index `x`.
    pub y: [f32; 4],
    /// Coefficients `[a,b,c,d]` for `a*x^3 + b*x^2 + c*x + d` for frame index `x`.
    pub z: [f32; 4],
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
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

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct PackedCubic {
    // TODO: same length and ordering as bone indices and hashes?
    pub tracks: BcList<PackedCubicTrack>,

    /// Coefficients `[a,b,c,d]` for `a*x^3 + b*x^2 + c*x + d` for frame index `x`.
    #[br(parse_with = parse_offset64_count32)]
    #[xc3(offset_count(u64, u32), align(16, 0xff))]
    pub vectors: Vec<[f32; 4]>,
    pub unk1: i32, // -1

    /// Coefficients `[a,b,c,d]` for `a*x^3 + b*x^2 + c*x + d` for frame index `x`.
    pub quaternions: BcList<[f32; 4]>,

    // TODO: Are these keyframe times?
    pub keyframes: BcList<u16>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct PackedCubicTrack {
    pub translation: SubTrack,
    pub rotation: SubTrack,
    pub scale: SubTrack,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct SubTrack {
    /// Index into [keyframes](struct.PackedCubic.html#structfield.keyframes).
    pub keyframe_start_index: u32,
    /// Index into [vectors](struct.PackedCubic.html#structfield.vectors)
    /// or [quaternions](struct.PackedCubic.html#structfield.quaternions).
    pub curves_start_index: u32,
    /// Index into [keyframes](struct.PackedCubic.html#structfield.keyframes).
    pub keyframe_end_index: u32,
}

xc3_write_binwrite_impl!(AnimationType, BlendMode, PlayMode, SpaceMode);

impl<'a> Xc3WriteOffsets for AnimOffsets<'a> {
    type Args = ();

    fn write_offsets<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
        endian: xc3_write::Endian,
        args: Self::Args,
    ) -> xc3_write::Xc3Result<()> {
        // The binding points backwards to the animation.
        // This means the animation needs to be written first.
        let animation_position = *data_ptr;
        let animation = self.binding.data.animation.xc3_write(writer, endian)?;
        animation
            .data
            .write_offsets(writer, base_offset, data_ptr, endian, ())?;

        // TODO: Nicer way of writing this?
        let notifies = if !animation.notifies.data.is_empty() {
            Some(
                animation
                    .notifies
                    .write(writer, base_offset, data_ptr, endian)?,
            )
        } else {
            None
        };

        animation
            .locomotion
            .write_full(writer, base_offset, data_ptr, endian, ())?;

        let binding = self.binding.write(writer, base_offset, data_ptr, endian)?;

        binding
            .animation
            .set_offset(writer, animation_position, endian)?;

        binding
            .bone_track_indices
            .write_offsets(writer, base_offset, data_ptr, endian, ())?;

        // The names are stored in a single section for XC1 and XC3.
        let mut string_section = StringSection::default();

        match &binding.inner {
            AnimationBindingInnerOffsets::Unk1(unk1) => {
                unk1.write_offsets(writer, base_offset, data_ptr, endian, &mut string_section)?;
            }
            AnimationBindingInnerOffsets::Unk2(unk2) => {
                unk2.write_offsets(writer, base_offset, data_ptr, endian, &mut string_section)?;
            }
            AnimationBindingInnerOffsets::Unk3(unk3) => {
                unk3.write_offsets(writer, base_offset, data_ptr, endian, &mut string_section)?;
            }
            AnimationBindingInnerOffsets::Unk4(unk4) => {
                unk4.write_offsets(writer, base_offset, data_ptr, endian, &mut string_section)?;
            }
        }

        string_section.insert_offset(&animation.name);
        if let Some(notifies) = &notifies {
            for n in &notifies.0 {
                string_section.insert_offset(&n.unk3);
                string_section.insert_offset(&n.unk4);
            }
        }

        // The names are the last item before the addresses.
        string_section.write(writer, data_ptr, 8, endian)?;

        Ok(())
    }
}

// TODO: Add a skip(condition) attribute to derive this.
impl<'a> Xc3WriteOffsets for AnimationBindingInner1Offsets<'a> {
    type Args = &'a mut StringSection;

    fn write_offsets<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
        endian: xc3_write::Endian,
        args: Self::Args,
    ) -> xc3_write::Xc3Result<()> {
        if !self.extra_track_bindings.data.is_empty() {
            // TODO: Find a way to support write_full with vec args.
            // self.extra_track_bindings
            //     .write_full(writer, base_offset, data_ptr, endian, ())?;
        }
        Ok(())
    }
}

impl<'a> Xc3WriteOffsets for AnimationBindingInner2Offsets<'a> {
    type Args = &'a mut StringSection;

    fn write_offsets<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
        endian: xc3_write::Endian,
        args: Self::Args,
    ) -> xc3_write::Xc3Result<()> {
        let bone_names = self
            .bone_names
            .write(writer, base_offset, data_ptr, endian)?;
        for bone_name in &bone_names.0 {
            args.insert_offset(&bone_name.name);
        }

        if !self.extra_track_bindings.data.is_empty() {
            let items = self
                .extra_track_bindings
                .write(writer, base_offset, data_ptr, endian)?;

            for item in &items.0 {
                let extra =
                    item.extra_track_animation
                        .write(writer, base_offset, data_ptr, endian)?;
                if let Some(extra) = extra {
                    extra
                        .values
                        .write_offsets(writer, base_offset, data_ptr, endian, ())?;
                    args.insert_offset(&extra.name);
                }

                item.track_indices
                    .write_offsets(writer, base_offset, data_ptr, endian, ())?;
            }
        }
        Ok(())
    }
}

impl<'a> Xc3WriteOffsets for AnimationBindingInner3Offsets<'a> {
    type Args = &'a mut StringSection;

    fn write_offsets<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
        endian: xc3_write::Endian,
        args: Self::Args,
    ) -> xc3_write::Xc3Result<()> {
        if !self.bone_names.data.is_empty() {
            let bone_names = self
                .bone_names
                .write(writer, base_offset, data_ptr, endian)?;
            for bone_name in &bone_names.0 {
                args.insert_offset(&bone_name.name);
            }
        }

        self.extra_track_data
            .write_offsets(writer, base_offset, data_ptr, endian, ())?;
        Ok(())
    }
}

impl<'a> Xc3WriteOffsets for AnimationBindingInner4Offsets<'a> {
    type Args = &'a mut StringSection;

    fn write_offsets<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
        endian: xc3_write::Endian,
        args: Self::Args,
    ) -> xc3_write::Xc3Result<()> {
        if !self.bone_names.data.is_empty() {
            let bone_names = self
                .bone_names
                .write(writer, base_offset, data_ptr, endian)?;
            for bone_name in &bone_names.0 {
                args.insert_offset(&bone_name.name);
            }
        }

        self.extra_track_data
            .write_offsets(writer, base_offset, data_ptr, endian, ())?;
        Ok(())
    }
}

impl<'a> Xc3WriteOffsets for ExtraTrackAnimationBindingOffsets<'a> {
    type Args = &'a mut StringSection;

    fn write_offsets<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
        endian: xc3_write::Endian,
        args: Self::Args,
    ) -> xc3_write::Xc3Result<()> {
        let animation = self
            .extra_track_animation
            .write(writer, base_offset, data_ptr, endian)?;

        if let Some(animation) = &animation {
            animation
                .values
                .write_offsets(writer, base_offset, data_ptr, endian, ())?;

            args.insert_offset(&animation.name);
        }

        self.track_indices
            .write_offsets(writer, base_offset, data_ptr, endian, ())?;

        Ok(())
    }
}

impl<'a> Xc3WriteOffsets for PackedCubicExtraDataOffsets<'a> {
    type Args = ();

    fn write_offsets<W: std::io::prelude::Write + std::io::prelude::Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
        endian: xc3_write::Endian,
        args: Self::Args,
    ) -> xc3_write::Xc3Result<()> {
        // Different order than field order.
        self.hashes
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.unk_offset1
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.unk_offset2
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.unk_offset3
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        // self.extra_track_bindings
        //     .write_full(writer, base_offset, data_ptr, endian, ())?;
        Ok(())
    }
}

impl<'a> Xc3WriteOffsets for UncompressedExtraDataOffsets<'a> {
    type Args = ();

    fn write_offsets<W: std::io::prelude::Write + std::io::prelude::Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
        endian: xc3_write::Endian,
        args: Self::Args,
    ) -> xc3_write::Xc3Result<()> {
        // Different order than field order.
        self.motion
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.hashes
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.unk1
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        Ok(())
    }
}

impl<'a> Xc3WriteOffsets for CubicExtraDataOffsets<'a> {
    type Args = ();

    fn write_offsets<W: std::io::prelude::Write + std::io::prelude::Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
        endian: xc3_write::Endian,
        args: Self::Args,
    ) -> xc3_write::Xc3Result<()> {
        // Different order than field order.
        self.data1
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.data2
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.unk1
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        Ok(())
    }
}
