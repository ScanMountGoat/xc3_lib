use std::fmt::Write;

use indexmap::IndexMap;
use indoc::formatdoc;
use log::{error, warn};
use smol_str::SmolStr;
use xc3_model::{
    material::{
        assignments::{Assignment, AssignmentValue, OutputAssignments},
        TextureAlphaTest,
    },
    shader_database::Operation,
    IndexMapExt,
};

use crate::pipeline::PipelineKey;

const OUT_VAR: &str = "RESULT";
const VAR_PREFIX: &str = "VAR";

// TODO: This needs to be 16 to support all in game shaders.
const MAX_SAMPLERS: usize = 15;

fn assignment_wgsl(
    value: &Assignment,
    name_to_index: &mut IndexMap<SmolStr, usize>,
) -> Option<String> {
    match value {
        Assignment::Func { op, args } => func_wgsl(op, args),
        Assignment::Value(v) => v
            .as_ref()
            .and_then(|v| channel_assignment_wgsl(v, name_to_index)),
    }
}

fn func_wgsl(op: &Operation, args: &[usize]) -> Option<String> {
    let arg0 = arg(args, 0);
    let arg1 = arg(args, 1);
    let arg2 = arg(args, 2);
    let arg3 = arg(args, 3);
    let arg4 = arg(args, 4);
    let arg5 = arg(args, 5);
    let arg6 = arg(args, 6);
    let arg7 = arg(args, 7);

    match op {
        Operation::Unk => None,
        Operation::Mix => Some(format!("mix({}, {}, {})", arg0?, arg1?, arg2?)),
        Operation::Mul => Some(format!("{} * {}", arg0?, arg1?)),
        Operation::Div => Some(format!("{} / {}", arg0?, arg1?)),
        Operation::Add => Some(format!("{} + {}", arg0?, arg1?)),
        Operation::AddNormalX => Some(format!(
            "add_normal_maps(create_normal_map({}, {}), create_normal_map({}, {}), {}).x * 0.5 + 0.5",
            arg0?, arg1?, arg2?, arg3?, arg4?
        )),
        Operation::AddNormalY => Some(format!(
            "add_normal_maps(create_normal_map({}, {}), create_normal_map({}, {}), {}).y * 0.5 + 0.5",
            arg0?, arg1?, arg2?, arg3?, arg4?
        )),
        Operation::OverlayRatio => Some(format!(
            "mix({0}, overlay_blend({0}, {1}), {2})",
            arg0?, arg1?, arg2?
        )),
        Operation::Overlay => Some(format!("overlay_blend({}, {})", arg0?, arg1?)),
        Operation::Overlay2 => Some(format!("overlay_blend2({}, {})", arg0?, arg1?)),
        Operation::Power => Some(format!("pow({}, {})", arg0?, arg1?)),
        Operation::Min => Some(format!("min({}, {})", arg0?, arg1?)),
        Operation::Max => Some(format!("max({}, {})", arg0?, arg1?)),
        Operation::Clamp => Some(format!("clamp({}, {}, {})", arg0?, arg1?, arg2?)),
        Operation::Sub => Some(format!("{} - {}", arg0?, arg1?)),
        Operation::Fma => Some(format!("{} * {} + {}", arg0?, arg1?, arg2?)),
        Operation::Abs => Some(format!("abs({})", arg0?)),
        Operation::Fresnel => Some(format!("fresnel_ratio({}, n_dot_v)", arg0?)),
        Operation::MulRatio => {
            Some(format!("mix({0}, {0} * {1}, {2})", arg0?, arg1?, arg2?))
        }
        Operation::Sqrt => Some(format!("sqrt({})", arg0?)),
        Operation::TexMatrix => Some(format!(
            "dot(vec4({}, {}, 0.0, 1.0), vec4({}, {}, {}, {}))",
            arg0?, arg1?, arg2?, arg3?, arg4?, arg5?
        )),
        Operation::TexParallaxX => {
            Some(format!("{} + uv_parallax(in, {}).x", arg0?, arg1?))
        }
        Operation::TexParallaxY => {
            Some(format!("{} + uv_parallax(in, {}).y", arg0?, arg1?))
        }
        Operation::ReflectX => Some(format!(
            "reflect(vec3({}, {}, {}), vec3({}, {}, {})).x",
            arg0?, arg1?, arg2?, arg3?, arg4?, arg5?
        )),
        Operation::ReflectY => Some(format!(
            "reflect(vec3({}, {}, {}), vec3({}, {}, {})).y",
            arg0?, arg1?, arg2?, arg3?, arg4?, arg5?
        )),
        Operation::ReflectZ => Some(format!(
            "reflect(vec3({}, {}, {}), vec3({}, {}, {})).z",
            arg0?, arg1?, arg2?, arg3?, arg4?, arg5?
        )),
        Operation::Floor => Some(format!("floor({})", arg0?)),
        Operation::Select => Some(format!("mix({}, {}, f32({}))", arg2?, arg1?, arg0?)),
        Operation::Equal => Some(format!("{} == {}", arg0?, arg1?)),
        Operation::NotEqual => Some(format!("{} != {}", arg0?, arg1?)),
        Operation::Less => Some(format!("{} < {}", arg0?, arg1?)),
        Operation::Greater => Some(format!("{} > {}", arg0?, arg1?)),
        Operation::LessEqual => Some(format!("{} <= {}", arg0?, arg1?)),
        Operation::GreaterEqual => Some(format!("{} >= {}", arg0?, arg1?)),
        Operation::Dot4 => Some(format!(
            "dot(vec4({}, {}, {}, {}), vec4({}, {}, {}, {}))",
            arg0?, arg1?, arg2?, arg3?, arg4?, arg5?, arg6?, arg7?
        )),
        Operation::NormalMapX => Some(format!(
            "apply_normal_map(create_normal_map({}, {}), in.tangent.xyz, bitangent.xyz, in.normal.xyz).x",
            arg0?, arg1?
        )),
        Operation::NormalMapY => Some(format!(
            "apply_normal_map(create_normal_map({}, {}), in.tangent.xyz, bitangent.xyz, in.normal.xyz).y",
            arg0?, arg1?
        )),
        Operation::NormalMapZ => Some(format!(
            "apply_normal_map(create_normal_map({}, {}), in.tangent.xyz, bitangent.xyz, in.normal.xyz).z",
            arg0?, arg1?
        )),
    }
}

fn arg(args: &[usize], i: usize) -> Option<String> {
    Some(format!("{VAR_PREFIX}{}", args.get(i)?))
}

pub fn create_model_shader(key: &PipelineKey) -> String {
    let mut source = include_str!("shader/model.wgsl").to_string();

    source = source.replace("// ASSIGN_VARS", &key.assignments_wgsl);

    for ((from, var), to) in [
        ("// ASSIGN_COLOR_GENERATED", "g_color"),
        ("// ASSIGN_ETC_GENERATED", "g_etc_buffer"),
        ("// ASSIGN_NORMAL_GENERATED", "g_normal"),
        ("// ASSIGN_G_LGT_COLOR_GENERATED", "g_lgt_color"),
    ]
    .iter()
    .zip(&key.output_layers_wgsl)
    {
        source = source.replace(from, &to.replace(OUT_VAR, var));
    }

    source = source.replace("// ALPHA_TEST_DISCARD_GENERATED", &key.alpha_test_wgsl);

    source = source.replace(
        "// ASSIGN_NORMAL_INTENSITY_GENERATED",
        &key.normal_intensity_wgsl.replace(OUT_VAR, "intensity"),
    );

    // This section is only used for wgsl_to_wgpu reachability analysis and can be removed.
    if let (Some(start), Some(end)) = (source.find("// REMOVE_BEGIN"), source.find("// REMOVE_END"))
    {
        source.replace_range(start..end, "");
    }

    source
}

pub fn generate_alpha_test_wgsl(
    alpha_test: &TextureAlphaTest,
    name_to_index: &mut IndexMap<SmolStr, usize>,
) -> String {
    let name: SmolStr = format!("s{}", alpha_test.texture_index).into();
    let i = name_to_index.entry_index(name.clone());

    if i < MAX_SAMPLERS {
        let c = ['x', 'y', 'z', 'w'][alpha_test.channel_index];

        // TODO: Detect the UV attribute to use with alpha testing.
        formatdoc! {"
            if textureSample(s{i}, alpha_test_sampler, tex0).{c} <= per_material.alpha_test_ref {{
                discard;
            }}
        "}
    } else {
        error!("Sampler index {i} exceeds supported max of {MAX_SAMPLERS}");
        String::new()
    }
}

// TODO: Struct to hold all the shader information?
pub fn generate_assignments_wgsl(
    assignments: &OutputAssignments,
    name_to_index: &mut IndexMap<SmolStr, usize>,
) -> String {
    let mut wgsl = String::new();

    // Write variables shared by all outputs.
    // Assume that values appear after values they depend on.
    for (i, value) in assignments.assignments.iter().enumerate() {
        let value_wgsl = assignment_wgsl(value, name_to_index);
        writeln!(
            wgsl,
            "let {VAR_PREFIX}{i} = {};",
            value_wgsl.unwrap_or("0.0".to_string())
        )
        .unwrap();
    }

    wgsl
}
pub fn generate_layering_wgsl(assignments: &OutputAssignments) -> Vec<String> {
    // Don't generate code for velocity or depth.
    assignments
        .output_assignments
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != 3 && *i != 4)
        .map(|(_, assignment)| {
            let mut wgsl = String::new();

            // Write any final assignments.
            if let Some(x) = assignment.x {
                writeln!(&mut wgsl, "{OUT_VAR}.x = {VAR_PREFIX}{x};").unwrap();
            }
            if let Some(y) = assignment.y {
                writeln!(&mut wgsl, "{OUT_VAR}.y = {VAR_PREFIX}{y};").unwrap();
            }
            if let Some(z) = assignment.z {
                writeln!(&mut wgsl, "{OUT_VAR}.z = {VAR_PREFIX}{z};").unwrap();
            }
            if let Some(w) = assignment.w {
                writeln!(&mut wgsl, "{OUT_VAR}.w = {VAR_PREFIX}{w};").unwrap();
            }

            wgsl
        })
        .collect()
}

pub fn generate_normal_intensity_wgsl(intensity: usize) -> String {
    format!("{OUT_VAR} = {VAR_PREFIX}{intensity};")
}

fn channel_assignment_wgsl(
    value: &AssignmentValue,
    name_to_index: &mut IndexMap<SmolStr, usize>,
) -> Option<String> {
    match value {
        AssignmentValue::Texture(t) => {
            let i = name_to_index.entry_index(t.name.clone());

            if i < MAX_SAMPLERS {
                let u = t.texcoords.first()?;
                let v = t.texcoords.get(1)?;

                Some(format!(
                    "textureSample(s{i}, s{i}_sampler, vec2({VAR_PREFIX}{u}, {VAR_PREFIX}{v})){}",
                    channel_wgsl(t.channel)
                ))
            } else {
                error!("Sampler index {i} exceeds supported max of {MAX_SAMPLERS}");
                None
            }
        }
        AssignmentValue::Attribute { name, channel } => {
            // TODO: Support more attributes.
            let c = channel_wgsl(*channel);
            match name.as_str() {
                "vColor" => Some(format!("in.vertex_color{c}")),
                "vPos" => Some(format!("in.position{c}")),
                "vNormal" => Some(format!("in.normal{c}")),
                "vTan" => Some(format!("in.tangent{c}")),
                "vTex0" => Some(format!("tex0{c}")),
                "vTex1" => Some(format!("tex1{c}")),
                "vTex2" => Some(format!("tex2{c}")),
                "vTex3" => Some(format!("tex3{c}")),
                "vTex4" => Some(format!("tex4{c}")),
                "vTex5" => Some(format!("tex5{c}")),
                "vTex6" => Some(format!("tex6{c}")),
                "vTex7" => Some(format!("tex7{c}")),
                "vTex8" => Some(format!("tex8{c}")),
                // The database uses "vBitan" to represent calculated bitangent attributes.
                "vBitan" => Some(format!("bitangent{c}")),
                _ => {
                    warn!("Unsupported attribute {name}{c}");
                    None
                }
            }
        }
        AssignmentValue::Float(f) => {
            if f.is_finite() {
                Some(format!("{f:?}"))
            } else {
                error!("Unsupported float literal {f:?}");
                None
            }
        }
    }
}

fn channel_wgsl(c: Option<char>) -> String {
    c.map(|c| format!(".{c}")).unwrap_or_default()
}
