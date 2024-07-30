//! Animation and skeleton data in `.anm` or `.motstm_data` files or [Sar1](crate::sar1::Sar1) archives.
use std::collections::BTreeMap;

use crate::{align, parse_offset64_count32, parse_ptr64, parse_string_ptr64};
use binrw::{args, binread, BinRead};
use xc3_write::{VecOffsets, Xc3Write, Xc3WriteOffsets};

use anim::Anim;
use asmb::Asmb;
use skdy::Skdy;
use skel::Skel;

pub mod anim;
pub mod asmb;
pub mod skdy;
pub mod skel;

// TODO: is the 64 byte alignment on the sar1 entry size?
// TODO: Add class names from xenoblade 2 binary where appropriate.
// Assume the BC is at the beginning of the reader to simplify offsets.
#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
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

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
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

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct Transform {
    pub translation: [f32; 4],
    pub rotation_quaternion: [f32; 4],
    pub scale: [f32; 4],
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct StringOffset {
    #[br(parse_with = parse_string_ptr64)]
    #[xc3(offset(u64))]
    pub name: String,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct BcOffset<T>
where
    T: Xc3Write + 'static,
    for<'a> T: BinRead<Args<'a> = ()>,
    for<'a> T::Offsets<'a>: Xc3WriteOffsets,
{
    #[br(parse_with = parse_ptr64)]
    #[xc3(offset(u64))]
    pub value: T,
}

// TODO: Make this generic over the alignment and padding byte?
#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct BcList<T>
where
    T: Xc3Write + 'static,
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

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Default)]
struct StringSection {
    // Unique strings are stored in alphabetical order.
    name_to_offsets: BTreeMap<String, Vec<u64>>,
}

impl StringSection {
    fn insert_offset(&mut self, offset: &xc3_write::Offset<'_, u64, String>) {
        self.name_to_offsets
            .entry(offset.data.clone())
            .or_default()
            .push(offset.position);
    }

    fn write<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        data_ptr: &mut u64,
        alignment: u64,
        endian: xc3_write::Endian,
    ) -> xc3_write::Xc3Result<()> {
        // Write the string data.
        // TODO: Cleaner way to handle alignment?
        let mut name_to_position = BTreeMap::new();
        writer.seek(std::io::SeekFrom::Start(*data_ptr))?;
        align(writer, *data_ptr, alignment, 0xff)?;

        for name in self.name_to_offsets.keys() {
            let offset = writer.stream_position()?;
            writer.write_all(name.as_bytes())?;
            writer.write_all(&[0u8])?;
            name_to_position.insert(name, offset);
        }
        *data_ptr = (*data_ptr).max(writer.stream_position()?);

        // Update offsets.
        for (name, offsets) in &self.name_to_offsets {
            for offset in offsets {
                let position = name_to_position[name];
                // Assume all string pointers are 8 bytes.
                writer.seek(std::io::SeekFrom::Start(*offset))?;
                position.xc3_write(writer, endian)?;
            }
        }

        Ok(())
    }
}
