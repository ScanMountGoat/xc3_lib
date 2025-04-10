use log::warn;
use ordered_float::OrderedFloat;
use smol_str::{SmolStr, ToSmolStr};

pub use xc3_lib::mxmd::{
    BlendMode, ColorWriteMode, CullMode, DepthFunc, FurShellParams, MaterialFlags,
    MaterialRenderFlags, RenderPassType, StateFlags, StencilMode, StencilValue, TextureUsage,
    WorkCallback,
};

use crate::{
    shader_database::{
        BufferDependency, Dependency, Layer, LayerBlendMode, LayerValue, ProgramHash,
        ShaderDatabase, ShaderProgram, TexCoordParams, TextureDependency,
    },
    ImageTexture, Sampler,
};

/// See [Material](xc3_lib::mxmd::Material) and [FoliageMaterial](xc3_lib::map::FoliageMaterial).
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Clone)]
pub struct Material {
    pub name: String,
    pub flags: MaterialFlags,
    pub render_flags: MaterialRenderFlags,
    pub state_flags: StateFlags,

    pub color: [f32; 4],

    pub textures: Vec<Texture>,
    pub alpha_test: Option<TextureAlphaTest>,

    pub work_values: Vec<f32>,
    pub shader_vars: Vec<(u16, u16)>,
    pub work_callbacks: Vec<WorkCallback>,

    // TODO: final byte controls reference?
    pub alpha_test_ref: f32,

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
    pub gbuffer_flags: u16,

    pub fur_params: Option<FurShellParams>,
}

/// Information for alpha testing based on sampled texture values.
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Clone)]
pub struct TextureAlphaTest {
    /// The texture in [textures](struct.Material.html#structfield.textures) used for alpha testing.
    pub texture_index: usize,
    /// The index of the [Sampler] in [samplers](struct.ModelGroup.html#structfield.samplers).
    pub sampler_index: usize,
    /// The RGBA channel to sample for the comparison.
    pub channel_index: usize,
}

/// Values assigned to known shader uniforms or `None` if not present.
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Clone, Default)]
pub struct MaterialParameters {
    // Assume each param type is used at most once.
    /// [xc3_lib::mxmd::ParamType::MaterialColor]
    pub material_color: [f32; 4],

    /// [xc3_lib::mxmd::ParamType::TexMatrix]
    pub tex_matrix: Option<Vec<[f32; 4]>>, // TODO: mat2x4?

    /// [xc3_lib::mxmd::ParamType::WorkFloat4]
    pub work_float4: Option<Vec<[f32; 4]>>,

    /// [xc3_lib::mxmd::ParamType::WorkColor]
    pub work_color: Option<Vec<[f32; 4]>>,

    /// Skin color param for some Xenoblade X DE models like L.
    pub ava_skin: Option<[f32; 4]>,
}

impl MaterialParameters {
    pub fn get_dependency(&self, p: &BufferDependency) -> Option<f32> {
        // TODO: How to handle the case where the input has no channels?
        let c = "xyzw".find(p.channel?).unwrap();
        let index = p.index.unwrap_or_default();
        let value = match (p.name.as_str(), p.field.as_str()) {
            ("U_Mate", "gMatCol") => self.material_color.get(c),
            ("U_Mate", "gWrkFl4") => self.work_float4.as_ref()?.get(index)?.get(c),
            ("U_Mate", "gWrkCol") => self.work_color.as_ref()?.get(index)?.get(c),
            ("U_Mate", "gTexMat") => self.tex_matrix.as_ref()?.get(index)?.get(c),
            // Xenoblade X DE
            ("U_CHR", "gAvaSkin") => self.ava_skin.as_ref()?.get(c),
            _ => None,
        };
        if value.is_none() {
            warn!(
                "Unable to assign parameter {}.{}{}{}",
                p.name,
                p.field,
                p.index.map(|i| format!("[{i}]")).unwrap_or_default(),
                p.channel.map(|c| format!(".{c}")).unwrap_or_default()
            );
        }

        value.copied()
    }
}

/// Selects an [ImageTexture] and [Sampler].
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Texture {
    /// The index of the [ImageTexture] in [image_textures](struct.ModelRoot.html#structfield.image_textures).
    pub image_texture_index: usize,
    /// The index of the [Sampler] in [samplers](struct.ModelGroup.html#structfield.samplers).
    pub sampler_index: usize,
}

pub(crate) fn create_materials(
    materials: &xc3_lib::mxmd::Materials,
    texture_indices: Option<&[u16]>,
    spch: &xc3_lib::spch::Spch,
    shader_database: Option<&ShaderDatabase>,
) -> Vec<Material> {
    materials
        .materials
        .iter()
        .enumerate()
        .map(|(i, material)| {
            let shader = get_shader(material, spch, shader_database);

            let textures = material
                .textures
                .iter()
                .map(|texture| {
                    // Legacy models can remap material texture indices.
                    Texture {
                        image_texture_index: texture_indices
                            .map(|indices| {
                                indices
                                    .iter()
                                    .position(|i| *i == texture.texture_index)
                                    .unwrap_or_default()
                            })
                            .unwrap_or(texture.texture_index as usize),
                        sampler_index: texture.sampler_index2 as usize,
                    }
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
            let work_values = materials
                .work_values
                .get(work_value_start..work_value_end)
                .unwrap_or_default()
                .to_vec();

            let shader_var_start = material.shader_var_start_index as usize;
            let shader_var_end = shader_var_start + material.shader_var_count as usize;

            let callback_start = material.callback_start_index as usize;
            let callback_end = callback_start + material.callback_count as usize;

            // TODO: Error for invalid parameters?
            let parameters =
                assign_parameters(materials, material, &work_values).unwrap_or_default();

            // TODO: It's redundant to make this optional and store the fur flag.
            let fur_params = materials.fur_shells.as_ref().and_then(|fur| {
                let param_index = *fur.material_param_indices.get(i)? as usize;
                let params = fur.params.get(param_index).cloned()?;
                material.flags.fur_shells().then_some(params)
            });

            Material {
                name: material.name.clone(),
                flags: material.flags,
                render_flags: material.render_flags,
                state_flags: material.state_flags,
                color: material.color,
                textures,
                alpha_test,
                alpha_test_ref: material.alpha_test_ref,
                shader,
                work_values,
                shader_vars: materials
                    .shader_vars
                    .get(shader_var_start..shader_var_end)
                    .unwrap_or_default()
                    .to_vec(),
                work_callbacks: materials
                    .callbacks
                    .as_ref()
                    .and_then(|c| c.work_callbacks.get(callback_start..callback_end))
                    .unwrap_or_default()
                    .to_vec(),
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
                gbuffer_flags: material.gbuffer_flags,
                fur_params,
            }
        })
        .collect()
}

pub(crate) fn create_materials_samplers_legacy<S>(
    materials: &xc3_lib::mxmd::legacy::Materials,
    texture_indices: &[u16],
    shaders: Option<&S>,
    shader_database: Option<&ShaderDatabase>,
) -> (Vec<Material>, Vec<Sampler>)
where
    S: GetProgramHash,
{
    let mut samplers = Vec::new();

    let materials = materials
        .materials
        .iter()
        .enumerate()
        .map(|(i, m)| {
            // Assume the work value start indices are in ascending order.
            let work_value_start = m.work_value_start_index as usize;
            let work_value_end = materials
                .materials
                .get(i + 1)
                .map(|m| m.work_value_start_index as usize)
                .unwrap_or(materials.work_values.len());
            let work_values = materials
                .work_values
                .get(work_value_start..work_value_end)
                .unwrap_or_default()
                .to_vec();

            let shader_var_start = m.shader_var_start_index as usize;
            let shader_var_end = shader_var_start + m.shader_var_count as usize;

            let alpha_test = find_alpha_test_texture_legacy(materials, m, &mut samplers);

            // TODO: Error for invalid parameters?
            let parameters =
                assign_parameters_legacy(materials, m, &work_values).unwrap_or_default();

            Material {
                name: m.name.clone(),
                flags: MaterialFlags::from(0u32),
                render_flags: MaterialRenderFlags::from(0u32),
                state_flags: m.state_flags,
                color: m.color,
                textures: m
                    .textures
                    .iter()
                    .map(|t| {
                        // Texture indices are remapped by some models like chr_np/np025301.camdo.
                        Texture {
                            image_texture_index: texture_indices
                                .iter()
                                .position(|i| *i == t.texture_index)
                                .unwrap_or_default(),
                            sampler_index: get_sampler_index(&mut samplers, t.sampler_flags),
                        }
                    })
                    .collect(),
                alpha_test,
                alpha_test_ref: 0.5,
                shader: get_shader_legacy(m, shaders, shader_database),
                technique_index: m
                    .techniques
                    .last()
                    .map(|t| t.technique_index as usize)
                    .unwrap_or_default(),
                pass_type: match m.techniques.last().map(|t| t.unk1) {
                    Some(xc3_lib::mxmd::legacy::UnkPassType::Unk0) => RenderPassType::Unk0,
                    Some(xc3_lib::mxmd::legacy::UnkPassType::Unk1) => RenderPassType::Unk1,
                    // TODO: How to handle these variants?
                    Some(xc3_lib::mxmd::legacy::UnkPassType::Unk2) => RenderPassType::Unk0,
                    Some(xc3_lib::mxmd::legacy::UnkPassType::Unk3) => RenderPassType::Unk0,
                    Some(xc3_lib::mxmd::legacy::UnkPassType::Unk5) => RenderPassType::Unk0,
                    Some(xc3_lib::mxmd::legacy::UnkPassType::Unk8) => RenderPassType::Unk0,
                    None => RenderPassType::Unk0,
                },
                parameters,
                work_values,
                shader_vars: materials
                    .shader_vars
                    .get(shader_var_start..shader_var_end)
                    .unwrap_or_default()
                    .to_vec(),
                work_callbacks: Vec::new(),
                m_unks1_1: 0,
                m_unks1_2: 0,
                m_unks1_3: 0,
                m_unks1_4: 0,
                m_unks2_2: 0,
                gbuffer_flags: 0,
                fur_params: None,
            }
        })
        .collect();

    (materials, samplers)
}

fn get_sampler_index(samplers: &mut Vec<Sampler>, flags: xc3_lib::mxmd::SamplerFlags) -> usize {
    // Legacy samplers aren't indexed, so create indices here.
    let sampler = Sampler::from(flags);
    samplers
        .iter()
        .position(|s| s == &sampler)
        .unwrap_or_else(|| {
            let index = samplers.len();
            samplers.push(sampler);
            index
        })
}

fn get_shader(
    material: &xc3_lib::mxmd::Material,
    spch: &xc3_lib::spch::Spch,
    shader_database: Option<&ShaderDatabase>,
) -> Option<ShaderProgram> {
    // TODO: How to handle multiple techniques?
    let program_index = material.techniques.first()?.technique_index as usize;
    let hash = spch.get_program_hash(program_index)?;
    shader_database?.shader_program(hash)
}

fn get_shader_legacy<S: GetProgramHash>(
    material: &xc3_lib::mxmd::legacy::Material,
    shaders: Option<&S>,
    shader_database: Option<&ShaderDatabase>,
) -> Option<ShaderProgram> {
    // TODO: How to handle multiple techniques?
    let program_index = material.techniques.last()?.technique_index as usize;
    let hash = shaders?.get_program_hash(program_index)?;
    let program = shader_database?.shader_program(hash)?;

    let is_single_output = program
        .output_dependencies
        .keys()
        .all(|k| matches!(k.as_str(), "o0.x" | "o0.y" | "o0.z" | "o0.w"));

    // The texture outputs are different in Xenoblade X and Xenoblade X DE.
    // We handle this here to avoid needing to modify the database itself.
    // G-Buffer Textures:
    // 0: lighting (ao * ???, alpha is specular brdf?)
    // 1: color (alpha is emission?)
    // 2: normal (only xy)
    // 3: specular (alpha is spec?)
    // 4: depth (alpha is glossiness)
    let output_dependencies = if !is_single_output {
        program
            .output_dependencies
            .iter()
            .filter_map(|(k, v)| match k.as_str() {
                "o0.x" => Some(("o2.z".into(), v.clone())),
                "o1.x" => Some(("o0.x".into(), v.clone())),
                "o1.y" => Some(("o0.y".into(), v.clone())),
                "o1.z" => Some(("o0.z".into(), v.clone())),
                "o1.w" => Some(("o0.w".into(), v.clone())),
                // The normal output has only RG channels.
                "o2.x" => Some(("o2.x".into(), v.clone())),
                "o2.y" => Some(("o2.y".into(), v.clone())),
                "o3.x" => Some(("o5.x".into(), v.clone())),
                "o3.y" => Some(("o5.y".into(), v.clone())),
                "o3.z" => Some(("o5.z".into(), v.clone())),
                "o3.w" => Some(("o5.w".into(), v.clone())),
                "o4.x" => Some(("o4.x".into(), v.clone())),
                "o4.y" => Some(("o4.y".into(), v.clone())),
                "o4.z" => Some(("o4.z".into(), v.clone())),
                "o4.w" => Some(("o1.y".into(), v.clone())),
                _ => None,
            })
            .collect()
    } else {
        // Some shaders only write to color and shouldn't be remapped.
        program.output_dependencies.clone()
    };

    Some(ShaderProgram {
        output_dependencies,
        outline_width: None,
    })
}

pub trait GetProgramHash {
    fn get_program_hash(&self, program_index: usize) -> Option<ProgramHash>;
}

impl GetProgramHash for xc3_lib::spch::Spch {
    fn get_program_hash(&self, program_index: usize) -> Option<ProgramHash> {
        let slct = self
            .slct_offsets
            .get(program_index)?
            .read_slct(&self.slct_section)
            .ok()?;
        let binaries = self.program_data_vertex_fragment_binaries(&slct);
        let (p, v, f) = binaries.first()?;
        Some(ProgramHash::from_spch_program(p, v, f))
    }
}

impl GetProgramHash for xc3_lib::mxmd::legacy::Shaders {
    fn get_program_hash(&self, program_index: usize) -> Option<ProgramHash> {
        let shader = self.shaders.get(program_index)?;
        let mths = xc3_lib::mths::Mths::from_bytes(&shader.mths_data).ok()?;
        Some(ProgramHash::from_mths(&mths))
    }
}

fn get_technique<'a>(
    material: &xc3_lib::mxmd::Material,
    techniques: &'a [xc3_lib::mxmd::Technique],
) -> Option<&'a xc3_lib::mxmd::Technique> {
    // TODO: Don't assume a single technique?
    let index = material.techniques.first()?.technique_index as usize;
    techniques.get(index)
}

fn get_technique_legacy<'a>(
    material: &xc3_lib::mxmd::legacy::Material,
    techniques: &'a [xc3_lib::mxmd::legacy::Technique],
) -> Option<&'a xc3_lib::mxmd::legacy::Technique> {
    // TODO: Don't assume a single technique?
    let index = material.techniques.first()?.technique_index as usize;
    techniques.get(index)
}

fn find_alpha_test_texture(
    materials: &xc3_lib::mxmd::Materials,
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
            sampler_index: alpha_texture.sampler_index as usize,
            channel_index,
        })
    } else {
        None
    }
}

fn find_alpha_test_texture_legacy(
    materials: &xc3_lib::mxmd::legacy::Materials,
    material: &xc3_lib::mxmd::legacy::Material,
    samplers: &mut Vec<Sampler>,
) -> Option<TextureAlphaTest> {
    // Find the texture used for alpha testing in the shader.
    // TODO: investigate how this works in game.
    let alpha_texture = materials
        .alpha_test_textures
        .as_ref()?
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
            sampler_index: get_sampler_index(samplers, alpha_texture.sampler_flags),
            channel_index,
        })
    } else {
        None
    }
}

// TODO: Some elements get set by values not in the floats array?
// TODO: How to test this?
fn assign_parameters(
    materials: &xc3_lib::mxmd::Materials,
    material: &xc3_lib::mxmd::Material,
    work_values: &[f32],
) -> Option<MaterialParameters> {
    let callback_start = material.callback_start_index as usize;
    let callbacks = materials
        .callbacks
        .as_ref()?
        .work_callbacks
        .get(callback_start..callback_start + material.callback_count as usize)
        .unwrap_or_default();

    let work_values = apply_callbacks(work_values, callbacks);

    let mut parameters = MaterialParameters {
        material_color: material.color,
        tex_matrix: None,
        work_float4: None,
        work_color: None,
        ava_skin: None,
    };

    if let Some(technique) = get_technique(material, &materials.techniques) {
        for param in &technique.parameters {
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
                // TODO: Find the corresponding uniform name.
                xc3_lib::mxmd::ParamType::Unk4 => (),
                // TODO: index and count is always 0?
                // TODO: Do these take values from the work values?
                xc3_lib::mxmd::ParamType::AlphaInfo => (),
                xc3_lib::mxmd::ParamType::MaterialColor => (),
                xc3_lib::mxmd::ParamType::Unk7 => (),
                xc3_lib::mxmd::ParamType::ToonHeadMatrix => (),
            }
        }
    }

    Some(parameters)
}

fn apply_callbacks(work_values: &[f32], callbacks: &[WorkCallback]) -> Vec<f32> {
    let mut work_values = work_values.to_vec();

    // Callbacks are applied directly to the work values.
    // TODO: What do the remaining callback types do?
    for callback in callbacks {
        match callback.unk1 {
            25 => {
                // TODO: outline width?
            }
            26 => {
                // (26, i) for dividing work value i value by 255?
                // TODO: do these values always come in pairs?
                let start = callback.unk2 as usize;
                if start + 1 < work_values.len() {
                    // Shader parameters reference the first value in the pair.
                    // Only editing the second value in the pair seems to matter in game.
                    work_values[start] = work_values[start + 1] / 255.0;
                }
            }
            _ => (),
        }
    }
    work_values
}

fn read_param<const N: usize>(
    param: &xc3_lib::mxmd::MaterialParameter,
    work_values: &[f32],
) -> Vec<[f32; N]> {
    // Assume any parameter can be an array, so read a vec.
    work_values
        .get(param.work_value_index as usize..)
        .map(|values| {
            values
                .chunks(N)
                .map(|v| {
                    // TODO: Just keep indices to reference values instead?
                    // TODO: The param count field doesn't work here for Pyra ho_BL_TS2?
                    let mut output = [0.0; N];
                    for (o, v) in output.iter_mut().zip(v) {
                        *o = *v;
                    }
                    output
                })
                .collect()
        })
        .unwrap_or_default()
}

fn assign_parameters_legacy(
    materials: &xc3_lib::mxmd::legacy::Materials,
    material: &xc3_lib::mxmd::legacy::Material,
    _work_values: &[f32],
) -> Option<MaterialParameters> {
    let parameters = MaterialParameters {
        material_color: material.color,
        tex_matrix: None,
        work_float4: None,
        work_color: None,
        ava_skin: materials.unks1_2_3.map(|v| [v[0], v[1], v[2], v[3]]),
    };

    if let Some(_technique) = get_technique_legacy(material, &materials.techniques) {
        // TODO: parameters
    }

    Some(parameters)
}

// TODO: Add a mat_id method that checks o1.w and returns an enum?
// TODO: create get methods for naming the outputs?
/// Assignment information for the channels of each output.
/// This includes channels from textures, material parameters, or shader constants.
#[derive(Debug, Clone, PartialEq)]
pub struct OutputAssignments {
    pub assignments: [OutputAssignment; 6],
    /// The parameter multiplied by vertex alpha to determine outline width.
    pub outline_width: Option<ValueAssignment>,
}

impl OutputAssignments {
    /// Calculate the material ID from a hardcoded shader constant if present.
    pub fn mat_id(&self) -> Option<u32> {
        if let LayerAssignmentValue::Value(Some(ValueAssignment::Value(v))) = self.assignments[1].w
        {
            // TODO: Why is this sometimes 7?
            Some((v.0 * 255.0 + 0.1) as u32 & 0x7)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct OutputAssignment {
    /// The x values.
    pub x: LayerAssignmentValue,
    /// The y values.
    pub y: LayerAssignmentValue,
    /// The z values.
    pub z: LayerAssignmentValue,
    /// The w values.
    pub w: LayerAssignmentValue,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LayerAssignment {
    /// The layer value to blend with the previous layer.
    pub value: LayerAssignmentValue,
    /// The factor or blend weight for this layer.
    pub weight: LayerAssignmentValue,
    /// The blending operation for this layer.
    pub blend_mode: LayerBlendMode,
    pub is_fresnel: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum LayerAssignmentValue {
    Value(Option<ValueAssignment>),
    Layers(Vec<LayerAssignment>),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ValueAssignment {
    Texture(TextureAssignment),
    Attribute { name: SmolStr, channel_index: usize },
    Value(OrderedFloat<f32>),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TextureAssignment {
    // TODO: Include matrix transform or scale?
    // TODO: Always convert everything to a matrix?
    // TODO: how often is the matrix even used?
    pub name: SmolStr,
    pub channels: SmolStr,
    pub texcoord_name: Option<SmolStr>,
    pub texcoord_transforms: Option<([OrderedFloat<f32>; 4], [OrderedFloat<f32>; 4])>,
    pub parallax: Option<TexCoordParallax>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TexCoordParallax {
    pub mask_a: Box<ValueAssignment>,
    pub mask_b: Box<ValueAssignment>,
    pub ratio: OrderedFloat<f32>,
}

impl Default for LayerAssignmentValue {
    fn default() -> Self {
        Self::Value(None)
    }
}

impl ValueAssignment {
    pub fn from_dependency(
        d: &Dependency,
        parameters: &MaterialParameters,
        channel: char,
    ) -> Option<Self> {
        match d {
            Dependency::Constant(f) => Some(Self::Value(f.0.into())),
            Dependency::Buffer(b) => parameters.get_dependency(b).map(|f| Self::Value(f.into())),
            Dependency::Texture(texture) => {
                Some(Self::Texture(texture_assignment(texture, parameters)))
            }
            Dependency::Attribute(a) => {
                // Attributes may have multiple accessed channels.
                // First check if the current channel is used.
                // TODO: Does this always work as intended?
                let c = if a.channel == Some(channel) {
                    channel
                } else {
                    // TODO: avoid unwrap.
                    a.channel.unwrap()
                };

                Some(Self::Attribute {
                    name: a.name.clone(),
                    channel_index: "xyzw".find(c).unwrap(),
                })
            }
        }
    }
}

// TODO: Test cases for this?
impl Material {
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
            LayerAssignmentValue::Value(i.map(|i| {
                ValueAssignment::Texture(TextureAssignment {
                    name: format!("s{i}").into(),
                    channels: ["x", "y", "z", "w"][c].into(),
                    texcoord_name: None,
                    texcoord_transforms: None,
                    parallax: None,
                })
            }))
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
                    ..Default::default()
                },
                OutputAssignment::default(),
                OutputAssignment::default(),
                OutputAssignment {
                    x: assignment(spm_index, 0),
                    y: assignment(spm_index, 1),
                    z: assignment(spm_index, 2),
                    ..Default::default()
                },
            ],
            outline_width: None,
        }
    }
}

fn output_assignments(
    shader: &ShaderProgram,
    parameters: &MaterialParameters,
) -> OutputAssignments {
    OutputAssignments {
        assignments: [0, 1, 2, 3, 4, 5].map(|i| output_assignment(shader, parameters, i)),
        outline_width: shader
            .outline_width
            .as_ref()
            .and_then(|d| ValueAssignment::from_dependency(d, parameters, 'x')),
    }
}

fn output_assignment(
    shader: &ShaderProgram,
    parameters: &MaterialParameters,
    output_index: usize,
) -> OutputAssignment {
    OutputAssignment {
        x: output_channel_assignment(shader, parameters, output_index, 0),
        y: output_channel_assignment(shader, parameters, output_index, 1),
        z: output_channel_assignment(shader, parameters, output_index, 2),
        w: output_channel_assignment(shader, parameters, output_index, 3),
    }
}

fn output_channel_assignment(
    shader: &ShaderProgram,
    parameters: &MaterialParameters,
    output_index: usize,
    channel_index: usize,
) -> LayerAssignmentValue {
    let layers = layer_assignments(shader, parameters, output_index, channel_index);
    if layers.is_empty() {
        LayerAssignmentValue::Value(channel_assignment(
            shader,
            parameters,
            output_index,
            channel_index,
        ))
    } else {
        LayerAssignmentValue::Layers(layers)
    }
}

fn layer_assignments(
    shader: &ShaderProgram,
    parameters: &MaterialParameters,
    output_index: usize,
    channel_index: usize,
) -> Vec<LayerAssignment> {
    let channel = ['x', 'y', 'z', 'w'][channel_index];
    let output = format!("o{output_index}.{channel}");
    let layers = shader
        .output_dependencies
        .get(&SmolStr::from(output))
        .map(|d| d.layers.as_slice())
        .unwrap_or_default();

    layers
        .iter()
        .map(|l| layer_channel_assignment(parameters, channel, l))
        .collect()
}

fn layer_channel_assignment(
    parameters: &MaterialParameters,
    channel: char,
    l: &Layer,
) -> LayerAssignment {
    let value = layer_channel_assignment_value(parameters, channel, &l.value);
    let weight = layer_channel_assignment_value(parameters, channel, &l.ratio);

    LayerAssignment {
        value,
        weight,
        blend_mode: l.blend_mode,
        is_fresnel: l.is_fresnel,
    }
}

fn layer_channel_assignment_value(
    parameters: &MaterialParameters,
    channel: char,
    value: &LayerValue,
) -> LayerAssignmentValue {
    let value = match value {
        crate::shader_database::LayerValue::Value(d) => {
            LayerAssignmentValue::Value(ValueAssignment::from_dependency(d, parameters, channel))
        }
        crate::shader_database::LayerValue::Layers(layers) => LayerAssignmentValue::Layers(
            layers
                .iter()
                .map(|l| layer_channel_assignment(parameters, channel, l))
                .collect(),
        ),
    };
    value
}

fn channel_assignment(
    shader: &ShaderProgram,
    parameters: &MaterialParameters,
    output_index: usize,
    channel_index: usize,
) -> Option<ValueAssignment> {
    let channel = ['x', 'y', 'z', 'w'][channel_index];
    let output = format!("o{output_index}.{channel}");

    let original_dependencies = shader.output_dependencies.get(&SmolStr::from(output))?;
    let mut dependencies = original_dependencies.clone();

    if !dependencies.layers.is_empty() {
        // Match the correct layer order if present.
        dependencies.dependencies.sort_by_cached_key(|d| {
            dependencies
                .layers
                .iter()
                .position(|l| layer_dependency(l) == Some(d))
                .unwrap_or(usize::MAX)
        });
    } else if output_index == 0 {
        // Color maps typically assign s0 using RGB or a single channel.
        dependencies
            .dependencies
            .sort_by_cached_key(|d| sampler_index(d).unwrap_or(usize::MAX));
    } else if output_index == 2 && matches!(channel, 'x' | 'y') {
        // Normal maps are usually just XY BC5 textures.
        // Sort so that these textures are accessed first.
        dependencies.dependencies.sort_by_cached_key(|d| {
            let count = original_dependencies
                .dependencies
                .iter()
                .filter(|d2| sampler_name(d2) == sampler_name(d))
                .count();
            count != 2
        });
    } else {
        // Color maps typically assign s0 using RGB or a single channel.
        // Ignore single channel masks if an RGB input is present.
        // Ignore XY BC5 normal maps by placing them at the end.
        dependencies.dependencies.sort_by_cached_key(|d| {
            let count = original_dependencies
                .dependencies
                .iter()
                .filter(|d2| sampler_name(d2) == sampler_name(d))
                .count();
            (
                match count {
                    3 => 0,
                    1 => 1,
                    2 => u8::MAX,
                    _ => 2,
                },
                sampler_index(d).unwrap_or(usize::MAX),
            )
        });
    }

    let dependency = if output_index != 1 {
        // Some textures like color or normal maps may use multiple input channels.
        // First check if the current channel is used.
        dependencies
            .dependencies
            .iter()
            .find(|d| {
                channels(d)
                    .map(|channels| channels.contains(channel))
                    .unwrap_or_default()
            })
            .or_else(|| dependencies.dependencies.first())
    } else {
        dependencies.dependencies.first()
    }?;

    // If a parameter or attribute is assigned, it will likely be the only dependency.
    ValueAssignment::from_dependency(dependency, parameters, channel)
}

fn texture_assignment(
    texture: &TextureDependency,
    parameters: &MaterialParameters,
) -> TextureAssignment {
    let texcoord_transforms = texcoord_transforms(texture, parameters);

    // TODO: different attribute for U and V?
    TextureAssignment {
        name: texture.name.clone(),
        channels: texture.channel.map(|c| c.to_smolstr()).unwrap_or_default(),
        texcoord_name: texture.texcoords.first().map(|t| t.name.clone()),
        texcoord_transforms,
        parallax: match texture.texcoords.first().and_then(|t| t.params.as_ref()) {
            Some(TexCoordParams::Parallax {
                mask_a,
                mask_b,
                ratio,
            }) => {
                let mask_a = ValueAssignment::from_dependency(mask_a, parameters, 'x');
                let mask_b = ValueAssignment::from_dependency(mask_b, parameters, 'x');
                // TODO: Why are these sometimes none for xcx de?
                match (mask_a, mask_b) {
                    (Some(mask_a), Some(mask_b)) => Some(TexCoordParallax {
                        mask_a: Box::new(mask_a),
                        mask_b: Box::new(mask_b),
                        ratio: parameters.get_dependency(ratio).unwrap_or_default().into(),
                    }),
                    _ => None,
                }
            }
            _ => None,
        },
    }
}

// TODO: make these methods.
fn channels(d: &Dependency) -> Option<SmolStr> {
    match d {
        Dependency::Constant(_) => None,
        Dependency::Buffer(b) => Some(b.channel.map(|c| c.to_smolstr()).unwrap_or_default()),
        Dependency::Texture(t) => Some(t.channel.map(|c| c.to_smolstr()).unwrap_or_default()),
        Dependency::Attribute(a) => Some(a.channel.map(|c| c.to_smolstr()).unwrap_or_default()),
    }
}

fn sampler_index(d: &Dependency) -> Option<usize> {
    // Convert names like "s3" to index 3.
    // Material textures always use this naming convention in the shader.
    sampler_name(d).and_then(|n| n.strip_prefix('s')?.parse().ok())
}

fn sampler_name(d: &Dependency) -> Option<&SmolStr> {
    // Convert names like "s3" to index 3.
    // Material textures always use this naming convention in the shader.
    match d {
        Dependency::Texture(t) => Some(&t.name),
        _ => None,
    }
}

fn layer_dependency(l: &Layer) -> Option<&Dependency> {
    match &l.value {
        LayerValue::Value(d) => Some(d),
        LayerValue::Layers(layers) => layer_dependency(layers.first()?),
    }
}

fn texcoord_transforms(
    texture: &TextureDependency,
    parameters: &MaterialParameters,
) -> Option<([OrderedFloat<f32>; 4], [OrderedFloat<f32>; 4])> {
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
) -> Option<[OrderedFloat<f32>; 4]> {
    match u.params.as_ref()? {
        crate::shader_database::TexCoordParams::Scale(s) => {
            // Select and scale the appropriate component.
            let scale = parameters.get_dependency(s)?;
            let mut transform = [0.0.into(); 4];
            transform[index] = scale.into();
            Some(transform)
        }
        crate::shader_database::TexCoordParams::Matrix([x, y, z, w]) => Some([
            parameters.get_dependency(x)?.into(),
            parameters.get_dependency(y)?.into(),
            parameters.get_dependency(z)?.into(),
            parameters.get_dependency(w)?.into(),
        ]),
        crate::shader_database::TexCoordParams::Parallax { .. } => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_material_callbacks() {
        // xeno3/chr/ch/ch01011013.wimdo, "body" callbacks
        let work_values: Vec<_> = (0..24).map(|i| i as f32).collect();
        assert_eq!(
            vec![
                0.0,
                1.0,
                2.0,
                3.0,
                4.0,
                5.0,
                6.0,
                7.0,
                8.0,
                9.0,
                10.0,
                12.0 / 255.0,
                12.0,
                13.0,
                14.0,
                15.0,
                16.0,
                17.0,
                18.0,
                19.0,
                20.0,
                21.0,
                22.0,
                23.0
            ],
            apply_callbacks(
                &work_values,
                &[
                    WorkCallback { unk1: 26, unk2: 11 },
                    WorkCallback { unk1: 36, unk2: 15 }
                ]
            )
        );
    }
}
