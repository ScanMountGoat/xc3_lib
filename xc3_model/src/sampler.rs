/// See [SamplerFlags](xc3_lib::mxmd::SamplerFlags).
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Clone, Eq)]
pub struct Sampler {
    /// Addressing for the U or S texture coordinate.
    pub address_mode_u: AddressMode,
    /// Addressing for the V or T texture coordinate.
    pub address_mode_v: AddressMode,
    /// Addressing for the W or R texture coordinate.
    pub address_mode_w: AddressMode,
    pub min_filter: FilterMode,
    pub mag_filter: FilterMode,
    /// Enables rendering mipmaps past the base mip when `true`.
    pub mipmaps: bool,
}

/// Texel mixing mode when sampling between texels.
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum FilterMode {
    Nearest,
    Linear,
}

/// How edges should be handled in texture addressing.
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum AddressMode {
    ClampToEdge,
    Repeat,
    MirrorRepeat,
}

impl From<xc3_lib::mxmd::SamplerFlags> for Sampler {
    fn from(flags: xc3_lib::mxmd::SamplerFlags) -> Self {
        Self {
            address_mode_u: address_mode(flags.repeat_u(), flags.mirror_u()),
            address_mode_v: address_mode(flags.repeat_v(), flags.mirror_v()),
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: filter_mode(flags.nearest()),
            min_filter: filter_mode(flags.nearest()),
            mipmaps: !flags.disable_mipmap_filter(),
        }
    }
}

fn filter_mode(nearest: bool) -> FilterMode {
    if nearest {
        FilterMode::Nearest
    } else {
        FilterMode::Linear
    }
}

fn address_mode(repeat: bool, mirror: bool) -> AddressMode {
    if mirror {
        AddressMode::MirrorRepeat
    } else if repeat {
        AddressMode::Repeat
    } else {
        AddressMode::ClampToEdge
    }
}

#[cfg(test)]
mod tests {
    use xc3_lib::mxmd::SamplerFlags;

    use super::*;

    // Test various flags values based on testing Vulkan samplers in RenderDoc.
    #[test]
    fn descriptor_0x0() {
        assert_eq!(
            Sampler {
                address_mode_u: AddressMode::ClampToEdge,
                address_mode_v: AddressMode::ClampToEdge,
                address_mode_w: AddressMode::ClampToEdge,
                mag_filter: FilterMode::Linear,
                min_filter: FilterMode::Linear,
                mipmaps: true,
            },
            Sampler::from(SamplerFlags::from(0x0))
        );
    }

    #[test]
    fn descriptor_0x3() {
        assert_eq!(
            Sampler {
                address_mode_u: AddressMode::Repeat,
                address_mode_v: AddressMode::Repeat,
                address_mode_w: AddressMode::ClampToEdge,
                mag_filter: FilterMode::Linear,
                min_filter: FilterMode::Linear,
                mipmaps: true,
            },
            Sampler::from(SamplerFlags::from(0b_11))
        );
    }

    #[test]
    fn descriptor_0x6() {
        assert_eq!(
            Sampler {
                address_mode_u: AddressMode::MirrorRepeat,
                address_mode_v: AddressMode::Repeat,
                address_mode_w: AddressMode::ClampToEdge,
                mag_filter: FilterMode::Linear,
                min_filter: FilterMode::Linear,
                mipmaps: true,
            },
            Sampler::from(SamplerFlags::from(0b_110))
        );
    }

    #[test]
    fn descriptor_0x12() {
        assert_eq!(
            Sampler {
                address_mode_u: AddressMode::MirrorRepeat,
                address_mode_v: AddressMode::MirrorRepeat,
                address_mode_w: AddressMode::ClampToEdge,
                mag_filter: FilterMode::Linear,
                min_filter: FilterMode::Linear,
                mipmaps: true,
            },
            Sampler::from(SamplerFlags::from(0b_1100))
        );
    }

    #[test]
    fn descriptor_0x40() {
        assert_eq!(
            Sampler {
                address_mode_u: AddressMode::ClampToEdge,
                address_mode_v: AddressMode::ClampToEdge,
                address_mode_w: AddressMode::ClampToEdge,
                mag_filter: FilterMode::Linear,
                min_filter: FilterMode::Linear,
                mipmaps: false,
            },
            Sampler::from(SamplerFlags::from(0b_01000000))
        );
    }

    #[test]
    fn descriptor_0x50() {
        assert_eq!(
            Sampler {
                address_mode_u: AddressMode::ClampToEdge,
                address_mode_v: AddressMode::ClampToEdge,
                address_mode_w: AddressMode::ClampToEdge,
                mag_filter: FilterMode::Nearest,
                min_filter: FilterMode::Nearest,
                mipmaps: false,
            },
            Sampler::from(SamplerFlags::from(0b_01010000))
        );
    }
}
