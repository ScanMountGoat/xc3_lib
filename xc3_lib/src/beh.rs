//! Cutscene data in .beh files.
//!
//! # File Paths
//!
//! | Game | Versions | File Patterns |
//! | --- | --- | --- |
//! | Xenoblade Chronicles 1 DE | |  |
//! | Xenoblade Chronicles 2 |  | |
//! | Xenoblade Chronicles 3 |  | `event/**/*.beh` |
use crate::{parse_offset32_count32, parse_ptr32};
use binrw::{binread, BinRead};
use xc3_write::{Xc3Write, Xc3WriteOffsets};

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, PartialEq, Clone)]
#[br(magic(b"hdev"))]
#[xc3(magic(b"hdev"))]
pub struct Beh {
    pub unk1: u32, // version?
    pub unk2: u32, // TODO: version 1, 2, 3?

    #[br(if(unk2 >= 2))]
    pub unk3: Option<u32>, // string section ptr?

    // TODO: what is the correct check for this?
    #[br(if(unk1 != 0))]
    pub inner: Option<BehInner>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct BehInner {
    #[br(parse_with = parse_ptr32)]
    #[xc3(offset(u32))]
    pub unk4: Unk4, // ptr?
    // TODO: padding?
    pub unks: [u32; 3],
}

#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(stream = r)]
#[br(magic(b"test"))]
#[xc3(magic(b"test"))]
#[xc3(base_offset)]
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
#[br(import_raw(base_offset: u64))]
pub struct Unk4Item {
    pub unk1: u32, // TODO: affects data type and size?

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32), align(16))]
    pub unk2: Vec<[u32; 4]>,
}

impl<'a> Xc3WriteOffsets for BehOffsets<'a> {
    fn write_offsets<W: std::io::prelude::Write + std::io::prelude::Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
    ) -> xc3_write::Xc3Result<()> {
        // Different order than field order.
        self.inner.write_offsets(writer, base_offset, data_ptr)?;
        self.unk3.write_offsets(writer, base_offset, data_ptr)?;
        Ok(())
    }
}
