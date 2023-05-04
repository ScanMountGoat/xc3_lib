use std::io::SeekFrom;

use binrw::{args, binread, BinRead, BinResult, FilePtr32, FilePtr64, NullString, PosValue};

#[binread]
#[derive(Debug)]
#[br(magic(b"HCPS"))]
pub struct Hpcs {
    version: u32,
    unk1: u32,
    count: u32, // shd{i} count?
    #[br(parse_with = FilePtr32::parse)]
    string_section: StringSection,
    unk4: u32,

    #[br(temp)]
    slct_base_offset: u32,

    unk6: u32,
    xv4_base_offset: u32, // pointer to first xV4 (shader binary)
    unk8: u32,
    unk9: u32,
    unk10: u32,
    #[br(pad_after = 20)]
    unk11: u32,

    // TODO: do these actually point to slcts?
    // u64 offsets to nvsd relative to 1364?
    // relative to end of header + header data?
    #[br(count = count)]
    #[br(args { inner: args! { base_offset: slct_base_offset as u64 }})]
    pub shader_programs: Vec<ShaderProgramOffset>,
}

#[binread]
#[derive(Debug)]
struct StringSection {
    count: u32, // same as header count?
    unk12: u32,
    unk13: u32,
    #[br(count = count)]
    string_pointers: Vec<u32>, // string pointers?
}

// TODO: Avoid creating another type for this?
#[binread]
#[derive(Debug)]
#[br(import { base_offset: u64 })]
pub struct ShaderProgramOffset {
    #[br(parse_with = FilePtr64::parse, offset = base_offset)]
    pub program: ShaderProgram,
}

#[binread]
#[derive(Debug)]
#[br(stream = r)]
pub struct ShaderProgram {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    slct: Slct,

    // TODO: DECL_GBL_CALC can make the slct bigger?
    #[br(try)]
    #[br(args { 
        string_offset: base_offset + slct.string_offset as u64,
        attribute_count: slct.attribute_count as usize,
        uniform_count: slct.uniform_count as usize,
    })]
    pub nvsd: Option<Nvsd>,
}

#[binread]
#[derive(Debug)]
#[br(magic(b"SLCT"))]
struct Slct {
    unk1: u32,
    unk2: u32,
    unk3: u32,
    unk4: u32,
    unk5: u32,
    unk6: u32,
    unk7: u32,          // offset?
    string_offset: u32, // base offset for strings relative to start of slct?
    unks1: [u32; 11],
    unk_offset1: u32,     // pointer to DECL_GBL_CALC
    unk_offset2: u32,     // pointer to after DECL_GBL_CALC
    unks2: [u32; 20],     // always 112 bytes?
    attribute_count: u32, // just inputs?
    unk9: u32,
    uniform_count: u32,
    unk11: u32,
    unks3: [u32; 4],
}

// TODO: figure out xv4 offsets and decompile with ryujinx
// This should make it easier to figure out the inputs
#[binread]
#[derive(Debug)]
#[br(magic(b"NVSD"))]
#[br(import { 
    string_offset: u64, 
    attribute_count: usize, 
    uniform_count: usize 
})]
pub struct Nvsd {
    version: u32,
    unk1: u32, // 0
    unk2: u32, // 0
    unk3: u32, // identical to xv4_size1?
    unk4: u32, // 0
    unk5: u32, // 2176
    unk6: u32, // 1

    // Each NVSD has its own compiled shaders?
    // Flattening out the NVSD sizes gives us the xV4 sizes at the end of the file?
    // TODO: Which one of these is fragment/vertex?
    pub xv4_size1: u32, // xv4 size
    pub xv4_size2: u32, // xv4 size

    unk9: u32,  // 2176
    unk10: u32, // 2176

    // TODO: How to determine these counts?
    #[br(args { string_offset })]
    unks2: [Unk2; 12],
    #[br(args { string_offset })]
    unks3: [Unk3; 7],

    #[br(args { count: attribute_count, inner: args! { string_offset } })]
    attributes: Vec<InputAttribute>,

    #[br(args { count: uniform_count, inner: args! { string_offset } })]
    uniforms: Vec<Uniform>,
}

#[binread]
#[derive(Debug)]
#[br(import { string_offset: u64 })]
struct Unk2 {
    #[br(parse_with = parse_string_ptr, args(string_offset))]
    name: String,
    unk1: u32,
    unk2: u32,
    unk3: u32,
}

#[binread]
#[derive(Debug)]
#[br(import { string_offset: u64 })]
struct Unk3 {
    #[br(parse_with = parse_string_ptr, args(string_offset))]
    name: String,
    unk1: u32,
    unk2: u32,
}

#[binread]
#[derive(Debug)]
#[br(import { string_offset: u64 })]
struct Uniform {
    #[br(parse_with = parse_string_ptr, args(string_offset))]
    name: String,
    unk1: u32,
}

#[binread]
#[derive(Debug)]
#[br(import { string_offset: u64 })]
struct InputAttribute {
    #[br(parse_with = parse_string_ptr, args(string_offset))]
    name: String,
    location: u32,
}

fn parse_string_ptr<R: std::io::Read + std::io::Seek>(
    reader: &mut R,
    endian: binrw::Endian,
    args: (u64,),
) -> BinResult<String> {
    let offset = u32::read_options(reader, endian, ())?;
    let saved_pos = reader.stream_position()?;

    reader.seek(SeekFrom::Start(args.0 + offset as u64))?;
    let value = NullString::read_options(reader, endian, ())?;
    reader.seek(SeekFrom::Start(saved_pos))?;

    Ok(value.to_string())
}
