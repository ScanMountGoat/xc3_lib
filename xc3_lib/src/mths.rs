//! Compiled shaders in `.cashd` files or embedded in `.camdo` files.
//!
//! # File Paths
//! | Game | File Patterns |
//! | --- | --- |
//! | Xenoblade Chronicles X | `monolib/shader/*.cashd` |
//! | Xenoblade Chronicles 1 DE |  |
//! | Xenoblade Chronicles 2 |  |
//! | Xenoblade Chronicles 3 |  |
use crate::parse_ptr32;
use binrw::{helpers::until_eof, BinRead};
use xc3_write::{Xc3Write, Xc3WriteOffsets};

// Assume the reader only contains the MTHS data.
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(magic(b"MTHS"))]
#[xc3(magic(b"MTHS"))]
pub struct Mths {
    pub version: u32, // 10001

    #[br(parse_with = parse_ptr32)]
    #[xc3(offset(u32))]
    pub vertex_shader: VertexShader,

    #[br(parse_with = parse_ptr32)]
    #[xc3(offset(u32))]
    pub fragment_shader: FragmentShader,

    pub unk3: u32, // geometry?

    pub uniform_buffer_offset: u32,
    pub uniform_offset: u32,
    pub attribute_offset: u32,
    pub sampler_offset: u32,
    pub unk_offset: u32,
    pub string_offset: u32,
    pub program_offset: u32,

    #[br(parse_with = until_eof)]
    pub data: Vec<u8>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct VertexShader {
    pub unk_count: u32,
    pub unk_offset: u32,

    pub program_length: u32,
    pub program_offset: u32,

    pub shader_mode: u32,

    pub uniform_buffer_count: u32,
    pub uniform_buffer_offset: u32,

    pub uniform_count: u32,
    pub uniform_offset: u32,

    pub unk9: [u32; 4],
    pub sampler_count: u32,
    pub sampler_offset: u32,

    pub attribute_count: u32,
    pub attribute_offset: u32,

    // TODO: padding?
    pub unks: [u32; 6],
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct FragmentShader {
    pub unk1: u32,
    pub unk2: u32,

    pub program_length: u32,
    pub program_offset: u32,

    pub shader_mode: u32,

    pub uniform_buffer_count: u32,
    pub uniform_buffer_offset: u32,

    pub uniform_count: u32,
    pub uniform_offset: u32,

    pub unk9: [u32; 4],

    pub sampler_count: u32,
    pub sampler_offset: u32,
}
