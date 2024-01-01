use crate::{
    parse_offset64_count32, parse_opt_ptr64, parse_ptr64, parse_string_ptr64,
    xc3_write_binwrite_impl,
};
use binrw::{binread, BinRead, BinWrite};
use xc3_write::{Xc3Write, Xc3WriteOffsets};

use super::{BcList, StringOffset, StringSection, Transform};

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
#[derive(Debug, BinRead, Xc3Write)]
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
    #[br(parse_with = parse_offset64_count32_unchecked)]
    #[xc3(offset_count(u64, u32), align(8, 0xff))]
    pub track_indices: Vec<i16>,
    pub unk1: i32, // -1
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
#[derive(Debug, BinRead, Xc3Write)]
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
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
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
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
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
    pub unk2: i32, // -1

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

#[derive(Debug, BinRead, Xc3Write)]
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

#[derive(Debug, BinRead, Xc3Write)]
pub struct CubicExtraData {
    // TODO: type?
    #[br(parse_with = parse_offset64_count32)]
    #[xc3(offset_count(u64, u32), align(8, 0xff))]
    pub unk1: Vec<u8>,
    pub unk2: i32, // -1

    // TODO: root motion?
    pub unk4: u64,

    #[br(parse_with = parse_ptr64)]
    #[xc3(offset(u64), align(8, 0xff))]
    pub data1: CubicExtraDataInner1,

    #[br(parse_with = parse_ptr64)]
    #[xc3(offset(u64), align(8, 0xff))]
    pub data2: CubicExtraDataInner2,

    // TODO: padding?
    pub unk: [u32; 4],
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct CubicExtraDataInner1 {
    // TODO: buffer?
    pub unk1: BcList<u8>,

    #[br(parse_with = parse_offset64_count32)]
    #[xc3(offset_count(u64, u32), align(8, 0xff))]
    pub unk2: Vec<u16>,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct CubicExtraDataInner2 {
    pub unk1: BcList<u8>,

    #[br(parse_with = parse_offset64_count32)]
    #[xc3(offset_count(u64, u32), align(8, 0xff))]
    pub unk2: Vec<u16>,
}

#[derive(Debug, BinRead, Xc3Write)]
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

    pub unk_offset1: u64,

    #[br(parse_with = parse_opt_ptr64)]
    #[xc3(offset(u64), align(8, 0xff))]
    pub unk_offset2: Option<PackedCubicExtraDataUnk2>,

    #[br(parse_with = parse_opt_ptr64)]
    #[xc3(offset(u64), align(8, 0xff))]
    pub unk_offset3: Option<PackedCubicExtraDataUnk3>,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct PackedCubicExtraDataUnk2 {
    #[br(parse_with = parse_offset64_count32)]
    #[xc3(offset_count(u64, u32))]
    pub items: Vec<f32>,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct PackedCubicExtraDataUnk3 {
    #[br(parse_with = parse_offset64_count32)]
    #[xc3(offset_count(u64, u32), align(16, 0xff))]
    pub items1: Vec<[f32; 4]>,
    pub unk1: i32, // -1

    #[br(parse_with = parse_offset64_count32)]
    #[xc3(offset_count(u64, u32))]
    pub items2: Vec<i16>,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
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
    pub unk1: i32, // -1
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
    #[br(parse_with = parse_offset64_count32)]
    #[xc3(offset_count(u64, u32), align(16, 0xff))]
    pub vectors: Vec<[f32; 4]>,
    pub unk1: i32, // -1

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

fn parse_offset64_count32_unchecked<T, R, Args>(
    reader: &mut R,
    endian: binrw::Endian,
    args: binrw::file_ptr::FilePtrArgs<Args>,
) -> binrw::BinResult<Vec<T>>
where
    for<'a> T: BinRead<Args<'a> = Args> + 'static,
    R: std::io::Read + std::io::Seek,
    Args: Clone,
{
    let offset = u64::read_options(reader, endian, ())?;
    let count = u32::read_options(reader, endian, ())?;

    crate::parse_vec(reader, endian, args, offset, count as usize)
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

        binding.animation.set_offset(writer, animation_position)?;

        binding
            .bone_track_indices
            .write_offsets(writer, base_offset, data_ptr)?;

        // The names are stored in a single section for XC1 and XC3.
        let mut string_section = StringSection::default();

        match &binding.inner {
            AnimationBindingInnerOffsets::Unk1(unk1) => {
                unk1.write_offsets(writer, base_offset, data_ptr)?;
            }
            AnimationBindingInnerOffsets::Unk2(unk2) => {
                let bone_names = unk2
                    .bone_names
                    .write_offset(writer, base_offset, data_ptr)?;
                for bone_name in &bone_names.0 {
                    string_section.insert_offset(&bone_name.name);
                }

                if !unk2.extra_track_bindings.data.is_empty() {
                    let items =
                        unk2.extra_track_bindings
                            .write_offset(writer, base_offset, data_ptr)?;

                    for item in &items.0 {
                        let extra = item.extra_track_animation.write_offset(
                            writer,
                            base_offset,
                            data_ptr,
                        )?;
                        if let Some(extra) = extra {
                            extra.values.write_offsets(writer, base_offset, data_ptr)?;
                            string_section.insert_offset(&extra.name);
                        }

                        item.track_indices
                            .write_full(writer, base_offset, data_ptr)?;
                    }
                }
            }
            AnimationBindingInnerOffsets::Unk3(unk3) => {
                if !unk3.bone_names.data.is_empty() {
                    let bone_names = unk3
                        .bone_names
                        .write_offset(writer, base_offset, data_ptr)?;
                    for bone_name in &bone_names.0 {
                        string_section.insert_offset(&bone_name.name);
                    }
                }

                unk3.extra_track_data
                    .write_offsets(writer, base_offset, data_ptr)?;
            }
            AnimationBindingInnerOffsets::Unk4(unk4) => {
                if !unk4.bone_names.data.is_empty() {
                    let bone_names = unk4
                        .bone_names
                        .write_offset(writer, base_offset, data_ptr)?;
                    for bone_name in &bone_names.0 {
                        string_section.insert_offset(&bone_name.name);
                    }
                }

                unk4.extra_track_data
                    .write_offsets(writer, base_offset, data_ptr)?;
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
        string_section.write(writer, data_ptr, 8)?;

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

impl<'a> Xc3WriteOffsets for PackedCubicExtraDataOffsets<'a> {
    fn write_offsets<W: std::io::prelude::Write + std::io::prelude::Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
    ) -> xc3_write::Xc3Result<()> {
        // Different order than field order.
        self.hashes.write_full(writer, base_offset, data_ptr)?;
        self.unk_offset2.write_full(writer, base_offset, data_ptr)?;
        self.unk_offset3.write_full(writer, base_offset, data_ptr)?;
        self.extra_track_bindings
            .write_full(writer, base_offset, data_ptr)?;
        Ok(())
    }
}

impl<'a> Xc3WriteOffsets for UncompressedExtraDataOffsets<'a> {
    fn write_offsets<W: std::io::prelude::Write + std::io::prelude::Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
    ) -> xc3_write::Xc3Result<()> {
        // Different order than field order.
        self.motion.write_full(writer, base_offset, data_ptr)?;
        self.hashes.write_full(writer, base_offset, data_ptr)?;
        self.unk1.write_full(writer, base_offset, data_ptr)?;
        Ok(())
    }
}

impl<'a> Xc3WriteOffsets for CubicExtraDataOffsets<'a> {
    fn write_offsets<W: std::io::prelude::Write + std::io::prelude::Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
    ) -> xc3_write::Xc3Result<()> {
        // Different order than field order.
        self.data1.write_full(writer, base_offset, data_ptr)?;
        self.data2.write_full(writer, base_offset, data_ptr)?;
        self.unk1.write_full(writer, base_offset, data_ptr)?;
        Ok(())
    }
}
