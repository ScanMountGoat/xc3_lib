use xc3_lib::spch::NvsdMetadata;

// TODO: A more reliable way to do replacement is to visit each identifier.
// Names should be replaced using a lookup table in a single pass.
// String replacement won't handle the case where names overlap.

pub fn annotate_fragment(glsl: String, metadata: &NvsdMetadata) -> String {
    let mut glsl = glsl;
    for sampler in &metadata.samplers {
        let handle = (sampler.unk2 - 256) * 2 + 8;
        let texture_name = format!("fp_tex_tcb_{handle:X}");
        glsl = glsl.replace(&texture_name, &sampler.name);
    }

    glsl
}

pub fn annotate_vertex(glsl: String, metadata: &NvsdMetadata) -> String {
    // TODO: Handle overlaps like in_attr1 and in_attr10 properly.
    let mut glsl = glsl;
    for attribute in &metadata.attributes {
        let attribute_name = format!("in_attr{}", attribute.location);
        glsl = glsl.replace(&attribute_name, &attribute.name);
    }

    glsl
}

// TODO: How to annotate uniform buffer names?
// TODO: Are vertex and fragment uniform buffers the same?

#[cfg(test)]
mod tests {
    use super::*;

    use indoc::indoc;
    use xc3_lib::spch::{InputAttribute, Sampler};

    fn create_metadata(samplers: Vec<Sampler>, attributes: Vec<InputAttribute>) -> NvsdMetadata {
        NvsdMetadata {
            samplers,
            attributes,
            ..Default::default()
        }
    }

    #[test]
    fn annotate_ch01012013_shd0008_vertex() {
        // TODO: One test for all annotations but with an abbreviated main function?
        let glsl = indoc! {"
            layout (location = 0) in vec4 in_attr0;
            layout (location = 1) in vec4 in_attr1;
            layout (location = 2) in vec4 in_attr2;
            layout (location = 3) in vec4 in_attr3;
            layout (location = 4) in vec4 in_attr4;
            layout (location = 5) in vec4 in_attr5;
        "};

        let metadata = create_metadata(
            Vec::new(),
            vec![
                InputAttribute {
                    name: "nWgtIdx".to_string(),
                    location: 1,
                },
                InputAttribute {
                    name: "vColor".to_string(),
                    location: 3,
                },
                InputAttribute {
                    name: "vNormal".to_string(),
                    location: 4,
                },
                InputAttribute {
                    name: "vPos".to_string(),
                    location: 0,
                },
                InputAttribute {
                    name: "vTan".to_string(),
                    location: 5,
                },
                InputAttribute {
                    name: "vTex0".to_string(),
                    location: 2,
                },
            ],
        );
        assert_eq!(
            indoc! {"
            layout (location = 0) in vec4 vPos;
            layout (location = 1) in vec4 nWgtIdx;
            layout (location = 2) in vec4 vTex0;
            layout (location = 3) in vec4 vColor;
            layout (location = 4) in vec4 vNormal;
            layout (location = 5) in vec4 vTan;
            "},
            annotate_vertex(glsl.to_string(), &metadata)
        );
    }

    #[test]
    fn annotate_ch01012013_shd0008_fragment() {
        // TODO: One test for all annotations but with an abbreviated main function?
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

        let metadata = create_metadata(
            vec![
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
            ],
            Vec::new(),
        );
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
            annotate_fragment(glsl.to_string(), &metadata)
        );
    }
}
