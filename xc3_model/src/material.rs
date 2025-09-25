use assignments::{OutputAssignments, infer_assignment_from_textures, output_assignments};
use log::warn;

pub use xc3_lib::mxmd::{
    BlendMode, ColorWriteMode, CullMode, DepthFunc, FurShellParams, MaterialFlags,
    MaterialRenderFlags, RenderPassType, StateFlags, StencilMode, StencilValue, TextureUsage,
    WorkCallback,
};

use crate::{
    ImageTexture, Sampler,
    shader_database::{
        BufferDependency, Dependency, Operation, OutputExpr, ProgramHash, ShaderDatabase,
        ShaderProgram,
    },
};

pub mod assignments;

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

// TODO: Is this even worth caching if it's only ever accessed by names?
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

    /// [xc3_lib::mxmd::ParamType::AlphaInfo]
    pub alpha_info: Option<Vec<[f32; 4]>>,

    /// [xc3_lib::mxmd::ParamType::DpRat]
    pub dp_rat: Option<Vec<[f32; 4]>>,

    /// [xc3_lib::mxmd::ParamType::DpRat]
    pub projection_tex_matrix: Option<Vec<[f32; 4]>>,

    /// [xc3_lib::mxmd::legacy::ParamType::MaterialAmbient]
    pub material_ambient: Option<Vec<[f32; 4]>>,

    /// [xc3_lib::mxmd::legacy::ParamType::MaterialSpecular]
    pub material_specular: Option<Vec<[f32; 4]>>,

    /// [xc3_lib::mxmd::legacy::ParamType::DtWork]
    pub dt_work: Option<Vec<[f32; 4]>>,

    /// [xc3_lib::mxmd::legacy::ParamType::MdlParam]
    pub mdl_param: Option<Vec<[f32; 4]>>,

    // TODO: Add missing xcx de parameters.
    /// Skin color param for some Xenoblade X DE models like L.
    pub ava_skin: Option<[f32; 4]>,
}

impl MaterialParameters {
    pub fn get_dependency(&self, p: &BufferDependency) -> Option<f32> {
        // TODO: camera parameters like U_Mdl.gmWorldView and U_Mdl.gmWVP?

        // TODO: How to handle the case where the input has no channels?
        let c = "xyzw".find(p.channel?).unwrap();
        let index = p.index.unwrap_or_default();
        let value = match (p.name.as_str(), p.field.as_str()) {
            // U_Mate uniform buffer from material parameters.
            ("U_Mate", "gMatCol") => self.material_color.get(c),
            ("U_Mate", "gWrkFl4") => self.work_float4.as_ref()?.get(index)?.get(c),
            ("U_Mate", "gWrkCol") => self.work_color.as_ref()?.get(index)?.get(c),
            ("U_Mate", "gTexMat") => self.tex_matrix.as_ref()?.get(index)?.get(c),
            ("U_Mate", "gAlInf") => self
                .alpha_info
                .as_ref()?
                .get(index)
                .unwrap_or(&[1.0, 0.999, 1.0, 1.0])
                .get(c),
            ("U_Mate", "gDpRat") => self
                .dp_rat
                .as_ref()?
                .get(index)
                .unwrap_or(&[1.0, 1.0, 1.0, 0.0])
                .get(c),
            ("U_Mate", "gProjTexMat") => self.projection_tex_matrix.as_ref()?.get(index)?.get(c),
            // U_Static uniform buffer values taken from XC3 in RenderDoc.
            // These appear to be constant across models.
            // TODO: Compare with other games.
            ("U_Static", "gEtcParm") => [37.68019, -0.00031, 1.376, 1.0].get(c),
            ("U_Static", "gCDep") => [-1.0, -0.2, 1.0, 2.2].get(c),
            ("U_Static", "gLightShaft") => [0.0; 4].get(c),
            // U_Toon2 uniform buffer values taken from XC3 in RenderDoc.
            // These appear to be constant across models.
            // TODO: Compare with other games.
            ("U_Toon2", "gToonParam") => [
                [0.00, 0.15, 0.23873, 0.02],
                [0.00, 0.00, 0.00, 0.46],
                [-0.64923, 0.40759, 0.64216, 0.50],
                [0.30, 2.00, 0.00, 0.00],
            ]
            .get(index)?
            .get(c),
            // Xenoblade X DE
            // TODO: Some materials have no work values but still have values set?
            // TODO: these default values aren't always the same?
            ("U_Mate", "gMatAmb") => self
                .material_ambient
                .as_ref()?
                .get(index)
                .unwrap_or(&[1.0; 4])
                .get(c),
            ("U_Mate", "gMatSpec") => self
                .material_specular
                .as_ref()?
                .get(index)
                .unwrap_or(&[0.0, 0.0, 0.0, 0.1])
                .get(c),
            ("U_Mate", "gDTWrk") => self.dt_work.as_ref()?.get(index)?.get(c),
            ("U_Mate", "gMdlParm") => self.mdl_param.as_ref()?.get(index)?.get(c),
            ("U_CHR", "gAvaSkin") => self.ava_skin.as_ref()?.get(c),
            // TODO: initialized somewhere else?
            ("U_CHR", "gAvaHair") => [
                [0.40392, 0.24314, 0.16078, 1.14844],
                [0.40392, 0.24314, 0.16078, 0.1875],
            ]
            .get(index)?
            .get(c),
            // TODO: Is it worth using in game values for these?
            ("U_Static", "gLgtPreDir") => [[0.0, 0.0, 1.0, 1.0]; 2].get(index)?.get(c),
            ("U_Static", "gLgtPreCol") => [[1.0; 4]; 2].get(index)?.get(c),
            ("U_Static", "gNewRimLgtPreDir") => [[0.0, 0.0, 1.0, 1.0]; 2].get(index)?.get(c),
            ("U_Static", "gNewRimLgtPreCol") => [[1.0; 4]; 2].get(index)?.get(c),
            ("U_Static", "gLgtPreSpe") => [[1.0; 4]; 2].get(index)?.get(c),
            _ => None,
        };
        if value.is_none() {
            warn!("Unable to assign parameter {p}");
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
    let mut exprs = program.exprs;
    let output_dependencies = if !is_single_output {
        program
            .output_dependencies
            .iter()
            .filter_map(|(k, v)| match k.as_str() {
                // Ambient Occlusion
                "o0.x" => {
                    // Undo the multiply by 0.5 used for XCX and XCX DE.
                    // This avoids needing to modify the actual database file.
                    let const_index = exprs.len();
                    exprs.push(OutputExpr::Value(Dependency::Float(2.0.into())));
                    let index = exprs.len();
                    exprs.push(OutputExpr::Func {
                        op: Operation::Mul,
                        args: vec![*v, const_index],
                    });
                    Some(("o2.z".into(), index))
                }
                // Color
                "o1.x" => Some(("o0.x".into(), *v)),
                "o1.y" => Some(("o0.y".into(), *v)),
                "o1.z" => Some(("o0.z".into(), *v)),
                "o1.w" => Some(("o0.w".into(), *v)),
                // The normal output has only XY channels.
                "o2.x" => Some(("o2.x".into(), *v)),
                "o2.y" => Some(("o2.y".into(), *v)),
                // Specular
                "o3.x" => Some(("o5.x".into(), *v)),
                "o3.y" => Some(("o5.y".into(), *v)),
                "o3.z" => Some(("o5.z".into(), *v)),
                "o3.w" => Some(("o5.w".into(), *v)),
                // Depth
                "o4.x" => Some(("o4.x".into(), *v)),
                "o4.y" => Some(("o4.y".into(), *v)),
                "o4.z" => Some(("o4.z".into(), *v)),
                // Glossiness
                "o4.w" => Some(("o1.y".into(), *v)),
                _ => None,
            })
            .collect()
    } else {
        // Some shaders only write to color and shouldn't be remapped.
        program.output_dependencies.clone()
    };

    Some(ShaderProgram {
        output_dependencies,
        exprs,
        ..program
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
        ..Default::default()
    };

    // TODO: Some parameters always have count 0 and aren't part of the work values?
    if let Some(technique) = get_technique(material, &materials.techniques) {
        for param in &technique.parameters {
            match param.param_type {
                xc3_lib::mxmd::ParamType::DpRat => {
                    parameters.dp_rat = Some(read_param(param, &work_values));
                }
                xc3_lib::mxmd::ParamType::TexMatrix => {
                    // TODO: Is there a better way of handling tex matrix counts?
                    let param = xc3_lib::mxmd::MaterialParameter {
                        count: param.count * 2,
                        ..param.clone()
                    };
                    parameters.tex_matrix = Some(read_param(&param, &work_values));
                }
                xc3_lib::mxmd::ParamType::WorkFloat4 => {
                    parameters.work_float4 = Some(read_param(param, &work_values));
                }
                xc3_lib::mxmd::ParamType::WorkColor => {
                    parameters.work_color = Some(read_param(param, &work_values));
                }
                xc3_lib::mxmd::ParamType::ProjectionTexMatrix => {
                    // TODO: Is there a better way of handling tex matrix counts?
                    let param = xc3_lib::mxmd::MaterialParameter {
                        count: param.count * 2,
                        ..param.clone()
                    };
                    parameters.projection_tex_matrix = Some(read_param(&param, &work_values));
                }
                xc3_lib::mxmd::ParamType::AlphaInfo => {
                    parameters.alpha_info = Some(read_param(param, &work_values));
                }
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
            36 => {
                // TODO: DpRat values are set from callbacks?
                // TODO: set value to previous value?
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
                .take(param.count as usize)
                .map(|v| {
                    // TODO: Just keep indices to reference values instead?
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

fn read_param_legacy<const N: usize>(
    param: &xc3_lib::mxmd::legacy::MaterialParameter,
    work_values: &[f32],
) -> Vec<[f32; N]> {
    // Assume any parameter can be an array, so read a vec.
    work_values
        .get(param.work_value_index as usize..)
        .map(|values| {
            values
                .chunks(N)
                .take(param.count as usize)
                .map(|v| {
                    // TODO: Just keep indices to reference values instead?
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
    work_values: &[f32],
) -> Option<MaterialParameters> {
    let mut parameters = MaterialParameters {
        material_color: material.color,
        ava_skin: materials.unks1_2_3.map(|v| [v[0], v[1], v[2], v[3]]),
        ..Default::default()
    };

    // TODO: Some parameters always have count 0 and aren't part of the work values?
    if let Some(technique) = get_technique_legacy(material, &materials.techniques) {
        for param in &technique.parameters {
            match param.param_type {
                xc3_lib::mxmd::legacy::ParamType::MaterialAmbient => {
                    parameters.material_ambient = Some(read_param_legacy(param, work_values));
                }
                xc3_lib::mxmd::legacy::ParamType::MaterialSpecular => {
                    parameters.material_specular = Some(read_param_legacy(param, work_values));
                }
                xc3_lib::mxmd::legacy::ParamType::DpRat => {
                    parameters.dp_rat = Some(read_param_legacy(param, work_values));
                }
                xc3_lib::mxmd::legacy::ParamType::TexMatrix => {
                    // TODO: Is there a better way of handling tex matrix counts?
                    let param = xc3_lib::mxmd::legacy::MaterialParameter {
                        count: param.count * 2,
                        ..param.clone()
                    };
                    parameters.tex_matrix = Some(read_param_legacy(&param, work_values));
                }
                xc3_lib::mxmd::legacy::ParamType::WorkFloat4 => {
                    parameters.work_float4 = Some(read_param_legacy(param, work_values));
                }
                xc3_lib::mxmd::legacy::ParamType::WorkColor => {
                    parameters.work_color = Some(read_param_legacy(param, work_values));
                }
                xc3_lib::mxmd::legacy::ParamType::ProjectionTexMatrix => {
                    // TODO: Is there a better way of handling tex matrix counts?
                    let param = xc3_lib::mxmd::legacy::MaterialParameter {
                        count: param.count * 2,
                        ..param.clone()
                    };
                    parameters.projection_tex_matrix = Some(read_param_legacy(&param, work_values));
                }
                xc3_lib::mxmd::legacy::ParamType::AlphaInfo => {
                    parameters.alpha_info = Some(read_param_legacy(param, work_values));
                }
                xc3_lib::mxmd::legacy::ParamType::MaterialColor => {}
                xc3_lib::mxmd::legacy::ParamType::DtWork => {
                    parameters.dt_work = Some(read_param_legacy(param, work_values));
                }
                xc3_lib::mxmd::legacy::ParamType::MdlParam => {
                    parameters.mdl_param = Some(read_param_legacy(param, work_values));
                }
            }
        }
    }

    Some(parameters)
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
                infer_assignment_from_textures(&self.textures, textures)
            })
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
