//! Compiled shaders in `.cashd` files or embedded in `.camdo` files.
//!
//! # File Paths
//! | Game | File Patterns |
//! | --- | --- |
//! | Xenoblade Chronicles X | `monolib/shader/*.cashd` |
//! | Xenoblade Chronicles 1 DE |  |
//! | Xenoblade Chronicles 2 |  |
//! | Xenoblade Chronicles 3 |  |
use std::io::Cursor;

use binrw::{helpers::until_eof, BinRead, BinResult, BinWrite};
use xc3_write::{Xc3Write, Xc3WriteOffsets};

use crate::{parse_count32_offset32_unchecked, parse_string_ptr32_unchecked};

const HEADER_SIZE: u32 = 48;

// TODO: list.cashd is an SHPC file.
// Assume the reader only contains the MTHS data.
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(magic(b"MTHS"))]
#[xc3(magic(b"MTHS"))]
pub struct Mths {
    pub version: u32, // 10001
    pub vertex_shader_offset: u32,
    pub fragment_shader_offset: u32,
    pub unk3: u32, // geometry?
    pub uniform_buffer_offset: u32,
    pub uniform_offset: u32,
    pub attribute_offset: u32,
    pub sampler_offset: u32,
    pub unk_offset: u32,
    pub string_offset: u32, // TODO: Why does this not always work?
    pub program_offset: u32,

    #[br(parse_with = until_eof)]
    pub data: Vec<u8>,
}

// TODO: Just duplicate the fields to avoid having a fragment inside a vertex shader?
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, PartialEq, Clone)]
#[br(import {
    program_offset: u64,
    uniform_buffer_offset: u64,
    uniform_offset: u64,
    string_offset: u64,
    sampler_offset: u64,
    attribute_offset: u64
})]
pub struct VertexShader {
    #[br(args { program_offset, uniform_buffer_offset, uniform_offset, string_offset, sampler_offset })]
    pub inner: FragmentShader,

    #[br(parse_with = parse_count32_offset32_unchecked)]
    #[br(args { offset: attribute_offset, inner: string_offset })]
    pub attributes: Vec<Attribute>,

    // TODO: padding?
    pub unks: [u32; 6],
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, PartialEq, Clone)]
#[br(import {
    program_offset: u64,
    uniform_buffer_offset: u64,
    uniform_offset: u64,
    string_offset: u64,
    sampler_offset: u64,
})]
pub struct FragmentShader {
    pub unk1: u32,
    pub unk2: u32,

    #[br(parse_with = parse_count32_offset32_unchecked, offset = program_offset)]
    pub program_binary: Vec<u8>,

    pub shader_mode: ShaderMode,

    #[br(parse_with = parse_count32_offset32_unchecked)]
    #[br(args { offset: uniform_buffer_offset, inner: string_offset })]
    pub uniform_buffers: Vec<UniformBuffer>,

    #[br(parse_with = parse_count32_offset32_unchecked)]
    #[br(args { offset: uniform_offset, inner: string_offset })]
    pub uniforms: Vec<Uniform>,

    pub unk9: [u32; 4],

    #[br(parse_with = parse_count32_offset32_unchecked)]
    #[br(args { offset: sampler_offset, inner: string_offset })]
    pub samplers: Vec<Sampler>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct UniformBuffer {
    #[br(parse_with = parse_string_ptr32_unchecked, offset = base_offset)]
    pub name: String,
    pub offset: u32,
    pub size: u32,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct Uniform {
    #[br(parse_with = parse_string_ptr32_unchecked, offset = base_offset)]
    pub name: String,
    pub data_type: VarType,
    pub count: u32,
    pub offset: u32,
    /// The index into [uniform_buffers](struct.FragmentShader.html#structfield.uniform_buffers)
    /// or `-1` if this uniform is not part of a buffer.
    pub uniform_buffer_index: i32,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct Attribute {
    #[br(parse_with = parse_string_ptr32_unchecked, offset = base_offset)]
    pub name: String,
    pub data_type: VarType,
    pub count: u32,
    pub location: u32,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct Sampler {
    #[br(parse_with = parse_string_ptr32_unchecked, offset = base_offset)]
    pub name: String,
    pub sampler_type: SamplerType,
    pub location: u32,
}

/// GX2ShaderMode variants used by Xenoblade X.
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, BinWrite, PartialEq, Eq, Clone, Copy, Hash)]
#[brw(repr(u32))]
pub enum ShaderMode {
    UniformRegister = 0, // TODO: uniforms but no buffers?
    UniformBlock = 1,
}

/// GX2VarType variants used by Xenoblade X.
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, BinWrite, PartialEq, Eq, Clone, Copy, Hash)]
#[brw(repr(u32))]
pub enum VarType {
    Void = 0,
    Bool = 1,
    Float = 4,
    Vec2 = 9,
    Vec3 = 10,
    Vec4 = 11,
    IVec2 = 15,
    IVec4 = 17,
    Mat2x4 = 23,
    Mat3x4 = 26,
    Mat4 = 29,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, BinWrite, PartialEq, Eq, Clone, Copy, Hash)]
#[brw(repr(u32))]
pub enum SamplerType {
    Unk1 = 0,
    D2 = 1,
    Unk2 = 2,
    Unk3 = 3,
    Unk4 = 4,
}

impl Mths {
    pub fn vertex_shader(&self) -> BinResult<VertexShader> {
        let mut reader = Cursor::new(&self.data);
        reader.set_position((self.vertex_shader_offset - HEADER_SIZE) as u64);
        VertexShader::read_be_args(
            &mut reader,
            VertexShaderBinReadArgs {
                program_offset: (self.program_offset - HEADER_SIZE) as u64,
                uniform_buffer_offset: (self.uniform_buffer_offset - HEADER_SIZE) as u64,
                uniform_offset: (self.uniform_offset - HEADER_SIZE) as u64,
                string_offset: (self.string_offset - HEADER_SIZE) as u64,
                sampler_offset: (self.sampler_offset - HEADER_SIZE) as u64,
                attribute_offset: (self.attribute_offset - HEADER_SIZE) as u64,
            },
        )
    }

    pub fn fragment_shader(&self) -> BinResult<FragmentShader> {
        let mut reader = Cursor::new(&self.data);
        reader.set_position((self.fragment_shader_offset - HEADER_SIZE) as u64);
        FragmentShader::read_be_args(
            &mut reader,
            FragmentShaderBinReadArgs {
                program_offset: (self.program_offset - HEADER_SIZE) as u64,
                uniform_buffer_offset: (self.uniform_buffer_offset - HEADER_SIZE) as u64,
                uniform_offset: (self.uniform_offset - HEADER_SIZE) as u64,
                string_offset: (self.string_offset - HEADER_SIZE) as u64,
                sampler_offset: (self.sampler_offset - HEADER_SIZE) as u64,
            },
        )
    }
}
