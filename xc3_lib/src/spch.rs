//! Compiled shaders in `.wishp` files or embedded in other formats.
//!
//! # File Paths
//! | Game | File Patterns |
//! | --- | --- |
//! | Xenoblade Chronicles 1 DE | `monolib/shader/*.wishp` |
//! | Xenoblade Chronicles 2 | `monolib/shader/*.wishp` |
//! | Xenoblade Chronicles 3 | `monolib/shader/*.wishp` |
use std::io::{Cursor, Read, Seek, SeekFrom};

use crate::{
    get_bytes, parse_count32_offset32, parse_offset32_count32, parse_opt_ptr32, parse_string_ptr32,
    xc3_write_binwrite_impl, StringOffset32,
};
use binrw::{args, binread, BinRead, BinReaderExt, BinResult, BinWrite};
use xc3_write::{Xc3Write, Xc3WriteOffsets};

// TODO: Add example code for extracting shaders.
/// .wishp, embedded in .wismt and .wimdo
#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, PartialEq, Clone)]
#[br(magic(b"HCPS"))]
#[xc3(magic(b"HCPS"))]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct Spch {
    // Subtract the magic size.
    #[br(temp, try_calc = r.stream_position().map(|p| p - 4))]
    base_offset: u64,

    pub version: u32,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub slct_offsets: Vec<SlctOffset>,

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
    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32), align(4096))]
    pub xv4_section: Vec<u8>,

    // data before the xV4 section
    // same count as xV4 but with magic 0x34127698?
    // each has length 2176 (referenced in shaders?)
    /// A collection of [UnkItem].
    // TODO: xc2 tg_ui_hitpoint.wimdo has some sort of assembly code?
    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32), align(8))]
    pub unk_section: Vec<u8>,

    // TODO: Does this actually need the slct count?
    #[br(parse_with = parse_opt_ptr32)]
    #[br(args {
        offset: base_offset,
        inner: args! { base_offset, count: slct_offsets.len()
    }})]
    #[xc3(offset(u32))]
    pub string_section: Option<StringSection>,

    pub unk7: u32,

    pub padding: [u32; 4],
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import { base_offset: u64, count: usize })]
pub struct StringSection {
    #[br(args { count, inner: base_offset})]
    pub program_names: Vec<StringOffset32>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct SlctOffset {
    /// The offset into [slct_section](struct.Spch.html#structfield.slct_section) for the [Slct].
    pub offset: u32,
    // TODO: flags?
    pub unk1: u32,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct Unk4 {
    pub unk1: u32,
    pub unk2: u32,
    pub unk3: u32,
}

#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug)]
#[br(magic(b"SLCT"))]
#[br(stream = r)]
pub struct Slct {
    pub unk1: u32,

    #[br(parse_with = parse_count32_offset32)]
    pub unk_strings: Vec<UnkString>,

    /// The compiled program binaries and associated metadata.
    ///
    /// This will have length 1 unless there are multiple shader permutations.
    /// Permutations may have different defines in the original source or even completely different code.
    #[br(parse_with = parse_count32_offset32)]
    pub programs: Vec<ShaderProgram>,

    pub unk5_count: u32,
    pub unk5_offset: u32,

    pub unk_offset: u32,

    pub unk_offset1: u32,

    /// The offset into [unk_section](struct.Spch.html#structfield.unk_section).
    pub unk_item_offset: u32,
    pub unk_item_total_size: u32,

    /// Relative to xv4 base offset.
    pub xv4_offset: u32,
    /// Vertex + fragment size for all NVSDs.
    pub xv4_total_size: u32,

    pub unks1: [u32; 4],
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead)]
pub struct UnkString {
    pub unk1: u32,
    pub unk2: u32,
    #[br(parse_with = parse_string_ptr32)]
    pub text: String,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead)]
pub struct ShaderProgram {
    /// Raw data for [Nvsd] for Switch files and [Nvsp] for PC files.
    #[br(parse_with = parse_offset32_count32)]
    pub program_data: Vec<u8>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Default)]
pub struct Nvsd {
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

#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug)]
#[br(magic(b"\x34\x12\x76\x98"))]
#[br(stream = r)]
pub struct UnkItem {
    // Subtract the magic size.
    #[br(temp, try_calc = r.stream_position().map(|p| p - 4))]
    base_offset: u64,

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

    // TODO: is this offset stored somewhere?
    #[br(seek_before = SeekFrom::Start(base_offset + 1776))]
    pub unk10: u32,
    pub unk11: u32,
    pub unk12: u32,
    pub unk13: u32,
    // TODO: Always 256 bytes in length?
    /// Offset relative to the start of the shader program binary for the constant buffer.
    /// This can be assumed to be be 256 bytes of floating point values at the end
    /// of the fragment shader program binary.
    pub constant_buffer_offset: u32,
    pub unk15: u32,
    pub unk16: u32,
    pub unk17: u32,
}

// TODO: Does anything actually point to the nvsd magic?
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead)]
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
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead)]
pub struct UniformBuffer {
    #[br(parse_with = parse_string_ptr32)]
    pub name: String,
    pub uniform_count: u16,
    /// Index into [uniforms](struct.Nvsd.html#structfield.uniforms).
    pub uniform_start_index: u16,
    pub unk3: u32,
    pub handle: Handle, // TODO: handle.handle + 3?
    pub size_in_bytes: u16,
}

// TODO: is this used for all handle fields?
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, BinWrite, PartialEq, Clone)]
pub struct Handle {
    pub handle: u8,
    pub visibility: Visibility,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, BinWrite, PartialEq, Clone)]
#[brw(repr(u8))]
pub enum Visibility {
    // TODO: this doesn't work for storage buffers?
    Vertex = 0,
    Fragment = 1,
    VertexFragment = 2,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead)]
pub struct Sampler {
    #[br(parse_with = parse_string_ptr32)]
    pub name: String,
    pub unk1: u32,
    // TODO: upper byte never set since samplers are fragment only?
    pub handle: Handle, // handle = (unk2 & 0xFF) * 2 + 8?
    pub unk: u16,       // TODO: always 0?
}

/// A `vec4` parameter in a [UniformBuffer].
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, PartialEq, Clone)]
pub struct Uniform {
    /// The name used to refer to the uniform like `gMatCol`.
    #[br(parse_with = parse_string_ptr32)]
    pub name: String,

    /// The offset into the parent buffer in bytes.
    /// Usually a multiple of 16 since buffers are declared as `vec4 data[0x1000];`.
    pub buffer_offset: u32,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead)]
pub struct InputAttribute {
    #[br(parse_with = parse_string_ptr32)]
    pub name: String,
    pub location: u32,
}

// TODO: This still has the 256 byte constant buffer at the end of the file?
// TODO: Does anything actually point to the NVSP magic?
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Default)]
pub struct Nvsp {
    /// GLSL shader source compressed with ZLF compression.
    #[br(parse_with = parse_offset32_count32)]
    pub shader_source: Vec<u8>,
    /// The size of [shader_source](#structfield.shader_source) after decompression.
    pub decompressed_size: u32,
    // offsets and sizes for different shaders in decompressed binary?
    pub vertex_offset: u32,
    pub vertex_length: u32,
    pub fragment_offset: u32,
    pub fragment_length: u32,
    // TODO: padding?
    pub unk: [u32; 10],
}

impl SlctOffset {
    pub fn read_slct(&self, slct_section: &[u8]) -> BinResult<Slct> {
        // Select the bytes first to avoid needing base offsets.
        let bytes = get_bytes(slct_section, self.offset, None)?;
        let mut reader = Cursor::new(bytes);
        reader.read_le()
    }
}

impl Slct {
    pub fn read_unk_item(&self, unk_section: &[u8]) -> BinResult<UnkItem> {
        let bytes = get_bytes(unk_section, self.unk_item_offset, None)?;
        let mut reader = Cursor::new(bytes);
        reader.read_le()
    }
}

impl ShaderProgram {
    pub fn read_nvsd(&self) -> BinResult<Nvsd> {
        let mut reader = Cursor::new(&self.program_data);
        reader.read_le()
    }

    // TODO: just for pc?
    pub fn read_nvsp(&self) -> BinResult<Nvsp> {
        let mut reader = Cursor::new(&self.program_data);
        reader.read_le()
    }
}

impl Nvsd {
    // TODO: Add option to strip xv4 header?
    fn read_vertex_binary<R: Read>(&self, reader: &mut R) -> Vec<u8> {
        // TODO: Always use the last item?
        let shaders = &self.nvsd_shaders.last().unwrap();
        let mut buffer = vec![0u8; shaders.vertex_xv4_size as usize];
        reader.read_exact(&mut buffer).unwrap();
        buffer
    }

    fn read_fragment_binary<R: Read>(&self, reader: &mut R) -> Vec<u8> {
        // TODO: Always use the last item?
        let shaders = &self.nvsd_shaders.last().unwrap();
        let mut buffer = vec![0u8; shaders.fragment_xv4_size as usize];
        reader.read_exact(&mut buffer).unwrap();
        buffer
    }

    fn read_fragment_unk_item<R: Read + Seek>(&self, reader: &mut R) -> BinResult<Option<UnkItem>> {
        // TODO: Always use the last item?
        let shaders = &self.nvsd_shaders.last().unwrap();

        let start = reader.stream_position()?;

        reader.seek(SeekFrom::Current(shaders.vertex_unk_item_size as i64))?;

        let fragment_unk = if shaders.fragment_unk_item_size > 0 {
            Some(reader.read_le()?)
        } else {
            None
        };

        // TODO: Read all data to avoid needing this?
        reader.seek(SeekFrom::Start(
            start + shaders.vertex_unk_item_size as u64 + shaders.fragment_unk_item_size as u64,
        ))?;

        Ok(fragment_unk)
    }
}

impl Nvsp {
    /// Decompress the GLSL source code for the vertex and fragment shader.
    pub fn vertex_fragment_source(&self) -> Option<(String, String)> {
        // TODO: Create an error type?
        let decompressed =
            lzf::decompress(&self.shader_source, self.decompressed_size as usize).ok()?;

        let vertex = String::from_utf8(
            decompressed
                .get(
                    self.vertex_offset as usize
                        ..self.vertex_offset as usize + self.vertex_length as usize,
                )?
                .to_vec(),
        )
        .ok()?;

        let fragment = String::from_utf8(
            decompressed
                .get(
                    self.fragment_offset as usize
                        ..self.fragment_offset as usize + self.fragment_length as usize,
                )?
                .to_vec(),
        )
        .ok()?;

        Some((vertex, fragment))
    }
}

#[derive(Debug, Clone)]
pub struct ShaderBinary {
    pub program_binary: Vec<u8>,
    pub constant_buffer: Option<[[f32; 4]; 16]>,
}

impl Spch {
    /// Extract the [Nvsd], vertex binary, and fragment binary for each of the programs in `slct`.
    pub fn nvsd_vertex_fragment_binaries(
        &self,
        slct: &Slct,
    ) -> Vec<(Nvsd, Option<ShaderBinary>, Option<ShaderBinary>)> {
        let nvsds: Vec<_> = slct
            .programs
            .iter()
            .map(|p| p.read_nvsd().unwrap())
            .collect();

        let binaries = vertex_fragment_binaries(
            &nvsds,
            &self.xv4_section,
            slct.xv4_offset,
            &self.unk_section,
            slct.unk_item_offset,
        );

        nvsds
            .into_iter()
            .zip(binaries)
            .map(|(n, (v, f))| (n, v, f))
            .collect()
    }

    /// Extract the [ShaderProgram], vertex binary, and fragment binary for each of the programs in `slct`.
    pub fn program_data_vertex_fragment_binaries<'a>(
        &self,
        slct: &'a Slct,
    ) -> Vec<(
        &'a ShaderProgram,
        Option<ShaderBinary>,
        Option<ShaderBinary>,
    )> {
        let nvsds: Vec<_> = slct
            .programs
            .iter()
            .map(|p| p.read_nvsd().unwrap())
            .collect();

        let binaries = vertex_fragment_binaries(
            &nvsds,
            &self.xv4_section,
            slct.xv4_offset,
            &self.unk_section,
            slct.unk_item_offset,
        );

        slct.programs
            .iter()
            .zip(binaries)
            .map(|(p, (v, f))| (p, v, f))
            .collect()
    }
}

/// Extract the vertex and fragment binary for each of the [Nvsd] in `nvsds`.
pub fn vertex_fragment_binaries(
    nvsds: &[Nvsd],
    xv4_section: &[u8],
    xv4_offset: u32,
    unk_section: &[u8],
    unk_offset: u32,
) -> Vec<(Option<ShaderBinary>, Option<ShaderBinary>)> {
    let mut xv4 = Cursor::new(xv4_section);
    xv4.set_position(xv4_offset as u64);

    let mut unk = Cursor::new(unk_section);
    unk.set_position(unk_offset as u64);

    // Each SLCT can have multiple NVSD.
    nvsds
        .iter()
        .map(|nvsd| {
            // Each NVSD can have a vertex and fragment shader.
            // TODO: Why is only the last set of shaders used?
            let vertex = nvsd.read_vertex_binary(&mut xv4);
            let fragment = nvsd.read_fragment_binary(&mut xv4);

            // TODO: Avoid unwrap.
            // TODO: do vertex shaders ever use constants?
            let fragment_unk = nvsd.read_fragment_unk_item(&mut unk).ok().flatten();

            // Assume each constant buffer is 256 bytes.
            let fragment_constants = fragment_unk.and_then(|u| {
                if u.constant_buffer_offset as usize == fragment.len() {
                    None
                } else {
                    let mut reader = Cursor::new(&fragment[u.constant_buffer_offset as usize..]);
                    Some(<[[f32; 4]; 16]>::read_le(&mut reader).unwrap())
                }
            });

            (
                (!vertex.is_empty()).then_some(ShaderBinary {
                    program_binary: vertex,
                    constant_buffer: None,
                }),
                (!fragment.is_empty()).then_some(ShaderBinary {
                    program_binary: fragment,
                    constant_buffer: fragment_constants,
                }),
            )
        })
        .collect()
}

impl<'a> Xc3WriteOffsets for SpchOffsets<'a> {
    type Args = ();

    fn write_offsets<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        _base_offset: u64,
        data_ptr: &mut u64,
        endian: xc3_write::Endian,
        args: Self::Args,
    ) -> xc3_write::Xc3Result<()> {
        // The ordering is slightly different than the field order.
        let base_offset = self.base_offset;
        self.slct_offsets
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.unk4s
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.string_section
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.slct_section
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.unk_section
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.xv4_section
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        Ok(())
    }
}

xc3_write_binwrite_impl!(Handle);
