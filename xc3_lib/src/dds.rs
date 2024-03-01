//! Conversions between [Mibl] and [Dds].
use std::{io::Cursor, path::Path};

use image_dds::ddsfile::Dds;
use image_dds::Surface;
use thiserror::Error;

use crate::{
    mibl::{CreateMiblError, Mibl},
    mibl::{ImageFormat, ViewDimension},
};

pub trait DdsExt: Sized {
    type Error;

    fn from_bytes<T: AsRef<[u8]>>(bytes: T) -> Result<Self, Self::Error>;
    fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, Self::Error>;
    fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), Self::Error>;
}

impl DdsExt for Dds {
    type Error = image_dds::ddsfile::Error;

    fn from_bytes<T: AsRef<[u8]>>(bytes: T) -> Result<Self, Self::Error> {
        Self::read(&mut Cursor::new(bytes))
    }

    fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, Self::Error> {
        let mut reader = Cursor::new(std::fs::read(path)?);
        Dds::read(&mut reader)
    }

    fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), Self::Error> {
        let mut writer = Cursor::new(Vec::new());
        self.write(&mut writer)?;
        std::fs::write(path, writer.into_inner()).map_err(Into::into)
    }
}

#[derive(Debug, Error)]
pub enum CreateDdsError {
    #[error("error deswizzling surface")]
    SwizzleError(#[from] tegra_swizzle::SwizzleError),

    #[error("error creating DDS")]
    DdsError(#[from] image_dds::CreateDdsError),
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

    /// Swizzles all layers and mipmaps in `dds` to an equivalent [Mibl].
    ///
    /// Returns an error if the conversion fails or the image format is not supported.
    pub fn from_dds(dds: &Dds) -> Result<Self, CreateMiblError> {
        let surface = image_dds::Surface::from_dds(dds)?;
        Self::from_surface(surface)
    }
}

impl From<ImageFormat> for image_dds::ImageFormat {
    fn from(value: ImageFormat) -> Self {
        match value {
            ImageFormat::R8Unorm => image_dds::ImageFormat::R8Unorm,
            ImageFormat::R8G8B8A8Unorm => image_dds::ImageFormat::Rgba8Unorm,
            ImageFormat::R16G16B16A16Float => image_dds::ImageFormat::Rgba16Float,
            ImageFormat::R4G4B4A4Unorm => image_dds::ImageFormat::Bgra4Unorm,
            ImageFormat::BC1Unorm => image_dds::ImageFormat::BC1RgbaUnorm,
            ImageFormat::BC2Unorm => image_dds::ImageFormat::BC2RgbaUnorm,
            ImageFormat::BC3Unorm => image_dds::ImageFormat::BC3RgbaUnorm,
            ImageFormat::BC4Unorm => image_dds::ImageFormat::BC4RUnorm,
            ImageFormat::BC5Unorm => image_dds::ImageFormat::BC5RgUnorm,
            ImageFormat::BC7Unorm => image_dds::ImageFormat::BC7RgbaUnorm,
            ImageFormat::BC6UFloat => image_dds::ImageFormat::BC6hRgbUfloat,
            ImageFormat::B8G8R8A8Unorm => image_dds::ImageFormat::Bgra8Unorm,
        }
    }
}

impl TryFrom<image_dds::ImageFormat> for ImageFormat {
    type Error = CreateMiblError;

    fn try_from(value: image_dds::ImageFormat) -> Result<Self, Self::Error> {
        match value {
            image_dds::ImageFormat::R8Unorm => Ok(ImageFormat::R8Unorm),
            image_dds::ImageFormat::Rgba8Unorm => Ok(ImageFormat::R8G8B8A8Unorm),
            image_dds::ImageFormat::Rgba16Float => Ok(ImageFormat::R16G16B16A16Float),
            image_dds::ImageFormat::Bgra8Unorm => Ok(ImageFormat::B8G8R8A8Unorm),
            image_dds::ImageFormat::BC1RgbaUnorm => Ok(ImageFormat::BC1Unorm),
            image_dds::ImageFormat::BC2RgbaUnorm => Ok(ImageFormat::BC2Unorm),
            image_dds::ImageFormat::BC3RgbaUnorm => Ok(ImageFormat::BC3Unorm),
            image_dds::ImageFormat::BC4RUnorm => Ok(ImageFormat::BC4Unorm),
            image_dds::ImageFormat::BC5RgUnorm => Ok(ImageFormat::BC5Unorm),
            image_dds::ImageFormat::BC6hRgbUfloat => Ok(ImageFormat::BC6UFloat),
            image_dds::ImageFormat::BC7RgbaUnorm => Ok(ImageFormat::BC7Unorm),
            image_dds::ImageFormat::Bgra4Unorm => Ok(ImageFormat::R4G4B4A4Unorm),
            _ => Err(CreateMiblError::UnsupportedImageFormat(value)),
        }
    }
}
