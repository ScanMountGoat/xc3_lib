use glam::{vec4, Vec4};
use log::warn;
use smol_str::SmolStr;
use xc3_lib::mxmd::{
    MaterialFlags, MaterialRenderFlags, Materials, RenderPassType, StateFlags, Technique,
    TextureUsage,
};

use crate::{
    shader_database::{BufferDependency, ModelPrograms, ShaderProgram, TextureDependency},
    ImageTexture,
};

/// See [Material](xc3_lib::mxmd::Material) and [FoliageMaterial](xc3_lib::map::FoliageMaterial).
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Clone)]
pub struct Material {
    pub name: String,
    pub flags: MaterialFlags,
    pub render_flags: MaterialRenderFlags,
    pub state_flags: StateFlags,

    pub textures: Vec<Texture>,
    pub alpha_test: Option<TextureAlphaTest>,

    pub work_values: Vec<f32>,
    pub shader_vars: Vec<(u16, u16)>,
    pub work_callbacks: Vec<(u16, u16)>,

    // TODO: final byte controls reference?
    pub alpha_test_ref: [u8; 4],

    // TODO: group indices for animations?
    pub m_unks1_1: u32,
    pub m_unks1_2: u32,
    pub m_unks1_3: u32,
    pub m_unks1_4: u32,

    /// Precomputed metadata from the decompiled shader source
    /// used to assign G-Buffer outputs
    /// or [None] if the database does not contain this model.
    pub shader: Option<ShaderProgram>,

    // material technique
    pub technique_index: usize,
    pub pass_type: RenderPassType,

    // TODO: keep these as views over the work values?
    // TODO: is there another way to preserve the work value buffer?
    pub parameters: MaterialParameters,

    pub m_unks2_2: u16,
    pub m_unks3_1: u16,
}

/// Information for alpha testing based on sampled texture values.
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Clone)]
pub struct TextureAlphaTest {
    /// The texture in [textures](struct.Material.html#structfield.textures) used for alpha testing.
    pub texture_index: usize,
    /// The RGBA channel to sample for the comparison.
    pub channel_index: usize,
}

/// Values assigned to known shader uniforms or `None` if not present.
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Clone)]
pub struct MaterialParameters {
    pub mat_color: [f32; 4],
    pub alpha_test_ref: f32,
    // Assume each param type is used at most once.
    pub tex_matrix: Option<Vec<[f32; 8]>>, // TODO: mat2x4?
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

/// Selects an [ImageTexture] and [Sampler](crate::Sampler).
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Texture {
    /// The index of the [ImageTexture] in [image_textures](struct.ModelRoot.html#structfield.image_textures).
    pub image_texture_index: usize,
    /// The index of the [Sampler](crate::Sampler) in [samplers](struct.ModelGroup.html#structfield.samplers).
    pub sampler_index: usize,
}

pub fn create_materials(
    materials: &Materials,
    model_programs: Option<&ModelPrograms>,
) -> Vec<Material> {
    materials
        .materials
        .iter()
        .enumerate()
        .map(|(i, material)| {
            let shader = get_shader(material, model_programs);

            let textures = material
                .textures
                .iter()
                .map(|texture| Texture {
                    image_texture_index: texture.texture_index as usize,
                    sampler_index: texture.sampler_index as usize,
                })
                .collect();

            let alpha_test = find_alpha_test_texture(materials, material);

            // Assume the work value start indices are in ascending order.
            let work_value_start = material.work_value_start_index as usize;
            let work_value_end = materials
                .materials
                .get(i + 1)
                .map(|m| m.work_value_start_index as usize)
                .unwrap_or(materials.work_values.len());
            let work_values = materials.work_values[work_value_start..work_value_end].to_vec();

            let shader_var_start = material.shader_var_start_index as usize;
            let shader_var_end = shader_var_start + material.shader_var_count as usize;

            let callback_start = material.callback_start_index as usize;
            let callback_end = callback_start + material.callback_count as usize;

            // TODO: Error for invalid parameters?
            let parameters =
                assign_parameters(materials, material, &work_values).unwrap_or_default();

            Material {
                name: material.name.clone(),
                flags: material.flags,
                render_flags: material.render_flags,
                state_flags: material.state_flags,
                textures,
                alpha_test,
                alpha_test_ref: material.alpha_test_ref,
                shader,
                work_values,
                shader_vars: materials.shader_vars[shader_var_start..shader_var_end].to_vec(),
                work_callbacks: materials
                    .callbacks
                    .as_ref()
                    .map(|c| c.work_callbacks[callback_start..callback_end].to_vec())
                    .unwrap_or_default(),
                technique_index: material
                    .techniques
                    .first()
                    .map(|t| t.technique_index as usize)
                    .unwrap_or_default(),
                pass_type: material
                    .techniques
                    .first()
                    .map(|t| t.pass_type)
                    .unwrap_or(RenderPassType::Unk0),
                parameters,
                m_unks1_1: material.m_unks1_1,
                m_unks1_2: material.m_unks1_2,
                m_unks1_3: material.m_unks1_3,
                m_unks1_4: material.m_unks1_4,
                m_unks2_2: material.m_unks2[2],
                m_unks3_1: material.m_unks3[1],
            }
        })
        .collect()
}

fn get_shader(
    material: &xc3_lib::mxmd::Material,
    model_programs: Option<&ModelPrograms>,
) -> Option<ShaderProgram> {
    let program_index = material.techniques.first()?.technique_index as usize;
    model_programs?.programs.get(program_index).cloned()
}

fn get_technique<'a>(
    material: &xc3_lib::mxmd::Material,
    techniques: &'a [Technique],
) -> Option<&'a Technique> {
    // TODO: Don't assume a single technique?
    let index = material.techniques.first()?.technique_index as usize;
    techniques.get(index)
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
        })
    } else {
        None
    }
}

// TODO: Some elements get set by values not in the floats array?
// TODO: How to test this?
fn assign_parameters(
    materials: &Materials,
    material: &xc3_lib::mxmd::Material,
    work_values: &[f32],
) -> Option<MaterialParameters> {
    let mut work_values = work_values.to_vec();

    // Callbacks are applied directly to the work values.
    if let Some(callbacks) = &materials.callbacks {
        let start = material.callback_start_index as usize;
        if let Some(callbacks) = callbacks
            .work_callbacks
            .get(start..start + material.callback_count as usize)
        {
            // TODO: What do the remaining callback types do?
            for callback in callbacks {
                // (26, i) for dividing work value i value by 255?
                if callback.0 == 26 {
                    if let Some(value) = work_values.get_mut(callback.1 as usize) {
                        *value /= 255.0;
                    }
                }
            }
        }
    }

    // TODO: alpha test ref?
    let mut parameters = MaterialParameters {
        mat_color: material.color, // TODO: store with material.
        alpha_test_ref: 0.5,
        tex_matrix: None,
        work_float4: None,
        work_color: None,
    };

    if let Some(info) = get_technique(material, &materials.techniques) {
        for param in &info.parameters {
            match param.param_type {
                xc3_lib::mxmd::ParamType::Unk0 => (),
                xc3_lib::mxmd::ParamType::TexMatrix => {
                    parameters.tex_matrix = Some(read_param(param, &work_values));
                }
                xc3_lib::mxmd::ParamType::WorkFloat4 => {
                    parameters.work_float4 = Some(read_param(param, &work_values));
                }
                xc3_lib::mxmd::ParamType::WorkColor => {
                    parameters.work_color = Some(read_param(param, &work_values));
                }
                xc3_lib::mxmd::ParamType::Unk4 => (),
                xc3_lib::mxmd::ParamType::Unk5 => (),
                xc3_lib::mxmd::ParamType::Unk6 => (),
                xc3_lib::mxmd::ParamType::Unk7 => (),
                xc3_lib::mxmd::ParamType::Unk10 => (),
            }
        }
    }

    Some(parameters)
}

fn read_param<const N: usize>(
    param: &xc3_lib::mxmd::MaterialParameter,
    work_values: &[f32],
) -> Vec<[f32; N]> {
    // Assume any parameter can be an array, so read a vec.
    // TODO: avoid unwrap.
    work_values[param.work_value_index as usize..]
        .chunks_exact(N)
        .take(param.count as usize)
        .map(|v| v.try_into().unwrap())
        .collect()
}

// TODO: Add a mat_id method that checks o1.w and returns an enum?
// TODO: create get methods for naming the outputs?
/// Assignment information for the channels of each output.
/// This includes channels from textures, material parameters, or shader constants.
#[derive(Debug, Clone, PartialEq)]
pub struct OutputAssignments {
    pub assignments: [OutputAssignment; 6],
}

impl OutputAssignments {
    /// Calculate the material ID from a hardcoded shader constant if present.
    pub fn mat_id(&self) -> Option<u32> {
        if let Some(ChannelAssignment::Value(v)) = self.assignments[1].w {
            // TODO: Why is this sometimes 7?
            Some((v * 255.0 + 0.1) as u32 & 0x7)
        } else {
            None
        }
    }
}

// TODO: Add some sort of default?
#[derive(Debug, Clone, PartialEq, Default)]
pub struct OutputAssignment {
    pub x: Option<ChannelAssignment>,
    pub y: Option<ChannelAssignment>,
    pub z: Option<ChannelAssignment>,
    pub w: Option<ChannelAssignment>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ChannelAssignment {
    Textures(Vec<TextureAssignment>),
    Attribute { name: SmolStr, channel_index: usize },
    Value(f32),
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextureAssignment {
    // TODO: Include matrix transform or scale?
    // TODO: Always convert everything to a matrix?
    // TODO: how often is the matrix even used?
    pub name: SmolStr,
    pub channels: SmolStr,
    pub texcoord_name: Option<SmolStr>,
    pub texcoord_transforms: Option<(Vec4, Vec4)>,
}

// TODO: Test cases for this?
impl Material {
    // TODO: Store these values instead of making them a method?
    /// Get the texture or value assigned to each shader output texture and channel.
    /// Most model shaders write to the G-Buffer textures.
    ///
    /// If no shader is assigned from the database, assignments are inferred from the usage hints in `textures`.
    /// This heuristic works well for detecting color and normal maps but cannot detect temp texture channels
    /// or material parameter values like texture tiling.
    pub fn output_assignments(&self, textures: &[ImageTexture]) -> OutputAssignments {
        self.shader
            .as_ref()
            .map(|s| output_assignments(s, &self.parameters))
            .unwrap_or_else(|| {
                warn!(
                    "Inferring assignments from texture names and usage types for {:?} due to unrecognized shader",
                    self.name
                );
                self.infer_assignment_from_textures(textures)
            })
    }

    fn infer_assignment_from_textures(&self, textures: &[ImageTexture]) -> OutputAssignments {
        // No assignment data is available.
        // Guess reasonable defaults based on the texture names or types.
        let assignment = |i: Option<usize>, c: usize| {
            i.map(|i| {
                ChannelAssignment::Textures(vec![TextureAssignment {
                    name: format!("s{i}").into(),
                    channels: ["x", "y", "z", "w"][c].into(),
                    texcoord_name: None,
                    texcoord_transforms: None,
                }])
            })
        };

        let color_index = self.textures.iter().position(|t| {
            matches!(
                // TODO: Why does this index out of range for xc2 legacy mxmd?
                textures.get(t.image_texture_index).and_then(|t| t.usage),
                Some(
                    TextureUsage::Col
                        | TextureUsage::Col2
                        | TextureUsage::Col3
                        | TextureUsage::Col4
                )
            )
        });

        // This may only have two channels since BC5 is common.
        let normal_index = self.textures.iter().position(|t| {
            matches!(
                textures.get(t.image_texture_index).and_then(|t| t.usage),
                Some(TextureUsage::Nrm | TextureUsage::Nrm2)
            )
        });

        let spm_index = self.textures.iter().position(|t| {
            matches!(
                textures.get(t.image_texture_index).and_then(|t| t.name.as_ref()),
                Some(name) if name.ends_with("_SPM")
            )
        });

        OutputAssignments {
            assignments: [
                OutputAssignment {
                    x: assignment(color_index, 0),
                    y: assignment(color_index, 1),
                    z: assignment(color_index, 2),
                    w: assignment(color_index, 3),
                },
                OutputAssignment::default(),
                OutputAssignment {
                    x: assignment(normal_index, 0),
                    y: assignment(normal_index, 1),
                    z: None,
                    w: None,
                },
                OutputAssignment::default(),
                OutputAssignment::default(),
                OutputAssignment {
                    x: assignment(spm_index, 0),
                    y: assignment(spm_index, 1),
                    z: assignment(spm_index, 2),
                    w: None,
                },
            ],
        }
    }
}

fn output_assignments(
    shader: &ShaderProgram,
    parameters: &MaterialParameters,
) -> OutputAssignments {
    OutputAssignments {
        assignments: [0, 1, 2, 3, 4, 5].map(|i| output_assignment(shader, parameters, i)),
    }
}

fn output_assignment(
    shader: &ShaderProgram,
    parameters: &MaterialParameters,
    output_index: usize,
) -> OutputAssignment {
    OutputAssignment {
        x: channel_assignment(shader, parameters, output_index, 0),
        y: channel_assignment(shader, parameters, output_index, 1),
        z: channel_assignment(shader, parameters, output_index, 2),
        w: channel_assignment(shader, parameters, output_index, 3),
    }
}

fn channel_assignment(
    shader: &ShaderProgram,
    parameters: &MaterialParameters,
    output_index: usize,
    channel_index: usize,
) -> Option<ChannelAssignment> {
    // Prioritize direct assignments like parameters or constants.
    let channel = ['x', 'y', 'z', 'w'][channel_index];
    param_or_const(shader, parameters, output_index, channel_index)
        .map(ChannelAssignment::Value)
        .or_else(|| {
            shader.attribute(output_index, channel).map(|attribute| {
                // Attributes may have multiple accessed channels like normal maps.
                // First check if the current channel is used.
                // TODO: Does this always work as intended?
                let c = if attribute.channels.contains(channel) {
                    channel
                } else {
                    attribute.channels.chars().next().unwrap()
                };

                ChannelAssignment::Attribute {
                    name: attribute.name.clone(),
                    channel_index: "xyzw".find(c).unwrap(),
                }
            })
        })
        .or_else(|| {
            // TODO: How to deal with multiple textures used for color and normal outputs?
            let textures: Vec<_> = shader
                .textures(output_index, channel)
                .iter()
                .map(|texture| {
                    let texcoord_transforms = texcoord_transforms(texture, parameters);

                    // TODO: different attribute for U and V?
                    TextureAssignment {
                        name: texture.name.clone(),
                        channels: texture.channels.clone(),
                        texcoord_name: texture.texcoords.first().map(|t| t.name.clone()),
                        texcoord_transforms,
                    }
                })
                .collect();

            (!textures.is_empty()).then_some(ChannelAssignment::Textures(textures))
        })
}

fn texcoord_transforms(
    texture: &TextureDependency,
    parameters: &MaterialParameters,
) -> Option<(Vec4, Vec4)> {
    // Each texcoord component has its own params.
    // TODO: return a vector for everything.
    if let Some([u, v]) = texture.texcoords.get(..2) {
        let transform_u = texcoord_transform(u, parameters, 0)?;
        let transform_v = texcoord_transform(v, parameters, 1)?;
        Some((transform_u, transform_v))
    } else {
        None
    }
}

fn texcoord_transform(
    u: &crate::shader_database::TexCoord,
    parameters: &MaterialParameters,
    index: usize,
) -> Option<Vec4> {
    match u.params.as_ref()? {
        crate::shader_database::TexCoordParams::Scale(s) => {
            // Select and scale the appropriate component.
            let scale = extract_parameter(s, parameters)?;
            let mut transform = Vec4::ZERO;
            transform[index] = scale;
            Some(transform)
        }
        crate::shader_database::TexCoordParams::Matrix([x, y, z, w]) => Some(
            vec4(
                extract_parameter(x, parameters)?,
                extract_parameter(y, parameters)?,
                extract_parameter(z, parameters)?,
                extract_parameter(w, parameters)?,
            )
            .into(),
        ),
    }
}

// TODO: Tests for this?
fn param_or_const(
    shader: &ShaderProgram,
    parameters: &MaterialParameters,
    i: usize,
    c: usize,
) -> Option<f32> {
    let channel = ['x', 'y', 'z', 'w'][c];
    shader
        .buffer_parameter(i, channel)
        .and_then(|p| extract_parameter(p, parameters))
        .or_else(|| shader.float_constant(i, channel))
}

fn extract_parameter(p: &BufferDependency, parameters: &MaterialParameters) -> Option<f32> {
    // TODO: Handle multiple channels?
    // TODO: How to handle the case where the input has no channels?
    let c = "xyzw".find(p.channels.chars().next()?).unwrap();
    match (p.name.as_str(), p.field.as_str()) {
        ("U_Mate", "gWrkFl4") => Some(parameters.work_float4.as_ref()?.get(p.index)?[c]),
        ("U_Mate", "gWrkCol") => Some(parameters.work_color.as_ref()?.get(p.index)?[c]),
        _ => None,
    }
}
