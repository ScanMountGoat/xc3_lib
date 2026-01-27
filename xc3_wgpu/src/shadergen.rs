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
    AhoCorasick::new(&[
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
        Operation::Mix => Some(write!(wgsl, "mix({}, {}, {})", arg0?, arg1?, arg2?).unwrap()),
        Operation::Mul => Some(write!(wgsl, "{} * {}", arg0?, arg1?).unwrap()),
        Operation::Div => Some(write!(wgsl, "{} / {}", arg0?, arg1?).unwrap()),
        Operation::Add => Some(write!(wgsl, "{} + {}", arg0?, arg1?).unwrap()),
        Operation::AddNormalX => Some(write!(wgsl,
            "add_normal_maps(create_normal_map({}, {}), create_normal_map({}, {}), {}).x * 0.5 + 0.5",
            arg0?, arg1?, arg2?, arg3?, arg4?
        ).unwrap()),
        Operation::AddNormalY => Some(write!(wgsl,
            "add_normal_maps(create_normal_map({}, {}), create_normal_map({}, {}), {}).y * 0.5 + 0.5",
            arg0?, arg1?, arg2?, arg3?, arg4?
        ).unwrap()),
        Operation::OverlayRatio => Some(write!(wgsl,
            "mix({0}, overlay_blend({0}, {1}), {2})",
            arg0?, arg1?, arg2?
        ).unwrap()),
        Operation::Overlay => Some(write!(wgsl, "overlay_blend({}, {})", arg0?, arg1?).unwrap()),
        Operation::Overlay2 => Some(write!(wgsl, "overlay_blend2({}, {})", arg0?, arg1?).unwrap()),
        Operation::Power => Some(write!(wgsl, "pow({}, {})", arg0?, arg1?).unwrap()),
        Operation::Min => Some(write!(wgsl, "min({}, {})", arg0?, arg1?).unwrap()),
        Operation::Max => Some(write!(wgsl, "max({}, {})", arg0?, arg1?).unwrap()),
        Operation::Clamp => Some(write!(wgsl, "clamp({}, {}, {})", arg0?, arg1?, arg2?).unwrap()),
        Operation::Sub => Some(write!(wgsl, "{} - {}", arg0?, arg1?).unwrap()),
        Operation::Fma => Some(write!(wgsl, "{} * {} + {}", arg0?, arg1?, arg2?).unwrap()),
        Operation::Abs => Some(write!(wgsl, "abs({})", arg0?).unwrap()),
        Operation::Fresnel => Some(write!(wgsl, "fresnel_ratio({}, n_dot_v)", arg0?).unwrap()),
        Operation::MulRatio => Some(write!(wgsl, "mix({0}, {0} * {1}, {2})", arg0?, arg1?, arg2?).unwrap()),
        Operation::Sqrt => Some(write!(wgsl, "sqrt({})", arg0?).unwrap()),
        Operation::TexMatrix => Some(write!(wgsl,
            "dot(vec4({}, {}, 0.0, 1.0), vec4({}, {}, {}, {}))",
            arg0?, arg1?, arg2?, arg3?, arg4?, arg5?
        ).unwrap()),
        Operation::TexParallaxX => Some(write!(wgsl, "{} + uv_parallax(in, {}).x", arg0?, arg1?).unwrap()),
        Operation::TexParallaxY => Some(write!(wgsl, "{} + uv_parallax(in, {}).y", arg0?, arg1?).unwrap()),
        Operation::ReflectX => Some(write!(wgsl,
            "reflect(vec3({}, {}, {}), vec3({}, {}, {})).x",
            arg0?, arg1?, arg2?, arg3?, arg4?, arg5?
        ).unwrap()),
        Operation::ReflectY => Some(write!(wgsl,
            "reflect(vec3({}, {}, {}), vec3({}, {}, {})).y",
            arg0?, arg1?, arg2?, arg3?, arg4?, arg5?
        ).unwrap()),
        Operation::ReflectZ => Some(write!(wgsl,
            "reflect(vec3({}, {}, {}), vec3({}, {}, {})).z",
            arg0?, arg1?, arg2?, arg3?, arg4?, arg5?
        ).unwrap()),
        Operation::Floor => Some(write!(wgsl, "floor({})", arg0?).unwrap()),
        Operation::Select => Some(write!(wgsl, "mix({}, {}, f32({}))", arg2?, arg1?, arg0?).unwrap()),
        Operation::Equal => Some(write!(wgsl, "{} == {}", arg0?, arg1?).unwrap()),
        Operation::NotEqual => Some(write!(wgsl, "{} != {}", arg0?, arg1?).unwrap()),
        Operation::Less => Some(write!(wgsl, "{} < {}", arg0?, arg1?).unwrap()),
        Operation::Greater => Some(write!(wgsl, "{} > {}", arg0?, arg1?).unwrap()),
        Operation::LessEqual => Some(write!(wgsl, "{} <= {}", arg0?, arg1?).unwrap()),
        Operation::GreaterEqual => Some(write!(wgsl, "{} >= {}", arg0?, arg1?).unwrap()),
        Operation::Dot4 => Some(write!(wgsl,
            "dot(vec4({}, {}, {}, {}), vec4({}, {}, {}, {}))",
            arg0?, arg1?, arg2?, arg3?, arg4?, arg5?, arg6?, arg7?
        ).unwrap()),
        Operation::NormalMapX => Some(write!(wgsl,
            "apply_normal_map(create_normal_map({}, {}), in.tangent.xyz, bitangent.xyz, in.normal.xyz).x",
            arg0?, arg1?
        ).unwrap()),
        Operation::NormalMapY => Some(write!(wgsl,
            "apply_normal_map(create_normal_map({}, {}), in.tangent.xyz, bitangent.xyz, in.normal.xyz).y",
            arg0?, arg1?
        ).unwrap()),
        Operation::NormalMapZ => Some(write!(wgsl,
            "apply_normal_map(create_normal_map({}, {}), in.tangent.xyz, bitangent.xyz, in.normal.xyz).z",
            arg0?, arg1?
        ).unwrap()),
        Operation::MonochromeX => Some(write!(wgsl,
            "monochrome({}, {}, {}, {}).x",
            arg0?, arg1?, arg2?, arg3?
        ).unwrap()),
        Operation::MonochromeY => Some(write!(wgsl,
            "monochrome({}, {}, {}, {}).y",
            arg0?, arg1?, arg2?, arg3?
        ).unwrap()),
        Operation::MonochromeZ => Some(write!(wgsl,
            "monochrome({}, {}, {}, {}).z",
            arg0?, arg1?, arg2?, arg3?
        ).unwrap()),
        Operation::Negate => Some(write!(wgsl, "-{}", arg0?).unwrap()),
        // TODO: Pass instance index to fragment shader instead?
        Operation::FurInstanceAlpha => Some(write!(wgsl, "in.vertex_color.a").unwrap()),
        Operation::Float => Some(write!(wgsl, "f32({})", arg0?).unwrap()),
        Operation::Int => Some(write!(wgsl, "i32({})", arg0?).unwrap()),
        Operation::Uint => Some(write!(wgsl, "u32({})", arg0?).unwrap()),
        Operation::Truncate => Some(write!(wgsl, "trunc({})", arg0?).unwrap()),
        Operation::FloatBitsToInt => Some(write!(wgsl, "bitcast<i32>({})", arg0?).unwrap()),
        Operation::IntBitsToFloat => Some(write!(wgsl, "bitcast<f32>({})", arg0?).unwrap()),
        Operation::UintBitsToFloat => Some(write!(wgsl, "bitcast<f32>({})", arg0?).unwrap()),
        Operation::InverseSqrt => Some(write!(wgsl, "inverseSqrt({})", arg0?).unwrap()),
        Operation::Not => Some(write!(wgsl, "!{}", arg0?).unwrap()),
        Operation::LeftShift => Some(write!(wgsl, "{} >> {}", arg0?, arg1?).unwrap()),
        Operation::RightShift => Some(write!(wgsl, "{} >> {}", arg0?, arg1?).unwrap()),
        Operation::PartialDerivativeX => Some(write!(wgsl, "dpdx({})", arg0?).unwrap()),
        Operation::PartialDerivativeY => Some(write!(wgsl, "dpdy({})", arg0?).unwrap()),
        Operation::Exp2 => Some(write!(wgsl, "exp2({})", arg0?).unwrap()),
        Operation::Log2 => Some(write!(wgsl, "log2({})", arg0?).unwrap()),
        Operation::Sin => Some(write!(wgsl, "sin({})", arg0?).unwrap()),
        Operation::Cos => Some(write!(wgsl, "cos({})", arg0?).unwrap()),
    }
}

fn arg(args: &[usize], i: usize) -> Option<String> {
    Some(format!("{VAR_PREFIX}{}", args.get(i)?))
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
                let coords = texture_coordinates(&t.texcoords)?;
                let channels = channel_wgsl(t.channel);
                // TODO: Support cube maps.
                if t.texcoords.len() == 3 {
                    Some(
                        write!(
                            wgsl,
                            "textureSample(textures_d3[{i}], samplers[{i}], {coords}){channels}",
                        )
                        .unwrap(),
                    )
                } else {
                    Some(
                        write!(
                            wgsl,
                            "textureSample(textures[{i}], samplers[{i}], {coords}){channels}",
                        )
                        .unwrap(),
                    )
                }
            } else {
                error!("Sampler index {i} exceeds supported max of {TEXTURE_SAMPLER_COUNT}");
                None
            }
        }
        AssignmentValue::Attribute { name, channel } => {
            // TODO: Support more attributes.
            let c = channel_wgsl(*channel);
            match name.as_str() {
                "vColor" => Some(write!(wgsl, "in.vertex_color{c}").unwrap()),
                "vPos" => Some(write!(wgsl, "in.position{c}").unwrap()),
                "vNormal" => Some(write!(wgsl, "in.normal{c}").unwrap()),
                "vTan" => Some(write!(wgsl, "in.tangent{c}").unwrap()),
                "vTex0" => Some(write!(wgsl, "tex0{c}").unwrap()),
                "vTex1" => Some(write!(wgsl, "tex1{c}").unwrap()),
                "vTex2" => Some(write!(wgsl, "tex2{c}").unwrap()),
                "vTex3" => Some(write!(wgsl, "tex3{c}").unwrap()),
                "vTex4" => Some(write!(wgsl, "tex4{c}").unwrap()),
                "vTex5" => Some(write!(wgsl, "tex5{c}").unwrap()),
                "vTex6" => Some(write!(wgsl, "tex6{c}").unwrap()),
                "vTex7" => Some(write!(wgsl, "tex7{c}").unwrap()),
                "vTex8" => Some(write!(wgsl, "tex8{c}").unwrap()),
                // The database uses "vBitan" to represent calculated bitangent attributes.
                "vBitan" => Some(write!(wgsl, "bitangent{c}").unwrap()),
                _ => {
                    warn!("Unsupported attribute {name}{c}");
                    None
                }
            }
        }
        AssignmentValue::Float(f) => {
            if f.is_finite() {
                Some(write!(wgsl, "{f:?}").unwrap())
            } else {
                error!("Unsupported float literal {f:?}");
                None
            }
        }
        AssignmentValue::Int(i) => {
            if *i >= 0 {
                Some(write!(wgsl, "{i}u").unwrap())
            } else {
                Some(write!(wgsl, "{i}i").unwrap())
            }
        }
    }
}

fn texture_coordinates(coords: &[usize]) -> Option<String> {
    match coords {
        [u, v] => Some(format!("vec2({VAR_PREFIX}{u}, {VAR_PREFIX}{v})")),
        [u, v, w] => Some(format!(
            "vec3({VAR_PREFIX}{u}, {VAR_PREFIX}{v}, {VAR_PREFIX}{w})"
        )),
        _ => {
            error!("Unexpected texture coordinates {coords:?}");
            None
        }
    }
}

fn channel_wgsl(c: Option<char>) -> String {
    c.map(|c| format!(".{c}")).unwrap_or_default()
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
    let arg0 = arg_xyz(args, output_index, 0);
    let arg1 = arg_xyz(args, output_index, 1);
    let arg2 = arg_xyz(args, output_index, 2);
    let arg3 = arg_xyz(args, output_index, 3);
    let arg4 = arg_xyz(args, output_index, 4);
    let arg5 = arg_xyz(args, output_index, 5);
    let arg6 = arg_xyz(args, output_index, 6);
    let arg7 = arg_xyz(args, output_index, 7);

    // TODO: Will these all work with xyz inputs?
    match op {
        Operation::Unk => None,
        Operation::Mix => Some(write!(wgsl, "mix({}, {}, {})", arg0?, arg1?, arg2?).unwrap()),
        Operation::Mul => Some(write!(wgsl, "{} * {}", arg0?, arg1?).unwrap()),
        Operation::Div => Some(write!(wgsl, "{} / {}", arg0?, arg1?).unwrap()),
        Operation::Add => Some(write!(wgsl, "{} + {}", arg0?, arg1?).unwrap()),
        Operation::AddNormalX => Some(write!(wgsl,
            "add_normal_maps(create_normal_map({}, {}), create_normal_map({}, {}), {}).x * 0.5 + 0.5",
            arg0?, arg1?, arg2?, arg3?, arg4?
        ).unwrap()),
        Operation::AddNormalY => Some(write!(wgsl,
            "add_normal_maps(create_normal_map({}, {}), create_normal_map({}, {}), {}).y * 0.5 + 0.5",
            arg0?, arg1?, arg2?, arg3?, arg4?
        ).unwrap()),
        Operation::OverlayRatio => Some(write!(wgsl,
            "mix({0}, overlay_blend_xyz({0}, {1}), {2})",
            arg0?, arg1?, arg2?
        ).unwrap()),
        Operation::Overlay => Some(write!(wgsl, "overlay_blend_xyz({}, {})", arg0?, arg1?).unwrap()),
        Operation::Overlay2 => Some(write!(wgsl, "overlay_blend2_xyz({}, {})", arg0?, arg1?).unwrap()),
        Operation::Power => Some(write!(wgsl, "pow({}, {})", arg0?, arg1?).unwrap()),
        Operation::Min => Some(write!(wgsl, "min({}, {})", arg0?, arg1?).unwrap()),
        Operation::Max => Some(write!(wgsl, "max({}, {})", arg0?, arg1?).unwrap()),
        Operation::Clamp => Some(write!(wgsl, "clamp({}, {}, {})", arg0?, arg1?, arg2?).unwrap()),
        Operation::Sub => Some(write!(wgsl, "{} - {}", arg0?, arg1?).unwrap()),
        Operation::Fma => Some(write!(wgsl, "{} * {} + {}", arg0?, arg1?, arg2?).unwrap()),
        Operation::Abs => Some(write!(wgsl, "abs({})", arg0?).unwrap()),
        Operation::Fresnel => Some(write!(wgsl, "fresnel_ratio_xyz({}, n_dot_v)", arg0?).unwrap()),
        Operation::MulRatio => Some(write!(wgsl, "mix({0}, {0} * {1}, {2})", arg0?, arg1?, arg2?).unwrap()),
        Operation::Sqrt => Some(write!(wgsl, "sqrt({})", arg0?).unwrap()),
        Operation::TexMatrix => Some(write!(wgsl,
            "dot(vec4({}, {}, 0.0, 1.0), vec4({}, {}, {}, {}))",
            arg0?, arg1?, arg2?, arg3?, arg4?, arg5?
        ).unwrap()),
        Operation::TexParallaxX => Some(write!(wgsl, "{} + uv_parallax(in, {}).x", arg0?, arg1?).unwrap()),
        Operation::TexParallaxY => Some(write!(wgsl, "{} + uv_parallax(in, {}).y", arg0?, arg1?).unwrap()),
        Operation::ReflectX => Some(write!(wgsl,
            "reflect(vec3({}, {}, {}), vec3({}, {}, {})).x",
            arg0?, arg1?, arg2?, arg3?, arg4?, arg5?
        ).unwrap()),
        Operation::ReflectY => Some(write!(wgsl,
            "reflect(vec3({}, {}, {}), vec3({}, {}, {})).y",
            arg0?, arg1?, arg2?, arg3?, arg4?, arg5?
        ).unwrap()),
        Operation::ReflectZ => Some(write!(wgsl,
            "reflect(vec3({}, {}, {}), vec3({}, {}, {})).z",
            arg0?, arg1?, arg2?, arg3?, arg4?, arg5?
        ).unwrap()),
        Operation::Floor => Some(write!(wgsl, "floor({})", arg0?).unwrap()),
        Operation::Select => Some(write!(wgsl, "mix({}, {}, vec3<f32>({}))", arg2?, arg1?, arg0?).unwrap()),
        Operation::Equal => Some(write!(wgsl, "{} == {}", arg0?, arg1?).unwrap()),
        Operation::NotEqual => Some(write!(wgsl, "{} != {}", arg0?, arg1?).unwrap()),
        Operation::Less => Some(write!(wgsl, "{} < {}", arg0?, arg1?).unwrap()),
        Operation::Greater => Some(write!(wgsl, "{} > {}", arg0?, arg1?).unwrap()),
        Operation::LessEqual => Some(write!(wgsl, "{} <= {}", arg0?, arg1?).unwrap()),
        Operation::GreaterEqual => Some(write!(wgsl, "{} >= {}", arg0?, arg1?).unwrap()),
        Operation::Dot4 => Some(write!(wgsl,
            "dot(vec4({}, {}, {}, {}), vec4({}, {}, {}, {}))",
            arg0?, arg1?, arg2?, arg3?, arg4?, arg5?, arg6?, arg7?
        ).unwrap()),
        Operation::NormalMapX => Some(write!(wgsl,
            "apply_normal_map(create_normal_map({}, {}), in.tangent.xyz, bitangent.xyz, in.normal.xyz).xxx",
            arg0?, arg1?
        ).unwrap()),
        Operation::NormalMapY => Some(write!(wgsl,
            "apply_normal_map(create_normal_map({}, {}), in.tangent.xyz, bitangent.xyz, in.normal.xyz).yyy",
            arg0?, arg1?
        ).unwrap()),
        Operation::NormalMapZ => Some(write!(wgsl,
            "apply_normal_map(create_normal_map({}, {}), in.tangent.xyz, bitangent.xyz, in.normal.xyz).zzz",
            arg0?, arg1?
        ).unwrap()),
        Operation::MonochromeX => Some(write!(wgsl,
            "monochrome_xyz_x({}, {}, {}, {})",
            arg0?, arg1?, arg2?, arg3?
        ).unwrap()),
        Operation::MonochromeY => Some(write!(wgsl,
            "monochrome_xyz_y({}, {}, {}, {})",
            arg0?, arg1?, arg2?, arg3?
        ).unwrap()),
        Operation::MonochromeZ => Some(write!(wgsl,
            "monochrome_xyz_z({}, {}, {}, {})",
            arg0?, arg1?, arg2?, arg3?
        ).unwrap()),
        Operation::Negate => Some(write!(wgsl, "-{}", arg0?).unwrap()),
        Operation::FurInstanceAlpha => Some(write!(wgsl, "in.vertex_color.a").unwrap()),
        Operation::Float => Some(write!(wgsl, "vec3<f32>({})", arg0?).unwrap()),
        Operation::Int => Some(write!(wgsl, "vec3<i32>({})", arg0?).unwrap()),
        Operation::Uint => Some(write!(wgsl, "vec3<u32>({})", arg0?).unwrap()),
        Operation::Truncate => Some(write!(wgsl, "trunc({})", arg0?).unwrap()),
        Operation::FloatBitsToInt => Some(write!(wgsl, "bitcast<i32>({})", arg0?).unwrap()),
        Operation::IntBitsToFloat => Some(write!(wgsl, "bitcast<f32>({})", arg0?).unwrap()),
        Operation::UintBitsToFloat => Some(write!(wgsl, "bitcast<f32>({})", arg0?).unwrap()),
        Operation::InverseSqrt => Some(write!(wgsl, "inverseSqrt({})", arg0?).unwrap()),
        Operation::Not => Some(write!(wgsl, "!{}", arg0?).unwrap()),
        Operation::LeftShift => Some(write!(wgsl, "{} >> {}", arg0?, arg1?).unwrap()),
        Operation::RightShift => Some(write!(wgsl, "{} >> {}", arg0?, arg1?).unwrap()),
        Operation::PartialDerivativeX => Some(write!(wgsl, "dpdx({})", arg0?).unwrap()),
        Operation::PartialDerivativeY => Some(write!(wgsl, "dpdy({})", arg0?).unwrap()),
        Operation::Exp2 => Some(write!(wgsl, "exp2({})", arg0?).unwrap()),
        Operation::Log2 => Some(write!(wgsl, "log2({})", arg0?).unwrap()),
        Operation::Sin => Some(write!(wgsl, "sin({})", arg0?).unwrap()),
        Operation::Cos => Some(write!(wgsl, "cos({})", arg0?).unwrap()),
    }
}

// TODO: Function for formatting the variable name instead?
fn arg_xyz(args: &[usize], output_index: usize, i: usize) -> Option<String> {
    Some(format!("{VAR_PREFIX_XYZ}_{output_index}_{}", args.get(i)?))
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
                let coords = texture_coordinates(&t.texcoords)?;
                let channels = channel_xyz_wgsl(t.channel);
                // TODO: Support cube maps.
                if t.texcoords.len() == 3 {
                    Some(
                        write!(
                            wgsl,
                            "textureSample(textures_d3[{i}], samplers[{i}], {coords}){channels}",
                        )
                        .unwrap(),
                    )
                } else {
                    Some(
                        write!(
                            wgsl,
                            "textureSample(textures[{i}], samplers[{i}], {coords}){channels}",
                        )
                        .unwrap(),
                    )
                }
            } else {
                error!("Sampler index {i} exceeds supported max of {TEXTURE_SAMPLER_COUNT}");
                None
            }
        }
        AssignmentValueXyz::Attribute { name, channel } => {
            // TODO: Support more attributes.
            let c = channel_xyz_wgsl(*channel);
            match name.as_str() {
                "vColor" => Some(write!(wgsl, "in.vertex_color{c}").unwrap()),
                "vPos" => Some(write!(wgsl, "in.position{c}").unwrap()),
                "vNormal" => Some(write!(wgsl, "in.normal{c}").unwrap()),
                "vTan" => Some(write!(wgsl, "in.tangent{c}").unwrap()),
                "vTex0" => Some(write!(wgsl, "tex0{c}").unwrap()),
                "vTex1" => Some(write!(wgsl, "tex1{c}").unwrap()),
                "vTex2" => Some(write!(wgsl, "tex2{c}").unwrap()),
                "vTex3" => Some(write!(wgsl, "tex3{c}").unwrap()),
                "vTex4" => Some(write!(wgsl, "tex4{c}").unwrap()),
                "vTex5" => Some(write!(wgsl, "tex5{c}").unwrap()),
                "vTex6" => Some(write!(wgsl, "tex6{c}").unwrap()),
                "vTex7" => Some(write!(wgsl, "tex7{c}").unwrap()),
                "vTex8" => Some(write!(wgsl, "tex8{c}").unwrap()),
                // The database uses "vBitan" to represent calculated bitangent attributes.
                "vBitan" => Some(write!(wgsl, "bitangent{c}").unwrap()),
                _ => {
                    warn!("Unsupported attribute {name}{c}");
                    None
                }
            }
        }
        AssignmentValueXyz::Float(f) => {
            if f.iter().all(|f| f.is_finite()) {
                Some(write!(wgsl, "vec3({:?}, {:?}, {:?})", f[0], f[1], f[2]).unwrap())
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
