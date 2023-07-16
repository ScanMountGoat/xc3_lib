use std::io::Cursor;

use crate::{
    mibl::Mibl,
    mxmd::PackedExternalTextures,
    parse_count_offset, parse_opt_ptr32, parse_ptr32,
    spch::Spch,
    vertex::VertexData,
    write::{xc3_write_binwrite_impl, Xc3Write},
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
    #[xc3(offset)]
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

#[derive(BinRead, BinWrite, Debug)]
pub struct StreamEntry {
    pub offset: u32,
    pub size: u32,
    pub unk_index: u16, // TODO: what does this do?
    pub item_type: EntryType,
    // TODO: padding?
    pub unk: [u32; 2],
}

#[derive(BinRead, BinWrite, Debug, PartialEq, Eq)]
#[brw(repr(u16))]
pub enum EntryType {
    Model = 0,
    Shader = 1,
    PackedTexture = 2,
    Texture = 3,
}

#[derive(BinRead, Xc3Write, Debug)]
pub struct Stream {
    pub comp_size: u32,
    pub decomp_size: u32, // TODO: slightly larger than xbc1 decomp size?
    #[br(parse_with = parse_ptr32)]
    #[xc3(offset)]
    pub xbc1: Xbc1,
}

#[derive(BinRead, BinWrite, Debug)]
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
                Mibl::from_bytes(bytes)
            })
            .collect()
    }

    pub fn extract_shader_data(&self) -> Spch {
        // TODO: is this always in the first stream?
        let bytes = self.decompress_stream(0, self.shader_entry_index);
        Spch::read(&mut Cursor::new(bytes)).unwrap()
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

xc3_write_binwrite_impl!(TextureResource, StreamEntry);

// TODO: Generate this with a macro rules macro?
// TODO: Include this in some sort of trait?
pub fn write_msrd<W: std::io::Write + std::io::Seek>(msrd: &Msrd, writer: &mut W) -> BinResult<()> {
    let mut data_ptr = 0;

    let msrd_offsets = msrd.write(writer, &mut data_ptr)?;

    // TODO: Rework the msrd types to handle this.
    let base_offset = 16;

    // Write offset data in the order items appear in the binary file.
    msrd_offsets
        .stream_entries
        .write_offset(writer, base_offset, &mut data_ptr)?;

    let stream_offsets = msrd_offsets
        .streams
        .write_offset(writer, base_offset, &mut data_ptr)?;

    msrd_offsets
        .texture_resources
        .write_offset(writer, base_offset, &mut data_ptr)?;

    msrd_offsets
        .texture_ids
        .write_offset(writer, base_offset, &mut data_ptr)?;

    // TODO: Store the base offset with the offsets themselves?
    // TODO: This would allow making Offset impl Xc3Write?
    // TODO: Every field in an offset type shares a base offset?
    if let Some(msrd_textures_offset) =
        msrd_offsets
            .textures
            .write_offset(writer, base_offset, &mut data_ptr)?
    {
        let textures_offsets = msrd_textures_offset.textures.write_offset(
            writer,
            msrd_textures_offset.base_offset,
            &mut data_ptr,
        )?;

        for offsets in textures_offsets {
            offsets
                .name
                .write_offset(writer, msrd_textures_offset.base_offset, &mut data_ptr)?;
        }
    }

    for offsets in stream_offsets {
        offsets.xbc1.write_offset(writer, 0, &mut data_ptr)?;
    }

    Ok(())
}
