use std::{borrow::Cow, path::Path};

use image_dds::ddsfile::Dds;
use log::error;

use crate::{
    error::DecompressStreamError, mibl::Mibl, mxmd::TextureUsage, spch::Spch, vertex::VertexData,
};

use super::*;

// TODO: Add a function to create an extractedtexture from a surface?
/// All the mip levels and metadata for an [Mibl] (Switch) or [Dds] (PC) texture.
#[derive(Debug)]
pub struct ExtractedTexture<T> {
    pub name: String,
    pub usage: TextureUsage,
    pub low: T,
    pub high: Option<HighTexture<T>>,
}

/// An additional texture that replaces the low resolution texture.
#[derive(Debug, Clone)]
pub struct HighTexture<T> {
    pub mid: T,
    pub base_mip: Option<Vec<u8>>,
}

impl ExtractedTexture<Dds> {
    /// Returns the highest possible quality [Dds] after trying low, high, or high + base mip level.
    pub fn dds_final(&self) -> &Dds {
        // TODO: Try and get the base mip level to work?
        // TODO: use a surface instead?
        self.high.as_ref().map(|h| &h.mid).unwrap_or(&self.low)
    }
}

impl ExtractedTexture<Mibl> {
    /// Returns the highest possible quality [Mibl] after trying low, high, or high + base mip level.
    /// Only high + base mip level returns [Cow::Owned].
    pub fn mibl_final(&self) -> Cow<'_, Mibl> {
        self.high
            .as_ref()
            .map(|h| {
                h.base_mip
                    .as_ref()
                    .map(|base| Cow::Owned(h.mid.with_base_mip(base)))
                    .unwrap_or(Cow::Borrowed(&h.mid))
            })
            .unwrap_or(Cow::Borrowed(&self.low))
    }
}

pub struct ChrTextureStreams {
    pub hash: u32,
    pub mid: Xbc1,
    pub base_mip: Xbc1,
}

impl ChrTextureStreams {
    /// Save the texture streams to `"chr/tex/nx/m"`` and `"chr/tex/nx/h"`.
    pub fn save<P: AsRef<Path>>(&self, chr_tex_nx: P) {
        let folder = chr_tex_nx.as_ref();
        self.mid
            .save(folder.join("m").join(format!("{:x}.wismt", self.hash)))
            .unwrap();
        self.base_mip
            .save(folder.join("h").join(format!("{:x}.wismt", self.hash)))
            .unwrap();
    }
}

impl Msrd {
    pub fn decompress_stream(&self, stream_index: u32) -> Result<Vec<u8>, DecompressStreamError> {
        match &self.streaming.inner {
            StreamingInner::StreamingLegacy(_) => todo!(),
            StreamingInner::Streaming(data) => data.decompress_stream(stream_index, &self.data),
        }
    }

    pub fn decompress_stream_entry(
        &self,
        stream_index: u32,
        entry_index: u32,
    ) -> Result<Vec<u8>, DecompressStreamError> {
        match &self.streaming.inner {
            StreamingInner::StreamingLegacy(_) => todo!(),
            StreamingInner::Streaming(data) => {
                data.decompress_stream_entry(stream_index, entry_index, &self.data)
            }
        }
    }

    /// Extract all embedded files for a `wismt` file.
    ///
    /// For Xenoblade 3 models, specify the path for "chr/tex/nx" for `chr_tex_nx`.
    pub fn extract_files(
        &self,
        chr_tex_nx: Option<&Path>,
    ) -> Result<(VertexData, Spch, Vec<ExtractedTexture<Mibl>>), DecompressStreamError> {
        // TODO: Return just textures for legacy data?
        match &self.streaming.inner {
            StreamingInner::StreamingLegacy(_) => todo!(),
            StreamingInner::Streaming(data) => data.extract_files(&self.data, chr_tex_nx),
        }
    }

    /// Extract all embedded files for a `pcsmt` file.
    pub fn extract_files_pc(
        &self,
    ) -> Result<(VertexData, Spch, Vec<ExtractedTexture<Dds>>), DecompressStreamError> {
        match &self.streaming.inner {
            StreamingInner::StreamingLegacy(_) => todo!(),
            StreamingInner::Streaming(data) => data.extract_files(&self.data, None),
        }
    }

    /// Pack and compress the files into new archive data.
    ///
    /// When `use_chr_textures` is `true`,
    /// the high resolution and base mip levels are packed into streams
    /// to be saved to the "chr/tex/nx" folder separately.
    /// This is only supported by Xenoblade 3 and should be `false` for other games.
    pub fn from_extracted_files(
        vertex: &VertexData,
        spch: &Spch,
        textures: &[ExtractedTexture<Mibl>],
        use_chr_textures: bool,
    ) -> (Self, Option<Vec<ChrTextureStreams>>) {
        let (mut streaming, data, chr_textures) =
            pack_files(vertex, spch, textures, use_chr_textures);

        // HACK: We won't know the first xbc1 offset until writing the header.
        let mut writer = Cursor::new(Vec::new());
        let mut data_ptr = 0;
        write_full(&streaming, &mut writer, 0, &mut data_ptr).unwrap();
        // Add the msrd and streaming header sizes.
        let first_xbc1_offset = round_up(data_ptr, 16) as u32 + 32;

        // TODO: Does this acount for the occasional extra 16 bytes?
        for stream in &mut streaming.streams {
            stream.xbc1_offset += first_xbc1_offset;
        }

        (
            Self {
                version: 10001,
                streaming: Streaming {
                    inner: StreamingInner::Streaming(streaming),
                },
                data,
            },
            chr_textures,
        )
    }
}

trait Texture: Sized {
    fn from_bytes<T: AsRef<[u8]>>(bytes: T) -> binrw::BinResult<Self>;
}

impl Texture for Mibl {
    fn from_bytes<T: AsRef<[u8]>>(bytes: T) -> binrw::BinResult<Self> {
        Mibl::from_bytes(bytes)
    }
}

impl Texture for Dds {
    fn from_bytes<T: AsRef<[u8]>>(bytes: T) -> binrw::BinResult<Self> {
        // TODO: Avoid unwrap by creating another error type?
        Ok(<Dds as DdsExt>::from_bytes(bytes).unwrap())
    }
}

impl StreamingData {
    pub fn decompress_stream(
        &self,
        stream_index: u32,
        data: &[u8],
    ) -> Result<Vec<u8>, DecompressStreamError> {
        let first_xbc1_offset = self.streams[0].xbc1_offset;
        let stream = &self.streams[stream_index as usize]
            .read_xbc1(data, first_xbc1_offset)?
            .decompress()?;
        Ok(stream.to_vec())
    }

    pub fn decompress_stream_entry(
        &self,
        stream_index: u32,
        entry_index: u32,
        data: &[u8],
    ) -> Result<Vec<u8>, DecompressStreamError> {
        let stream = self.decompress_stream(stream_index, data)?;
        let entry = &self.stream_entries[entry_index as usize];
        Ok(stream[entry.offset as usize..entry.offset as usize + entry.size as usize].to_vec())
    }

    fn entry_bytes<'a>(&self, entry_index: u32, bytes: &'a [u8]) -> &'a [u8] {
        let entry = &self.stream_entries[entry_index as usize];
        &bytes[entry.offset as usize..entry.offset as usize + entry.size as usize]
    }

    fn extract_files<T: Texture>(
        &self,
        data: &[u8],
        chr_tex_nx: Option<&Path>,
    ) -> Result<(VertexData, Spch, Vec<ExtractedTexture<T>>), DecompressStreamError> {
        let first_xbc1_offset = self.streams[0].xbc1_offset;

        // Extract all at once to avoid costly redundant decompression operations.
        // TODO: is this always in the first stream?
        let stream0 = self.streams[0]
            .read_xbc1(data, first_xbc1_offset)?
            .decompress()?;
        let vertex =
            VertexData::from_bytes(self.entry_bytes(self.vertex_data_entry_index, &stream0))?;
        let spch = Spch::from_bytes(self.entry_bytes(self.shader_entry_index, &stream0))?;

        // TODO: is this always in the first stream?
        let low_texture_bytes = self.entry_bytes(self.low_textures_entry_index, &stream0);
        let textures = self.extract_textures(data, low_texture_bytes, chr_tex_nx)?;

        Ok((vertex, spch, textures))
    }

    fn extract_low_textures<T: Texture>(
        &self,
        low_texture_data: &[u8],
    ) -> Result<Vec<ExtractedTexture<T>>, DecompressStreamError> {
        match &self.texture_resources.low_textures {
            Some(low_textures) => low_textures
                .textures
                .iter()
                .map(|t| {
                    let mibl_bytes = &low_texture_data
                        [t.mibl_offset as usize..t.mibl_offset as usize + t.mibl_length as usize];
                    Ok(ExtractedTexture {
                        name: t.name.clone(),
                        usage: t.usage,
                        low: T::from_bytes(mibl_bytes)?,
                        high: None,
                    })
                })
                .collect(),
            None => Ok(Vec::new()),
        }
    }

    // TODO: avoid unwrap?
    fn extract_textures<T: Texture, P: AsRef<Path>>(
        &self,
        data: &[u8],
        low_texture_data: &[u8],
        chr_tex_nx: Option<P>,
    ) -> Result<Vec<ExtractedTexture<T>>, DecompressStreamError> {
        // Start with no high res textures or base mip levels.
        let mut textures = self.extract_low_textures(low_texture_data)?;

        if self.textures_stream_entry_count > 0 {
            // The high resolution textures are packed into a single stream.
            let first_xbc1_offset = self.streams[0].xbc1_offset;
            let stream = &self.streams[self.textures_stream_index as usize]
                .read_xbc1(data, first_xbc1_offset)?
                .decompress()?;

            // TODO: Par iter?
            let start = self.textures_stream_entry_start_index as usize;
            let count = self.textures_stream_entry_count as usize;
            for (i, entry) in self
                .texture_resources
                .texture_indices
                .iter()
                .zip(&self.stream_entries[start..start + count])
            {
                let bytes =
                    &stream[entry.offset as usize..entry.offset as usize + entry.size as usize];
                let mid = T::from_bytes(bytes)?;

                // Indices start from 1 for the base mip level.
                // Base mip levels are stored in their own streams.
                let base_mip_stream_index = entry.texture_base_mip_stream_index.saturating_sub(1);
                let base_mip = if base_mip_stream_index != 0 {
                    Some(
                        self.streams[base_mip_stream_index as usize]
                            .read_xbc1(data, first_xbc1_offset)?
                            .decompress()?,
                    )
                } else {
                    None
                };

                textures[*i as usize].high = Some(HighTexture { mid, base_mip });
            }
        }

        if let Some(chr_textures) = &self.texture_resources.chr_textures {
            if let Some(chr_tex_nx) = chr_tex_nx {
                let chr_tex_nx = chr_tex_nx.as_ref();

                for (i, chr_tex) in self
                    .texture_resources
                    .texture_indices
                    .iter()
                    .zip(chr_textures.chr_textures.iter())
                {
                    // TODO: Is the name always the hash in lowercase hex?
                    let name = format!("{:08x}", chr_tex.hash);

                    let m_path = chr_tex_nx.join("m").join(&name).with_extension("wismt");
                    let xbc1 = Xbc1::from_file(m_path)?;
                    let bytes = xbc1.decompress()?;
                    let mid = T::from_bytes(bytes)?;

                    let h_path = chr_tex_nx.join("h").join(&name).with_extension("wismt");
                    let base_mip = Xbc1::from_file(h_path)?.decompress()?;

                    textures[*i as usize].high = Some(HighTexture {
                        mid,
                        base_mip: Some(base_mip),
                    });
                }
            }
        }

        Ok(textures)
    }
}

fn pack_files(
    vertex: &VertexData,
    spch: &Spch,
    textures: &[ExtractedTexture<Mibl>],
    use_chr_textures: bool,
) -> (StreamingData, Vec<u8>, Option<Vec<ChrTextureStreams>>) {
    let (stream_entries, streams, low_textures, data) =
        create_streams(vertex, spch, textures, use_chr_textures);

    let textures_stream_entry_start_index = stream_entries
        .iter()
        .position(|e| e.entry_type == EntryType::Texture)
        .unwrap_or_default() as u32;

    let textures_stream_entry_count = stream_entries
        .iter()
        .filter(|e| e.entry_type == EntryType::Texture)
        .count() as u32;

    let (chr_textures, chr_texture_streams) = use_chr_textures
        .then(|| pack_chr_textures(textures))
        .unzip();

    // TODO: Search stream entries to get indices?
    (
        StreamingData {
            stream_flags: StreamFlags::new(
                true,
                true,
                true,
                textures_stream_entry_count > 0,
                false, // TODO:Does this matter?
                true,  // TODO:Does this matter?
                use_chr_textures,
                0u8.into(),
            ),
            stream_entries,
            streams,
            vertex_data_entry_index: 0,
            shader_entry_index: 1,
            low_textures_entry_index: 2,
            low_textures_stream_index: 0, // TODO: always 0?
            textures_stream_index: if textures_stream_entry_count > 0 {
                1
            } else {
                0
            },
            textures_stream_entry_start_index,
            textures_stream_entry_count,
            texture_resources: TextureResources {
                texture_indices: textures
                    .iter()
                    .enumerate()
                    .filter_map(|(i, t)| t.high.as_ref().map(|_| i as u16))
                    .collect(),
                low_textures: (!low_textures.is_empty()).then_some(PackedExternalTextures {
                    textures: low_textures,
                    unk2: 0,
                    strings_offset: 0,
                }),
                unk1: 0,
                // TODO: How to properly create this field?
                chr_textures,
                unk: [0; 2],
            },
        },
        data,
        chr_texture_streams,
    )
}

fn pack_chr_textures(
    textures: &[ExtractedTexture<Mibl>],
) -> (ChrTexTextures, Vec<ChrTextureStreams>) {
    let (chr_textures, streams) = textures
        .iter()
        .filter_map(|t| {
            // TODO: Always assume the name is already a hash?
            // TODO: How to handle missing high resolution textures?
            // TODO: Stream names?
            let high = t.high.as_ref()?;
            let mid = Xbc1::new("0000".to_string(), &high.mid).unwrap();
            let base_mip = Xbc1::new("0000".to_string(), high.base_mip.as_ref()?).unwrap();
            let hash = u32::from_str_radix(&t.name, 16).expect(&t.name);

            Some((
                ChrTexTexture {
                    hash,
                    decompressed_size: mid.decompressed_size,
                    compressed_size: (round_up(mid.compressed_size as u64, 16) + 48) as u32,
                    base_mip_decompressed_size: base_mip.decompressed_size,
                    base_mip_compressed_size: (round_up(base_mip.compressed_size as u64, 16) + 48)
                        as u32,
                },
                ChrTextureStreams {
                    hash,
                    mid,
                    base_mip,
                },
            ))
        })
        .unzip();

    (
        ChrTexTextures {
            chr_textures,
            unk: [0; 2],
        },
        streams,
    )
}

fn create_streams(
    vertex: &VertexData,
    spch: &Spch,
    textures: &[ExtractedTexture<Mibl>],
    use_chr_textures: bool,
) -> (
    Vec<StreamEntry>,
    Vec<Stream>,
    Vec<PackedExternalTexture>,
    Vec<u8>,
) {
    // Entries are in ascending order by offset and stream.
    // Data order is Vertex, Shader, LowTextures, Textures.
    let mut xbc1s = Vec::new();
    let mut stream_entries = Vec::new();

    let low_textures = write_stream0(&mut xbc1s, &mut stream_entries, vertex, spch, textures);

    if !use_chr_textures {
        let entry_start_index = stream_entries.len();
        write_stream1(&mut xbc1s, &mut stream_entries, textures);

        write_base_mip_streams(&mut xbc1s, &mut stream_entries, textures, entry_start_index);
    }

    let mut streams = Vec::new();
    let mut data = Cursor::new(Vec::new());
    for xbc1 in xbc1s {
        // This needs to be updated later to be relative to the start of the msrd.
        let xbc1_offset = data.stream_position().unwrap() as u32;
        xbc1.write(&mut data).unwrap();

        // TODO: Should this make sure the xbc1 decompressed data is actually aligned?
        streams.push(Stream {
            compressed_size: (round_up(xbc1.compressed_stream.len() as u64, 16) + 48) as u32,
            decompressed_size: round_up(xbc1.decompressed_size as u64, 4096) as u32,
            xbc1_offset,
        });
    }

    (stream_entries, streams, low_textures, data.into_inner())
}

fn write_stream0(
    streams: &mut Vec<Xbc1>,
    stream_entries: &mut Vec<StreamEntry>,
    vertex: &VertexData,
    spch: &Spch,
    textures: &[ExtractedTexture<Mibl>],
) -> Vec<PackedExternalTexture> {
    // Data in streams is tightly packed.
    let mut writer = Cursor::new(Vec::new());
    stream_entries.push(write_stream_data(&mut writer, vertex, EntryType::Vertex));
    stream_entries.push(write_stream_data(&mut writer, spch, EntryType::Shader));

    let (entry, low_textures) = write_low_textures(&mut writer, textures);
    stream_entries.push(entry);

    let xbc1 = Xbc1::from_decompressed("0000".to_string(), &writer.into_inner()).unwrap();
    streams.push(xbc1);

    low_textures
}

fn write_stream1(
    streams: &mut Vec<Xbc1>,
    stream_entries: &mut Vec<StreamEntry>,
    textures: &[ExtractedTexture<Mibl>],
) {
    // Add higher resolution textures.
    let mut writer = Cursor::new(Vec::new());
    let mut is_empty = true;

    for texture in textures {
        if let Some(high) = &texture.high {
            let entry = write_stream_data(&mut writer, &high.mid, EntryType::Texture);
            stream_entries.push(entry);
            is_empty = false;
        }
    }

    if !is_empty {
        let xbc1 = Xbc1::from_decompressed("0000".to_string(), &writer.into_inner()).unwrap();
        streams.push(xbc1);
    }
}

fn write_base_mip_streams(
    streams: &mut Vec<Xbc1>,
    stream_entries: &mut [StreamEntry],
    textures: &[ExtractedTexture<Mibl>],
    entry_start_index: usize,
) {
    // Only count textures with a higher resolution version to match entry ordering.
    for (i, high) in textures.iter().filter_map(|t| t.high.as_ref()).enumerate() {
        if let Some(base) = &high.base_mip {
            stream_entries[entry_start_index + i].texture_base_mip_stream_index =
                streams.len() as u16 + 1;

            // TODO: Should this be aligned in any way?
            let xbc1 = Xbc1::from_decompressed("0000".to_string(), base).unwrap();
            streams.push(xbc1);
        }
    }
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
        texture_base_mip_stream_index: 0,
        entry_type: item_type,
        unk: [0; 2],
    }
}

fn write_low_textures(
    writer: &mut Cursor<Vec<u8>>,
    textures: &[ExtractedTexture<Mibl>],
) -> (StreamEntry, Vec<PackedExternalTexture>) {
    let mut low_textures = Vec::new();

    let offset = writer.stream_position().unwrap();
    for texture in textures {
        let mibl_offset = writer.stream_position().unwrap();
        texture.low.write(writer).unwrap();
        let mibl_length = writer.stream_position().unwrap() - mibl_offset;

        low_textures.push(PackedExternalTexture {
            usage: texture.usage,
            mibl_length: mibl_length as u32,
            mibl_offset: mibl_offset as u32 - offset as u32,
            name: texture.name.clone(),
        })
    }
    let end_offset = writer.stream_position().unwrap();

    // Assume the Mibl already have the required 4096 byte alignment.
    (
        StreamEntry {
            offset: offset as u32,
            size: (end_offset - offset) as u32,
            texture_base_mip_stream_index: 0,
            entry_type: EntryType::LowTextures,
            unk: [0; 2],
        },
        low_textures,
    )
}

impl StreamingDataLegacy {
    pub fn extract_textures(&self, data: &[u8]) -> Vec<ExtractedTexture<Mibl>> {
        // Start with lower resolution textures.
        let low_data = self.low_texture_data(data);

        let mut textures: Vec<_> = self
            .low_textures
            .textures
            .iter()
            .map(|t| {
                let mibl = Mibl::from_bytes(
                    &low_data
                        [t.mibl_offset as usize..t.mibl_offset as usize + t.mibl_length as usize],
                )
                .unwrap();
                ExtractedTexture {
                    name: t.name.clone(),
                    usage: t.usage,
                    low: mibl,
                    high: None,
                }
            })
            .collect();

        // Apply higher resolution texture data if present.
        if let (Some(texture_indices), Some(high_textures)) =
            (&self.texture_indices, &self.textures)
        {
            let high_data = self.high_texture_data(data);

            for (i, t) in texture_indices.iter().zip(high_textures.textures.iter()) {
                // TODO: Are these sometimes base mip levels?
                if let Some(bytes) = high_data
                    .get(t.mibl_offset as usize..t.mibl_offset as usize + t.mibl_length as usize)
                {
                    match Mibl::from_bytes(bytes) {
                        Ok(mibl) => {
                            textures[*i as usize].high = Some(HighTexture {
                                mid: mibl,
                                base_mip: None,
                            });
                        }
                        Err(e) => error!("Error loading legacy high resolution Mibl: {e}"),
                    }
                } else {
                    error!("Legacy high resolution Mibl bytes out of range")
                }
            }
        }

        textures
    }

    fn low_texture_data<'a>(&self, data: &'a [u8]) -> Cow<'a, [u8]> {
        match self.flags {
            StreamingFlagsLegacy::Uncompressed => {
                Cow::Borrowed(&data[self.low_texture_data_offset as usize..])
            }
            StreamingFlagsLegacy::Xbc1 => {
                let xbc1 = Xbc1::from_bytes(data).unwrap();
                Cow::Owned(xbc1.decompress().unwrap())
            }
        }
    }

    fn high_texture_data<'a>(&self, data: &'a [u8]) -> Cow<'a, [u8]> {
        match self.flags {
            StreamingFlagsLegacy::Uncompressed => {
                Cow::Borrowed(&data[self.texture_data_offset as usize..])
            }
            StreamingFlagsLegacy::Xbc1 => {
                let xbc1 = Xbc1::from_bytes(data).unwrap();
                Cow::Owned(xbc1.decompress().unwrap())
            }
        }
    }
}
