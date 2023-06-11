use xc3_lib::mxmd::SamplerFlags;

pub fn create_sampler(device: &wgpu::Device, flags: SamplerFlags) -> wgpu::Sampler {
    device.create_sampler(&sampler_descriptor(flags))
}

fn sampler_descriptor(flags: SamplerFlags) -> wgpu::SamplerDescriptor<'static> {
    // TODO: anisotropic filtering and lod bias?
    wgpu::SamplerDescriptor {
        label: None,
        address_mode_u: address_mode(flags.repeat_u(), flags.mirror_u()),
        address_mode_v: address_mode(flags.repeat_v(), flags.mirror_v()),
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: filter_mode(flags.nearest()),
        min_filter: filter_mode(flags.nearest()),
        mipmap_filter: wgpu::FilterMode::Nearest,
        lod_min_clamp: 0.0,
        lod_max_clamp: if flags.disable_mipmap_filter() {
            0.25
        } else {
            15.0
        },
        ..Default::default()
    }
}

fn filter_mode(nearest: bool) -> wgpu::FilterMode {
    if nearest {
        wgpu::FilterMode::Nearest
    } else {
        wgpu::FilterMode::Linear
    }
}

fn address_mode(repeat: bool, mirror: bool) -> wgpu::AddressMode {
    if mirror {
        wgpu::AddressMode::MirrorRepeat
    } else if repeat {
        wgpu::AddressMode::Repeat
    } else {
        wgpu::AddressMode::ClampToEdge
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test various flags values based on testing Vulkan samplers in RenderDoc.
    #[test]
    fn descriptor_0x0() {
        assert_eq!(
            wgpu::SamplerDescriptor {
                label: None,
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                mipmap_filter: wgpu::FilterMode::Nearest,
                lod_min_clamp: 0.0,
                lod_max_clamp: 15.0,
                ..Default::default()
            },
            sampler_descriptor(SamplerFlags::from(0x0))
        );
    }

    #[test]
    fn descriptor_0x3() {
        assert_eq!(
            wgpu::SamplerDescriptor {
                label: None,
                address_mode_u: wgpu::AddressMode::Repeat,
                address_mode_v: wgpu::AddressMode::Repeat,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                mipmap_filter: wgpu::FilterMode::Nearest,
                lod_min_clamp: 0.0,
                lod_max_clamp: 15.0,
                ..Default::default()
            },
            sampler_descriptor(SamplerFlags::from(0b_11))
        );
    }

    #[test]
    fn descriptor_0x6() {
        assert_eq!(
            wgpu::SamplerDescriptor {
                label: None,
                address_mode_u: wgpu::AddressMode::MirrorRepeat,
                address_mode_v: wgpu::AddressMode::Repeat,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                mipmap_filter: wgpu::FilterMode::Nearest,
                lod_min_clamp: 0.0,
                lod_max_clamp: 15.0,
                ..Default::default()
            },
            sampler_descriptor(SamplerFlags::from(0b_110))
        );
    }

    #[test]
    fn descriptor_0x12() {
        assert_eq!(
            wgpu::SamplerDescriptor {
                label: None,
                address_mode_u: wgpu::AddressMode::MirrorRepeat,
                address_mode_v: wgpu::AddressMode::MirrorRepeat,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                mipmap_filter: wgpu::FilterMode::Nearest,
                lod_min_clamp: 0.0,
                lod_max_clamp: 15.0,
                ..Default::default()
            },
            sampler_descriptor(SamplerFlags::from(0b_1100))
        );
    }

    #[test]
    fn descriptor_0x40() {
        assert_eq!(
            wgpu::SamplerDescriptor {
                label: None,
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                mipmap_filter: wgpu::FilterMode::Nearest,
                lod_min_clamp: 0.0,
                lod_max_clamp: 0.25,
                ..Default::default()
            },
            sampler_descriptor(SamplerFlags::from(0b_01000000))
        );
    }

    #[test]
    fn descriptor_0x50() {
        assert_eq!(
            wgpu::SamplerDescriptor {
                label: None,
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Nearest,
                min_filter: wgpu::FilterMode::Nearest,
                mipmap_filter: wgpu::FilterMode::Nearest,
                lod_min_clamp: 0.0,
                lod_max_clamp: 0.25,
                ..Default::default()
            },
            sampler_descriptor(SamplerFlags::from(0b_01010000))
        );
    }
}
