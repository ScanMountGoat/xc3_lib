//! Compiled shaders in `.cashd` files or embedded in `.camdo` files.
//!
//! # File Paths
//! | Game | File Patterns |
//! | --- | --- |
//! | Xenoblade X | `monolib/shader/*.cashd` |
use std::io::Cursor;

use binrw::{BinRead, BinResult, BinWrite};
use xc3_write::{
    strings::{StringSectionUnique, WriteOptions},
    Xc3Write, Xc3WriteOffsets,
};

use crate::{
    parse_count32_offset32, parse_count32_offset32_unchecked, parse_offset32_count32_unchecked,
    parse_string_ptr32_unchecked, until_eof, xc3_write_binwrite_impl,
};

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
    pub uniform_block_offset: u32,
    pub uniform_offset: u32,
    pub attribute_offset: u32,
    pub sampler_offset: u32,
    pub register_offset: u32, // TODO: 52 for vert and 41 for pixel
    pub string_offset: u32,   // TODO: Why does this not always work?
    pub program_offset: u32,

    #[br(parse_with = until_eof)]
    pub data: Vec<u8>,
}

// TODO: Just duplicate the fields to avoid having a fragment inside a vertex shader?
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, PartialEq, Clone)]
#[br(import {
    register_offset: u64,
    program_offset: u64,
    uniform_block_offset: u64,
    uniform_offset: u64,
    string_offset: u64,
    sampler_offset: u64,
    attribute_offset: u64
})]
pub struct VertexShader {
    #[br(args { register_offset, program_offset, uniform_block_offset, uniform_offset, string_offset, sampler_offset })]
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
    register_offset: u64,
    program_offset: u64,
    uniform_block_offset: u64,
    uniform_offset: u64,
    string_offset: u64,
    sampler_offset: u64,
})]
pub struct FragmentShader {
    #[br(parse_with = parse_offset32_count32_unchecked, offset = register_offset)]
    pub registers: Vec<u32>,

    #[br(parse_with = parse_count32_offset32_unchecked, offset = program_offset)]
    pub program_binary: Vec<u8>,

    pub shader_mode: ShaderMode,

    #[br(parse_with = parse_count32_offset32_unchecked)]
    #[br(args { offset: uniform_block_offset, inner: string_offset })]
    pub uniform_blocks: Vec<UniformBlock>,

    #[br(parse_with = parse_count32_offset32_unchecked)]
    #[br(args { offset: uniform_offset, inner: string_offset })]
    pub uniform_vars: Vec<Uniform>,

    pub unk9: [u32; 4], // TODO: initial values and loop vars

    #[br(parse_with = parse_count32_offset32_unchecked)]
    #[br(args { offset: sampler_offset, inner: string_offset })]
    pub samplers: Vec<Sampler>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct UniformBlock {
    #[br(parse_with = parse_string_ptr32_unchecked, offset = base_offset)]
    #[xc3(offset(u32))]
    pub name: String,
    pub offset: u32,
    pub size: u32,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct Uniform {
    #[br(parse_with = parse_string_ptr32_unchecked, offset = base_offset)]
    #[xc3(offset(u32))]
    pub name: String,
    pub data_type: VarType,
    pub count: u32,
    pub offset: u32,
    /// The index into [uniform_buffers](struct.FragmentShader.html#structfield.uniform_buffers)
    /// or `-1` if this uniform is not part of a buffer.
    pub uniform_buffer_index: i32,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct Attribute {
    #[br(parse_with = parse_string_ptr32_unchecked, offset = base_offset)]
    #[xc3(offset(u32))]
    pub name: String,
    pub data_type: VarType,
    pub count: u32,
    pub location: u32,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct Sampler {
    #[br(parse_with = parse_string_ptr32_unchecked, offset = base_offset)]
    #[xc3(offset(u32))]
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
    Unk10 = 10,
    Unk13 = 13,
}

impl Mths {
    pub fn vertex_shader(&self) -> BinResult<Gx2VertexShader> {
        let mut reader = Cursor::new(&self.data);
        reader.set_position((self.vertex_shader_offset - HEADER_SIZE) as u64);
        let vert = VertexShader::read_be_args(
            &mut reader,
            VertexShaderBinReadArgs {
                register_offset: (self.register_offset - HEADER_SIZE) as u64,
                program_offset: (self.program_offset - HEADER_SIZE) as u64,
                uniform_block_offset: (self.uniform_block_offset - HEADER_SIZE) as u64,
                uniform_offset: (self.uniform_offset - HEADER_SIZE) as u64,
                string_offset: (self.string_offset - HEADER_SIZE) as u64,
                sampler_offset: (self.sampler_offset - HEADER_SIZE) as u64,
                attribute_offset: (self.attribute_offset - HEADER_SIZE) as u64,
            },
        )?;
        // GX2 types are better supported by existing tooling.
        // TODO: Why are there occasionally extra "registers" in the cashd?
        Ok(Gx2VertexShader {
            registers: convert_ne_bytes(&vert.inner.registers)?,
            program_binary: vert.inner.program_binary,
            shader_mode: vert.inner.shader_mode,
            uniform_blocks: vert.inner.uniform_blocks,
            uniform_vars: vert.inner.uniform_vars,
            unk9: vert.inner.unk9,
            sampler_vars: vert.inner.samplers,
            attributes: vert.attributes,
            ring_item_size: 0,
            has_stream_out: 0,
            stream_out_stride: [0; 4],
            r_buffer: [0; 4],
        })
    }

    pub fn pixel_shader(&self) -> BinResult<Gx2PixelShader> {
        let mut reader = Cursor::new(&self.data);
        reader.set_position((self.fragment_shader_offset - HEADER_SIZE) as u64);
        let frag = FragmentShader::read_be_args(
            &mut reader,
            FragmentShaderBinReadArgs {
                register_offset: (self.register_offset - HEADER_SIZE) as u64,
                program_offset: (self.program_offset - HEADER_SIZE) as u64,
                uniform_block_offset: (self.uniform_block_offset - HEADER_SIZE) as u64,
                uniform_offset: (self.uniform_offset - HEADER_SIZE) as u64,
                string_offset: (self.string_offset - HEADER_SIZE) as u64,
                sampler_offset: (self.sampler_offset - HEADER_SIZE) as u64,
            },
        )?;
        // GX2 types are better supported by existing tooling.
        // TODO: Why are there occasionally extra "registers" in the cashd?
        Ok(Gx2PixelShader {
            registers: convert_ne_bytes(&frag.registers)?,
            program_binary: frag.program_binary,
            shader_mode: frag.shader_mode,
            uniform_blocks: frag.uniform_blocks,
            uniform_vars: frag.uniform_vars,
            unk9: frag.unk9,
            sampler_vars: frag.samplers,
            r_buffer: [0; 4],
        })
    }
}

#[derive(Debug, BinRead, Xc3Write, PartialEq, Clone)]
pub struct Gx2VertexShader {
    pub registers: Gx2VertexShaderRegisters,

    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32), align(4096))]
    pub program_binary: Vec<u8>,

    pub shader_mode: ShaderMode,

    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub uniform_blocks: Vec<UniformBlock>,

    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub uniform_vars: Vec<Uniform>,

    pub unk9: [u32; 4], // TODO: initial values and loop vars

    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub sampler_vars: Vec<Sampler>,

    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub attributes: Vec<Attribute>,

    pub ring_item_size: u32,
    pub has_stream_out: u32,
    pub stream_out_stride: [u32; 4],
    pub r_buffer: [u32; 4],
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct Gx2VertexShaderRegisters {
    pub sq_pgm_resources_vs: u32,
    pub vgt_primitiveid_en: u32,
    pub spi_vs_out_config: u32,
    pub num_spi_vs_out_id: u32,
    pub spi_vs_out_id: [u32; 10],
    pub pa_cl_vs_out_cntl: u32,
    pub sq_vtx_semantic_clear: u32,
    pub num_sq_vtx_semantic: u32,
    pub sq_vtx_semantic: [u32; 32],
    pub vgt_strmout_buffer_en: u32,
    pub vgt_vertex_reuse_block_cntl: u32,
    pub vgt_hos_reuse_depth: u32,
}

#[derive(Debug, BinRead, Xc3Write, PartialEq, Clone)]
pub struct Gx2PixelShader {
    pub registers: Gx2PixelShaderRegisters,

    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32), align(4096))]
    pub program_binary: Vec<u8>,

    pub shader_mode: ShaderMode,

    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub uniform_blocks: Vec<UniformBlock>,

    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub uniform_vars: Vec<Uniform>,

    pub unk9: [u32; 4], // TODO: initial values and loop vars

    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub sampler_vars: Vec<Sampler>,

    pub r_buffer: [u32; 4],
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct Gx2PixelShaderRegisters {
    pub sq_pgm_resources_ps: u32,
    pub sq_pgm_exports_ps: u32,
    pub spi_ps_in_control_0: u32,
    pub spi_ps_in_control_1: u32,
    pub num_spi_ps_input_cntl: u32,
    pub spi_ps_input_cntls: [u32; 32],
    pub cb_shader_mask: u32,
    pub cb_shader_control: u32,
    pub db_shader_control: u32,
    pub spi_input_z: u32,
}

xc3_write_binwrite_impl!(VarType, ShaderMode, SamplerType);

fn convert_ne_bytes<T, U>(value: &T) -> BinResult<U>
where
    for<'a> T: BinWrite<Args<'a> = ()>,
    for<'a> U: BinRead<Args<'a> = ()>,
{
    let mut cursor = Cursor::new(Vec::new());
    value.write_ne(&mut cursor).unwrap();
    U::read_ne(&mut cursor)
}

impl Xc3WriteOffsets for Gx2VertexShaderOffsets<'_> {
    type Args = ();

    fn write_offsets<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
        endian: xc3_write::Endian,
        args: Self::Args,
    ) -> xc3_write::Xc3Result<()> {
        // Different order than field order.
        let mut strings = StringSectionUnique::default();
        let blocks = self
            .uniform_blocks
            .write(writer, base_offset, data_ptr, endian)?;
        for b in blocks.0 {
            strings.insert_offset32(&b.name);
        }
        let uniforms = self
            .uniform_vars
            .write(writer, base_offset, data_ptr, endian)?;
        for u in uniforms.0 {
            strings.insert_offset32(&u.name);
        }
        let samplers = self
            .sampler_vars
            .write(writer, base_offset, data_ptr, endian)?;
        for s in samplers.0 {
            strings.insert_offset32(&s.name);
        }
        let attributes = self
            .attributes
            .write(writer, base_offset, data_ptr, endian)?;
        for a in attributes.0 {
            strings.insert_offset32(&a.name);
        }
        strings.write(
            writer,
            base_offset,
            data_ptr,
            &WriteOptions::default(),
            endian,
        )?;
        self.program_binary
            .write_full(writer, base_offset, data_ptr, endian, args)?;
        Ok(())
    }
}

impl Xc3WriteOffsets for Gx2PixelShaderOffsets<'_> {
    type Args = ();

    fn write_offsets<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
        endian: xc3_write::Endian,
        args: Self::Args,
    ) -> xc3_write::Xc3Result<()> {
        // Different order than field order.
        let mut strings = StringSectionUnique::default();
        let blocks = self
            .uniform_blocks
            .write(writer, base_offset, data_ptr, endian)?;
        for b in blocks.0 {
            strings.insert_offset32(&b.name);
        }
        let uniforms = self
            .uniform_vars
            .write(writer, base_offset, data_ptr, endian)?;
        for u in uniforms.0 {
            strings.insert_offset32(&u.name);
        }
        let samplers = self
            .sampler_vars
            .write(writer, base_offset, data_ptr, endian)?;
        for s in samplers.0 {
            strings.insert_offset32(&s.name);
        }
        strings.write(
            writer,
            base_offset,
            data_ptr,
            &WriteOptions::default(),
            endian,
        )?;
        self.program_binary
            .write_full(writer, base_offset, data_ptr, endian, args)?;
        Ok(())
    }
}
