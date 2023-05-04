use ddsfile::Dds;

use anyhow::Result;

use crate::{lbim::ImageFormat, Libm};

// TODO: add a dds_from_surface() function to image_dds that takes a compressed surface?
pub(crate) fn create_dds(mibl: &Libm) -> Result<Dds> {
    let mut dds = Dds::new_dxgi(ddsfile::NewDxgiParams {
        height: mibl.footer.height,
        width: mibl.footer.width,
        depth: if mibl.footer.depth > 1 {
            Some(mibl.footer.depth)
        } else {
            None
        },
        format: mibl.footer.image_format.into(),
        mipmap_levels: if mibl.footer.mipmap_count > 1 {
            Some(mibl.footer.mipmap_count)
        } else {
            None
        },
        array_layers: None, // TODO: cube maps?
        caps2: None,
        is_cubemap: false,
        resource_dimension: if mibl.footer.depth > 1 {
            ddsfile::D3D10ResourceDimension::Texture3D
        } else {
            ddsfile::D3D10ResourceDimension::Texture2D
        },
        alpha_mode: ddsfile::AlphaMode::Straight, // TODO: Does this matter?
    })?;

    dds.data = tegra_swizzle::surface::deswizzle_surface(
        mibl.footer.width as usize,
        mibl.footer.height as usize,
        mibl.footer.depth as usize,
        &mibl.image_data,
        mibl.footer.image_format.block_dim(),
        None,
        mibl.footer.image_format.bytes_per_pixel(),
        mibl.footer.mipmap_count as usize,
        1, // TODO: cube maps?
    )?;

    Ok(dds)
}

impl From<ImageFormat> for ddsfile::DxgiFormat {
    fn from(value: ImageFormat) -> Self {
        match value {
            ImageFormat::R8Unorm => Self::R8_UNorm,
            ImageFormat::R8G8B8A8Unorm => Self::R8G8B8A8_UNorm,
            ImageFormat::Bc1Unorm => Self::BC1_UNorm,
            ImageFormat::Bc3Unorm => Self::BC3_UNorm,
            ImageFormat::Bc4Unorm => Self::BC4_UNorm,
            ImageFormat::Bc5Unorm => Self::BC5_UNorm,
            ImageFormat::Bc7Unorm => Self::BC7_UNorm,
        }
    }
}
