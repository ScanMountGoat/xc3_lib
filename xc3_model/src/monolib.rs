//! Utilities for loading textures from the `monolib/shader` folder.
use crate::ImageTexture;
use std::path::Path;
use xc3_lib::mibl::Mibl;

// TODO: Document this as a table instead with sampler, file?
// TODO: Store as a map from sampler name to image texture?
/// Textures and resources from the `monolib/shader` folder.
#[derive(Debug, PartialEq, Clone)]
pub struct ShaderTextures {
    /// `monolib/shader/toon_grad.witex`
    pub toon_grad: Option<ImageTexture>,

    /// `monolib/shader/toon_grad_night.witex`
    pub toon_grad_night: Option<ImageTexture>,

    /// `monolib/shader/eyepatch_col.witex`
    pub eyepatch_col: Option<ImageTexture>,

    /// `monolib/shader/eyepatch_nrm.witex`
    pub eyepatch_nrm: Option<ImageTexture>,

    /// `monolib/shader/eyepatch_ao.witex`
    pub eyepatch_ao: Option<ImageTexture>,

    /// `monolib/shader/eyepatch_mask.witex`
    pub eyepatch_mask: Option<ImageTexture>,

    /// `monolib/shader/hatching_a_ptnrm.witex`
    pub hatching_a_ptnrm: Option<ImageTexture>,

    /// `monolib/shader/dirt_col.witex`
    pub dirt_col: Option<ImageTexture>,

    /// `monolib/shader/a_armor_env.witex`
    pub a_armor_env: Option<ImageTexture>,

    /// `monolib/shader/hat_a_ptmsk.witex`
    pub hat_a_ptmsk: Option<ImageTexture>,

    /// `monolib/shader/dirtskin_col.witex`
    pub dirtskin_col: Option<ImageTexture>,

    /// `monolib/shader/kokuin_mskb.witex`
    pub kokuin_mskb: Option<ImageTexture>,
}

impl ShaderTextures {
    pub fn from_folder<P: AsRef<Path>>(path: P) -> Self {
        // TODO: Are the name mappings the same for all 3 games?
        Self {
            toon_grad: load_mibl(path.as_ref(), "toon_grad.witex"),
            toon_grad_night: load_mibl(path.as_ref(), "toon_grad_night.witex"),
            eyepatch_col: load_mibl(path.as_ref(), "eyepatch_col.witex"),
            eyepatch_nrm: load_mibl(path.as_ref(), "eyepatch_nrm.witex"),
            eyepatch_ao: load_mibl(path.as_ref(), "eyepatch_ao.witex"),
            eyepatch_mask: load_mibl(path.as_ref(), "eyepatch_mask.witex"),
            hatching_a_ptnrm: load_mibl(path.as_ref(), "hatching_a_ptnrm.witex"),
            dirt_col: load_mibl(path.as_ref(), "dirt_col.witex"),
            a_armor_env: load_mibl(path.as_ref(), "a_armor_env.witex"),
            hat_a_ptmsk: load_mibl(path.as_ref(), "hat_a_ptmsk.witex"),
            dirtskin_col: load_mibl(path.as_ref(), "dirtskin_col.witex"),
            kokuin_mskb: load_mibl(path.as_ref(), "kokuin_mskb.witex"),
        }
    }

    // TODO: Load defaults if texture is missing?
    /// Find the texture corresponding to a `sampler_name` like `gTResidentTex44`.
    pub fn global_texture(&self, sampler_name: &str) -> Option<&ImageTexture> {
        match sampler_name {
            "gTResidentTex02" => self.kokuin_mskb.as_ref(),
            "gTResidentTex04" => self.dirt_col.as_ref(),
            "gTResidentTex06" => self.dirtskin_col.as_ref(),
            "gTResidentTex08" => self.hat_a_ptmsk.as_ref(),
            "gTResidentTex09" => self.hatching_a_ptnrm.as_ref(),
            "gTResidentTex11" => self.a_armor_env.as_ref(),
            "gTResidentTex43" => self.eyepatch_ao.as_ref(),
            "gTResidentTex44" => self.eyepatch_col.as_ref(),
            "gTResidentTex45" => self.eyepatch_mask.as_ref(),
            "gTResidentTex46" => self.eyepatch_nrm.as_ref(),
            "gTToonGrad" => self.toon_grad.as_ref(),
            "gTToonDarkGrad" => self.toon_grad_night.as_ref(),
            _ => None,
        }
    }
}

fn load_mibl(path: &Path, name: &str) -> Option<ImageTexture> {
    let mibl = Mibl::from_file(path.join(name)).ok()?;
    Some(ImageTexture::from_mibl(&mibl, Some(name.to_string()), None).unwrap())
}
