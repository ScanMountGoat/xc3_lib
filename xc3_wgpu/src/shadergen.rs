use std::fmt::Write;

use indexmap::IndexMap;
use indoc::formatdoc;
use log::error;
use smol_str::SmolStr;
use xc3_model::{
    material::{
        LayerAssignmentValue, OutputAssignment, TexCoordParallax, TextureAlphaTest,
        TextureAssignment, ValueAssignment,
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
        blend_mode: LayerBlendMode,
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
                        if layers.is_empty() {
                            // Avoid empty layers that cause problems with code gen.
                            let value_index = self.insert_value(ValueAssignment::Value(0.0.into()));
                            let node = NodeValue::Value(value_index);

                            let i = self.nodes.len();
                            self.value_to_node_index.insert(layer_value.clone(), i);
                            self.nodes.push(node);
                            i
                        } else {
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
                                    blend_mode: layer.blend_mode,
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

    fn write_wgsl_xy(
        wgsl: &mut String,
        nodes_x: &Self,
        nodes_y: &Self,
        prefix: &str,
        name_to_index: &mut IndexMap<SmolStr, usize>,
        name_to_uv_wgsl: &mut IndexMap<SmolStr, String>,
    ) -> Option<(String, String)> {
        let prefix_x = format!("{prefix}_x");
        let prefix_y = format!("{prefix}_y");

        let mut final_xy = None;

        // Blend modes that use multiple channels require special handling.
        // Interleave x and y channel assignments to enable blending both channels.
        // This assumes the database xy entries differ only in the accessed channel.
        for (i, (value_x, value_y)) in nodes_x.nodes.iter().zip(&nodes_y.nodes).enumerate() {
            match (value_x, value_y) {
                (
                    NodeValue::Layer {
                        a_node_index: ax,
                        b_node_index: bx,
                        ratio_node_index: rx,
                        blend_mode: LayerBlendMode::AddNormal,
                        is_fresnel: fx,
                    },
                    NodeValue::Layer {
                        a_node_index: ay,
                        b_node_index: by,
                        ratio_node_index: _ry,
                        blend_mode: LayerBlendMode::AddNormal,
                        is_fresnel: _fy,
                    },
                ) => {
                    // TODO: check that ratios and fresnel match.
                    let r = if *fx {
                        format!("fresnel_ratio({prefix_x}{rx}, n_dot_v)")
                    } else {
                        format!("{prefix_x}{rx}")
                    };

                    let a_nrm = format!("vec3({prefix_x}{ax}, {prefix_y}{ay}, normal_z({prefix_x}{ax}, {prefix_y}{ay}))");
                    let b_nrm = format!("create_normal_map({prefix_x}{bx}, {prefix_y}{by})");
                    writeln!(
                        wgsl,
                        "let {prefix}_xy{i} = add_normal_maps({a_nrm}, {b_nrm}, {r});",
                    )
                    .unwrap();

                    let x_value = format!("{prefix}_xy{i}.x");
                    let y_value = format!("{prefix}_xy{i}.y");
                    writeln!(wgsl, "let {prefix_x}{i} = {x_value};",).unwrap();
                    writeln!(wgsl, "let {prefix_y}{i} = {y_value};",).unwrap();
                    final_xy = Some((x_value, y_value));
                }
                _ => {
                    let value1_wgsl =
                        nodes_x.node_wgsl(value_x, &prefix_x, name_to_index, name_to_uv_wgsl);
                    let value2_wgsl =
                        nodes_y.node_wgsl(value_y, &prefix_y, name_to_index, name_to_uv_wgsl);

                    // TODO: How to handle missing values?
                    let v1 = value1_wgsl.unwrap_or("0.0".to_string());
                    let v2 = value2_wgsl.unwrap_or("0.0".to_string());

                    let x_value = format!("{prefix_x}{i}");
                    let y_value = format!("{prefix_y}{i}");

                    // TODO: Handle value ranges and channels with add normal itself?
                    if i == 0 {
                        writeln!(wgsl, "let {prefix}_xy{i} = create_normal_map({v1}, {v2});")
                            .unwrap();
                        writeln!(wgsl, "let {prefix_x}{i} = {prefix}_xy{i}.x;").unwrap();
                        writeln!(wgsl, "let {prefix_y}{i} = {prefix}_xy{i}.y;").unwrap();
                    } else {
                        writeln!(wgsl, "let {prefix_x}{i} = {v1};").unwrap();
                        writeln!(wgsl, "let {prefix_y}{i} = {v2};").unwrap();
                    }

                    final_xy = Some((x_value, y_value));
                }
            }
        }

        final_xy
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
                blend_mode,
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
                    LayerBlendMode::AddNormal => {
                        // TODO: this should never happen?
                        error!("Unexpected blend mode {blend_mode:?}");
                        "0.0".to_string()
                    }
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
        ("// ASSIGN_NORMAL_GENERATED", "g_normal"),
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

pub fn generate_normal_layering_wgsl(
    assignment: &OutputAssignment,
    name_to_index: &mut IndexMap<SmolStr, usize>,
    name_to_uv_wgsl: &mut IndexMap<SmolStr, String>,
) -> String {
    let mut wgsl = String::new();

    let node_prefix = format!("{OUT_VAR}_n");

    let mut nodes_x = Nodes::default();
    insert_assignment(&mut nodes_x, &assignment.x);

    let mut nodes_y = Nodes::default();
    insert_assignment(&mut nodes_y, &assignment.y);

    let xy_values = Nodes::write_wgsl_xy(
        &mut wgsl,
        &nodes_x,
        &nodes_y,
        &node_prefix,
        name_to_index,
        name_to_uv_wgsl,
    );

    // TODO: Share this cache with all outputs?
    let mut nodes = Nodes::default();

    let z_index = insert_assignment(&mut nodes, &assignment.z);
    let w_index = insert_assignment(&mut nodes, &assignment.w);

    nodes.write_wgsl(&mut wgsl, &node_prefix, name_to_index, name_to_uv_wgsl);

    // Write any final assignments.
    if let Some((x_value, y_value)) = xy_values {
        writeln!(&mut wgsl, "{OUT_VAR}.x = {x_value};").unwrap();
        writeln!(&mut wgsl, "{OUT_VAR}.y = {y_value};").unwrap();
    }
    if let Some(z) = z_index {
        writeln!(&mut wgsl, "{OUT_VAR}.z = {node_prefix}{z};").unwrap();
    }
    if let Some(w) = w_index {
        writeln!(&mut wgsl, "{OUT_VAR}.w = {node_prefix}{w};").unwrap();
    }

    wgsl
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
