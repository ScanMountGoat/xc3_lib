use std::fmt::Write;

use indexmap::IndexMap;
use indoc::formatdoc;
use log::{error, warn};
use smol_str::SmolStr;
use xc3_model::{
    material::{
        assignments::{Assignment, AssignmentValue, OutputAssignment},
        TextureAlphaTest,
    },
    shader_database::Operation,
    IndexMapExt,
};

use crate::pipeline::PipelineKey;

const OUT_VAR: &str = "RESULT";

// TODO: This needs to be 16 to support all in game shaders.
const MAX_SAMPLERS: usize = 15;

// TODO: Just use the existing assignment instead of nodes?
// TODO: move these docs to xc3_model?

/// Static single assignment (SSA) representation for [LayerAssignmentValue]
/// where each [NodeValue] represents a single assignment for that node index.
/// This results in less generated code by reusing intermediate values.
#[derive(Debug, Default)]
struct Nodes {
    nodes: Vec<NodeValue>,
    values: Vec<AssignmentValue>,
    value_to_node_index: IndexMap<Assignment, usize>,
}

#[derive(Debug)]
enum NodeValue {
    Func {
        op: Operation,
        args: Vec<usize>,
    },
    Texture {
        name: SmolStr,
        coords: Vec<usize>,
        channel: Option<char>,
    },
    Value(usize), // TODO: just store the value directly?
}

impl Nodes {
    fn insert_layer_value(&mut self, assignments: &[Assignment], value: usize) -> usize {
        match self.value_to_node_index.get(&assignments[value]) {
            Some(i) => *i,
            None => {
                match &assignments[value] {
                    Assignment::Value(Some(AssignmentValue::Texture(texture))) => {
                        let coords = texture
                            .texcoords
                            .iter()
                            .map(|c| self.insert_layer_value(assignments, *c))
                            .collect();
                        let node = NodeValue::Texture {
                            name: texture.name.clone(),
                            coords,
                            channel: texture.channel,
                        };

                        self.insert_node_value(assignments[value].clone(), node)
                    }
                    Assignment::Value(v) => {
                        // TODO: how to handle missing values?
                        let v = v.clone().unwrap_or(AssignmentValue::Float(0.0.into()));
                        let value_index = self.insert_value(v);
                        let node = NodeValue::Value(value_index);

                        self.insert_node_value(assignments[value].clone(), node)
                    }
                    Assignment::Func { op, args } => {
                        if *op == Operation::Unk {
                            // Avoid unrecognized values that cause problems with code gen.
                            let value_index = self.insert_value(AssignmentValue::Float(0.0.into()));
                            let node = NodeValue::Value(value_index);

                            self.insert_node_value(assignments[value].clone(), node)
                        } else {
                            // Insert values that this value depends on first.
                            let args = args
                                .iter()
                                .map(|a| self.insert_layer_value(assignments, *a))
                                .collect();
                            let node = NodeValue::Func { op: *op, args };

                            self.insert_node_value(assignments[value].clone(), node)
                        }
                    }
                }
            }
        }
    }

    fn insert_node_value(&mut self, layer_value: Assignment, node: NodeValue) -> usize {
        let i = self.nodes.len();
        self.value_to_node_index.insert(layer_value, i);
        self.nodes.push(node);
        i
    }

    fn insert_value(&mut self, value: AssignmentValue) -> usize {
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
    ) {
        for (i, value) in self.nodes.iter().enumerate() {
            let value_wgsl = self.node_wgsl(value, node_prefix, name_to_index);
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
    ) -> Option<String> {
        match value {
            NodeValue::Func { op, args } => {
                let arg0 = arg(args, 0, node_prefix);
                let arg1 = arg(args, 1, node_prefix);
                let arg2 = arg(args, 2, node_prefix);

                match op {
                    Operation::Mix => Some(format!("mix({}, {}, {})", arg0?, arg1?, arg2?)),
                    Operation::Mul => Some(format!("{} * {}", arg0?, arg1?)),
                    Operation::Div => Some(format!("{} / {}", arg0?, arg1?)),
                    Operation::Add => Some(format!("{} + {}", arg0?, arg1?)),
                    Operation::AddNormal => {
                        // TODO: only normals xy should use this blend mode?
                        // TODO: Some shaders with two outputs use this?
                        error!("Unexpected operation {op:?}");
                        None
                    }
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
                    Operation::TexMatrix => {
                        // TODO: how to handle multiple channels like this?
                        Some(format!("{}", arg0?))
                    }
                    Operation::TexParallax => {
                        // TODO: how to handle multiple channels like this?
                        Some(format!("{} + uv_parallax(in, {}).x", arg0?, arg1?))
                    }
                    Operation::Unk => None,
                }
            }
            NodeValue::Texture {
                name,
                coords,
                channel,
            } => {
                let i = name_to_index.entry_index(name.clone());

                if i < MAX_SAMPLERS {
                    // TODO: This won't work for normals.
                    let u = coords.first()?;
                    let v = coords.get(1)?;

                    Some(format!(
                        "textureSample(s{i}, s{i}_sampler, vec2({node_prefix}{u}, {node_prefix}{v})){}",
                        channel_wgsl(*channel)
                    ))
                } else {
                    error!("Sampler index {i} exceeds supported max of {MAX_SAMPLERS}");
                    None
                }
            }
            NodeValue::Value(i) => channel_assignment_wgsl(&self.values[*i]),
        }
    }
}

fn arg(args: &[usize], i: usize, prefix: &str) -> Option<String> {
    Some(format!("{prefix}{}", args.get(i)?))
}

fn write_wgsl_xy(
    wgsl: &mut String,
    nodes_x: &Nodes,
    nodes_y: &Nodes,
    prefix: &str,
    name_to_index: &mut IndexMap<SmolStr, usize>,
) -> Option<(String, String)> {
    let prefix_x = format!("{prefix}_x");
    let prefix_y = format!("{prefix}_y");

    let mut final_xy = None;

    // Blend modes that use multiple channels require special handling.
    // Interleave x and y channel assignments to enable blending both channels.
    // This assumes the database xy entries differ only in the accessed channel.
    // TODO: The first nodes might be UVs instead of actual normal values?
    for (i, (value_x, value_y)) in nodes_x.nodes.iter().zip(&nodes_y.nodes).enumerate() {
        match (value_x, value_y) {
            (
                NodeValue::Func {
                    op: Operation::AddNormal,
                    args: args_x,
                },
                NodeValue::Func {
                    op: Operation::AddNormal,
                    args: args_y,
                },
            ) => {
                // TODO: check that ratios match.
                let ax = args_x.first()?;
                let bx = args_x.get(1)?;
                let rx = args_x.get(2)?;

                let ay = args_y.first()?;
                let by = args_y.get(1)?;
                let _ry = args_y.get(2)?;

                let r = format!("{prefix_x}{rx}");

                // Always keep the current normal map XY values in the range [0.0, 1.0].
                // This makes it easier to blend different channels together.
                let a_nrm = format!("create_normal_map({prefix_x}{ax}, {prefix_y}{ay})");
                let b_nrm = format!("create_normal_map({prefix_x}{bx}, {prefix_y}{by})");
                writeln!(
                    wgsl,
                    "let {prefix}_xy{i} = add_normal_maps({a_nrm}, {b_nrm}, {r}) * 0.5 + 0.5;",
                )
                .unwrap();

                let x_value = format!("{prefix}_xy{i}.x");
                let y_value = format!("{prefix}_xy{i}.y");
                writeln!(wgsl, "let {prefix_x}{i} = {x_value};",).unwrap();
                writeln!(wgsl, "let {prefix_y}{i} = {y_value};",).unwrap();
                final_xy = Some((x_value, y_value));
            }
            _ => {
                let value1_wgsl = nodes_x.node_wgsl(value_x, &prefix_x, name_to_index);
                let value2_wgsl = nodes_y.node_wgsl(value_y, &prefix_y, name_to_index);

                // TODO: How to handle missing values?
                let v1 = value1_wgsl.unwrap_or("0.0".to_string());
                let v2 = value2_wgsl.unwrap_or("0.0".to_string());

                let x_value = format!("{prefix_x}{i}");
                let y_value = format!("{prefix_y}{i}");

                writeln!(wgsl, "let {prefix_x}{i} = {v1};").unwrap();
                writeln!(wgsl, "let {prefix_y}{i} = {v2};").unwrap();

                final_xy = Some((x_value, y_value));
            }
        }
    }

    final_xy
}

pub fn create_model_shader(key: &PipelineKey) -> String {
    let mut source = include_str!("shader/model.wgsl").to_string();

    for ((from, var), to) in [
        ("// ASSIGN_COLOR_GENERATED", "g_color"),
        ("// ASSIGN_ETC_GENERATED", "g_etc_buffer"),
        ("// ASSIGN_NORMAL_GENERATED", "g_normal"),
        ("// ASSIGN_G_LGT_COLOR_GENERATED", "g_lgt_color"),
    ]
    .iter()
    .zip(&key.output_layers_wgsl)
    {
        // TODO: This causes slow compiles and very complex shaders?
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

pub fn generate_layering_wgsl(
    assignment: &OutputAssignment,
    assignments: &[Assignment],
    name_to_index: &mut IndexMap<SmolStr, usize>,
) -> String {
    let mut wgsl = String::new();

    // TODO: Share this cache with all outputs.
    let mut nodes = Nodes::default();

    let x_index = insert_assignment(&mut nodes, assignments, assignment.x);
    let y_index = insert_assignment(&mut nodes, assignments, assignment.y);
    let z_index = insert_assignment(&mut nodes, assignments, assignment.z);
    let w_index = insert_assignment(&mut nodes, assignments, assignment.w);

    let node_prefix = format!("{OUT_VAR}_n");
    nodes.write_wgsl(&mut wgsl, &node_prefix, name_to_index);

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

fn insert_assignment(
    nodes: &mut Nodes,
    assignments: &[Assignment],
    index: Option<usize>,
) -> Option<usize> {
    let index = index?;
    if assignments[index] != Assignment::Value(None) {
        Some(nodes.insert_layer_value(assignments, index))
    } else {
        None
    }
}

pub fn generate_normal_layering_wgsl(
    assignment: &OutputAssignment,
    assignments: &[Assignment],
    name_to_index: &mut IndexMap<SmolStr, usize>,
) -> String {
    let mut wgsl = String::new();

    let node_prefix = format!("{OUT_VAR}_n");

    let mut nodes_x = Nodes::default();
    insert_assignment(&mut nodes_x, assignments, assignment.x);

    let mut nodes_y = Nodes::default();
    insert_assignment(&mut nodes_y, assignments, assignment.y);

    let xy_values = write_wgsl_xy(&mut wgsl, &nodes_x, &nodes_y, &node_prefix, name_to_index);

    let mut nodes_zw = Nodes::default();
    let z_index = insert_assignment(&mut nodes_zw, assignments, assignment.z);
    let w_index = insert_assignment(&mut nodes_zw, assignments, assignment.w);

    nodes_zw.write_wgsl(&mut wgsl, &node_prefix, name_to_index);

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

pub fn generate_normal_intensity_wgsl(
    intensity: usize,
    assignments: &[Assignment],
    name_to_index: &mut IndexMap<SmolStr, usize>,
) -> String {
    let mut wgsl = String::new();

    let node_prefix = format!("{OUT_VAR}_nrm_intensity");

    let mut nodes = Nodes::default();
    let index = insert_assignment(&mut nodes, assignments, Some(intensity));

    nodes.write_wgsl(&mut wgsl, &node_prefix, name_to_index);

    if let Some(i) = index {
        writeln!(&mut wgsl, "{OUT_VAR} = {node_prefix}{i};").unwrap();
    }
    wgsl
}

fn channel_assignment_wgsl(value: &AssignmentValue) -> Option<String> {
    match value {
        AssignmentValue::Texture(_) => {
            // TODO: This shouldn't happen?
            todo!()
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
                _ => {
                    warn!("Unsupported attribute {name}{c}");
                    None
                }
            }
        }
        AssignmentValue::Float(f) => Some(format!("{f:?}")),
    }
}

fn channel_wgsl(c: Option<char>) -> String {
    c.map(|c| format!(".{c}")).unwrap_or_default()
}
