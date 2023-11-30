//! Model resources like shaders, geometry, or textures in `.wismt` files.
//!
//! XC3: `chr/{ch,en,oj,wp}/*.wismt`
use std::io::{Cursor, Seek};

use crate::{
    error::DecompressStreamError, mibl::Mibl, mxmd::TextureResources, parse_count32_offset32,
    parse_ptr32, spch::Spch, vertex::VertexData, xbc1::Xbc1, xc3_write_binwrite_impl,
};
use bilge::prelude::*;
use binrw::{binread, BinRead, BinWrite};
use xc3_write::{write_full, Xc3Write, Xc3WriteOffsets};

#[binread]
#[derive(Debug, Xc3Write)]
#[br(magic(b"DRSM"))]
#[xc3(magic(b"DRSM"))]
pub struct Msrd {
    /// Version `10001`
    pub version: u32,
    // rounded or aligned in some way?
    pub header_size: u32, // TODO: xbc1 offset - 16?
    pub offset: u32,      // TODO: Pointer to an inner type?

    // TODO: variable size?
    pub tag: u32, // 4097?

    pub stream_flags: StreamFlags,

    // TODO: This offset depends on flags like with mxmd models?
    /// Files contained within [streams](#structfield.streams).
    #[br(parse_with = parse_count32_offset32, offset = offset as u64)]
    #[xc3(count_offset(u32, u32))]
    pub stream_entries: Vec<StreamEntry>,

    /// A collection of [Xbc1] streams with decompressed layout
    /// specified in [stream_entries](#structfield.stream_entries).
    #[br(parse_with = parse_count32_offset32, offset = offset as u64)]
    #[xc3(count_offset(u32, u32))]
    pub streams: Vec<Stream>,

    /// The [StreamEntry] for [Self::extract_vertex_data] with [EntryType::Vertex].
    pub vertex_data_entry_index: u32,
    /// The [StreamEntry] for [Self::extract_shader_data] with [EntryType::Shader].
    pub shader_entry_index: u32,

    /// The [StreamEntry] for [Self::extract_low_textures] with [EntryType::LowTextures].
    pub low_textures_entry_index: u32,
    /// The [Stream] for [Self::extract_low_textures].
    pub low_textures_stream_index: u32,

    /// The [Stream] for [Self::extract_textures].
    pub textures_stream_index: u32,
    /// The first [StreamEntry] for [Self::extract_textures].
    pub textures_stream_entry_start_index: u32,
    /// The number of [StreamEntry] corresponding
    /// to the number of textures in [Self::extract_textures].
    pub textures_stream_entry_count: u32,

    #[br(args_raw(offset as u64))]
    pub texture_resources: TextureResources,

    // TODO: offset 76?
    // TODO: optional 16 bytes of padding?
    pub unks: [u32; 4],
}

/// A file contained in a [Stream].
#[derive(Debug, BinRead, BinWrite)]
pub struct StreamEntry {
    /// The offset in bytes for the decompressed data range in the stream.
    pub offset: u32,
    /// The size in bytes of the decompressed data range in the stream.
    pub size: u32,
    // TODO: index into textures or low textures?
    pub texture_index: u16, // TODO: what does this do?
    pub item_type: EntryType,
    // TODO: padding?
    pub unk: [u32; 2],
}

/// Flags indicating what stream data is present.
#[bitsize(32)]
#[derive(DebugBits, FromBits, BinRead, BinWrite, Clone, Copy)]
#[br(map = u32::into)]
#[bw(map = |&x| u32::from(x))]
pub struct StreamFlags {
    pub has_vertex: bool,
    pub has_spch: bool,
    pub has_low_textures: bool,
    pub has_textures: bool,
    pub unk5: bool,
    pub unk6: bool,
    pub has_texture_resources: bool,
    pub unk: u25,
}

/// The type of data for a [StreamEntry].
#[derive(Debug, BinRead, BinWrite, PartialEq, Eq)]
#[brw(repr(u16))]
pub enum EntryType {
    /// A single [VertexData].
    Vertex = 0,
    /// A single [Spch].
    Shader = 1,
    /// A collection of [Mibl].
    LowTextures = 2,
    /// A single [Mibl].
    Texture = 3,
}

/// A compressed [Xbc1] stream with items determined by [StreamEntry].
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct Stream {
    pub compressed_size: u32,   // TODO: rounded up?
    pub decompressed_size: u32, // TODO: slightly larger than xbc1 decomp size?
    // TODO: Why does this sometimes have an extra 16 bytes of padding?
    #[br(parse_with = parse_ptr32)]
    #[xc3(offset(u32))]
    pub xbc1: Xbc1,
}

impl Msrd {
    pub fn decompress_stream(
        &self,
        stream_index: u32,
        entry_index: u32,
    ) -> Result<Vec<u8>, DecompressStreamError> {
        let stream = &self.streams[stream_index as usize].xbc1.decompress()?;
        let entry = &self.stream_entries[entry_index as usize];
        Ok(stream[entry.offset as usize..entry.offset as usize + entry.size as usize].to_vec())
    }

    pub fn extract_vertex_data(&self) -> Result<VertexData, DecompressStreamError> {
        // TODO: is this always in the first stream?
        let bytes = self.decompress_stream(0, self.vertex_data_entry_index)?;
        VertexData::from_bytes(bytes).map_err(Into::into)
    }

    pub fn extract_low_textures(&self) -> Result<Vec<Mibl>, DecompressStreamError> {
        let bytes = self.decompress_stream(
            self.low_textures_stream_index,
            self.low_textures_entry_index,
        )?;

        match &self.texture_resources.low_textures {
            Some(low_textures) => low_textures
                .textures
                .iter()
                .map(|t| {
                    let mibl_bytes = &bytes
                        [t.mibl_offset as usize..t.mibl_offset as usize + t.mibl_length as usize];
                    Mibl::from_bytes(mibl_bytes).map_err(Into::into)
                })
                .collect(),
            None => Ok(Vec::new()),
        }
    }

    pub fn extract_textures(&self) -> Result<Vec<Mibl>, DecompressStreamError> {
        // The textures are packed into a single stream.
        let stream = &self.streams[self.textures_stream_index as usize]
            .xbc1
            .decompress()?;

        let start = self.textures_stream_entry_start_index as usize;
        let count = self.textures_stream_entry_count as usize;
        self.stream_entries[start..start + count]
            .iter()
            .map(|entry| {
                let bytes =
                    &stream[entry.offset as usize..entry.offset as usize + entry.size as usize];
                Mibl::from_bytes(bytes).map_err(Into::into)
            })
            .collect::<Result<_, _>>()
    }

    pub fn extract_shader_data(&self) -> Result<Spch, DecompressStreamError> {
        // TODO: is this always in the first stream?
        let bytes = self.decompress_stream(0, self.shader_entry_index)?;
        Spch::from_bytes(bytes).map_err(Into::into)
    }

    /// Pack and compress the files into a new [Msrd] archive file.
    pub fn from_unpacked_files(vertex: &VertexData, spch: &Spch, low_textures: &Vec<Mibl>) -> Self {
        // TODO: handle other streams.
        let (stream_entries, stream0) = create_stream0(vertex, spch, low_textures);

        let xbc1 = Xbc1::from_decompressed("0000".to_string(), &stream0).unwrap();
        let stream = Stream {
            compressed_size: xbc1.compressed_size,
            decompressed_size: xbc1.decompressed_size,
            xbc1,
        };

        // TODO: Search stream entries to get indices?
        Self {
            version: 10001,
            header_size: 976, // TODO: calculate this during writing
            offset: 16,
            tag: 4097,
            stream_flags: StreamFlags::new(
                true,
                true,
                true,
                false,
                false,
                false,
                false,
                0u8.into(),
            ),
            stream_entries,
            streams: vec![stream],
            vertex_data_entry_index: 0,
            shader_entry_index: 1,
            low_textures_entry_index: 2,
            low_textures_stream_index: 0,
            textures_stream_index: 0,
            textures_stream_entry_start_index: 0,
            textures_stream_entry_count: 0,
            // TODO: How to properly create these fields?
            texture_resources: TextureResources {
                texture_indices: todo!(),
                low_textures: todo!(),
                unk1: 0,
                chr_textures: todo!(),
            },
            unks: [0; 4],
        }
    }
}

pub fn create_stream0(
    vertex: &VertexData,
    spch: &Spch,
    low_textures: &Vec<Mibl>,
) -> (Vec<StreamEntry>, Vec<u8>) {
    // Data in streams is tightly packed.
    let mut writer = Cursor::new(Vec::new());
    let entries = vec![
        write_stream_data(&mut writer, vertex, EntryType::Vertex),
        write_stream_data(&mut writer, spch, EntryType::Shader),
        write_stream_data(&mut writer, low_textures, EntryType::LowTextures),
    ];

    (entries, writer.into_inner())
}

fn write_stream_data<'a, T>(
    writer: &mut Cursor<Vec<u8>>,
    data: &'a T,
    item_type: EntryType,
) -> StreamEntry
where
    T: Xc3Write + 'static,
    T::Offsets<'a>: Xc3WriteOffsets,
{
    let offset = writer.stream_position().unwrap() as u32;
    write_full(data, writer, 0, &mut 0).unwrap();
    let end_offset = writer.stream_position().unwrap() as u32;

    StreamEntry {
        offset,
        size: end_offset - offset,
        texture_index: 0,
        item_type,
        unk: [0; 2],
    }
}

xc3_write_binwrite_impl!(StreamEntry, StreamFlags);

impl<'a> Xc3WriteOffsets for MsrdOffsets<'a> {
    fn write_offsets<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        _base_offset: u64,
        data_ptr: &mut u64,
    ) -> xc3_write::Xc3Result<()> {
        // TODO: Rework the msrd types to handle this.
        let base_offset = 16;

        // Write offset data in the order items appear in the binary file.
        self.stream_entries
            .write_offset(writer, base_offset, data_ptr)?;

        let stream_offsets = self.streams.write_offset(writer, base_offset, data_ptr)?;

        self.texture_resources
            .write_offsets(writer, base_offset, data_ptr)?;

        // TODO: Variable padding?
        for offsets in stream_offsets.0 {
            offsets.xbc1.write_offset(writer, 0, data_ptr)?;
        }

        Ok(())
    }
}
