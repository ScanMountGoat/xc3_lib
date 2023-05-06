use crate::parse_string_ptr;
use binrw::{args, binread, FilePtr32};

/// .wismt files
#[binread]
#[derive(Debug)]
#[br(magic(b"DRSM"))]
pub struct Drsm {
    version: u32,
    header_size: u32,

    #[br(temp)]
    offset: u32,

    tag: u32,
    revision: u32,

    #[br(temp)]
    data_items_count: u32,

    #[br(parse_with = FilePtr32::parse, offset = offset as u64)]
    #[br(args { inner: args!(count: data_items_count as usize) })]
    pub data_items: Vec<DataItem>,

    #[br(temp)]
    toc_count: u32,

    #[br(parse_with = FilePtr32::parse, offset = 16)]
    #[br(args { inner: args!(count: toc_count as usize) })]
    pub tocs: Vec<Toc>,

    unknown1: [u8; 28],

    #[br(temp)]
    texture_id_count: u32,

    #[br(parse_with = FilePtr32::parse, offset = 16)]
    #[br(args { inner: args!(count: texture_id_count as usize) })]
    texture_ids: Vec<u16>,

    #[br(parse_with = FilePtr32::parse, offset = 16)]
    pub texture_name_table: TextureNameTable,
}

#[binread]
#[derive(Debug)]
pub struct DataItem {
    pub offset: u32,
    pub size: u32,
    pub toc_index: u16,
    pub item_type: DataItemType,
    unk: [u8; 8],
}

#[binread]
#[br(repr(u16))]
#[derive(Debug)]
pub enum DataItemType {
    Model = 0,
    ShaderBundle = 1,
    CachedTexture = 2,
    Texture = 3,
}

#[binread]
#[derive(Debug)]
#[br(stream = r)]
pub struct TextureNameTable {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    count: u32,
    unk0: u32,
    unk1: u32,
    unk2: u32,

    // Same order as the data in the wimdo file?
    #[br(args { count: count as usize, inner: (base_offset,) })]
    pub textures: Vec<TextureInfo>,
}

#[binread]
#[derive(Debug)]
#[br(import(base_offset: u64))]
pub struct TextureInfo {
    unk1: u16,
    unk2: u16,
    pub size: u32,
    pub offset: u32,
    // Same as the file names in chr/tex/nx/m and chr/tex/nx/h?
    #[br(parse_with = parse_string_ptr, args(base_offset))]
    pub name: String,
}

#[binread]
#[derive(Debug)]
#[br(magic(b"xbc1"))]
pub struct Xbc1 {
    unk1: u32,
    pub decomp_size: u32,
    pub comp_size: u32,
    unk2: u32,
    #[br(pad_after = 24)]
    unk3: u32,
    #[br(count = comp_size)]
    pub deflate_stream: Vec<u8>,
}

// TODO: what does toc stand for?
#[binread]
#[derive(Debug)]
pub struct Toc {
    comp_size: u32,
    decomp_size: u32, // slightly larger than xbc1 decomp size?
    #[br(parse_with = FilePtr32::parse)]
    pub xbc1: Xbc1,
}
