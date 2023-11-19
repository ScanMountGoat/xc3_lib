use std::collections::HashMap;

use glsl_lang::{
    ast::{
        ArraySpecifierData, ArraySpecifierDimensionData, ArrayedIdentifierData, Block, ExprData,
        Identifier, Node, StructFieldSpecifierData, TranslationUnit, TypeSpecifierData,
        TypeSpecifierNonArrayData,
    },
    parse::DefaultParse,
    transpiler::glsl::{show_translation_unit, FormattingState},
    visitor::{HostMut, Visit, VisitorMut},
};
use xc3_lib::spch::Nvsd;

// TODO: A more reliable way to do replacement is to visit each identifier.
// Names should be replaced using a lookup table in a single pass.
// String replacement won't handle the case where names overlap.
// TODO: What is the performance cost of annotation?
const VEC4_SIZE: u32 = 16;

struct Annotator {
    replacements: HashMap<String, String>,
    struct_fields: HashMap<String, Vec<Field>>,
}

struct Field {
    name: String,
    ty: TypeSpecifierNonArrayData,
    array_length: Option<i32>,
}

impl VisitorMut for Annotator {
    fn visit_identifier(&mut self, ident: &mut Identifier) -> Visit {
        if let Some(name) = self.replacements.get(ident.as_str()) {
            ident.0 = name.into();
        }
        Visit::Children
    }

    fn visit_block(&mut self, block: &mut Block) -> Visit {
        // TODO: Only set fields based on some sort of map.
        block.fields = vec![
            field(&Field {
                name: "a".to_string(),
                ty: TypeSpecifierNonArrayData::Vec4,
                array_length: Some(5),
            }),
            field(&Field {
                name: "b".to_string(),
                ty: TypeSpecifierNonArrayData::Vec4,
                array_length: None,
            }),
        ];

        Visit::Children
    }
}

fn field(field: &Field) -> Node<StructFieldSpecifierData> {
    Node::new(
        StructFieldSpecifierData {
            qualifier: None,
            ty: Node::new(
                TypeSpecifierData {
                    ty: Node::new(field.ty.clone(), None),
                    array_specifier: None,
                },
                None,
            ),
            identifiers: vec![Node::new(
                ArrayedIdentifierData {
                    ident: Node::new(field.name.as_str().into(), None),
                    array_spec: field.array_length.map(|i| {
                        Node::new(
                            ArraySpecifierData {
                                dimensions: vec![Node::new(
                                    ArraySpecifierDimensionData::ExplicitlySized(Box::new(
                                        Node::new(ExprData::IntConst(i), None),
                                    )),
                                    None,
                                )],
                            },
                            None,
                        )
                    }),
                },
                None,
            )],
        },
        None,
    )
}

pub fn annotate_fragment(glsl: String, metadata: &Nvsd) -> String {
    let mut replacements = HashMap::new();
    let mut struct_fields = HashMap::new();

    annotate_samplers(&mut replacements, metadata);
    annotate_buffers(&mut replacements, "fp", metadata);

    let mut visitor = Annotator {
        replacements,
        struct_fields,
    };

    let mut translation_unit = TranslationUnit::parse(&glsl).unwrap();
    translation_unit.visit_mut(&mut visitor);

    let mut text = String::new();
    show_translation_unit(&mut text, &translation_unit, FormattingState::default()).unwrap();

    text
}

fn annotate_samplers(replacements: &mut HashMap<String, String>, metadata: &Nvsd) {
    if let Some(samplers) = &metadata.samplers {
        for sampler in samplers {
            let handle = sampler.handle.handle * 2 + 8;
            let texture_name = format!("fp_t_tcb_{handle:X}");
            replacements.insert(texture_name, sampler.name.clone());
        }
    }
}

pub fn annotate_vertex(glsl: String, metadata: &Nvsd) -> String {
    let mut replacements = HashMap::new();
    let mut struct_fields = HashMap::new();

    for attribute in &metadata.attributes {
        let attribute_name = format!("in_attr{}", attribute.location);
        replacements.insert(attribute_name, attribute.name.clone());
    }
    annotate_buffers(&mut replacements, "vp", metadata);

    let mut visitor = Annotator {
        replacements,
        struct_fields,
    };

    let mut translation_unit = TranslationUnit::parse(&glsl).unwrap();
    translation_unit.visit_mut(&mut visitor);

    let mut text = String::new();
    show_translation_unit(&mut text, &translation_unit, FormattingState::default()).unwrap();

    text
}

// TODO: Modify the uniform buffer datablocks?

fn annotate_buffers(replacements: &mut HashMap<String, String>, prefix: &str, metadata: &Nvsd) {
    // TODO: annotate constants from fp_v1 or vp_c1.
    // TODO: How to determine which constant elements are actually used?
    // TODO: are all uniforms vec4 params?
    // TODO: add initialization code so that annotated shaders still compile.
    if let Some(uniform_buffers) = &metadata.uniform_buffers {
        for buffer in uniform_buffers {
            // TODO: why is this always off by 3?
            // TODO: Is there an fp_c2?
            let handle = buffer.handle.handle + 3;
            replacements.insert(format!("_{prefix}_c{handle}"), buffer.name.clone());
            replacements.insert(format!("{prefix}_c{handle}"), format!("_{}", buffer.name));

            let start = buffer.uniform_start_index as usize;
            let count = buffer.uniform_count as usize;

            // Sort to make it easier to convert offsets to sizes.
            let mut uniforms = metadata.uniforms[start..start + count].to_vec();
            uniforms.sort_by_key(|u| u.buffer_offset);

            for (uniform_index, uniform) in uniforms.iter().enumerate() {
                let vec4_index = uniform.buffer_offset / VEC4_SIZE;
                if let Some(bracket_index) = uniform.name.find('[') {
                    // Handle array uniforms like "array[0]".
                    // The array has elements until the next uniform.
                    if let Some(array_length) = uniforms
                        .get(uniform_index + 1)
                        .map(|u| (u.buffer_offset - uniform.buffer_offset) / VEC4_SIZE)
                    {
                        // Annotate all elments from array[0] to array[length-1].
                        // This avoids unannotated entries in the gbuffer database.
                        for i in 0..array_length {
                            let pattern = format!("{}.data[{}]", buffer.name, vec4_index + i);
                            // Reindex the array starting from the base offset.
                            let uniform_name =
                                format!("{}_{}[{i}]", buffer.name, &uniform.name[..bracket_index]);
                            replacements.insert(pattern, uniform_name);
                        }
                    }
                } else {
                    // Convert "buffer.data[3].x" to "buffer_uniform.x".
                    let pattern = format!("{}.data[{vec4_index}]", buffer.name);
                    let uniform_name = format!("{}_{}", buffer.name, uniform.name);
                    replacements.insert(pattern, uniform_name);
                }
            }
        }
    }

    if let Some(storage_buffers) = &metadata.storage_buffers {
        for buffer in storage_buffers {
            let handle = buffer.handle.handle;
            replacements.insert(format!("_{prefix}_s{handle}"), buffer.name.clone());
            replacements.insert(format!("{prefix}_s{handle}"), format!("_{}", buffer.name));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use indoc::indoc;
    use pretty_assertions::assert_eq;
    use xc3_lib::spch::{Handle, InputAttribute, Sampler, Uniform, UniformBuffer, Visibility};

    fn metadata() -> Nvsd {
        Nvsd {
            uniform_buffers: Some(vec![
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
            ]),
            storage_buffers: Some(vec![
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
            ]),
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
            uniforms: vec![
                Uniform {
                    name: "gCamouflageCalcWork[0]".to_string(),
                    buffer_offset: 0,
                },
                Uniform {
                    name: "gTexMat".to_string(),
                    buffer_offset: 0,
                },
                Uniform {
                    name: "gWrkCol".to_string(),
                    buffer_offset: 80,
                },
                Uniform {
                    name: "gWrkFl4[0]".to_string(),
                    buffer_offset: 32,
                },
                Uniform {
                    name: "gMdlParm".to_string(),
                    buffer_offset: 160,
                },
                Uniform {
                    name: "gmWVP".to_string(),
                    buffer_offset: 0,
                },
                Uniform {
                    name: "gmWorld".to_string(),
                    buffer_offset: 64,
                },
                Uniform {
                    name: "gmWorldView".to_string(),
                    buffer_offset: 112,
                },
                Uniform {
                    name: "gRimBloomCalcWork[0]".to_string(),
                    buffer_offset: 0,
                },
                Uniform {
                    name: "gBilMat".to_string(),
                    buffer_offset: 224,
                },
                Uniform {
                    name: "gBilYJiku".to_string(),
                    buffer_offset: 272,
                },
                Uniform {
                    name: "gCDep".to_string(),
                    buffer_offset: 352,
                },
                Uniform {
                    name: "gDitTMAAVal".to_string(),
                    buffer_offset: 480,
                },
                Uniform {
                    name: "gDitVal".to_string(),
                    buffer_offset: 368,
                },
                Uniform {
                    name: "gEtcParm".to_string(),
                    buffer_offset: 320,
                },
                Uniform {
                    name: "gJitter".to_string(),
                    buffer_offset: 464,
                },
                Uniform {
                    name: "gLightShaft".to_string(),
                    buffer_offset: 624,
                },
                Uniform {
                    name: "gPreMat".to_string(),
                    buffer_offset: 384,
                },
                Uniform {
                    name: "gScreenSize".to_string(),
                    buffer_offset: 448,
                },
                Uniform {
                    name: "gViewYVec".to_string(),
                    buffer_offset: 336,
                },
                Uniform {
                    name: "gWetParam[0]".to_string(),
                    buffer_offset: 640,
                },
                Uniform {
                    name: "gmDiffPreMat".to_string(),
                    buffer_offset: 560,
                },
                Uniform {
                    name: "gmInvView".to_string(),
                    buffer_offset: 176,
                },
                Uniform {
                    name: "gmProj".to_string(),
                    buffer_offset: 48,
                },
                Uniform {
                    name: "gmProjNonJitter".to_string(),
                    buffer_offset: 496,
                },
                Uniform {
                    name: "gmView".to_string(),
                    buffer_offset: 0,
                },
                Uniform {
                    name: "gmViewProj".to_string(),
                    buffer_offset: 112,
                },
                Uniform {
                    name: "gVolTexCalcWork[0]".to_string(),
                    buffer_offset: 0,
                },
                Uniform {
                    name: "gmSkinMtx[0]".to_string(),
                    buffer_offset: 0,
                },
                Uniform {
                    name: "gmOldSkinMtx[0]".to_string(),
                    buffer_offset: 0,
                },
            ],
            samplers: Some(vec![
                Sampler {
                    name: "gTResidentTex04".to_string(),
                    unk1: 694,
                    handle: Handle {
                        handle: 6,
                        visibility: Visibility::Fragment,
                    },
                    unk: 0,
                },
                Sampler {
                    name: "gTResidentTex05".to_string(),
                    unk1: 696,
                    handle: Handle {
                        handle: 7,
                        visibility: Visibility::Fragment,
                    },
                    unk: 0,
                },
                Sampler {
                    name: "gTSpEffNoise1".to_string(),
                    unk1: 698,
                    handle: Handle {
                        handle: 5,
                        visibility: Visibility::Fragment,
                    },
                    unk: 0,
                },
                Sampler {
                    name: "gTSpEffVtxNoise1".to_string(),
                    unk1: 700,
                    handle: Handle {
                        handle: 3,
                        visibility: Visibility::Fragment,
                    },
                    unk: 0,
                },
                Sampler {
                    name: "s0".to_string(),
                    unk1: 702,
                    handle: Handle {
                        handle: 0,
                        visibility: Visibility::Fragment,
                    },
                    unk: 0,
                },
                Sampler {
                    name: "s1".to_string(),
                    unk1: 704,
                    handle: Handle {
                        handle: 1,
                        visibility: Visibility::Fragment,
                    },
                    unk: 0,
                },
                Sampler {
                    name: "s2".to_string(),
                    unk1: 706,
                    handle: Handle {
                        handle: 2,
                        visibility: Visibility::Fragment,
                    },
                    unk: 0,
                },
                Sampler {
                    name: "volTex0".to_string(),
                    unk1: 708,
                    handle: Handle {
                        handle: 4,
                        visibility: Visibility::Fragment,
                    },
                    unk: 0,
                },
            ]),
            ..Default::default()
        }
    }

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

        let metadata = metadata();

        assert_eq!(
            indoc! {"
                layout(binding = 9, std140) uniform U_CamoflageCalc {
                    precise vec4 data[4096];
                }_U_CamoflageCalc;
                layout(binding = 4, std140) uniform U_Static {
                    precise vec4 data[4096];
                }_U_Static;
                layout(binding = 5, std140) uniform U_Mate {
                    precise vec4 data[4096];
                }_U_Mate;
                layout(binding = 6, std140) uniform U_Mdl {
                    precise vec4 data[4096];
                }_U_Mdl;
                layout(binding = 0, std430) buffer U_Bone {
                    uint data[];
                }_U_Bone;
                layout(binding = 1, std430) buffer U_OdB {
                    uint data[];
                }_U_OdB;
                layout(binding = 0) uniform sampler2D vp_t_tcb_E;
                layout(location = 0) in vec4 vPos;
                layout(location = 1) in vec4 nWgtIdx;
                layout(location = 2) in vec4 vTex0;
                layout(location = 3) in vec4 vColor;
                layout(location = 4) in vec4 vNormal;
                layout(location = 5) in vec4 vTan;"
            },
            annotate_vertex(glsl.to_string(), &metadata)
        );
    }

    #[test]
    fn annotate_ch01011013_shd0056_fragment() {
        // Main function modified to test more indices.
        // TODO: Include uniform buffers.
        let glsl = indoc! {"
            layout (binding = 0) uniform sampler2D fp_t_tcb_C;
            layout (binding = 1) uniform sampler3D fp_t_tcb_10;
            layout (binding = 2) uniform sampler2D fp_t_tcb_A;
            layout (binding = 3) uniform sampler2D fp_t_tcb_16;
            layout (binding = 4) uniform sampler2D fp_t_tcb_14;
            layout (binding = 5) uniform sampler2D fp_t_tcb_8;
            layout (binding = 6) uniform sampler2D fp_t_tcb_12;

            void main() {
                out_attr0.x = fp_c4.data[2].x;
                out_attr0.y = fp_c4.data[3].y;
                out_attr0.z = fp_c4.data[4].z;
                out_attr0.w = temp_620;
                out_attr1.x = fp_c4.data[5].x;
                out_attr1.y = temp_623;
                out_attr1.z = 0.0;
                out_attr1.w = 0.00823529344;
            }
        "};

        let metadata = metadata();

        // TODO: Test declarations.
        // TODO: create a vec4[] to support array indexing syntax?
        assert_eq!(
            indoc! {"
                layout (binding = 0) uniform sampler2D s2;
                layout (binding = 1) uniform sampler3D volTex0;
                layout (binding = 2) uniform sampler2D s1;
                layout (binding = 3) uniform sampler2D gTResidentTex05;
                layout (binding = 4) uniform sampler2D gTResidentTex04;
                layout (binding = 5) uniform sampler2D s0;
                layout (binding = 6) uniform sampler2D gTSpEffNoise1;
                void main() {
                    out_attr0.x = _U_Mate_gWrkFl4[0].x;
                    out_attr0.y = _U_Mate_gWrkFl4[1].y;
                    out_attr0.z = _U_Mate_gWrkFl4[2].z;
                    out_attr0.w = temp_620;
                    out_attr1.x = _U_Mate_gWrkCol.x;
                    out_attr1.y = temp_623;
                    out_attr1.z = 0.0;
                    out_attr1.w = 0.00823529344;
                }
            "},
            annotate_fragment(glsl.to_string(), &metadata)
        );
    }
}
