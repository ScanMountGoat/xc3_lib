//! Skeletons in .hkt files.
//!
//! # File Paths
//!
//! | Game | Versions | File Patterns |
//! | --- | --- | --- |
//! | Xenoblade Chronicles X | `**/*.hkt` |
//! | Xenoblade Chronicles 1 DE | |  |
//! | Xenoblade Chronicles 2 |  |  |
//! | Xenoblade Chronicles 3 |  |  |
use binrw::{BinRead, NullString};
use xc3_write::{Xc3Write, Xc3WriteOffsets};

// TODO: Come up with a better name
// TODO: implement proper write support
// TODO: havok skeleton file?
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(magic(b"\x57\xe0\xe0\x57\x10\xc0\xc0\x10"))]
#[xc3(magic(b"\x57\xe0\xe0\x57\x10\xc0\xc0\x10"))]
pub struct Hkt {
    // TODO: data and then Khk_2013.1.0-r1 version string?
    pub unk1: [u32; 14],

    // __classnames__
    #[br(map(|x: NullString| x.to_string()))]
    #[br(pad_size_to = 16)]
    pub unk2: String,
    pub unk3: [u32; 8],

    // __types__
    #[br(map(|x: NullString| x.to_string()))]
    #[br(pad_size_to = 16)]
    pub unk4: String,
    pub unk5: [u32; 8],

    // __data__
    #[br(map(|x: NullString| x.to_string()))]
    #[br(pad_size_to = 16)]
    pub unk6: String,
    pub unk7: [u32; 8],

    #[br(count = unk3[2])]
    pub unk8_1: Vec<u8>,

    pub unk8_2: [u32; 52],

    // TODO: counts?
    pub count: u32,
    pub unk9_2: [u32; 19],

    // TODO: offset 640 or 672?
    // TODO: root bone name?
    #[br(map(|x: NullString| x.to_string()))]
    #[br(pad_size_to = 16)]
    pub unk10: String,

    // Parent indices?
    #[br(count = count)]
    #[br(align_after = 16)]
    pub parent_indices: Vec<i16>,

    // TODO: padding until names?
    #[br(count = count)]
    pub unk11: Vec<u64>,

    // // TODO: root name is included in name list?
    // #[br(map(|x: NullString| x.to_string()))]
    // #[br(pad_size_to = 24)]
    // pub root_name: String,
    #[br(count = count)]
    pub names: Vec<BoneName>,

    // pub unk14: [u32; 4],
    #[br(count = count)]
    pub transforms: Vec<Transform>,
    // TODO: more integer values?
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
pub struct BoneName {
    #[br(map(|x: NullString| x.to_string()))]
    #[br(align_after = 16)]
    pub name: String,
}
