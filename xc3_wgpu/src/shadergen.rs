use std::{fmt::Write, sync::LazyLock};

use aho_corasick::AhoCorasick;
use indexmap::IndexMap;
use indoc::formatdoc;
use log::{error, warn};
use smol_str::SmolStr;
use xc3_model::{
    IndexMapExt,
    material::{
        Texture,
        assignments::{
            Assignment, AssignmentValue, AssignmentValueXyz, AssignmentXyz, ChannelXyz,
            OutputAssignment, OutputAssignmentXyz, OutputAssignments,
        },
    },
    shader_database::Operation,
};

use crate::{material::ALPHA_TEST_TEXTURE, shader::model::TEXTURE_SAMPLER_COUNT};

const OUT_VAR: &str = "RESULT";
const VAR_PREFIX: &str = "VAR";
const VAR_PREFIX_XYZ: &str = "VAR_XYZ";

static WGSL_REPLACEMENTS: LazyLock<AhoCorasick> = LazyLock::new(|| {
    AhoCorasick::new([
        "let ASSIGN_VARS = 0.0;",
        "let ALPHA_TEST_DISCARD_GENERATED = 0.0;",
        "let ASSIGN_NORMAL_INTENSITY_GENERATED = 0.0;",
        "let ASSIGN_COLOR_GENERATED = 0.0;",
        "let ASSIGN_ETC_GENERATED = 0.0;",
        "let ASSIGN_NORMAL_GENERATED = 0.0;",
        "let ASSIGN_G_LGT_COLOR_GENERATED = 0.0;",
    ])
    .unwrap()
});

/// Generated WGSL model shader code for a material.
#[derive(Debug, Hash, Clone, PartialEq, Eq)]
pub struct ShaderWgsl {
    assignments: String,
    outputs: Vec<String>,
    alpha_test: String,
    normal_intensity: String,
}

impl ShaderWgsl {
    pub fn new(
        output_assignments: &OutputAssignments,
        alpha_test: Option<&Texture>,
        alpha_test_channel_index: usize,
        name_to_index: &mut IndexMap<SmolStr, usize>,
    ) -> Self {
        let xyz_assignments: Vec<_> = output_assignments
            .output_assignments
            .iter()
            .map(|a| a.merge_xyz(&output_assignments.assignments))
            .collect();

        let assignments =
            generate_assignments_wgsl(output_assignments, &xyz_assignments, name_to_index);

        let outputs =
            generate_outputs_wgsl(&output_assignments.output_assignments, &xyz_assignments);

        // Generate empty code if alpha testing is disabled.
        let alpha_test = alpha_test
            .map(|_| generate_alpha_test_wgsl(alpha_test_channel_index, name_to_index))
            .unwrap_or_default();

        let normal_intensity = output_assignments
            .normal_intensity
            .as_ref()
            .map(|i| generate_normal_intensity_wgsl(*i))
            .unwrap_or_default();

        Self {
            assignments,
            outputs,
            alpha_test,
            normal_intensity,
        }
    }

    pub fn create_model_shader(&self) -> String {
        let replace_with = &[
            &self.assignments,
            &self.alpha_test,
            // TODO: Avoid these replace calls?
            &self.normal_intensity.replace(OUT_VAR, "intensity"),
            &self.outputs[0].replace(OUT_VAR, "g_color"),
            &self.outputs[1].replace(OUT_VAR, "g_etc_buffer"),
            &self.outputs[2].replace(OUT_VAR, "g_normal"),
            &self.outputs[3].replace(OUT_VAR, "g_lgt_color"),
        ];

        let mut source = WGSL_REPLACEMENTS.replace_all(crate::shader::model::SOURCE, replace_with);

        // This section is only used for wgsl_to_wgpu reachability analysis and can be removed.
        if let (Some(start), Some(end)) = (
            source.find("let REMOVE_BEGIN = 0.0;"),
            source.find("let REMOVE_END = 0.0;"),
        ) {
            source.replace_range(start..end, "");
        }

        source
    }
}

fn write_assignment(
    wgsl: &mut String,
    value: &Assignment,
    name_to_index: &mut IndexMap<SmolStr, usize>,
) -> Option<()> {
    match value {
        Assignment::Func { op, args } => write_func(wgsl, op, args),
        Assignment::Value(v) => v
            .as_ref()
            .and_then(|v| write_assignment_value(wgsl, v, name_to_index)),
    }
}

fn write_func(wgsl: &mut String, op: &Operation, args: &[usize]) -> Option<()> {
    let arg0 = args.first();
    let arg1 = args.get(1);
    let arg2 = args.get(2);
    let arg3 = args.get(3);
    let arg4 = args.get(4);
    let arg5 = args.get(5);
    let arg6 = args.get(6);
    let arg7 = args.get(7);

    let a = VAR_PREFIX;
    match op {
        Operation::Unk => return None,
        Operation::Mix => write!(wgsl, "mix({a}{}, {a}{}, {a}{})", arg0?, arg1?, arg2?).unwrap(),
        Operation::Mul => write!(wgsl, "{a}{} * {a}{}", arg0?, arg1?).unwrap(),
        Operation::Div => write!(wgsl, "{a}{} / {a}{}", arg0?, arg1?).unwrap(),
        Operation::Add => write!(wgsl, "{a}{} + {a}{}", arg0?, arg1?).unwrap(),
        Operation::AddNormalX => write!(wgsl,
            "add_normal_maps(create_normal_map({a}{}, {a}{}), create_normal_map({a}{}, {a}{}), {a}{}).x * 0.5 + 0.5",
            arg0?, arg1?, arg2?, arg3?, arg4?
        ).unwrap(),
        Operation::AddNormalY => write!(wgsl,
            "add_normal_maps(create_normal_map({a}{}, {a}{}), create_normal_map({a}{}, {a}{}), {a}{}).y * 0.5 + 0.5",
            arg0?, arg1?, arg2?, arg3?, arg4?
        ).unwrap(),
        Operation::OverlayRatio => write!(wgsl,
            "mix({a}{0}, overlay_blend({a}{0}, {a}{1}), {a}{2})",
            arg0?, arg1?, arg2?
        ).unwrap(),
        Operation::Overlay => write!(wgsl, "overlay_blend({a}{}, {a}{})", arg0?, arg1?).unwrap(),
        Operation::Overlay2 => write!(wgsl, "overlay_blend2({a}{}, {a}{})", arg0?, arg1?).unwrap(),
        Operation::Power => write!(wgsl, "pow({a}{}, {a}{})", arg0?, arg1?).unwrap(),
        Operation::Min => write!(wgsl, "min({a}{}, {a}{})", arg0?, arg1?).unwrap(),
        Operation::Max => write!(wgsl, "max({a}{}, {a}{})", arg0?, arg1?).unwrap(),
        Operation::Clamp => write!(wgsl, "clamp({a}{}, {a}{}, {a}{})", arg0?, arg1?, arg2?).unwrap(),
        Operation::Sub => write!(wgsl, "{a}{} - {a}{}", arg0?, arg1?).unwrap(),
        Operation::Fma => write!(wgsl, "{a}{} * {a}{} + {a}{}", arg0?, arg1?, arg2?).unwrap(),
        Operation::Abs => write!(wgsl, "abs({a}{})", arg0?).unwrap(),
        Operation::Fresnel => write!(wgsl, "fresnel_ratio({a}{}, n_dot_v)", arg0?).unwrap(),
        Operation::MulRatio => write!(wgsl, "mix({a}{0}, {a}{0} * {a}{1}, {a}{2})", arg0?, arg1?, arg2?).unwrap(),
        Operation::Sqrt => write!(wgsl, "sqrt({a}{})", arg0?).unwrap(),
        Operation::TexMatrix => write!(wgsl,
            "dot(vec4({a}{}, {a}{}, 0.0, 1.0), vec4({a}{}, {a}{}, {a}{}, {a}{}))",
            arg0?, arg1?, arg2?, arg3?, arg4?, arg5?
        ).unwrap(),
        Operation::TexParallaxX => write!(wgsl, "{a}{} + uv_parallax(in, {a}{}).x", arg0?, arg1?).unwrap(),
        Operation::TexParallaxY => write!(wgsl, "{a}{} + uv_parallax(in, {a}{}).y", arg0?, arg1?).unwrap(),
        Operation::ReflectX => write!(wgsl,
            "reflect(vec3({a}{}, {a}{}, {a}{}), vec3({a}{}, {a}{}, {a}{})).x",
            arg0?, arg1?, arg2?, arg3?, arg4?, arg5?
        ).unwrap(),
        Operation::ReflectY => write!(wgsl,
            "reflect(vec3({a}{}, {a}{}, {a}{}), vec3({a}{}, {a}{}, {a}{})).y",
            arg0?, arg1?, arg2?, arg3?, arg4?, arg5?
        ).unwrap(),
        Operation::ReflectZ => write!(wgsl,
            "reflect(vec3({a}{}, {a}{}, {a}{}), vec3({a}{}, {a}{}, {a}{})).z",
            arg0?, arg1?, arg2?, arg3?, arg4?, arg5?
        ).unwrap(),
        Operation::Floor => write!(wgsl, "floor({a}{})", arg0?).unwrap(),
        Operation::Select => write!(wgsl, "mix({a}{}, {a}{}, f32({a}{}))", arg2?, arg1?, arg0?).unwrap(),
        Operation::Equal => write!(wgsl, "{a}{} == {a}{}", arg0?, arg1?).unwrap(),
        Operation::NotEqual => write!(wgsl, "{a}{} != {a}{}", arg0?, arg1?).unwrap(),
        Operation::Less => write!(wgsl, "{a}{} < {a}{}", arg0?, arg1?).unwrap(),
        Operation::Greater => write!(wgsl, "{a}{} > {a}{}", arg0?, arg1?).unwrap(),
        Operation::LessEqual => write!(wgsl, "{a}{} <= {a}{}", arg0?, arg1?).unwrap(),
        Operation::GreaterEqual => write!(wgsl, "{a}{} >= {a}{}", arg0?, arg1?).unwrap(),
        Operation::Dot4 => write!(wgsl,
            "dot(vec4({a}{}, {a}{}, {a}{}, {a}{}), vec4({a}{}, {a}{}, {a}{}, {a}{}))",
            arg0?, arg1?, arg2?, arg3?, arg4?, arg5?, arg6?, arg7?
        ).unwrap(),
        Operation::NormalMapX => write!(wgsl,
            "apply_normal_map(create_normal_map({a}{}, {a}{}), in.tangent.xyz, bitangent.xyz, in.normal.xyz).x",
            arg0?, arg1?
        ).unwrap(),
        Operation::NormalMapY => write!(wgsl,
            "apply_normal_map(create_normal_map({a}{}, {a}{}), in.tangent.xyz, bitangent.xyz, in.normal.xyz).y",
            arg0?, arg1?
        ).unwrap(),
        Operation::NormalMapZ => write!(wgsl,
            "apply_normal_map(create_normal_map({a}{}, {a}{}), in.tangent.xyz, bitangent.xyz, in.normal.xyz).z",
            arg0?, arg1?
        ).unwrap(),
        Operation::MonochromeX => write!(wgsl,
            "monochrome({a}{}, {a}{}, {a}{}, {a}{}).x",
            arg0?, arg1?, arg2?, arg3?
        ).unwrap(),
        Operation::MonochromeY => write!(wgsl,
            "monochrome({a}{}, {a}{}, {a}{}, {a}{}).y",
            arg0?, arg1?, arg2?, arg3?
        ).unwrap(),
        Operation::MonochromeZ => write!(wgsl,
            "monochrome({a}{}, {a}{}, {a}{}, {a}{}).z",
            arg0?, arg1?, arg2?, arg3?
        ).unwrap(),
        Operation::Negate => write!(wgsl, "-{a}{}", arg0?).unwrap(),
        // TODO: Pass instance index to fragment shader instead?
        Operation::FurInstanceAlpha => write!(wgsl, "in.vertex_color.a").unwrap(),
        Operation::Float => write!(wgsl, "f32({a}{})", arg0?).unwrap(),
        Operation::Int => write!(wgsl, "i32({a}{})", arg0?).unwrap(),
        Operation::Uint => write!(wgsl, "u32({a}{})", arg0?).unwrap(),
        Operation::Truncate => write!(wgsl, "trunc({a}{})", arg0?).unwrap(),
        Operation::FloatBitsToInt => write!(wgsl, "bitcast<i32>({a}{})", arg0?).unwrap(),
        Operation::IntBitsToFloat => write!(wgsl, "bitcast<f32>({a}{})", arg0?).unwrap(),
        Operation::UintBitsToFloat => write!(wgsl, "bitcast<f32>({a}{})", arg0?).unwrap(),
        Operation::InverseSqrt => write!(wgsl, "inverseSqrt({a}{})", arg0?).unwrap(),
        Operation::Not => write!(wgsl, "!{a}{}", arg0?).unwrap(),
        Operation::LeftShift => write!(wgsl, "{a}{} >> {a}{}", arg0?, arg1?).unwrap(),
        Operation::RightShift => write!(wgsl, "{a}{} >> {a}{}", arg0?, arg1?).unwrap(),
        Operation::PartialDerivativeX => write!(wgsl, "dpdx({a}{})", arg0?).unwrap(),
        Operation::PartialDerivativeY => write!(wgsl, "dpdy({a}{})", arg0?).unwrap(),
        Operation::Exp2 => write!(wgsl, "exp2({a}{})", arg0?).unwrap(),
        Operation::Log2 => write!(wgsl, "log2({a}{})", arg0?).unwrap(),
        Operation::Sin => write!(wgsl, "sin({a}{})", arg0?).unwrap(),
        Operation::Cos => write!(wgsl, "cos({a}{})", arg0?).unwrap(),
    }
    Some(())
}

fn generate_alpha_test_wgsl(
    alpha_test_channel_index: usize,
    name_to_index: &mut IndexMap<SmolStr, usize>,
) -> String {
    let i = name_to_index[ALPHA_TEST_TEXTURE];

    if i < TEXTURE_SAMPLER_COUNT as usize {
        let c = ['x', 'y', 'z', 'w'][alpha_test_channel_index];

        // TODO: Detect the UV attribute to use with alpha testing.
        formatdoc! {"
            if textureSample(textures[{i}], alpha_test_sampler, tex0.xy).{c} <= per_material.alpha_test_ref {{
                discard;
            }}
        "}
    } else {
        error!("Sampler index {i} exceeds supported max of {TEXTURE_SAMPLER_COUNT}");
        String::new()
    }
}

fn generate_assignments_wgsl(
    assignments: &OutputAssignments,
    xyz_assignments: &[Option<OutputAssignmentXyz>],
    name_to_index: &mut IndexMap<SmolStr, usize>,
) -> String {
    let mut wgsl = String::new();

    // Write variables shared by all outputs.
    // Assume that values appear after values they depend on.
    for (i, value) in assignments.assignments.iter().enumerate() {
        write!(wgsl, "let {VAR_PREFIX}{i} = ",).unwrap();
        if write_assignment(&mut wgsl, value, name_to_index).is_none() {
            write!(&mut wgsl, "0.0").unwrap();
        }
        writeln!(wgsl, ";",).unwrap();
    }

    // TODO: Share xyz assignments with all channels?
    for (i, assignment) in xyz_assignments.iter().enumerate() {
        if let Some(assignment) = assignment {
            for (j, value) in assignment.assignments.iter().enumerate() {
                write!(wgsl, "let {VAR_PREFIX_XYZ}_{i}_{j} = ",).unwrap();
                if write_assignment_xyz(&mut wgsl, value, i, name_to_index).is_none() {
                    write!(&mut wgsl, "vec3(0.0)").unwrap();
                }
                writeln!(wgsl, ";",).unwrap();
            }
        }
    }

    wgsl
}

fn generate_outputs_wgsl(
    assignments: &[OutputAssignment],
    xyz_assignments: &[Option<OutputAssignmentXyz>],
) -> Vec<String> {
    // Don't generate code for velocity or depth.
    assignments
        .iter()
        .zip(xyz_assignments)
        .enumerate()
        .filter(|(i, _)| *i != 3 && *i != 4)
        .map(|(i, (assignment, xyz_assignment))| {
            let mut wgsl = String::new();

            // Write any final assignments.
            if let Some(xyz) = xyz_assignment {
                for c in "xyz".chars() {
                    // TODO: Share xyz assignments with all channels?
                    writeln!(
                        &mut wgsl,
                        "{OUT_VAR}.{c} = {VAR_PREFIX_XYZ}_{i}_{}.{c};",
                        xyz.assignment
                    )
                    .unwrap();
                }
            } else {
                if let Some(x) = assignment.x {
                    writeln!(&mut wgsl, "{OUT_VAR}.x = {VAR_PREFIX}{x};").unwrap();
                }
                if let Some(y) = assignment.y {
                    writeln!(&mut wgsl, "{OUT_VAR}.y = {VAR_PREFIX}{y};").unwrap();
                }
                if let Some(z) = assignment.z {
                    writeln!(&mut wgsl, "{OUT_VAR}.z = {VAR_PREFIX}{z};").unwrap();
                }
            }

            if let Some(w) = assignment.w {
                writeln!(&mut wgsl, "{OUT_VAR}.w = {VAR_PREFIX}{w};").unwrap();
            }

            wgsl
        })
        .collect()
}

fn generate_normal_intensity_wgsl(intensity: usize) -> String {
    format!("{OUT_VAR} = {VAR_PREFIX}{intensity};")
}

fn write_assignment_value(
    wgsl: &mut String,
    value: &AssignmentValue,
    name_to_index: &mut IndexMap<SmolStr, usize>,
) -> Option<()> {
    match value {
        AssignmentValue::Texture(t) => {
            let i = name_to_index.entry_index(t.name.clone());

            if i < TEXTURE_SAMPLER_COUNT as usize {
                // TODO: Support cube maps.
                if t.texcoords.len() == 3 {
                    write!(wgsl, "textureSample(textures_d3[{i}], samplers[{i}], ",).unwrap();
                } else {
                    write!(wgsl, "textureSample(textures[{i}], samplers[{i}], ",).unwrap();
                }
                write_texture_coordinates(wgsl, &t.texcoords)?;
                write!(wgsl, ")").unwrap();
                write_channel(wgsl, t.channel);
            } else {
                error!("Sampler index {i} exceeds supported max of {TEXTURE_SAMPLER_COUNT}");
                return None;
            }
        }
        AssignmentValue::Attribute { name, channel } => {
            // TODO: Support more attributes.
            match name.as_str() {
                "vColor" => write_attribute(wgsl, "in.vertex_color", channel),
                "vPos" => write_attribute(wgsl, "in.position", channel),
                "vNormal" => write_attribute(wgsl, "in.normal", channel),
                "vTan" => write_attribute(wgsl, "in.tangent", channel),
                "vTex0" => write_attribute(wgsl, "tex0", channel),
                "vTex1" => write_attribute(wgsl, "tex1", channel),
                "vTex2" => write_attribute(wgsl, "tex2", channel),
                "vTex3" => write_attribute(wgsl, "tex3", channel),
                "vTex4" => write_attribute(wgsl, "tex4", channel),
                "vTex5" => write_attribute(wgsl, "tex5", channel),
                "vTex6" => write_attribute(wgsl, "tex6", channel),
                "vTex7" => write_attribute(wgsl, "tex7", channel),
                "vTex8" => write_attribute(wgsl, "tex8", channel),
                // The database uses "vBitan" to represent calculated bitangent attributes.
                "vBitan" => write_attribute(wgsl, "bitangent", channel),
                _ => {
                    if let Some(c) = channel {
                        warn!("Unsupported attribute {name}.{c}");
                    } else {
                        warn!("Unsupported attribute {name}");
                    }
                    return None;
                }
            }
        }
        AssignmentValue::Float(f) => {
            if f.is_finite() {
                write!(wgsl, "{f:?}").unwrap()
            } else {
                error!("Unsupported float literal {f:?}");
                return None;
            }
        }
        AssignmentValue::Int(i) => {
            if *i >= 0 {
                write!(wgsl, "{i}u").unwrap()
            } else {
                write!(wgsl, "{i}i").unwrap()
            }
        }
    }
    Some(())
}

fn write_attribute(wgsl: &mut String, name: &str, channel: &Option<char>) {
    write!(wgsl, "{name}").unwrap();
    write_channel(wgsl, *channel);
}

fn write_texture_coordinates(wgsl: &mut String, coords: &[usize]) -> Option<()> {
    match coords {
        [u, v] => write!(wgsl, "vec2({VAR_PREFIX}{u}, {VAR_PREFIX}{v})").unwrap(),
        [u, v, w] => write!(
            wgsl,
            "vec3({VAR_PREFIX}{u}, {VAR_PREFIX}{v}, {VAR_PREFIX}{w})"
        )
        .unwrap(),
        _ => {
            error!("Unexpected texture coordinates {coords:?}");
            return None;
        }
    }
    Some(())
}

fn write_channel(wgsl: &mut String, c: Option<char>) {
    if let Some(c) = c {
        write!(wgsl, ".{c}").unwrap();
    }
}

fn write_assignment_xyz(
    wgsl: &mut String,
    value: &AssignmentXyz,
    output_index: usize,
    name_to_index: &mut IndexMap<SmolStr, usize>,
) -> Option<()> {
    match value {
        AssignmentXyz::Func { op, args } => write_func_xyz(wgsl, op, args, output_index),
        AssignmentXyz::Value(v) => v
            .as_ref()
            .and_then(|v| write_assignment_value_xyz(wgsl, v, name_to_index)),
    }
}

fn write_func_xyz(
    wgsl: &mut String,
    op: &Operation,
    args: &[usize],
    output_index: usize,
) -> Option<()> {
    let arg0 = args.first();
    let arg1 = args.get(1);
    let arg2 = args.get(2);
    let arg3 = args.get(3);
    let arg4 = args.get(4);
    let arg5 = args.get(5);
    let arg6 = args.get(6);
    let arg7 = args.get(7);

    // TODO: Will these operations all work with xyz inputs?
    let a = format_args!("{VAR_PREFIX_XYZ}_{output_index}_");
    match op {
        Operation::Unk => return None,
        Operation::Mix => write!(wgsl, "mix({a}{}, {a}{}, {a}{})", arg0?, arg1?, arg2?).unwrap(),
        Operation::Mul => write!(wgsl, "{a}{} * {a}{}", arg0?, arg1?).unwrap(),
        Operation::Div => write!(wgsl, "{a}{} / {a}{}", arg0?, arg1?).unwrap(),
        Operation::Add => write!(wgsl, "{a}{} + {a}{}", arg0?, arg1?).unwrap(),
        Operation::AddNormalX => write!(wgsl,
            "add_normal_maps(create_normal_map({a}{}, {a}{}), create_normal_map({a}{}, {a}{}), {a}{}).x * 0.5 + 0.5",
            arg0?, arg1?, arg2?, arg3?, arg4?
        ).unwrap(),
        Operation::AddNormalY => write!(wgsl,
            "add_normal_maps(create_normal_map({a}{}, {a}{}), create_normal_map({a}{}, {a}{}), {a}{}).y * 0.5 + 0.5",
            arg0?, arg1?, arg2?, arg3?, arg4?
        ).unwrap(),
        Operation::OverlayRatio => write!(wgsl,
            "mix({a}{0}, overlay_blend_xyz({a}{0}, {a}{1}), {a}{2})",
            arg0?, arg1?, arg2?
        ).unwrap(),
        Operation::Overlay => write!(wgsl, "overlay_blend_xyz({a}{}, {a}{})", arg0?, arg1?).unwrap(),
        Operation::Overlay2 => write!(wgsl, "overlay_blend2_xyz({a}{}, {a}{})", arg0?, arg1?).unwrap(),
        Operation::Power => write!(wgsl, "pow({a}{}, {a}{})", arg0?, arg1?).unwrap(),
        Operation::Min => write!(wgsl, "min({a}{}, {a}{})", arg0?, arg1?).unwrap(),
        Operation::Max => write!(wgsl, "max({a}{}, {a}{})", arg0?, arg1?).unwrap(),
        Operation::Clamp => write!(wgsl, "clamp({a}{}, {a}{}, {a}{})", arg0?, arg1?, arg2?).unwrap(),
        Operation::Sub => write!(wgsl, "{a}{} - {a}{}", arg0?, arg1?).unwrap(),
        Operation::Fma => write!(wgsl, "{a}{} * {a}{} + {a}{}", arg0?, arg1?, arg2?).unwrap(),
        Operation::Abs => write!(wgsl, "abs({a}{})", arg0?).unwrap(),
        Operation::Fresnel => write!(wgsl, "fresnel_ratio_xyz({a}{}, n_dot_v)", arg0?).unwrap(),
        Operation::MulRatio => write!(wgsl, "mix({a}{0}, {a}{0} * {a}{1}, {a}{2})", arg0?, arg1?, arg2?).unwrap(),
        Operation::Sqrt => write!(wgsl, "sqrt({a}{})", arg0?).unwrap(),
        Operation::TexMatrix => write!(wgsl,
            "dot(vec4({a}{}, {a}{}, 0.0, 1.0), vec4({a}{}, {a}{}, {a}{}, {a}{}))",
            arg0?, arg1?, arg2?, arg3?, arg4?, arg5?
        ).unwrap(),
        Operation::TexParallaxX => write!(wgsl, "{a}{} + uv_parallax(in, {a}{}).x", arg0?, arg1?).unwrap(),
        Operation::TexParallaxY => write!(wgsl, "{a}{} + uv_parallax(in, {a}{}).y", arg0?, arg1?).unwrap(),
        Operation::ReflectX => write!(wgsl,
            "reflect(vec3({a}{}, {a}{}, {a}{}), vec3({a}{}, {a}{}, {a}{})).x",
            arg0?, arg1?, arg2?, arg3?, arg4?, arg5?
        ).unwrap(),
        Operation::ReflectY => write!(wgsl,
            "reflect(vec3({a}{}, {a}{}, {a}{}), vec3({a}{}, {a}{}, {a}{})).y",
            arg0?, arg1?, arg2?, arg3?, arg4?, arg5?
        ).unwrap(),
        Operation::ReflectZ => write!(wgsl,
            "reflect(vec3({a}{}, {a}{}, {a}{}), vec3({a}{}, {a}{}, {a}{})).z",
            arg0?, arg1?, arg2?, arg3?, arg4?, arg5?
        ).unwrap(),
        Operation::Floor => write!(wgsl, "floor({a}{})", arg0?).unwrap(),
        Operation::Select => write!(wgsl, "mix({a}{}, {a}{}, vec3<f32>({a}{}))", arg2?, arg1?, arg0?).unwrap(),
        Operation::Equal => write!(wgsl, "{a}{} == {a}{}", arg0?, arg1?).unwrap(),
        Operation::NotEqual => write!(wgsl, "{a}{} != {a}{}", arg0?, arg1?).unwrap(),
        Operation::Less => write!(wgsl, "{a}{} < {a}{}", arg0?, arg1?).unwrap(),
        Operation::Greater => write!(wgsl, "{a}{} > {a}{}", arg0?, arg1?).unwrap(),
        Operation::LessEqual => write!(wgsl, "{a}{} <= {a}{}", arg0?, arg1?).unwrap(),
        Operation::GreaterEqual => write!(wgsl, "{a}{} >= {a}{}", arg0?, arg1?).unwrap(),
        Operation::Dot4 => write!(wgsl,
            "dot(vec4({a}{}, {a}{}, {a}{}, {a}{}), vec4({a}{}, {a}{}, {a}{}, {a}{}))",
            arg0?, arg1?, arg2?, arg3?, arg4?, arg5?, arg6?, arg7?
        ).unwrap(),
        Operation::NormalMapX => write!(wgsl,
            "apply_normal_map(create_normal_map({a}{}, {a}{}), in.tangent.xyz, bitangent.xyz, in.normal.xyz).xxx",
            arg0?, arg1?
        ).unwrap(),
        Operation::NormalMapY => write!(wgsl,
            "apply_normal_map(create_normal_map({a}{}, {a}{}), in.tangent.xyz, bitangent.xyz, in.normal.xyz).yyy",
            arg0?, arg1?
        ).unwrap(),
        Operation::NormalMapZ => write!(wgsl,
            "apply_normal_map(create_normal_map({a}{}, {a}{}), in.tangent.xyz, bitangent.xyz, in.normal.xyz).zzz",
            arg0?, arg1?
        ).unwrap(),
        Operation::MonochromeX => write!(wgsl,
            "monochrome_xyz_x({a}{}, {a}{}, {a}{}, {a}{})",
            arg0?, arg1?, arg2?, arg3?
        ).unwrap(),
        Operation::MonochromeY => write!(wgsl,
            "monochrome_xyz_y({a}{}, {a}{}, {a}{}, {a}{})",
            arg0?, arg1?, arg2?, arg3?
        ).unwrap(),
        Operation::MonochromeZ => write!(wgsl,
            "monochrome_xyz_z({a}{}, {a}{}, {a}{}, {a}{})",
            arg0?, arg1?, arg2?, arg3?
        ).unwrap(),
        Operation::Negate => write!(wgsl, "-{a}{}", arg0?).unwrap(),
        Operation::FurInstanceAlpha => write!(wgsl, "in.vertex_color.a").unwrap(),
        Operation::Float => write!(wgsl, "vec3<f32>({a}{})", arg0?).unwrap(),
        Operation::Int => write!(wgsl, "vec3<i32>({a}{})", arg0?).unwrap(),
        Operation::Uint => write!(wgsl, "vec3<u32>({a}{})", arg0?).unwrap(),
        Operation::Truncate => write!(wgsl, "trunc({a}{})", arg0?).unwrap(),
        Operation::FloatBitsToInt => write!(wgsl, "bitcast<i32>({a}{})", arg0?).unwrap(),
        Operation::IntBitsToFloat => write!(wgsl, "bitcast<f32>({a}{})", arg0?).unwrap(),
        Operation::UintBitsToFloat => write!(wgsl, "bitcast<f32>({a}{})", arg0?).unwrap(),
        Operation::InverseSqrt => write!(wgsl, "inverseSqrt({a}{})", arg0?).unwrap(),
        Operation::Not => write!(wgsl, "!{a}{}", arg0?).unwrap(),
        Operation::LeftShift => write!(wgsl, "{a}{} >> {a}{}", arg0?, arg1?).unwrap(),
        Operation::RightShift => write!(wgsl, "{a}{} >> {a}{}", arg0?, arg1?).unwrap(),
        Operation::PartialDerivativeX => write!(wgsl, "dpdx({a}{})", arg0?).unwrap(),
        Operation::PartialDerivativeY => write!(wgsl, "dpdy({a}{})", arg0?).unwrap(),
        Operation::Exp2 => write!(wgsl, "exp2({a}{})", arg0?).unwrap(),
        Operation::Log2 => write!(wgsl, "log2({a}{})", arg0?).unwrap(),
        Operation::Sin => write!(wgsl, "sin({a}{})", arg0?).unwrap(),
        Operation::Cos => write!(wgsl, "cos({a}{})", arg0?).unwrap(),
    }
    Some(())
}

fn write_assignment_value_xyz(
    wgsl: &mut String,
    value: &AssignmentValueXyz,
    name_to_index: &mut IndexMap<SmolStr, usize>,
) -> Option<()> {
    match value {
        AssignmentValueXyz::Texture(t) => {
            let i = name_to_index.entry_index(t.name.clone());

            if i < TEXTURE_SAMPLER_COUNT as usize {
                let channels = channel_xyz_wgsl(t.channel);
                // TODO: Support cube maps.
                if t.texcoords.len() == 3 {
                    write!(wgsl, "textureSample(textures_d3[{i}], samplers[{i}], ",).unwrap();
                } else {
                    write!(wgsl, "textureSample(textures[{i}], samplers[{i}], ",).unwrap();
                }
                write_texture_coordinates(wgsl, &t.texcoords)?;
                write!(wgsl, "){channels}").unwrap();
                Some(())
            } else {
                error!("Sampler index {i} exceeds supported max of {TEXTURE_SAMPLER_COUNT}");
                None
            }
        }
        AssignmentValueXyz::Attribute { name, channel } => {
            // TODO: Support more attributes.
            let c = channel_xyz_wgsl(*channel);
            match name.as_str() {
                "vColor" => write!(wgsl, "in.vertex_color{c}").unwrap(),
                "vPos" => write!(wgsl, "in.position{c}").unwrap(),
                "vNormal" => write!(wgsl, "in.normal{c}").unwrap(),
                "vTan" => write!(wgsl, "in.tangent{c}").unwrap(),
                "vTex0" => write!(wgsl, "tex0{c}").unwrap(),
                "vTex1" => write!(wgsl, "tex1{c}").unwrap(),
                "vTex2" => write!(wgsl, "tex2{c}").unwrap(),
                "vTex3" => write!(wgsl, "tex3{c}").unwrap(),
                "vTex4" => write!(wgsl, "tex4{c}").unwrap(),
                "vTex5" => write!(wgsl, "tex5{c}").unwrap(),
                "vTex6" => write!(wgsl, "tex6{c}").unwrap(),
                "vTex7" => write!(wgsl, "tex7{c}").unwrap(),
                "vTex8" => write!(wgsl, "tex8{c}").unwrap(),
                // The database uses "vBitan" to represent calculated bitangent attributes.
                "vBitan" => write!(wgsl, "bitangent{c}").unwrap(),
                _ => {
                    warn!("Unsupported attribute {name}{c}");
                    return None;
                }
            }
            Some(())
        }
        AssignmentValueXyz::Float(f) => {
            if f.iter().all(|f| f.is_finite()) {
                write!(wgsl, "vec3({:?}, {:?}, {:?})", f[0], f[1], f[2]).unwrap();
                Some(())
            } else {
                error!("Unsupported float literals {f:?}");
                None
            }
        }
    }
}

fn channel_xyz_wgsl(c: Option<ChannelXyz>) -> &'static str {
    c.map(|c| match c {
        ChannelXyz::Xyz => ".xyz",
        ChannelXyz::X => ".xxx",
        ChannelXyz::Y => ".yyy",
        ChannelXyz::Z => ".zzz",
        ChannelXyz::W => ".www",
    })
    .unwrap_or_default()
}
