//! Cutscene data in .beh files.
//!
//! # File Paths
//!
//! | Game |  File Patterns |
//! | --- | --- |
//! | Xenoblade 1 DE |  |
//! | Xenoblade 2 |  |
//! | Xenoblade 3 |  `event/**/*.beh` |
use crate::{Offset32, datasheet::DataSheet, parse_opt_ptr32};
use binrw::{BinRead, binread};
use xc3_write::{Xc3Write, Xc3WriteOffsets};

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, PartialEq, Clone)]
#[br(magic(b"hdev"))]
#[xc3(magic(b"hdev"))]
pub struct Beh {
    pub count: u32,
    pub unk2: u32, // TODO: version 1, 2, 3?

    #[br(if(unk2 >= 2))]
    #[br(parse_with = parse_opt_ptr32)]
    #[xc3(offset(u32))]
    pub data_sheet: Option<DataSheet>,

    #[br(count = count)]
    pub offsets: Vec<Offset32<Unk4>>,
}

#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(stream = r)]
#[br(magic(b"test"))]
#[xc3(magic(b"test"))]
#[xc3(base_offset)]
#[xc3(align(1))]
pub struct Unk4 {
    // Subtract the magic size.
    #[br(temp, try_calc = r.stream_position().map(|p| p - 4))]
    base_offset: u64,

    pub count: u32,

    #[br(args { count: count as usize, inner: base_offset })]
    pub items: Vec<Unk4Item>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(_base_offset: u64))]
pub struct Unk4Item {
    // TODO: what is this hashing?
    /// Hash using [hash_str_crc](crate::hash::hash_str_crc).
    pub hash: u32, // TODO: affects data type and size?

    pub offset: u32,
    pub count: u32,
}

impl Xc3WriteOffsets for BehOffsets<'_> {
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
        *data_ptr = data_ptr.next_multiple_of(16);
        self.offsets
            .write_offsets(writer, base_offset, data_ptr, endian, ())?;
        self.data_sheet
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        Ok(())
    }
}
