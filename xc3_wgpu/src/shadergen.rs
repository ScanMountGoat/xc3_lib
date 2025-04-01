use std::fmt::Write;

use indexmap::IndexMap;
use indoc::writedoc;
use log::error;
use smol_str::SmolStr;
use xc3_model::{
    material::{ChannelAssignment, LayerChannelAssignment, OutputAssignment, TextureAlphaTest},
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

    source = source.replace("// ALPHA_TEST_DISCARD_GENERATED", &key.alpha_test_wgsl);
    source
}

pub fn generate_alpha_test_wgsl(
    alpha_test: &TextureAlphaTest,
    name_to_index: &IndexMap<SmolStr, usize>,
) -> String {
    // TODO: Select sampler for alpha testing.
    let mut wgsl = String::new();

    let name: SmolStr = format!("s{}", alpha_test.texture_index).into();
    let i = name_to_index[&name];
    let c = ['x', 'y', 'z', 'w'][alpha_test.channel_index];

    writedoc!(
        &mut wgsl,
        "
        if s{i}_color.{c} <= per_material.alpha_test_ref {{
            discard;
        }}
        "
    )
    .unwrap();

    wgsl
}

pub fn generate_assignment_wgsl(
    assignments: &OutputAssignment,
    name_to_index: &mut IndexMap<SmolStr, usize>,
) -> String {
    let mut wgsl = String::new();
    if let Some(value) = channel_assignment_wgsl(name_to_index, assignments.x.as_ref()) {
        writeln!(&mut wgsl, "{OUT_VAR}.x = {value};").unwrap();
    }
    if let Some(value) = channel_assignment_wgsl(name_to_index, assignments.y.as_ref()) {
        writeln!(&mut wgsl, "{OUT_VAR}.y = {value};").unwrap();
    }
    if let Some(value) = channel_assignment_wgsl(name_to_index, assignments.z.as_ref()) {
        writeln!(&mut wgsl, "{OUT_VAR}.z = {value};").unwrap();
    }
    if let Some(value) = channel_assignment_wgsl(name_to_index, assignments.w.as_ref()) {
        writeln!(&mut wgsl, "{OUT_VAR}.w = {value};").unwrap();
    }
    wgsl
}

pub fn generate_layering_wgsl(
    assignments: &OutputAssignment,
    name_to_index: &mut IndexMap<SmolStr, usize>,
) -> String {
    let mut wgsl = String::new();
    write_layers(&mut wgsl, name_to_index, &assignments.x_layers, 'x');
    write_layers(&mut wgsl, name_to_index, &assignments.y_layers, 'y');
    write_layers(&mut wgsl, name_to_index, &assignments.z_layers, 'z');
    write_layers(&mut wgsl, name_to_index, &assignments.w_layers, 'w');
    wgsl
}

fn write_layers(
    wgsl: &mut String,
    name_to_index: &mut IndexMap<SmolStr, usize>,
    layers: &[LayerChannelAssignment],
    c: char,
) {
    for layer in layers {
        // TODO: How to handle missing values?
        // TODO: function to reduce nesting?
        write_layer(wgsl, name_to_index, layer, c);
    }
}

fn write_layer(
    wgsl: &mut String,
    name_to_index: &mut IndexMap<SmolStr, usize>,
    layer: &LayerChannelAssignment,
    c: char,
) -> Option<()> {
    let b = channel_assignment_wgsl(name_to_index, layer.value.as_ref())?;

    let mut ratio = channel_assignment_wgsl(name_to_index, layer.weight.as_ref())?;
    if layer.is_fresnel {
        ratio = format!("fresnel_ratio({ratio}, n_dot_v)");
    }

    match layer.blend_mode {
        LayerBlendMode::Mix => {
            writeln!(wgsl, "{OUT_VAR}.{c} = mix({OUT_VAR}.{c}, {b}, {ratio});").unwrap();
        }
        LayerBlendMode::MixRatio => {
            writeln!(
                wgsl,
                "{OUT_VAR}.{c} = mix({OUT_VAR}.{c}, {OUT_VAR}.{c} * {b}, {ratio});"
            )
            .unwrap();
        }
        LayerBlendMode::Add => {
            writeln!(wgsl, "{OUT_VAR}.{c} = {OUT_VAR}.{c} + {b} * {ratio};").unwrap();
        }
        LayerBlendMode::AddNormal => {
            let (b, _) = b.split_once('.').unwrap_or((&b, ""));

            // Assume this mode applies to all relevant channels.
            // Ensure that z blending does not affect normals.
            writedoc!(
                wgsl,
                "
                {{
                    let a_nrm = vec3({OUT_VAR}.xy, normal_z({OUT_VAR}.x, {OUT_VAR}.y));
                    let b_nrm = create_normal_map({b}.xy);
                    let result = add_normal_maps(a_nrm, b_nrm, {ratio});
                    {OUT_VAR}.x = result.x;
                    {OUT_VAR}.y = result.y;
                }}
                "
            )
            .unwrap();
        }
        LayerBlendMode::Overlay => {
            writeln!(
                wgsl,
                "{OUT_VAR}.{c} = mix({OUT_VAR}.{c}, overlay_blend({OUT_VAR}.{c}, {b}), {ratio});"
            )
            .unwrap();
        }
    };
    Some(())
}

fn channel_assignment_wgsl(
    name_to_index: &mut IndexMap<SmolStr, usize>,
    value: Option<&ChannelAssignment>,
) -> Option<String> {
    match value? {
        ChannelAssignment::Texture(t) => {
            let i = name_to_index.entry_index(t.name.clone());
            if i < 10 {
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

// TODO: create tests for sample shader from each game.
