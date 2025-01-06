use crate::{parse_ptr64, parse_string_opt_ptr64, parse_string_ptr64};
use binrw::BinRead;
use xc3_write::{Xc3Write, Xc3WriteOffsets};

use super::{BcList, BcOffset, StringOffset, StringSection};

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(magic(b"ASMB"))]
#[xc3(magic(b"ASMB"))]
pub struct Asmb {
    #[br(parse_with = parse_ptr64)]
    #[xc3(offset(u64))]
    pub inner: AsmbInner,
}

// TODO: How to select the version?
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub enum AsmbInner {
    /// Xenoblade 2
    V1(AsmbInnerV1),
    /// Xenoblade 1 DE and Xenoblade 3
    V2(AsmbInnerV2),
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, PartialEq, Clone)]
pub struct AsmbInnerV1 {
    pub unk1: u32,
    pub unk2: i32,
    pub folders: BcList<StringOffset>,
    pub unk4: BcList<BcOffset<StateV1>>,
    pub unk5: BcList<VarParamV1>,
    pub unk6: BcList<AnimationV1>,

    // TODO: This doesn't always match the chr name?
    #[br(parse_with = parse_string_opt_ptr64)]
    #[xc3(offset(u64))]
    pub skeleton_file_name: Option<String>,

    pub unk8: BcList<KeyValueV1>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct StateV1 {
    pub unk1: u32,
    pub unk2: i32,

    #[br(parse_with = parse_string_ptr64)]
    #[xc3(offset(u64))]
    pub name: String,

    pub unk3: [i32; 4],

    // TODO: types?
    pub children: BcList<BcOffset<StateTransitionV1>>,
    pub unk9: u64,
    pub unk10: BcList<StateKeyValueV1>,
    pub unk11: BcList<StateKeyValueV1>,
    pub unk12: BcList<u64>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct AnimationV1 {
    #[br(parse_with = parse_string_ptr64)]
    #[xc3(offset(u64))]
    pub file_name: String,

    pub unk1: BcList<AnimationUnk1V1>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct VarParamV1 {
    pub unk1: u32,
    pub unk2: i32,

    #[br(parse_with = parse_string_ptr64)]
    #[xc3(offset(u64))]
    pub name: String,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct AnimationUnk1V1 {
    pub unk1: u16,
    pub unk2: u16,
    pub unk3: i32,
}

// TODO: more fields?
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct StateTransitionV1 {
    #[br(parse_with = parse_string_ptr64)]
    #[xc3(offset(u64))]
    pub name: String,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct KeyValueV1 {
    #[br(parse_with = parse_string_ptr64)]
    #[xc3(offset(u64))]
    pub key: String,

    #[br(parse_with = parse_string_ptr64)]
    #[xc3(offset(u64))]
    pub value: String,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct StateKeyValueV1 {
    #[br(parse_with = parse_string_ptr64)]
    #[xc3(offset(u64))]
    pub key: String,

    pub unk1: u32,
    pub unk2: i32,

    #[br(parse_with = parse_string_ptr64)]
    #[xc3(offset(u64))]
    pub value: String,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct AsmbInnerV2 {
    pub folders: BcList<StringOffset>,
    pub unk2: BcList<FsmGroupV2>,
    pub unk3: u64,        // 0?
    pub unk4: BcList<u8>, // TODO: type?
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct FsmGroupV2 {
    pub unk1: BcList<BcOffset<StateV2>>,
    pub unk2: BcList<BcOffset<StateTransitionV2>>,

    #[br(parse_with = parse_string_ptr64)]
    #[xc3(offset(u64))]
    pub unk3: String,

    #[br(parse_with = parse_string_ptr64)]
    #[xc3(offset(u64))]
    pub unk4: String,

    #[br(parse_with = parse_string_opt_ptr64)]
    #[xc3(offset(u64))]
    pub unk5: Option<String>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct StateV2 {
    #[br(parse_with = parse_string_ptr64)]
    #[xc3(offset(u64))]
    pub name: String,

    /// Hash of [name](#structfield.name) using [murmur3](crate::hash::murmur3).
    pub name_hash: u32,
    pub unk3: u32,
    pub unk4: u32,
    pub unk5: u32,
    pub unk6: u32,
    pub unk7: u32,

    // TODO: types?
    pub children: BcList<BcOffset<StateTransitionV2>>,
    pub unk9: BcList<u8>,
    pub unk10: BcList<u8>,
    pub unk11: BcList<u8>,
    pub unk12: BcList<u8>,
    pub unk13: BcList<u8>,

    // TODO: only in xc3?
    #[br(parse_with = parse_string_ptr64)]
    #[xc3(offset(u64))]
    pub unk14: String,

    pub unk15: [f32; 8],
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct StateTransitionV2 {
    #[br(parse_with = parse_string_ptr64)]
    #[xc3(offset(u64))]
    pub name1: String,

    #[br(parse_with = parse_string_ptr64)]
    #[xc3(offset(u64))]
    pub name2: String,

    /// Hash of [name2](#structfield.name) using [murmur3](crate::hash::murmur3).
    pub name2_hash: u32,
    pub unk4: [f32; 4],
    pub unk5: i32,
    pub unk6: [f32; 2],
    pub unk7: u32,
    pub unk8: u32,
    pub unk9: [i16; 8],
    pub unk10: f32,
    pub unk11: u32,
    pub unk12: u32,
    pub unk13: i32,
}

// TODO: Is there a cleaner way to defer and sort strings?
impl Xc3WriteOffsets for AsmbInnerV1Offsets<'_> {
    type Args = ();

    fn write_offsets<W: std::io::prelude::Write + std::io::prelude::Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
        endian: xc3_write::Endian,
        _args: Self::Args,
    ) -> xc3_write::Xc3Result<()> {
        let mut string_section = StringSection::default();

        // Different order than field order.
        let folders = self
            .folders
            .0
            .write(writer, base_offset, data_ptr, endian)?;
        for f in folders.0 {
            string_section.insert_offset(&f.name);
        }

        let unk5 = self.unk5.0.write(writer, base_offset, data_ptr, endian)?;
        for u in unk5.0 {
            string_section.insert_offset(&u.name);
        }

        let unk6 = self.unk6.0.write(writer, base_offset, data_ptr, endian)?;
        for u in unk6.0 {
            string_section.insert_offset(&u.file_name);
        }

        // TODO: find a better way to handle nested data.
        let unk4 = self.unk4.0.write(writer, base_offset, data_ptr, endian)?;
        for u in unk4.0 {
            let u = u.value.write(writer, base_offset, data_ptr, endian)?;
            string_section.insert_offset(&u.name);

            let children = u.children.0.write(writer, base_offset, data_ptr, endian)?;
            for c in children.0 {
                let c = c.value.write(writer, base_offset, data_ptr, endian)?;
                string_section.insert_offset(&c.name);
            }
        }

        let unk8 = self.unk8.0.write(writer, base_offset, data_ptr, endian)?;
        for u in unk8.0 {
            string_section.insert_offset(&u.key);
            string_section.insert_offset(&u.value);
        }

        // TODO: How to handle an optional string?
        // if let Some(name) = &self.skeleton_file_name.write_offset(writer, base_offset, data_ptr)? {
        //     string_section.insert_offset(name);
        // }

        // The names are the last item before the addresses.
        string_section.write(writer, data_ptr, 8, endian)?;

        Ok(())
    }
}
