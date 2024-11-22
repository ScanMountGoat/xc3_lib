use std::{collections::BTreeMap, path::Path};

use crate::texture::create_texture;

/// Textures and resources from the game's `monolib/shader` folder.
pub struct MonolibShaderTextures {
    global_textures: BTreeMap<String, wgpu::Texture>,
}

impl MonolibShaderTextures {
    pub fn from_file<P: AsRef<Path>>(device: &wgpu::Device, queue: &wgpu::Queue, path: P) -> Self {
        let textures = xc3_model::monolib::ShaderTextures::from_folder(path.as_ref());

        // TODO: How to avoid duplicating this list with xc3_model?
        let global_textures = [
            "gTResidentTex02",
            "gTResidentTex04",
            "gTResidentTex06",
            "gTResidentTex08",
            "gTResidentTex11",
            "gTResidentTex08",
            "gTResidentTex09",
            "gTResidentTex43",
            "gTResidentTex44",
            "gTResidentTex45",
            "gTResidentTex46",
            "gTToonGrad",
            "gTToonDarkGrad",
        ]
        .into_iter()
        .filter_map(|name| {
            Some((
                name.to_string(),
                textures
                    .global_texture(name)
                    .map(|image| create_texture(device, queue, &image))?,
            ))
        })
        .collect();

        Self { global_textures }
    }

    /// Find the texture corresponding to a `sampler_name` like `gTResidentTex44`.
    pub fn global_texture(&self, sampler_name: &str) -> Option<&wgpu::Texture> {
        self.global_textures.get(sampler_name)
    }
}
