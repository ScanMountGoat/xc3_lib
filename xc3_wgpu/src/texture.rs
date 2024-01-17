use std::borrow::Cow;

use log::warn;
use wgpu::util::DeviceExt;
use xc3_model::{ImageFormat, ImageTexture, ViewDimension};

pub fn create_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &ImageTexture,
) -> wgpu::Texture {
    let (format, data) = image_format_data(texture);

    let layers = match texture.view_dimension {
        ViewDimension::Cube => 6,
        _ => 1,
    };

    // TODO: Not all map textures are a multiple of the block width?
    // TODO: How to handle not being a multiple of the block dimensions?
    // Always choose a smaller size to avoid indexing out of bounds.
    let round_dimension = |x: u32, d: u32| {
        if x <= d {
            d
        } else {
            (x - d).next_multiple_of(d)
        }
    };

    let (block_width, block_height) = format.block_dimensions();
    let rounded_width = round_dimension(texture.width, block_width);
    let rounded_height = round_dimension(texture.height, block_height);

    if texture.width % block_width != 0
        || texture.height % block_height != 0
        || texture.width < block_width
        || texture.height < block_height
    {
        warn!(
            "Dimensions {}x{}x{} are not divisible by block dimensions {}x{}. Rounding to {}x{}x{}",
            texture.width,
            texture.height,
            texture.depth,
            block_width,
            block_height,
            rounded_width,
            rounded_height,
            texture.depth
        );
    }

    let size = wgpu::Extent3d {
        width: rounded_width,
        height: rounded_height,
        depth_or_array_layers: std::cmp::max(layers, texture.depth),
    };

    let dimension = match texture.view_dimension {
        ViewDimension::D2 => wgpu::TextureDimension::D2,
        ViewDimension::D3 => wgpu::TextureDimension::D3,
        ViewDimension::Cube => wgpu::TextureDimension::D2,
    };

    device.create_texture_with_data(
        queue,
        &wgpu::TextureDescriptor {
            label: texture.name.as_deref(),
            size,
            // TODO: Why are some mipmap counts too high?
            mip_level_count: texture.mipmap_count.min(size.max_mips(dimension)),
            sample_count: 1,
            dimension,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        },
        &data,
    )
}

fn image_format_data(texture: &ImageTexture) -> (wgpu::TextureFormat, Cow<'_, Vec<u8>>) {
    // Convert unsupported formats to rgba8 for compatibility.
    match texture_format(texture.image_format) {
        Some(format) => (format, Cow::Borrowed(&texture.image_data)),
        None => {
            let rgba8 = texture.to_surface().decode_rgba8().unwrap();
            (wgpu::TextureFormat::Rgba8Unorm, Cow::Owned(rgba8.data))
        }
    }
}

pub fn create_default_black_texture(device: &wgpu::Device, queue: &wgpu::Queue) -> wgpu::Texture {
    device.create_texture_with_data(
        queue,
        &wgpu::TextureDescriptor {
            label: Some("Default Black"),
            size: wgpu::Extent3d {
                width: 4,
                height: 4,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        },
        &[0u8; 4 * 4 * 4],
    )
}

fn texture_format(format: ImageFormat) -> Option<wgpu::TextureFormat> {
    match format {
        ImageFormat::R8Unorm => Some(wgpu::TextureFormat::R8Unorm),
        ImageFormat::R8G8B8A8Unorm => Some(wgpu::TextureFormat::Rgba8Unorm),
        ImageFormat::R16G16B16A16Float => Some(wgpu::TextureFormat::Rgba16Float),
        ImageFormat::R4G4B4A4Unorm => None,
        ImageFormat::BC1Unorm => Some(wgpu::TextureFormat::Bc1RgbaUnorm),
        ImageFormat::BC2Unorm => Some(wgpu::TextureFormat::Bc2RgbaUnorm),
        ImageFormat::BC3Unorm => Some(wgpu::TextureFormat::Bc3RgbaUnorm),
        ImageFormat::BC4Unorm => Some(wgpu::TextureFormat::Bc4RUnorm),
        ImageFormat::BC5Unorm => Some(wgpu::TextureFormat::Bc5RgUnorm),
        ImageFormat::BC7Unorm => Some(wgpu::TextureFormat::Bc7RgbaUnorm),
        ImageFormat::BC6UFloat => Some(wgpu::TextureFormat::Bc6hRgbUfloat),
        ImageFormat::B8G8R8A8Unorm => Some(wgpu::TextureFormat::Bgra8Unorm),
    }
}
