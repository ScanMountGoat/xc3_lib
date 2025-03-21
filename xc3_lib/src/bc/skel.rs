use crate::{align, parse_offset64_count32, parse_opt_ptr64, parse_ptr64, parse_string_ptr64};
use binrw::{binread, BinRead};
use xc3_write::{
    strings::{StringSectionUniqueSorted, WriteOptions},
    Xc3Write, Xc3WriteOffsets,
};

use super::{BcList, StringOffset, Transform};

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(magic(b"SKEL"))]
#[xc3(magic(b"SKEL"))]
pub struct Skel {
    #[br(parse_with = parse_ptr64)]
    #[xc3(offset(u64))]
    pub skeleton: Skeleton,
}

// TODO: variable size?
// 160, 192, 224, 240

#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, PartialEq, Clone)]
#[br(stream = r)]
pub struct Skeleton {
    // Use temp fields to estimate the struct size.
    // These fields will be skipped when writing.
    // TODO: is there a better way to handle game specific differences?
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    pub unk1: BcList<u8>,
    pub unk2: u64, // 0

    #[br(parse_with = parse_string_ptr64)]
    #[xc3(offset(u64))]
    pub root_bone_name: String,

    pub parent_indices: BcList<i16>,

    pub names: BcList<BoneName>,

    // Store the offset for the next field.
    #[br(temp, restore_position)]
    transforms_offset: u32,

    #[br(parse_with = parse_offset64_count32)]
    #[xc3(offset_count(u64, u32), align(16, 0xff))]
    pub transforms: Vec<Transform>,
    pub unk3: i32, // -1

    #[br(parse_with = parse_offset64_count32)]
    #[xc3(offset_count(u64, u32), align(8, 0xff))]
    pub extra_track_slots: Vec<SkeletonExtraTrackSlot>,
    pub unk4: i32, // -1

    // MT_ or mount bones?
    #[br(parse_with = parse_offset64_count32)]
    #[xc3(offset_count(u64, u32), align(8, 0xff))]
    pub mt_parent_indices: Vec<i16>,
    pub unk5: i32, // -1

    #[br(parse_with = parse_offset64_count32)]
    #[xc3(offset_count(u64, u32), align(8, 0xff))]
    pub mt_names: Vec<StringOffset>,
    pub unk6: i32, // -1

    #[br(parse_with = parse_offset64_count32)]
    #[xc3(offset_count(u64, u32), align(16, 0xff))]
    pub mt_transforms: Vec<Transform>,
    pub unk7: i32, // -1

    pub labels: BcList<SkeletonLabel>,

    #[br(args_raw(transforms_offset as u64 - base_offset))]
    pub extra: SkeletonExtra,
}

// TODO: Make this an option instead?

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))] // Up to 80 bytes of optional data for XC3.
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(size: u64))]
pub enum SkeletonExtra {
    #[br(pre_assert(size == 160))]
    Unk0,

    #[br(pre_assert(size == 192))]
    Unk1(SkeletonExtraUnk1),

    #[br(pre_assert(size == 224))]
    Unk2(SkeletonExtraUnk2),

    #[br(pre_assert(size == 240))]
    Unk3(SkeletonExtraUnk3),

    // TODO: extra variant?
    Unk,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))] // TODO: Fix writing.
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct SkeletonExtraUnk1 {
    #[br(parse_with = parse_opt_ptr64)]
    #[xc3(offset(u64), align(16, 0xff))]
    pub unk6: Option<SkeletonUnk6Unk1>,

    #[br(parse_with = parse_opt_ptr64)]
    #[xc3(offset(u64), align(16, 0xff))]
    pub unk7: Option<SkeletonUnk7>,

    #[br(parse_with = parse_opt_ptr64)]
    #[xc3(offset(u64), align(16, 0xff))]
    pub unk8: Option<SkeletonUnk8>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct SkeletonExtraUnk2 {
    #[br(parse_with = parse_opt_ptr64)]
    #[xc3(offset(u64), align(16, 0xff))]
    pub unk6: Option<SkeletonUnk6>,

    #[br(parse_with = parse_opt_ptr64)]
    #[xc3(offset(u64), align(16, 0xff))]
    pub unk7: Option<SkeletonUnk7>,

    #[br(parse_with = parse_opt_ptr64)]
    #[xc3(offset(u64), align(16, 0xff))]
    pub unk8: Option<SkeletonUnk8>,

    #[br(parse_with = parse_opt_ptr64)]
    #[xc3(offset(u64), align(16, 0xff))]
    pub unk9: Option<SkeletonUnk9>,

    #[br(parse_with = parse_opt_ptr64)]
    #[xc3(offset(u64), align(16, 0xff))]
    pub unk10: Option<SkeletonUnk10>,

    #[br(parse_with = parse_opt_ptr64)]
    #[xc3(offset(u64), align(16, 0xff))]
    pub unk11: Option<SkeletonUnk11>,

    pub unk2: u64,
    pub unk3: i64,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, PartialEq, Clone)]
pub struct SkeletonExtraUnk3 {
    #[br(parse_with = parse_opt_ptr64)]
    #[xc3(offset(u64), align(16, 0xff))]
    pub unk6: Option<SkeletonUnk6>,

    #[br(parse_with = parse_opt_ptr64)]
    #[xc3(offset(u64), align(16, 0xff))]
    pub unk7: Option<SkeletonUnk7>,

    #[br(parse_with = parse_opt_ptr64)]
    #[xc3(offset(u64), align(16, 0xff))]
    pub unk8: Option<SkeletonUnk8>,

    #[br(parse_with = parse_opt_ptr64)]
    #[xc3(offset(u64), align(16, 0xff))]
    pub unk9: Option<SkeletonUnk9>,

    #[br(parse_with = parse_opt_ptr64)]
    #[xc3(offset(u64), align(16, 0xff))]
    pub unk10: Option<SkeletonUnk10>,

    #[br(parse_with = parse_opt_ptr64)]
    #[xc3(offset(u64), align(16, 0xff))]
    pub unk11: Option<SkeletonUnk11>,

    #[br(parse_with = parse_opt_ptr64)]
    #[xc3(offset(u64), align(8, 0xff))]
    pub unk12: Option<SkeletonUnk12>,

    #[br(parse_with = parse_opt_ptr64)]
    #[xc3(offset(u64), align(8, 0xff))]
    pub unk13: Option<SkeletonUnk13>,

    pub unk2: u64,
    pub unk3: i64,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct SkeletonLabel {
    pub bone_type: u32, // enum?
    pub index: u16,     // incremented if type is the same?
    pub bone_index: u16,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct BoneName {
    #[br(parse_with = parse_string_ptr64)]
    #[xc3(offset(u64))]
    pub name: String,

    // TODO: padding?
    pub unk: [u32; 2],
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct SkeletonExtraTrackSlot {
    #[br(parse_with = parse_string_ptr64)]
    #[xc3(offset(u64))]
    pub unk1: String,

    pub unk2: BcList<StringOffset>,

    pub unk3: BcList<f32>,

    #[br(parse_with = parse_offset64_count32)]
    #[xc3(offset_count(u64, u32), align(8, 0xff))]
    pub unk4: Vec<[f32; 2]>,
    pub unk1_1: i32, // -1
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct SkeletonUnk6 {
    pub unk1: BcList<u8>,

    #[br(parse_with = parse_offset64_count32)]
    #[xc3(offset_count(u64, u32), align(4, 0xff))]
    pub unk2: Vec<u16>,
    pub unk2_1: i32, // -1

    #[br(parse_with = parse_offset64_count32)]
    #[xc3(offset_count(u64, u32), align(8, 0xff))]
    pub unk3: Vec<u32>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct SkeletonUnk6Unk1 {
    pub unk1: BcList<u8>,

    #[br(parse_with = parse_offset64_count32)]
    #[xc3(offset_count(u64, u32), align(4, 0xff))]
    pub unk2: Vec<u16>,
    pub unk2_1: i32, // -1
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct SkeletonUnk7 {
    pub unk1: BcList<u8>,

    #[br(parse_with = parse_offset64_count32)]
    #[xc3(offset_count(u64, u32), align(4, 0xff))]
    pub unk2: Vec<u16>,
    pub unk2_1: i32, // -1

    // TODO: type?
    #[br(parse_with = parse_offset64_count32)]
    #[xc3(offset_count(u64, u32), align(8, 0xff))]
    pub unk3: Vec<u32>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct SkeletonUnk8 {
    #[br(parse_with = parse_offset64_count32)]
    #[xc3(offset_count(u64, u32))]
    pub unk1: Vec<u32>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct SkeletonUnk9 {
    // TODO: type?
    pub unk1: BcList<[u32; 13]>,

    // TODO: type?
    #[br(parse_with = parse_offset64_count32)]
    #[xc3(offset_count(u64, u32))]
    pub unk2: Vec<u64>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct SkeletonUnk10 {
    // TODO: type?
    pub unk1: [u32; 8],
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct SkeletonUnk11 {
    #[br(parse_with = parse_offset64_count32)]
    #[xc3(offset_count(u64, u32))]
    pub unk1: Vec<u8>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct SkeletonUnk12 {
    #[br(parse_with = parse_offset64_count32)]
    #[xc3(offset_count(u64, u32))]
    pub unk1: Vec<u16>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct SkeletonUnk13 {
    pub unk1: BcList<[f32; 4]>,
    pub unk2: BcList<i16>,
}

impl Xc3WriteOffsets for SkeletonOffsets<'_> {
    type Args = ();

    fn write_offsets<W: std::io::prelude::Write + std::io::prelude::Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
        endian: xc3_write::Endian,
        _args: Self::Args,
    ) -> xc3_write::Xc3Result<()> {
        // The names are stored in a single section.
        let mut string_section = StringSectionUniqueSorted::default();
        string_section.insert_offset64(&self.root_bone_name);

        // Different order than field order.
        if !self.unk1.0.data.is_empty() {
            self.unk1
                .write_offsets(writer, base_offset, data_ptr, endian, ())?;
        }
        self.transforms
            .write_full(writer, base_offset, data_ptr, endian, ())?;

        let names = self.names.0.write(writer, base_offset, data_ptr, endian)?;
        for name in names.0 {
            string_section.insert_offset64(&name.name);
        }

        self.parent_indices
            .write_offsets(writer, base_offset, data_ptr, endian, ())?;

        if !self.extra_track_slots.data.is_empty() {
            let slots = self
                .extra_track_slots
                .write(writer, base_offset, data_ptr, endian)?;
            for slot in slots.0 {
                string_section.insert_offset64(&slot.unk1);

                if !slot.unk2.0.data.is_empty() {
                    let names = slot.unk2.0.write(writer, base_offset, data_ptr, endian)?;
                    for name in names.0 {
                        string_section.insert_offset64(&name.name);
                    }
                }

                if !slot.unk3.0.data.is_empty() {
                    slot.unk3
                        .write_offsets(writer, base_offset, data_ptr, endian, ())?;
                }
                if !slot.unk4.data.is_empty() {
                    slot.unk4
                        .write_full(writer, base_offset, data_ptr, endian, ())?;
                }
            }
        }

        if !self.mt_parent_indices.data.is_empty() {
            self.mt_parent_indices
                .write_full(writer, base_offset, data_ptr, endian, ())?;
        }
        if !self.mt_names.data.is_empty() {
            let names = self.mt_names.write(writer, base_offset, data_ptr, endian)?;
            for name in names.0 {
                string_section.insert_offset64(&name.name);
            }
        }
        if !self.mt_transforms.data.is_empty() {
            self.mt_transforms
                .write_full(writer, base_offset, data_ptr, endian, ())?;
        }

        // TODO: Only padded if MT data is not present?
        if self.mt_parent_indices.data.is_empty() {
            weird_skel_alignment(writer, data_ptr, endian)?;
        }

        if !self.labels.0.data.is_empty() {
            self.labels
                .write_offsets(writer, base_offset, data_ptr, endian, ())?;
        }

        self.extra
            .write_offsets(writer, base_offset, data_ptr, endian, ())?;

        // The names are the last item before the addresses.
        let start_alignment = match self.extra {
            SkeletonExtraOffsets::Unk0 => 4,
            SkeletonExtraOffsets::Unk1(_) => 8,
            SkeletonExtraOffsets::Unk2(_) => 8,
            SkeletonExtraOffsets::Unk3(_) => 8,
            SkeletonExtraOffsets::Unk => todo!(),
        };
        string_section.write(
            writer,
            data_ptr,
            &WriteOptions {
                start_alignment,
                start_padding_byte: 0xff,
                string_alignment: 1,
                string_padding_byte: 0,
            },
            endian,
        )?;

        Ok(())
    }
}

fn weird_skel_alignment<W: std::io::Write + std::io::Seek>(
    writer: &mut W,
    data_ptr: &mut u64,
    endian: xc3_write::Endian,
) -> xc3_write::Xc3Result<()> {
    // TODO: What is this strange padding?
    // First align to 8.
    // FF...
    let pos = writer.stream_position()?;
    align(writer, pos, 8, 0xff)?;

    // Now align to 16.
    // 0000 FF...
    [0u8; 2].xc3_write(writer, endian)?;
    *data_ptr = (*data_ptr).max(writer.stream_position()?);

    let pos = writer.stream_position()?;
    align(writer, pos, 16, 0xff)?;

    // 0000
    [0u8; 4].xc3_write(writer, endian)?;
    *data_ptr = (*data_ptr).max(writer.stream_position()?);
    Ok(())
}

impl Xc3WriteOffsets for SkeletonExtraUnk3Offsets<'_> {
    type Args = ();

    fn write_offsets<W: std::io::prelude::Write + std::io::prelude::Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
        endian: xc3_write::Endian,
        _args: Self::Args,
    ) -> xc3_write::Xc3Result<()> {
        // Different order than field order.
        self.unk6
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.unk7
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.unk12
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.unk9
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.unk8
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.unk10
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.unk11
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.unk13
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        Ok(())
    }
}
