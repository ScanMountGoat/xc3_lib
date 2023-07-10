use std::io::Cursor;

use crate::{
    mibl::Mibl,
    mxmd::PackedExternalTextures,
    parse_count_offset, parse_opt_ptr32, parse_ptr32,
    spch::Spch,
    vertex::VertexData,
    write::{write_offset, Xc3Write},
    xbc1::Xbc1,
};
use binrw::{binread, BinRead, BinResult, BinWrite};
use serde::Serialize;

/// .wismt model files in `chr/bt`, `chr/ch/`, `chr/en`, `chr/oj`, and `chr/wp`.
#[binread]
#[derive(Debug, Serialize)]
#[br(magic(b"DRSM"))]
pub struct Msrd {
    pub version: u32,
    pub header_size: u32, // xbc1 offset - 16?

    // TODO: Pointer to an inner type?
    #[br(temp)]
    offset: u32,

    pub tag: u32, // 4097?
    pub revision: u32,

    #[br(parse_with = parse_count_offset, offset = offset as u64)]
    pub stream_entries: Vec<StreamEntry>,

    #[br(parse_with = parse_count_offset, offset = offset as u64)]
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
    pub texture_ids: Vec<u16>,

    #[br(parse_with = parse_opt_ptr32, offset = offset as u64)]
    pub textures: Option<PackedExternalTextures>,

    pub unk1: u32,

    // TODO: Same count as textures?
    // TODO: This doesn't work for pc000101.wismt?
    #[br(parse_with = parse_count_offset, offset = offset as u64)]
    pub texture_resources: Vec<TextureResource>,

    // TODO: padding:
    pub unk: [u32; 4],
}

#[derive(BinRead, BinWrite, Debug, Serialize)]
pub struct StreamEntry {
    pub offset: u32,
    pub size: u32,
    pub unk_index: u16, // TODO: what does this do?
    pub item_type: EntryType,
    // TODO: padding?
    pub unk: [u32; 2],
}

#[derive(BinRead, BinWrite, Debug, Serialize, PartialEq, Eq)]
#[brw(repr(u16))]
pub enum EntryType {
    Model = 0,
    Shader = 1,
    PackedTexture = 2,
    Texture = 3,
}

#[derive(BinRead, Debug, Serialize)]
pub struct Stream {
    pub comp_size: u32,
    pub decomp_size: u32, // TODO: slightly larger than xbc1 decomp size?
    #[br(parse_with = parse_ptr32)] // TODO: always at the end of the file?
    pub xbc1: Xbc1,
}

#[derive(BinRead, BinWrite, Debug, Serialize)]
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

// TODO: Store the offsets in the type itself?
// TODO: Indicate that this is the position of the offset?
#[derive(Debug)]
pub(crate) struct MsrdOffsets {
    stream_entries: u64,
    streams: u64,
    texture_ids: u64,
    textures: u64,
    texture_resources: u64,
}

impl Xc3Write for TextureResource {
    type Offsets = ();

    fn write<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        data_ptr: &mut u64,
    ) -> BinResult<Self::Offsets> {
        let result = self.write_le(writer);
        *data_ptr = (*data_ptr).max(writer.stream_position()?);
        result
    }
}

impl Xc3Write for StreamEntry {
    type Offsets = ();

    fn write<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        data_ptr: &mut u64,
    ) -> BinResult<Self::Offsets> {
        let result = self.write_le(writer);
        *data_ptr = (*data_ptr).max(writer.stream_position()?);
        result
    }
}

pub(crate) struct StreamOffsets {
    xbc1: u64,
}

impl Xc3Write for Stream {
    type Offsets = StreamOffsets;

    fn write<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        data_ptr: &mut u64,
    ) -> BinResult<Self::Offsets> {
        self.comp_size.write_le(writer)?;
        self.decomp_size.write_le(writer)?;
        let xbc1 = writer.stream_position()?;
        0u32.write_le(writer)?;

        *data_ptr = (*data_ptr).max(writer.stream_position()?);
        Ok(StreamOffsets { xbc1 })
    }
}

pub(crate) struct PackedExternalTexturesOffsets {
    base_offset: u64,
    textures: u64,
}

impl Xc3Write for PackedExternalTextures {
    type Offsets = PackedExternalTexturesOffsets;

    fn write<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        data_ptr: &mut u64,
    ) -> BinResult<Self::Offsets> {
        let base_offset = writer.stream_position()?;

        (self.textures.len() as u32).write_le(writer)?;
        let textures = writer.stream_position()?;
        0u32.write_le(writer)?;

        self.unk2.write_le(writer)?;
        self.strings_offset.write_le(writer)?;

        *data_ptr = (*data_ptr).max(writer.stream_position()?);
        Ok(PackedExternalTexturesOffsets {
            base_offset,
            textures,
        })
    }
}

pub(crate) struct PackedExternalTextureOffsets {
    name: u64,
}

impl Xc3Write for crate::mxmd::PackedExternalTexture {
    type Offsets = PackedExternalTextureOffsets;

    fn write<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        data_ptr: &mut u64,
    ) -> BinResult<Self::Offsets> {
        self.unk1.write_le(writer)?;
        self.mibl_length.write_le(writer)?;
        self.mibl_offset.write_le(writer)?;

        let name = writer.stream_position()?;
        0u32.write_le(writer)?;

        *data_ptr = (*data_ptr).max(writer.stream_position()?);
        Ok(PackedExternalTextureOffsets { name })
    }
}

impl Xc3Write for Msrd {
    type Offsets = MsrdOffsets;

    // TODO: find a way to just use binwrite?
    fn write<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        data_ptr: &mut u64,
    ) -> BinResult<MsrdOffsets> {
        b"DRSM".write_le(writer)?;
        self.version.write_le(writer)?;
        self.header_size.write_le(writer)?;

        16u32.write_le(writer)?;

        self.tag.write_le(writer)?; // 4097?
        self.revision.write_le(writer)?;

        // TODO: Create a custom write trait?
        // TODO: Interior mutability on pointer types to store offset using cell?
        (self.stream_entries.len() as u32).write_le(writer)?;
        let stream_entries = writer.stream_position()?;
        0u32.write_le(writer)?;

        (self.streams.len() as u32).write_le(writer)?;
        let streams = writer.stream_position()?;
        0u32.write_le(writer)?;

        self.vertex_data_entry_index.write_le(writer)?;
        self.shader_entry_index.write_le(writer)?;
        self.low_textures_entry_index.write_le(writer)?;
        self.low_textures_stream_index.write_le(writer)?;
        self.middle_textures_stream_index.write_le(writer)?;
        self.middle_textures_stream_entry_start_index
            .write_le(writer)?;
        self.middle_textures_stream_entry_count.write_le(writer)?;

        (self.texture_ids.len() as u32).write_le(writer)?;
        let texture_ids = writer.stream_position()?;
        0u32.write_le(writer)?;

        let textures = writer.stream_position()?;
        0u32.write_le(writer)?;

        self.unk1.write_le(writer)?;

        (self.texture_resources.len() as u32).write_le(writer)?;
        let texture_resources = writer.stream_position()?;
        0u32.write_le(writer)?;

        self.unk.write_le(writer)?;

        *data_ptr = (*data_ptr).max(writer.stream_position()?);
        Ok(MsrdOffsets {
            stream_entries,
            streams,
            texture_ids,
            textures,
            texture_resources,
        })
    }
}

// TODO: Generate this with a macro?
// TODO: Include this in some sort of trait?
pub fn write_msrd<W: std::io::Write + std::io::Seek>(msrd: &Msrd, writer: &mut W) -> BinResult<()> {
    let mut data_ptr = 0;

    let msrd_offsets = msrd.write(writer, &mut data_ptr)?;

    // TODO: Rework the msrd types to handle this.
    let base_offset = 16;

    // Write offset data in the order items appear in the binary file.
    write_offset(
        writer,
        msrd_offsets.stream_entries,
        base_offset,
        &mut data_ptr,
        &msrd.stream_entries,
    )?;

    let stream_offsets = write_offset(
        writer,
        msrd_offsets.streams,
        base_offset,
        &mut data_ptr,
        &msrd.streams,
    )?;

    write_offset(
        writer,
        msrd_offsets.texture_resources,
        base_offset,
        &mut data_ptr,
        &msrd.texture_resources,
    )?;

    write_offset(
        writer,
        msrd_offsets.texture_ids,
        base_offset,
        &mut data_ptr,
        &msrd.texture_ids,
    )?;

    // TODO: Implement Xc3Write for Option?
    if let Some(textures) = &msrd.textures {
        let packed_external_textures_offsets = write_offset(
            writer,
            msrd_offsets.textures,
            base_offset,
            &mut data_ptr,
            textures,
        )?;

        let textures_offsets = write_offset(
            writer,
            packed_external_textures_offsets.textures,
            packed_external_textures_offsets.base_offset,
            &mut data_ptr,
            &textures.textures,
        )?;

        for (texture, offsets) in textures.textures.iter().zip(textures_offsets.iter()) {
            write_offset(
                writer,
                offsets.name,
                packed_external_textures_offsets.base_offset,
                &mut data_ptr,
                &texture.name,
            )?;
        }
    }

    for (stream, offsets) in msrd.streams.iter().zip(stream_offsets.iter()) {
        write_offset(writer, offsets.xbc1, 0, &mut data_ptr, &stream.xbc1)?;
    }

    Ok(())
}
