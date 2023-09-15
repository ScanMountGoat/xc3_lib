pub fn create_sampler(device: &wgpu::Device, sampler: &xc3_model::Sampler) -> wgpu::Sampler {
    device.create_sampler(&sampler_descriptor(sampler))
}

fn sampler_descriptor(sampler: &xc3_model::Sampler) -> wgpu::SamplerDescriptor<'static> {
    // TODO: anisotropic filtering and lod bias?
    wgpu::SamplerDescriptor {
        label: None,
        address_mode_u: address_mode(sampler.address_mode_u),
        address_mode_v: address_mode(sampler.address_mode_v),
        address_mode_w: address_mode(sampler.address_mode_w),
        mag_filter: filter_mode(sampler.mag_filter),
        min_filter: filter_mode(sampler.min_filter),
        mipmap_filter: wgpu::FilterMode::Nearest,
        lod_min_clamp: 0.0,
        // Values taken from tests using Ryujinx with Vulkan.
        lod_max_clamp: if sampler.mipmaps { 15.0 } else { 0.25 },
        ..Default::default()
    }
}

fn filter_mode(value: xc3_model::FilterMode) -> wgpu::FilterMode {
    match value {
        xc3_model::FilterMode::Nearest => wgpu::FilterMode::Nearest,
        xc3_model::FilterMode::Linear => wgpu::FilterMode::Linear,
    }
}

fn address_mode(value: xc3_model::AddressMode) -> wgpu::AddressMode {
    match value {
        xc3_model::AddressMode::ClampToEdge => wgpu::AddressMode::ClampToEdge,
        xc3_model::AddressMode::Repeat => wgpu::AddressMode::Repeat,
        xc3_model::AddressMode::MirrorRepeat => wgpu::AddressMode::MirrorRepeat,
    }
}
