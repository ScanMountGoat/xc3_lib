use std::path::Path;

use crate::texture::create_texture;
use xc3_lib::mibl::Mibl;
use xc3_model::ImageTexture;

// TODO: Make this part of xc3_model to also support gltf?
// TODO: Document the uniform name for each field.
// TODO: Does this work for each game?
// TODO: Make each field optional or just log and assign defaults?
/// Textures and resources from the game's `monolib/shader` folder.
pub struct MonolibShaderTextures {
    pub toon_grad: wgpu::Texture,
    pub eyepatch_col: wgpu::Texture,
    pub eyepatch_nrm: wgpu::Texture,
    pub eyepatch_ao: wgpu::Texture,
    pub eyepatch_mask: wgpu::Texture,
}

impl MonolibShaderTextures {
    pub fn from_file<P: AsRef<Path>>(device: &wgpu::Device, queue: &wgpu::Queue, path: P) -> Self {
        // TODO: Are the mappings the same for all 3 games?
        // TODO: Add an option to load defaults if no path is provided?
        let toon_grad = load_mibl(device, queue, path.as_ref(), "toon_grad.witex");
        let eyepatch_col = load_mibl(device, queue, path.as_ref(), "eyepatch_col.witex");
        let eyepatch_nrm = load_mibl(device, queue, path.as_ref(), "eyepatch_nrm.witex");
        let eyepatch_ao = load_mibl(device, queue, path.as_ref(), "eyepatch_ao.witex");
        let eyepatch_mask = load_mibl(device, queue, path.as_ref(), "eyepatch_mask.witex");

        Self {
            toon_grad,
            eyepatch_col,
            eyepatch_nrm,
            eyepatch_ao,
            eyepatch_mask,
        }
    }

    /// Find the texture corresponding to a `sampler_name` like `gTResidentTex44`.
    pub fn global_texture(&self, sampler_name: &str) -> Option<&wgpu::Texture> {
        match sampler_name {
            "gTResidentTex43" => Some(&self.eyepatch_ao),
            "gTResidentTex44" => Some(&self.eyepatch_col),
            "gTResidentTex45" => Some(&self.eyepatch_mask),
            "gTResidentTex46" => Some(&self.eyepatch_nrm),
            _ => None,
        }
    }
}

fn load_mibl(device: &wgpu::Device, queue: &wgpu::Queue, path: &Path, name: &str) -> wgpu::Texture {
    let mibl = Mibl::from_file(path.join(name)).unwrap();
    let image = ImageTexture::from_mibl(&mibl, None, None).unwrap();
    create_texture(device, queue, &image)
}
