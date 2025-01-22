use std::{
    borrow::Cow,
    path::{Path, PathBuf},
};

use image_dds::{ddsfile::Dds, Surface};
use rayon::prelude::*;
use thiserror::Error;
use xc3_write::Xc3Result;

use crate::{
    align,
    error::DecompressStreamError,
    get_bytes,
    mibl::Mibl,
    mtxt::Mtxt,
    mxmd::TextureUsage,
    spch::Spch,
    vertex::VertexData,
    xbc1::{CompressionType, CreateXbc1Error},
    ReadFileError,
};

use super::*;

#[derive(Debug, Error)]
pub enum ExtractFilesError {
    #[error("error decompressing stream")]
    Stream(#[from] DecompressStreamError),

    #[error("error reading chr/tex texture")]
    ChrTexTexture(#[from] ReadFileError),

    #[error("legacy streams do not contain all necessary data")]
    LegacyStream,
}

// TODO: Add a function to create an extractedtexture from a surface?
/// All the mip levels and metadata for an [Mibl] (Switch) or [Dds] (PC) texture.
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug)]
pub struct ExtractedTexture<T, U> {
    pub name: String,
    pub usage: U,
    pub low: T,
    pub high: Option<HighTexture<T>>,
}

/// An additional texture that replaces the low resolution texture.
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Clone)]
pub struct HighTexture<T> {
    pub mid: T,
    pub base_mip: Option<Vec<u8>>,
}

impl ExtractedTexture<Dds, TextureUsage> {
    /// Returns the highest possible quality [Dds] after trying low, high, or high + base mip level.
    pub fn dds_final(&self) -> &Dds {
        // TODO: Try and get the base mip level to work?
        // TODO: use a surface instead?
        self.high.as_ref().map(|h| &h.mid).unwrap_or(&self.low)
    }
}

impl ExtractedTexture<Mibl, TextureUsage> {
    /// Returns the highest possible quality deswizzled data after trying low, high, or high + base mip level.
    /// Only high + base mip level returns [Cow::Owned].
    pub fn surface_final(
        &self,
    ) -> Result<image_dds::Surface<Vec<u8>>, tegra_swizzle::SwizzleError> {
        self.high
            .as_ref()
            .map(|h| {
                h.base_mip
                    .as_ref()
                    .map(|base| h.mid.to_surface_with_base_mip(base))
                    .unwrap_or_else(|| h.mid.to_surface())
            })
            .unwrap_or_else(|| self.low.to_surface())
    }

    /// Split a full resolution `mibl` into a low texture, medium texture, and base mipmap.
    pub fn from_mibl(
        mibl: &Mibl,
        name: String,
        usage: TextureUsage,
    ) -> ExtractedTexture<Mibl, TextureUsage> {
        let low = low_texture(mibl);
        let (mid, base_mip) = mibl.split_base_mip();

        ExtractedTexture {
            name,
            usage,
            low,
            high: Some(HighTexture {
                mid,
                base_mip: Some(base_mip),
            }),
        }
    }
}

fn low_texture(mibl: &Mibl) -> Mibl {
    // The low texture is only visible briefly before data is streamed in.
    // Find a balance between blurry distance rendering and increased file sizes.
    // 32x32 is the highest resolution typically found for in game low textures.
    let surface = mibl.to_surface().unwrap();
    create_desired_mip(surface.as_ref(), 32)
        .or_else(|| create_desired_mip(surface.as_ref(), 4))
        .unwrap_or_else(|| {
            // Resizing and decoding and encoding the full texture is expensive.
            // We can cheat and just use the first GOB (512 bytes) of compressed image data.
            let mut low_image_data = mibl
                .image_data
                .get(..512)
                .unwrap_or(&mibl.image_data)
                .to_vec();
            low_image_data.resize(512, 0);

            Mibl {
                image_data: low_image_data,
                footer: crate::mibl::MiblFooter {
                    image_size: 4096,
                    unk: 0x1000,
                    width: 4,
                    height: 4,
                    depth: 1,
                    view_dimension: crate::mibl::ViewDimension::D2,
                    image_format: mibl.footer.image_format,
                    mipmap_count: 1,
                    version: 10001,
                },
            }
        })
}

fn create_desired_mip(surface: Surface<&[u8]>, desired_dimension: u32) -> Option<Mibl> {
    for mip in (0..surface.mipmaps).rev() {
        if let Some(data) = surface.get(0, 0, mip) {
            let width = image_dds::mip_dimension(surface.width, mip);
            let height = image_dds::mip_dimension(surface.height, mip);
            if width >= desired_dimension || height >= desired_dimension {
                // TODO: use remaining mimpmaps if available.
                // TODO: add surface.get_mipmaps(i..) to image_dds?
                return Mibl::from_surface(Surface {
                    width,
                    height,
                    depth: 1,
                    layers: 1,
                    mipmaps: 1,
                    image_format: surface.image_format,
                    data,
                })
                .ok();
            }
        }
    }
    None
}

impl ExtractedTexture<Mtxt, crate::mxmd::legacy::TextureUsage> {
    /// Returns the highest possible quality [Mtxt] after trying low and high.
    pub fn mtxt_final(&self) -> &Mtxt {
        self.high.as_ref().map(|h| &h.mid).unwrap_or(&self.low)
    }
}

/// `chr/tex/nx` stream files for a single texture.
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Clone)]
pub struct ChrTextureStreams {
    /// The texture hash used for the file name.
    pub hash: u32,
    /// The high resolution texture for `chr/tex/nx/m`.
    pub mid: Xbc1,
    /// The base mip level for `chr/tex/nx/h`.
    pub base_mip: Xbc1,
}

impl ChrTextureStreams {
    /// Save the texture streams to `chr/tex/nx/m` and `chr/tex/nx/h`.
    pub fn save<P: AsRef<Path>>(&self, chr_tex_nx: P) -> Xc3Result<()> {
        let folder = chr_tex_nx.as_ref();
        self.mid
            .save(folder.join("m").join(format!("{:x}.wismt", self.hash)))?;
        self.base_mip
            .save(folder.join("h").join(format!("{:x}.wismt", self.hash)))?;
        Ok(())
    }
}

/// Compress the high resolution and base mip levels for `textures`
/// to Xenoblade 3 `chr/tex/nx` folder data.
pub fn pack_chr_textures(
    textures: &[ExtractedTexture<Mibl, TextureUsage>],
) -> Result<(ChrTexTextures, Vec<ChrTextureStreams>), CreateXbc1Error> {
    let streams = textures
        .iter()
        .filter_map(|t| {
            let high = t.high.as_ref()?;
            Some((&high.mid, high.base_mip.as_ref()?, &t.name))
        })
        .map(|(mid, base_mip, name)| {
            // TODO: Always assume the name is already a hash?
            // TODO: How to handle missing high resolution textures?
            // TODO: Stream names?
            let mid = Xbc1::new("0000".to_string(), mid, CompressionType::Zlib)?;
            let base_mip = Xbc1::new("0000".to_string(), base_mip, CompressionType::Zlib)?;
            let hash = u32::from_str_radix(name, 16).expect(name);

            Ok(ChrTextureStreams {
                hash,
                mid,
                base_mip,
            })
        })
        .collect::<Result<Vec<_>, CreateXbc1Error>>()?;

    let chr_textures = streams
        .iter()
        .map(|stream| ChrTexTexture {
            hash: stream.hash,
            decompressed_size: stream.mid.decompressed_size,
            compressed_size: stream
                .mid
                .compressed_size
                .next_multiple_of(Xbc1::ALIGNMENT as u32)
                + 48,
            base_mip_decompressed_size: stream.base_mip.decompressed_size,
            base_mip_compressed_size: stream
                .base_mip
                .compressed_size
                .next_multiple_of(Xbc1::ALIGNMENT as u32)
                + 48,
        })
        .collect();

    Ok((
        ChrTexTextures {
            chr_textures,
            unk: [0; 2],
        },
        streams,
    ))
}

impl Msrd {
    /// Extract all embedded files for a `wismt` file.
    ///
    /// For Xenoblade 3 models, specify the path for the `chr/tex/nx` folder
    /// to properly extract higher resolution textures.
    /// If the path is part of the Xenoblade 3 dump, see [chr_tex_nx_folder].
    pub fn extract_files(
        &self,
        chr_tex_nx: Option<&Path>,
    ) -> Result<(VertexData, Spch, Vec<ExtractedTexture<Mibl, TextureUsage>>), ExtractFilesError>
    {
        // TODO: Return just textures for legacy data?
        match &self.streaming.inner {
            StreamingInner::StreamingLegacy(_) => Err(ExtractFilesError::LegacyStream),
            StreamingInner::Streaming(data) => data.extract_files(&self.data, chr_tex_nx),
        }
    }

    /// Extract all embedded files for a `pcsmt` file.
    pub fn extract_files_pc(
        &self,
    ) -> Result<(VertexData, Spch, Vec<ExtractedTexture<Dds, TextureUsage>>), ExtractFilesError>
    {
        match &self.streaming.inner {
            StreamingInner::StreamingLegacy(_) => Err(ExtractFilesError::LegacyStream),
            StreamingInner::Streaming(data) => data.extract_files(&self.data, None),
        }
    }

    // TODO: Create a dedicated error type for this?
    /// Pack and compress the files into new archive data.
    ///
    /// The final [Msrd] will embed high resolution textures
    /// and not reference any textures in the `chr/tex/nx` folder
    /// to avoid modifying textures files shared with other models.
    ///
    /// Set `use_chr_textures` to `true` for Xenoblade 3 models
    /// to ensure the correct empty data entries are generated
    /// for the file to load properly in game.
    /// This should always be `false` for other Xenoblade games.
    ///
    /// # Examples
    /// ```rust no_run
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use xc3_lib::msrd::Msrd;
    ///
    /// let msrd = Msrd::from_file("ch01011013.wismt")?;
    /// let chr_tex_nx = Some(std::path::Path::new("chr/tex/nx"));
    /// let (mut vertex, mut spch, mut textures) = msrd.extract_files(chr_tex_nx)?;
    ///
    /// // modify any of the embedded data
    ///
    /// let new_msrd = Msrd::from_extracted_files(&vertex, &spch, &textures, true)?;
    /// new_msrd.save("ch01011013.wismt")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn from_extracted_files(
        vertex: &VertexData,
        spch: &Spch,
        textures: &[ExtractedTexture<Mibl, TextureUsage>],
        use_chr_textures: bool,
    ) -> Result<Self, CreateXbc1Error> {
        // TODO: This should actually be checking if the game is xenoblade 3.
        let (mut streaming, data) = pack_files(vertex, spch, textures, use_chr_textures)?;

        // HACK: We won't know the first xbc1 offset until writing the header.
        let mut writer = Cursor::new(Vec::new());
        let mut data_ptr = 0;
        write_full(
            &streaming,
            &mut writer,
            0,
            &mut data_ptr,
            xc3_write::Endian::Little,
            (),
        )?;
        // Add the streaming tag and msrd header size.
        let first_xbc1_offset = (data_ptr + 4).next_multiple_of(Xbc1::ALIGNMENT) as u32 + 16;

        for stream in &mut streaming.streams {
            stream.xbc1_offset += first_xbc1_offset;
        }

        Ok(Self {
            version: 10001,
            streaming: Streaming {
                inner: StreamingInner::Streaming(streaming),
            },
            data,
        })
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
    fn get_stream(&self, index: usize) -> Result<&Stream, DecompressStreamError> {
        self.streams
            .get(index)
            .ok_or(DecompressStreamError::MissingStream {
                index,
                count: self.streams.len(),
            })
    }

    pub fn decompress_stream(
        &self,
        stream_index: u32,
        data: &[u8],
    ) -> Result<Vec<u8>, DecompressStreamError> {
        let first_xbc1_offset = self.get_stream(0)?.xbc1_offset;
        self.get_stream(stream_index as usize)?
            .read_xbc1(data, first_xbc1_offset)?
            .decompress()
    }

    pub fn decompress_stream_entry(
        &self,
        stream_index: u32,
        entry_index: u32,
        data: &[u8],
    ) -> Result<Vec<u8>, DecompressStreamError> {
        let stream = self.decompress_stream(stream_index, data)?;
        Ok(self.entry_bytes(entry_index, &stream)?.to_vec())
    }

    fn entry_bytes<'a>(&self, entry_index: u32, bytes: &'a [u8]) -> std::io::Result<&'a [u8]> {
        let entry = &self.stream_entries[entry_index as usize];
        get_bytes(bytes, entry.offset, Some(entry.size))
    }

    fn extract_files<T: Texture>(
        &self,
        data: &[u8],
        chr_tex_nx: Option<&Path>,
    ) -> Result<(VertexData, Spch, Vec<ExtractedTexture<T, TextureUsage>>), ExtractFilesError> {
        let stream0 = self.get_stream(0)?;
        let first_xbc1_offset = stream0.xbc1_offset;

        // Extract all at once to avoid costly redundant decompression operations.
        // TODO: is this always in the first stream?
        let stream0 = stream0
            .read_xbc1(data, first_xbc1_offset)
            .map_err(DecompressStreamError::from)?
            .decompress()?;

        let vertex_bytes = self
            .entry_bytes(self.vertex_data_entry_index, &stream0)
            .map_err(DecompressStreamError::Io)?;
        let vertex = VertexData::from_bytes(vertex_bytes).map_err(DecompressStreamError::from)?;

        let spch_bytes = self
            .entry_bytes(self.shader_entry_index, &stream0)
            .map_err(DecompressStreamError::Io)?;
        let spch = Spch::from_bytes(spch_bytes).map_err(DecompressStreamError::from)?;

        // TODO: is this always in the first stream?
        let low_texture_bytes = self
            .entry_bytes(self.low_textures_entry_index, &stream0)
            .map_err(DecompressStreamError::Io)?;
        let textures = self.extract_textures(data, low_texture_bytes, chr_tex_nx)?;

        Ok((vertex, spch, textures))
    }

    fn extract_low_textures<T: Texture>(
        &self,
        low_texture_data: &[u8],
    ) -> Result<Vec<ExtractedTexture<T, TextureUsage>>, DecompressStreamError> {
        match &self.texture_resources.low_textures {
            Some(low_textures) => low_textures
                .textures
                .iter()
                .map(|t| {
                    let mibl_bytes = get_bytes(low_texture_data, t.offset, Some(t.length))?;
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

    fn extract_textures<T: Texture, P: AsRef<Path>>(
        &self,
        data: &[u8],
        low_texture_data: &[u8],
        chr_tex_nx: Option<P>,
    ) -> Result<Vec<ExtractedTexture<T, TextureUsage>>, ExtractFilesError> {
        // Start with no high res textures or base mip levels.
        let mut textures = self.extract_low_textures(low_texture_data)?;

        if self.textures_stream_entry_count > 0 {
            // The high resolution textures are packed into a single stream.
            let first_xbc1_offset = self.get_stream(0)?.xbc1_offset;
            let stream = self
                .get_stream(self.textures_stream_index as usize)?
                .read_xbc1(data, first_xbc1_offset)
                .map_err(DecompressStreamError::from)?
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
                let bytes = get_bytes(&stream, entry.offset, Some(entry.size))
                    .map_err(DecompressStreamError::Io)?;
                let mid = T::from_bytes(bytes).map_err(DecompressStreamError::from)?;

                // Indices start from 1 for the base mip level.
                // Base mip levels are stored in their own streams.
                let base_mip_stream_index = entry.texture_base_mip_stream_index.saturating_sub(1);
                let base_mip = if base_mip_stream_index != 0 {
                    Some(
                        self.get_stream(base_mip_stream_index as usize)?
                            .read_xbc1(data, first_xbc1_offset)
                            .map_err(DecompressStreamError::from)?
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
                    let mid = read_chr_tex_m_texture(&m_path)?;

                    let h_path = chr_tex_nx.join("h").join(&name).with_extension("wismt");
                    let base_mip = read_chr_tex_h_texture(&h_path)?;

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

fn read_chr_tex_h_texture(h_path: &Path) -> Result<Vec<u8>, ExtractFilesError> {
    let base_mip = Xbc1::from_file(h_path)?.decompress()?;
    Ok(base_mip)
}

fn read_chr_tex_m_texture<T: Texture>(m_path: &Path) -> Result<T, ExtractFilesError> {
    let xbc1 = Xbc1::from_file(m_path)?;
    let bytes = xbc1.decompress()?;
    let mid = T::from_bytes(bytes).map_err(|e| {
        ExtractFilesError::ChrTexTexture(ReadFileError {
            path: m_path.to_owned(),
            source: e,
        })
    })?;
    Ok(mid)
}

fn pack_files(
    vertex: &VertexData,
    spch: &Spch,
    textures: &[ExtractedTexture<Mibl, TextureUsage>],
    use_chr_textures: bool,
) -> Result<(StreamingData, Vec<u8>), CreateXbc1Error> {
    let Streams {
        stream_entries,
        streams,
        low_textures,
        data,
    } = create_streams(vertex, spch, textures)?;

    let vertex_data_entry_index = stream_entry_index(&stream_entries, EntryType::Vertex);
    let shader_entry_index = stream_entry_index(&stream_entries, EntryType::Shader);
    let low_textures_entry_index = stream_entry_index(&stream_entries, EntryType::LowTextures);
    let textures_stream_entry_start_index = stream_entry_index(&stream_entries, EntryType::Texture);

    let textures_stream_entry_count = stream_entries
        .iter()
        .filter(|e| e.entry_type == EntryType::Texture)
        .count() as u32;

    // Replacing chr/tex/nx textures is problematic since texture wismts are shared.
    // We can avoid conflicts by embedding the high resolution textures in the model wismt.
    // Xenoblade 3 still requires dummy data even if the chr/tex/nx textures aren't used.
    let chr_textures = use_chr_textures.then_some(ChrTexTextures {
        chr_textures: Vec::new(),
        unk: [0; 2],
    });

    Ok((
        StreamingData {
            flags: StreamFlags::new(
                true,
                true,
                true,
                textures_stream_entry_count > 0,
                false, // TODO:Does this matter?
                false, // TODO:Does this matter?
                false,
                0u8.into(),
            ),
            stream_entries,
            streams,
            vertex_data_entry_index,
            shader_entry_index,
            low_textures_entry_index,
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
                chr_textures,
                unk: [0; 2],
            },
        },
        data,
    ))
}

fn stream_entry_index(stream_entries: &[StreamEntry], entry_type: EntryType) -> u32 {
    stream_entries
        .iter()
        .position(|e| e.entry_type == entry_type)
        .unwrap_or_default() as u32
}

struct Streams {
    stream_entries: Vec<StreamEntry>,
    streams: Vec<Stream>,
    low_textures: Vec<PackedExternalTexture<TextureUsage>>,
    data: Vec<u8>,
}

fn create_streams(
    vertex: &VertexData,
    spch: &Spch,
    textures: &[ExtractedTexture<Mibl, TextureUsage>],
) -> Result<Streams, CreateXbc1Error> {
    // Entries are in ascending order by offset and stream.
    // Data order is Vertex, Shader, LowTextures, Textures.
    let mut stream_entries = Vec::new();

    let (low_textures, stream0_data) = write_stream0(&mut stream_entries, vertex, spch, textures)?;

    // Always write high resolution textures to wismt for compatibility.
    // This works across all switch games and doesn't interfere with chr/tex/nx textures.
    let entry_start_index = stream_entries.len();
    let stream1_data = write_stream1(&mut stream_entries, textures);

    // Ignore unused empty streams.
    let mut streams_data = vec![&stream0_data];
    if !stream1_data.is_empty() {
        streams_data.push(&stream1_data);
    };

    let base_mips = write_base_mip_streams(
        &mut stream_entries,
        textures,
        streams_data.len() as u16,
        entry_start_index,
    );

    streams_data.extend_from_slice(&base_mips);

    // Only parallelize the expensive compression operations to avoid locking.
    let xbc1s: Vec<_> = streams_data
        .par_iter()
        .map(|data| {
            Xbc1::from_decompressed("0000".to_string(), data, CompressionType::Zlib).unwrap()
        })
        .collect();

    let mut streams = Vec::new();
    let mut data = Cursor::new(Vec::new());
    for xbc1 in xbc1s {
        // This needs to be updated later to be relative to the start of the msrd.
        let xbc1_start = data.stream_position()? as u32;
        xbc1.write(&mut data)?;

        let pos = data.position();
        align(&mut data, pos, Xbc1::ALIGNMENT, 0)?;

        let xbc1_end = data.stream_position()? as u32;

        // TODO: Should this make sure the xbc1 decompressed data is actually aligned?
        streams.push(Stream {
            compressed_size: xbc1_end - xbc1_start,
            decompressed_size: xbc1.decompressed_size.next_multiple_of(4096),
            xbc1_offset: xbc1_start,
        });
    }

    Ok(Streams {
        stream_entries,
        streams,
        low_textures,
        data: data.into_inner(),
    })
}

fn write_stream0(
    stream_entries: &mut Vec<StreamEntry>,
    vertex: &VertexData,
    spch: &Spch,
    textures: &[ExtractedTexture<Mibl, TextureUsage>],
) -> Result<(Vec<PackedExternalTexture<TextureUsage>>, Vec<u8>), CreateXbc1Error> {
    // Data in streams is tightly packed.
    let mut writer = Cursor::new(Vec::new());
    stream_entries.push(write_stream_data(&mut writer, vertex, EntryType::Vertex)?);
    stream_entries.push(write_stream_data(&mut writer, spch, EntryType::Shader)?);

    let (entry, low_textures) = write_low_textures(&mut writer, textures)?;
    stream_entries.push(entry);

    Ok((low_textures, writer.into_inner()))
}

fn write_stream1(
    stream_entries: &mut Vec<StreamEntry>,
    textures: &[ExtractedTexture<Mibl, TextureUsage>],
) -> Vec<u8> {
    // Add higher resolution textures.
    let mut writer = Cursor::new(Vec::new());

    for texture in textures {
        if let Some(high) = &texture.high {
            let entry = write_stream_data(&mut writer, &high.mid, EntryType::Texture).unwrap();
            stream_entries.push(entry);
        }
    }

    writer.into_inner()
}

fn write_base_mip_streams<'a>(
    stream_entries: &mut [StreamEntry],
    textures: &'a [ExtractedTexture<Mibl, TextureUsage>],
    streams_count: u16,
    entry_start_index: usize,
) -> Vec<&'a Vec<u8>> {
    // Count previous streams with indexing starting from 1.
    let mut stream_index = streams_count + 1;

    let mut base_mips = Vec::new();
    for (i, high) in textures.iter().filter_map(|t| t.high.as_ref()).enumerate() {
        // Only count textures with a higher resolution version to match entry ordering.
        if let Some(base) = &high.base_mip {
            stream_entries[entry_start_index + i].texture_base_mip_stream_index = stream_index;
            base_mips.push(base);
            stream_index += 1;
        }
    }

    // TODO: Should these be aligned in any way?
    base_mips
}

fn write_stream_data<'a, T>(
    writer: &mut Cursor<Vec<u8>>,
    data: &'a T,
    item_type: EntryType,
) -> Xc3Result<StreamEntry>
where
    T: Xc3Write + 'static,
    T::Offsets<'a>: Xc3WriteOffsets<Args = ()>,
{
    let offset = writer.stream_position()?;
    write_full(data, writer, 0, &mut 0, xc3_write::Endian::Little, ())?;

    // Stream data is aligned to 4096 bytes.
    align(writer, writer.position(), 4096, 0)?;

    let end_offset = writer.stream_position()?;

    Ok(StreamEntry {
        offset: offset as u32,
        size: (end_offset - offset) as u32,
        texture_base_mip_stream_index: 0,
        entry_type: item_type,
        unk: [0; 2],
    })
}

fn write_low_textures(
    writer: &mut Cursor<Vec<u8>>,
    textures: &[ExtractedTexture<Mibl, TextureUsage>],
) -> Xc3Result<(StreamEntry, Vec<PackedExternalTexture<TextureUsage>>)> {
    let mut low_textures = Vec::new();

    let offset = writer.stream_position()?;
    for texture in textures {
        let mibl_offset = writer.stream_position()?;
        texture.low.write(writer)?;
        let mibl_length = writer.stream_position()? - mibl_offset;

        low_textures.push(PackedExternalTexture {
            usage: texture.usage,
            length: mibl_length as u32,
            offset: mibl_offset as u32 - offset as u32,
            name: texture.name.clone(),
        })
    }
    let end_offset = writer.stream_position()?;

    // Assume the Mibl already have the required 4096 byte alignment.
    Ok((
        StreamEntry {
            offset: offset as u32,
            size: (end_offset - offset) as u32,
            texture_base_mip_stream_index: 0,
            entry_type: EntryType::LowTextures,
            unk: [0; 2],
        },
        low_textures,
    ))
}

// TODO: Find a way to share code with casmt.
impl StreamingDataLegacy {
    pub fn extract_textures(
        &self,
        data: &[u8],
    ) -> Result<(Vec<u16>, Vec<ExtractedTexture<Mibl, TextureUsage>>), DecompressStreamError> {
        let low_data = self.low_texture_data(data)?;
        let high_data = self.high_texture_data(data)?;
        self.inner
            .extract_textures(&low_data, &high_data, |bytes| Mibl::from_bytes(bytes))
    }

    fn low_texture_data<'a>(&self, data: &'a [u8]) -> Result<Cow<'a, [u8]>, DecompressStreamError> {
        match self.flags {
            StreamingFlagsLegacy::Uncompressed => {
                let bytes = get_bytes(data, self.low_texture_data_offset, None)?;
                Ok(Cow::Borrowed(bytes))
            }
            StreamingFlagsLegacy::Xbc1 => {
                let xbc1 = Xbc1::from_bytes(data)?;
                Ok(Cow::Owned(xbc1.decompress()?))
            }
        }
    }

    fn high_texture_data<'a>(
        &self,
        data: &'a [u8],
    ) -> Result<Cow<'a, [u8]>, DecompressStreamError> {
        match self.flags {
            StreamingFlagsLegacy::Uncompressed => {
                let bytes = get_bytes(data, self.texture_data_offset, None)?;
                Ok(Cow::Borrowed(bytes))
            }
            StreamingFlagsLegacy::Xbc1 => {
                // Read the second xbc1 file.
                let bytes = get_bytes(data, self.low_texture_data_compressed_size, None)?;
                let xbc1 = Xbc1::from_bytes(bytes)?;
                Ok(Cow::Owned(xbc1.decompress()?))
            }
        }
    }
}

impl<U> StreamingDataLegacyInner<U>
where
    U: Xc3Write + Copy + 'static,
    for<'a> U: BinRead<Args<'a> = ()>,
    for<'a> U::Offsets<'a>: Xc3WriteOffsets<Args = ()>,
{
    pub fn extract_textures<T, F>(
        &self,
        low_data: &[u8],
        high_data: &[u8],
        read_t: F,
    ) -> Result<(Vec<u16>, Vec<ExtractedTexture<T, U>>), DecompressStreamError>
    where
        F: Fn(&[u8]) -> BinResult<T>,
    {
        // Start with lower resolution textures.
        let mut textures = self
            .low_textures
            .textures
            .iter()
            .map(|t| {
                let bytes = get_bytes(low_data, t.offset, Some(t.length))?;
                let low = read_t(bytes)?;
                Ok(ExtractedTexture {
                    name: t.name.clone(),
                    usage: t.usage,
                    low,
                    high: None,
                })
            })
            .collect::<Result<Vec<_>, DecompressStreamError>>()?;

        // Apply higher resolution texture data if present.
        if let (Some(texture_indices), Some(high_textures)) =
            (&self.texture_indices, &self.textures)
        {
            for (i, t) in texture_indices.iter().zip(high_textures.textures.iter()) {
                let bytes = get_bytes(high_data, t.offset, Some(t.length))?;
                let mid = read_t(bytes)?;
                textures[*i as usize].high = Some(HighTexture {
                    mid,
                    base_mip: None,
                });
            }
        }

        // Material texture indices can be remapped.
        Ok((self.low_texture_indices.clone(), textures))
    }
}

/// Get the path for "chr/tex/nx" from a file or `None` if not present.
pub fn chr_tex_nx_folder<P: AsRef<Path>>(input: P) -> Option<PathBuf> {
    // "chr/en/file.wismt" -> "chr/tex/nx"
    let parent = input.as_ref().parent()?.parent()?;

    if parent.file_name().and_then(|f| f.to_str()) == Some("chr") {
        Some(parent.join("tex").join("nx"))
    } else {
        // Not an xc3 chr model or not in the right folder.
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chr_tex_nx_folders() {
        assert_eq!(None, chr_tex_nx_folder(""));
        assert_eq!(Some("chr/tex/nx".into()), chr_tex_nx_folder("chr/tex/nx"));
        assert_eq!(
            Some("xeno3/extracted/chr/tex/nx".into()),
            chr_tex_nx_folder("xeno3/extracted/chr/ch/ch01011013.wimdo")
        );
        assert_eq!(
            None,
            chr_tex_nx_folder("xeno2/extracted/model/bl/bl000101.wimdo")
        );
    }
}
