use xc3_lib::spch::NvsdMetadata;

// TODO: A more reliable way to do replacement is to visit each identifier.
// Names should be replaced using a lookup table in a single pass.
// String replacement won't handle the case where names overlap.
// TODO: annotate both vertex and fragment shaders?
pub fn annotate_fragment(frag_glsl: &str, metadata: &NvsdMetadata) -> String {
    // TODO: Avoid initial clone?
    let mut frag_glsl = frag_glsl.to_string();
    for sampler in &metadata.samplers {
        let handle = (sampler.unk2 - 256) * 2 + 8;
        let texture_name = format!("fp_tex_tcb_{handle:X}");
        frag_glsl = frag_glsl.replace(&texture_name, &sampler.name);
    }

    frag_glsl
}

#[cfg(test)]
mod tests {
    use super::*;

    use indoc::indoc;
    use xc3_lib::spch::{Nvsd, Sampler};

    fn create_metadata(samplers: Vec<Sampler>) -> NvsdMetadata {
        // TODO: Derive default for these types?
        NvsdMetadata {
            unks2: Default::default(),
            unk_count1: Default::default(),
            unk_count2: Default::default(),
            buffers1: Default::default(),
            unk13: Default::default(),
            unk_count3: Default::default(),
            unk_count4: Default::default(),
            buffers2: Default::default(),
            unk15: Default::default(),
            unk_count6: Default::default(),
            samplers,
            unks2_1: Default::default(),
            attributes: Default::default(),
            uniforms: Default::default(),
            unks3: Default::default(),
            // TODO: We shouldn't even need to construct this?
            nvsd: Nvsd::default(),
        }
    }

    // TODO: One test for all annotations but with an abbreviated main function?
    #[test]
    fn annotate_ch01012013_shd0008_fragment() {
        let glsl = indoc! {"
            layout (binding = 0) uniform sampler2D fp_tex_tcb_8;
            layout (binding = 1) uniform sampler2D fp_tex_tcb_A;
            layout (binding = 2) uniform sampler2D fp_tex_tcb_C;
            layout (binding = 3) uniform sampler2D fp_tex_tcb_E;
            layout (binding = 4) uniform sampler2D fp_tex_tcb_10;
            layout (binding = 5) uniform sampler2D fp_tex_tcb_12;
            layout (binding = 6) uniform sampler2D fp_tex_tcb_14;
            layout (binding = 7) uniform sampler2D fp_tex_tcb_16;
            layout (binding = 8) uniform sampler2D fp_tex_tcb_18;
        "};

        let metadata = create_metadata(vec![
            Sampler {
                name: "gTResidentTex04".to_string(),
                unk1: 576,
                unk2: 262,
            },
            Sampler {
                name: "gTResidentTex05".to_string(),
                unk1: 578,
                unk2: 263,
            },
            Sampler {
                name: "gTResidentTex11".to_string(),
                unk1: 580,
                unk2: 264,
            },
            Sampler {
                name: "s0".to_string(),
                unk1: 582,
                unk2: 256,
            },
            Sampler {
                name: "s1".to_string(),
                unk1: 584,
                unk2: 257,
            },
            Sampler {
                name: "s2".to_string(),
                unk1: 586,
                unk2: 258,
            },
            Sampler {
                name: "s3".to_string(),
                unk1: 588,
                unk2: 259,
            },
            Sampler {
                name: "s4".to_string(),
                unk1: 590,
                unk2: 260,
            },
            Sampler {
                name: "s5".to_string(),
                unk1: 592,
                unk2: 261,
            },
        ]);
        assert_eq!(
            indoc! {"
                layout (binding = 0) uniform sampler2D s0;
                layout (binding = 1) uniform sampler2D s1;
                layout (binding = 2) uniform sampler2D s2;
                layout (binding = 3) uniform sampler2D s3;
                layout (binding = 4) uniform sampler2D s4;
                layout (binding = 5) uniform sampler2D s5;
                layout (binding = 6) uniform sampler2D gTResidentTex04;
                layout (binding = 7) uniform sampler2D gTResidentTex05;
                layout (binding = 8) uniform sampler2D gTResidentTex11;
            "},
            annotate_fragment(glsl, &metadata)
        );
    }
}
