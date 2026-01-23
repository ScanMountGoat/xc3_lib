// TODO: field for each flag field with getters to always create valid flags?
/// See [SamplerFlags](xc3_lib::mxmd::SamplerFlags).
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Clone)]
pub struct Sampler {
    pub address_mode_u: AddressMode,
    pub address_mode_v: AddressMode,
    pub address_mode_w: AddressMode,
    pub min_filter: FilterMode,
    pub mag_filter: FilterMode,
    pub mip_filter: FilterMode,
    /// Enables rendering mipmaps past the base mip when `true`.
    pub mipmaps: bool,
    pub lod_bias: f32,
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

impl Sampler {
    /// The highest rendered texture mipmap LOD.
    pub fn lod_max_clamp(&self) -> f32 {
        // Values taken from tests using Ryujinx with Vulkan.
        if self.mipmaps { 15.0 } else { 0.25 }
    }

    /// Returns `true` if the sampler uses anisotropic filtering.
    /// This is set to 4x in game.
    pub fn anisotropic_filtering(&self) -> bool {
        self.mipmaps
            && self.min_filter == FilterMode::Linear
            && self.mag_filter == FilterMode::Linear
            && self.mip_filter == FilterMode::Linear
    }

    fn repeat_uvw_3d(lod_bias: f32) -> Self {
        Self {
            address_mode_u: AddressMode::Repeat,
            address_mode_v: AddressMode::Repeat,
            address_mode_w: AddressMode::Repeat,
            min_filter: FilterMode::Linear,
            mag_filter: FilterMode::Linear,
            mip_filter: FilterMode::Linear,
            mipmaps: true,
            lod_bias,
        }
    }
}

impl Sampler {
    pub fn from_flags(flags: xc3_lib::mxmd::SamplerFlags, lod_bias: f32) -> Self {
        // TODO: Force clamp
        if flags.repeat_uvw_3d() {
            Self::repeat_uvw_3d(lod_bias)
        } else {
            Self {
                address_mode_u: address_mode(flags.repeat_u(), flags.mirror_u()),
                address_mode_v: address_mode(flags.repeat_v(), flags.mirror_v()),
                address_mode_w: AddressMode::ClampToEdge,
                mag_filter: filter_mode(flags.nearest()),
                min_filter: filter_mode(flags.nearest()),
                mip_filter: filter_mode(flags.nearest()),
                mipmaps: !flags.disable_mipmap_filter(),
                lod_bias,
            }
        }
    }

    pub fn to_flags(&self) -> xc3_lib::mxmd::SamplerFlags {
        xc3_lib::mxmd::SamplerFlags::new(
            self.address_mode_u == AddressMode::Repeat,
            self.address_mode_v == AddressMode::Repeat,
            self.address_mode_u == AddressMode::MirrorRepeat,
            self.address_mode_v == AddressMode::MirrorRepeat,
            // TODO: make filter a method to preserve values here?
            self.mag_filter == FilterMode::Nearest,
            false,
            !self.mipmaps,
            *self == Self::repeat_uvw_3d(self.lod_bias),
            false,
            Default::default(),
        )
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
        let sampler = Sampler {
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mip_filter: FilterMode::Linear,
            mipmaps: true,
            lod_bias: 0.0,
        };
        let flags = SamplerFlags::from(0x0);
        assert_eq!(sampler.to_flags(), flags);
        assert_eq!(Sampler::from_flags(flags, 0.0), sampler);
    }

    #[test]
    fn descriptor_0x3() {
        let sampler = Sampler {
            address_mode_u: AddressMode::Repeat,
            address_mode_v: AddressMode::Repeat,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mip_filter: FilterMode::Linear,
            mipmaps: true,
            lod_bias: 0.0,
        };
        let flags = SamplerFlags::from(0b_11);
        assert_eq!(sampler.to_flags(), flags);
        assert_eq!(Sampler::from_flags(flags, 0.0), sampler);
    }

    #[test]
    fn descriptor_0x6() {
        let sampler = Sampler {
            address_mode_u: AddressMode::MirrorRepeat,
            address_mode_v: AddressMode::Repeat,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mip_filter: FilterMode::Linear,
            mipmaps: true,
            lod_bias: 0.0,
        };
        let flags = SamplerFlags::from(0b_110);
        assert_eq!(sampler.to_flags(), flags);
        assert_eq!(Sampler::from_flags(flags, 0.0), sampler);
    }

    #[test]
    fn descriptor_0x12() {
        let sampler = Sampler {
            address_mode_u: AddressMode::MirrorRepeat,
            address_mode_v: AddressMode::MirrorRepeat,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mip_filter: FilterMode::Linear,
            mipmaps: true,
            lod_bias: 0.0,
        };
        let flags = SamplerFlags::from(0b_1100);
        assert_eq!(sampler.to_flags(), flags);
        assert_eq!(Sampler::from_flags(flags, 0.0), sampler);
    }

    #[test]
    fn descriptor_0x40() {
        let sampler = Sampler {
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mip_filter: FilterMode::Linear,
            mipmaps: false,
            lod_bias: 0.0,
        };
        let flags = SamplerFlags::from(0b_01000000);
        assert_eq!(sampler.to_flags(), flags);
        assert_eq!(Sampler::from_flags(flags, 0.0), sampler);
    }

    #[test]
    fn descriptor_0x50() {
        let sampler = Sampler {
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Nearest,
            min_filter: FilterMode::Nearest,
            mip_filter: FilterMode::Nearest,
            mipmaps: false,
            lod_bias: 0.0,
        };
        let flags = SamplerFlags::from(0b_01010000);
        assert_eq!(sampler.to_flags(), flags);
        assert_eq!(Sampler::from_flags(flags, 0.0), sampler);
    }

    #[test]
    fn descriptor_0x83() {
        let sampler = Sampler {
            address_mode_u: AddressMode::Repeat,
            address_mode_v: AddressMode::Repeat,
            address_mode_w: AddressMode::Repeat,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mip_filter: FilterMode::Linear,
            mipmaps: true,
            lod_bias: -0.5,
        };
        let flags = SamplerFlags::from(0b_10000011);
        assert_eq!(sampler.to_flags(), flags);
        assert_eq!(Sampler::from_flags(flags, -0.5), sampler);
    }
}
