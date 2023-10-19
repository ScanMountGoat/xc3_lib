//! Compiled shaders in `.wishp` files or embedded in other formats.
//!
//! XC3: `monolib/shader/*.wishp`
use std::io::{Cursor, SeekFrom};

use crate::{parse_count32_offset32, parse_offset32_count32, parse_opt_ptr32, parse_string_ptr32};
use binrw::{args, binread, BinRead, BinReaderExt, BinResult};
use xc3_write::{VecOffsets, Xc3Write, Xc3WriteOffsets};

/// .wishp, embedded in .wismt and .wimdo
#[binread]
#[derive(Debug, Xc3Write)]
#[br(magic(b"HCPS"))]
#[xc3(magic(b"HCPS"))]
#[br(stream = r)]
pub struct Spch {
    // Subtract the magic size.
    #[br(temp, try_calc = r.stream_position().map(|p| p - 4))]
    base_offset: u64,

    pub version: u32,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub shader_programs: Vec<ShaderProgram>,

    // TODO: Related to string section?
    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub unk4s: Vec<Unk4>,

    /// A collection of [Slct].
    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub slct_section: Vec<u8>,

    /// Compiled shader binaries.
    /// Alternates between vertex and fragment shaders.
    // TODO: Optimized function for reading bytes?
    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32), align(4096))]
    pub xv4_section: Vec<u8>,

    // data before the xV4 section
    // same count as xV4 but with magic 0x34127698?
    // each has length 2176 (referenced in shaders?)
    // TODO: Optimized function for reading bytes?
    /// A collection of [UnkItem].
    // TODO: xc2 tg_ui_hitpoint.wimdo has some sort of assembly code?
    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32), align(8))]
    pub unk_section: Vec<u8>,

    // TODO: Does this actually need the program count?
    #[br(parse_with = parse_opt_ptr32)]
    #[br(args {
        offset: base_offset,
        inner: args! { base_offset, count: shader_programs.len()
    }})]
    #[xc3(offset(u32))]
    pub string_section: Option<StringSection>,

    pub unk7: u32,

    pub padding: [u32; 4],
}

#[derive(Debug, BinRead)]
#[br(import { base_offset: u64, count: usize })]
pub struct StringSection {
    #[br(args { count, inner: base_offset})]
    pub program_names: Vec<StringOffset>,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
#[br(import_raw(base_offset: u64))]
pub struct StringOffset {
    #[br(parse_with = parse_string_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub string: String,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct ShaderProgram {
    /// The offset into [slct_section](struct.Spch.html#structfield.slct_section).
    pub slct_offset: u32,
    // TODO: flags?
    pub unk1: u32,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct Unk4 {
    pub unk1: u32,
    pub unk2: u32,
    pub unk3: u32,
}

#[binread]
#[derive(Debug)]
#[br(magic(b"SLCT"))]
#[br(stream = r)]
pub struct Slct {
    pub unk1: u32,

    #[br(parse_with = parse_count32_offset32)]
    pub unk_strings: Vec<UnkString>,

    #[br(parse_with = parse_count32_offset32)]
    pub nvsds: Vec<NvsdMetadataOffset>,

    pub unk5_count: u32,
    pub unk5_offset: u32,

    pub unk_offset: u32,

    pub unk_offset1: u32,

    /// The offset into [unk_section](struct.Spch.html#structfield.unk_section).
    pub unk_item_offset: u32,
    pub unk_item_total_size: u32,

    // relative to xv4 base offset
    pub xv4_offset: u32,
    // vertex + fragment size for all NVSDs
    pub xv4_total_size: u32,

    pub unks1: [u32; 4],
    // end of slct main header?
}

#[derive(BinRead, Debug)]
pub struct UnkString {
    pub unk1: u32,
    pub unk2: u32,
    #[br(parse_with = parse_string_ptr32)]
    pub text: String,
}

#[derive(BinRead, Debug)]
pub struct NvsdMetadataOffset {
    #[br(parse_with = parse_offset32_count32)]
    pub nvsd_data: Vec<u8>,
}

#[derive(Debug, BinRead, Default)]
pub struct NvsdMetadata {
    pub unks2: [u32; 6],

    #[br(parse_with = parse_offset32_count32)]
    pub nvsd_shaders: Vec<NvsdShaders>,

    pub buffers1_count: u16,
    // TODO: not always the same as above?
    pub buffers1_index_count: u16,

    // TODO: Make a parsing helper for this?
    #[br(parse_with = parse_opt_ptr32)]
    #[br(args { inner: args! { count: buffers1_count as usize } })]
    pub uniform_buffers: Option<Vec<UniformBuffer>>,

    #[br(parse_with = parse_opt_ptr32)]
    #[br(args { inner: args! { count: buffers1_index_count as usize } })]
    pub buffers1_indices: Option<Vec<i8>>,

    pub buffers2_count: u16,
    // TODO: not always the same as above?
    pub buffers2_index_count: u16,

    // TODO: SSBOs in Ryujinx?
    // TODO: make a separate type for this?
    #[br(parse_with = parse_opt_ptr32)]
    #[br(args { inner: args! { count: buffers2_count as usize } })]
    pub storage_buffers: Option<Vec<UniformBuffer>>,

    #[br(parse_with = parse_opt_ptr32)]
    #[br(args { inner: args! { count: buffers2_index_count as usize } })]
    pub buffers2_indices: Option<Vec<i8>>,

    // Count of non negative indices?
    pub sampler_count: u16,
    pub sampler_index_count: u16,

    #[br(parse_with = parse_opt_ptr32)]
    #[br(args { inner: args! { count: sampler_count as usize } })]
    pub samplers: Option<Vec<Sampler>>,

    // TODO: The index of each sampler in the shader?
    // TODO: is this ordering based on sampler.unk2 handle?
    #[br(parse_with = parse_opt_ptr32)]
    #[br(args { inner: args! { count: sampler_index_count as usize } })]
    pub samplers_indices: Option<Vec<i8>>,

    pub unks2_1: [u32; 3],

    #[br(parse_with = parse_count32_offset32)]
    pub attributes: Vec<InputAttribute>,

    // TODO: uniforms for buffers1 and then buffers2 buffers in order?
    #[br(parse_with = parse_count32_offset32)]
    pub uniforms: Vec<Uniform>,

    pub unk3_1: u32,
    pub unk3_2: u32,
    // TODO: Why do these not match the same values in the slct?
    pub xv4_total_size: u32,
    pub unk_item_total_size: u32,
}

// TODO: add read method to slct?
#[derive(BinRead, Debug)]
pub struct UnkItem {
    pub unk1: u32,
    pub unk2: u32,
    pub unk3: u32,
    pub unk4: u32,
    pub unk5: u32,

    // TODO: relative to start of data for this unk item?
    pub assembly_code_string_offset: u32,
    pub assembly_code_string_length: u32,

    pub unk8: u32,
    pub unk9: u32,
    // TODO: more fields?

    // TODO: Always 256 bytes in length?
    // TODO: same as fragment xv4 size?
    #[br(seek_before = SeekFrom::Start(3968))]
    pub const_buffer_offset: u32,
    pub shader_size: u32,
}

// TODO: Does anything actually point to the nvsd magic?
#[derive(BinRead, Debug)]
pub struct NvsdShaders {
    pub unk6: u32, // 1
    /// The size of the vertex shader pointed to by the [Slct].
    pub vertex_xv4_size: u32,
    /// The size of the fragment shader pointed to by the [Slct].
    pub fragment_xv4_size: u32,
    /// The size of the [UnkItem] for the vertex shader.
    pub vertex_unk_item_size: u32,
    /// The size of the [UnkItem] for the fragment shader.
    pub fragment_unk_item_size: u32,
}

// TODO: CBuffer?
#[derive(BinRead, Debug)]
pub struct UniformBuffer {
    #[br(parse_with = parse_string_ptr32)]
    pub name: String,
    pub uniform_count: u16,
    pub uniform_start_index: u16,
    pub unk3: u32,
    pub handle: Handle, // TODO: handle.handle + 3?
    pub unk5: u16,      // (start + count) * 32 for buffers1?
}

// TODO: is this used for all handle fields?
#[derive(BinRead, Debug)]
pub struct Handle {
    pub handle: u8,
    pub visibility: Visibility,
}

#[derive(BinRead, Debug)]
#[br(repr(u8))]
pub enum Visibility {
    // TODO: this doesn't work for storage buffers?
    Fragment = 1,
    VertexFragment = 2,
}

#[derive(BinRead, Debug)]
pub struct Sampler {
    #[br(parse_with = parse_string_ptr32)]
    pub name: String,
    pub unk1: u32,
    // TODO: upper byte never set since samplers are fragment only?
    pub handle: Handle, // handle = (unk2 & 0xFF) * 2 + 8?
    pub unk: u16,       // TODO: always 0?
}

/// A `vec4` parameter in a [UniformBuffer].
#[derive(BinRead, Debug, Clone)]
pub struct Uniform {
    /// The name used to refer to the uniform like `gMatCol`.
    #[br(parse_with = parse_string_ptr32)]
    pub name: String,

    /// The offset into the parent buffer in bytes.
    /// Usually a multiple of 16 since buffers are declared as `vec4 data[0x1000];`.
    pub buffer_offset: u32,
}

#[derive(BinRead, Debug)]
pub struct InputAttribute {
    #[br(parse_with = parse_string_ptr32)]
    pub name: String,
    pub location: u32,
}

impl ShaderProgram {
    pub fn read_slct(&self, slct_section: &[u8]) -> BinResult<Slct> {
        // Select the bytes first to avoid needing base offsets.
        let bytes = &slct_section[self.slct_offset as usize..];
        let mut reader = Cursor::new(bytes);
        reader.read_le()
    }
}

impl Slct {
    pub fn read_unk_item(&self, unk_section: &[u8]) -> BinResult<UnkItem> {
        let bytes = &unk_section[self.unk_item_offset as usize..];
        let mut reader = Cursor::new(bytes);
        reader.read_le()
    }
}

impl NvsdMetadataOffset {
    pub fn read_nvsd(&self) -> BinResult<NvsdMetadata> {
        let mut reader = Cursor::new(&self.nvsd_data);
        reader.read_le()
    }
}

impl NvsdMetadata {
    // TODO: Add option to strip xv4 header?
    /// Returns the bytes for the compiled fragment shader, including the 48-byte xv4 header.
    pub fn vertex_binary<'a>(&self, slct_xv4_offset: u32, xv4_section: &'a [u8]) -> &'a [u8] {
        // TODO: Do all models use the second item?
        let shaders = &self.nvsd_shaders[1];

        // The first offset is the vertex shader.
        let offset = slct_xv4_offset as usize;
        &xv4_section[offset..offset + shaders.vertex_xv4_size as usize]
    }

    /// Returns the bytes for the compiled vertex shader, including the 48-byte xv4 header.
    pub fn fragment_binary<'a>(&self, slct_xv4_offset: u32, xv4_section: &'a [u8]) -> &'a [u8] {
        // TODO: Do all models use the second item?
        let shaders = &self.nvsd_shaders.last().unwrap();

        // The fragment shader immediately follows the vertex shader.
        let offset = slct_xv4_offset as usize + shaders.vertex_xv4_size as usize;
        &xv4_section[offset..offset + shaders.fragment_xv4_size as usize]
    }
}

impl Xc3Write for StringSection {
    type Offsets<'a> = VecOffsets<StringOffsetOffsets<'a>>;

    fn xc3_write<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        data_ptr: &mut u64,
    ) -> binrw::BinResult<Self::Offsets<'_>> {
        self.program_names.xc3_write(writer, data_ptr)
    }
}

impl<'a> Xc3WriteOffsets for SpchOffsets<'a> {
    fn write_offsets<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
    ) -> binrw::BinResult<()> {
        // The ordering is slightly different than the field order.
        self.shader_programs
            .write_full(writer, base_offset, data_ptr)?;
        self.unk4s.write_full(writer, base_offset, data_ptr)?;
        self.string_section
            .write_full(writer, base_offset, data_ptr)?;
        self.slct_section
            .write_full(writer, base_offset, data_ptr)?;
        self.unk_section.write_full(writer, base_offset, data_ptr)?;
        self.xv4_section.write_full(writer, base_offset, data_ptr)?;
        Ok(())
    }
}
