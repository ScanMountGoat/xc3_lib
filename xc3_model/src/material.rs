use xc3_lib::mxmd::{Materials, ShaderUnkType, StateFlags};

use crate::shader_database::{Shader, Spch};

/// See [Material](xc3_lib::mxmd::Material) and [FoliageMaterial](xc3_lib::map::FoliageMaterial).
#[derive(Debug, Clone, PartialEq)]
pub struct Material {
    pub name: String,
    pub flags: StateFlags,
    pub textures: Vec<Texture>,

    pub alpha_test: Option<TextureAlphaTest>,

    // TODO: Also store parameters?
    /// Precomputed metadata from the decompiled shader source
    /// or [None] if the database does not contain this model.
    pub shader: Option<Shader>,

    // TODO: include with shader?
    pub unk_type: ShaderUnkType,
    pub parameters: MaterialParameters,
}

/// Information for alpha testing based on sampled texture values.
#[derive(Debug, Clone, PartialEq)]
pub struct TextureAlphaTest {
    /// The texture in [textures](struct.Material.html#structfield.textures) used for alpha testing.
    pub texture_index: usize,
    /// The RGBA channel to sample for the comparison.
    pub channel_index: usize,
    // TODO: alpha test ref value?
    pub ref_value: f32,
}

/// Values assigned to known shader uniforms or `None` if not present.
#[derive(Debug, Clone, PartialEq)]
pub struct MaterialParameters {
    pub mat_color: [f32; 4],
    pub alpha_test_ref: f32,
    // Assume each param type is used at most once.
    pub tex_matrix: Option<Vec<[f32; 16]>>,
    pub work_float4: Option<Vec<[f32; 4]>>,
    pub work_color: Option<Vec<[f32; 4]>>,
}

impl Default for MaterialParameters {
    fn default() -> Self {
        Self {
            mat_color: [1.0; 4],
            alpha_test_ref: 1.0,
            tex_matrix: None,
            work_float4: None,
            work_color: None,
        }
    }
}

/// Selects an [ImageTexture] and [Sampler].
#[derive(Debug, Clone, PartialEq)]
pub struct Texture {
    /// The index of the [ImageTexture] in [image_textures](struct.ModelRoot.html#structfield.image_textures).
    pub image_texture_index: usize,
    /// The index of the [Sampler] in [samplers](struct.ModelGroup.html#structfield.samplers).
    pub sampler_index: usize,
}

pub fn create_materials(materials: &Materials, spch: Option<&Spch>) -> Vec<Material> {
    materials
        .materials
        .iter()
        .map(|material| {
            // TODO: How to choose between the two fragment shaders?
            let program_index = material.shader_programs[0].program_index as usize;
            let shader = spch
                .and_then(|spch| spch.programs.get(program_index))
                .map(|program| &program.shaders[0])
                .cloned();

            let textures = material
                .textures
                .iter()
                .map(|texture| Texture {
                    image_texture_index: texture.texture_index as usize,
                    sampler_index: texture.sampler_index as usize,
                })
                .collect();

            let parameters = assign_parameters(materials, material);

            let alpha_test = find_alpha_test_texture(materials, material);

            Material {
                name: material.name.clone(),
                flags: material.state_flags,
                textures,
                alpha_test,
                shader,
                unk_type: material.shader_programs[0].unk_type,
                parameters,
            }
        })
        .collect()
}

fn find_alpha_test_texture(
    materials: &Materials,
    material: &xc3_lib::mxmd::Material,
) -> Option<TextureAlphaTest> {
    // Find the texture used for alpha testing in the shader.
    // TODO: investigate how this works in game.
    let alpha_texture = materials
        .alpha_test_textures
        .get(material.alpha_test_texture_index as usize)?;
    if material.flags.alpha_mask() {
        // TODO: Do some materials require separate textures in a separate pass?
        let texture_index = material
            .textures
            .iter()
            .position(|t| t.texture_index == alpha_texture.texture_index)?;

        // Some materials use the red channel of a dedicated mask instead of alpha.
        let channel_index = if material.flags.separate_mask() { 0 } else { 3 };

        Some(TextureAlphaTest {
            texture_index,
            channel_index,
            ref_value: 0.5,
        })
    } else {
        None
    }
}

// TODO: Some elements get set by values not in the floats array?
// TODO: How to test this?
// TODO: This doesn't work properly for all models?
fn assign_parameters(
    materials: &Materials,
    material: &xc3_lib::mxmd::Material,
) -> MaterialParameters {
    // TODO: Don't assume a single program info?
    let info = &materials.shader_programs[material.shader_programs[0].program_index as usize];
    let floats = &materials.floats;
    let start_index = material.floats_start_index;

    // TODO: alpha test ref?
    let mut parameters = MaterialParameters {
        mat_color: material.color,
        alpha_test_ref: 0.5,
        tex_matrix: None,
        work_float4: None,
        work_color: None,
    };

    // TODO: Not set properly for WorkFloat4 for Mio o1.z?
    // TODO: Some values need to be divided by 255?
    for param in &info.parameters {
        match param.param_type {
            xc3_lib::mxmd::ParamType::Unk0 => (),
            xc3_lib::mxmd::ParamType::TexMatrix => {
                parameters.tex_matrix = Some(read_param(param, floats, start_index));
            }
            xc3_lib::mxmd::ParamType::WorkFloat4 => {
                parameters.work_float4 = Some(read_param(param, floats, start_index));
            }
            xc3_lib::mxmd::ParamType::WorkColor => {
                parameters.work_color = Some(read_param(param, floats, start_index));
            }
            xc3_lib::mxmd::ParamType::Unk4 => (),
            xc3_lib::mxmd::ParamType::Unk5 => (),
            xc3_lib::mxmd::ParamType::Unk6 => (),
            xc3_lib::mxmd::ParamType::Unk7 => (),
            xc3_lib::mxmd::ParamType::Unk10 => (),
        }
    }

    parameters
}

fn read_param<const N: usize>(
    param: &xc3_lib::mxmd::MaterialParameter,
    floats: &[f32],
    start_index: u32,
) -> Vec<[f32; N]> {
    // Assume any parameter can be an array, so read a vec.
    // TODO: avoid unwrap.
    let start = param.floats_index_offset as usize + start_index as usize;
    floats[start..]
        .chunks_exact(N)
        .take(param.count as usize)
        .map(|v| v.try_into().unwrap())
        .collect()
}
