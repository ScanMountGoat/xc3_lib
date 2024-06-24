use std::path::Path;

use crate::texture::create_texture;
use xc3_lib::mibl::Mibl;
use xc3_model::ImageTexture;

// TODO: Make this part of xc3_model to also support gltf?
/// Textures and resources from the game's `monolib/shader` folder.
pub struct MonolibShaderTextures {
    /// `monolib/shader/toon_grad.witex`
    pub toon_grad: Option<wgpu::Texture>,

    /// `monolib/shader/eyepatch_col.witex`
    pub eyepatch_col: Option<wgpu::Texture>,

    /// `monolib/shader/eyepatch_nrm.witex`
    pub eyepatch_nrm: Option<wgpu::Texture>,

    /// `monolib/shader/eyepatch_ao.witex`
    pub eyepatch_ao: Option<wgpu::Texture>,

    /// `monolib/shader/eyepatch_mask.witex`
    pub eyepatch_mask: Option<wgpu::Texture>,

    /// `monolib/shader/hatching_a_ptnrm.witex`
    pub hatching_a_ptrnm: Option<wgpu::Texture>,
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
        let hatching_a_ptrnm = load_mibl(device, queue, path.as_ref(), "hatching_a_ptnrm.witex");

        Self {
            toon_grad,
            eyepatch_col,
            eyepatch_nrm,
            eyepatch_ao,
            eyepatch_mask,
            hatching_a_ptrnm,
        }
    }

    /// Find the texture corresponding to a `sampler_name` like `gTResidentTex44`.
    pub fn global_texture(&self, sampler_name: &str) -> Option<&wgpu::Texture> {
        match sampler_name {
            "gTResidentTex09" => self.hatching_a_ptrnm.as_ref(),
            "gTResidentTex43" => self.eyepatch_ao.as_ref(),
            "gTResidentTex44" => self.eyepatch_col.as_ref(),
            "gTResidentTex45" => self.eyepatch_mask.as_ref(),
            "gTResidentTex46" => self.eyepatch_nrm.as_ref(),
            _ => None,
        }
    }
}

fn load_mibl(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    path: &Path,
    name: &str,
) -> Option<wgpu::Texture> {
    let mibl = Mibl::from_file(path.join(name)).ok()?;
    let image = ImageTexture::from_mibl(&mibl, None, None).unwrap();
    Some(create_texture(device, queue, &image))
}
