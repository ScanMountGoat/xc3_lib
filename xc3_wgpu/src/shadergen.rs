use std::fmt::Write;

use indexmap::IndexMap;
use indoc::{formatdoc, writedoc};
use log::error;
use smol_str::SmolStr;
use xc3_model::{
    material::{
        ChannelAssignment, LayerChannelAssignment, LayerChannelAssignmentValue, OutputAssignment,
        TexCoordParallax, TextureAlphaTest, TextureAssignment,
    },
    shader_database::LayerBlendMode,
    IndexMapExt,
};

use crate::pipeline::PipelineKey;

const OUT_VAR: &str = "RESULT";

pub fn create_model_shader(key: &PipelineKey) -> String {
    let mut source = include_str!("shader/model.wgsl").to_string();

    for ((from, var), to) in [
        ("// ASSIGN_G_COLOR_GENERATED", "g_color"),
        ("// ASSIGN_G_ETC_BUFFER_GENERATED", "g_etc_buffer"),
        ("// ASSIGN_G_NORMAL_GENERATED", "g_normal"),
        ("// ASSIGN_G_VELOCITY_GENERATED", "g_velocity"),
        ("// ASSIGN_G_DEPTH_GENERATED", "g_depth"),
        ("// ASSIGN_G_LGT_COLOR_GENERATED", "g_lgt_color"),
    ]
    .iter()
    .zip(&key.output_assignments_wgsl)
    {
        source = source.replace(from, &to.replace(OUT_VAR, var));
    }

    for ((from, var), to) in [
        ("// BLEND_COLOR_LAYERS_GENERATED", "color"),
        ("// BLEND_ETC_LAYERS_GENERATED", "etc_buffer"),
        ("// BLEND_NORMAL_LAYERS_GENERATED", "normal_map"),
    ]
    .iter()
    .zip(&key.output_layers_wgsl)
    {
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

pub fn generate_assignment_wgsl(
    assignments: &OutputAssignment,
    name_to_index: &mut IndexMap<SmolStr, usize>,
    name_to_uv_wgsl: &mut IndexMap<SmolStr, String>,
) -> String {
    let mut wgsl = String::new();
    if let Some(value) =
        channel_assignment_wgsl(name_to_index, name_to_uv_wgsl, assignments.x.as_ref())
    {
        writeln!(&mut wgsl, "{OUT_VAR}.x = {value};").unwrap();
    }
    if let Some(value) =
        channel_assignment_wgsl(name_to_index, name_to_uv_wgsl, assignments.y.as_ref())
    {
        writeln!(&mut wgsl, "{OUT_VAR}.y = {value};").unwrap();
    }
    if let Some(value) =
        channel_assignment_wgsl(name_to_index, name_to_uv_wgsl, assignments.z.as_ref())
    {
        writeln!(&mut wgsl, "{OUT_VAR}.z = {value};").unwrap();
    }
    if let Some(value) =
        channel_assignment_wgsl(name_to_index, name_to_uv_wgsl, assignments.w.as_ref())
    {
        writeln!(&mut wgsl, "{OUT_VAR}.w = {value};").unwrap();
    }
    wgsl
}

pub fn generate_layering_wgsl(
    assignments: &OutputAssignment,
    name_to_index: &mut IndexMap<SmolStr, usize>,
    name_to_uv_wgsl: &mut IndexMap<SmolStr, String>,
) -> String {
    let mut wgsl = String::new();
    write_layers(
        &mut wgsl,
        name_to_index,
        name_to_uv_wgsl,
        &assignments.x_layers,
        'x',
    );
    write_layers(
        &mut wgsl,
        name_to_index,
        name_to_uv_wgsl,
        &assignments.y_layers,
        'y',
    );
    write_layers(
        &mut wgsl,
        name_to_index,
        name_to_uv_wgsl,
        &assignments.z_layers,
        'z',
    );
    write_layers(
        &mut wgsl,
        name_to_index,
        name_to_uv_wgsl,
        &assignments.w_layers,
        'w',
    );
    wgsl
}

fn write_layers(
    wgsl: &mut String,
    name_to_index: &mut IndexMap<SmolStr, usize>,
    name_to_uv_wgsl: &mut IndexMap<SmolStr, String>,
    layers: &[LayerChannelAssignment],
    c: char,
) {
    // TODO: properly handle variable assignments.
    for layer in layers {
        // TODO: How to handle missing values?
        write_layer(
            wgsl,
            name_to_index,
            name_to_uv_wgsl,
            layer,
            &format!("{OUT_VAR}.{c}"),
        );
    }
}

fn write_layer(
    wgsl: &mut String,
    name_to_index: &mut IndexMap<SmolStr, usize>,
    name_to_uv_wgsl: &mut IndexMap<SmolStr, String>,
    layer: &LayerChannelAssignment,
    var: &str,
) -> Option<String> {
    let b = match &layer.value {
        LayerChannelAssignmentValue::Value(value) => {
            channel_assignment_wgsl(name_to_index, name_to_uv_wgsl, value.as_ref())
        }
        LayerChannelAssignmentValue::Layers(layers) => {
            // Layers can be defined recursively.
            let mut output = None;
            for (i, layer) in layers.iter().enumerate() {
                let layer_var = format!("{}_{i}", var.replace('.', "_"));
                let mut layer_wgsl = String::new();
                if let Some(new_output) = write_layer(
                    &mut layer_wgsl,
                    name_to_index,
                    name_to_uv_wgsl,
                    layer,
                    &layer_var,
                ) {
                    writeln!(wgsl, "var {layer_var} = 0.0;").unwrap();
                    writeln!(wgsl, "{layer_wgsl}").unwrap();
                    output = Some(new_output);
                }
            }
            // Get the final assigned value.
            output
        }
    }?;

    let mut ratio = channel_assignment_wgsl(name_to_index, name_to_uv_wgsl, layer.weight.as_ref())?;
    if layer.is_fresnel {
        ratio = format!("fresnel_ratio({ratio}, n_dot_v)");
    }

    match layer.blend_mode {
        LayerBlendMode::Mix => {
            writeln!(wgsl, "{var} = mix({var}, {b}, {ratio});").unwrap();
        }
        LayerBlendMode::MixRatio => {
            writeln!(wgsl, "{var} = mix({var}, {var} * {b}, {ratio});").unwrap();
        }
        LayerBlendMode::Add => {
            writeln!(wgsl, "{var} = {var} + {b} * {ratio};").unwrap();
        }
        LayerBlendMode::AddNormal => {
            let (var, _) = var.split_once('.').unwrap_or((&var, ""));
            let (b, _) = b.split_once('.').unwrap_or((&b, ""));

            // Assume this mode applies to all relevant channels.
            // Ensure that z blending does not affect normals.
            writedoc!(
                wgsl,
                "
                {{
                    let a_nrm = vec3({var}.xy, normal_z({var}.x, {var}.y));
                    let b_nrm = create_normal_map({b}.xy);
                    let result = add_normal_maps(a_nrm, b_nrm, {ratio});
                    {var}.x = result.x;
                    {var}.y = result.y;
                }}
                "
            )
            .unwrap();
        }
        LayerBlendMode::Overlay => {
            writeln!(
                wgsl,
                "{var} = mix({var}, overlay_blend({var}, {b}), {ratio});"
            )
            .unwrap();
        }
        LayerBlendMode::Power => {
            writeln!(wgsl, "{var} = pow({var}, {b});").unwrap();
        }
    };

    Some(var.to_string())
}

fn channel_assignment_wgsl(
    name_to_index: &mut IndexMap<SmolStr, usize>,
    name_to_uv_wgsl: &mut IndexMap<SmolStr, String>,
    value: Option<&ChannelAssignment>,
) -> Option<String> {
    match value? {
        ChannelAssignment::Texture(t) => {
            let i = name_to_index.entry_index(t.name.clone());

            if i < 10 {
                let uvs = generate_uv_wgsl(t, name_to_index);
                name_to_uv_wgsl.insert(t.name.clone(), uvs);

                Some(format!("s{i}_color.{}", t.channels))
            } else {
                error!("Sampler index {i} exceeds supported max of 10");
                None
            }
        }
        ChannelAssignment::Attribute {
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
        ChannelAssignment::Value(f) => Some(format!("{f:?}")),
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
    value: Option<&ChannelAssignment>,
) -> Option<String> {
    match value? {
        ChannelAssignment::Texture(t) => {
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
        ChannelAssignment::Attribute {
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
        ChannelAssignment::Value(f) => Some(format!("{f:?}")),
    }
}

// TODO: create tests for sample shader from each game.
