use binrw::{args, binread, BinReaderExt, FilePtr32};
use flate2::bufread::ZlibDecoder;

// wismt/msrd format referece:
// https://github.com/Turk645/Xenoblade-Switch-Model-Importer-Noesis/blob/main/fmt_wismt.py
// https://github.com/BlockBuilder57/XB2AssetTool/blob/master/include/xb2at/structs/msrd.h
// https://github.com/BlockBuilder57/XB2AssetTool/blob/master/src/core/readers/msrd_reader.cpp
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
pub struct TextureNameTable {
    count: u32,
    unk0: u32, // names offset relative to start of this struct?
    unk1: u32,
    unk2: u32,
    // TODO: texture names?
    #[br(count = count)]
    pub textures: Vec<TextureInfo>,
}

#[binread]
#[derive(Debug)]
pub struct TextureInfo {
    unk1: u32,
    pub size: u32,
    pub offset: u32,
    name_offset: u32, // relative to start of TextureNameTable?
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
