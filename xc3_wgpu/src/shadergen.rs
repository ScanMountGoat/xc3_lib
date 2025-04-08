use std::fmt::Write;

use indexmap::IndexMap;
use indoc::formatdoc;
use log::error;
use smol_str::SmolStr;
use xc3_model::{
    material::{
        LayerAssignment, LayerAssignmentValue, OutputAssignment, TexCoordParallax,
        TextureAlphaTest, TextureAssignment, ValueAssignment,
    },
    shader_database::LayerBlendMode,
    IndexMapExt,
};

use crate::pipeline::PipelineKey;

const OUT_VAR: &str = "RESULT";

pub fn create_model_shader(key: &PipelineKey) -> String {
    let mut source = include_str!("shader/model.wgsl").to_string();

    for ((from, var), to) in [
        ("// ASSIGN_COLOR_GENERATED", "g_color"),
        ("// ASSIGN_ETC_GENERATED", "g_etc_buffer"),
        ("// ASSIGN_NORMAL_GENERATED", "normal_map"),
        ("// ASSIGN_G_VELOCITY_GENERATED", "g_velocity"),
        ("// ASSIGN_G_DEPTH_GENERATED", "g_depth"),
        ("// ASSIGN_G_LGT_COLOR_GENERATED", "g_lgt_color"),
    ]
    .iter()
    .zip(&key.output_layers_wgsl)
    {
        // TODO: This causes slow compiles and very complex shaders?
        source = source.replace(from, &to.replace(OUT_VAR, var));
    }

    source = source.replace("// UVS_GENERATED", &key.uvs_wgsl.join("\n"));

    source = source.replace("// ALPHA_TEST_DISCARD_GENERATED", &key.alpha_test_wgsl);

    source
}

fn generate_uv_wgsl(
    texture: &TextureAssignment,
    name_to_index: &mut IndexMap<SmolStr, usize>,
) -> String {
    // TODO: Select sampler for alpha testing.
    let mut wgsl = String::new();

    let i = name_to_index.entry_index(texture.name.clone());

    let parallax = texture
        .parallax
        .as_ref()
        .and_then(|p| parallax_wgsl(name_to_index, p));

    let uv = transformed_uv_wgsl(texture);
    writeln!(&mut wgsl, "uv{i} = {uv};").unwrap();

    if let Some(parallax) = parallax {
        writeln!(&mut wgsl, "uv{i} += {parallax};").unwrap();
    }

    wgsl
}

fn transformed_uv_wgsl(texture: &TextureAssignment) -> String {
    let index = texture
        .texcoord_name
        .as_deref()
        .and_then(texcoord_index)
        .unwrap_or_default();

    if let Some((u, v)) = texture.texcoord_transforms {
        let u = format!("vec4({}, {}, {}, {})", u[0], u[1], u[2], u[3]);
        let v = format!("vec4({}, {}, {}, {})", v[0], v[1], v[2], v[3]);
        format!("transform_uv(tex{index}, {u}, {v})")
    } else {
        format!("tex{index}")
    }
}

fn texcoord_index(name: &str) -> Option<u32> {
    // vTex1 -> 1
    name.strip_prefix("vTex")?.parse().ok()
}

pub fn generate_alpha_test_wgsl(
    alpha_test: &TextureAlphaTest,
    name_to_index: &mut IndexMap<SmolStr, usize>,
) -> String {
    let name: SmolStr = format!("s{}", alpha_test.texture_index).into();
    let i = name_to_index.entry_index(name.clone());

    if i < 10 {
        let c = ['x', 'y', 'z', 'w'][alpha_test.channel_index];

        formatdoc! {"
            if textureSample(s{i}, alpha_test_sampler, uv{i}).{c} <= per_material.alpha_test_ref {{
                discard;
            }}
        "}
    } else {
        error!("Sampler index {i} exceeds supported max of 10");
        String::new()
    }
}

pub fn generate_layering_wgsl(
    assignments: &OutputAssignment,
    name_to_index: &mut IndexMap<SmolStr, usize>,
    name_to_uv_wgsl: &mut IndexMap<SmolStr, String>,
) -> String {
    let mut value_to_wgsl = WgslVarCache::new(OUT_VAR.to_string());

    let mut wgsl = String::new();
    write_value_to_output(
        &mut wgsl,
        name_to_index,
        name_to_uv_wgsl,
        &mut value_to_wgsl,
        &assignments.x,
        'x',
    );
    write_value_to_output(
        &mut wgsl,
        name_to_index,
        name_to_uv_wgsl,
        &mut value_to_wgsl,
        &assignments.y,
        'y',
    );
    write_value_to_output(
        &mut wgsl,
        name_to_index,
        name_to_uv_wgsl,
        &mut value_to_wgsl,
        &assignments.z,
        'z',
    );
    write_value_to_output(
        &mut wgsl,
        name_to_index,
        name_to_uv_wgsl,
        &mut value_to_wgsl,
        &assignments.w,
        'w',
    );

    prepend_assignments(value_to_wgsl, wgsl)
}

fn prepend_assignments(value_to_wgsl: WgslVarCache<'_>, wgsl: String) -> String {
    let mut assignments = String::new();
    for WgslVar { name, wgsl } in value_to_wgsl.value_to_var.values() {
        writeln!(&mut assignments, "let {name} = {wgsl};").unwrap();
    }

    assignments + &wgsl
}

pub fn generate_normal_layering_wgsl(
    assignment: &OutputAssignment,
    name_to_index: &mut IndexMap<SmolStr, usize>,
    name_to_uv_wgsl: &mut IndexMap<SmolStr, String>,
) -> String {
    let mut wgsl = String::new();

    // XY channels need special logic since normal blend modes can affect multiple channels.
    let mut value_to_wgsl_xy = WgslVarCache::new(format!("{OUT_VAR}_xy"));
    write_normal_xy_to_output(
        &mut wgsl,
        name_to_index,
        name_to_uv_wgsl,
        assignment,
        &mut value_to_wgsl_xy,
    );

    let mut value_to_wgsl = WgslVarCache::new(OUT_VAR.to_string());
    write_value_to_output(
        &mut wgsl,
        name_to_index,
        name_to_uv_wgsl,
        &mut value_to_wgsl,
        &assignment.z,
        'z',
    );
    write_value_to_output(
        &mut wgsl,
        name_to_index,
        name_to_uv_wgsl,
        &mut value_to_wgsl,
        &assignment.z,
        'w',
    );

    wgsl = prepend_assignments(value_to_wgsl_xy, wgsl);
    prepend_assignments(value_to_wgsl, wgsl)
}

fn write_normal_xy_to_output<'a>(
    wgsl: &mut String,
    name_to_index: &mut IndexMap<SmolStr, usize>,
    name_to_uv_wgsl: &mut IndexMap<SmolStr, String>,
    assignments: &'a OutputAssignment,
    value_to_wgsl: &mut WgslVarCache<'a>,
) {
    // The database and shader analysis all use scalar values for simplicity.
    // Assume normals always blend xy values to make normal layers work.
    match (&assignments.x, &assignments.y) {
        (LayerAssignmentValue::Value(x), LayerAssignmentValue::Value(_y)) => {
            if let Some(value) =
                channel_assignment_wgsl(name_to_index, name_to_uv_wgsl, x.as_ref(), Some("xy"))
            {
                writeln!(wgsl, "{OUT_VAR}.x = {value}.x;").unwrap();
                writeln!(wgsl, "{OUT_VAR}.y = {value}.y;").unwrap();
            }
            // TODO: Find a better way to handle normal map value ranges.
            writeln!(wgsl, "{OUT_VAR}.x = create_normal_map({OUT_VAR}.xy).x;").unwrap();
            writeln!(wgsl, "{OUT_VAR}.y = create_normal_map({OUT_VAR}.xy).y;").unwrap();
        }
        (LayerAssignmentValue::Layers(x), LayerAssignmentValue::Layers(y)) => {
            for (i, (x_layer, y_layer)) in x.iter().zip(y).enumerate() {
                if x_layer.blend_mode != y_layer.blend_mode {
                    error!("o2.x and o2.y layer blend modes do not match");
                }

                if x_layer.blend_mode == LayerBlendMode::AddNormal {
                    if let Some(value) = layer_wgsl(
                        name_to_index,
                        name_to_uv_wgsl,
                        x_layer,
                        OUT_VAR,
                        Some("xy"),
                        value_to_wgsl,
                    ) {
                        writeln!(wgsl, "{OUT_VAR}.x = {value}.x;").unwrap();
                        writeln!(wgsl, "{OUT_VAR}.y = {value}.y;").unwrap();
                    }
                } else {
                    if let Some(value) = layer_wgsl(
                        name_to_index,
                        name_to_uv_wgsl,
                        x_layer,
                        &format!("{OUT_VAR}.x"),
                        Some("xy"),
                        value_to_wgsl,
                    ) {
                        writeln!(wgsl, "{OUT_VAR}.x = {value}.x;").unwrap();
                        writeln!(wgsl, "{OUT_VAR}.y = {value}.y;").unwrap();
                    }
                }

                if i == 0 {
                    // TODO: Find a better way to handle normal map value ranges.
                    writeln!(wgsl, "{OUT_VAR}.x = create_normal_map({OUT_VAR}.xy).x;").unwrap();
                    writeln!(wgsl, "{OUT_VAR}.y = create_normal_map({OUT_VAR}.xy).y;").unwrap();
                }
            }
        }
        _ => error!("o2.x and o2.y layer values do not match"),
    };
}

fn write_value_to_output<'a>(
    wgsl: &mut String,
    name_to_index: &mut IndexMap<SmolStr, usize>,
    name_to_uv_wgsl: &mut IndexMap<SmolStr, String>,
    value_to_wgsl: &mut WgslVarCache<'a>,
    value: &'a LayerAssignmentValue,
    c: char,
) {
    if let Some(value) = layer_value_wgsl(
        name_to_index,
        name_to_uv_wgsl,
        value,
        &format!("{OUT_VAR}.{c}"),
        None,
        value_to_wgsl,
    ) {
        writeln!(wgsl, "{OUT_VAR}.{c} = {value};").unwrap();
    }
}

fn layer_wgsl<'a>(
    name_to_index: &mut IndexMap<SmolStr, usize>,
    name_to_uv_wgsl: &mut IndexMap<SmolStr, String>,
    layer: &'a LayerAssignment,
    var: &str,
    channel_override: Option<&str>,
    value_to_wgsl: &mut WgslVarCache<'a>,
) -> Option<String> {
    // TODO: Skip missing values instead of using a default?
    let b = layer_value_wgsl(
        name_to_index,
        name_to_uv_wgsl,
        &layer.value,
        var,
        channel_override,
        value_to_wgsl,
    )?;

    let mut var = var.to_string();
    let mut b = b;
    if let Some(channels) = channel_override {
        var = replace_channels(&var, channels);
        b = replace_channels(&b, channels);
    }

    let mut ratio = layer_value_wgsl(
        name_to_index,
        name_to_uv_wgsl,
        &layer.weight,
        "0.0",
        None,
        value_to_wgsl,
    )?;
    if layer.is_fresnel {
        ratio = format!("fresnel_ratio({ratio}, n_dot_v)");
    }

    if ratio == "0.0" {
        return Some(var);
    }

    let result = match layer.blend_mode {
        LayerBlendMode::Mix => {
            if ratio == "1.0" {
                b
            } else {
                format!("mix({var}, {b}, {ratio})")
            }
        }
        LayerBlendMode::MixRatio => {
            if ratio == "1.0" {
                format!("({var} * {b})")
            } else {
                format!("mix({var}, {var} * {b}, {ratio})")
            }
        }
        LayerBlendMode::Add => {
            if ratio == "1.0" {
                format!("({var} + {b})")
            } else {
                format!("{var} + {b} * {ratio}")
            }
        }
        LayerBlendMode::AddNormal => {
            // Assume this mode applies to both x and y.
            // Ensure that z blending does not affect normals.
            let a_nrm = format!("vec3({var}.xy, normal_z({var}.x, {var}.y))");
            let b_nrm = format!("create_normal_map({b}.xy)");
            format!("add_normal_maps({a_nrm}, {b_nrm}, {ratio})")
        }
        LayerBlendMode::Overlay => {
            if ratio == "1.0" {
                format!("overlay_blend({var}, {b})")
            } else {
                format!("mix({var}, overlay_blend({var}, {b}), {ratio})")
            }
        }
        LayerBlendMode::Power => {
            if ratio == "1.0" {
                format!("pow({var}, {b})")
            } else {
                format!("mix({var}, pow({var}, {b}), {ratio})")
            }
        }
        LayerBlendMode::Min => format!("min({var}, {b})"),
        LayerBlendMode::Max => format!("max({var}, {b})"),
    };
    Some(result)
}

struct WgslVar {
    name: String,
    wgsl: String,
}

struct WgslVarCache<'a> {
    prefix: String,
    value_to_var: IndexMap<&'a LayerAssignmentValue, WgslVar>,
}

impl<'a> WgslVarCache<'a> {
    fn new(name: String) -> WgslVarCache<'a> {
        Self {
            prefix: name,
            value_to_var: IndexMap::new(),
        }
    }
}

fn layer_value_wgsl<'a>(
    name_to_index: &mut IndexMap<SmolStr, usize>,
    name_to_uv_wgsl: &mut IndexMap<SmolStr, String>,
    value: &'a LayerAssignmentValue,
    var: &str,
    channel_override: Option<&str>,
    value_to_wgsl: &mut WgslVarCache<'a>,
) -> Option<String> {
    match value_to_wgsl.value_to_var.get(value) {
        Some(var) => Some(var.name.clone()),
        None => {
            let wgsl = match value {
                LayerAssignmentValue::Value(value) => channel_assignment_wgsl(
                    name_to_index,
                    name_to_uv_wgsl,
                    value.as_ref(),
                    channel_override,
                ),
                LayerAssignmentValue::Layers(layers) => {
                    // Get the final assigned value after applying all layers recursively.
                    let mut output = var.to_string();
                    for layer in layers {
                        if let Some(new_output) = layer_wgsl(
                            name_to_index,
                            name_to_uv_wgsl,
                            layer,
                            &output,
                            channel_override,
                            value_to_wgsl,
                        ) {
                            output = new_output;
                        }
                    }
                    Some(output)
                }
            }?;

            // Give each variable a unique name.
            let name = format!(
                "{}_{}",
                value_to_wgsl.prefix,
                value_to_wgsl.value_to_var.len()
            );

            // Workaround for normal map XY layers.
            let wgsl = if let Some(channels) = channel_override {
                replace_channels(&wgsl, channels)
            } else {
                wgsl
            };

            value_to_wgsl.value_to_var.insert(
                value,
                WgslVar {
                    name: name.clone(),
                    wgsl,
                },
            );

            Some(name)
        }
    }
}

fn channel_assignment_wgsl(
    name_to_index: &mut IndexMap<SmolStr, usize>,
    name_to_uv_wgsl: &mut IndexMap<SmolStr, String>,
    value: Option<&ValueAssignment>,
    channel_override: Option<&str>,
) -> Option<String> {
    match value? {
        ValueAssignment::Texture(t) => {
            let i = name_to_index.entry_index(t.name.clone());

            if i < 10 {
                let uvs = generate_uv_wgsl(t, name_to_index);
                name_to_uv_wgsl.insert(t.name.clone(), uvs);

                Some(format!(
                    "s{i}_color.{}",
                    channel_override.unwrap_or_else(|| t.channels.as_str())
                ))
            } else {
                error!("Sampler index {i} exceeds supported max of 10");
                None
            }
        }
        ValueAssignment::Attribute {
            name,
            channel_index,
        } => {
            // TODO: Support attributes other than vertex color.
            // TODO: log errors
            let name = match name.as_str() {
                "vColor" => Some("in.vertex_color"),
                _ => None,
            }?;
            Some(format!(
                "{name}.{}",
                channel_override.unwrap_or_else(|| &["x", "y", "z", "w"][*channel_index])
            ))
        }
        ValueAssignment::Value(f) => match channel_override {
            Some(channels) => Some(format!("vec{}({f:?})", channels.len())),
            None => Some(format!("{f:?}")),
        },
    }
}

fn replace_channels(s: &str, c: &str) -> String {
    if s.ends_with(".x") {
        format!("{}.{c}", s.trim_end_matches(".x"))
    } else if s.ends_with(".y") {
        format!("{}.{c}", s.trim_end_matches(".y"))
    } else if s.ends_with(".z") {
        format!("{}.{c}", s.trim_end_matches(".z"))
    } else if s.ends_with(".w") {
        format!("{}.{c}", s.trim_end_matches(".w"))
    } else {
        s.to_string()
    }
}

fn parallax_wgsl(
    name_to_index: &mut IndexMap<SmolStr, usize>,
    parallax: &TexCoordParallax,
) -> Option<String> {
    let mask_a = channel_assignment_wgsl_parallax(name_to_index, Some(&parallax.mask_a))?;
    let mask_b = channel_assignment_wgsl_parallax(name_to_index, Some(&parallax.mask_b))?;
    let ratio = format!("{:?}", parallax.ratio);

    Some(format!("uv_parallax(in, {mask_a}, {mask_b}, {ratio})"))
}

fn channel_assignment_wgsl_parallax(
    name_to_index: &mut IndexMap<SmolStr, usize>,
    value: Option<&ValueAssignment>,
) -> Option<String> {
    match value? {
        ValueAssignment::Texture(t) => {
            let i = name_to_index.entry_index(t.name.clone());

            // Parallax masks affect UVs, which may themselves depend on textures.
            // Assume the masks themselves have no parallax to avoid recursion.
            // TODO: Assume textures are accessed once and adjust the order of assignment instead?X
            if t.parallax.is_some() {
                error!("Unexpected recursion when processing texture coordinate parallax");
            }

            if i < 10 {
                let uvs = transformed_uv_wgsl(t);

                Some(format!(
                    "textureSample(s{i}, s{i}_sampler, {uvs}).{}",
                    t.channels
                ))
            } else {
                error!("Sampler index {i} exceeds supported max of 10");
                None
            }
        }
        ValueAssignment::Attribute {
            name,
            channel_index,
        } => {
            // TODO: Support attributes other than vertex color.
            // TODO: log errors
            let name = match name.as_str() {
                "vColor" => Some("in.vertex_color"),
                _ => None,
            }?;
            Some(format!("{name}.{}", ['x', 'y', 'z', 'w'][*channel_index]))
        }
        ValueAssignment::Value(f) => Some(format!("{f:?}")),
    }
}

// TODO: create tests for sample shader from each game.
