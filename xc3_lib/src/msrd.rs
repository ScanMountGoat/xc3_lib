//! Model resources like shaders, geometry, or textures in `.wismt` files.
//!
//! XC3: `chr/{ch,en,oj,wp}/*.wismt`
use std::io::{Cursor, Seek, Write};

use crate::{
    error::DecompressStreamError, mibl::Mibl, mxmd::PackedExternalTextures, parse_count32_offset32,
    parse_opt_ptr32, parse_ptr32, spch::Spch, vertex::VertexData, xbc1::Xbc1,
    xc3_write_binwrite_impl,
};
use bilge::prelude::*;
use binrw::{args, binread, BinRead, BinWrite};
use image_dds::ddsfile::Dds;
use xc3_write::{round_up, write_full, Xc3Write, Xc3WriteOffsets};

#[binread]
#[derive(Debug, Xc3Write, Xc3WriteOffsets)]
#[br(magic(b"DRSM"))]
#[xc3(magic(b"DRSM"))]
pub struct Msrd {
    /// Version `10001`
    pub version: u32,
    // rounded or aligned in some way?
    pub header_size: u32, // TODO: xbc1 offset - 16?

    #[br(parse_with = parse_ptr32)]
    #[xc3(offset(u32))]
    pub data: Streaming<Stream>,
}

#[binread]
#[derive(Debug, Xc3Write, Xc3WriteOffsets)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct Streaming<S>
where
    S: Xc3Write + 'static,
    for<'a> <S as Xc3Write>::Offsets<'a>: Xc3WriteOffsets,
    for<'a> S: BinRead<Args<'a> = ()>,
{
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    pub tag: u32, // 4097 or sometimes 0?

    #[br(args { base_offset, tag })]
    pub inner: StreamingDataInner<S>,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
#[br(import { base_offset: u64, tag: u32 })]
pub enum StreamingDataInner<S>
where
    S: Xc3Write + 'static,
    for<'b> <S as Xc3Write>::Offsets<'b>: Xc3WriteOffsets,
    for<'a> S: BinRead<Args<'a> = ()>,
{
    #[br(pre_assert(tag == 0))]
    StreamingLegacy(#[br(args_raw(base_offset))] StreamingDataLegacy),

    #[br(pre_assert(tag == 4097))]
    Streaming(#[br(args_raw(base_offset))] StreamingData<S>),
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
#[br(import_raw(base_offset: u64))]
pub struct StreamingDataLegacy {
    pub flags: StreamingFlagsLegacy,

    #[br(parse_with = parse_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub low_textures: PackedExternalTextures,

    #[br(parse_with = parse_opt_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub textures: Option<PackedExternalTextures>,

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

    pub low_texture_data_compressed_size: u32,
    pub texture_data_compressed_size: u32,

    pub low_texture_data_uncompressed_size: u32,
    pub texture_data_uncompressed_size: u32,
}

/// Flags indicating the way data is stored in the model's `wismt` file.
#[derive(Debug, BinRead, BinWrite, Clone, Copy, PartialEq, Eq, Hash)]
#[brw(repr(u32))]
pub enum StreamingFlagsLegacy {
    Uncompressed = 1,
    Xbc1 = 2,
}

// TODO: 76 or 92 bytes?
#[derive(Debug, BinRead, Xc3Write)]
#[br(import_raw(base_offset: u64))]
pub struct StreamingData<S>
where
    S: Xc3Write + 'static,
    for<'a> S: BinRead<Args<'a> = ()>,
{
    pub stream_flags: StreamFlags,

    /// Files contained within [streams](#structfield.streams).
    #[br(parse_with = parse_count32_offset32, offset = base_offset)]
    #[xc3(count_offset(u32, u32))]
    pub stream_entries: Vec<StreamEntry>,

    /// A collection of [Xbc1] streams with decompressed layout
    /// specified in [stream_entries](#structfield.stream_entries).
    #[br(parse_with = parse_count32_offset32, offset = base_offset)]
    #[xc3(count_offset(u32, u32))]
    pub streams: Vec<S>,

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

    #[br(args_raw(base_offset))]
    pub texture_resources: TextureResources,

    // TODO: not always present?
    // #[br(assert(unks.iter().all(|u| *u == 0)))]
    pub unks: [u32; 4],
}

// TODO: Better name?
// TODO: Always identical to mxmf?
#[derive(Debug, BinRead, Xc3Write)]
#[br(import_raw(base_offset: u64))]
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
    #[xc3(offset(u32))]
    pub low_textures: Option<PackedExternalTextures>,

    /// Always `0`.
    pub unk1: u32,

    // TODO: only used for xc3 models with chr/tex textures?
    #[br(parse_with = parse_count32_offset32, offset = base_offset)]
    #[xc3(count_offset(u32, u32))]
    pub chr_textures: Vec<ChrTexTexture>,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct ChrTexTexture {
    // TODO: The texture name hash as an integer for xc3?
    pub hash: u32,
    // TODO: related to packed texture unk1?
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
    // TODO: index into textures or low textures?
    pub texture_index: u16, // TODO: what does this do?
    pub item_type: EntryType,
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
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct Stream {
    /// The size of [xbc1](#structfield.xbc1), including the header.
    pub compressed_size: u32,
    /// The size of the decompressed data in [xbc1](#structfield.xbc1).
    /// Aligned to 4096 (0x1000).
    pub decompressed_size: u32,
    #[br(parse_with = parse_ptr32)]
    #[xc3(offset(u32))]
    pub xbc1: Xbc1,
}

impl Stream {
    pub fn from_xbc1(xbc1: Xbc1) -> Self {
        // TODO: Should this make sure the xbc1 decompressed data is actually aligned?
        Self {
            compressed_size: (round_up(xbc1.compressed_stream.len() as u64, 16) + 48) as u32,
            decompressed_size: round_up(xbc1.decompressed_size as u64, 4096) as u32,
            xbc1,
        }
    }
}

impl Msrd {
    pub fn decompress_stream(
        &self,
        stream_index: u32,
        entry_index: u32,
    ) -> Result<Vec<u8>, DecompressStreamError> {
        match &self.data.inner {
            StreamingDataInner::StreamingLegacy(_) => todo!(),
            StreamingDataInner::Streaming(data) => {
                data.decompress_stream(stream_index, entry_index)
            }
        }
    }

    // TODO: also add these methods to StreamingData<Stream>?
    /// Extract geometry for `wismt` and `pcsmt` files.
    pub fn extract_vertex_data(&self) -> Result<VertexData, DecompressStreamError> {
        match &self.data.inner {
            StreamingDataInner::StreamingLegacy(_) => todo!(),
            StreamingDataInner::Streaming(data) => data.extract_vertex_data(),
        }
    }

    /// Extract low resolution textures for `wismt` files.
    pub fn extract_low_textures(&self) -> Result<Vec<Mibl>, DecompressStreamError> {
        match &self.data.inner {
            StreamingDataInner::StreamingLegacy(_) => todo!(),
            StreamingDataInner::Streaming(data) => data.extract_low_textures(),
        }
    }

    /// Extract low resolution textures for `pcsmt` files.
    pub fn extract_low_pc_textures(&self) -> Vec<Dds> {
        match &self.data.inner {
            StreamingDataInner::StreamingLegacy(_) => todo!(),
            StreamingDataInner::Streaming(data) => data.extract_low_pc_textures(),
        }
    }

    /// Extract high resolution textures for `wismt` files.
    pub fn extract_textures(&self) -> Result<Vec<Mibl>, DecompressStreamError> {
        match &self.data.inner {
            StreamingDataInner::StreamingLegacy(_) => todo!(),
            StreamingDataInner::Streaming(data) => data.extract_textures(),
        }
    }

    // TODO: share code with above?
    /// Extract high resolution textures for `pcsmt` files.
    pub fn extract_pc_textures(&self) -> Vec<Dds> {
        match &self.data.inner {
            StreamingDataInner::StreamingLegacy(_) => todo!(),
            StreamingDataInner::Streaming(data) => data.extract_pc_textures(),
        }
    }

    /// Extract shader programs for `wismt` and `pcsmt` files.
    pub fn extract_shader_data(&self) -> Result<Spch, DecompressStreamError> {
        match &self.data.inner {
            StreamingDataInner::StreamingLegacy(_) => todo!(),
            StreamingDataInner::Streaming(data) => data.extract_shader_data(),
        }
    }
}

impl StreamingData<Stream> {
    pub fn decompress_stream(
        &self,
        stream_index: u32,
        entry_index: u32,
    ) -> Result<Vec<u8>, DecompressStreamError> {
        let stream = &self.streams[stream_index as usize].xbc1.decompress()?;
        let entry = &self.stream_entries[entry_index as usize];
        Ok(stream[entry.offset as usize..entry.offset as usize + entry.size as usize].to_vec())
    }

    /// Extract geometry for `wismt` and `pcsmt` files.
    pub fn extract_vertex_data(&self) -> Result<VertexData, DecompressStreamError> {
        // TODO: is this always in the first stream?
        let bytes = self.decompress_stream(0, self.vertex_data_entry_index)?;
        VertexData::from_bytes(bytes).map_err(Into::into)
    }

    /// Extract low resolution textures for `wismt` files.
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

    /// Extract low resolution textures for `pcsmt` files.
    pub fn extract_low_pc_textures(&self) -> Vec<Dds> {
        // TODO: Avoid unwrap.
        let bytes = self
            .decompress_stream(
                self.low_textures_stream_index,
                self.low_textures_entry_index,
            )
            .unwrap();

        match &self.texture_resources.low_textures {
            Some(low_textures) => low_textures
                .textures
                .iter()
                .map(|t| {
                    let dds_bytes = &bytes
                        [t.mibl_offset as usize..t.mibl_offset as usize + t.mibl_length as usize];
                    Dds::read(dds_bytes).unwrap()
                })
                .collect(),
            None => Vec::new(),
        }
    }

    /// Extract high resolution textures for `wismt` files.
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

    // TODO: share code with above?
    /// Extract high resolution textures for `pcsmt` files.
    pub fn extract_pc_textures(&self) -> Vec<Dds> {
        // The textures are packed into a single stream.
        let stream = &self.streams[self.textures_stream_index as usize]
            .xbc1
            .decompress()
            .unwrap();

        // TODO: avoid unwrap.
        let start = self.textures_stream_entry_start_index as usize;
        let count = self.textures_stream_entry_count as usize;
        self.stream_entries[start..start + count]
            .iter()
            .map(|entry| {
                let bytes =
                    &stream[entry.offset as usize..entry.offset as usize + entry.size as usize];
                Dds::read(bytes).unwrap()
            })
            .collect()
    }

    /// Extract shader programs for `wismt` and `pcsmt` files.
    pub fn extract_shader_data(&self) -> Result<Spch, DecompressStreamError> {
        // TODO: is this always in the first stream?
        let bytes = self.decompress_stream(0, self.shader_entry_index)?;
        Spch::from_bytes(bytes).map_err(Into::into)
    }

    /// Pack and compress the files into new archive data.
    pub fn from_unpacked_files(vertex: &VertexData, spch: &Spch, low_textures: &Vec<Mibl>) -> Self {
        // TODO: handle other streams.
        let (stream_entries, stream0) = create_stream0(vertex, spch, low_textures);

        let xbc1 = Xbc1::from_decompressed("0000".to_string(), &stream0).unwrap();
        let stream = Stream::from_xbc1(xbc1);

        // TODO: Search stream entries to get indices?
        StreamingData {
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
    let offset = writer.stream_position().unwrap();
    write_full(data, writer, 0, &mut 0).unwrap();
    let end_offset = writer.stream_position().unwrap();

    // Stream data is aligned to 4096 bytes.
    // TODO: Create a function for padding to an alignment?
    let size = end_offset - offset;
    let desired_size = round_up(size, 4096);
    let padding = desired_size - size;
    writer.write_all(&vec![0u8; padding as usize]).unwrap();
    let end_offset = writer.stream_position().unwrap();

    StreamEntry {
        offset: offset as u32,
        size: (end_offset - offset) as u32,
        texture_index: 0,
        item_type,
        unk: [0; 2],
    }
}

xc3_write_binwrite_impl!(StreamEntry, StreamFlags, StreamingFlagsLegacy);

impl<'a, S> Xc3WriteOffsets for StreamingDataOffsets<'a, S>
where
    S: Xc3Write + 'static,
    for<'b> <S as Xc3Write>::Offsets<'b>: Xc3WriteOffsets,
    for<'b> S: BinRead<Args<'b> = ()>,
{
    fn write_offsets<W: std::io::prelude::Write + Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
    ) -> xc3_write::Xc3Result<()> {
        // Write offset data in the order items appear in the binary file.
        self.stream_entries
            .write_offset(writer, base_offset, data_ptr)?;

        let stream_offsets = self.streams.write_offset(writer, base_offset, data_ptr)?;

        self.texture_resources
            .write_offsets(writer, base_offset, data_ptr)?;

        // TODO: Variable padding of 0 or 16 bytes?

        // Write the xbc1 data at the end.
        // This also works for mxmd streams that don't need to write anything.
        for offsets in stream_offsets.0 {
            // The xbc1 offset is relative to the start of the file.
            offsets.write_offsets(writer, 0, data_ptr)?;
        }

        Ok(())
    }
}

impl<'a> Xc3WriteOffsets for TextureResourcesOffsets<'a> {
    fn write_offsets<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
    ) -> xc3_write::Xc3Result<()> {
        // Different order than field order.
        self.chr_textures
            .write_full(writer, base_offset, data_ptr)?;
        self.texture_indices
            .write_full(writer, base_offset, data_ptr)?;
        self.low_textures
            .write_full(writer, base_offset, data_ptr)?;

        Ok(())
    }
}
