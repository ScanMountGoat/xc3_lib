use xc3_lib::spch::NvsdMetadata;

// TODO: A more reliable way to do replacement is to visit each identifier.
// Names should be replaced using a lookup table in a single pass.
// String replacement won't handle the case where names overlap.
// TODO: What is the performance cost of annotation?

pub fn annotate_fragment(glsl: String, metadata: &NvsdMetadata) -> String {
    let mut glsl = glsl;
    for sampler in &metadata.samplers {
        let handle = sampler.handle.handle * 2 + 8;
        let texture_name = format!("fp_t_tcb_{handle:X}");
        glsl = glsl.replace(&texture_name, &sampler.name);
    }

    annotate_buffers(&mut glsl, "fp", metadata);

    glsl
}

fn annotate_buffers(glsl: &mut String, prefix: &str, metadata: &NvsdMetadata) {
    // TODO: annotate constants from fp_v1 or vp_c1.
    // TODO: How to determine which constant elements are actually used?
    // TODO: annotate parameters similar to smush_materials based on offset
    // TODO: are all uniforms vec4 params?
    for buffer in &metadata.uniform_buffers {
        // TODO: why is this always off by 3?
        // TODO: Is there an fp_c2?
        let handle = buffer.handle.handle + 3;
        let buffer_name = format!("{prefix}_c{handle}");
        *glsl = glsl.replace(&buffer_name, &buffer.name);
    }

    for buffer in &metadata.storage_buffers {
        let handle = buffer.handle.handle;
        let buffer_name = format!("{prefix}_s{handle}");
        *glsl = glsl.replace(&buffer_name, &buffer.name);
    }
}

pub fn annotate_vertex(glsl: String, metadata: &NvsdMetadata) -> String {
    // TODO: Handle overlaps like in_attr1 and in_attr10 properly.
    let mut glsl = glsl;
    for attribute in &metadata.attributes {
        let attribute_name = format!("in_attr{}", attribute.location);
        glsl = glsl.replace(&attribute_name, &attribute.name);
    }

    annotate_buffers(&mut glsl, "vp", metadata);

    glsl
}

#[cfg(test)]
mod tests {
    use super::*;

    use indoc::indoc;
    use pretty_assertions::assert_eq;
    use xc3_lib::spch::{Handle, InputAttribute, Sampler, UniformBuffer, Visibility};

    #[test]
    fn annotate_ch01011013_shd0056_vertex() {
        let glsl = indoc! {"
            layout (binding = 9, std140) uniform _vp_c8
            {
                precise vec4 data[4096];
            } vp_c8;
            
            layout (binding = 4, std140) uniform _vp_c3
            {
                precise vec4 data[4096];
            } vp_c3;
            
            layout (binding = 5, std140) uniform _vp_c4
            {
                precise vec4 data[4096];
            } vp_c4;
            
            layout (binding = 6, std140) uniform _vp_c5
            {
                precise vec4 data[4096];
            } vp_c5;
            
            layout (binding = 0, std430) buffer _vp_s0
            {
                uint data[];
            } vp_s0;
            
            layout (binding = 1, std430) buffer _vp_s1
            {
                uint data[];
            } vp_s1;
            
            layout (binding = 0) uniform sampler2D vp_t_tcb_E;
            layout (location = 0) in vec4 in_attr0;
            layout (location = 1) in vec4 in_attr1;
            layout (location = 2) in vec4 in_attr2;
            layout (location = 3) in vec4 in_attr3;
            layout (location = 4) in vec4 in_attr4;
            layout (location = 5) in vec4 in_attr5;
        "};

        let metadata = NvsdMetadata {
            uniform_buffers: vec![
                UniformBuffer {
                    name: "U_CamoflageCalc".to_string(),
                    uniform_count: 1,
                    uniform_start_index: 0,
                    unk3: 672,
                    handle: Handle {
                        handle: 5,
                        visibility: Visibility::VertexFragment,
                    },
                    unk5: 224,
                },
                UniformBuffer {
                    name: "U_Mate".to_string(),
                    uniform_count: 3,
                    uniform_start_index: 1,
                    unk3: 676,
                    handle: Handle {
                        handle: 1,
                        visibility: Visibility::VertexFragment,
                    },
                    unk5: 96,
                },
                UniformBuffer {
                    name: "U_Mdl".to_string(),
                    uniform_count: 4,
                    uniform_start_index: 4,
                    unk3: 680,
                    handle: Handle {
                        handle: 2,
                        visibility: Visibility::Fragment,
                    },
                    unk5: 176,
                },
                UniformBuffer {
                    name: "U_RimBloomCalc".to_string(),
                    uniform_count: 1,
                    uniform_start_index: 8,
                    unk3: 682,
                    handle: Handle {
                        handle: 4,
                        visibility: Visibility::Fragment,
                    },
                    unk5: 32,
                },
                UniformBuffer {
                    name: "U_Static".to_string(),
                    uniform_count: 18,
                    uniform_start_index: 9,
                    unk3: 684,
                    handle: Handle {
                        handle: 0,
                        visibility: Visibility::VertexFragment,
                    },
                    unk5: 672,
                },
                UniformBuffer {
                    name: "U_VolTexCalc".to_string(),
                    uniform_count: 1,
                    uniform_start_index: 27,
                    unk3: 688,
                    handle: Handle {
                        handle: 3,
                        visibility: Visibility::Fragment,
                    },
                    unk5: 176,
                },
            ],
            storage_buffers: vec![
                UniformBuffer {
                    name: "U_Bone".to_string(),
                    uniform_count: 1,
                    uniform_start_index: 28,
                    unk3: 690,
                    handle: Handle {
                        handle: 0,
                        visibility: Visibility::Fragment,
                    },
                    unk5: 48,
                },
                UniformBuffer {
                    name: "U_OdB".to_string(),
                    uniform_count: 1,
                    uniform_start_index: 29,
                    unk3: 692,
                    handle: Handle {
                        handle: 1,
                        visibility: Visibility::Fragment,
                    },
                    unk5: 48,
                },
            ],
            attributes: vec![
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
            ..Default::default()
        };

        assert_eq!(
            indoc! {"
                layout (binding = 9, std140) uniform _U_CamoflageCalc
                {
                    precise vec4 data[4096];
                } U_CamoflageCalc;
                
                layout (binding = 4, std140) uniform _U_Static
                {
                    precise vec4 data[4096];
                } U_Static;
                
                layout (binding = 5, std140) uniform _U_Mate
                {
                    precise vec4 data[4096];
                } U_Mate;
                
                layout (binding = 6, std140) uniform _U_Mdl
                {
                    precise vec4 data[4096];
                } U_Mdl;
                
                layout (binding = 0, std430) buffer _U_Bone
                {
                    uint data[];
                } U_Bone;
                
                layout (binding = 1, std430) buffer _U_OdB
                {
                    uint data[];
                } U_OdB;

                layout (binding = 0) uniform sampler2D vp_t_tcb_E;
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
            layout (binding = 0) uniform sampler2D fp_t_tcb_8;
            layout (binding = 1) uniform sampler2D fp_t_tcb_A;
            layout (binding = 2) uniform sampler2D fp_t_tcb_C;
            layout (binding = 3) uniform sampler2D fp_t_tcb_E;
            layout (binding = 4) uniform sampler2D fp_t_tcb_10;
            layout (binding = 5) uniform sampler2D fp_t_tcb_12;
            layout (binding = 6) uniform sampler2D fp_t_tcb_14;
            layout (binding = 7) uniform sampler2D fp_t_tcb_16;
            layout (binding = 8) uniform sampler2D fp_t_tcb_18;
        "};

        let metadata = NvsdMetadata {
            samplers: vec![
                Sampler {
                    name: "gTResidentTex04".to_string(),
                    unk1: 576,
                    handle: Handle {
                        handle: 6,
                        visibility: Visibility::Fragment,
                    },
                    unk: 0,
                },
                Sampler {
                    name: "gTResidentTex05".to_string(),
                    unk1: 578,
                    handle: Handle {
                        handle: 7,
                        visibility: Visibility::Fragment,
                    },
                    unk: 0,
                },
                Sampler {
                    name: "gTResidentTex11".to_string(),
                    unk1: 580,
                    handle: Handle {
                        handle: 8,
                        visibility: Visibility::Fragment,
                    },
                    unk: 0,
                },
                Sampler {
                    name: "s0".to_string(),
                    unk1: 582,
                    handle: Handle {
                        handle: 0,
                        visibility: Visibility::Fragment,
                    },
                    unk: 0,
                },
                Sampler {
                    name: "s1".to_string(),
                    unk1: 584,
                    handle: Handle {
                        handle: 1,
                        visibility: Visibility::Fragment,
                    },
                    unk: 0,
                },
                Sampler {
                    name: "s2".to_string(),
                    unk1: 586,
                    handle: Handle {
                        handle: 2,
                        visibility: Visibility::Fragment,
                    },
                    unk: 0,
                },
                Sampler {
                    name: "s3".to_string(),
                    unk1: 588,
                    handle: Handle {
                        handle: 3,
                        visibility: Visibility::Fragment,
                    },
                    unk: 0,
                },
                Sampler {
                    name: "s4".to_string(),
                    unk1: 590,
                    handle: Handle {
                        handle: 4,
                        visibility: Visibility::Fragment,
                    },
                    unk: 0,
                },
                Sampler {
                    name: "s5".to_string(),
                    unk1: 592,
                    handle: Handle {
                        handle: 5,
                        visibility: Visibility::Fragment,
                    },
                    unk: 0,
                },
            ],
            ..Default::default()
        };

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
