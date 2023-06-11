use std::io::Cursor;

use crate::{
    model::ModelData, parse_count_offset, parse_ptr32, parse_string_ptr32, spch::Spch, xbc1::Xbc1,
};
use binrw::{binread, FilePtr32};
use serde::Serialize;

/// .wismt model files in `chr/bt`, `chr/ch/`, `chr/en`, `chr/oj`, and `chr/wp`.
#[binread]
#[derive(Debug, Serialize)]
#[br(magic(b"DRSM"))]
pub struct Msrd {
    version: u32,
    header_size: u32,

    #[br(temp)]
    offset: u32,

    tag: u32,
    revision: u32,

    #[br(parse_with = parse_count_offset, args_raw(offset as u64))]
    pub stream_entries: Vec<StreamEntry>,

    #[br(parse_with = parse_count_offset, args_raw(offset as u64))]
    pub streams: Vec<Stream>,

    pub model_entry_index: u32,
    pub shader_entry_index: u32,
    pub texture_entry_index: u32,
    unk1: [u32; 4],

    #[br(parse_with = parse_count_offset, args_raw(offset as u64))]
    texture_ids: Vec<u16>,

    #[br(parse_with = parse_ptr32, args_raw(offset as u64))]
    pub texture_name_table: Option<TextureNameTable>,
}

#[binread]
#[derive(Debug, Serialize)]
pub struct StreamEntry {
    pub offset: u32,
    pub size: u32,
    pub stream_index: u16,
    pub item_type: EntryType,
    unk: [u8; 8],
}

#[binread]
#[br(repr(u16))]
#[derive(Debug, Serialize, PartialEq, Eq)]
pub enum EntryType {
    Model = 0,
    Shader = 1,
    CachedTexture = 2,
    Texture = 3,
}

#[binread]
#[derive(Debug, Serialize)]
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
#[derive(Debug, Serialize)]
#[br(import(base_offset: u64))]
pub struct TextureInfo {
    unk1: u16,
    unk2: u16,
    pub size: u32,
    pub offset: u32,
    // Same as the file names in chr/tex/nx/m and chr/tex/nx/h?
    #[br(parse_with = parse_string_ptr32, args(base_offset))]
    pub name: String,
}

#[binread]
#[derive(Debug, Serialize)]
pub struct Stream {
    comp_size: u32,
    decomp_size: u32, // slightly larger than xbc1 decomp size?
    #[br(parse_with = FilePtr32::parse)]
    pub xbc1: Xbc1,
}

impl Msrd {
    // TODO: Avoid unwrap.
    pub fn extract_model_data(&self) -> ModelData {
        let bytes = self.decompress_stream(self.model_entry_index);
        ModelData::read(&mut Cursor::new(bytes)).unwrap()
    }

    pub fn extract_texture_data(&self) -> Vec<u8> {
        self.decompress_stream(self.texture_entry_index)
    }

    pub fn extract_shader_data(&self) -> Spch {
        let bytes = self.decompress_stream(self.shader_entry_index);
        Spch::read(&mut Cursor::new(bytes)).unwrap()
    }

    fn decompress_stream(&self, entry_index: u32) -> Vec<u8> {
        let entry = &self.stream_entries[entry_index as usize];
        let stream = &self.streams[entry.stream_index as usize]
            .xbc1
            .decompress()
            .unwrap();
        stream[entry.offset as usize..entry.offset as usize + entry.size as usize].to_vec()
    }
}
