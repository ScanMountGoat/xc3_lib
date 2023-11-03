//! Conversions between [Mibl] and [ddsfile::Dds].
use std::{io::BufWriter, path::Path};

use anyhow::Result;
use image_dds::ddsfile::Dds;
use image_dds::Surface;

use crate::{
    mibl::Mibl,
    mibl::{ImageFormat, MiblFooter, ViewDimension},
    PAGE_SIZE,
};
use xc3_write::round_up;

pub fn save_dds<P: AsRef<Path>>(path: P, dds: &Dds) {
    let mut writer = BufWriter::new(std::fs::File::create(path).unwrap());
    dds.write(&mut writer).unwrap();
}

impl Mibl {
    /// Deswizzles all layers and mipmaps to a Direct Draw Surface (DDS).
    pub fn to_dds(&self) -> Result<Dds> {
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
            image_format: surface_image_format(self.footer.image_format).unwrap(),
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
    pub fn from_dds(dds: &Dds) -> Result<Self> {
        // TODO: Avoid unwrap.
        let Surface {
            width,
            height,
            depth,
            layers,
            mipmaps,
            image_format,
            data,
        } = image_dds::Surface::from_dds(dds).unwrap();
        let image_format = image_format_from_surface(image_format).unwrap();

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

// TODO: try_into?
pub fn surface_image_format(value: ImageFormat) -> Option<image_dds::ImageFormat> {
    match value {
        ImageFormat::R8Unorm => Some(image_dds::ImageFormat::R8Unorm),
        ImageFormat::R8G8B8A8Unorm => Some(image_dds::ImageFormat::R8G8B8A8Unorm),
        ImageFormat::R16G16B16A16Float => Some(image_dds::ImageFormat::R16G16B16A16Float),
        ImageFormat::BC1Unorm => Some(image_dds::ImageFormat::BC1Unorm),
        ImageFormat::BC2Unorm => Some(image_dds::ImageFormat::BC2Unorm),
        ImageFormat::BC3Unorm => Some(image_dds::ImageFormat::BC3Unorm),
        ImageFormat::BC4Unorm => Some(image_dds::ImageFormat::BC4Unorm),
        ImageFormat::BC5Unorm => Some(image_dds::ImageFormat::BC5Unorm),
        ImageFormat::BC7Unorm => Some(image_dds::ImageFormat::BC7Unorm),
        ImageFormat::B8G8R8A8Unorm => Some(image_dds::ImageFormat::B8G8R8A8Unorm),
    }
}

pub fn image_format_from_surface(value: image_dds::ImageFormat) -> Option<ImageFormat> {
    match value {
        image_dds::ImageFormat::R8Unorm => Some(ImageFormat::R8Unorm),
        image_dds::ImageFormat::R8G8B8A8Unorm => Some(ImageFormat::R8G8B8A8Unorm),
        image_dds::ImageFormat::R8G8B8A8Srgb => None,
        image_dds::ImageFormat::R16G16B16A16Float => Some(ImageFormat::R16G16B16A16Float),
        image_dds::ImageFormat::R32G32B32A32Float => None,
        image_dds::ImageFormat::B8G8R8A8Unorm => Some(ImageFormat::B8G8R8A8Unorm),
        image_dds::ImageFormat::B8G8R8A8Srgb => None,
        image_dds::ImageFormat::BC1Unorm => Some(ImageFormat::BC1Unorm),
        image_dds::ImageFormat::BC1Srgb => None,
        image_dds::ImageFormat::BC2Unorm => Some(ImageFormat::BC2Unorm),
        image_dds::ImageFormat::BC2Srgb => None,
        image_dds::ImageFormat::BC3Unorm => Some(ImageFormat::BC3Unorm),
        image_dds::ImageFormat::BC3Srgb => None,
        image_dds::ImageFormat::BC4Unorm => Some(ImageFormat::BC4Unorm),
        image_dds::ImageFormat::BC4Snorm => None,
        image_dds::ImageFormat::BC5Unorm => Some(ImageFormat::BC5Unorm),
        image_dds::ImageFormat::BC5Snorm => None,
        image_dds::ImageFormat::BC6Ufloat => None,
        image_dds::ImageFormat::BC6Sfloat => None,
        image_dds::ImageFormat::BC7Unorm => Some(ImageFormat::BC7Unorm),
        image_dds::ImageFormat::BC7Srgb => None,
    }
}
