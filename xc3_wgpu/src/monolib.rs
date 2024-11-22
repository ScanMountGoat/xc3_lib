use std::{collections::BTreeMap, path::Path};

use crate::texture::create_texture;

/// Texture resources from the game's `monolib/shader` folder.
pub struct MonolibShaderTextures {
    global_textures: BTreeMap<String, wgpu::Texture>,
}

impl MonolibShaderTextures {
    pub fn from_file<P: AsRef<Path>>(device: &wgpu::Device, queue: &wgpu::Queue, path: P) -> Self {
        let textures = xc3_model::monolib::ShaderTextures::from_folder(path.as_ref());

        let global_textures = textures
            .textures
            .keys()
            .filter_map(|name| {
                Some((
                    name.to_string(),
                    textures
                        .global_texture(name)
                        .map(|image| create_texture(device, queue, image))?,
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
