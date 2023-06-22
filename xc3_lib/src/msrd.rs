use std::io::Cursor;

use crate::{
    mibl::Mibl, parse_count_offset, parse_ptr32, parse_string_ptr32, spch::Spch,
    vertex::VertexData, xbc1::Xbc1,
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
    pub low_textures_entry_index: u32,
    pub low_textures_stream_index: u32,
    pub middle_textures_stream_index: u32,
    pub middle_textures_stream_entry_start_index: u32,
    pub middle_textures_stream_entry_count: u32,

    #[br(parse_with = parse_count_offset, args_raw(offset as u64))]
    texture_ids: Vec<u16>,

    #[br(parse_with = parse_ptr32, args_raw(offset as u64))]
    pub textures: Option<Textures>,
}

#[binread]
#[derive(Debug, Serialize)]
pub struct StreamEntry {
    pub offset: u32,
    pub size: u32,
    pub unk_index: u16, // TODO: what does this do?
    pub item_type: EntryType,
    // TODO: padding?
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
pub struct Textures {
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
    decomp_size: u32, // TODO: slightly larger than xbc1 decomp size?
    #[br(parse_with = FilePtr32::parse)]
    pub xbc1: Xbc1,
}

impl Msrd {
    // TODO: Avoid unwrap.
    pub fn extract_vertex_data(&self) -> VertexData {
        // TODO: is this always in the first stream?
        let bytes = self.decompress_stream(0, self.model_entry_index);
        VertexData::read(&mut Cursor::new(bytes)).unwrap()
    }

    // TODO: Return mibl instead?
    pub fn extract_low_texture_data(&self) -> Vec<u8> {
        self.decompress_stream(
            self.low_textures_stream_index,
            self.low_textures_entry_index,
        )
    }

    pub fn extract_middle_textures(&self) -> Vec<Mibl> {
        // The middle textures are packed into a single stream.
        // TODO: Where are the high textures?
        let stream = &self.streams[self.middle_textures_stream_index as usize]
            .xbc1
            .decompress()
            .unwrap();

        let start = self.middle_textures_stream_entry_start_index as usize;
        let count = self.middle_textures_stream_entry_count as usize;
        self.stream_entries[start..start + count]
            .iter()
            .map(|entry| {
                let bytes =
                    &stream[entry.offset as usize..entry.offset as usize + entry.size as usize];
                Mibl::read(&mut Cursor::new(bytes)).unwrap()
            })
            .collect()
    }

    pub fn extract_shader_data(&self) -> Spch {
        // TODO: is this always in the first stream?
        let bytes = self.decompress_stream(0, self.shader_entry_index);
        Spch::read(&mut Cursor::new(bytes)).unwrap()
    }

    fn decompress_stream(&self, stream_index: u32, entry_index: u32) -> Vec<u8> {
        let entry = &self.stream_entries[entry_index as usize];
        let stream = &self.streams[stream_index as usize]
            .xbc1
            .decompress()
            .unwrap();
        stream[entry.offset as usize..entry.offset as usize + entry.size as usize].to_vec()
    }
}
