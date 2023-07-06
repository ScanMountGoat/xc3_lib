use std::io::{Cursor, Seek, SeekFrom};

use crate::{
    parse_count_offset, parse_offset_count, parse_opt_ptr32, parse_ptr32, parse_string_ptr32,
};
use binrw::{args, binread, BinRead, BinReaderExt};
use serde::Serialize;

/// .wishp, embedded in .wismt and .wimdo
#[binread]
#[derive(Debug, Serialize)]
#[br(magic(b"HCPS"))]
#[br(stream = r)]
pub struct Spch {
    // Subtract the magic size.
    #[br(temp, try_calc = r.stream_position().map(|p| p - 4))]
    base_offset: u64,

    version: u32,

    #[br(parse_with = parse_offset_count, offset = base_offset)]
    pub shader_programs: Vec<ShaderProgram>,

    // TODO: Related to string section?
    #[br(parse_with = parse_offset_count, offset = base_offset)]
    pub unk4s: Vec<(u32, u32, u32)>,

    /// A collection of [Slct].
    #[br(parse_with = parse_offset_count, offset = base_offset)]
    pub slct_section: Vec<u8>,

    /// Compiled shader binaries.
    /// Alternates between vertex and fragment shaders.
    // TODO: Optimized function for reading bytes?
    #[br(parse_with = parse_offset_count, offset = base_offset)]
    pub xv4_section: Vec<u8>,

    // data before the xV4 section
    // same count as xV4 but with magic 0x34127698?
    // each has length 2176 (referenced in shaders?)
    // TODO: Optimized function for reading bytes?
    /// A collection of [UnkItem].
    // TODO: xc2 tg_ui_hitpoint.wimdo has some sort of assembly code?
    #[br(parse_with = parse_offset_count, offset = base_offset)]
    pub unk_section: Vec<u8>,

    // TODO: Does this actually need the program count?
    #[br(parse_with = parse_opt_ptr32, offset = base_offset)]
    #[br(args { inner: (base_offset, shader_programs.len()) })]
    pub string_section: Option<StringSection>,

    #[br(pad_after = 16)]
    unk7: u32,
    // end of header?
}

#[derive(Debug, Serialize)]
pub struct StringSection {
    pub program_names: Vec<String>,
}

// TODO: Derive this?
impl BinRead for StringSection {
    type Args<'a> = (u64, usize);

    fn read_options<R: std::io::Read + std::io::Seek>(
        reader: &mut R,
        endian: binrw::Endian,
        args: Self::Args<'_>,
    ) -> binrw::BinResult<Self> {
        let (base_offset, count) = args;

        let mut program_names = Vec::new();
        for _ in 0..count {
            let name = parse_string_ptr32(
                reader,
                endian,
                binrw::file_ptr::FilePtrArgs {
                    offset: base_offset,
                    inner: (),
                },
            )?;
            program_names.push(name);
        }

        Ok(StringSection { program_names })
    }
}

#[derive(BinRead, Debug, Serialize)]
pub struct ShaderProgram {
    pub slct_offset: u32,
    unk1: u32,
}

#[binread]
#[derive(Debug, Serialize)]
#[br(magic(b"SLCT"))]
#[br(stream = r)]
pub struct Slct {
    // Subtract the magic size.
    #[br(temp, try_calc = r.stream_position().map(|p| p - 4))]
    base_offset: u64,

    unk1: u32,

    #[br(parse_with = parse_count_offset, args { offset: base_offset, inner: base_offset })]
    unk_strings: Vec<UnkString>,

    #[br(parse_with = parse_count_offset, args { offset: base_offset, inner: base_offset })]
    pub nvsds: Vec<NvsdMetadataOffset>,

    unk5_count: u32,
    unk5_offset: u32,

    unk_offset: u32,

    unk_offset1: u32,

    /// The offset into [unk_section](struct.Spch.html#structfield.unk_section).
    pub unk_item_offset: u32,
    pub unk_item_size: u32,

    // relative to xv4 base offset
    pub xv4_offset: u32,
    // vertex + fragment size for all NVSDs
    xv4_total_size: u32,

    unks1: [u32; 4],
    // end of slct main header?
}

#[derive(BinRead, Debug, Serialize)]
#[br(import_raw(base_offset: u64))]
struct UnkString {
    unk1: u32,
    unk2: u32,
    #[br(parse_with = parse_string_ptr32, offset = base_offset)]
    text: String,
}

#[derive(BinRead, Debug, Serialize)]
#[br(import_raw(base_offset: u64))]
pub struct NvsdMetadataOffset {
    #[br(parse_with = parse_ptr32, offset = base_offset)]
    pub inner: NvsdMetadata,
    size: u32,
}

#[binread]
#[derive(Debug, Serialize)]
#[br(stream = r)]
pub struct NvsdMetadata {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    pub unks2: [u32; 8],

    pub unk_count1: u16,
    // TODO: not always the same as above?
    pub unk_count2: u16,

    #[br(parse_with = parse_ptr32)]
    #[br(args {
        offset: base_offset,
        inner: args! { count: unk_count1 as usize, inner: args! { base_offset } }
    })]
    pub buffers1: Vec<UniformBuffer>,

    pub unk13: u32, // end of strings offset?

    pub unk_count3: u16,
    // TODO: not always the same as above?
    pub unk_count4: u16,

    #[br(parse_with = parse_ptr32)]
    #[br(args {
        offset: base_offset,
        inner: args! { count: unk_count3 as usize, inner: args! { base_offset } }
    })]
    pub buffers2: Vec<UniformBuffer>,

    pub unk15: u32, // offset?

    #[br(temp)]
    sampler_count: u16,
    // TODO: not always the same as above?
    pub unk_count6: u16,

    #[br(parse_with = parse_ptr32)]
    #[br(args {
        offset: base_offset,
        inner: args! { count: sampler_count as usize, inner: args! { base_offset } }
    })]
    pub samplers: Vec<Sampler>,

    pub unks2_1: [u32; 4],

    #[br(parse_with = parse_count_offset, args { offset: base_offset, inner: base_offset })]
    pub attributes: Vec<InputAttribute>,

    #[br(parse_with = parse_count_offset, args { offset: base_offset, inner: base_offset })]
    pub uniforms: Vec<Uniform>,

    pub unks3: [u32; 4],

    // TODO: Separate this from the metadata type?
    pub nvsd: Nvsd,
}

// TODO: add read method to slct?
#[derive(BinRead, Debug, Serialize)]
pub struct UnkItem {
    unk1: u32,
    unk2: u32,
    unk3: u32,
    unk4: u32,
    unk5: u32,

    // TODO: relative to start of data for this unk item?
    assembly_code_string_offset: u32,
    assembly_code_string_length: u32,

    unk8: u32,
    unk9: u32,
    // TODO: more fields?
}

// TODO: Create a more meaningful default?
#[derive(BinRead, Debug, Serialize, Default)]
#[br(magic(b"NVSD"))]
pub struct Nvsd {
    version: u32,
    unk1: u32, // 0
    unk2: u32, // 0
    unk3: u32, // identical to vertex_xv4_size?
    unk4: u32, // 0
    unk5: u32, // identical to unk_size1?
    // end of nvsd?

    // TODO: this section isn't always present?
    unk6: u32, // 1
    /// The size of the vertex shader pointed to by the [Slct].
    pub vertex_xv4_size: u32,
    /// The size of the fragment shader pointed to by the [Slct].
    pub fragment_xv4_size: u32,
    // Corresponding unk entry size for the two shaders?
    unk_size1: u32, // 2176
    unk_size2: u32, // 2176

    // TODO: What controls this count?
    unks4: [u16; 8],
}

// TODO: CBuffer?
#[derive(BinRead, Debug, Serialize)]
#[br(import { base_offset: u64 })]
pub struct UniformBuffer {
    #[br(parse_with = parse_string_ptr32, offset = base_offset)]
    pub name: String,
    pub uniform_count: u16,
    pub uniform_start_index: u16,
    pub unk3: u32, // ??? + handle * 2?
    pub unk4: u16,
    pub unk5: u16,
}

#[derive(BinRead, Debug, Serialize)]
#[br(import { base_offset: u64 })]
pub struct Sampler {
    #[br(parse_with = parse_string_ptr32, offset = base_offset)]
    pub name: String,
    pub unk1: u32,
    pub unk2: u32, // handle = (unk2 - 256) * 2 + 8?
}

/// A `vec4` parameter in a [UniformBuffer].
#[derive(BinRead, Debug, Serialize)]
#[br(import_raw(base_offset: u64))]
pub struct Uniform {
    /// The name used to refer to the uniform like `gMatCol`.
    #[br(parse_with = parse_string_ptr32, offset = base_offset)]
    pub name: String,

    /// The offset into the parent buffer in bytes.
    /// Usually a multiple of 16 since buffers are declared as `vec4 data[0x1000];`.
    pub buffer_offset: u32,
}

#[derive(BinRead, Debug, Serialize)]
#[br(import_raw(base_offset: u64))]
pub struct InputAttribute {
    #[br(parse_with = parse_string_ptr32, offset = base_offset)]
    pub name: String,
    pub location: u32,
}

impl ShaderProgram {
    pub fn read_slct(&self, slct_section: &[u8]) -> Slct {
        let mut reader = Cursor::new(slct_section);
        reader
            .seek(SeekFrom::Start(self.slct_offset as u64))
            .unwrap();
        reader.read_le().unwrap()
    }
}
