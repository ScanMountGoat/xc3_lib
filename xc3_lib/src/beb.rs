//! Cutscene data in .beb files.
//!
//! # File Paths
//!
//! | Game | Versions | File Patterns |
//! | --- | --- | --- |
//! | Xenoblade Chronicles 1 DE | |  |
//! | Xenoblade Chronicles 2 |  | |
//! | Xenoblade Chronicles 3 |  | `event/**/*.beb` |
use crate::{xbc1::Xbc1, Offset32};
use binrw::BinRead;
use xc3_write::{Xc3Write, Xc3WriteOffsets};

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, PartialEq, Clone)]
pub struct Beb {
    pub xbc1_count: u32,

    // TODO: Some sort of container for bc anims?
    #[br(count = xbc1_count)]
    pub xbc1_offsets: Vec<Offset32<Xbc1>>,
}

impl<'a> Xc3WriteOffsets for BebOffsets<'a> {
    type Args = ();

    fn write_offsets<W: std::io::prelude::Write + std::io::prelude::Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
        endian: xc3_write::Endian,
        _args: Self::Args,
    ) -> xc3_write::Xc3Result<()> {
        if self.xbc1_offsets.0.is_empty() {
            writer.write_all(&[0u8; 12])?;
        } else {
            self.xbc1_offsets
                .write_offsets(writer, base_offset, data_ptr, endian, ())?;
        }
        Ok(())
    }
}
