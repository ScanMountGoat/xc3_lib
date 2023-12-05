use std::{error::Error, path::Path};

use image_dds::{ddsfile::Dds, Surface};
use log::error;
use thiserror::Error;
use xc3_lib::{
    mibl::{Mibl, SwizzleError},
    mxmd::{Mxmd, PackedTexture},
    xbc1::Xbc1,
};

pub use xc3_lib::mibl::{ImageFormat, ViewDimension};

use crate::StreamingData;

#[derive(Debug, Error)]
pub enum CreateImageTextureError {
    #[error("error deswizzling surface: {0}")]
    Swizzle(#[from] SwizzleError),

    #[error("error reading data: {0}")]
    Binrw(#[from] binrw::Error),

    #[error("error decompressing stream: {0}")]
    Stream(#[from] xc3_lib::error::DecompressStreamError),
}

/// A non swizzled version of an [Mibl] texture.
#[derive(Debug, Clone, PartialEq)]
pub struct ImageTexture {
    /// An optional name assigned to some textures.
    /// This will typically be [None]
    /// and can not be used for reliably identifying textures.
    pub name: Option<String>,
    /// The width of the base mip level in pixels.
    pub width: u32,
    /// The height of the base mip level in pixels.
    pub height: u32,
    /// The depth of the base mip level in pixels.
    pub depth: u32,
    pub view_dimension: ViewDimension, // TODO: is this redundant?
    pub image_format: ImageFormat,
    /// The number of mip levels or 1 if there are no mipmaps.
    pub mipmap_count: u32,
    /// The combined image surface data in a standard row-major layout.
    /// Ordered as `Layer 0 Mip 0, Layer 0 Mip 1, ... Layer L-1 Mip M-1`
    /// for L layers and M mipmaps similar to DDS files.
    pub image_data: Vec<u8>,
}

impl ImageTexture {
    /// Deswizzle the data from `mibl`.
    /// The `name` is not required but creates more descriptive file names and debug information.
    pub fn from_mibl(mibl: &Mibl, name: Option<String>) -> Result<Self, SwizzleError> {
        Ok(Self {
            name,
            width: mibl.footer.width,
            height: mibl.footer.height,
            depth: mibl.footer.depth,
            view_dimension: mibl.footer.view_dimension,
            image_format: mibl.footer.image_format,
            mipmap_count: mibl.footer.mipmap_count,
            image_data: mibl.deswizzled_image_data()?,
        })
    }

    /// Deswizzle and combine the data from `base_mip_level` for mip 0 and `mibl_m` for the remaining mip levels.
    pub fn from_mibl_base_mip(
        base_mip_level: Vec<u8>,
        mibl_m: Mibl,
        name: Option<String>,
    ) -> Result<Self, SwizzleError> {
        // TODO: double depth?
        let width = mibl_m.footer.width * 2;
        let height = mibl_m.footer.height * 2;
        let depth = mibl_m.footer.depth;

        let image_data = mibl_m.deswizzle_image_data_base_mip(base_mip_level)?;
        Ok(ImageTexture {
            name,
            width,
            height,
            depth,
            view_dimension: mibl_m.footer.view_dimension,
            image_format: mibl_m.footer.image_format,
            mipmap_count: mibl_m.footer.mipmap_count + 1,
            image_data,
        })
    }

    pub fn from_packed_texture(texture: &PackedTexture) -> Result<Self, CreateImageTextureError> {
        let mibl = Mibl::from_bytes(&texture.mibl_data)?;
        Self::from_mibl(&mibl, Some(texture.name.clone())).map_err(Into::into)
    }

    pub fn to_image(&self) -> Result<image_dds::image::RgbaImage, Box<dyn Error>> {
        let dds = self.to_dds()?;
        image_dds::image_from_dds(&dds, 0).map_err(Into::into)
    }

    pub fn to_surface(&self) -> image_dds::Surface<&[u8]> {
        Surface {
            width: self.width,
            height: self.height,
            depth: self.depth,
            layers: if self.view_dimension == ViewDimension::Cube {
                6
            } else {
                1
            },
            mipmaps: self.mipmap_count,
            image_format: self.image_format.into(),
            data: &self.image_data,
        }
    }

    // TODO: use a dedicated error type
    pub fn to_dds(&self) -> Result<Dds, Box<dyn Error>> {
        self.to_surface().to_dds().map_err(Into::into)
    }

    pub fn from_surface<T: AsRef<[u8]>>(
        surface: Surface<T>,
        name: Option<String>,
    ) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            name,
            width: surface.width,
            height: surface.height,
            depth: surface.depth,
            view_dimension: if surface.layers == 6 {
                ViewDimension::Cube
            } else if surface.depth > 1 {
                ViewDimension::D3
            } else {
                ViewDimension::D2
            },
            image_format: surface.image_format.try_into()?,
            mipmap_count: surface.mipmaps,
            image_data: surface.data.as_ref().to_vec(),
        })
    }

    pub fn from_dds(dds: &Dds, name: Option<String>) -> Result<Self, Box<dyn Error>> {
        Self::from_surface(Surface::from_dds(dds)?, name)
    }

    // TODO: to_mibl?
}

// TODO: clean this up.
pub fn load_textures(
    mxmd: &Mxmd,
    streaming_data: Option<&StreamingData>,
    m_tex_folder: &Path,
    h_tex_folder: &Path,
    is_pc: bool,
) -> Vec<ImageTexture> {
    // TODO: what is the correct priority for the different texture sources?
    if let Some(data) = streaming_data {
        match data {
            StreamingData::Msrd { streaming, msrd } => {
                let mxmd_textures = streaming
                    .inner
                    .texture_resources
                    .low_textures
                    .as_ref()
                    .map(|t| &t.textures);

                // TODO: Not all formats used by PC DDS files are supported.
                let low_textures = if is_pc {
                    msrd.extract_low_pc_textures()
                } else {
                    msrd.extract_low_textures()
                        .unwrap()
                        .iter()
                        .map(|m| m.to_dds().unwrap())
                        .collect()
                };
                let textures = if is_pc {
                    msrd.extract_pc_textures()
                } else {
                    msrd.extract_textures()
                        .unwrap()
                        .iter()
                        .map(|m| m.to_dds().unwrap())
                        .collect()
                };

                let texture_indices = &streaming.inner.texture_resources.texture_indices;

                // Assume the packed and non packed textures have the same ordering.
                // TODO: Are the mxmd and msrd packed texture lists always identical?
                // TODO: Only assign chr textures if the appropriate fields are present?
                mxmd_textures
                    .map(|external_textures| {
                        external_textures
                            .iter()
                            .enumerate()
                            .map(|(i, texture)| {
                                load_chr_tex_texture(m_tex_folder, h_tex_folder, &texture.name)
                                    .ok()
                                    .or_else(|| {
                                        // TODO: Assign in a second pass to avoid O(N) find.
                                        texture_indices
                                            .iter()
                                            .position(|id| *id as usize == i)
                                            .and_then(|index| {
                                                textures.get(index).map(|dds| {
                                                    ImageTexture::from_dds(
                                                        dds,
                                                        Some(texture.name.clone()),
                                                    )
                                                    .unwrap()
                                                })
                                            })
                                    })
                                    .unwrap_or_else(|| {
                                        // Some textures only have a low resolution version.
                                        ImageTexture::from_dds(
                                            &low_textures[i],
                                            Some(texture.name.clone()),
                                        )
                                        .unwrap()
                                    })
                            })
                            .collect()
                    })
                    .unwrap_or_default()
            }
            StreamingData::Legacy { legacy, data } => {
                // TODO: high resolution textures?
                legacy
                    .low_textures
                    .textures
                    .iter()
                    .map(|t| {
                        let offset = legacy.low_texture_data_offset + t.mibl_offset;
                        let mibl = Mibl::from_bytes(
                            &data[offset as usize..offset as usize + t.mibl_length as usize],
                        )
                        .unwrap();
                        ImageTexture::from_mibl(&mibl, Some(t.name.clone())).unwrap()
                    })
                    .collect()
            }
        }
    } else if let Some(packed_textures) = &mxmd.packed_textures {
        packed_textures
            .textures
            .iter()
            .map(|t| ImageTexture::from_packed_texture(t).unwrap())
            .collect()
    } else {
        // TODO: How to handle this case?
        error!("Failed to load textures");
        Vec::new()
    }
}

fn load_chr_tex_texture(
    m_texture_folder: &Path,
    h_texture_folder: &Path,
    texture_name: &str,
) -> Result<ImageTexture, CreateImageTextureError> {
    // Xenoblade 3 has some textures in the chr/tex folder.
    let xbc1 = Xbc1::from_file(m_texture_folder.join(texture_name).with_extension("wismt"))?;
    let mibl_m: Mibl = xbc1.extract()?;

    let base_mip_level =
        Xbc1::from_file(h_texture_folder.join(texture_name).with_extension("wismt"))?
            .decompress()?;

    ImageTexture::from_mibl_base_mip(base_mip_level, mibl_m, Some(texture_name.to_string()))
        .map_err(Into::into)
}
