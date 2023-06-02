use wgpu::util::DeviceExt;
use xc3_lib::mibl::{ImageFormat, Mibl, MiblFooter, ViewDimension};

pub fn create_texture(device: &wgpu::Device, queue: &wgpu::Queue, mibl: &Mibl) -> wgpu::Texture {
    let data = mibl.deswizzled_image_data().unwrap();

    create_texture_from_footer_data(
        device,
        queue,
        mibl.footer.width,
        mibl.footer.height,
        mibl.footer.depth,
        mibl.footer.view_dimension,
        mibl.footer.image_format,
        mibl.footer.mipmap_count,
        data,
    )
}

pub fn create_texture_with_base_mip(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    mibl: &Mibl,
    base_mip: &[u8],
) -> wgpu::Texture {
    // TODO: Find a cleaner way of doing this.
    // The base mip level doubles each dimension.
    let width = mibl.footer.width * 2;
    let height = mibl.footer.height * 2;
    let depth = if mibl.footer.depth > 1 {
        mibl.footer.depth * 2
    } else {
        mibl.footer.depth
    };

    // TODO: Don't require tegra_swizzle in this crate.
    // Deswizzle the single mip from the base level texture.
    let mut data_x2 = tegra_swizzle::surface::deswizzle_surface(
        width as usize,
        height as usize,
        depth as usize,
        base_mip,
        mibl.footer.image_format.block_dim(),
        None,
        mibl.footer.image_format.bytes_per_pixel(),
        1,
        if mibl.footer.view_dimension == ViewDimension::Cube {
            6
        } else {
            1
        },
    ).unwrap();

    data_x2.extend_from_slice(&mibl.deswizzled_image_data().unwrap());

    create_texture_from_footer_data(
        device,
        queue,
        width,
        height,
        depth,
        mibl.footer.view_dimension,
        mibl.footer.image_format,
        mibl.footer.mipmap_count + 1,
        data_x2,
    )
}

fn create_texture_from_footer_data(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    width: u32,
    height: u32,
    depth: u32,
    view_dimension: ViewDimension,
    image_format: ImageFormat,
    mipmap_count: u32,
    data: Vec<u8>,
) -> wgpu::Texture {
    let layers = match view_dimension {
        xc3_lib::mibl::ViewDimension::Cube => 6,
        _ => 1,
    };

    // TODO: label?
    device.create_texture_with_data(
        queue,
        &wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: width,
                height: height,
                depth_or_array_layers: std::cmp::max(layers, depth),
            },
            mip_level_count: mipmap_count,
            sample_count: 1,
            dimension: match view_dimension {
                xc3_lib::mibl::ViewDimension::D2 => wgpu::TextureDimension::D2,
                xc3_lib::mibl::ViewDimension::D3 => wgpu::TextureDimension::D3,
                xc3_lib::mibl::ViewDimension::Cube => wgpu::TextureDimension::D2,
            },
            format: texture_format(image_format),
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        },
        &data,
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
