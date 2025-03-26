use std::{collections::BTreeMap, path::Path, sync::LazyLock};

use bimap::BiBTreeMap;
use glsl_lang::{
    ast::{
        ExprData, LayoutQualifierSpecData, SingleDeclaration, StorageQualifierData,
        TranslationUnit, TypeQualifierSpecData,
    },
    parse::DefaultParse,
    visitor::{Host, Visit, Visitor},
};
use indexmap::IndexMap;
use indoc::indoc;
use log::error;
use rayon::prelude::*;
use xc3_lib::{
    mths::{FragmentShader, Mths},
    spch::Spch,
};
use xc3_model::shader_database::{
    AttributeDependency, BufferDependency, Dependency, LayerBlendMode, OutputDependencies,
    ProgramHash, ShaderDatabase, ShaderProgram, TextureLayer,
};

use crate::{
    dependencies::{
        attribute_dependencies, buffer_dependency, input_dependencies, texcoord_params,
    },
    extract::nvsd_glsl_name,
    graph::{
        glsl::shader_source_no_extensions,
        query::{
            assign_x, assign_x_recursive, dot3_a_b, fma_a_b_c, fma_half_half, mix_a_b_ratio,
            node_expr, normalize, query_nodes,
        },
        Expr, Graph, Node,
    },
};

fn shader_from_glsl(vertex: Option<&TranslationUnit>, fragment: &TranslationUnit) -> ShaderProgram {
    let frag = &Graph::from_glsl(fragment);
    let frag_attributes = &find_attribute_locations(fragment);

    let vertex = vertex.map(|v| (Graph::from_glsl(v), find_attribute_locations(v)));
    let (vert, vert_attributes) = vertex.unzip();

    let outline_width = vert
        .as_ref()
        .map(outline_width_parameter)
        .unwrap_or_default();

    let mut output_dependencies = IndexMap::new();
    for i in 0..=5 {
        for c in "xyzw".chars() {
            let name = format!("out_attr{i}");
            let assignments = frag.assignments_recursive(&name, Some(c), None);
            let dependent_lines = frag.dependencies_recursive(&name, Some(c), None);

            let mut dependencies =
                input_dependencies(frag, frag_attributes, &assignments, &dependent_lines);

            if let (Some(vert), Some(vert_attributes)) = (&vert, &vert_attributes) {
                // Add texture parameters used for the corresponding vertex output.
                // Most shaders apply UV transforms in the vertex shader.
                // This will be used later for texture layers.
                apply_vertex_tex_coord_params(
                    vert,
                    vert_attributes,
                    frag_attributes,
                    &mut dependencies,
                );

                for d in &mut dependencies {
                    apply_attribute_names(vert, vert_attributes, frag_attributes, d);
                }
            }

            let mut layers = Vec::new();

            // Xenoblade X DE uses different outputs than other games.
            // Detect color or params to handle different outputs and channels.
            if i == 0 || i == 1 {
                layers = find_color_or_param_layers(frag, &dependent_lines, &dependencies)
                    .unwrap_or_default();
            } else if i == 2 {
                if c == 'x' || c == 'y' {
                    // The normals use XY for output index 2 for all games.
                    layers = find_normal_layers(frag, &dependent_lines, &dependencies)
                        .unwrap_or_default();
                } else if c == 'z' {
                    layers = find_color_or_param_layers(frag, &dependent_lines, &dependencies)
                        .unwrap_or_default();
                }
            }

            // Avoid storing redundant information with dependencies.
            if let [layer0] = &layers[..] {
                dependencies.sort_by_key(|d| d != &layer0.value);
                layers = Vec::new();
            }

            if i == 1 && c == 'y' {
                if let Some(param) = geometric_specular_aa(frag) {
                    dependencies = vec![Dependency::Buffer(param)];
                }
            }

            // TODO: Is there a better way of doing this?
            if let (Some(vert), Some(vert_attributes)) = (&vert, &vert_attributes) {
                for layer in &mut layers {
                    apply_attribute_names(vert, vert_attributes, frag_attributes, &mut layer.value);
                    if let Some(r) = &mut layer.ratio {
                        apply_attribute_names(vert, vert_attributes, frag_attributes, r);
                    }
                }
            }

            if !dependencies.is_empty() {
                // Simplify the output name to save space.
                let output_name = format!("o{i}.{c}");
                output_dependencies.insert(
                    output_name.into(),
                    OutputDependencies {
                        dependencies,
                        layers,
                    },
                );
            }
        }
    }

    ShaderProgram {
        // IndexMap gives consistent ordering for attribute names.
        output_dependencies,
        outline_width,
    }
}

static OUTLINE_WIDTH_PARAMETER: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
        void main() {
            alpha = vColor.w;
            result = param * alpha;
            result = 0.0 - result;
            result = temp * result;
        }
    "};
    Graph::parse_glsl(query).unwrap()
});

fn outline_width_parameter(vert: &Graph) -> Option<Dependency> {
    vert.nodes.iter().find_map(|n| {
        // TODO: Add a way to match identifiers like "vColor" exactly.
        let result = query_nodes(&n.input, &vert.nodes, &OUTLINE_WIDTH_PARAMETER.nodes)?;
        let param = result.get("param")?;
        let vcolor = result.get("vColor")?;

        if matches!(vcolor, Expr::Global { name, channel } if name == "vColor" && *channel == Some('w')) {
            // TODO: Handle other dependency types?
            buffer_dependency(param).map(Dependency::Buffer)
        } else {
            None
        }
    })
}

fn shader_from_latte_asm(
    _vertex: &str,
    fragment: &str,
    fragment_shader: &FragmentShader,
) -> ShaderProgram {
    let frag = &Graph::from_latte_asm(fragment);
    let frag_attributes = &Attributes::default();

    // TODO: Fix vertex parsing errors.
    // let vert = &Graph::from_latte_asm(vertex);
    // let vert_attributes = &Attributes::default();

    // TODO: What is the largest number of outputs?
    let output_dependencies = (0..=5)
        .flat_map(|i| {
            "xyzw".chars().map(move |c| {
                let name = format!("PIX{i}");

                let assignments = frag.assignments_recursive(&name, Some(c), None);
                let dependent_lines = frag.dependencies_recursive(&name, Some(c), None);

                let mut dependencies =
                    input_dependencies(frag, frag_attributes, &assignments, &dependent_lines);

                // Add texture parameters used for the corresponding vertex output.
                // Most shaders apply UV transforms in the vertex shader.
                // apply_vertex_texcoord_params(
                //     vert,
                //     vert_attributes,
                //     frag_attributes,
                //     &mut dependencies,
                // );

                // apply_attribute_names(vert, vert_attributes, frag_attributes, &mut dependencies);

                // Apply annotations from the shader metadata.
                // We don't annotate the assembly itself to avoid parsing errors.
                for d in &mut dependencies {
                    match d {
                        Dependency::Constant(_) => (),
                        Dependency::Buffer(_) => (),
                        Dependency::Texture(t) => {
                            for sampler in &fragment_shader.samplers {
                                if t.name == format!("t{}", sampler.location) {
                                    t.name = (&sampler.name).into();
                                }
                            }
                        }
                        Dependency::Attribute(_) => todo!(),
                    }
                }

                // Simplify the output name to save space.
                let output_name = format!("o{i}.{c}");
                (
                    output_name.into(),
                    OutputDependencies {
                        dependencies,
                        layers: Vec::new(),
                    },
                )
            })
        })
        .filter(|(_, dependencies)| !dependencies.dependencies.is_empty())
        .collect();

    ShaderProgram {
        // IndexMap gives consistent ordering for attribute names.
        output_dependencies,
        outline_width: None,
    }
}

fn find_color_or_param_layers(
    frag: &Graph,
    dependent_lines: &[usize],
    dependencies: &[Dependency],
) -> Option<Vec<TextureLayer>> {
    let last_node_index = *dependent_lines.last()?;
    let last_node = frag.nodes.get(last_node_index)?;

    // matCol.xyz in pcmdo shaders.
    let mut current_col = &last_node.input;

    // Remove some redundant conversions found in some shaders.
    if let Expr::Func { name, args, .. } = current_col {
        if name == "intBitsToFloat" {
            current_col = assign_x_recursive(&frag.nodes, &args[0]);

            if let Expr::Func { name, args, .. } = current_col {
                if name == "floatBitsToInt" {
                    current_col = &args[0];
                }
            }
        }
    }

    current_col = assign_x_recursive(&frag.nodes, current_col);

    // This isn't always present for all materials in all games.
    // Xenoblade 1 DE and Xenoblade 3 both seem to do this for non map materials.
    if let Some((mat_cols, _monochrome_ratio)) = calc_monochrome(&frag.nodes, current_col) {
        let mat_col = match last_node.output.channel {
            Some('x') => &mat_cols[0],
            Some('y') => &mat_cols[1],
            Some('z') => &mat_cols[2],
            _ => &mat_cols[0],
        };
        current_col = node_expr(&frag.nodes, mat_col)?;
    }

    let layers = find_layers(&frag.nodes, current_col, dependencies);

    Some(layers)
}

fn sampler_index(sampler_name: &str) -> Option<usize> {
    // Convert names like "s3" to index 3.
    sampler_name.strip_prefix('s')?.parse().ok()
}

fn calc_monochrome<'a>(nodes: &'a [Node], expr: &'a Expr) -> Option<([&'a Expr; 3], &'a Expr)> {
    // calcMonochrome in pcmdo fragment shaders for XC1 and XC3.
    // TODO: Check weight values for XC1 (0.3, 0.59, 0.11) or XC3 (0.01, 0.01, 0.01)?
    let (_mat_col, monochrome, monochrome_ratio) = mix_a_b_ratio(nodes, expr)?;
    let monochrome = node_expr(nodes, monochrome)?;
    let (mat_col, _monochrome_weights) = dot3_a_b(nodes, monochrome)?;
    Some((mat_col, monochrome_ratio))
}

fn find_normal_layers(
    frag: &Graph,
    dependent_lines: &[usize],
    dependencies: &[Dependency],
) -> Option<Vec<TextureLayer>> {
    let last_node_index = *dependent_lines.last()?;
    let last_node = frag.nodes.get(last_node_index)?;

    let node = assign_x(&frag.nodes, &last_node.input)?;

    // setMrtNormal in pcmdo shaders.
    let view_normal = fma_half_half(&frag.nodes, node)?;
    let view_normal = assign_x_recursive(&frag.nodes, view_normal);
    let view_normal = normalize(&frag.nodes, view_normal)?;

    // TODO: front facing in calcNormalZAbs in pcmdo?

    // nomWork input for getCalcNormalMap in pcmdo shaders.
    let nom_work = calc_normal_map(frag, &view_normal.input)?;
    let nom_work = node_expr(&frag.nodes, nom_work[0])?;

    let mut layers = find_layers(&frag.nodes, nom_work, dependencies);

    // TODO: Modify the query instead to find the appropriate channel?
    // Assume that normal inputs are always XY for now.
    let channel = last_node.output.channel;

    for layer in &mut layers {
        match &mut layer.value {
            Dependency::Constant(_) => (),
            Dependency::Buffer(b) => b.channel = channel,
            Dependency::Texture(t) => t.channel = channel,
            Dependency::Attribute(a) => a.channel = channel,
        }
    }

    Some(layers)
}

fn find_layers(nodes: &[Node], current: &Expr, dependencies: &[Dependency]) -> Vec<TextureLayer> {
    let mut layers = Vec::new();

    let mut current = current;

    // Detect the layers and blend mode from most to least specific.
    while let Some((layer_a, layer_b, ratio, blend_mode)) = blend_add_normal(nodes, current)
        .or_else(|| blend_overlay_ratio(nodes, current))
        .or_else(|| blend_overlay(nodes, current))
        .or_else(|| blend_over(nodes, current))
        .or_else(|| blend_ratio(nodes, current))
        .or_else(|| blend_mul(nodes, current, dependencies))
        .or_else(|| blend_add_ratio(current))
        .or_else(|| blend_sub(nodes, current))
        .or_else(|| blend_add(nodes, current, dependencies))
    {
        let layer_b = extract_layer_value(nodes, layer_b, dependencies);

        if let Some(value) = layer_b {
            let (fresnel_ratio, ratio) = ratio_dependency(ratio, nodes, dependencies);
            layers.push(TextureLayer {
                value,
                ratio,
                blend_mode,
                is_fresnel: fresnel_ratio,
            });
            current = assign_x_recursive(nodes, layer_a);
        } else {
            break;
        }
    }

    // Detect the base layer.
    if let Some(value) = extract_layer_value(nodes, current, dependencies) {
        layers.push(TextureLayer {
            value,
            ratio: None,
            blend_mode: LayerBlendMode::Mix,
            is_fresnel: false,
        });
    }

    // We start from the output, so these are in reverse order.
    layers.reverse();
    layers
}

fn extract_layer_value(
    nodes: &[Node],
    layer: &Expr,
    dependencies: &[Dependency],
) -> Option<Dependency> {
    let mut layer = assign_x_recursive(nodes, layer);
    if let Some(new_layer) = normal_map_fma(nodes, layer) {
        layer = new_layer;
    }

    // TODO: Is it worth storing information about component max?
    if let Some(new_layer) = component_max_xyz(nodes, layer) {
        layer = new_layer;
    }

    layer_value(layer, dependencies)
}

static BLEND_OVER: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
        void main() {
            neg_a = 0.0 - a;
            b_minus_a = neg_a + b;
            result = fma(b_minus_a, ratio, a);
        }
    "};
    Graph::parse_glsl(query).unwrap()
});

fn blend_over<'a>(
    nodes: &'a [Node],
    expr: &'a Expr,
) -> Option<(&'a Expr, &'a Expr, &'a Expr, LayerBlendMode)> {
    // getPixelCalcOver in pcmdo fragment shaders for XC1 and XC3.
    let result = query_nodes(expr, nodes, &BLEND_OVER.nodes)?;
    let a = result.get("a")?;
    let b = result.get("b")?;
    let ratio = result.get("ratio")?;
    Some((a, b, ratio, LayerBlendMode::Mix))
}

static BLEND_RATIO: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
        void main() {
            neg_a = 0.0 - a;
            ab_minus_a = fma(a, b, neg_a);
            result = fma(ab_minus_a, ratio, a);
        }
    "};
    Graph::parse_glsl(query).unwrap()
});

fn blend_ratio<'a>(
    nodes: &'a [Node],
    expr: &'a Expr,
) -> Option<(&'a Expr, &'a Expr, &'a Expr, LayerBlendMode)> {
    // getPixelCalcRatioBlend in pcmdo fragment shaders for XC1 and XC3.
    let result = query_nodes(expr, nodes, &BLEND_RATIO.nodes)?;
    let a = result.get("a")?;
    let b = result.get("b")?;
    let ratio = result.get("ratio")?;
    Some((a, b, ratio, LayerBlendMode::MixRatio))
}

fn blend_add_ratio(expr: &Expr) -> Option<(&Expr, &Expr, &Expr, LayerBlendMode)> {
    // += getPixelCalcRatio in pcmdo fragment shaders for XC1 and XC3.
    let (a, b, c) = fma_a_b_c(expr)?;
    Some((c, a, b, LayerBlendMode::Add))
}

static BLEND_ADD: LazyLock<Graph> =
    LazyLock::new(|| Graph::parse_glsl("void main() { result = a + b; }").unwrap());

fn blend_add<'a>(
    nodes: &'a [Node],
    expr: &'a Expr,
    dependencies: &[Dependency],
) -> Option<(&'a Expr, &'a Expr, &'a Expr, LayerBlendMode)> {
    // Some layers are simply added together like for xeno3/chr/chr/ch05042101.wimdo "hat_toon".
    let result = query_nodes(expr, nodes, &BLEND_ADD.nodes)?;
    let a = result.get("a")?;
    let b = result.get("b")?;
    // The ordering is ambiguous since a+b == b+a.
    // Assume the base layer is not a global texture.
    if let (Some(Dependency::Texture(t1)), Some(Dependency::Texture(t2))) = (
        layer_value(assign_x_recursive(nodes, a), dependencies),
        layer_value(assign_x_recursive(nodes, b), dependencies),
    ) {
        if sampler_index(&t1.name).unwrap_or(usize::MAX)
            > sampler_index(&t2.name).unwrap_or(usize::MAX)
        {
            return Some((b, a, &Expr::Float(1.0), LayerBlendMode::Add));
        }
    }
    Some((a, b, &Expr::Float(1.0), LayerBlendMode::Add))
}
static BLEND_SUB: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
        void main() {
            neg_b = 0.0 - b;
            result = a + neg_b;
        }
    "};
    Graph::parse_glsl(query).unwrap()
});

fn blend_sub<'a>(
    nodes: &'a [Node],
    expr: &'a Expr,
) -> Option<(&'a Expr, &'a Expr, &'a Expr, LayerBlendMode)> {
    // Some layers are simply subtracted like for xeno3/chr/chr/ch44000210.wimdo "ch45133501_body".
    let result = query_nodes(expr, nodes, &BLEND_SUB.nodes)?;
    let a = result.get("a")?;
    let b = result.get("b")?;
    Some((a, b, &Expr::Float(-1.0), LayerBlendMode::Add))
}

static BLEND_MUL: LazyLock<Graph> =
    LazyLock::new(|| Graph::parse_glsl("void main() { result = a * b; }").unwrap());

fn blend_mul<'a>(
    nodes: &'a [Node],
    expr: &'a Expr,
    dependencies: &[Dependency],
) -> Option<(&'a Expr, &'a Expr, &'a Expr, LayerBlendMode)> {
    // Some layers are simply multiplied together.
    let result = query_nodes(expr, nodes, &BLEND_MUL.nodes)?;
    let a = result.get("a")?;
    let b = result.get("b")?;
    // TODO: The ordering is ambiguous since a*b == b*a.
    let a_value = layer_value(assign_x_recursive(nodes, a), dependencies);
    let b_value = layer_value(assign_x_recursive(nodes, b), dependencies);
    if !matches!(a_value, Some(Dependency::Texture(_)))
        && matches!(b_value, Some(Dependency::Texture(_)))
    {
        Some((b, a, &Expr::Float(1.0), LayerBlendMode::MixRatio))
    } else {
        Some((a, b, &Expr::Float(1.0), LayerBlendMode::MixRatio))
    }
}

static BLEND_OVERLAY_XC2: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
        void main() {
            two_a = 2.0 * a;
            a_b_multiply = two_a * b;
            neg_a_b_multiply = 0.0 - a_b_multiply;
            a_b_multiply = fma(a_gt_half, neg_a_b_multiply, a_b_multiply);

            a_b_screen = fma(b, neg_temp, temp);
            neg_a_gt_half = 0.0 - a_gt_half;
            a_b_screen = fma(a_b_screen, neg_a_gt_half, a_gt_half);

            a_b_overlay = a_b_screen + a_b_multiply;
            neg_ratio = 0.0 - ratio;
            result = fma(a, neg_ratio, a);
            result = fma(a_b_overlay, ratio, result);
        }
    "};
    Graph::parse_glsl(query).unwrap()
});

fn blend_overlay_ratio<'a>(
    nodes: &'a [Node],
    expr: &'a Expr,
) -> Option<(&'a Expr, &'a Expr, &'a Expr, LayerBlendMode)> {
    // Overlay combines multiply and screen blend modes.
    // Some XC2 models use overlay blending for metalness.
    let result = query_nodes(expr, nodes, &BLEND_OVERLAY_XC2.nodes)?;
    let a = result.get("a")?;
    let b = result.get("b")?;
    let ratio = result.get("ratio")?;
    Some((a, b, ratio, LayerBlendMode::Overlay))
}

static BLEND_OVERLAY_XCX_DE: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
        void main() {
            neg_b = 0.0 - b; 
            one_minus_b = neg_b + 1.0;
            two_b = b * 2.0;
            multiply = two_b * a;
            temp_181 = a + -0.5;
            temp_182 = 0.0 - one_minus_b;
            temp_183 = fma(a, temp_182, one_minus_b);
            temp_189 = temp_181 * 1000.0;
            is_a_gt_half = clamp(temp_189, 0.0, 1.0);
            temp_193 = 0.0 - multiply;
            temp_194 = fma(temp_183, -2.0, temp_193);
            temp_208 = fma(is_a_gt_half, temp_194, is_a_gt_half);
            result = multiply + temp_208;
        }
    "};
    Graph::parse_glsl(query).unwrap()
});

fn blend_overlay<'a>(
    nodes: &'a [Node],
    expr: &'a Expr,
) -> Option<(&'a Expr, &'a Expr, &'a Expr, LayerBlendMode)> {
    // Overlay combines multiply and screen blend modes.
    // Some XCX DE models use overlay for face coloring.
    let result = query_nodes(expr, nodes, &BLEND_OVERLAY_XCX_DE.nodes)?;
    let a = result.get("a")?;
    let b = result.get("b")?;
    Some((a, b, &Expr::Float(1.0), LayerBlendMode::Overlay))
}

static RATIO_DEPENDENCY: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
        void main() {
            a = ratio * 5.0;
            result = a * b;
            result = exp2(result);
        }
    "};
    Graph::parse_glsl(query).unwrap()
});

fn ratio_dependency(
    ratio: &Expr,
    nodes: &[Node],
    dependencies: &[Dependency],
) -> (bool, Option<Dependency>) {
    // Reduce any assignment chains for what's likely a parameter or texture assignment.
    let mut ratio = assign_x_recursive(nodes, ratio);

    let mut is_fresnel = false;

    // Extract the ratio from getPixelCalcFresnel in pcmdo shaders if present.
    let result = query_nodes(ratio, nodes, &RATIO_DEPENDENCY.nodes);
    if let Some(new_ratio) = result.as_ref().and_then(|r| r.get("ratio")) {
        ratio = new_ratio;
        is_fresnel = true;
    }

    (is_fresnel, dependency_cached_texture(ratio, dependencies))
}

fn dependency_cached_texture(ratio: &Expr, dependencies: &[Dependency]) -> Option<Dependency> {
    buffer_dependency(ratio)
        .map(Dependency::Buffer)
        .or_else(|| match ratio {
            Expr::Func {
                name,
                args,
                channel,
            } => {
                if name.starts_with("texture") {
                    if let Some(Expr::Global { name, .. }) = args.first() {
                        // Texture dependencies have already been found recursively.
                        // Use existing dependencies to include texcoord params.
                        dependencies
                            .iter()
                            .find(|d| {
                                if let Dependency::Texture(t) = d {
                                    t.name == name && (channel.is_none() || *channel == t.channel)
                                } else {
                                    false
                                }
                            })
                            .cloned()
                    } else {
                        // TODO: How to handle this case?
                        None
                    }
                } else {
                    None
                }
            }
            // TODO: Why does handling other constants break base layer detection?
            Expr::Float(1.0) => Some(Dependency::Constant(1.0.into())),
            Expr::Float(-1.0) => Some(Dependency::Constant((-1.0).into())),
            // TODO: Find dependencies recursively?
            _ => None,
        })
}

fn layer_value(input: &Expr, dependencies: &[Dependency]) -> Option<Dependency> {
    dependency_cached_texture(input, dependencies)
        .or_else(|| buffer_dependency(input).map(Dependency::Buffer))
        .or_else(|| {
            // TODO: Also check if this matches a vertex input name?
            if let Expr::Global { name, channel } = input {
                Some(Dependency::Attribute(AttributeDependency {
                    name: name.into(),
                    channel: *channel,
                }))
            } else {
                None
            }
        })
}

static BLEND_ADD_NORMAL: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
        void main() {
            n = n2;
            n = n.x;
            n = fma(n, 2.0, neg_one);
            n = n * temp;
            neg_n = 0.0 - n;
            n = fma(temp, temp, neg_n);
            n_inv_sqrt = inversesqrt(temp);
            neg_n1 = 0.0 - n1;
            r = fma(n, n_inv_sqrt, neg_n1);

            nom_work = nom_work;
            nom_work = fma(r, ratio, nom_work);
            inv_sqrt = inversesqrt(temp);
            nom_work = nom_work * inv_sqrt;
        }
    "};
    Graph::parse_glsl(query).unwrap()
});

fn blend_add_normal<'a>(
    nodes: &'a [Node],
    nom_work: &'a Expr,
) -> Option<(&'a Expr, &'a Expr, &'a Expr, LayerBlendMode)> {
    // getPixelCalcAddNormal in pcmdo shaders.
    // normalize(mix(nomWork, normalize(r), ratio))
    // XC2: ratio * (normalize(r) - nomWork) + nomWork
    // XC3: (normalize(r) - nomWork) * ratio + nomWork
    // TODO: Is it worth detecting the textures used for r?
    // TODO: nom_work and n1 are the same?
    // TODO: Reduce assignments to allow combining lines?
    // TODO: Allow 0.0 - x or -x
    let result = query_nodes(nom_work, nodes, &BLEND_ADD_NORMAL.nodes)?;
    let nom_work = result.get("nom_work")?;
    let ratio = result.get("ratio")?;
    let n2 = result.get("n2")?;
    Some((nom_work, n2, ratio, LayerBlendMode::AddNormal))
}

static NORMAL_MAP_FMA: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
        void main() {
            result = result;
            result = result.x;
            result = fma(result, 2.0, temp);
        }
    "};
    Graph::parse_glsl(query).unwrap()
});

fn normal_map_fma<'a>(nodes: &'a [Node], nom_work: &'a Expr) -> Option<&'a Expr> {
    // Extract the texture for n1 if present.
    // This could be fma(x, 2.0, -1.0) or fma(x, 2.0, -1.0039216)
    // This will only work for base layers.
    let result = query_nodes(nom_work, nodes, &NORMAL_MAP_FMA.nodes)?;
    result.get("result").copied()
}

static COMPONENT_MAX_XYZ: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
        void main() {
            y = value.y;
            z = value.z;
            x = value.x;
            result = max(x, y);
            result = max(z, result);
        }
    "};
    Graph::parse_glsl(query).unwrap()
});

fn component_max_xyz<'a>(nodes: &'a [Node], expr: &'a Expr) -> Option<&'a Expr> {
    let result = query_nodes(expr, nodes, &COMPONENT_MAX_XYZ.nodes)?;
    result.get("value").copied()
}

fn calc_normal_map<'a>(frag: &'a Graph, view_normal: &'a Expr) -> Option<[&'a Expr; 3]> {
    // getCalcNormalMap in pcmdo shaders.
    // result = normalize(nomWork).x, normalize(tangent).x
    // result = fma(normalize(nomWork).y, normalize(bitangent).x, result)
    // result = fma(normalize(nomWork).z, normalize(normal).x, result)
    let (nrm, _tangent_normal_bitangent) = dot3_a_b(&frag.nodes, view_normal)?;
    Some(nrm)
}

static GEOMETRIC_SPECULAR_AA: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
        void main() {
            result = 0.0 - glossiness;
            result = 1.0 + result;
            result = fma(result, result, temp);
            result = clamp(result, 0.0, 1.0);
            result = sqrt(result);
            result = 0.0 - result;
            result = result + 1.0;
            result = result;
        }
    "};
    Graph::parse_glsl(query).unwrap()
});

fn geometric_specular_aa(frag: &Graph) -> Option<BufferDependency> {
    let node_index = frag
        .nodes
        .iter()
        .rposition(|n| n.output.name == "out_attr1" && n.output.channel == Some('y'))?;
    let last_node_index = *frag.node_dependencies_recursive(node_index, None).last()?;
    let last_node = frag.nodes.get(last_node_index)?;

    // calcGeometricSpecularAA in pcmdo shaders.
    // glossiness = 1.0 - sqrt(clamp((1.0 - glossiness)^2 + kernelRoughness2, 0.0, 1.0))
    // TODO: reduce assignments to allow combining lines
    // TODO: Allow 0.0 - x or -x
    let result = query_nodes(&last_node.input, &frag.nodes, &GEOMETRIC_SPECULAR_AA.nodes)?;

    // TODO: Will this final node ever not be a parameter?
    // TODO: is specular AA ever used with textures as input?
    result
        .get("glossiness")
        .copied()
        .and_then(buffer_dependency)
}

fn apply_vertex_tex_coord_params(
    vertex: &Graph,
    vertex_attributes: &Attributes,
    fragment_attributes: &Attributes,
    dependencies: &mut [Dependency],
) {
    for dependency in dependencies {
        if let Dependency::Texture(texture) = dependency {
            for texcoord in &mut texture.texcoords {
                // Convert a fragment input like "in_attr4" to its vertex output like "vTex0".
                if let Some(fragment_location) = fragment_attributes
                    .input_locations
                    .get_by_left(texcoord.name.as_str())
                {
                    if let Some(vertex_output_name) = vertex_attributes
                        .output_locations
                        .get_by_right(fragment_location)
                    {
                        // Preserve the channel ordering here.
                        // Find any additional scale parameters.
                        if let Some(node) = vertex.nodes.iter().rfind(|n| {
                            &n.output.name == vertex_output_name
                                && n.output.channel == texcoord.channel
                        }) {
                            // Detect common cases for transforming UV coordinates.
                            if let Some(new_texcoord) =
                                texcoord_params(vertex, &node.input, vertex_attributes)
                            {
                                *texcoord = new_texcoord;
                            }
                        }

                        // Also fix channels since the zw output may just be scaled vTex0.xy.
                        if let Some((actual_name, actual_channel)) =
                            find_texcoord_input_name_channel(
                                vertex,
                                texcoord,
                                vertex_output_name,
                                vertex_attributes,
                            )
                        {
                            texcoord.name = actual_name.into();
                            texcoord.channel = actual_channel;
                        }
                    }
                }
            }
        }
    }
}

// TODO: Share code with texcoord function.
fn apply_attribute_names(
    vertex: &Graph,
    vertex_attributes: &Attributes,
    fragment_attributes: &Attributes,
    dependency: &mut Dependency,
) {
    if let Dependency::Attribute(attribute) = dependency {
        // Convert a fragment input like "in_attr4" to its vertex output like "vTex0".
        if let Some(fragment_location) = fragment_attributes
            .input_locations
            .get_by_left(attribute.name.as_str())
        {
            if let Some(vertex_output_name) = vertex_attributes
                .output_locations
                .get_by_right(fragment_location)
            {
                // TODO: Avoid calculating this more than once.
                let dependent_lines =
                    vertex.dependencies_recursive(vertex_output_name, attribute.channel, None);

                if let Some(input_attribute) =
                    attribute_dependencies(vertex, &dependent_lines, vertex_attributes, None)
                        .first()
                {
                    attribute.name.clone_from(&input_attribute.name);
                }
            }
        }
    }
}

fn find_texcoord_input_name_channel(
    vertex: &Graph,
    texcoord: &xc3_model::shader_database::TexCoord,
    vertex_output_name: &str,
    vertex_attributes: &Attributes,
) -> Option<(String, Option<char>)> {
    // We only need to look up one output per texcoord.
    let c = texcoord.channel;

    // TODO: Avoid calculating this more than once.
    let dependent_lines = vertex.dependencies_recursive(vertex_output_name, c, None);

    attribute_dependencies(vertex, &dependent_lines, vertex_attributes, None)
        .first()
        .map(|a| (a.name.to_string(), a.channel))
}

pub fn create_shader_database(input: &str) -> ShaderDatabase {
    let mut programs = BTreeMap::new();

    for folder in std::fs::read_dir(input).unwrap().map(|e| e.unwrap().path()) {
        // TODO: Find a better way to detect maps.
        if !folder.join("map").exists()
            && !folder.join("prop").exists()
            && !folder.join("env").exists()
        {
            add_programs(&mut programs, &folder);
        } else {
            add_map_programs(&mut programs, &folder.join("map"));
            add_map_programs(&mut programs, &folder.join("prop"));
            add_map_programs(&mut programs, &folder.join("env"));
        }
    }

    ShaderDatabase::from_programs(programs)
}

pub fn create_shader_database_legacy(input: &str) -> ShaderDatabase {
    let mut programs = BTreeMap::new();

    for folder in std::fs::read_dir(input).unwrap().map(|e| e.unwrap().path()) {
        add_programs_legacy(&mut programs, &folder);
    }

    ShaderDatabase::from_programs(programs)
}

fn add_map_programs(programs: &mut BTreeMap<ProgramHash, ShaderProgram>, folder: &Path) {
    // TODO: Not all maps have env or prop models?
    if let Ok(dir) = std::fs::read_dir(folder) {
        // Folders are generated like "ma01a/prop/4".
        for path in dir.into_iter().map(|e| e.unwrap().path()) {
            add_programs(programs, &path);
        }
    }
}

fn add_programs(programs: &mut BTreeMap<ProgramHash, ShaderProgram>, folder: &Path) {
    if let Ok(spch) = Spch::from_file(folder.join("shaders.wishp")) {
        // Avoid processing the same program more than once.
        let mut unique_hash_slct_index = BTreeMap::new();

        for (i, slct_offset) in spch.slct_offsets.iter().enumerate() {
            let slct = slct_offset.read_slct(&spch.slct_section).unwrap();

            // Only check the first shader for now.
            // TODO: What do additional nvsd shader entries do?
            if let Some((p, vert, frag)) = spch.program_data_vertex_fragment_binaries(&slct).first()
            {
                let hash = ProgramHash::from_spch_program(p, vert, frag);

                if !programs.contains_key(&hash) {
                    unique_hash_slct_index.insert(hash, i);
                }
            }
        }

        // Shader processing is CPU intensive and benefits from parallelism.
        programs.par_extend(unique_hash_slct_index.into_par_iter().map(|(hash, i)| {
            let path = &folder
                .join(nvsd_glsl_name(&spch, i, 0))
                .with_extension("frag");

            // TODO: Should the vertex shader be mandatory?
            let vertex_source = std::fs::read_to_string(path.with_extension("vert")).ok();
            let vertex = vertex_source.and_then(|s| {
                let source = shader_source_no_extensions(&s);
                match TranslationUnit::parse(source) {
                    Ok(vertex) => Some(vertex),
                    Err(e) => {
                        error!("Error parsing {path:?}: {e}");
                        None
                    }
                }
            });

            let frag_source = std::fs::read_to_string(path).ok();
            let shader_program = frag_source
                .map(|s| {
                    let source = shader_source_no_extensions(&s);
                    match TranslationUnit::parse(source) {
                        Ok(fragment) => shader_from_glsl(vertex.as_ref(), &fragment),
                        Err(e) => {
                            error!("Error parsing {path:?}: {e}");
                            ShaderProgram::default()
                        }
                    }
                })
                .unwrap_or_default();
            (hash, shader_program)
        }));
    }
}

fn add_programs_legacy(programs: &mut BTreeMap<ProgramHash, ShaderProgram>, folder: &Path) {
    // Only check the first shader for now.
    // TODO: What do additional nvsd shader entries do?
    for path in globwalk::GlobWalkerBuilder::from_patterns(folder, &["*.cashd"])
        .build()
        .unwrap()
        .filter_map(|e| e.map(|e| e.path().to_owned()).ok())
    {
        let mths = Mths::from_file(&path).unwrap();

        let hash = ProgramHash::from_mths(&mths);
        // Avoid processing the same program more than once.
        programs.entry(hash).or_insert_with(|| {
            // TODO: Should both shaders be mandatory?
            let vertex_source = std::fs::read_to_string(path.with_extension("vert.txt")).unwrap();
            let frag_source = std::fs::read_to_string(path.with_extension("frag.txt")).unwrap();
            let fragment_shader = mths.fragment_shader().unwrap();
            shader_from_latte_asm(&vertex_source, &frag_source, &fragment_shader)
        });
    }
}

// TODO: module for this?
#[derive(Debug, Default)]
struct AttributeVisitor {
    attributes: Attributes,
}

#[derive(Debug, Default, PartialEq)]
pub struct Attributes {
    pub input_locations: BiBTreeMap<String, i32>,
    pub output_locations: BiBTreeMap<String, i32>,
}

impl Visitor for AttributeVisitor {
    fn visit_single_declaration(&mut self, declaration: &SingleDeclaration) -> Visit {
        if let Some(name) = &declaration.name {
            if let Some(qualifier) = &declaration.ty.content.qualifier {
                let mut is_input = None;
                let mut location = None;

                for q in &qualifier.qualifiers {
                    match &q.content {
                        TypeQualifierSpecData::Storage(storage) => match &storage.content {
                            StorageQualifierData::In => {
                                is_input = Some(true);
                            }
                            StorageQualifierData::Out => {
                                is_input = Some(false);
                            }
                            _ => (),
                        },
                        TypeQualifierSpecData::Layout(layout) => {
                            if let Some(id) = layout.content.ids.first() {
                                if let LayoutQualifierSpecData::Identifier(key, value) = &id.content
                                {
                                    if key.0 == "location" {
                                        if let Some(ExprData::IntConst(i)) =
                                            value.as_ref().map(|v| &v.content)
                                        {
                                            location = Some(*i);
                                        }
                                    }
                                }
                            }
                        }
                        _ => (),
                    }
                }

                if let (Some(is_input), Some(location)) = (is_input, location) {
                    if is_input {
                        self.attributes
                            .input_locations
                            .insert(name.0.to_string(), location);
                    } else {
                        self.attributes
                            .output_locations
                            .insert(name.0.to_string(), location);
                    }
                }
            }
        }

        Visit::Children
    }
}

pub fn find_attribute_locations(translation_unit: &TranslationUnit) -> Attributes {
    let mut visitor = AttributeVisitor::default();
    translation_unit.visit(&mut visitor);
    visitor.attributes
}

#[cfg(test)]
mod tests {
    use super::*;

    use indoc::indoc;
    use pretty_assertions::assert_eq;
    use smol_str::SmolStr;
    use xc3_model::shader_database::{
        AttributeDependency, BufferDependency, LayerBlendMode, TexCoord, TexCoordParams,
        TextureDependency,
    };

    #[test]
    fn find_attribute_locations_outputs() {
        let glsl = indoc! {"
            layout(location = 0) in vec4 in_attr0;
            layout(location = 4) in vec4 in_attr1;
            layout(location = 3) in vec4 in_attr2;

            layout(location = 3) out vec4 out_attr0;
            layout(location = 5) out vec4 out_attr1;
            layout(location = 7) out vec4 out_attr2;

            void main() {}
        "};

        let tu = TranslationUnit::parse(glsl).unwrap();
        assert_eq!(
            Attributes {
                input_locations: [
                    ("in_attr0".to_string(), 0),
                    ("in_attr1".to_string(), 4),
                    ("in_attr2".to_string(), 3)
                ]
                .into_iter()
                .collect(),
                output_locations: [
                    ("out_attr0".to_string(), 3),
                    ("out_attr1".to_string(), 5),
                    ("out_attr2".to_string(), 7)
                ]
                .into_iter()
                .collect(),
            },
            find_attribute_locations(&tu)
        );
    }

    #[test]
    fn shader_from_vertex_fragment_pyra_body() {
        // Test shaders from Pyra's metallic chest material.
        // xeno2/model/bl/bl000101, "ho_BL_TS2", shd0022.vert
        let glsl = include_str!("data/xc2/bl000101.22.vert");
        let vertex = TranslationUnit::parse(glsl).unwrap();

        // xeno2/model/bl/bl000101, "ho_BL_TS2", shd0022.frag
        let glsl = include_str!("data/xc2/bl000101.22.frag");
        let fragment = TranslationUnit::parse(glsl).unwrap();

        let shader = shader_from_glsl(Some(&vertex), &fragment);

        assert_eq!(
            OutputDependencies {
                dependencies: vec![Dependency::Texture(TextureDependency {
                    name: "s4".into(),
                    channel: Some('y'),
                    texcoords: vec![
                        TexCoord {
                            name: "vTex0".into(),
                            channel: Some('x'),
                            params: None,
                        },
                        TexCoord {
                            name: "vTex0".into(),
                            channel: Some('y'),
                            params: None,
                        },
                    ]
                })],
                layers: Vec::new()
            },
            shader.output_dependencies[&SmolStr::from("o1.x")]
        );
        assert_eq!(
            OutputDependencies {
                dependencies: vec![Dependency::Buffer(BufferDependency {
                    name: "U_Mate".into(),
                    field: "gWrkFl4".into(),
                    index: Some(2),
                    channel: Some('x'),
                })],
                layers: Vec::new()
            },
            shader.output_dependencies[&SmolStr::from("o1.y")]
        );
        assert_eq!(
            OutputDependencies {
                dependencies: vec![Dependency::Buffer(BufferDependency {
                    name: "U_Mate".into(),
                    field: "gWrkFl4".into(),
                    index: Some(1),
                    channel: Some('y'),
                })],
                layers: Vec::new()
            },
            shader.output_dependencies[&SmolStr::from("o1.z")]
        );
        assert_eq!(
            OutputDependencies {
                dependencies: vec![Dependency::Constant(0.07098039.into())],
                layers: Vec::new()
            },
            shader.output_dependencies[&SmolStr::from("o1.w")]
        );
        assert_eq!(
            OutputDependencies {
                dependencies: vec![Dependency::Texture(TextureDependency {
                    name: "s5".into(),
                    channel: Some('x'),
                    texcoords: vec![
                        TexCoord {
                            name: "vTex0".into(),
                            channel: Some('x'),
                            params: Some(TexCoordParams::Scale(BufferDependency {
                                name: "U_Mate".into(),
                                field: "gWrkFl4".into(),
                                index: Some(0),
                                channel: Some('x'),
                            }))
                        },
                        TexCoord {
                            name: "vTex0".into(),
                            channel: Some('y'),
                            params: Some(TexCoordParams::Scale(BufferDependency {
                                name: "U_Mate".into(),
                                field: "gWrkFl4".into(),
                                index: Some(0),
                                channel: Some('y'),
                            }))
                        },
                    ],
                })],
                layers: Vec::new()
            },
            shader.output_dependencies[&SmolStr::from("o5.x")]
        );
        assert_eq!(
            OutputDependencies {
                dependencies: vec![Dependency::Texture(TextureDependency {
                    name: "s5".into(),
                    channel: Some('y'),
                    texcoords: vec![
                        TexCoord {
                            name: "vTex0".into(),
                            channel: Some('x'),
                            params: Some(TexCoordParams::Scale(BufferDependency {
                                name: "U_Mate".into(),
                                field: "gWrkFl4".into(),
                                index: Some(0),
                                channel: Some('x'),
                            }))
                        },
                        TexCoord {
                            name: "vTex0".into(),
                            channel: Some('y'),
                            params: Some(TexCoordParams::Scale(BufferDependency {
                                name: "U_Mate".into(),
                                field: "gWrkFl4".into(),
                                index: Some(0),
                                channel: Some('y'),
                            }))
                        },
                    ],
                })],
                layers: Vec::new()
            },
            shader.output_dependencies[&SmolStr::from("o5.y")]
        );
        assert_eq!(
            OutputDependencies {
                dependencies: vec![Dependency::Texture(TextureDependency {
                    name: "s5".into(),
                    channel: Some('z'),
                    texcoords: vec![
                        TexCoord {
                            name: "vTex0".into(),
                            channel: Some('x'),
                            params: Some(TexCoordParams::Scale(BufferDependency {
                                name: "U_Mate".into(),
                                field: "gWrkFl4".into(),
                                index: Some(0),
                                channel: Some('x'),
                            }))
                        },
                        TexCoord {
                            name: "vTex0".into(),
                            channel: Some('y'),
                            params: Some(TexCoordParams::Scale(BufferDependency {
                                name: "U_Mate".into(),
                                field: "gWrkFl4".into(),
                                index: Some(0),
                                channel: Some('y'),
                            }))
                        },
                    ],
                })],
                layers: Vec::new()
            },
            shader.output_dependencies[&SmolStr::from("o5.z")]
        );
        assert_eq!(
            OutputDependencies {
                dependencies: vec![Dependency::Constant(0.0.into())],
                layers: Vec::new()
            },
            shader.output_dependencies[&SmolStr::from("o5.w")]
        );
    }

    #[test]
    fn shader_from_fragment_pyra_hair() {
        // xeno2/model/bl/bl000101, "_ho_hair_new", shd0008.vert
        let glsl = include_str!("data/xc2/bl000101.8.frag");
        let fragment = TranslationUnit::parse(glsl).unwrap();

        let shader = shader_from_glsl(None, &fragment);

        // Check that the color texture is multiplied by vertex color.
        assert_eq!(
            OutputDependencies {
                dependencies: vec![Dependency::Texture(TextureDependency {
                    name: "s0".into(),
                    channel: Some('x'),
                    texcoords: vec![
                        TexCoord {
                            name: "in_attr2".into(),
                            channel: Some('x'),
                            params: None,
                        },
                        TexCoord {
                            name: "in_attr2".into(),
                            channel: Some('y'),
                            params: None,
                        },
                    ]
                })],
                layers: vec![
                    TextureLayer {
                        value: Dependency::Texture(TextureDependency {
                            name: "s0".into(),
                            channel: Some('x'),
                            texcoords: vec![
                                TexCoord {
                                    name: "in_attr2".into(),
                                    channel: Some('x'),
                                    params: None,
                                },
                                TexCoord {
                                    name: "in_attr2".into(),
                                    channel: Some('y'),
                                    params: None,
                                },
                            ],
                        }),
                        ratio: None,
                        blend_mode: LayerBlendMode::Mix,
                        is_fresnel: false,
                    },
                    TextureLayer {
                        value: Dependency::Attribute(AttributeDependency {
                            name: "in_attr3".into(),
                            channel: Some('x'),
                        }),
                        ratio: Some(Dependency::Constant(1.0.into())),
                        blend_mode: LayerBlendMode::MixRatio,
                        is_fresnel: false,
                    },
                ],
            },
            shader.output_dependencies[&SmolStr::from("o0.x")]
        );
    }

    #[test]
    fn shader_from_fragment_mio_skirt() {
        // xeno3/chr/ch/ch11021013, "body_skert2", shd0028.frag
        let glsl = include_str!("data/xc3/ch11021013.28.frag");

        // The pcmdo calcGeometricSpecularAA function compiles to the expression
        // glossiness = 1.0 - sqrt(clamp((1.0 - glossiness)^2 + kernelRoughness2 0.0, 1.0))
        // Consuming applications only care about the glossiness input.
        // This also avoids considering normal maps as a dependency.
        let fragment = TranslationUnit::parse(glsl).unwrap();
        let shader = shader_from_glsl(None, &fragment);
        assert_eq!(
            vec![
                TextureLayer {
                    value: Dependency::Texture(TextureDependency {
                        name: "s0".into(),
                        channel: Some('x'),
                        texcoords: vec![
                            TexCoord {
                                name: "in_attr3".into(),
                                channel: Some('x'),
                                params: None
                            },
                            TexCoord {
                                name: "in_attr3".into(),
                                channel: Some('y'),
                                params: None
                            }
                        ]
                    }),
                    ratio: None,
                    blend_mode: LayerBlendMode::Mix,
                    is_fresnel: false
                },
                TextureLayer {
                    value: Dependency::Texture(TextureDependency {
                        name: "gTResidentTex04".into(),
                        channel: Some('x'),
                        texcoords: vec![
                            TexCoord {
                                name: "in_attr4".into(),
                                channel: Some('x'),
                                params: None
                            },
                            TexCoord {
                                name: "in_attr4".into(),
                                channel: Some('y'),
                                params: None
                            }
                        ]
                    }),
                    ratio: None,
                    blend_mode: LayerBlendMode::Mix,
                    is_fresnel: false
                }
            ],
            shader.output_dependencies[&SmolStr::from("o0.x")].layers
        );
        assert_eq!(
            vec![
                TextureLayer {
                    value: Dependency::Texture(TextureDependency {
                        name: "s0".into(),
                        channel: Some('y'),
                        texcoords: vec![
                            TexCoord {
                                name: "in_attr3".into(),
                                channel: Some('x'),
                                params: None
                            },
                            TexCoord {
                                name: "in_attr3".into(),
                                channel: Some('y'),
                                params: None
                            }
                        ]
                    }),
                    ratio: None,
                    blend_mode: LayerBlendMode::Mix,
                    is_fresnel: false
                },
                TextureLayer {
                    value: Dependency::Texture(TextureDependency {
                        name: "gTResidentTex04".into(),
                        channel: Some('y'),
                        texcoords: vec![
                            TexCoord {
                                name: "in_attr4".into(),
                                channel: Some('x'),
                                params: None
                            },
                            TexCoord {
                                name: "in_attr4".into(),
                                channel: Some('y'),
                                params: None
                            }
                        ]
                    }),
                    ratio: None,
                    blend_mode: LayerBlendMode::Mix,
                    is_fresnel: false
                }
            ],
            shader.output_dependencies[&SmolStr::from("o0.y")].layers
        );
        assert_eq!(
            vec![
                TextureLayer {
                    value: Dependency::Texture(TextureDependency {
                        name: "s0".into(),
                        channel: Some('z'),
                        texcoords: vec![
                            TexCoord {
                                name: "in_attr3".into(),
                                channel: Some('x'),
                                params: None
                            },
                            TexCoord {
                                name: "in_attr3".into(),
                                channel: Some('y'),
                                params: None
                            }
                        ]
                    }),
                    ratio: None,
                    blend_mode: LayerBlendMode::Mix,
                    is_fresnel: false
                },
                TextureLayer {
                    value: Dependency::Texture(TextureDependency {
                        name: "gTResidentTex04".into(),
                        channel: Some('z'),
                        texcoords: vec![
                            TexCoord {
                                name: "in_attr4".into(),
                                channel: Some('x'),
                                params: None
                            },
                            TexCoord {
                                name: "in_attr4".into(),
                                channel: Some('y'),
                                params: None
                            }
                        ]
                    }),
                    ratio: None,
                    blend_mode: LayerBlendMode::Mix,
                    is_fresnel: false
                }
            ],
            shader.output_dependencies[&SmolStr::from("o0.z")].layers
        );
        assert_eq!(
            vec![
                TextureLayer {
                    value: Dependency::Texture(TextureDependency {
                        name: "s2".into(),
                        channel: Some('x'),
                        texcoords: vec![
                            TexCoord {
                                name: "in_attr3".into(),
                                channel: Some('x'),
                                params: None
                            },
                            TexCoord {
                                name: "in_attr3".into(),
                                channel: Some('y'),
                                params: None
                            }
                        ]
                    }),
                    ratio: None,
                    blend_mode: LayerBlendMode::Add,
                    is_fresnel: false
                },
                TextureLayer {
                    value: Dependency::Texture(TextureDependency {
                        name: "gTResidentTex09".into(),
                        channel: Some('x'),
                        texcoords: vec![
                            TexCoord {
                                name: "in_attr3".into(),
                                channel: Some('z'),
                                params: None
                            },
                            TexCoord {
                                name: "in_attr3".into(),
                                channel: Some('w'),
                                params: None
                            }
                        ]
                    }),
                    ratio: Some(Dependency::Buffer(BufferDependency {
                        name: "U_Mate".into(),
                        field: "gWrkFl4".into(),
                        index: Some(1),
                        channel: Some('z')
                    })),
                    blend_mode: LayerBlendMode::AddNormal,
                    is_fresnel: false
                }
            ],
            shader.output_dependencies[&SmolStr::from("o2.x")].layers
        );

        assert_eq!(
            OutputDependencies {
                dependencies: vec![Dependency::Buffer(BufferDependency {
                    name: "U_Mate".into(),
                    field: "gWrkFl4".into(),
                    index: Some(2),
                    channel: Some('y')
                })],
                layers: Vec::new()
            },
            shader.output_dependencies[&SmolStr::from("o1.y")]
        );
    }

    #[test]
    fn shader_from_fragment_mio_metal() {
        // xeno3/chr/ch/ch11021013, "tlent_mio_metal1", shd0031.frag
        let glsl = include_str!("data/xc3/ch11021013.31.frag");

        // Test multiple calls to getPixelCalcAddNormal.
        let fragment = TranslationUnit::parse(glsl).unwrap();
        let shader = shader_from_glsl(None, &fragment);
        assert_eq!(
            vec![
                TextureLayer {
                    value: Dependency::Texture(TextureDependency {
                        name: "s0".into(),
                        channel: Some('x'),
                        texcoords: vec![
                            TexCoord {
                                name: "in_attr4".into(),
                                channel: Some('x'),
                                params: None
                            },
                            TexCoord {
                                name: "in_attr4".into(),
                                channel: Some('y'),
                                params: None
                            }
                        ]
                    }),
                    ratio: None,
                    blend_mode: LayerBlendMode::Mix,
                    is_fresnel: false
                },
                TextureLayer {
                    value: Dependency::Buffer(BufferDependency {
                        name: "U_Mate".into(),
                        field: "gWrkCol".into(),
                        index: Some(1),
                        channel: Some('x'),
                    }),
                    ratio: Some(Dependency::Buffer(BufferDependency {
                        name: "U_Mate".into(),
                        field: "gWrkFl4".into(),
                        index: Some(1),
                        channel: Some('z'),
                    })),
                    blend_mode: LayerBlendMode::Mix,
                    is_fresnel: true
                },
                TextureLayer {
                    value: Dependency::Texture(TextureDependency {
                        name: "gTResidentTex04".into(),
                        channel: Some('x'),
                        texcoords: vec![
                            TexCoord {
                                name: "in_attr5".into(),
                                channel: Some('z'),
                                params: None
                            },
                            TexCoord {
                                name: "in_attr5".into(),
                                channel: Some('w'),
                                params: None
                            }
                        ]
                    }),
                    ratio: None,
                    blend_mode: LayerBlendMode::Mix,
                    is_fresnel: false
                }
            ],
            shader.output_dependencies[&SmolStr::from("o0.x")].layers
        );
        assert_eq!(
            OutputDependencies {
                dependencies: vec![Dependency::Buffer(BufferDependency {
                    name: "U_Mate".into(),
                    field: "gWrkFl4".into(),
                    index: Some(3),
                    channel: Some('y')
                })],
                layers: Vec::new()
            },
            shader.output_dependencies[&SmolStr::from("o1.y")]
        );
        assert_eq!(
            OutputDependencies {
                dependencies: vec![
                    Dependency::Texture(TextureDependency {
                        name: "gTResidentTex09".into(),
                        channel: Some('x'),
                        texcoords: vec![
                            TexCoord {
                                name: "in_attr4".into(),
                                channel: Some('z'),
                                params: None,
                            },
                            TexCoord {
                                name: "in_attr4".into(),
                                channel: Some('w'),
                                params: None,
                            },
                        ],
                    }),
                    Dependency::Texture(TextureDependency {
                        name: "gTResidentTex09".into(),
                        channel: Some('y'),
                        texcoords: vec![
                            TexCoord {
                                name: "in_attr4".into(),
                                channel: Some('z'),
                                params: None,
                            },
                            TexCoord {
                                name: "in_attr4".into(),
                                channel: Some('w'),
                                params: None,
                            },
                        ],
                    }),
                    Dependency::Texture(TextureDependency {
                        name: "s2".into(),
                        channel: Some('x'),
                        texcoords: vec![
                            TexCoord {
                                name: "in_attr4".into(),
                                channel: Some('x'),
                                params: None,
                            },
                            TexCoord {
                                name: "in_attr4".into(),
                                channel: Some('y'),
                                params: None,
                            },
                        ],
                    }),
                    Dependency::Texture(TextureDependency {
                        name: "s2".into(),
                        channel: Some('y'),
                        texcoords: vec![
                            TexCoord {
                                name: "in_attr4".into(),
                                channel: Some('x'),
                                params: None,
                            },
                            TexCoord {
                                name: "in_attr4".into(),
                                channel: Some('y'),
                                params: None,
                            },
                        ],
                    }),
                    Dependency::Texture(TextureDependency {
                        name: "s3".into(),
                        channel: Some('x'),
                        texcoords: vec![
                            TexCoord {
                                name: "in_attr5".into(),
                                channel: Some('x'),
                                params: None,
                            },
                            TexCoord {
                                name: "in_attr5".into(),
                                channel: Some('y'),
                                params: None,
                            },
                        ],
                    }),
                    Dependency::Texture(TextureDependency {
                        name: "s3".into(),
                        channel: Some('y'),
                        texcoords: vec![
                            TexCoord {
                                name: "in_attr5".into(),
                                channel: Some('x'),
                                params: None,
                            },
                            TexCoord {
                                name: "in_attr5".into(),
                                channel: Some('y'),
                                params: None,
                            },
                        ],
                    }),
                ],
                layers: vec![
                    TextureLayer {
                        value: Dependency::Texture(TextureDependency {
                            name: "s2".into(),
                            channel: Some('x'),
                            texcoords: vec![
                                TexCoord {
                                    name: "in_attr4".into(),
                                    channel: Some('x'),
                                    params: None
                                },
                                TexCoord {
                                    name: "in_attr4".into(),
                                    channel: Some('y'),
                                    params: None
                                }
                            ]
                        }),
                        ratio: None,
                        blend_mode: LayerBlendMode::Add,
                        is_fresnel: false
                    },
                    TextureLayer {
                        value: Dependency::Texture(TextureDependency {
                            name: "gTResidentTex09".into(),
                            channel: Some('x'),
                            texcoords: vec![
                                TexCoord {
                                    name: "in_attr4".into(),
                                    channel: Some('z'),
                                    params: None
                                },
                                TexCoord {
                                    name: "in_attr4".into(),
                                    channel: Some('w'),
                                    params: None
                                }
                            ]
                        }),
                        ratio: Some(Dependency::Buffer(BufferDependency {
                            name: "U_Mate".into(),
                            field: "gWrkFl4".into(),
                            index: Some(2),
                            channel: Some('y')
                        })),
                        blend_mode: LayerBlendMode::AddNormal,
                        is_fresnel: false
                    },
                    TextureLayer {
                        value: Dependency::Texture(TextureDependency {
                            name: "s3".into(),
                            channel: Some('x'),
                            texcoords: vec![
                                TexCoord {
                                    name: "in_attr5".into(),
                                    channel: Some('x'),
                                    params: None
                                },
                                TexCoord {
                                    name: "in_attr5".into(),
                                    channel: Some('y'),
                                    params: None
                                }
                            ]
                        }),
                        ratio: Some(Dependency::Buffer(BufferDependency {
                            name: "U_Mate".into(),
                            field: "gWrkFl4".into(),
                            index: Some(2),
                            channel: Some('z')
                        })),
                        blend_mode: LayerBlendMode::AddNormal,
                        is_fresnel: false
                    },
                ],
            },
            shader.output_dependencies[&SmolStr::from("o2.x")]
        );
    }

    #[test]
    fn shader_from_fragment_mio_legs() {
        // xeno3/chr/ch/ch11021013, "body_stking1", shd0016.frag
        let glsl = include_str!("data/xc3/ch11021013.16.frag");

        // Test that color layers use the appropriate fresnel blending mode.
        let fragment = TranslationUnit::parse(glsl).unwrap();
        let shader = shader_from_glsl(None, &fragment);
        assert_eq!(
            vec![
                TextureLayer {
                    value: Dependency::Texture(TextureDependency {
                        name: "s0".into(),
                        channel: Some('x'),
                        texcoords: vec![
                            TexCoord {
                                name: "in_attr4".into(),
                                channel: Some('x'),
                                params: None
                            },
                            TexCoord {
                                name: "in_attr4".into(),
                                channel: Some('y'),
                                params: None
                            }
                        ]
                    }),
                    ratio: None,
                    blend_mode: LayerBlendMode::Mix,
                    is_fresnel: false
                },
                TextureLayer {
                    value: Dependency::Buffer(BufferDependency {
                        name: "U_Mate".into(),
                        field: "gWrkCol".into(),
                        index: Some(1),
                        channel: Some('x'),
                    }),
                    ratio: Some(Dependency::Buffer(BufferDependency {
                        name: "U_Mate".into(),
                        field: "gWrkFl4".into(),
                        index: Some(0),
                        channel: Some('z'),
                    })),
                    blend_mode: LayerBlendMode::Mix,
                    is_fresnel: true
                },
                TextureLayer {
                    value: Dependency::Texture(TextureDependency {
                        name: "gTResidentTex04".into(),
                        channel: Some('x'),
                        texcoords: vec![
                            TexCoord {
                                name: "in_attr4".into(),
                                channel: Some('z'),
                                params: None
                            },
                            TexCoord {
                                name: "in_attr4".into(),
                                channel: Some('w'),
                                params: None
                            }
                        ]
                    }),
                    ratio: None,
                    blend_mode: LayerBlendMode::Mix,
                    is_fresnel: false
                }
            ],
            shader.output_dependencies[&SmolStr::from("o0.x")].layers
        );
        assert_eq!(
            OutputDependencies {
                dependencies: vec![Dependency::Buffer(BufferDependency {
                    name: "U_Mate".into(),
                    field: "gWrkFl4".into(),
                    index: Some(1),
                    channel: Some('w')
                })],
                layers: Vec::new()
            },
            shader.output_dependencies[&SmolStr::from("o1.y")]
        );
        assert_eq!(
            OutputDependencies {
                dependencies: vec![
                    Dependency::Texture(TextureDependency {
                        name: "s2".into(),
                        channel: Some('x'),
                        texcoords: vec![
                            TexCoord {
                                name: "in_attr4".into(),
                                channel: Some('x'),
                                params: None,
                            },
                            TexCoord {
                                name: "in_attr4".into(),
                                channel: Some('y'),
                                params: None,
                            },
                        ],
                    }),
                    Dependency::Texture(TextureDependency {
                        name: "s2".into(),
                        channel: Some('y'),
                        texcoords: vec![
                            TexCoord {
                                name: "in_attr4".into(),
                                channel: Some('x'),
                                params: None,
                            },
                            TexCoord {
                                name: "in_attr4".into(),
                                channel: Some('y'),
                                params: None,
                            },
                        ],
                    }),
                ],
                layers: Vec::new()
            },
            shader.output_dependencies[&SmolStr::from("o2.x")]
        );
    }

    #[test]
    fn shader_from_fragment_mio_eyes() {
        // xeno3/chr/ch/ch01021011, "eye4", shd0063.frag
        let glsl = include_str!("data/xc3/ch01021011.63.frag");

        // Detect parallax mapping for texture coordinates.
        let fragment = TranslationUnit::parse(glsl).unwrap();
        let shader = shader_from_glsl(None, &fragment);
        assert_eq!(
            Dependency::Texture(TextureDependency {
                name: "s0".into(),
                channel: Some('x'),
                texcoords: vec![
                    TexCoord {
                        name: "in_attr3".into(),
                        channel: Some('x'),
                        params: Some(TexCoordParams::Parallax {
                            mask_a: Dependency::Buffer(BufferDependency {
                                name: "U_Mate".into(),
                                field: "gWrkFl4".into(),
                                index: Some(0),
                                channel: Some('x'),
                            }),
                            mask_b: Dependency::Texture(TextureDependency {
                                name: "s2".into(),
                                channel: Some('z'),
                                texcoords: vec![
                                    TexCoord {
                                        name: "in_attr3".into(),
                                        channel: Some('x'),
                                        params: None,
                                    },
                                    TexCoord {
                                        name: "in_attr3".into(),
                                        channel: Some('y'),
                                        params: None,
                                    },
                                ],
                            }),
                            ratio: BufferDependency {
                                name: "U_Mate".into(),
                                field: "gWrkFl4".into(),
                                index: Some(0),
                                channel: Some('z'),
                            },
                        }),
                    },
                    TexCoord {
                        name: "in_attr3".into(),
                        channel: Some('y'),
                        params: Some(TexCoordParams::Parallax {
                            mask_a: Dependency::Buffer(BufferDependency {
                                name: "U_Mate".into(),
                                field: "gWrkFl4".into(),
                                index: Some(0),
                                channel: Some('x'),
                            }),
                            mask_b: Dependency::Texture(TextureDependency {
                                name: "s2".into(),
                                channel: Some('z'),
                                texcoords: vec![
                                    TexCoord {
                                        name: "in_attr3".into(),
                                        channel: Some('x'),
                                        params: None,
                                    },
                                    TexCoord {
                                        name: "in_attr3".into(),
                                        channel: Some('y'),
                                        params: None,
                                    },
                                ],
                            }),
                            ratio: BufferDependency {
                                name: "U_Mate".into(),
                                field: "gWrkFl4".into(),
                                index: Some(0),
                                channel: Some('z'),
                            },
                        }),
                    },
                ],
            }),
            shader.output_dependencies[&SmolStr::from("o0.x")].dependencies[0]
        );
    }

    #[test]
    fn shader_from_fragment_wild_ride_body() {
        // xeno3/chr/ch/ch02010110, "body_m", shd0028.frag
        let glsl = include_str!("data/xc3/ch02010110.28.frag");

        // Some shaders use a simple mix() for normal blending.
        let fragment = TranslationUnit::parse(glsl).unwrap();
        let shader = shader_from_glsl(None, &fragment);
        assert!(shader.output_dependencies[&SmolStr::from("o0.x")]
            .layers
            .is_empty());
        assert_eq!(
            OutputDependencies {
                dependencies: vec![Dependency::Texture(TextureDependency {
                    name: "s8".into(),
                    channel: Some('x'),
                    texcoords: vec![
                        TexCoord {
                            name: "in_attr3".into(),
                            channel: Some('x'),
                            params: None,
                        },
                        TexCoord {
                            name: "in_attr3".into(),
                            channel: Some('y'),
                            params: None,
                        },
                    ],
                })],
                layers: Vec::new()
            },
            shader.output_dependencies[&SmolStr::from("o1.y")]
        );
        assert_eq!(
            vec![
                TextureLayer {
                    value: Dependency::Texture(TextureDependency {
                        name: "s6".into(),
                        channel: Some('x'),
                        texcoords: vec![
                            TexCoord {
                                name: "in_attr3".into(),
                                channel: Some('x'),
                                params: None
                            },
                            TexCoord {
                                name: "in_attr3".into(),
                                channel: Some('y'),
                                params: None
                            }
                        ]
                    }),
                    ratio: None,
                    blend_mode: LayerBlendMode::Add,
                    is_fresnel: false
                },
                TextureLayer {
                    value: Dependency::Texture(TextureDependency {
                        name: "s7".into(),
                        channel: Some('x'),
                        texcoords: vec![
                            TexCoord {
                                name: "in_attr3".into(),
                                channel: Some('z'),
                                params: None
                            },
                            TexCoord {
                                name: "in_attr3".into(),
                                channel: Some('w'),
                                params: None
                            }
                        ]
                    }),
                    ratio: Some(Dependency::Texture(TextureDependency {
                        name: "s1".into(),
                        channel: Some('x'),
                        texcoords: vec![
                            TexCoord {
                                name: "in_attr3".into(),
                                channel: Some('x'),
                                params: None,
                            },
                            TexCoord {
                                name: "in_attr3".into(),
                                channel: Some('y'),
                                params: None,
                            },
                        ],
                    })),
                    blend_mode: LayerBlendMode::Mix,
                    is_fresnel: false
                }
            ],
            shader.output_dependencies[&SmolStr::from("o2.x")].layers
        );
        assert_eq!(
            vec![
                TextureLayer {
                    value: Dependency::Texture(TextureDependency {
                        name: "s6".into(),
                        channel: Some('y'),
                        texcoords: vec![
                            TexCoord {
                                name: "in_attr3".into(),
                                channel: Some('x'),
                                params: None
                            },
                            TexCoord {
                                name: "in_attr3".into(),
                                channel: Some('y'),
                                params: None
                            }
                        ]
                    }),
                    ratio: None,
                    blend_mode: LayerBlendMode::Add,
                    is_fresnel: false
                },
                TextureLayer {
                    value: Dependency::Texture(TextureDependency {
                        name: "s7".into(),
                        channel: Some('y'),
                        texcoords: vec![
                            TexCoord {
                                name: "in_attr3".into(),
                                channel: Some('z'),
                                params: None
                            },
                            TexCoord {
                                name: "in_attr3".into(),
                                channel: Some('w'),
                                params: None
                            }
                        ]
                    }),
                    ratio: Some(Dependency::Texture(TextureDependency {
                        name: "s1".into(),
                        channel: Some('x'),
                        texcoords: vec![
                            TexCoord {
                                name: "in_attr3".into(),
                                channel: Some('x'),
                                params: None,
                            },
                            TexCoord {
                                name: "in_attr3".into(),
                                channel: Some('y'),
                                params: None,
                            },
                        ],
                    })),
                    blend_mode: LayerBlendMode::Mix,
                    is_fresnel: false
                }
            ],
            shader.output_dependencies[&SmolStr::from("o2.y")].layers
        );
    }

    #[test]
    fn shader_from_fragment_sena_body() {
        // xeno3/chr/ch/ch11061013, "bodydenim_toon", shd0009.frag
        let glsl = include_str!("data/xc3/ch11061013.9.frag");

        // Some shaders use multiple color blending modes.
        let fragment = TranslationUnit::parse(glsl).unwrap();
        let shader = shader_from_glsl(None, &fragment);
        assert_eq!(
            vec![
                TextureLayer {
                    value: Dependency::Texture(TextureDependency {
                        name: "s0".into(),
                        channel: Some('x'),
                        texcoords: vec![
                            TexCoord {
                                name: "in_attr4".into(),
                                channel: Some('x'),
                                params: None
                            },
                            TexCoord {
                                name: "in_attr4".into(),
                                channel: Some('y'),
                                params: None
                            }
                        ]
                    }),
                    ratio: None,
                    blend_mode: LayerBlendMode::Mix,
                    is_fresnel: false
                },
                TextureLayer {
                    value: Dependency::Texture(TextureDependency {
                        name: "gTResidentTex03".into(),
                        channel: Some('x'),
                        texcoords: vec![
                            TexCoord {
                                name: "in_attr4".into(),
                                channel: Some('x'),
                                params: None
                            },
                            TexCoord {
                                name: "in_attr4".into(),
                                channel: Some('x'),
                                params: None
                            }
                        ]
                    }),
                    ratio: Some(Dependency::Texture(TextureDependency {
                        name: "s1".into(),
                        channel: Some('x'),
                        texcoords: vec![
                            TexCoord {
                                name: "in_attr4".into(),
                                channel: Some('x'),
                                params: None,
                            },
                            TexCoord {
                                name: "in_attr4".into(),
                                channel: Some('y'),
                                params: None,
                            },
                        ],
                    })),
                    blend_mode: LayerBlendMode::Add,
                    is_fresnel: false
                },
                TextureLayer {
                    value: Dependency::Texture(TextureDependency {
                        name: "gTResidentTex04".into(),
                        channel: Some('x'),
                        texcoords: vec![
                            TexCoord {
                                name: "in_attr5".into(),
                                channel: Some('x'),
                                params: None
                            },
                            TexCoord {
                                name: "in_attr5".into(),
                                channel: Some('y'),
                                params: None
                            }
                        ]
                    }),
                    ratio: None,
                    blend_mode: LayerBlendMode::Mix,
                    is_fresnel: false
                },
            ],
            shader.output_dependencies[&SmolStr::from("o0.x")].layers
        );

        assert_eq!(
            OutputDependencies {
                dependencies: vec![Dependency::Texture(TextureDependency {
                    name: "s3".into(),
                    channel: Some('x'),
                    texcoords: vec![
                        TexCoord {
                            name: "in_attr4".into(),
                            channel: Some('x'),
                            params: None,
                        },
                        TexCoord {
                            name: "in_attr4".into(),
                            channel: Some('y'),
                            params: None,
                        },
                    ],
                })],
                layers: Vec::new()
            },
            shader.output_dependencies[&SmolStr::from("o1.y")]
        );
        assert_eq!(
            vec![
                TextureLayer {
                    value: Dependency::Texture(TextureDependency {
                        name: "s2".into(),
                        channel: Some('x'),
                        texcoords: vec![
                            TexCoord {
                                name: "in_attr4".into(),
                                channel: Some('x'),
                                params: None
                            },
                            TexCoord {
                                name: "in_attr4".into(),
                                channel: Some('y'),
                                params: None
                            }
                        ]
                    }),
                    ratio: None,
                    blend_mode: LayerBlendMode::Add,
                    is_fresnel: false
                },
                TextureLayer {
                    value: Dependency::Texture(TextureDependency {
                        name: "gTResidentTex09".into(),
                        channel: Some('x'),
                        texcoords: vec![
                            TexCoord {
                                name: "in_attr4".into(),
                                channel: Some('z'),
                                params: None
                            },
                            TexCoord {
                                name: "in_attr4".into(),
                                channel: Some('w'),
                                params: None
                            }
                        ]
                    }),
                    ratio: Some(Dependency::Buffer(BufferDependency {
                        name: "U_Mate".into(),
                        field: "gWrkFl4".into(),
                        index: Some(2),
                        channel: Some('x')
                    })),
                    blend_mode: LayerBlendMode::AddNormal,
                    is_fresnel: false
                }
            ],
            shader.output_dependencies[&SmolStr::from("o2.x")].layers
        );
    }

    #[test]
    fn shader_from_fragment_haze_body() {
        // xeno2/model/np/np001101, "body", shd0013.frag
        let glsl = include_str!("data/xc2/np001101.13.frag");

        // Test multiple normal layers with texture masks.
        let fragment = TranslationUnit::parse(glsl).unwrap();
        let shader = shader_from_glsl(None, &fragment);
        assert_eq!(
            OutputDependencies {
                dependencies: vec![
                    Dependency::Texture(TextureDependency {
                        name: "s2".into(),
                        channel: Some('x'),
                        texcoords: vec![
                            TexCoord {
                                name: "in_attr4".into(),
                                channel: Some('x'),
                                params: None,
                            },
                            TexCoord {
                                name: "in_attr4".into(),
                                channel: Some('y'),
                                params: None,
                            },
                        ],
                    }),
                    Dependency::Texture(TextureDependency {
                        name: "s2".into(),
                        channel: Some('y'),
                        texcoords: vec![
                            TexCoord {
                                name: "in_attr4".into(),
                                channel: Some('x'),
                                params: None,
                            },
                            TexCoord {
                                name: "in_attr4".into(),
                                channel: Some('y'),
                                params: None,
                            },
                        ],
                    }),
                    Dependency::Texture(TextureDependency {
                        name: "s3".into(),
                        channel: Some('x'),
                        texcoords: vec![
                            TexCoord {
                                name: "in_attr4".into(),
                                channel: Some('z'),
                                params: None,
                            },
                            TexCoord {
                                name: "in_attr4".into(),
                                channel: Some('w'),
                                params: None,
                            },
                        ],
                    }),
                    Dependency::Texture(TextureDependency {
                        name: "s3".into(),
                        channel: Some('y'),
                        texcoords: vec![
                            TexCoord {
                                name: "in_attr4".into(),
                                channel: Some('z'),
                                params: None,
                            },
                            TexCoord {
                                name: "in_attr4".into(),
                                channel: Some('w'),
                                params: None,
                            },
                        ],
                    }),
                    Dependency::Texture(TextureDependency {
                        name: "s4".into(),
                        channel: Some('x'),
                        texcoords: vec![
                            TexCoord {
                                name: "in_attr4".into(),
                                channel: Some('x'),
                                params: None,
                            },
                            TexCoord {
                                name: "in_attr4".into(),
                                channel: Some('y'),
                                params: None,
                            },
                        ],
                    }),
                    Dependency::Texture(TextureDependency {
                        name: "s5".into(),
                        channel: Some('x'),
                        texcoords: vec![
                            TexCoord {
                                name: "in_attr5".into(),
                                channel: Some('x'),
                                params: None,
                            },
                            TexCoord {
                                name: "in_attr5".into(),
                                channel: Some('y'),
                                params: None,
                            },
                        ],
                    }),
                    Dependency::Texture(TextureDependency {
                        name: "s5".into(),
                        channel: Some('y'),
                        texcoords: vec![
                            TexCoord {
                                name: "in_attr5".into(),
                                channel: Some('x'),
                                params: None,
                            },
                            TexCoord {
                                name: "in_attr5".into(),
                                channel: Some('y'),
                                params: None,
                            },
                        ],
                    }),
                    Dependency::Texture(TextureDependency {
                        name: "s6".into(),
                        channel: Some('x'),
                        texcoords: vec![
                            TexCoord {
                                name: "in_attr4".into(),
                                channel: Some('x'),
                                params: None,
                            },
                            TexCoord {
                                name: "in_attr4".into(),
                                channel: Some('y'),
                                params: None,
                            },
                        ],
                    }),
                ],
                layers: vec![
                    TextureLayer {
                        value: Dependency::Texture(TextureDependency {
                            name: "s2".into(),
                            channel: Some('x'),
                            texcoords: vec![
                                TexCoord {
                                    name: "in_attr4".into(),
                                    channel: Some('x'),
                                    params: None,
                                },
                                TexCoord {
                                    name: "in_attr4".into(),
                                    channel: Some('y'),
                                    params: None,
                                },
                            ],
                        }),
                        ratio: None,
                        blend_mode: LayerBlendMode::Add,
                        is_fresnel: false,
                    },
                    TextureLayer {
                        value: Dependency::Texture(TextureDependency {
                            name: "s3".into(),
                            channel: Some('x'),
                            texcoords: vec![
                                TexCoord {
                                    name: "in_attr4".into(),
                                    channel: Some('z'),
                                    params: None,
                                },
                                TexCoord {
                                    name: "in_attr4".into(),
                                    channel: Some('w'),
                                    params: None,
                                },
                            ],
                        }),
                        ratio: Some(Dependency::Texture(TextureDependency {
                            name: "s4".into(),
                            channel: Some('x'),
                            texcoords: vec![
                                TexCoord {
                                    name: "in_attr4".into(),
                                    channel: Some('x'),
                                    params: None,
                                },
                                TexCoord {
                                    name: "in_attr4".into(),
                                    channel: Some('y'),
                                    params: None,
                                },
                            ],
                        })),
                        blend_mode: LayerBlendMode::AddNormal,
                        is_fresnel: false,
                    },
                    TextureLayer {
                        value: Dependency::Texture(TextureDependency {
                            name: "s5".into(),
                            channel: Some('x'),
                            texcoords: vec![
                                TexCoord {
                                    name: "in_attr5".into(),
                                    channel: Some('x'),
                                    params: None,
                                },
                                TexCoord {
                                    name: "in_attr5".into(),
                                    channel: Some('y'),
                                    params: None,
                                },
                            ],
                        }),
                        ratio: Some(Dependency::Texture(TextureDependency {
                            name: "s6".into(),
                            channel: Some('x'),
                            texcoords: vec![
                                TexCoord {
                                    name: "in_attr4".into(),
                                    channel: Some('x'),
                                    params: None,
                                },
                                TexCoord {
                                    name: "in_attr4".into(),
                                    channel: Some('y'),
                                    params: None,
                                },
                            ],
                        })),
                        blend_mode: LayerBlendMode::AddNormal,
                        is_fresnel: false,
                    },
                ],
            },
            shader.output_dependencies[&SmolStr::from("o2.x")]
        );
    }

    #[test]
    fn shader_from_fragment_pneuma_chest() {
        // xeno2/model/bl/bl000301, "tights_TS", shd0021.frag
        let glsl = include_str!("data/xc2/bl000301.21.frag");

        // Test detecting the "PNEUMA" color layer.
        let fragment = TranslationUnit::parse(glsl).unwrap();
        let shader = shader_from_glsl(None, &fragment);
        assert_eq!(
            vec![
                TextureLayer {
                    value: Dependency::Texture(TextureDependency {
                        name: "s0".into(),
                        channel: Some('x'),
                        texcoords: vec![
                            TexCoord {
                                name: "in_attr4".into(),
                                channel: Some('x'),
                                params: None
                            },
                            TexCoord {
                                name: "in_attr4".into(),
                                channel: Some('y'),
                                params: None
                            }
                        ]
                    }),
                    ratio: None,
                    blend_mode: LayerBlendMode::Mix,
                    is_fresnel: false
                },
                TextureLayer {
                    value: Dependency::Texture(TextureDependency {
                        name: "s1".into(),
                        channel: Some('x'),
                        texcoords: vec![
                            TexCoord {
                                name: "in_attr4".into(),
                                channel: Some('x'),
                                params: None
                            },
                            TexCoord {
                                name: "in_attr4".into(),
                                channel: Some('y'),
                                params: None
                            }
                        ]
                    }),
                    ratio: None,
                    blend_mode: LayerBlendMode::Mix,
                    is_fresnel: false
                },
                TextureLayer {
                    value: Dependency::Texture(TextureDependency {
                        name: "s3".into(),
                        channel: Some('x'),
                        texcoords: vec![
                            TexCoord {
                                name: "in_attr4".into(),
                                channel: Some('z'),
                                params: None,
                            },
                            TexCoord {
                                name: "in_attr4".into(),
                                channel: Some('w'),
                                params: None,
                            },
                        ],
                    }),
                    ratio: Some(Dependency::Texture(TextureDependency {
                        name: "s4".into(),
                        channel: Some('x'),
                        texcoords: vec![
                            TexCoord {
                                name: "in_attr4".into(),
                                channel: Some('z'),
                                params: None,
                            },
                            TexCoord {
                                name: "in_attr4".into(),
                                channel: Some('w'),
                                params: None,
                            },
                        ],
                    })),
                    blend_mode: LayerBlendMode::Mix,
                    is_fresnel: false,
                },
                TextureLayer {
                    value: Dependency::Buffer(BufferDependency {
                        name: "U_Mate".into(),
                        field: "gWrkCol".into(),
                        index: None,
                        channel: Some('x'),
                    }),
                    ratio: Some(Dependency::Buffer(BufferDependency {
                        name: "U_Mate".into(),
                        field: "gWrkFl4".into(),
                        index: Some(0),
                        channel: Some('y'),
                    })),
                    blend_mode: LayerBlendMode::Mix,
                    is_fresnel: true,
                },
            ],
            shader.output_dependencies[&SmolStr::from("o0.x")].layers
        );
    }

    #[test]
    fn shader_from_fragment_tirkin_weapon() {
        // xeno2/model/we/we010402, "body_MT", shd0000.frag
        let glsl = include_str!("data/xc2/we010402.0.frag");

        // Test detecting layers for metalness.
        let fragment = TranslationUnit::parse(glsl).unwrap();
        let shader = shader_from_glsl(None, &fragment);
        assert_eq!(
            vec![
                TextureLayer {
                    value: Dependency::Texture(TextureDependency {
                        name: "s2".into(),
                        channel: Some('y'),
                        texcoords: vec![
                            TexCoord {
                                name: "in_attr4".into(),
                                channel: Some('x'),
                                params: None,
                            },
                            TexCoord {
                                name: "in_attr4".into(),
                                channel: Some('y'),
                                params: None,
                            },
                        ],
                    }),
                    ratio: None,
                    blend_mode: LayerBlendMode::Mix,
                    is_fresnel: false,
                },
                TextureLayer {
                    value: Dependency::Texture(TextureDependency {
                        name: "s4".into(),
                        channel: Some('y'),
                        texcoords: vec![
                            TexCoord {
                                name: "in_attr4".into(),
                                channel: Some('z'),
                                params: None,
                            },
                            TexCoord {
                                name: "in_attr4".into(),
                                channel: Some('w'),
                                params: None,
                            },
                        ],
                    }),
                    ratio: Some(Dependency::Texture(TextureDependency {
                        name: "s5".into(),
                        channel: Some('x'),
                        texcoords: vec![
                            TexCoord {
                                name: "in_attr4".into(),
                                channel: Some('x'),
                                params: None,
                            },
                            TexCoord {
                                name: "in_attr4".into(),
                                channel: Some('y'),
                                params: None,
                            },
                        ],
                    })),
                    blend_mode: LayerBlendMode::Overlay,
                    is_fresnel: false,
                },
            ],
            shader.output_dependencies[&SmolStr::from("o1.x")].layers
        );
    }

    #[test]
    fn shader_from_fragment_behemoth_fins() {
        // xeno2/model/en/en020601, "hire_a", shd0000.frag
        let glsl = include_str!("data/xc2/en020601.0.frag");

        // Test detecting layers for ambient occlusion.
        let fragment = TranslationUnit::parse(glsl).unwrap();
        let shader = shader_from_glsl(None, &fragment);
        assert_eq!(
            vec![
                TextureLayer {
                    value: Dependency::Texture(TextureDependency {
                        name: "s2".into(),
                        channel: Some('z'),
                        texcoords: vec![
                            TexCoord {
                                name: "in_attr4".into(),
                                channel: Some('x'),
                                params: None,
                            },
                            TexCoord {
                                name: "in_attr4".into(),
                                channel: Some('y'),
                                params: None,
                            },
                        ],
                    }),
                    ratio: None,
                    blend_mode: LayerBlendMode::Mix,
                    is_fresnel: false,
                },
                TextureLayer {
                    value: Dependency::Buffer(BufferDependency {
                        name: "U_Mate".into(),
                        field: "gWrkFl4".into(),
                        index: Some(0),
                        channel: Some('z'),
                    }),
                    ratio: Some(Dependency::Buffer(BufferDependency {
                        name: "U_Mate".into(),
                        field: "gWrkFl4".into(),
                        index: Some(1),
                        channel: Some('z'),
                    })),
                    blend_mode: LayerBlendMode::Mix,
                    is_fresnel: false,
                },
                TextureLayer {
                    value: Dependency::Attribute(AttributeDependency {
                        name: "in_attr5".into(),
                        channel: Some('y'),
                    }),
                    ratio: Some(Dependency::Constant(1.0.into())),
                    blend_mode: LayerBlendMode::MixRatio,
                    is_fresnel: false,
                },
            ],
            shader.output_dependencies[&SmolStr::from("o2.z")].layers
        );
    }

    #[test]
    fn shader_from_fragment_gramps_fur() {
        // xeno2/model/np/np000101, "_body_far_Fur", shd0009.frag
        let glsl = include_str!("data/xc2/np000101.9.frag");

        // TODO: Is it worth detecting fur layering code?
        let fragment = TranslationUnit::parse(glsl).unwrap();
        let shader = shader_from_glsl(None, &fragment);
        assert!(shader.output_dependencies[&SmolStr::from("o0.x")]
            .layers
            .is_empty());
    }

    #[test]
    fn shader_from_fragment_lysaat_eyes() {
        // xeno2/model/en/en030601, "phong3", shd0009.frag
        let glsl = include_str!("data/xc2/en030601.2.frag");

        // Detect parallax mapping for texture coordinates.
        let fragment = TranslationUnit::parse(glsl).unwrap();
        let shader = shader_from_glsl(None, &fragment);
        assert_eq!(
            vec![
                Dependency::Texture(TextureDependency {
                    name: "s0".into(),
                    channel: Some('x'),
                    texcoords: vec![
                        TexCoord {
                            name: "in_attr4".into(),
                            channel: Some('x'),
                            params: Some(TexCoordParams::Parallax {
                                mask_a: Dependency::Texture(TextureDependency {
                                    name: "s1".into(),
                                    channel: Some('x'),
                                    texcoords: vec![
                                        TexCoord {
                                            name: "in_attr4".into(),
                                            channel: Some('x'),
                                            params: None,
                                        },
                                        TexCoord {
                                            name: "in_attr4".into(),
                                            channel: Some('y'),
                                            params: None,
                                        },
                                    ],
                                }),
                                mask_b: Dependency::Buffer(BufferDependency {
                                    name: "U_Mate".into(),
                                    field: "gWrkFl4".into(),
                                    index: Some(0),
                                    channel: Some('y'),
                                }),
                                ratio: BufferDependency {
                                    name: "U_Mate".into(),
                                    field: "gWrkFl4".into(),
                                    index: Some(0),
                                    channel: Some('w'),
                                },
                            }),
                        },
                        TexCoord {
                            name: "in_attr4".into(),
                            channel: Some('y'),
                            params: Some(TexCoordParams::Parallax {
                                mask_a: Dependency::Texture(TextureDependency {
                                    name: "s1".into(),
                                    channel: Some('x'),
                                    texcoords: vec![
                                        TexCoord {
                                            name: "in_attr4".into(),
                                            channel: Some('x'),
                                            params: None,
                                        },
                                        TexCoord {
                                            name: "in_attr4".into(),
                                            channel: Some('y'),
                                            params: None,
                                        },
                                    ],
                                }),
                                mask_b: Dependency::Buffer(BufferDependency {
                                    name: "U_Mate".into(),
                                    field: "gWrkFl4".into(),
                                    index: Some(0),
                                    channel: Some('y'),
                                }),
                                ratio: BufferDependency {
                                    name: "U_Mate".into(),
                                    field: "gWrkFl4".into(),
                                    index: Some(0),
                                    channel: Some('w'),
                                },
                            }),
                        },
                    ],
                }),
                Dependency::Texture(TextureDependency {
                    name: "s1".into(),
                    channel: Some('x'),
                    texcoords: vec![
                        TexCoord {
                            name: "in_attr4".into(),
                            channel: Some('x'),
                            params: None,
                        },
                        TexCoord {
                            name: "in_attr4".into(),
                            channel: Some('y'),
                            params: None,
                        },
                    ],
                }),
            ],
            shader.output_dependencies[&SmolStr::from("o0.x")].dependencies
        );
    }

    #[test]
    fn shader_from_vertex_fragment_noah_body_outline() {
        // xeno3/chr/ch/ch01011013, "body_outline", shd0000.frag
        let vert_glsl = include_str!("data/xc3/ch01011013.0.vert");
        let frag_glsl = include_str!("data/xc3/ch01011013.0.frag");

        // Check for outline data.
        let vertex = TranslationUnit::parse(vert_glsl).unwrap();
        let fragment = TranslationUnit::parse(frag_glsl).unwrap();
        let shader = shader_from_glsl(Some(&vertex), &fragment);
        assert_eq!(
            Some(Dependency::Buffer(BufferDependency {
                name: "U_Mate".into(),
                field: "gWrkFl4".into(),
                index: Some(0),
                channel: Some('z'),
            })),
            shader.outline_width
        );
    }

    #[test]
    fn shader_from_fragment_panacea_body() {
        // xeno3/chr/ch/ch44000210, "ch45133501_body", shd0029.frag
        let glsl = include_str!("data/xc3/ch44000210.29.frag");

        // Check for correct color layers
        let fragment = TranslationUnit::parse(glsl).unwrap();
        let shader = shader_from_glsl(None, &fragment);
        assert_eq!(
            vec![
                TextureLayer {
                    value: Dependency::Texture(TextureDependency {
                        name: "s0".into(),
                        channel: Some('x'),
                        texcoords: vec![
                            TexCoord {
                                name: "in_attr4".into(),
                                channel: Some('x'),
                                params: None,
                            },
                            TexCoord {
                                name: "in_attr4".into(),
                                channel: Some('y'),
                                params: None,
                            },
                        ],
                    }),
                    ratio: None,
                    blend_mode: LayerBlendMode::Mix,
                    is_fresnel: false,
                },
                TextureLayer {
                    value: Dependency::Buffer(BufferDependency {
                        name: "U_Mate".into(),
                        field: "gWrkCol".into(),
                        index: Some(1),
                        channel: Some('x'),
                    }),
                    ratio: Some(Dependency::Constant((-1.0).into())),
                    blend_mode: LayerBlendMode::Add,
                    is_fresnel: false,
                },
                TextureLayer {
                    value: Dependency::Texture(TextureDependency {
                        name: "gTResidentTex11".into(),
                        channel: Some('x'),
                        texcoords: vec![
                            TexCoord {
                                name: "in_attr4".into(),
                                channel: Some('x'),
                                params: None,
                            },
                            TexCoord {
                                name: "in_attr4".into(),
                                channel: Some('x'),
                                params: None,
                            },
                        ],
                    }),
                    ratio: Some(Dependency::Texture(TextureDependency {
                        name: "s1".into(),
                        channel: Some('x'),
                        texcoords: vec![
                            TexCoord {
                                name: "in_attr4".into(),
                                channel: Some('x'),
                                params: None,
                            },
                            TexCoord {
                                name: "in_attr4".into(),
                                channel: Some('y'),
                                params: None,
                            },
                        ],
                    })),
                    blend_mode: LayerBlendMode::Add,
                    is_fresnel: false,
                },
                TextureLayer {
                    value: Dependency::Texture(TextureDependency {
                        name: "gTResidentTex04".into(),
                        channel: Some('x'),
                        texcoords: vec![
                            TexCoord {
                                name: "in_attr4".into(),
                                channel: Some('z'),
                                params: None,
                            },
                            TexCoord {
                                name: "in_attr4".into(),
                                channel: Some('w'),
                                params: None,
                            },
                        ],
                    }),
                    ratio: None,
                    blend_mode: LayerBlendMode::Mix,
                    is_fresnel: false,
                }
            ],
            shader.output_dependencies[&SmolStr::from("o0.x")].layers
        );
    }

    #[test]
    fn shader_from_latte_asm_elma_leg() {
        // xenox/chr_pc/pc221115.camdo, "leg_mat", shd0000.frag
        let asm = include_str!("data/xcx/pc221115.0.frag.txt");

        // TODO: Make this easier to test by taking metadata directly?
        let fragment_shader = xc3_lib::mths::FragmentShader {
            unk1: 0,
            unk2: 0,
            program_binary: Vec::new(),
            shader_mode: xc3_lib::mths::ShaderMode::UniformBlock,
            uniform_buffers: vec![xc3_lib::mths::UniformBuffer {
                name: "U_Mate".to_string(),
                offset: 1,
                size: 48,
            }],
            uniforms: vec![
                xc3_lib::mths::Uniform {
                    name: "Q".to_string(),
                    data_type: xc3_lib::mths::VarType::Vec4,
                    count: 1,
                    offset: 0,
                    uniform_buffer_index: 0,
                },
                xc3_lib::mths::Uniform {
                    name: "Q".to_string(),
                    data_type: xc3_lib::mths::VarType::Vec4,
                    count: 1,
                    offset: 8,
                    uniform_buffer_index: 0,
                },
                xc3_lib::mths::Uniform {
                    name: "Q".to_string(),
                    data_type: xc3_lib::mths::VarType::Vec4,
                    count: 1,
                    offset: 4,
                    uniform_buffer_index: 0,
                },
            ],
            unk9: [0, 0, 0, 0],
            samplers: vec![
                xc3_lib::mths::Sampler {
                    name: "gIBL".to_string(),
                    sampler_type: xc3_lib::mths::SamplerType::D2,
                    location: 0,
                },
                xc3_lib::mths::Sampler {
                    name: "s0".to_string(),
                    sampler_type: xc3_lib::mths::SamplerType::D2,
                    location: 1,
                },
                xc3_lib::mths::Sampler {
                    name: "s1".to_string(),
                    sampler_type: xc3_lib::mths::SamplerType::D2,
                    location: 2,
                },
                xc3_lib::mths::Sampler {
                    name: "s2".to_string(),
                    sampler_type: xc3_lib::mths::SamplerType::D2,
                    location: 3,
                },
                xc3_lib::mths::Sampler {
                    name: "s3".to_string(),
                    sampler_type: xc3_lib::mths::SamplerType::D2,
                    location: 4,
                },
                xc3_lib::mths::Sampler {
                    name: "texRef".to_string(),
                    sampler_type: xc3_lib::mths::SamplerType::D2,
                    location: 5,
                },
            ],
        };
        let shader = shader_from_latte_asm("", asm, &fragment_shader);
        assert_eq!(
            ShaderProgram {
                output_dependencies: [
                    (
                        "o0.x".into(),
                        OutputDependencies {
                            dependencies: vec![
                                Dependency::Texture(TextureDependency {
                                    name: "s2".into(),
                                    channel: Some('x'),
                                    texcoords: Vec::new(),
                                }),
                                Dependency::Texture(TextureDependency {
                                    name: "s2".into(),
                                    channel: Some('y'),
                                    texcoords: Vec::new(),
                                }),
                                Dependency::Texture(TextureDependency {
                                    name: "s1".into(),
                                    channel: Some('y'),
                                    texcoords: Vec::new(),
                                }),
                                Dependency::Texture(TextureDependency {
                                    name: "gIBL".into(),
                                    channel: Some('x'),
                                    texcoords: Vec::new(),
                                }),
                            ],
                            layers: Vec::new()
                        },
                    ),
                    (
                        "o0.y".into(),
                        OutputDependencies {
                            dependencies: vec![
                                Dependency::Texture(TextureDependency {
                                    name: "s2".into(),
                                    channel: Some('x'),
                                    texcoords: Vec::new(),
                                }),
                                Dependency::Texture(TextureDependency {
                                    name: "s2".into(),
                                    channel: Some('y'),
                                    texcoords: Vec::new(),
                                }),
                                Dependency::Texture(TextureDependency {
                                    name: "s1".into(),
                                    channel: Some('y'),
                                    texcoords: Vec::new(),
                                }),
                                Dependency::Texture(TextureDependency {
                                    name: "gIBL".into(),
                                    channel: Some('y'),
                                    texcoords: Vec::new(),
                                }),
                            ],
                            layers: Vec::new()
                        },
                    ),
                    (
                        "o0.z".into(),
                        OutputDependencies {
                            dependencies: vec![
                                Dependency::Texture(TextureDependency {
                                    name: "s2".into(),
                                    channel: Some('x'),
                                    texcoords: Vec::new(),
                                }),
                                Dependency::Texture(TextureDependency {
                                    name: "s2".into(),
                                    channel: Some('y'),
                                    texcoords: Vec::new(),
                                }),
                                Dependency::Texture(TextureDependency {
                                    name: "s1".into(),
                                    channel: Some('y'),
                                    texcoords: Vec::new(),
                                }),
                                Dependency::Texture(TextureDependency {
                                    name: "gIBL".into(),
                                    channel: Some('z'),
                                    texcoords: Vec::new(),
                                }),
                            ],
                            layers: Vec::new()
                        },
                    ),
                    (
                        "o0.w".into(),
                        OutputDependencies {
                            dependencies: vec![
                                Dependency::Texture(TextureDependency {
                                    name: "s2".into(),
                                    channel: Some('x'),
                                    texcoords: Vec::new(),
                                }),
                                Dependency::Texture(TextureDependency {
                                    name: "s2".into(),
                                    channel: Some('y'),
                                    texcoords: Vec::new(),
                                }),
                                Dependency::Texture(TextureDependency {
                                    name: "gIBL".into(),
                                    channel: Some('w'),
                                    texcoords: Vec::new(),
                                }),
                            ],
                            layers: Vec::new()
                        },
                    ),
                    (
                        "o1.x".into(),
                        OutputDependencies {
                            dependencies: vec![
                                Dependency::Texture(TextureDependency {
                                    name: "s2".into(),
                                    channel: Some('x'),
                                    texcoords: Vec::new(),
                                }),
                                Dependency::Texture(TextureDependency {
                                    name: "s2".into(),
                                    channel: Some('y'),
                                    texcoords: Vec::new(),
                                }),
                                Dependency::Texture(TextureDependency {
                                    name: "s1".into(),
                                    channel: Some('x'),
                                    texcoords: Vec::new(),
                                }),
                                Dependency::Texture(TextureDependency {
                                    name: "s0".into(),
                                    channel: Some('x'),
                                    texcoords: Vec::new(),
                                }),
                                Dependency::Texture(TextureDependency {
                                    name: "texRef".into(),
                                    channel: Some('x'),
                                    texcoords: Vec::new(),
                                }),
                            ],
                            layers: Vec::new()
                        },
                    ),
                    (
                        "o1.y".into(),
                        OutputDependencies {
                            dependencies: vec![
                                Dependency::Texture(TextureDependency {
                                    name: "s2".into(),
                                    channel: Some('x'),
                                    texcoords: Vec::new(),
                                }),
                                Dependency::Texture(TextureDependency {
                                    name: "s2".into(),
                                    channel: Some('y'),
                                    texcoords: Vec::new(),
                                }),
                                Dependency::Texture(TextureDependency {
                                    name: "s1".into(),
                                    channel: Some('x'),
                                    texcoords: Vec::new(),
                                }),
                                Dependency::Texture(TextureDependency {
                                    name: "s0".into(),
                                    channel: Some('y'),
                                    texcoords: Vec::new(),
                                }),
                                Dependency::Texture(TextureDependency {
                                    name: "texRef".into(),
                                    channel: Some('y'),
                                    texcoords: Vec::new(),
                                }),
                            ],
                            layers: Vec::new()
                        },
                    ),
                    (
                        "o1.z".into(),
                        OutputDependencies {
                            dependencies: vec![
                                Dependency::Texture(TextureDependency {
                                    name: "s2".into(),
                                    channel: Some('x'),
                                    texcoords: Vec::new(),
                                }),
                                Dependency::Texture(TextureDependency {
                                    name: "s2".into(),
                                    channel: Some('y'),
                                    texcoords: Vec::new(),
                                }),
                                Dependency::Texture(TextureDependency {
                                    name: "s1".into(),
                                    channel: Some('x'),
                                    texcoords: Vec::new(),
                                }),
                                Dependency::Texture(TextureDependency {
                                    name: "s0".into(),
                                    channel: Some('z'),
                                    texcoords: Vec::new(),
                                }),
                                Dependency::Texture(TextureDependency {
                                    name: "texRef".into(),
                                    channel: Some('z'),
                                    texcoords: Vec::new(),
                                }),
                            ],
                            layers: Vec::new()
                        },
                    ),
                    (
                        "o1.w".into(),
                        OutputDependencies {
                            dependencies: vec![Dependency::Constant(0.0.into())],
                            layers: Vec::new()
                        },
                    ),
                    (
                        "o2.x".into(),
                        OutputDependencies {
                            dependencies: vec![
                                Dependency::Texture(TextureDependency {
                                    name: "s2".into(),
                                    channel: Some('x'),
                                    texcoords: Vec::new(),
                                }),
                                Dependency::Texture(TextureDependency {
                                    name: "s2".into(),
                                    channel: Some('y'),
                                    texcoords: Vec::new(),
                                }),
                            ],
                            layers: Vec::new()
                        },
                    ),
                    (
                        "o2.y".into(),
                        OutputDependencies {
                            dependencies: vec![
                                Dependency::Texture(TextureDependency {
                                    name: "s2".into(),
                                    channel: Some('x'),
                                    texcoords: Vec::new(),
                                }),
                                Dependency::Texture(TextureDependency {
                                    name: "s2".into(),
                                    channel: Some('y'),
                                    texcoords: Vec::new(),
                                }),
                            ],
                            layers: Vec::new()
                        },
                    ),
                    (
                        "o3.x".into(),
                        OutputDependencies {
                            dependencies: vec![Dependency::Texture(TextureDependency {
                                name: "s3".into(),
                                channel: Some('x'),
                                texcoords: Vec::new(),
                            })],
                            layers: Vec::new()
                        },
                    ),
                    (
                        "o3.y".into(),
                        OutputDependencies {
                            dependencies: vec![Dependency::Texture(TextureDependency {
                                name: "s3".into(),
                                channel: Some('y'),
                                texcoords: Vec::new(),
                            })],
                            layers: Vec::new()
                        },
                    ),
                    (
                        "o3.z".into(),
                        OutputDependencies {
                            dependencies: vec![Dependency::Texture(TextureDependency {
                                name: "s3".into(),
                                channel: Some('z'),
                                texcoords: Vec::new(),
                            })],
                            layers: Vec::new()
                        },
                    ),
                    (
                        "o3.w".into(),
                        OutputDependencies {
                            dependencies: vec![Dependency::Buffer(BufferDependency {
                                name: "KC0".into(),
                                field: "".into(),
                                index: Some(1),
                                channel: Some('x'),
                            })],
                            layers: Vec::new()
                        },
                    ),
                    (
                        "o4.w".into(),
                        OutputDependencies {
                            dependencies: vec![Dependency::Texture(TextureDependency {
                                name: "s1".into(),
                                channel: Some('z'),
                                texcoords: Vec::new(),
                            })],
                            layers: Vec::new()
                        },
                    )
                ]
                .into(),
                outline_width: None
            },
            shader
        );
    }

    #[test]
    fn shader_from_fragment_l_face() {
        // xenoxde/chr/fc/fc181020, "facemat", shd0008.frag
        let vert_glsl = include_str!("data/xcxde/fc181020.8.vert");
        let frag_glsl = include_str!("data/xcxde/fc181020.8.frag");

        // Check for overlay blending to make the face blue.
        let vertex = TranslationUnit::parse(vert_glsl).unwrap();
        let fragment = TranslationUnit::parse(frag_glsl).unwrap();
        let shader = shader_from_glsl(Some(&vertex), &fragment);
        assert_eq!(
            OutputDependencies {
                dependencies: vec![Dependency::Texture(TextureDependency {
                    name: "s0".into(),
                    channel: Some('x'),
                    texcoords: vec![
                        TexCoord {
                            name: "vTex0".into(),
                            channel: Some('x'),
                            params: None,
                        },
                        TexCoord {
                            name: "vTex0".into(),
                            channel: Some('y'),
                            params: None,
                        },
                    ],
                })],
                layers: vec![
                    TextureLayer {
                        value: Dependency::Texture(TextureDependency {
                            name: "s0".into(),
                            channel: Some('x'),
                            texcoords: vec![
                                TexCoord {
                                    name: "vTex0".into(),
                                    channel: Some('x'),
                                    params: None,
                                },
                                TexCoord {
                                    name: "vTex0".into(),
                                    channel: Some('y'),
                                    params: None,
                                },
                            ],
                        }),
                        ratio: None,
                        blend_mode: LayerBlendMode::Mix,
                        is_fresnel: false,
                    },
                    TextureLayer {
                        value: Dependency::Buffer(BufferDependency {
                            name: "U_CHR".into(),
                            field: "gAvaSkin".into(),
                            index: None,
                            channel: Some('x'),
                        }),
                        ratio: Some(Dependency::Constant(1.0.into())),
                        blend_mode: LayerBlendMode::Overlay,
                        is_fresnel: false,
                    },
                    TextureLayer {
                        value: Dependency::Attribute(AttributeDependency {
                            name: "vColor".into(),
                            channel: Some('x'),
                        }),
                        ratio: Some(Dependency::Constant(1.0.into())),
                        blend_mode: LayerBlendMode::MixRatio,
                        is_fresnel: false,
                    },
                ],
            },
            shader.output_dependencies[&SmolStr::from("o1.x")]
        );
        assert_eq!(
            OutputDependencies {
                dependencies: vec![Dependency::Texture(TextureDependency {
                    name: "s0".into(),
                    channel: Some('y'),
                    texcoords: vec![
                        TexCoord {
                            name: "vTex0".into(),
                            channel: Some('x'),
                            params: None,
                        },
                        TexCoord {
                            name: "vTex0".into(),
                            channel: Some('y'),
                            params: None,
                        },
                    ],
                })],
                layers: vec![
                    TextureLayer {
                        value: Dependency::Texture(TextureDependency {
                            name: "s0".into(),
                            channel: Some('y'),
                            texcoords: vec![
                                TexCoord {
                                    name: "vTex0".into(),
                                    channel: Some('x'),
                                    params: None,
                                },
                                TexCoord {
                                    name: "vTex0".into(),
                                    channel: Some('y'),
                                    params: None,
                                },
                            ],
                        }),
                        ratio: None,
                        blend_mode: LayerBlendMode::Mix,
                        is_fresnel: false,
                    },
                    TextureLayer {
                        value: Dependency::Buffer(BufferDependency {
                            name: "U_CHR".into(),
                            field: "gAvaSkin".into(),
                            index: None,
                            channel: Some('y'),
                        }),
                        ratio: Some(Dependency::Constant(1.0.into())),
                        blend_mode: LayerBlendMode::Overlay,
                        is_fresnel: false,
                    },
                    TextureLayer {
                        value: Dependency::Attribute(AttributeDependency {
                            name: "vColor".into(),
                            channel: Some('y'),
                        }),
                        ratio: Some(Dependency::Constant(1.0.into())),
                        blend_mode: LayerBlendMode::MixRatio,
                        is_fresnel: false,
                    },
                ],
            },
            shader.output_dependencies[&SmolStr::from("o1.y")]
        );
        assert_eq!(
            OutputDependencies {
                dependencies: vec![Dependency::Texture(TextureDependency {
                    name: "s0".into(),
                    channel: Some('z'),
                    texcoords: vec![
                        TexCoord {
                            name: "vTex0".into(),
                            channel: Some('x'),
                            params: None,
                        },
                        TexCoord {
                            name: "vTex0".into(),
                            channel: Some('y'),
                            params: None,
                        },
                    ],
                })],
                layers: vec![
                    TextureLayer {
                        value: Dependency::Texture(TextureDependency {
                            name: "s0".into(),
                            channel: Some('z'),
                            texcoords: vec![
                                TexCoord {
                                    name: "vTex0".into(),
                                    channel: Some('x'),
                                    params: None,
                                },
                                TexCoord {
                                    name: "vTex0".into(),
                                    channel: Some('y'),
                                    params: None,
                                },
                            ],
                        }),
                        ratio: None,
                        blend_mode: LayerBlendMode::Mix,
                        is_fresnel: false,
                    },
                    TextureLayer {
                        value: Dependency::Buffer(BufferDependency {
                            name: "U_CHR".into(),
                            field: "gAvaSkin".into(),
                            index: None,
                            channel: Some('z'),
                        }),
                        ratio: Some(Dependency::Constant(1.0.into())),
                        blend_mode: LayerBlendMode::Overlay,
                        is_fresnel: false,
                    },
                    TextureLayer {
                        value: Dependency::Attribute(AttributeDependency {
                            name: "vColor".into(),
                            channel: Some('z'),
                        }),
                        ratio: Some(Dependency::Constant(1.0.into())),
                        blend_mode: LayerBlendMode::MixRatio,
                        is_fresnel: false,
                    },
                ],
            },
            shader.output_dependencies[&SmolStr::from("o1.z")]
        );
    }
}
