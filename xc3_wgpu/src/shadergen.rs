use std::fmt::Write;

use indexmap::IndexMap;
use indoc::formatdoc;
use log::{error, warn};
use smol_str::SmolStr;
use xc3_model::{
    material::{
        assignments::{
            Assignment, AssignmentValue, AssignmentValueXyz, AssignmentXyz, ChannelXyz,
            OutputAssignment, OutputAssignmentXyz, OutputAssignments,
        },
        TextureAlphaTest,
    },
    shader_database::Operation,
    IndexMapExt,
};

use crate::shader::model::TEXTURE_SAMPLER_COUNT;

const OUT_VAR: &str = "RESULT";
const VAR_PREFIX: &str = "VAR";
const VAR_PREFIX_XYZ: &str = "VAR_XYZ";

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
        alpha_test: Option<&TextureAlphaTest>,
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
            .map(|a| generate_alpha_test_wgsl(a, name_to_index))
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
        let mut source = crate::shader::model::SOURCE.to_string();

        // TODO: use vars instead of comments.
        source = source.replace("let ASSIGN_VARS = 0.0;", &self.assignments);

        for ((from, var), to) in [
            ("let ASSIGN_COLOR_GENERATED = 0.0;", "g_color"),
            ("let ASSIGN_ETC_GENERATED = 0.0;", "g_etc_buffer"),
            ("let ASSIGN_NORMAL_GENERATED = 0.0;", "g_normal"),
            ("let ASSIGN_G_LGT_COLOR_GENERATED = 0.0;", "g_lgt_color"),
        ]
        .iter()
        .zip(&self.outputs)
        {
            source = source.replace(from, &to.replace(OUT_VAR, var));
        }

        source = source.replace("let ALPHA_TEST_DISCARD_GENERATED = 0.0;", &self.alpha_test);

        source = source.replace(
            "let ASSIGN_NORMAL_INTENSITY_GENERATED = 0.0;",
            &self.normal_intensity.replace(OUT_VAR, "intensity"),
        );

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

fn assignment_wgsl(
    value: &Assignment,
    name_to_index: &mut IndexMap<SmolStr, usize>,
) -> Option<String> {
    match value {
        Assignment::Func { op, args } => func_wgsl(op, args),
        Assignment::Value(v) => v
            .as_ref()
            .and_then(|v| assignment_value_wgsl(v, name_to_index)),
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
        Operation::MonochromeX => Some(format!(
            "monochrome({}, {}, {}, {}).x",
            arg0?, arg1?, arg2?, arg3?
        )),
        Operation::MonochromeY => Some(format!(
            "monochrome({}, {}, {}, {}).y",
            arg0?, arg1?, arg2?, arg3?
        )),
        Operation::MonochromeZ => Some(format!(
            "monochrome({}, {}, {}, {}).z",
            arg0?, arg1?, arg2?, arg3?
        )),
    }
}

fn arg(args: &[usize], i: usize) -> Option<String> {
    Some(format!("{VAR_PREFIX}{}", args.get(i)?))
}

fn generate_alpha_test_wgsl(
    alpha_test: &TextureAlphaTest,
    name_to_index: &mut IndexMap<SmolStr, usize>,
) -> String {
    let name: SmolStr = format!("s{}", alpha_test.texture_index).into();
    let i = name_to_index.entry_index(name.clone());

    if i < TEXTURE_SAMPLER_COUNT as usize {
        let c = ['x', 'y', 'z', 'w'][alpha_test.channel_index];

        // TODO: Detect the UV attribute to use with alpha testing.
        formatdoc! {"
            if textureSample(textures[{i}], alpha_test_sampler, tex0).{c} <= per_material.alpha_test_ref {{
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
        let value_wgsl = assignment_wgsl(value, name_to_index);
        writeln!(
            wgsl,
            "let {VAR_PREFIX}{i} = {};",
            value_wgsl.unwrap_or("0.0".to_string())
        )
        .unwrap();
    }

    // TODO: Share xyz assignments with all channels?
    for (i, assignment) in xyz_assignments.iter().enumerate() {
        if let Some(assignment) = assignment {
            for (j, value) in assignment.assignments.iter().enumerate() {
                let value_wgsl = assignment_xyz_wgsl(value, i, name_to_index);
                writeln!(
                    wgsl,
                    "let {VAR_PREFIX_XYZ}_{i}_{j} = {};",
                    value_wgsl.unwrap_or("vec3(0.0)".to_string())
                )
                .unwrap();
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

fn assignment_value_wgsl(
    value: &AssignmentValue,
    name_to_index: &mut IndexMap<SmolStr, usize>,
) -> Option<String> {
    match value {
        AssignmentValue::Texture(t) => {
            let i = name_to_index.entry_index(t.name.clone());

            if i < TEXTURE_SAMPLER_COUNT as usize {
                let coords = texture_coordinates(&t.texcoords)?;
                let channels = channel_wgsl(t.channel);
                // TODO: Support cube maps.
                if t.texcoords.len() == 3 {
                    Some(format!(
                        "textureSample(textures_d3[{i}], samplers[{i}], {coords}){channels}",
                    ))
                } else {
                    Some(format!(
                        "textureSample(textures[{i}], samplers[{i}], {coords}){channels}",
                    ))
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

fn assignment_xyz_wgsl(
    value: &AssignmentXyz,
    output_index: usize,
    name_to_index: &mut IndexMap<SmolStr, usize>,
) -> Option<String> {
    match value {
        AssignmentXyz::Func { op, args } => func_xyz_wgsl(op, args, output_index),
        AssignmentXyz::Value(v) => v
            .as_ref()
            .and_then(|v| assignment_value_xyz_wgsl(v, name_to_index)),
    }
}

fn func_xyz_wgsl(op: &Operation, args: &[usize], output_index: usize) -> Option<String> {
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
            "mix({0}, overlay_blend_xyz({0}, {1}), {2})",
            arg0?, arg1?, arg2?
        )),
        Operation::Overlay => Some(format!("overlay_blend_xyz({}, {})", arg0?, arg1?)),
        Operation::Overlay2 => Some(format!("overlay_blend2_xyz({}, {})", arg0?, arg1?)),
        Operation::Power => Some(format!("pow({}, {})", arg0?, arg1?)),
        Operation::Min => Some(format!("min({}, {})", arg0?, arg1?)),
        Operation::Max => Some(format!("max({}, {})", arg0?, arg1?)),
        Operation::Clamp => Some(format!("clamp({}, {}, {})", arg0?, arg1?, arg2?)),
        Operation::Sub => Some(format!("{} - {}", arg0?, arg1?)),
        Operation::Fma => Some(format!("{} * {} + {}", arg0?, arg1?, arg2?)),
        Operation::Abs => Some(format!("abs({})", arg0?)),
        Operation::Fresnel => Some(format!("fresnel_ratio_xyz({}, n_dot_v)", arg0?)),
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
        Operation::Select => Some(format!("mix({}, {}, vec3<f32>({}))", arg2?, arg1?, arg0?)),
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
            "apply_normal_map(create_normal_map({}, {}), in.tangent.xyz, bitangent.xyz, in.normal.xyz).xxx",
            arg0?, arg1?
        )),
        Operation::NormalMapY => Some(format!(
            "apply_normal_map(create_normal_map({}, {}), in.tangent.xyz, bitangent.xyz, in.normal.xyz).yyy",
            arg0?, arg1?
        )),
        Operation::NormalMapZ => Some(format!(
            "apply_normal_map(create_normal_map({}, {}), in.tangent.xyz, bitangent.xyz, in.normal.xyz).zzz",
            arg0?, arg1?
        )),
        Operation::MonochromeX => Some(format!(
            "monochrome_xyz_x({}, {}, {}, {})",
            arg0?, arg1?, arg2?, arg3?
        )),
        Operation::MonochromeY => Some(format!(
            "monochrome_xyz_y({}, {}, {}, {})",
            arg0?, arg1?, arg2?, arg3?
        )),
        Operation::MonochromeZ => Some(format!(
            "monochrome_xyz_z({}, {}, {}, {})",
            arg0?, arg1?, arg2?, arg3?
        )),
    }
}

// TODO: Function for formatting the variable name instead?
fn arg_xyz(args: &[usize], output_index: usize, i: usize) -> Option<String> {
    Some(format!("{VAR_PREFIX_XYZ}_{output_index}_{}", args.get(i)?))
}

fn assignment_value_xyz_wgsl(
    value: &AssignmentValueXyz,
    name_to_index: &mut IndexMap<SmolStr, usize>,
) -> Option<String> {
    match value {
        AssignmentValueXyz::Texture(t) => {
            let i = name_to_index.entry_index(t.name.clone());

            if i < TEXTURE_SAMPLER_COUNT as usize {
                let coords = texture_coordinates(&t.texcoords)?;
                let channels = channel_xyz_wgsl(t.channel);
                // TODO: Support cube maps.
                if t.texcoords.len() == 3 {
                    Some(format!(
                        "textureSample(textures_d3[{i}], samplers[{i}], {coords}){channels}",
                    ))
                } else {
                    Some(format!(
                        "textureSample(textures[{i}], samplers[{i}], {coords}){channels}",
                    ))
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
        AssignmentValueXyz::Float(f) => {
            if f.iter().all(|f| f.is_finite()) {
                Some(format!("vec3({:?}, {:?}, {:?})", f[0], f[1], f[2]))
            } else {
                error!("Unsupported float literals {f:?}");
                None
            }
        }
    }
}

fn channel_xyz_wgsl(c: Option<ChannelXyz>) -> String {
    c.map(|c| match c {
        ChannelXyz::Xyz => ".xyz".to_string(),
        ChannelXyz::X => ".xxx".to_string(),
        ChannelXyz::Y => ".yyy".to_string(),
        ChannelXyz::Z => ".zzz".to_string(),
        ChannelXyz::W => ".www".to_string(),
    })
    .unwrap_or_default()
}
