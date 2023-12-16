use std::borrow::Cow;

use image_dds::ddsfile::Dds;

use crate::{
    error::DecompressStreamError, mibl::Mibl, mxmd::TextureUsage, spch::Spch, vertex::VertexData,
};

use super::*;

// TODO: Add a function to create an extractedtexture from a surface?
#[derive(Debug)]
pub struct ExtractedTexture<T> {
    pub name: String,
    pub usage: TextureUsage,
    pub low: T,
    pub high: Option<HighTexture<T>>,
}

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

impl Msrd {
    pub fn decompress_stream(
        &self,
        stream_index: u32,
        entry_index: u32,
    ) -> Result<Vec<u8>, DecompressStreamError> {
        match &self.streaming.inner {
            StreamingInner::StreamingLegacy(_) => todo!(),
            StreamingInner::Streaming(data) => data.decompress_stream(stream_index, entry_index),
        }
    }

    // TODO: also add these methods to StreamingData<Stream>?
    /// Extract geometry for `wismt` and `pcsmt` files.
    pub fn extract_vertex_data(&self) -> Result<VertexData, DecompressStreamError> {
        match &self.streaming.inner {
            StreamingInner::StreamingLegacy(_) => todo!(),
            StreamingInner::Streaming(data) => data.extract_vertex_data(),
        }
    }

    /// Extract all textures for `wismt`` files.
    pub fn extract_textures(&self) -> Result<Vec<ExtractedTexture<Mibl>>, DecompressStreamError> {
        match &self.streaming.inner {
            StreamingInner::StreamingLegacy(_) => todo!(),
            StreamingInner::Streaming(data) => {
                let low_texture_data = data.decompress_stream(
                    data.low_textures_stream_index,
                    data.low_textures_entry_index,
                )?;
                data.extract_textures(&low_texture_data)
            }
        }
    }

    // TODO: share code with above?
    /// Extract high resolution textures for `pcsmt` files.
    pub fn extract_pc_textures(&self) -> Result<Vec<ExtractedTexture<Dds>>, DecompressStreamError> {
        match &self.streaming.inner {
            StreamingInner::StreamingLegacy(_) => todo!(),
            StreamingInner::Streaming(data) => data.extract_pc_textures(),
        }
    }

    /// Extract shader programs for `wismt` and `pcsmt` files.
    pub fn extract_shader_data(&self) -> Result<Spch, DecompressStreamError> {
        match &self.streaming.inner {
            StreamingInner::StreamingLegacy(_) => todo!(),
            StreamingInner::Streaming(data) => data.extract_shader_data(),
        }
    }

    /// Extract all embedded files for a `wismt` file.
    pub fn extract_files(
        &self,
    ) -> Result<(VertexData, Spch, Vec<ExtractedTexture<Mibl>>), DecompressStreamError> {
        match &self.streaming.inner {
            StreamingInner::StreamingLegacy(_) => todo!(),
            StreamingInner::Streaming(data) => data.extract_files(),
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

    fn entry_bytes<'a>(&self, entry_index: u32, bytes: &'a [u8]) -> &'a [u8] {
        let entry = &self.stream_entries[entry_index as usize];
        &bytes[entry.offset as usize..entry.offset as usize + entry.size as usize]
    }

    /// Extract all embedded files for a `wismt` file.
    pub fn extract_files(
        &self,
    ) -> Result<(VertexData, Spch, Vec<ExtractedTexture<Mibl>>), DecompressStreamError> {
        // Extract all at once to avoid costly redundant decompression operations.
        // TODO: is this always in the first stream?
        let stream0 = self.streams[0].xbc1.decompress()?;
        let vertex =
            VertexData::from_bytes(self.entry_bytes(self.vertex_data_entry_index, &stream0))?;
        let spch = Spch::from_bytes(self.entry_bytes(self.shader_entry_index, &stream0))?;

        // TODO: is this always in the first stream?
        let low_texture_bytes = self.entry_bytes(self.low_textures_entry_index, &stream0);
        let textures = self.extract_textures(low_texture_bytes)?;

        Ok((vertex, spch, textures))
    }

    /// Extract geometry for `wismt` and `pcsmt` files.
    pub fn extract_vertex_data(&self) -> Result<VertexData, DecompressStreamError> {
        let bytes = self.decompress_stream(0, self.vertex_data_entry_index)?;
        VertexData::from_bytes(bytes).map_err(Into::into)
    }

    fn extract_low_textures(
        &self,
        low_texture_data: &[u8],
    ) -> Result<Vec<ExtractedTexture<Mibl>>, DecompressStreamError> {
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
                        low: Mibl::from_bytes(mibl_bytes)?,
                        high: None,
                    })
                })
                .collect(),
            None => Ok(Vec::new()),
        }
    }

    fn extract_low_pc_textures(&self) -> Vec<ExtractedTexture<Dds>> {
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

                    ExtractedTexture {
                        name: t.name.clone(),
                        usage: t.usage,
                        low: Dds::read(dds_bytes).unwrap(),
                        high: None,
                    }
                })
                .collect(),
            None => Vec::new(),
        }
    }

    // TODO: avoid unwrap?
    /// Extract all textures for `wismt` files.
    fn extract_textures(
        &self,
        low_texture_data: &[u8],
    ) -> Result<Vec<ExtractedTexture<Mibl>>, DecompressStreamError> {
        self.extract_textures_inner(
            |s| s.extract_low_textures(low_texture_data).unwrap(),
            |b| Mibl::from_bytes(b).unwrap(),
        )
    }

    /// Extract high resolution textures for `pcsmt` files.
    pub fn extract_pc_textures(&self) -> Result<Vec<ExtractedTexture<Dds>>, DecompressStreamError> {
        self.extract_textures_inner(Self::extract_low_pc_textures, |b| {
            Dds::from_bytes(b).unwrap()
        })
    }

    fn extract_textures_inner<T, F1, F2>(
        &self,
        read_low: F1,
        read_t: F2,
    ) -> Result<Vec<ExtractedTexture<T>>, DecompressStreamError>
    where
        F1: Fn(&Self) -> Vec<ExtractedTexture<T>>,
        F2: Fn(&[u8]) -> T,
    {
        // Start with no high res textures or base mip levels.
        let mut textures = read_low(self);

        // The high resolution textures are packed into a single stream.
        let stream = &self.streams[self.textures_stream_index as usize]
            .xbc1
            .decompress()?;

        // TODO: Par iter?
        let start = self.textures_stream_entry_start_index as usize;
        let count = self.textures_stream_entry_count as usize;
        for (i, entry) in self
            .texture_resources
            .texture_indices
            .iter()
            .zip(self.stream_entries[start..start + count].iter())
        {
            let bytes = &stream[entry.offset as usize..entry.offset as usize + entry.size as usize];
            let mid = read_t(bytes);

            // Indices start from 1 for the base mip level.
            let base_mip_stream_index = entry.texture_base_mip_stream_index.saturating_sub(1);
            let base_mip = if base_mip_stream_index != 0 {
                Some(
                    self.streams[base_mip_stream_index as usize]
                        .xbc1
                        .decompress()?,
                )
            } else {
                None
            };

            textures[*i as usize].high = Some(HighTexture { mid, base_mip });
        }

        Ok(textures)
    }

    /// Extract shader programs for `wismt` and `pcsmt` files.
    pub fn extract_shader_data(&self) -> Result<Spch, DecompressStreamError> {
        // TODO: is this always in the first stream?
        let bytes = self.decompress_stream(0, self.shader_entry_index)?;
        Spch::from_bytes(bytes).map_err(Into::into)
    }

    // TODO: This needs to create the entire Msrd since each stream offset depends on the header size?
    /// Pack and compress the files into new archive data.
    pub fn from_extracted_files(
        vertex: &VertexData,
        spch: &Spch,
        textures: &[ExtractedTexture<Mibl>],
    ) -> Self {
        // TODO: handle other streams.
        let (stream_entries, streams, low_textures) = create_streams(vertex, spch, textures);

        // TODO: Search stream entries to get indices?
        // TODO: How are entry indices set if there are no textures?
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
            streams,
            vertex_data_entry_index: 0,
            shader_entry_index: 1,
            low_textures_entry_index: 2,
            low_textures_stream_index: 0, // TODO: always 0?
            textures_stream_index: 0,     // TODO: always 1 if textures are present?
            textures_stream_entry_start_index: 0,
            textures_stream_entry_count: 0,
            // TODO: How to properly create these fields?
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
                chr_textures: None,
                unk: [0; 2],
            },
        }
    }
}

fn create_streams(
    vertex: &VertexData,
    spch: &Spch,
    textures: &[ExtractedTexture<Mibl>],
) -> (Vec<StreamEntry>, Vec<Stream>, Vec<PackedExternalTexture>) {
    // Entries are in ascending order by offset and stream.
    // Data order is Vertex, Shader, LowTextures, Textures.
    let mut streams = Vec::new();
    let mut stream_entries = Vec::new();

    let low_textures = write_stream0(&mut streams, &mut stream_entries, vertex, spch, textures);

    let entry_start_index = stream_entries.len();
    write_stream1(&mut streams, &mut stream_entries, textures);

    write_base_mip_streams(
        &mut streams,
        &mut stream_entries,
        textures,
        entry_start_index,
    );

    (stream_entries, streams, low_textures)
}

fn write_stream0(
    streams: &mut Vec<Stream>,
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
    let stream = Stream::from_xbc1(xbc1);

    streams.push(stream);

    low_textures
}

fn write_stream1(
    streams: &mut Vec<Stream>,
    stream_entries: &mut Vec<StreamEntry>,
    textures: &[ExtractedTexture<Mibl>],
) {
    // Add higher resolution textures.
    let mut writer = Cursor::new(Vec::new());

    for texture in textures {
        if let Some(high) = &texture.high {
            let entry = write_stream_data(&mut writer, &high.mid, EntryType::Texture);
            stream_entries.push(entry);
        }
    }

    let xbc1 = Xbc1::from_decompressed("0000".to_string(), &writer.into_inner()).unwrap();
    let stream = Stream::from_xbc1(xbc1);
    streams.push(stream);
}

fn write_base_mip_streams(
    streams: &mut Vec<Stream>,
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
            streams.push(Stream::from_xbc1(xbc1));
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
        let low_data = self.decompress_stream(
            data,
            self.low_texture_data_offset,
            self.low_texture_data_compressed_size,
        );

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
            let high_data = self.decompress_stream(
                data,
                self.texture_data_offset,
                self.texture_data_compressed_size,
            );

            for (i, t) in texture_indices.iter().zip(high_textures.textures.iter()) {
                let mibl = Mibl::from_bytes(
                    &high_data
                        [t.mibl_offset as usize..t.mibl_offset as usize + t.mibl_length as usize],
                )
                .unwrap();

                textures[*i as usize].high = Some(HighTexture {
                    mid: mibl,
                    base_mip: None,
                });
            }
        }

        textures
    }

    fn decompress_stream<'a>(&self, data: &'a [u8], offset: u32, size: u32) -> Cow<'a, [u8]> {
        let data = &data[offset as usize..offset as usize + size as usize];
        match self.flags {
            StreamingFlagsLegacy::Uncompressed => Cow::Borrowed(data),
            StreamingFlagsLegacy::Xbc1 => {
                let xbc1 = Xbc1::from_bytes(data).unwrap();
                Cow::Owned(xbc1.decompress().unwrap())
            }
        }
    }
}
