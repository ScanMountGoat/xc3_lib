//! Model resources like shaders, geometry, or textures in `.wismt` files.
use crate::{
    mibl::Mibl,
    mxmd::PackedExternalTextures,
    parse_count_offset, parse_opt_ptr32, parse_ptr32,
    spch::Spch,
    vertex::VertexData,
    write::{xc3_write_binwrite_impl, Xc3Write, Xc3WriteFull},
    xbc1::Xbc1,
};
use binrw::{binread, BinRead, BinResult, BinWrite};

/// .wismt model files in `chr/bt`, `chr/ch/`, `chr/en`, `chr/oj`, and `chr/wp`.
#[binread]
#[derive(Debug, Xc3Write)]
#[br(magic(b"DRSM"))]
#[xc3(magic(b"DRSM"))]
pub struct Msrd {
    pub version: u32,
    pub header_size: u32, // xbc1 offset - 16?

    // TODO: Pointer to an inner type?
    offset: u32,

    pub tag: u32, // 4097?
    // TODO: This affects the fields in the file?
    pub revision: u32,

    #[br(parse_with = parse_count_offset, offset = offset as u64)]
    #[xc3(count_offset)]
    pub stream_entries: Vec<StreamEntry>,

    #[br(parse_with = parse_count_offset, offset = offset as u64)]
    #[xc3(count_offset)]
    pub streams: Vec<Stream>,

    pub vertex_data_entry_index: u32,
    pub shader_entry_index: u32,
    pub low_textures_entry_index: u32,
    pub low_textures_stream_index: u32,
    pub middle_textures_stream_index: u32,
    pub middle_textures_stream_entry_start_index: u32,
    pub middle_textures_stream_entry_count: u32,

    // TODO: identical to indices in mxmd?
    #[br(parse_with = parse_count_offset, offset = offset as u64)]
    #[xc3(count_offset)]
    pub texture_ids: Vec<u16>,

    #[br(parse_with = parse_opt_ptr32, offset = offset as u64)]
    #[xc3(offset, align(2))]
    pub textures: Option<PackedExternalTextures>,

    pub unk1: u32,

    // TODO: Same count as textures?
    // TODO: This doesn't work for pc000101.wismt?
    #[br(parse_with = parse_count_offset, offset = offset as u64)]
    #[xc3(count_offset)]
    pub texture_resources: Vec<TextureResource>,

    // TODO: padding:
    pub unk: [u32; 4],
}

#[derive(Debug, BinRead, BinWrite)]
pub struct StreamEntry {
    pub offset: u32,
    pub size: u32,
    pub unk_index: u16, // TODO: what does this do?
    pub item_type: EntryType,
    // TODO: padding?
    pub unk: [u32; 2],
}

#[derive(Debug, BinRead, BinWrite, PartialEq, Eq)]
#[brw(repr(u16))]
pub enum EntryType {
    Model = 0,
    Shader = 1,
    PackedTexture = 2,
    Texture = 3,
}

#[derive(Debug, BinRead, Xc3Write)]
pub struct Stream {
    pub comp_size: u32,
    pub decomp_size: u32, // TODO: slightly larger than xbc1 decomp size?
    // TODO: Why does this sometimes have an extra 16 bytes of padding?
    #[br(parse_with = parse_ptr32)]
    #[xc3(offset)]
    pub xbc1: Xbc1,
}

#[derive(Debug, BinRead, Xc3Write)]
pub struct TextureResource {
    // TODO: The the texture name hash as an integer?
    pub hash: u32,
    pub unk2: u32,
    pub unk3: u32,
    pub unk4: u32,
    pub unk5: u32,
}

impl Msrd {
    // TODO: Avoid unwrap.
    pub fn extract_vertex_data(&self) -> VertexData {
        // TODO: is this always in the first stream?
        let bytes = self.decompress_stream(0, self.vertex_data_entry_index);
        VertexData::from_bytes(&bytes).unwrap()
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
                Mibl::from_bytes(bytes).unwrap()
            })
            .collect()
    }

    pub fn extract_shader_data(&self) -> Spch {
        // TODO: is this always in the first stream?
        let bytes = self.decompress_stream(0, self.shader_entry_index);
        Spch::from_bytes(&bytes).unwrap()
    }

    pub fn decompress_stream(&self, stream_index: u32, entry_index: u32) -> Vec<u8> {
        let entry = &self.stream_entries[entry_index as usize];
        let stream = &self.streams[stream_index as usize]
            .xbc1
            .decompress()
            .unwrap();
        stream[entry.offset as usize..entry.offset as usize + entry.size as usize].to_vec()
    }
}

xc3_write_binwrite_impl!(StreamEntry);

impl<'a> Xc3WriteFull for MsrdOffsets<'a> {
    fn write_full<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        _base_offset: u64,
        data_ptr: &mut u64,
    ) -> BinResult<()> {
        // TODO: Rework the msrd types to handle this.
        let base_offset = 16;

        // Write offset data in the order items appear in the binary file.
        self.stream_entries
            .write_offset(writer, base_offset, data_ptr)?;

        let stream_offsets = self.streams.write_offset(writer, base_offset, data_ptr)?;

        self.texture_resources
            .write_offset(writer, base_offset, data_ptr)?;

        self.texture_ids
            .write_offset(writer, base_offset, data_ptr)?;

        self.textures.write_full(writer, base_offset, data_ptr)?;

        for offsets in stream_offsets.0 {
            offsets.xbc1.write_offset(writer, 0, data_ptr)?;
        }

        Ok(())
    }
}
