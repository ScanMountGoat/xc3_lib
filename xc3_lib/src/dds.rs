//! Conversions between [Mibl] and [ddsfile::Dds].
use std::{io::BufWriter, path::Path};

use image_dds::ddsfile::Dds;
use image_dds::Surface;
use thiserror::Error;

use crate::{
    mibl::Mibl,
    mibl::{ImageFormat, MiblFooter, ViewDimension},
    PAGE_SIZE,
};
use xc3_write::round_up;

#[derive(Debug, Error)]
pub enum CreateDdsError {
    #[error("error deswizzling surface: {0}")]
    SwizzleError(#[from] tegra_swizzle::SwizzleError),

    #[error("error creating DDS: {0}")]
    DdsError(#[from] image_dds::CreateDdsError),
}

#[derive(Debug, Error)]
pub enum CreateMiblError {
    #[error("error swizzling surface: {0}")]
    SwizzleError(#[from] tegra_swizzle::SwizzleError),

    #[error("error creating surface from DDS: {0}")]
    DdsError(#[from] image_dds::error::SurfaceError),

    #[error("image format {0:?} is not supported by Mibl")]
    UnsupportedImageFormat(image_dds::ImageFormat),
}

pub fn save_dds<P: AsRef<Path>>(path: P, dds: &Dds) {
    let mut writer = BufWriter::new(std::fs::File::create(path).unwrap());
    dds.write(&mut writer).unwrap();
}

impl Mibl {
    /// Deswizzles all layers and mipmaps to a Direct Draw Surface (DDS).
    pub fn to_dds(&self) -> Result<Dds, CreateDdsError> {
        Surface {
            width: self.footer.width,
            height: self.footer.height,
            depth: self.footer.depth,
            layers: if self.footer.view_dimension == ViewDimension::Cube {
                6
            } else {
                1
            },
            mipmaps: self.footer.mipmap_count,
            image_format: self.footer.image_format.into(),
            data: self.deswizzled_image_data()?,
        }
        .to_dds()
        .map_err(Into::into)
    }

    // TODO: Add a more general from_image_data function.
    /// Swizzles all layers and mipmaps in `dds` to an equivalent [Mibl].
    ///
    /// Returns an error if the conversion fails or the image format is
    /// not supported.
    pub fn from_dds(dds: &Dds) -> Result<Self, CreateMiblError> {
        let Surface {
            width,
            height,
            depth,
            layers,
            mipmaps,
            image_format,
            data,
        } = image_dds::Surface::from_dds(dds)?;
        let image_format = ImageFormat::try_from(image_format)?;

        let mut image_data = tegra_swizzle::surface::swizzle_surface(
            width as usize,
            height as usize,
            depth as usize,
            data,
            image_format.block_dim(),
            None,
            image_format.bytes_per_pixel(),
            mipmaps as usize,
            layers as usize,
        )?;

        // TODO: expose round up in tegra_swizzle?
        let aligned_size = round_up(image_data.len() as u64, PAGE_SIZE);
        image_data.resize(aligned_size as usize, 0);

        Ok(Self {
            image_data,
            footer: MiblFooter {
                image_size: aligned_size as u32,
                unk: 4096,
                width: dds.get_width(),
                height: dds.get_height(),
                depth: dds.get_depth(),
                view_dimension: if dds.get_depth() > 1 {
                    ViewDimension::D3
                } else if layers == 6 {
                    ViewDimension::Cube
                } else {
                    ViewDimension::D2
                },
                image_format,
                mipmap_count: dds.get_num_mipmap_levels(),
                version: 10001,
            },
        })
    }
}

impl From<ImageFormat> for image_dds::ImageFormat {
    fn from(value: ImageFormat) -> Self {
        match value {
            ImageFormat::R8Unorm => image_dds::ImageFormat::R8Unorm,
            ImageFormat::R8G8B8A8Unorm => image_dds::ImageFormat::R8G8B8A8Unorm,
            ImageFormat::R16G16B16A16Float => image_dds::ImageFormat::R16G16B16A16Float,
            ImageFormat::BC1Unorm => image_dds::ImageFormat::BC1Unorm,
            ImageFormat::BC2Unorm => image_dds::ImageFormat::BC2Unorm,
            ImageFormat::BC3Unorm => image_dds::ImageFormat::BC3Unorm,
            ImageFormat::BC4Unorm => image_dds::ImageFormat::BC4Unorm,
            ImageFormat::BC5Unorm => image_dds::ImageFormat::BC5Unorm,
            ImageFormat::BC7Unorm => image_dds::ImageFormat::BC7Unorm,
            ImageFormat::B8G8R8A8Unorm => image_dds::ImageFormat::B8G8R8A8Unorm,
        }
    }
}

impl TryFrom<image_dds::ImageFormat> for ImageFormat {
    type Error = CreateMiblError;

    fn try_from(value: image_dds::ImageFormat) -> Result<Self, Self::Error> {
        match value {
            image_dds::ImageFormat::R8Unorm => Ok(ImageFormat::R8Unorm),
            image_dds::ImageFormat::R8G8B8A8Unorm => Ok(ImageFormat::R8G8B8A8Unorm),
            image_dds::ImageFormat::R16G16B16A16Float => Ok(ImageFormat::R16G16B16A16Float),
            image_dds::ImageFormat::B8G8R8A8Unorm => Ok(ImageFormat::B8G8R8A8Unorm),
            image_dds::ImageFormat::BC1Unorm => Ok(ImageFormat::BC1Unorm),
            image_dds::ImageFormat::BC2Unorm => Ok(ImageFormat::BC2Unorm),
            image_dds::ImageFormat::BC3Unorm => Ok(ImageFormat::BC3Unorm),
            image_dds::ImageFormat::BC4Unorm => Ok(ImageFormat::BC4Unorm),
            image_dds::ImageFormat::BC5Unorm => Ok(ImageFormat::BC5Unorm),
            image_dds::ImageFormat::BC7Unorm => Ok(ImageFormat::BC7Unorm),
            _ => Err(CreateMiblError::UnsupportedImageFormat(value)),
        }
    }
}
