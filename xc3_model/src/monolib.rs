//! Utilities for loading textures from the `monolib/shader` folder.
//!
//! # Supported Global Textures
//! | Sampler | Texture |
//! | --- | --- |
//! | gTResidentTex02 | monolib/shader/kokuin_mskb.witex |
//! | gTResidentTex04 | monolib/shader/dirt_col.witex |
//! | gTResidentTex06 | monolib/shader/dirtskin_col.witex |
//! | gTResidentTex08 | monolib/shader/hat_a_ptmsk.witex |
//! | gTResidentTex09 | monolib/shader/hatching_a_ptnrm.witex |
//! | gTResidentTex11 | monolib/shader/a_armor_env.witex |
//! | gTResidentTex43 | monolib/shader/eyepatch_ao.witex |
//! | gTResidentTex44 | monolib/shader/eyepatch_col.witex |
//! | gTResidentTex45 | monolib/shader/eyepatch_mask.witex |
//! | gTResidentTex46 | monolib/shader/eyepatch_nrm.witex |
//! | gTToonGrad | monolib/shader/toon_grad.witex |
//! | gTToonDarkGrad | monolib/shader/toon_grad_night.witex |
use crate::ImageTexture;
use std::{collections::BTreeMap, path::Path};
use xc3_lib::mibl::Mibl;

/// Textures and resources from the `monolib/shader` folder.
#[derive(Debug, PartialEq, Clone)]
pub struct ShaderTextures {
    /// The texture like `toon_grad.witex` for supported sampler names like `gTToonGrad`.
    /// Missing files will be set to `None`.
    pub textures: BTreeMap<&'static str, Option<ImageTexture>>,
}

impl ShaderTextures {
    pub fn from_folder<P: AsRef<Path>>(path: P) -> Self {
        // TODO: Are the name mappings the same for all 3 games?
        let path = path.as_ref();
        Self {
            textures: [
                ("gTResidentTex02", load_mibl(path, "kokuin_mskb")),
                ("gTResidentTex04", load_mibl(path, "dirt_col")),
                ("gTResidentTex06", load_mibl(path, "dirtskin_col")),
                ("gTResidentTex08", load_mibl(path, "hat_a_ptmsk")),
                ("gTResidentTex09", load_mibl(path, "hatching_a_ptnrm")),
                ("gTResidentTex11", load_mibl(path, "a_armor_env")),
                ("gTResidentTex43", load_mibl(path, "eyepatch_ao")),
                ("gTResidentTex44", load_mibl(path, "eyepatch_col")),
                ("gTResidentTex45", load_mibl(path, "eyepatch_mask")),
                ("gTResidentTex46", load_mibl(path, "eyepatch_nrm")),
                ("gTToonGrad", load_mibl(path, "toon_grad")),
                ("gTToonDarkGrad", load_mibl(path, "toon_grad_night")),
            ]
            .into(),
        }
    }

    // TODO: Load defaults if texture is missing?
    /// Find the texture corresponding to a `sampler_name` like `gTResidentTex44`.
    pub fn global_texture(&self, sampler_name: &str) -> Option<&ImageTexture> {
        self.textures.get(sampler_name)?.as_ref()
    }
}

fn load_mibl(path: &Path, name: &str) -> Option<ImageTexture> {
    let mibl = Mibl::from_file(path.join(name)).ok()?;
    Some(ImageTexture::from_mibl(&mibl, Some(name.to_string()), None).unwrap())
}
