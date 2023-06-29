use wgpu::util::DeviceExt;
use xc3_lib::mibl::ImageFormat;
use xc3_model::texture::ImageTexture;

pub fn create_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &ImageTexture,
) -> wgpu::Texture {
    let width = texture.width;
    let height = texture.height;
    let depth = texture.depth;
    let view_dimension = texture.view_dimension;
    let image_format = texture.image_format;
    let mipmap_count = texture.mipmap_count;
    let layers = match view_dimension {
        xc3_lib::mibl::ViewDimension::Cube => 6,
        _ => 1,
    };

    let size = wgpu::Extent3d {
        // TODO: How to handle not being a multiple of the block dimensions?
        width: width.max(4),
        height: height.max(4),
        depth_or_array_layers: std::cmp::max(layers, depth),
    };

    let dimension = match view_dimension {
        xc3_lib::mibl::ViewDimension::D2 => wgpu::TextureDimension::D2,
        xc3_lib::mibl::ViewDimension::D3 => wgpu::TextureDimension::D3,
        xc3_lib::mibl::ViewDimension::Cube => wgpu::TextureDimension::D2,
    };

    // TODO: label?
    device.create_texture_with_data(
        queue,
        &wgpu::TextureDescriptor {
            label: None,
            size,
            // TODO: Why are some mipmap counts too high?
            mip_level_count: mipmap_count.min(size.max_mips(dimension)),
            sample_count: 1,
            dimension,
            format: texture_format(image_format),
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
        ImageFormat::BC3Unorm => wgpu::TextureFormat::Bc3RgbaUnorm,
        ImageFormat::BC4Unorm => wgpu::TextureFormat::Bc4RUnorm,
        ImageFormat::BC5Unorm => wgpu::TextureFormat::Bc5RgUnorm,
        ImageFormat::BC7Unorm => wgpu::TextureFormat::Bc7RgbaUnorm,
        ImageFormat::B8G8R8A8Unorm => wgpu::TextureFormat::Bgra8Unorm,
    }
}
