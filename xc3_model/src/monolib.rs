//! Utilities for loading textures from the `monolib/shader` folder.
//!
//! # Supported Global Textures
//! | Sampler | monolib/shader Texture |
//! | --- | --- |
//! | gTResidentTex01 | kokuin_mska.witex |
//! | gTResidentTex02 | kokuin_mskb.witex |
//! | gTResidentTex03 | a_armor_env.witex |
//! | gTResidentTex04 | dirt_col.witex |
//! | gTResidentTex05 | dirt_mask.witex |
//! | gTResidentTex06 | dirtskin_col.witex |
//! | gTResidentTex07 | dirtskin_mask.witex |
//! | gTResidentTex08 | hat_a_ptmsk.witex |
//! | gTResidentTex09 | hatching_a_ptnrm.witex |
//! | gTResidentTex11 | k_metal_env.witex |
//! | gTResidentTex12 | k_visor_env.witex |
//! | gTResidentTex15 | mobeye_female_col.witex |
//! | gTResidentTex16 | mobeye_female_mask.witex |
//! | gTResidentTex17 | mobeye_male_col.witex |
//! | gTResidentTex18 | mobeye_male_mask.witex |
//! | gTResidentTex19 | mobhair_col.witex |
//! | gTResidentTex20 | firedial_a_amb.witex |
//! | gTResidentTex21 | firedial_a_col.witex |
//! | gTResidentTex22 | firedial_a_mtl.witex |
//! | gTResidentTex23 | firedial_a_nrm.witex |
//! | gTResidentTex24 | firedial_a_shy.witex |
//! | gTResidentTex25 | firedial_b_amb.witex |
//! | gTResidentTex26 | firedial_b_col.witex |
//! | gTResidentTex27 | firedial_b_mtl.witex |
//! | gTResidentTex28 | firedial_b_nrm.witex |
//! | gTResidentTex29 | firedial_b_shy.witex |
//! | gTResidentTex30 | firedial_c_amb.witex |
//! | gTResidentTex31 | firedial_c_col.witex |
//! | gTResidentTex32 | firedial_c_mtl.witex |
//! | gTResidentTex33 | firedial_c_nrm.witex |
//! | gTResidentTex43 | eyepatch_ao.witex |
//! | gTResidentTex44 | eyepatch_col.witex |
//! | gTResidentTex45 | eyepatch_mask.witex |
//! | gTResidentTex46 | eyepatch_nrm.witex |
//! | gTToonGrad | toon_grad.witex |
//! | gTToonDarkGrad | toon_grad_night.witex |
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
                ("gTResidentTex01", tex(path, "kokuin_mska.witex")),
                ("gTResidentTex02", tex(path, "kokuin_mskb.witex")),
                ("gTResidentTex03", tex(path, "a_armor_env.witex")),
                ("gTResidentTex04", tex(path, "dirt_col.witex")),
                ("gTResidentTex05", tex(path, "dirt_mask.witex")),
                ("gTResidentTex06", tex(path, "dirtskin_col.witex")),
                ("gTResidentTex07", tex(path, "dirtskin_mask.witex")),
                ("gTResidentTex08", tex(path, "hat_a_ptmsk.witex")),
                ("gTResidentTex09", tex(path, "hatching_a_ptnrm.witex")),
                ("gTResidentTex11", tex(path, "k_metal_env.witex")),
                ("gTResidentTex12", tex(path, "k_visor_env.witex")),
                ("gTResidentTex15", tex(path, "mobeye_female_col.witex")),
                ("gTResidentTex16", tex(path, "mobeye_female_mask.witex")),
                ("gTResidentTex17", tex(path, "mobeye_male_col.witex")),
                ("gTResidentTex18", tex(path, "mobeye_male_mask.witex")),
                ("gTResidentTex19", tex(path, "mobhair_col.witex")),
                ("gTResidentTex20", tex(path, "firedial_a_amb.witex")),
                ("gTResidentTex21", tex(path, "firedial_a_col.witex")),
                ("gTResidentTex22", tex(path, "firedial_a_mtl.witex")),
                ("gTResidentTex23", tex(path, "firedial_a_nrm.witex")),
                ("gTResidentTex24", tex(path, "firedial_a_shy.witex")),
                ("gTResidentTex25", tex(path, "firedial_b_amb.witex")),
                ("gTResidentTex26", tex(path, "firedial_b_col.witex")),
                ("gTResidentTex27", tex(path, "firedial_b_mtl.witex")),
                ("gTResidentTex28", tex(path, "firedial_b_nrm.witex")),
                ("gTResidentTex29", tex(path, "firedial_b_shy.witex")),
                ("gTResidentTex30", tex(path, "firedial_c_amb.witex")),
                ("gTResidentTex31", tex(path, "firedial_c_col.witex")),
                ("gTResidentTex32", tex(path, "firedial_c_mtl.witex")),
                ("gTResidentTex33", tex(path, "firedial_c_nrm.witex")),
                ("gTResidentTex43", tex(path, "eyepatch_ao.witex")),
                ("gTResidentTex44", tex(path, "eyepatch_col.witex")),
                ("gTResidentTex45", tex(path, "eyepatch_mask.witex")),
                ("gTResidentTex46", tex(path, "eyepatch_nrm.witex")),
                ("gTAmbBRDF", tex(path, "ambientbrdf.witex")),
                ("gTToonGrad", tex(path, "toon_grad.witex")),
                ("gTToonDarkGrad", tex(path, "toon_grad_night.witex")),
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

fn tex(path: &Path, name: &str) -> Option<ImageTexture> {
    let mibl = Mibl::from_file(path.join(name)).ok()?;
    Some(ImageTexture::from_mibl(&mibl, Some(name.to_string()), None).unwrap())
}
