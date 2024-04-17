//! Cutscene data in .beb files.
//!
//! # File Paths
//!
//! | Game | Versions | File Patterns |
//! | --- | --- | --- |
//! | Xenoblade Chronicles 1 DE | |  |
//! | Xenoblade Chronicles 2 |  | |
//! | Xenoblade Chronicles 3 |  | `event/**/*.beb` |
use crate::{parse_ptr32, xbc1::Xbc1};
use binrw::BinRead;
use xc3_write::{Xc3Write, Xc3WriteOffsets};

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, PartialEq, Clone)]
pub struct Beb {
    pub xbc1_count: u32,

    #[br(count = xbc1_count)]
    pub xbc1_offsets: Vec<Xbc1Offset>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct Xbc1Offset {
    // TODO: Some sort of container for bc anims?
    #[br(parse_with = parse_ptr32)]
    #[xc3(offset(u32))]
    pub xbc1: Xbc1,
}

impl<'a> Xc3WriteOffsets for BebOffsets<'a> {
    fn write_offsets<W: std::io::prelude::Write + std::io::prelude::Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
    ) -> xc3_write::Xc3Result<()> {
        if self.xbc1_offsets.0.is_empty() {
            writer.write_all(&[0u8; 12])?;
        } else {
            self.xbc1_offsets
                .write_offsets(writer, base_offset, data_ptr)?;
        }
        Ok(())
    }
}
