//! Effect point data in `.wiefp` files.
//!
//! # File Paths
//! | Game | File Patterns |
//! | --- | --- |
//! | Xenoblade 1 DE |  |
//! | Xenoblade 2 | `model/{bl,en,np,oj,pc,we,wp}/*.wiefp` |
//! | Xenoblade 3 |  |
use binrw::BinRead;
use xc3_write::{Xc3Write, Xc3WriteOffsets};

// TODO: BVSC to consistently use BE for name?
// TODO: Is the padding always aligned?
// "effpnt" or "effect" "point"?
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(magic(b"CSVB"))]
#[xc3(magic(b"CSVB"))]
#[xc3(align_after(64))] // TODO: this only applies when in a sar1 archive?
pub struct Csvb {
    pub item_count: u16,
    pub unk_count: u16,
    pub unk_section_length: u32,
    pub string_section_length: u32,

    // TODO: Why do we need to divide here?
    #[br(count = unk_count as usize / 8)]
    pub unks: Vec<u16>,

    #[br(count = item_count as usize)]
    pub unk6: Vec<CvsbItem>,

    #[br(count = unk_section_length as usize)]
    pub unk_section: Vec<u8>,

    #[br(count = string_section_length as usize)]
    pub string_section: Vec<u8>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct CvsbItem {
    // TODO: Offsets relative to start of string section.
    pub name1_offset: u16,
    pub name2_offset: u16,
    pub unk3: u32,
    pub unk4: u32,
    pub unk5: u32,
}
