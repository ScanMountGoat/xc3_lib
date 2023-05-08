use std::io::SeekFrom;

use binrw::{args, binread, BinRead, BinResult, FilePtr32, NamedArgs, NullString};
use serde::Serialize;

/// .wimdo files
#[binread]
#[derive(Debug, Serialize)]
#[br(magic(b"DMXM"))]
pub struct Mxmd {
    version: u32,

    #[br(parse_with = FilePtr32::parse)]
    mesh: Mesh,

    #[br(parse_with = FilePtr32::parse)]
    materials: Materials,

    unk1: u32, // points after the texture names?
    unk2: u32,
    unk3: u32,
    unk4: u32,
    unk5: u32,

    // uncached textures?
    #[br(parse_with = FilePtr32::parse)]
    textures: Textures,
}

#[binread]
#[derive(Debug, Serialize)]
#[br(stream = r)]
pub struct Materials {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    #[br(args { base_offset, inner: base_offset })]
    materials: Container<Material>,

    unk1: u32,
    unk2: u32,

    #[br(args { base_offset })]
    floats: Container<f32>,

    #[br(args { base_offset })]
    ints: Container<u32>,

    // TODO: what type is this?
    unk3: u32,
    unk4: u32,

    // TODO: How large is each element?
    #[br(args { base_offset })]
    unks: Container<[u16; 8]>,

    unk: [u32; 16],
}

#[binread]
#[derive(Debug, Serialize)]
#[br(import_raw(base_offset: u64))]
pub struct Material {
    #[br(parse_with = parse_string_ptr, args(base_offset))]
    name: String,

    unk1: u16,
    unk2: u16,
    unk3: u16,
    unk4: u16,

    unks1: [f32; 5],

    // Fills in bindings in order in shader?
    // TODO: Some shader samplers use other textures?
    // TODO: do these fill in s0, s1, etc in order?
    // TODO: zprepass doesn't use these textures?
    #[br(args { base_offset })]
    textures: Container<Texture>,

    m_unks1: [u32; 8],

    m_unk5: u32,

    // always count 1?
    #[br(args { base_offset })]
    shader_programs: Container<ShaderProgram>,

    m_unks2: [u32; 8],
}

#[binread]
#[derive(Debug, Serialize)]
pub struct ShaderProgram {
    program_index: u32, // index into programs in wismt?
    unk2: u16,
    unk3: u16,
    unk4: u32,
}

#[binread]
#[derive(Debug, Serialize)]
pub struct Texture {
    texture_index: u16,
    unk1: u16,
    unk2: u16,
    unk3: u16,
}

#[binread]
#[derive(Debug, Serialize)]
#[br(stream = r)]
pub struct Mesh {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    unk1: u32,

    max_xyz: [f32; 3],
    min_xyz: [f32; 3],

    #[br(args { base_offset })]
    items: Container<DataItem>,

    unk2: u32,
    bone_offset: u32, // relative to start of mesh
}

// TODO: Padding?
#[binread]
#[derive(Debug, Serialize)]
#[br(stream = r)]
pub struct DataItem {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    unk1: u32,
    #[br(args { base_offset })]
    sub_items: Container<SubDataItem>,
}

#[binread]
#[derive(Debug, Serialize)]
pub struct SubDataItem {
    unk1: u32,
    flag: u32,
    vertex_buffer_index: i16,
    index_buffer_index: i16, // TODO: why is this sometimes invalid?
    unk_index: i16,
    material_index: i16,
    unk2: i16,
    unk3: i16,
    unk4: i16,
    unk5: i16,
    unk6: i16,
    unk7: i16,
    unk8: i16,
    unk9: i16,
    unks: [i16; 8],
}

#[binread]
#[derive(Debug, Serialize)]
#[br(stream = r)]
pub struct Textures {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    unks: [u32; 15],

    #[br(parse_with = FilePtr32::parse, offset = base_offset)]
    items: TextureItems,
}

#[binread]
#[derive(Debug, Serialize)]
#[br(stream = r)]
pub struct TextureItems {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    count: u32,
    unk1: u32,
    unk2: u32,

    // TODO: Why is the first element repeated?
    #[br(args { count: count as usize + 1, inner: args! { base_offset } })]
    textures: Vec<TextureItem>,
}

#[binread]
#[derive(Debug, Serialize)]
#[br(import { base_offset: u64 })]
pub struct TextureItem {
    #[br(parse_with = parse_string_ptr, args(base_offset))]
    name: String,
    unk1: u16,
    unk2: u16,
    unk3: u16,
    unk4: u16,
    unk5: u16,
    unk6: u16,
}

// TODO: type for this shared with hpcs?
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

/// A [u32] offset and [u32] count with an optional base offset.
#[derive(Clone, NamedArgs)]
struct ContainerArgs<Inner: Default> {
    #[named_args(default = 0)]
    base_offset: u64,
    #[named_args(default = Inner::default())]
    inner: Inner,
}

#[binread]
#[derive(Debug, Serialize)]
#[br(import_raw(args: ContainerArgs<T::Args<'_>>))]
#[serde(transparent)]
struct Container<T>
where
    T: BinRead + 'static,
    for<'a> <T as BinRead>::Args<'a>: Clone + Default,
{
    #[br(temp)]
    offset: u32,
    #[br(temp)]
    count: u32,

    #[br(args { count: count as usize, inner: args.inner })]
    #[br(seek_before = SeekFrom::Start(args.base_offset + offset as u64))]
    #[br(restore_position)]
    elements: Vec<T>,
}
