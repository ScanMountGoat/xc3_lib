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

/// Static single assignment (SSA) representation for [LayerAssignmentValue]
/// where each [NodeValue] represents a single assignment for that node index.
/// This results in less generated code by reusing intermediate values.
#[derive(Debug, Default)]
struct Nodes {
    nodes: Vec<NodeValue>,
    values: Vec<ValueAssignment>,
    value_to_node_index: IndexMap<LayerAssignmentValue, usize>,
}

#[derive(Debug)]
enum NodeValue {
    Layer {
        a_node_index: usize,
        b_node_index: usize,
        ratio_node_index: usize,
        blend_model: LayerBlendMode,
        is_fresnel: bool,
    },
    Value(usize), // TODO: just store the value directly?
}

impl Nodes {
    fn insert_layer_value(&mut self, layer_value: &LayerAssignmentValue) -> usize {
        match self.value_to_node_index.get(layer_value) {
            Some(i) => *i,
            None => {
                match layer_value {
                    LayerAssignmentValue::Value(v) => {
                        // TODO: how to handle missing values?
                        let v = v.clone().unwrap_or(ValueAssignment::Value(0.0.into()));
                        let value_index = self.insert_value(v);
                        let node = NodeValue::Value(value_index);

                        let i = self.nodes.len();
                        self.value_to_node_index.insert(layer_value.clone(), i);
                        self.nodes.push(node);
                        i
                    }
                    LayerAssignmentValue::Layers(layers) => {
                        // TODO: always blend with previous node?
                        let mut i = self.nodes.len().saturating_sub(1);

                        for layer in layers {
                            // Insert values that this value depends on first.
                            let b_node_index = self.insert_layer_value(&layer.value);
                            let ratio_node_index = self.insert_layer_value(&layer.weight);

                            let node = NodeValue::Layer {
                                a_node_index: i,
                                b_node_index,
                                ratio_node_index,
                                blend_model: layer.blend_mode,
                                is_fresnel: layer.is_fresnel,
                            };

                            i = self.nodes.len();
                            self.value_to_node_index.insert(layer_value.clone(), i);
                            self.nodes.push(node);
                        }

                        i
                    }
                }
            }
        }
    }

    fn insert_value(&mut self, value: ValueAssignment) -> usize {
        match self.values.iter().position(|v| v == &value) {
            Some(i) => i,
            None => {
                let i = self.values.len();
                self.values.push(value);
                i
            }
        }
    }

    fn write_wgsl(
        &self,
        wgsl: &mut String,
        node_prefix: &str,
        name_to_index: &mut IndexMap<SmolStr, usize>,
        name_to_uv_wgsl: &mut IndexMap<SmolStr, String>,
    ) {
        for (i, value) in self.nodes.iter().enumerate() {
            let value_wgsl = self.node_wgsl(value, node_prefix, name_to_index, name_to_uv_wgsl);
            writeln!(
                wgsl,
                "let {node_prefix}{i} = {};",
                value_wgsl.unwrap_or("0.0".to_string())
            )
            .unwrap();
        }
    }

    fn node_wgsl(
        &self,
        value: &NodeValue,
        node_prefix: &str,
        name_to_index: &mut IndexMap<SmolStr, usize>,
        name_to_uv_wgsl: &mut IndexMap<SmolStr, String>,
    ) -> Option<String> {
        match value {
            NodeValue::Layer {
                a_node_index,
                b_node_index,
                ratio_node_index,
                blend_model: blend_mode,
                is_fresnel,
            } => {
                let a = format!("{node_prefix}{a_node_index}");
                let b = format!("{node_prefix}{b_node_index}");
                let ratio = if *is_fresnel {
                    format!("fresnel_ratio({node_prefix}{ratio_node_index}, n_dot_v)")
                } else {
                    format!("{node_prefix}{ratio_node_index}")
                };

                let result = match blend_mode {
                    LayerBlendMode::Mix => format!("mix({a}, {b}, {ratio})"),
                    LayerBlendMode::Mul => format!("mix({a}, {a} * {b}, {ratio})"),
                    LayerBlendMode::Add => format!("{a} + {b} * {ratio}"),
                    LayerBlendMode::AddNormal => todo!(), // TODO: how to handle this?
                    LayerBlendMode::Overlay2 => {
                        format!("mix({a}, overlay_blend({a}, {b}), {ratio})")
                    }
                    LayerBlendMode::Overlay => {
                        format!("mix({a}, overlay_blend2({a}, {b}), {ratio})")
                    }
                    LayerBlendMode::Power => format!("mix({a}, pow({a}, {b}), {ratio})"),
                    LayerBlendMode::Min => format!("mix({a}, min({a}, {b}), {ratio})"),
                    LayerBlendMode::Max => format!("mix({a}, max({a}, {b}), {ratio})"),
                };
                Some(result)
            }
            NodeValue::Value(i) => {
                channel_assignment_wgsl(name_to_index, name_to_uv_wgsl, Some(&self.values[*i]))
            }
        }
    }
}

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
    assignment: &OutputAssignment,
    name_to_index: &mut IndexMap<SmolStr, usize>,
    name_to_uv_wgsl: &mut IndexMap<SmolStr, String>,
) -> String {
    let mut wgsl = String::new();

    // TODO: Share this cache with all outputs.
    let mut nodes = Nodes::default();

    let x_index = insert_assignment(&mut nodes, &assignment.x);
    let y_index = insert_assignment(&mut nodes, &assignment.y);
    let z_index = insert_assignment(&mut nodes, &assignment.z);
    let w_index = insert_assignment(&mut nodes, &assignment.w);

    let node_prefix = format!("{OUT_VAR}_n");
    nodes.write_wgsl(&mut wgsl, &node_prefix, name_to_index, name_to_uv_wgsl);

    // Write any final assignments.
    if let Some(x) = x_index {
        writeln!(&mut wgsl, "{OUT_VAR}.x = {node_prefix}{x};").unwrap();
    }
    if let Some(y) = y_index {
        writeln!(&mut wgsl, "{OUT_VAR}.y = {node_prefix}{y};").unwrap();
    }
    if let Some(z) = z_index {
        writeln!(&mut wgsl, "{OUT_VAR}.z = {node_prefix}{z};").unwrap();
    }
    if let Some(w) = w_index {
        writeln!(&mut wgsl, "{OUT_VAR}.w = {node_prefix}{w};").unwrap();
    }

    wgsl
}

fn insert_assignment(nodes: &mut Nodes, value: &LayerAssignmentValue) -> Option<usize> {
    if *value != LayerAssignmentValue::Value(None) {
        Some(nodes.insert_layer_value(value))
    } else {
        None
    }
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
    let mut value_to_wgsl = WgslVarCache::new(format!("{OUT_VAR}"));
    write_normal_xy_to_output(
        &mut wgsl,
        name_to_index,
        name_to_uv_wgsl,
        assignment,
        &mut value_to_wgsl,
    );

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
        &assignment.w,
        'w',
    );

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
        (LayerAssignmentValue::Value(x), LayerAssignmentValue::Value(y)) => {
            if let Some(value) = channel_assignment_wgsl(name_to_index, name_to_uv_wgsl, x.as_ref())
            {
                writeln!(wgsl, "{OUT_VAR}.x = {value};").unwrap();
            }
            if let Some(value) = channel_assignment_wgsl(name_to_index, name_to_uv_wgsl, y.as_ref())
            {
                writeln!(wgsl, "{OUT_VAR}.y = {value};").unwrap();
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
                    // Handle multi channel blend modes manually since the layers use scalar values.
                    if let (Some(x_value), Some(y_value), Some(ratio)) = (
                        layer_value_wgsl(
                            name_to_index,
                            name_to_uv_wgsl,
                            &x_layer.value,
                            "0.0",
                            value_to_wgsl,
                        ),
                        layer_value_wgsl(
                            name_to_index,
                            name_to_uv_wgsl,
                            &y_layer.value,
                            "0.0",
                            value_to_wgsl,
                        ),
                        layer_value_wgsl(
                            name_to_index,
                            name_to_uv_wgsl,
                            &x_layer.weight,
                            "0.0",
                            value_to_wgsl,
                        ),
                    ) {
                        let a_nrm =
                            format!("vec3({OUT_VAR}.xy, normal_z({OUT_VAR}.x, {OUT_VAR}.y))");
                        let b_nrm = format!("create_normal_map(vec2({x_value}, {y_value}))");
                        writeln!(
                            wgsl,
                            "{OUT_VAR}.x = add_normal_maps({a_nrm}, {b_nrm}, {ratio}).x;"
                        )
                        .unwrap();
                        writeln!(
                            wgsl,
                            "{OUT_VAR}.y = add_normal_maps({a_nrm}, {b_nrm}, {ratio}).y;"
                        )
                        .unwrap();
                    }
                } else {
                    if let Some(value) = layer_wgsl(
                        name_to_index,
                        name_to_uv_wgsl,
                        x_layer,
                        "0.0",
                        value_to_wgsl,
                    ) {
                        writeln!(wgsl, "{OUT_VAR}.x = {value};").unwrap();
                    }
                    if let Some(value) = layer_wgsl(
                        name_to_index,
                        name_to_uv_wgsl,
                        y_layer,
                        "0.0",
                        value_to_wgsl,
                    ) {
                        writeln!(wgsl, "{OUT_VAR}.y = {value};").unwrap();
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
    value_to_wgsl: &mut WgslVarCache<'a>,
) -> Option<String> {
    // TODO: Skip missing values instead of using a default?
    let b = layer_value_wgsl(
        name_to_index,
        name_to_uv_wgsl,
        &layer.value,
        var,
        value_to_wgsl,
    )?;

    let mut ratio = layer_value_wgsl(
        name_to_index,
        name_to_uv_wgsl,
        &layer.weight,
        "0.0",
        value_to_wgsl,
    )?;
    if layer.is_fresnel {
        ratio = format!("fresnel_ratio({ratio}, n_dot_v)");
    }

    if ratio == "0.0" {
        return Some(var.to_string());
    }

    let result = match layer.blend_mode {
        LayerBlendMode::Mix => {
            if ratio == "1.0" {
                b
            } else {
                format!("mix({var}, {b}, {ratio})")
            }
        }
        LayerBlendMode::Mul => {
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
            // TODO: Always handle this separately for normals?
            todo!()
        }
        LayerBlendMode::Overlay2 => {
            if ratio == "1.0" {
                format!("overlay_blend2({var}, {b})")
            } else {
                format!("mix({var}, overlay_blend2({var}, {b}), {ratio})")
            }
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
    value_to_wgsl: &mut WgslVarCache<'a>,
) -> Option<String> {
    match value_to_wgsl.value_to_var.get(value) {
        Some(var) => Some(var.name.clone()),
        None => {
            let wgsl = match value {
                LayerAssignmentValue::Value(value) => {
                    channel_assignment_wgsl(name_to_index, name_to_uv_wgsl, value.as_ref())
                }
                LayerAssignmentValue::Layers(layers) => {
                    // Get the final assigned value after applying all layers recursively.
                    let mut output = var.to_string();
                    for layer in layers {
                        if let Some(new_output) = layer_wgsl(
                            name_to_index,
                            name_to_uv_wgsl,
                            layer,
                            &output,
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
) -> Option<String> {
    match value? {
        ValueAssignment::Texture(t) => {
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
            Some(format!("{name}.{}", ["x", "y", "z", "w"][*channel_index]))
        }
        ValueAssignment::Value(f) => Some(format!("{f:?}")),
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
