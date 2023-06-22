use ddsfile::{D3DFormat, Dds, DxgiFormat, FourCC};

use anyhow::Result;

use crate::{
    mibl::Mibl,
    mibl::{ImageFormat, MiblFooter, ViewDimension},
};

// TODO: Create a write_dds and save_dds helper functions or trait?

// TODO: add a dds_from_surface() function to image_dds that takes a compressed surface?
pub fn create_dds(mibl: &Mibl) -> Result<Dds> {
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
        array_layers: if mibl.footer.view_dimension == ViewDimension::Cube {
            Some(6)
        } else {
            None
        },
        caps2: None,
        is_cubemap: false,
        resource_dimension: if mibl.footer.depth > 1 {
            ddsfile::D3D10ResourceDimension::Texture3D
        } else {
            ddsfile::D3D10ResourceDimension::Texture2D
        },
        alpha_mode: ddsfile::AlphaMode::Straight, // TODO: Does this matter?
    })?;

    dds.data = mibl.deswizzled_image_data()?;

    Ok(dds)
}

// TODO: Add a more general from_image_data function.
pub fn create_mibl(dds: &Dds) -> Result<Mibl> {
    // TODO: Avoid unwrap.
    let image_format =
        dds_image_format(dds).unwrap_or_else(|| panic!("{:?}", dds.get_dxgi_format().unwrap()));

    let layer_count = layer_count(dds);

    let mut image_data = tegra_swizzle::surface::swizzle_surface(
        dds.get_width() as usize,
        dds.get_height() as usize,
        dds.get_depth() as usize,
        &dds.data,
        image_format.block_dim(),
        None,
        image_format.bytes_per_pixel(),
        dds.get_num_mipmap_levels() as usize,
        layer_count as usize,
    )?;

    // TODO: expose round up in tegra_swizzle?
    let aligned_size = tegra_swizzle::div_round_up(image_data.len(), 4096) * 4096;
    image_data.resize(aligned_size, 0);

    Ok(Mibl {
        image_data,
        footer: MiblFooter {
            image_size: aligned_size as u32,
            unk: 4096,
            width: dds.get_width(),
            height: dds.get_height(),
            depth: dds.get_depth(),
            view_dimension: if dds.get_depth() > 1 {
                ViewDimension::D3
            } else if layer_count == 6 {
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

impl From<ImageFormat> for ddsfile::DxgiFormat {
    fn from(value: ImageFormat) -> Self {
        match value {
            ImageFormat::R8Unorm => Self::R8_UNorm,
            ImageFormat::R8G8B8A8Unorm => Self::R8G8B8A8_UNorm,
            ImageFormat::R16G16B16A16Float => Self::R16G16B16A16_Float,
            ImageFormat::BC1Unorm => Self::BC1_UNorm,
            ImageFormat::BC3Unorm => Self::BC3_UNorm,
            ImageFormat::BC4Unorm => Self::BC4_UNorm,
            ImageFormat::BC5Unorm => Self::BC5_UNorm,
            ImageFormat::BC7Unorm => Self::BC7_UNorm,
            ImageFormat::B8G8R8A8Unorm => Self::B8G8R8A8_UNorm,
        }
    }
}

// TODO: Convert image format the other way.

fn layer_count(dds: &Dds) -> u32 {
    // Array layers for DDS are calculated differently for cube maps.
    if matches!(&dds.header10, Some(header10) if header10.misc_flag == ddsfile::MiscFlag::TEXTURECUBE)
    {
        dds.get_num_array_layers() * 6
    } else {
        dds.get_num_array_layers()
    }
}

fn dds_image_format(dds: &Dds) -> Option<ImageFormat> {
    // The format can be DXGI, D3D, or specified in the FOURCC.
    // This is necessary for compatibility with different programs.
    let dxgi = dds.get_dxgi_format();
    let d3d = dds.get_d3d_format();
    let fourcc = dds.header.spf.fourcc.as_ref();

    dxgi.and_then(image_format_from_dxgi)
        .or_else(|| d3d.and_then(image_format_from_d3d))
        .or_else(|| fourcc.and_then(image_format_from_fourcc))
}

fn image_format_from_dxgi(format: DxgiFormat) -> Option<ImageFormat> {
    match format {
        DxgiFormat::R8_UNorm => Some(ImageFormat::R8Unorm),
        DxgiFormat::R8G8B8A8_UNorm => Some(ImageFormat::R8G8B8A8Unorm),
        DxgiFormat::R16G16B16A16_Float => Some(ImageFormat::R16G16B16A16Float),
        DxgiFormat::BC1_UNorm => Some(ImageFormat::BC1Unorm),
        DxgiFormat::BC3_UNorm => Some(ImageFormat::BC3Unorm),
        DxgiFormat::BC4_UNorm => Some(ImageFormat::BC4Unorm),
        DxgiFormat::BC5_UNorm => Some(ImageFormat::BC5Unorm),
        DxgiFormat::BC7_UNorm => Some(ImageFormat::BC7Unorm),
        DxgiFormat::B8G8R8A8_UNorm => Some(ImageFormat::B8G8R8A8Unorm),
        _ => None,
    }
}

fn image_format_from_d3d(format: D3DFormat) -> Option<ImageFormat> {
    match format {
        D3DFormat::DXT1 => Some(ImageFormat::BC1Unorm),
        D3DFormat::DXT4 => Some(ImageFormat::BC3Unorm),
        D3DFormat::DXT5 => Some(ImageFormat::BC3Unorm),
        _ => None,
    }
}

const BC5U: u32 = u32::from_le_bytes(*b"BC5U");
const ATI2: u32 = u32::from_le_bytes(*b"ATI2");

fn image_format_from_fourcc(fourcc: &FourCC) -> Option<ImageFormat> {
    match fourcc.0 {
        FourCC::DXT1 => Some(ImageFormat::BC1Unorm),
        FourCC::DXT4 => Some(ImageFormat::BC3Unorm),
        FourCC::DXT5 => Some(ImageFormat::BC3Unorm),
        FourCC::BC4_UNORM => Some(ImageFormat::BC4Unorm),
        ATI2 | BC5U => Some(ImageFormat::BC5Unorm),
        _ => None,
    }
}
