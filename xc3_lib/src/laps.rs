//! User interface data in `.wilay` files.
//!
//! # File Paths
//!
//! | Game | Versions | File Patterns |
//! | --- | --- | --- |
//! | Xenoblade 1 DE | |  |
//! | Xenoblade 2 | 10001  | `menu/image/*.wilay` |
//! | Xenoblade 3 |  |  |
use crate::{parse_offset32_count32, parse_string_ptr32};
use binrw::BinRead;
use xc3_write::{Xc3Write, Xc3WriteOffsets};

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, PartialEq, Clone)]
#[br(magic(b"LAPS"))]
#[xc3(magic(b"LAPS"))]
pub struct Laps {
    // TODO: enum?
    pub version: u32,

    pub width: u32,  // 1280
    pub height: u32, // 720

    pub unk1: u32, // 0

    #[br(parse_with = parse_offset32_count32)]
    #[xc3(offset_count(u32, u32))]
    pub unk2: Vec<Unk2>,

    #[br(parse_with = parse_offset32_count32)]
    #[xc3(offset_count(u32, u32))]
    pub unk3: Vec<Unk3>,

    pub unk5: u32,

    // TODO: padding?
    pub unk: [u32; 5],
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct Unk2 {
    pub unk1: u32,

    #[br(parse_with = parse_string_ptr32)]
    #[xc3(offset(u32))]
    pub unk2: String,

    // base offset for items?
    pub unk3: u32,
    pub unk4: u32,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct Unk3 {
    pub unk1: u32,

    // TODO: sometimes float?
    #[br(parse_with = parse_string_ptr32)]
    #[xc3(offset(u32))]
    pub unk2: String,

    pub unk3: u32,
    pub unk4: i32,

    pub unk5: u32,
    pub unk6: u32,
    pub unk7: [f32; 5],
    pub unk8: u32,
    pub unk9: u32,
}

impl Xc3WriteOffsets for LapsOffsets<'_> {
    type Args = ();

    fn write_offsets<W: std::io::prelude::Write + std::io::prelude::Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
        endian: xc3_write::Endian,
        _args: Self::Args,
    ) -> xc3_write::Xc3Result<()> {
        // Strings at the end of the file.
        let unk2 = self.unk2.write(writer, base_offset, data_ptr, endian)?;
        let unk3 = self.unk3.write(writer, base_offset, data_ptr, endian)?;

        for u in unk2.0 {
            u.write_offsets(writer, base_offset, data_ptr, endian, ())?;
        }

        for u in unk3.0 {
            u.write_offsets(writer, base_offset, data_ptr, endian, ())?;
        }

        // Align the file size to 16.
        let padding = data_ptr.next_multiple_of(16) - *data_ptr;
        vec![0u8; padding as usize].xc3_write(writer, endian)?;
        *data_ptr = (*data_ptr).max(writer.stream_position()?);
        Ok(())
    }
}
