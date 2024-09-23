//! Utilities for loading textures from the `monolib/shader` folder.
use crate::ImageTexture;
use std::path::Path;
use xc3_lib::mibl::Mibl;

/// Textures and resources from the `monolib/shader` folder.
pub struct ShaderTextures {
    /// `monolib/shader/toon_grad.witex`
    pub toon_grad: Option<ImageTexture>,

    /// `monolib/shader/eyepatch_col.witex`
    pub eyepatch_col: Option<ImageTexture>,

    /// `monolib/shader/eyepatch_nrm.witex`
    pub eyepatch_nrm: Option<ImageTexture>,

    /// `monolib/shader/eyepatch_ao.witex`
    pub eyepatch_ao: Option<ImageTexture>,

    /// `monolib/shader/eyepatch_mask.witex`
    pub eyepatch_mask: Option<ImageTexture>,

    /// `monolib/shader/hatching_a_ptnrm.witex`
    pub hatching_a_ptrnm: Option<ImageTexture>,
}

impl ShaderTextures {
    pub fn from_folder<P: AsRef<Path>>(path: P) -> Self {
        // TODO: Are the mappings the same for all 3 games?
        let toon_grad = load_mibl(path.as_ref(), "toon_grad.witex");
        let eyepatch_col = load_mibl(path.as_ref(), "eyepatch_col.witex");
        let eyepatch_nrm = load_mibl(path.as_ref(), "eyepatch_nrm.witex");
        let eyepatch_ao = load_mibl(path.as_ref(), "eyepatch_ao.witex");
        let eyepatch_mask = load_mibl(path.as_ref(), "eyepatch_mask.witex");
        let hatching_a_ptrnm = load_mibl(path.as_ref(), "hatching_a_ptnrm.witex");

        Self {
            toon_grad,
            eyepatch_col,
            eyepatch_nrm,
            eyepatch_ao,
            eyepatch_mask,
            hatching_a_ptrnm,
        }
    }

    // TODO: Load defaults if texture is missing?
    /// Find the texture corresponding to a `sampler_name` like `gTResidentTex44`.
    pub fn global_texture(&self, sampler_name: &str) -> Option<&ImageTexture> {
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

fn load_mibl(path: &Path, name: &str) -> Option<ImageTexture> {
    let mibl = Mibl::from_file(path.join(name)).ok()?;
    Some(ImageTexture::from_mibl(&mibl, None, None).unwrap())
}
