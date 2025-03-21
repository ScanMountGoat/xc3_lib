use image_dds::{ddsfile::Dds, error::CreateImageError, CreateDdsError, Surface};
use log::error;
use thiserror::Error;
use xc3_lib::{
    mibl::{CreateMiblError, Mibl},
    msrd::streaming::ExtractedTexture,
    mtxt::Mtxt,
    mxmd::{legacy::MxmdLegacy, PackedTexture},
};

pub use xc3_lib::mibl::{ImageFormat, ViewDimension};
pub use xc3_lib::mxmd::TextureUsage;

use crate::get_bytes;

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug)]
pub enum ExtractedTextures {
    Switch(Vec<ExtractedTexture<Mibl, TextureUsage>>),
    Pc(
        #[cfg_attr(feature = "arbitrary", arbitrary(with = arbitrary_dds_textures))]
        Vec<ExtractedTexture<Dds, TextureUsage>>,
    ),
}

#[derive(Debug, Error)]
pub enum CreateImageTextureError {
    #[error("error deswizzling surface")]
    SwizzleMibl(#[from] xc3_lib::mibl::SwizzleError),

    #[error("error deswizzling surface")]
    SwizzleMtxt(#[from] xc3_lib::mtxt::SwizzleError),

    #[error("error reading data")]
    Binrw(#[from] binrw::Error),

    #[error("error decompressing stream")]
    Stream(#[from] xc3_lib::error::DecompressStreamError),

    #[error("error converting image surface")]
    Surface(#[from] image_dds::error::SurfaceError),

    #[error("error converting Mibl texture")]
    Mibl(#[from] xc3_lib::mibl::CreateMiblError),
}

/// A non swizzled version of an [Mibl] or [Mtxt] texture.
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Clone)]
pub struct ImageTexture {
    /// An optional name assigned to some textures.
    /// This will typically be [None]
    /// and can not be used for reliably identifying textures.
    pub name: Option<String>,
    /// Hints on how the texture is used.
    /// Actual usage is determined by the shader code.
    pub usage: Option<TextureUsage>,
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
    ///
    /// The `name` is not required but creates more descriptive file names and debug information.
    /// The `usage` improves the accuracy of texture assignments if the shader database is not specified.
    pub fn from_mibl(
        mibl: &Mibl,
        name: Option<String>,
        usage: Option<TextureUsage>,
    ) -> Result<Self, CreateImageTextureError> {
        Ok(Self {
            name,
            usage,
            width: mibl.footer.width,
            height: mibl.footer.height,
            depth: mibl.footer.depth,
            view_dimension: mibl.footer.view_dimension,
            image_format: mibl.footer.image_format,
            mipmap_count: mibl.footer.mipmap_count,
            image_data: mibl.deswizzled_image_data()?,
        })
    }

    /// Deswizzle the data from `mtxt`.
    ///
    /// The `name` is not required but creates more descriptive file names and debug information.
    /// The `usage` improves the accuracy of texture assignments if the shader database is not specified.
    pub fn from_mtxt(
        mtxt: &Mtxt,
        name: Option<String>,
        usage: Option<xc3_lib::mxmd::legacy::TextureUsage>,
    ) -> Result<Self, CreateImageTextureError> {
        Ok(Self {
            name,
            usage: usage.and_then(mtxt_usage),
            width: mtxt.footer.width,
            height: mtxt.footer.height,
            depth: mtxt.footer.depth_or_array_layers,
            view_dimension: ViewDimension::D2,
            image_format: mtxt_image_format(mtxt.footer.surface_format),
            mipmap_count: mtxt.footer.mipmap_count,
            image_data: mtxt.deswizzled_image_data()?,
        })
    }

    pub(crate) fn from_packed_texture(
        texture: &PackedTexture,
    ) -> Result<Self, CreateImageTextureError> {
        let mibl = Mibl::from_bytes(&texture.mibl_data)?;
        Self::from_mibl(&mibl, Some(texture.name.clone()), Some(texture.usage))
    }

    /// Decode the first mip level for all depth slices and array layers to RGBA8.
    pub fn to_image(&self) -> Result<image_dds::image::RgbaImage, CreateImageError> {
        // Only decode the mip we actually use to improve performance.
        self.to_surface()
            .decode_layers_mipmaps_rgba8(0..self.layers(), 0..1)?
            .into_image()
    }

    /// Return the number of array layers in this surface.
    pub fn layers(&self) -> u32 {
        if self.view_dimension == ViewDimension::Cube {
            6
        } else {
            1
        }
    }

    /// Create a view of all image data in this texture
    /// to use with encode or decode operations.
    pub fn to_surface(&self) -> image_dds::Surface<&[u8]> {
        Surface {
            width: self.width,
            height: self.height,
            depth: self.depth,
            layers: self.layers(),
            mipmaps: self.mipmap_count,
            image_format: self.image_format.into(),
            data: &self.image_data,
        }
    }

    // TODO: use a dedicated error type
    pub fn to_dds(&self) -> Result<Dds, CreateDdsError> {
        self.to_surface().to_dds()
    }

    pub fn from_surface<T: AsRef<[u8]>>(
        surface: Surface<T>,
        name: Option<String>,
        usage: Option<TextureUsage>,
    ) -> Result<Self, CreateImageTextureError> {
        Ok(Self {
            name,
            usage,
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

    pub fn from_dds(
        dds: &Dds,
        name: Option<String>,
        usage: Option<TextureUsage>,
    ) -> Result<Self, CreateImageTextureError> {
        Self::from_surface(Surface::from_dds(dds)?, name, usage)
    }

    pub fn to_mibl(&self) -> Result<Mibl, CreateMiblError> {
        Mibl::from_surface(self.to_surface())
    }

    pub(crate) fn to_extracted_texture(&self) -> ExtractedTexture<Mibl, TextureUsage> {
        // TODO: Avoid unwrap.
        let mibl = self.to_mibl().unwrap();
        ExtractedTexture::from_mibl(
            &mibl,
            self.name.clone().unwrap_or_default(),
            self.usage.unwrap_or(TextureUsage::Col),
        )
    }
}

// TODO: Should the publicly exposed image format type just use image_dds?
fn mtxt_image_format(image_format: xc3_lib::mtxt::SurfaceFormat) -> ImageFormat {
    match image_format {
        xc3_lib::mtxt::SurfaceFormat::R8G8B8A8Unorm => ImageFormat::R8G8B8A8Unorm,
        xc3_lib::mtxt::SurfaceFormat::BC1Unorm => ImageFormat::BC1Unorm,
        xc3_lib::mtxt::SurfaceFormat::BC2Unorm => ImageFormat::BC2Unorm,
        xc3_lib::mtxt::SurfaceFormat::BC3Unorm => ImageFormat::BC3Unorm,
        xc3_lib::mtxt::SurfaceFormat::BC4Unorm => ImageFormat::BC4Unorm,
        xc3_lib::mtxt::SurfaceFormat::BC5Unorm => ImageFormat::BC5Unorm,
    }
}

fn mtxt_usage(usage: xc3_lib::mxmd::legacy::TextureUsage) -> Option<TextureUsage> {
    // TODO: Create a separate enum instead of exposing the xc3_lib types?
    match usage {
        xc3_lib::mxmd::legacy::TextureUsage::Nrm => Some(TextureUsage::Nrm),
        xc3_lib::mxmd::legacy::TextureUsage::Unk32 => Some(TextureUsage::Col),
        xc3_lib::mxmd::legacy::TextureUsage::Unk48 => Some(TextureUsage::Col),
        xc3_lib::mxmd::legacy::TextureUsage::Col => Some(TextureUsage::Col),
        xc3_lib::mxmd::legacy::TextureUsage::Unk96 => Some(TextureUsage::Col),
        xc3_lib::mxmd::legacy::TextureUsage::Nrm2 => Some(TextureUsage::Nrm),
        xc3_lib::mxmd::legacy::TextureUsage::Cube => None,
        _ => None,
    }
}

pub fn load_textures(
    textures: &ExtractedTextures,
) -> Result<Vec<ImageTexture>, CreateImageTextureError> {
    // TODO: what is the correct priority for the different texture sources?
    match textures {
        ExtractedTextures::Switch(textures) => textures
            .iter()
            .map(|texture| {
                ImageTexture::from_surface(
                    texture.surface_final()?,
                    Some(texture.name.clone()),
                    Some(texture.usage),
                )
            })
            .collect(),
        ExtractedTextures::Pc(textures) => textures
            .iter()
            .map(|texture| {
                ImageTexture::from_dds(
                    texture.dds_final(),
                    Some(texture.name.clone()),
                    Some(texture.usage),
                )
            })
            .collect(),
    }
}

pub fn load_textures_legacy(
    mxmd: &MxmdLegacy,
    casmt: Option<Vec<u8>>,
) -> Result<(Vec<u16>, Vec<ImageTexture>), CreateImageTextureError> {
    let mut image_textures: Vec<_> = mxmd
        .packed_textures
        .as_ref()
        .map(|textures| {
            textures
                .textures
                .iter()
                .map(|t| {
                    let mtxt = Mtxt::from_bytes(&t.mtxt_data)?;
                    ImageTexture::from_mtxt(&mtxt, Some(t.name.clone()), Some(t.usage))
                })
                .collect()
        })
        .transpose()?
        .unwrap_or_default();

    // Material texture indices can be remapped.
    let mut low_texture_indices: Vec<_> = (0..image_textures.len() as u16).collect();

    // TODO: Share code for loading streaming data with legacy mibl data?
    if let Some(streaming) = &mxmd.streaming {
        if let Some(casmt) = casmt {
            // TODO: Why are the sizes sometimes 0?
            let low_data = get_bytes(&casmt, streaming.low_texture_data_offset, None)
                .map_err(binrw::Error::Io)?;
            let high_data =
                get_bytes(&casmt, streaming.texture_data_offset, None).map_err(binrw::Error::Io)?;

            let (indices, textures) =
                streaming
                    .inner
                    .extract_textures(low_data, high_data, |bytes| Mtxt::from_bytes(bytes))?;

            image_textures = textures
                .into_iter()
                .map(|t| {
                    ImageTexture::from_mtxt(t.mtxt_final(), Some(t.name.clone()), Some(t.usage))
                })
                .collect::<Result<Vec<_>, _>>()?;
            low_texture_indices = indices;
        }
    }

    Ok((low_texture_indices, image_textures))
}

#[cfg(feature = "arbitrary")]
fn arbitrary_dds_textures(
    _u: &mut arbitrary::Unstructured,
) -> arbitrary::Result<Vec<ExtractedTexture<Dds, TextureUsage>>> {
    // TODO: Generate random DDS files?
    Ok(Vec::new())
}
