use log::warn;
use wgpu::util::DeviceExt;
use xc3_lib::mibl::ImageFormat;
use xc3_model::ImageTexture;

pub fn create_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &ImageTexture,
) -> wgpu::Texture {
    let format = texture_format(texture.image_format);

    let layers = match texture.view_dimension {
        xc3_lib::mibl::ViewDimension::Cube => 6,
        _ => 1,
    };

    // TODO: Not all map textures are a multiple of the block width?
    // TODO: How to handle not being a multiple of the block dimensions?
    let (block_width, block_height) = format.block_dimensions();
    let rounded_width = texture.width.max(4) / 4 * 4;
    let rounded_height = texture.height.max(4) / 4 * 4;

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
        xc3_lib::mibl::ViewDimension::D2 => wgpu::TextureDimension::D2,
        xc3_lib::mibl::ViewDimension::D3 => wgpu::TextureDimension::D3,
        xc3_lib::mibl::ViewDimension::Cube => wgpu::TextureDimension::D2,
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
        &texture.image_data,
    )
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

fn texture_format(format: ImageFormat) -> wgpu::TextureFormat {
    match format {
        ImageFormat::R8Unorm => wgpu::TextureFormat::R8Unorm,
        ImageFormat::R8G8B8A8Unorm => wgpu::TextureFormat::Rgba8Unorm,
        ImageFormat::R16G16B16A16Float => wgpu::TextureFormat::Rgba16Float,
        ImageFormat::BC1Unorm => wgpu::TextureFormat::Bc1RgbaUnorm,
        ImageFormat::BC2Unorm => wgpu::TextureFormat::Bc2RgbaUnorm,
        ImageFormat::BC3Unorm => wgpu::TextureFormat::Bc3RgbaUnorm,
        ImageFormat::BC4Unorm => wgpu::TextureFormat::Bc4RUnorm,
        ImageFormat::BC5Unorm => wgpu::TextureFormat::Bc5RgUnorm,
        ImageFormat::BC7Unorm => wgpu::TextureFormat::Bc7RgbaUnorm,
        ImageFormat::B8G8R8A8Unorm => wgpu::TextureFormat::Bgra8Unorm,
    }
}
