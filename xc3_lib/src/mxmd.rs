use std::io::SeekFrom;

use crate::parse_ptr32;
use binrw::{args, binread, BinRead, BinResult, FilePtr32, NamedArgs, NullString};
use serde::Serialize;

/// .wimdo files
#[binread]
#[derive(Debug, Serialize)]
#[br(magic(b"DMXM"))]
pub struct Mxmd {
    version: u32,

    #[br(parse_with = FilePtr32::parse)]
    pub mesh: Mesh,

    #[br(parse_with = FilePtr32::parse)]
    pub materials: Materials,

    unk1: u32, // points after the texture names?
    unk2: u32,
    unk3: u32,
    unk4: u32,
    unk5: u32,

    // uncached textures?
    #[br(parse_with = FilePtr32::parse)]
    pub textures: Textures,
}

#[binread]
#[derive(Debug, Serialize)]
#[br(stream = r)]
pub struct Materials {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    #[br(args { base_offset, inner: base_offset })]
    pub materials: List<Material>,

    unk1: u32,
    unk2: u32,

    #[br(args { base_offset })]
    floats: List<f32>,

    #[br(args { base_offset })]
    ints: List<u32>,

    // TODO: what type is this?
    unk3: u32,
    unk4: u32,

    // TODO: How large is each element?
    #[br(args { base_offset })]
    unks: List<[u16; 8]>,

    unk: [u32; 16],
}

#[binread]
#[derive(Debug, Serialize)]
#[br(import_raw(base_offset: u64))]
pub struct Material {
    #[br(parse_with = parse_string_ptr32, args(base_offset))]
    pub name: String,

    unk1: u16,
    unk2: u16,
    unk3: u16,
    unk4: u16,

    unks1: [f32; 5],

    // TODO: materials with zero textures?
    /// Defines the shader's sampler bindings in order for s0, s1, s2, ...
    #[br(args { base_offset })]
    pub textures: List<Texture>,

    // TODO: are these sampler parameters?
    pub unk_flag1: [u8; 4],
    pub unk_flag2: [u8; 4],

    m_unks1: [u32; 6],

    m_unk5: u32,

    // always count 1?
    #[br(args { base_offset })]
    pub shader_programs: List<ShaderProgram>,

    m_unks2: [u32; 8],
}

#[binread]
#[derive(Debug, Serialize)]
pub struct ShaderProgram {
    pub program_index: u32, // index into programs in wismt?
    pub unk_type: ShaderUnkType,
    pub parent_material_index: u16, // index of the parent material?
    pub unk4: u32,                  // always 1?
}

// Affects what pass the object renders in?
// Each "pass" has different render targets?
// _trans = 1,
// _ope = 0,1,7
// _zpre = 0
// _outline = 0
#[binread]
#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize)]
#[br(repr(u16))]
pub enum ShaderUnkType {
    Unk0 = 0, // main opaque + some transparent?
    Unk1 = 1, // second layer transparent?
    Unk7 = 7, // additional eye effect layer?
}

#[binread]
#[derive(Debug, Serialize)]
pub struct Texture {
    pub texture_index: u16,
    pub unk1: u16,
    pub unk2: u16,
    pub unk3: u16,
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

    #[br(args { base_offset, inner: base_offset })]
    pub items: List<DataItem>,

    unk2: u32,
    bone_offset: u32, // relative to start of model?

    unks3: [u32; 24],

    #[br(parse_with = parse_ptr32, args(base_offset))]
    lod_data: Option<LodData>,
}

// TODO: Better names for these types
#[binread]
#[derive(Debug, Serialize)]
#[br(import_raw(base_offset: u64))]
pub struct DataItem {
    #[br(args { base_offset })]
    pub sub_items: List<SubDataItem>,
    unk1: u32,
    max_xyz: [f32; 3],
    min_xyz: [f32; 3],
    bounding_radius: f32,
    unks: [u32; 7],
}

// TODO: Better names for these types
#[binread]
#[derive(Debug, Serialize)]
pub struct SubDataItem {
    flags1: u32,
    flags2: u32,
    pub vertex_buffer_index: u16,
    pub index_buffer_index: u16,
    unk_index: u16,
    pub material_index: u16,
    unk2: u32,
    unk3: u32,
    unk4: u32,
    unk5: u16,
    pub lod: u16,
    // TODO: groups?
    unks6: [i32; 4],
}

#[binread]
#[derive(Debug, Serialize)]
#[br(stream = r)]
pub struct LodData {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    unk1: u32,

    // another list?
    unk2: u32,
    unk3: u32,

    #[br(args { base_offset })]
    items: List<(u16, u16)>,
}

#[binread]
#[derive(Debug, Serialize)]
#[br(stream = r)]
pub struct Textures {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    unks: [u32; 15],

    #[br(parse_with = parse_ptr32, args(base_offset))]
    pub items: Option<TextureItems>,
}

#[binread]
#[derive(Debug, Serialize)]
#[br(stream = r)]
pub struct TextureItems {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    count: u32,
    offset: u32,
    unk2: u32,
    strings_offset: u32,

    #[br(args { count: count as usize, inner: args! { base_offset } })]
    pub textures: Vec<TextureItem>,
}

#[binread]
#[derive(Debug, Serialize)]
#[br(import { base_offset: u64 })]
pub struct TextureItem {
    unk1: u16,
    unk2: u16,
    unk3: u16, // size?
    unk4: u16,
    unk5: u16, // some sort of offset (sum of previous unk3)?
    unk6: u16,

    #[br(parse_with = parse_string_ptr32, args(base_offset))]
    pub name: String,
}

// TODO: type for this shared with hpcs?
fn parse_string_ptr32<R: std::io::Read + std::io::Seek>(
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
pub struct ListArgs<Inner: Default> {
    #[named_args(default = 0)]
    base_offset: u64,
    #[named_args(default = Inner::default())]
    inner: Inner,
}

#[binread]
#[derive(Debug, Serialize)]
#[br(import_raw(args: ListArgs<T::Args<'_>>))]
#[serde(transparent)]
pub struct List<T>
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
    pub elements: Vec<T>,
}
