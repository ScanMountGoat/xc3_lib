use std::fmt::Write;

use indexmap::IndexMap;
use log::error;
use smol_str::SmolStr;
use xc3_model::{
    material::{ChannelAssignment, LayerChannelAssignment, OutputAssignment},
    shader_database::LayerBlendMode,
};

pub fn generate_layering_code(
    assignments: &OutputAssignment,
    name_to_index: &IndexMap<SmolStr, usize>,
) -> String {
    let mut code = String::new();
    write_layers(&mut code, name_to_index, &assignments.x_layers, 'x');
    write_layers(&mut code, name_to_index, &assignments.y_layers, 'y');
    write_layers(&mut code, name_to_index, &assignments.z_layers, 'z');
    write_layers(&mut code, name_to_index, &assignments.w_layers, 'w');
    code
}

pub fn create_model_shader(output_layers_wgsl: &[String]) -> String {
    let mut source = include_str!("shader/model.wgsl").to_string();
    source = source.replace(
        "// BLEND_COLOR_LAYERS_GENERATED",
        &output_layers_wgsl[0].replace(crate::shadergen::OUT_VAR, "color"),
    );
    source = source.replace(
        "// BLEND_ETC_LAYERS_GENERATED",
        &output_layers_wgsl[1].replace(crate::shadergen::OUT_VAR, "etc_buffer"),
    );
    source = source.replace(
        "// BLEND_NORMAL_LAYERS_GENERATED",
        &output_layers_wgsl[2].replace(crate::shadergen::OUT_VAR, "normal_map"),
    );
    source
}

pub const OUT_VAR: &str = "RESULT";

fn write_layers(
    code: &mut String,
    name_to_index: &IndexMap<SmolStr, usize>,
    layers: &[LayerChannelAssignment],
    c: char,
) {
    for layer in layers {
        // TODO: How to handle missing values?
        // TODO: function to reduce nesting?
        write_layer(code, name_to_index, layer, c);
    }
}

fn write_layer(
    code: &mut String,
    name_to_index: &IndexMap<SmolStr, usize>,
    layer: &LayerChannelAssignment,
    c: char,
) -> Option<()> {
    let value = layer.value.as_ref()?;
    let b = channel_assignment_code(name_to_index, value)?;
    let mut ratio = layer
        .weight
        .as_ref()
        .and_then(|w| channel_assignment_code(name_to_index, w))?;

    if layer.is_fresnel {
        ratio = format!("fresnel_ratio({ratio}, n_dot_v)");
    }

    match layer.blend_mode {
        LayerBlendMode::Mix => {
            writeln!(code, "{OUT_VAR}.{c} = mix({OUT_VAR}.{c}, {b}, {ratio});").unwrap();
        }
        LayerBlendMode::MixRatio => {
            writeln!(
                code,
                "{OUT_VAR}.{c} = mix({OUT_VAR}.{c}, {OUT_VAR}.{c} * {b}, {ratio});"
            )
            .unwrap();
        }
        LayerBlendMode::Add => {
            writeln!(code, "{OUT_VAR}.{c} = {OUT_VAR}.{c} + {b} * {ratio};").unwrap();
        }
        LayerBlendMode::AddNormal => {
            writeln!(code, "{{").unwrap();

            // Ensure that z blending does not affect normals.
            let a = format!("vec3({OUT_VAR}.xy, normal_z({OUT_VAR}.x, {OUT_VAR}.y))");
            let (b, _) = b.split_once('.').unwrap_or((&b, ""));
            let b = format!("create_normal_map({b}.xy)");

            // Assume this mode applies to all relevant channels.
            writeln!(code, "let result = add_normal_maps({a}, {b}, {ratio});").unwrap();
            writeln!(code, "{OUT_VAR}.x = result.x;").unwrap();
            writeln!(code, "{OUT_VAR}.y = result.y;").unwrap();

            writeln!(code, "}}").unwrap();
        }
        LayerBlendMode::Overlay => {
            writeln!(
                code,
                "{OUT_VAR}.{c} = mix({OUT_VAR}.{c}, overlay_blend({OUT_VAR}.{c}, {b}), {ratio});"
            )
            .unwrap();
        }
    };
    Some(())
}

fn channel_assignment_code(
    name_to_index: &IndexMap<SmolStr, usize>,
    value: &ChannelAssignment,
) -> Option<String> {
    match value {
        ChannelAssignment::Texture(t) => name_to_index.get(&t.name).and_then(|i| {
            if *i < 10 {
                Some(format!("samplers[{i}].{}", t.channels))
            } else {
                error!("Sampler index {i} exceeds supported max of 10");
                None
            }
        }),
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
