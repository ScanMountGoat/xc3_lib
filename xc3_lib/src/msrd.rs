//! Streamed model resources like shaders, geometry, or textures in `.wismt` files.
//!
//! # File Paths
//! | Game | File Patterns |
//! | --- | --- |
//! | Xenoblade Chronicles 1 DE | `chr/{en,np,obj,pc,wp}/*.wismt` |
//! | Xenoblade Chronicles 2 | `model/{bl,en,np,oj,pc,we,wp}/*.wismt` |
//! | Xenoblade Chronicles 3 | `chr/{bt,ch,en,oj,wp}/*.wismt`, `map/*.wismt` |
use std::io::{Cursor, Read, Seek, SeekFrom, Write};

use crate::{
    dds::DdsExt,
    mxmd::{PackedExternalTexture, PackedExternalTextures},
    parse_count32_offset32, parse_opt_ptr32, parse_ptr32,
    xbc1::Xbc1,
    xc3_write_binwrite_impl,
};
use bilge::prelude::*;
use binrw::{args, binread, until_eof, BinRead, BinResult, BinWrite};
use xc3_write::{round_up, write_full, Xc3Write, Xc3WriteOffsets};

pub mod streaming;

// TODO: how to set the xbc1 offsets when repacking the msrd?
#[binread]
#[derive(Debug, Xc3Write, Clone, PartialEq)]
#[br(magic(b"DRSM"))]
#[xc3(magic(b"DRSM"))]
pub struct Msrd {
    /// Version `10001`
    pub version: u32,

    // Don't have the streams own the data so mxmd can use the same types.
    #[br(parse_with = parse_data)]
    #[xc3(offset(u32), align(16))]
    pub data: Vec<u8>,

    /// Information on the files in [data](#structfield.data).
    /// Identical to [streaming](../mxmd/struct.Mxmd.html#structfield.streaming)
    /// for the corresponding [Mxmd](crate::mxmd::Mxmd).
    #[br(parse_with = parse_ptr32)]
    #[xc3(offset(u32))]
    pub streaming: Streaming,
}

#[binread]
#[derive(Debug, Xc3Write, Xc3WriteOffsets, Clone, PartialEq)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct Streaming {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    #[br(args_raw(base_offset))]
    pub inner: StreamingInner,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, Clone, PartialEq)]
#[br(import_raw(base_offset: u64))]
pub enum StreamingInner {
    #[br(magic(0u32))]
    #[xc3(magic(0u32))]
    StreamingLegacy(#[br(args_raw(base_offset))] StreamingDataLegacy),

    #[br(magic(4097u32))]
    #[xc3(magic(4097u32))]
    Streaming(#[br(args_raw(base_offset))] StreamingData),
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, Clone, PartialEq)]
#[br(import_raw(base_offset: u64))]
pub struct StreamingDataLegacy {
    pub flags: StreamingFlagsLegacy,

    #[br(parse_with = parse_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub low_textures: PackedExternalTextures,

    #[br(parse_with = parse_opt_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub textures: Option<PackedExternalTextures>,

    // TODO: Why are these necessary?
    #[br(parse_with = parse_ptr32)]
    #[br(args { offset: base_offset, inner: args! { count: low_textures.textures.len() }})]
    #[xc3(offset(u32))]
    pub low_texture_indices: Vec<u16>,

    #[br(parse_with = parse_opt_ptr32)]
    #[br(args {
        offset: base_offset,
        inner: args! { count: textures.as_ref().map(|t| t.textures.len()).unwrap_or_default() }
    })]
    #[xc3(offset(u32))]
    pub texture_indices: Option<Vec<u16>>,

    pub low_texture_data_offset: u32,
    pub texture_data_offset: u32,

    pub low_texture_data_uncompressed_size: u32,
    pub texture_data_uncompressed_size: u32,

    pub low_texture_data_compressed_size: u32,
    pub texture_data_compressed_size: u32,
}

/// Flags indicating the way data is stored in the model's `wismt` file.
#[derive(Debug, BinRead, BinWrite, Clone, Copy, PartialEq, Eq, Hash)]
#[brw(repr(u32))]
pub enum StreamingFlagsLegacy {
    Uncompressed = 1,
    Xbc1 = 2,
}

// TODO: Variable padding of 0 or 16 bytes?
// 76 (xc1, xc2, xc3) or 92 (xc3) bytes.
#[binread]
#[derive(Debug, Xc3Write, Xc3WriteOffsets, Clone, PartialEq)]
#[br(import_raw(base_offset: u64))]
pub struct StreamingData {
    pub stream_flags: StreamFlags,

    // Used for estimating the struct size.
    #[br(temp, restore_position)]
    offset: (u32, u32),

    /// Files contained within [streams](#structfield.streams).
    #[br(parse_with = parse_count32_offset32, offset = base_offset)]
    #[xc3(count_offset(u32, u32))]
    pub stream_entries: Vec<StreamEntry>,

    // TODO: Document the typical ordering of streams?
    /// A collection of [Xbc1] streams with decompressed layout
    /// specified in [stream_entries](#structfield.stream_entries).
    #[br(parse_with = parse_count32_offset32, offset = base_offset)]
    #[xc3(count_offset(u32, u32))]
    pub streams: Vec<Stream>,

    /// The [StreamEntry] for [Msrd::extract_vertex_data] with [EntryType::Vertex].
    pub vertex_data_entry_index: u32,
    /// The [StreamEntry] for [Msrd::extract_shader_data] with [EntryType::Shader].
    pub shader_entry_index: u32,

    /// The [StreamEntry] for [Msrd::extract_low_textures] with [EntryType::LowTextures].
    pub low_textures_entry_index: u32,
    /// The [Stream] for [Msrd::extract_low_textures].
    pub low_textures_stream_index: u32,

    /// The [Stream] for [Msrd::extract_textures].
    pub textures_stream_index: u32,
    /// The first [StreamEntry] for [Msrd::extract_textures].
    pub textures_stream_entry_start_index: u32,
    /// The number of [StreamEntry] corresponding
    /// to the number of textures in [Msrd::extract_textures].
    pub textures_stream_entry_count: u32,

    #[br(args { base_offset, size: offset.1 })]
    pub texture_resources: TextureResources,
}

// TODO: Better name?
// TODO: Always identical to mxmf?
#[derive(Debug, BinRead, Xc3Write, Clone, PartialEq)]
#[br(import { base_offset: u64, size: u32 })]
pub struct TextureResources {
    // TODO: also used for chr textures?
    /// Index into [low_textures](#structfield.low_textures)
    /// for each of the textures in [Msrd::extract_textures](crate::msrd::Msrd::extract_textures).
    /// This allows assigning higher resolution versions to only some of the textures.
    #[br(parse_with = parse_count32_offset32, offset = base_offset)]
    #[xc3(count_offset(u32, u32))]
    pub texture_indices: Vec<u16>,

    // TODO: Some of these use actual names?
    // TODO: Possible to figure out the hash function used?
    /// Name and data range for each of the [Mibl] textures.
    #[br(parse_with = parse_opt_ptr32, offset = base_offset)]
    #[xc3(offset(u32), align(2))]
    pub low_textures: Option<PackedExternalTextures>,

    /// Always `0`.
    pub unk1: u32,

    // TODO: only used for some xc3 models with chr/tex textures?
    #[br(if(size == 92), args_raw(base_offset))]
    pub chr_textures: Option<ChrTexTextures>,

    // TODO: padding?
    pub unk: [u32; 2],
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, Clone, PartialEq)]
#[br(import_raw(base_offset: u64))]
pub struct ChrTexTextures {
    #[br(parse_with = parse_count32_offset32, offset = base_offset)]
    #[xc3(count_offset(u32, u32))]
    pub chr_textures: Vec<ChrTexTexture>,

    // TODO: additional padding?
    pub unk: [u32; 2],
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, Clone, PartialEq)]
pub struct ChrTexTexture {
    // TODO: The texture name hash as an integer for xc3?
    pub hash: u32,
    pub unk2: u32,
    pub unk3: u32,
    pub unk4: u32,
    pub unk5: u32,
}

/// A file contained in a [Stream].
#[derive(Debug, BinRead, BinWrite, PartialEq, Eq, Clone)]
pub struct StreamEntry {
    /// The offset in bytes for the decompressed data range in the stream.
    pub offset: u32,
    /// The size in bytes of the decompressed data range in the stream.
    pub size: u32,
    /// Index into [streams](struct.StreamingData.html#structfield.streams)
    /// for the high resolution base mip level starting from 1.
    /// Has no effect if [entry_type](#structfield.entry_type) is not [EntryType::Texture]
    /// or the index is 0.
    pub texture_base_mip_stream_index: u16,
    pub entry_type: EntryType,
    // TODO: padding?
    pub unk: [u32; 2],
}

/// Flags indicating what stream data is present.
#[bitsize(32)]
#[derive(DebugBits, FromBits, BinRead, BinWrite, PartialEq, Eq, Clone, Copy)]
#[br(map = u32::into)]
#[bw(map = |&x| u32::from(x))]
pub struct StreamFlags {
    pub has_vertex: bool,
    pub has_spch: bool,
    pub has_low_textures: bool,
    pub has_textures: bool,
    pub unk5: bool,
    pub unk6: bool,
    pub has_chr_textures: bool,
    pub unk: u25,
}

/// The type of data for a [StreamEntry].
#[derive(Debug, BinRead, BinWrite, PartialEq, Eq, Clone, Copy)]
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
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, Clone, PartialEq)]
pub struct Stream {
    /// The size of the [Xbc1], including its header.
    pub compressed_size: u32,
    /// The size of the decompressed data in [xbc1](#structfield.xbc1).
    /// Aligned to 4096 (0x1000).
    pub decompressed_size: u32,
    /// The offset for the [Xbc1] relative to the start of the file.
    pub xbc1_offset: u32,
}

impl Stream {
    /// Read the [Xbc1] from `data`.
    /// This requires the [xbc1_offset](struct.Stream.html.structfield#xbc1_offset)
    /// from the first stream to correctly calculate the offset in the data section.
    pub fn read_xbc1(&self, data: &[u8], first_xbc1_offset: u32) -> binrw::BinResult<Xbc1> {
        let start = self.xbc1_offset - first_xbc1_offset;
        Xbc1::from_bytes(&data[start as usize..start as usize + self.compressed_size as usize])
    }
}

fn parse_data<R>(reader: &mut R, endian: binrw::Endian, _args: ()) -> BinResult<Vec<u8>>
where
    R: Read + Seek,
{
    // This is technically the streaming header's size.
    // Use it as an offset to the xbc1 to work with the write derive.
    let offset = u32::read_options(reader, endian, ())?;
    let saved_pos = reader.stream_position()?;

    if offset == 0 {
        return Err(binrw::Error::AssertFail {
            pos: saved_pos,
            message: "unexpected null offset".to_string(),
        });
    }

    reader.seek(SeekFrom::Start(offset as u64 + 16))?;
    let bytes = until_eof(reader, endian, ())?;
    reader.seek(SeekFrom::Start(saved_pos))?;

    Ok(bytes)
}

xc3_write_binwrite_impl!(StreamEntry, StreamFlags, StreamingFlagsLegacy);

impl<'a> Xc3WriteOffsets for MsrdOffsets<'a> {
    fn write_offsets<W: Write + Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
    ) -> xc3_write::Xc3Result<()> {
        // Different order than field order.
        self.streaming.write_full(writer, base_offset, data_ptr)?;
        self.data.write_full(writer, base_offset + 16, data_ptr)?;
        Ok(())
    }
}

impl<'a> Xc3WriteOffsets for TextureResourcesOffsets<'a> {
    fn write_offsets<W: Write + Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
    ) -> xc3_write::Xc3Result<()> {
        // Different order than field order.
        self.chr_textures
            .write_offsets(writer, base_offset, data_ptr)?;
        self.texture_indices
            .write_full(writer, base_offset, data_ptr)?;
        self.low_textures
            .write_full(writer, base_offset, data_ptr)?;

        Ok(())
    }
}
