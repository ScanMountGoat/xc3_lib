use std::{collections::BTreeMap, path::Path, sync::LazyLock};

use approx::AbsDiffEq;
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
    AttributeDependency, Dependency, Operation, OutputExpr, ProgramHash, ShaderDatabase,
    ShaderProgram,
};

use crate::{
    dependencies::{
        attribute_dependencies, buffer_dependency, input_dependencies, texcoord_params,
        texture_dependency,
    },
    extract::nvsd_glsl_name,
    graph::{
        glsl::shader_source_no_extensions,
        query::{
            assign_x, assign_x_recursive, dot3_a_b, fma_a_b_c, fma_half_half, mix_a_b_ratio,
            node_expr, normalize, query_nodes,
        },
        BinaryOp, Expr, Graph, Node, UnaryOp,
    },
};

pub fn shader_from_glsl(
    vertex: Option<&TranslationUnit>,
    fragment: &TranslationUnit,
) -> ShaderProgram {
    let frag = Graph::from_glsl(fragment);
    let frag_attributes = find_attribute_locations(fragment);

    let vertex = vertex.map(|v| (Graph::from_glsl(v), find_attribute_locations(v)));
    let (vert, vert_attributes) = vertex.unzip();

    let outline_width = vert
        .as_ref()
        .map(outline_width_parameter)
        .unwrap_or_default();

    let mut output_dependencies = IndexMap::new();
    let mut normal_intensity = None;

    // TODO: Some shaders have more than 6 outputs?
    for i in 0..=5 {
        for c in "xyzw".chars() {
            let name = format!("out_attr{i}");
            let dependent_lines = frag.dependencies_recursive(&name, Some(c), None);

            let mut value;
            if i == 2 && (c == 'x' || c == 'y') {
                // The normals use XY for output index 2 for all games.
                let (new_value, intensity) =
                    normal_output_expr(&frag, &frag_attributes, &dependent_lines)
                        .unwrap_or_default();
                value = new_value;
                normal_intensity = intensity;
            } else {
                // Xenoblade X DE uses different outputs than other games.
                // Detect color or params to handle different outputs and channels.
                value = color_or_param_output_expr(&frag, &frag_attributes, &dependent_lines)
                    .unwrap_or_default()
            };

            if let (Some(vert), Some(vert_attributes)) = (&vert, &vert_attributes) {
                apply_expr_attribute_names(&mut value, vert, vert_attributes, &frag_attributes);
                apply_expr_vertex_uv_params(&mut value, vert, vert_attributes, &frag_attributes);

                if let Some(i) = &mut normal_intensity {
                    apply_expr_attribute_names(i, vert, vert_attributes, &frag_attributes);
                    apply_expr_vertex_uv_params(i, vert, vert_attributes, &frag_attributes);
                }
            }

            // TODO: skip entirely if the value is none or use a special value?
            if !dependent_lines.is_empty() {
                // Simplify the output name to save space.
                let output_name = format!("o{i}.{c}");
                output_dependencies.insert(output_name.into(), value);
            }
        }
    }

    ShaderProgram {
        output_dependencies,
        outline_width,
        normal_intensity,
    }
}

fn apply_expr_attribute_names(
    value: &mut OutputExpr,
    vert: &Graph,
    vert_attributes: &Attributes,
    frag_attributes: &Attributes,
) {
    // Names are only present for vertex input attributes.
    match value {
        OutputExpr::Value(d) => {
            apply_attribute_names(vert, vert_attributes, frag_attributes, d);
        }
        OutputExpr::Func { args, .. } => {
            for arg in args {
                apply_expr_attribute_names(arg, vert, vert_attributes, frag_attributes);
            }
        }
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
    vertex: &str,
    fragment: &str,
    fragment_shader: &FragmentShader,
) -> ShaderProgram {
    let _vert = Graph::from_latte_asm(vertex);
    let _vert_attributes = Attributes::default();

    let frag = Graph::from_latte_asm(fragment);
    let frag_attributes = Attributes::default();

    // TODO: What is the largest number of outputs?
    let mut output_dependencies = IndexMap::new();
    for i in 0..=5 {
        for c in "xyzw".chars() {
            let name = format!("PIX{i}");

            let assignments = frag.assignments_recursive(&name, Some(c), None);
            let dependent_lines = frag.dependencies_recursive(&name, Some(c), None);

            let mut dependencies =
                input_dependencies(&frag, &frag_attributes, &assignments, &dependent_lines);

            // TODO: Add texture parameters used for the corresponding vertex output.

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
                    Dependency::Attribute(_) => (),
                }
            }

            // TODO: How much of the query code can be reused for Wii U?
            if let Some(d) = dependencies.first() {
                // Simplify the output name to save space.
                let output_name = format!("o{i}.{c}");
                output_dependencies.insert(output_name.into(), OutputExpr::Value(d.clone()));
            }
        }
    }

    ShaderProgram {
        output_dependencies,
        outline_width: None,
        normal_intensity: None,
    }
}

fn color_or_param_output_expr(
    frag: &Graph,
    frag_attributes: &Attributes,
    dependent_lines: &[usize],
) -> Option<OutputExpr> {
    let last_node_index = *dependent_lines.last()?;
    let last_node = frag.nodes.get(last_node_index)?;

    // matCol.xyz in pcmdo shaders.
    let mut current = &last_node.input;

    // Remove some redundant conversions found in some shaders.
    if let Expr::Func { name, args, .. } = current {
        if name == "intBitsToFloat" {
            current = assign_x_recursive(&frag.nodes, &args[0]);

            if let Expr::Func { name, args, .. } = current {
                if name == "floatBitsToInt" {
                    current = &args[0];
                }
            }
        }
    }

    current = assign_x_recursive(&frag.nodes, current);

    // This isn't always present for all materials in all games.
    // Xenoblade 1 DE and Xenoblade 3 both seem to do this for non map materials.
    // TODO: Include calc_monochrome as func?
    if let Some((mat_cols, _monochrome_ratio)) = calc_monochrome(&frag.nodes, current) {
        let mat_col = match last_node.output.channel {
            Some('x') => &mat_cols[0],
            Some('y') => &mat_cols[1],
            Some('z') => &mat_cols[2],
            _ => &mat_cols[0],
        };
        current = assign_x_recursive(&frag.nodes, mat_col);
    }

    if let Some(new_current) = geometric_specular_aa(&frag.nodes, current) {
        current = new_current;
    }

    Some(output_expr(current, frag, frag_attributes))
}

fn calc_monochrome<'a>(nodes: &'a [Node], expr: &'a Expr) -> Option<([&'a Expr; 3], &'a Expr)> {
    // calcMonochrome in pcmdo fragment shaders for XC1 and XC3.
    let (_mat_col, monochrome, monochrome_ratio) = mix_a_b_ratio(nodes, expr)?;
    let monochrome = node_expr(nodes, monochrome)?;
    let (a, b) = dot3_a_b(nodes, monochrome)?;

    // TODO: Check weight values for XC1 (0.3, 0.59, 0.11) or XC3 (0.01, 0.01, 0.01)?
    let mat_col = match (a, b) {
        ([Expr::Float(_), Expr::Float(_), Expr::Float(_)], mat_col) => Some(mat_col),
        (mat_col, [Expr::Float(_), Expr::Float(_), Expr::Float(_)]) => Some(mat_col),
        _ => None,
    }?;
    Some((mat_col, monochrome_ratio))
}

fn normal_output_expr(
    frag: &Graph,
    frag_attributes: &Attributes,
    dependent_lines: &[usize],
) -> Option<(OutputExpr, Option<OutputExpr>)> {
    let last_node_index = *dependent_lines.last()?;
    let last_node = frag.nodes.get(last_node_index)?;

    let mut view_normal = assign_x(&frag.nodes, &last_node.input)?;

    // setMrtNormal in pcmdo shaders.
    // Xenoblade X uses RG16Float and doesn't require remapping the value range.
    if let Some(new_view_normal) = fma_half_half(&frag.nodes, view_normal) {
        view_normal = new_view_normal;
    }
    view_normal = assign_x_recursive(&frag.nodes, view_normal);
    view_normal = normalize(&frag.nodes, view_normal)?;

    // TODO: front facing in calcNormalZAbs in pcmdo?

    // nomWork input for getCalcNormalMap in pcmdo shaders.
    let (nom_work, intensity) = calc_normal_map(&frag.nodes, view_normal)
        .map(|n| (n, None))
        .or_else(|| calc_normal_map_xcx(&frag.nodes, view_normal).map(|n| (n, None)))
        .or_else(|| {
            calc_normal_map_w_intensity(&frag.nodes, view_normal).map(|(n, i)| (n, Some(i)))
        })?;

    let nom_work = match last_node.output.channel {
        Some('x') => nom_work[0],
        Some('y') => nom_work[1],
        Some('z') => nom_work[2],
        _ => nom_work[0],
    };
    let nom_work = assign_x_recursive(&frag.nodes, nom_work);

    let value = output_expr(nom_work, frag, frag_attributes);

    let intensity = intensity.map(|i| output_expr(i, frag, frag_attributes));

    Some((value, intensity))
}

fn output_expr(expr: &Expr, graph: &Graph, attributes: &Attributes) -> OutputExpr {
    // Simplify any expressions that would interfere with queries.
    let mut expr = assign_x_recursive(&graph.nodes, expr);
    if let Some(new_expr) = normal_map_fma(&graph.nodes, expr) {
        expr = new_expr;
    }

    if let Some(value) = extract_value(expr, graph, attributes) {
        // The base case is a single value.
        OutputExpr::Value(value)
    } else {
        // Detect operations from most specific to least specific.
        // This results in fewer operations in many cases.
        if let Some((op, args)) = op_add_normal(&graph.nodes, expr)
            .or_else(|| op_fresnel_ratio(&graph.nodes, expr))
            .or_else(|| op_overlay2(&graph.nodes, expr))
            .or_else(|| op_overlay_ratio(&graph.nodes, expr))
            .or_else(|| op_overlay(&graph.nodes, expr))
            .or_else(|| op_mix(&graph.nodes, expr))
            .or_else(|| op_mul_ratio(&graph.nodes, expr))
            .or_else(|| op_add_ratio(expr))
            .or_else(|| op_sub(&graph.nodes, expr))
            .or_else(|| binary_op(expr, BinaryOp::Mul, Operation::Mul))
            .or_else(|| binary_op(expr, BinaryOp::Div, Operation::Div))
            .or_else(|| binary_op(expr, BinaryOp::Add, Operation::Add))
            .or_else(|| op_pow(&graph.nodes, expr))
            .or_else(|| op_clamp(&graph.nodes, expr))
            .or_else(|| op_min(&graph.nodes, expr))
            .or_else(|| op_max(&graph.nodes, expr))
            .or_else(|| unary_op(expr, "abs", Operation::Abs))
        {
            // Recursively detect values or functions.
            // TODO: caching to avoid visiting expr more than once?
            let args: Vec<_> = args
                .into_iter()
                .map(|arg| output_expr(arg, graph, attributes))
                .collect();
            OutputExpr::Func { op, args }
        } else {
            // TODO: sqrt, inversesqrt, floor, ternary, comparisons,
            // TODO: exp2 should always be part of a pow expression
            // TODO: better fallback for unrecognized function or values?
            // TODO: log unsupported expr during database creation?
            println!("{}", graph.expr_to_glsl(expr));
            OutputExpr::Func {
                op: Operation::Unk,
                args: Vec::new(),
            }
        }
    }
}

fn extract_value(expr: &Expr, graph: &Graph, attributes: &Attributes) -> Option<Dependency> {
    let mut expr = assign_x_recursive(&graph.nodes, expr);
    if let Some(new_expr) = normalize(&graph.nodes, expr) {
        expr = new_expr;
    }
    if let Some(new_expr) = normal_map_fma(&graph.nodes, expr) {
        expr = new_expr;
    }

    // TODO: Is it worth storing information about component max?
    if let Some(new_expr) = component_max_xyz(&graph.nodes, expr) {
        expr = new_expr;
    }

    dependency_expr(expr, graph, attributes)
}

static OP_OVER: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
        void main() {
            neg_a = 0.0 - a;
            b_minus_a = neg_a + b;
            result = fma(b_minus_a, ratio, a);
        }
    "};
    Graph::parse_glsl(query).unwrap()
});

static OP_OVER2: LazyLock<Graph> = LazyLock::new(|| {
    // Alternative form used for some shaders.
    let query = indoc! {"
        void main() {
            neg_ratio = 0.0 - ratio;
            a_inv_ratio = fma(a, neg_ratio, a);
            result = fma(b, ratio, a_inv_ratio);
        }
    "};
    Graph::parse_glsl(query).unwrap()
});

fn op_mix<'a>(nodes: &'a [Node], expr: &'a Expr) -> Option<(Operation, Vec<&'a Expr>)> {
    // getPixelCalcOver in pcmdo fragment shaders for XC1 and XC3.
    let result = query_nodes(expr, nodes, &OP_OVER.nodes)
        .or_else(|| query_nodes(expr, nodes, &OP_OVER2.nodes))?;
    let a = result.get("a")?;
    let b = result.get("b")?;
    let ratio = result.get("ratio")?;
    Some((Operation::Mix, vec![a, b, ratio]))
}

static OP_RATIO: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
        void main() {
            neg_a = 0.0 - a;
            ab_minus_a = fma(a, b, neg_a);
            result = fma(ab_minus_a, ratio, a);
        }
    "};
    Graph::parse_glsl(query).unwrap()
});

// TODO: Is it better to just detect this as mix -> mul?
fn op_mul_ratio<'a>(nodes: &'a [Node], expr: &'a Expr) -> Option<(Operation, Vec<&'a Expr>)> {
    // getPixelCalcRatioBlend in pcmdo fragment shaders for XC1 and XC3.
    let result = query_nodes(expr, nodes, &OP_RATIO.nodes)?;
    let a = result.get("a")?;
    let b = result.get("b")?;
    let ratio = result.get("ratio")?;
    Some((Operation::MulRatio, vec![a, b, ratio]))
}

fn op_add_ratio(expr: &Expr) -> Option<(Operation, Vec<&Expr>)> {
    // += getPixelCalcRatio in pcmdo fragment shaders for XC1 and XC3.
    let (a, b, c) = fma_a_b_c(expr)?;
    Some((Operation::Fma, vec![a, b, c]))
}

static OP_OVERLAY_XC2: LazyLock<Graph> = LazyLock::new(|| {
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

// TODO: This can just be detected as mix -> overlay2?
fn op_overlay_ratio<'a>(nodes: &'a [Node], expr: &'a Expr) -> Option<(Operation, Vec<&'a Expr>)> {
    // Overlay combines multiply and screen blend modes.
    // Some XC2 models use overlay blending for metalness.
    let result = query_nodes(expr, nodes, &OP_OVERLAY_XC2.nodes)?;
    let a = result.get("a")?;
    let b = result.get("b")?;
    let ratio = result.get("ratio")?;
    Some((Operation::OverlayRatio, vec![a, b, ratio]))
}

static OP_OVERLAY_XCX_DE: LazyLock<Graph> = LazyLock::new(|| {
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

fn op_overlay<'a>(nodes: &'a [Node], expr: &'a Expr) -> Option<(Operation, Vec<&'a Expr>)> {
    // Overlay combines multiply and screen blend modes.
    // Some XCX DE models use overlay for face coloring.
    let result = query_nodes(expr, nodes, &OP_OVERLAY_XCX_DE.nodes)?;
    let a = result.get("a")?;
    let b = result.get("b")?;
    Some((Operation::Overlay, vec![a, b]))
}

static FRESNEL_RATIO: LazyLock<Graph> = LazyLock::new(|| {
    // getPixelCalcFresnel in pcmdo shaders for XC3.
    // pow(1.0 - n_dot_v, ratio * 5.0)
    let query = indoc! {"
        void main() {
            n_dot_v = abs(n_dot_v);
            neg_n_dot_v = 0.0 - n_dot_v;
            one_minus_n_dot_v = neg_n_dot_v + 1.0;
            result = log2(one_minus_n_dot_v);
            ratio = ratio * 5.0;
            result = ratio * result;
            result = exp2(result);
        }
    "};
    Graph::parse_glsl(query).unwrap()
});

fn op_fresnel_ratio<'a>(nodes: &'a [Node], expr: &'a Expr) -> Option<(Operation, Vec<&'a Expr>)> {
    // TODO: Blend mode for this?
    let result = query_nodes(expr, nodes, &FRESNEL_RATIO.nodes)?;
    let a = result.get("ratio")?;
    Some((Operation::Fresnel, vec![a]))
}

static OP_POW: LazyLock<Graph> = LazyLock::new(|| {
    // Equivalent to pow(a, b)
    let query = indoc! {"
        void main() {
            a = abs(a);
            a = log2(a);
            a = a * b;
            a = exp2(a);
        }
    "};
    Graph::parse_glsl(query).unwrap()
});

static OP_POW2: LazyLock<Graph> = LazyLock::new(|| {
    // Equivalent to pow(a, b)
    let query = indoc! {"
        void main() {
            a = log2(a);
            a = a * b;
            a = exp2(a);
        }
    "};
    Graph::parse_glsl(query).unwrap()
});

fn op_pow<'a>(nodes: &'a [Node], expr: &'a Expr) -> Option<(Operation, Vec<&'a Expr>)> {
    let result = query_nodes(expr, nodes, &OP_POW.nodes)
        .or_else(|| query_nodes(expr, nodes, &OP_POW2.nodes))?;
    let a = result.get("a")?;
    let b = result.get("b")?;
    Some((Operation::Power, vec![a, b]))
}

static OP_MAX: LazyLock<Graph> =
    LazyLock::new(|| Graph::parse_glsl("void main() { result = max(a, b); }").unwrap());

fn op_max<'a>(nodes: &'a [Node], expr: &'a Expr) -> Option<(Operation, Vec<&'a Expr>)> {
    let result = query_nodes(expr, nodes, &OP_MAX.nodes)?;
    let a = result.get("a")?;
    let b = result.get("b")?;
    Some((Operation::Max, vec![a, b]))
}

static OP_MIN: LazyLock<Graph> =
    LazyLock::new(|| Graph::parse_glsl("void main() { result = min(a, b); }").unwrap());

fn op_min<'a>(nodes: &'a [Node], expr: &'a Expr) -> Option<(Operation, Vec<&'a Expr>)> {
    let result = query_nodes(expr, nodes, &OP_MIN.nodes)?;
    let a = result.get("a")?;
    let b = result.get("b")?;
    Some((Operation::Min, vec![a, b]))
}

static OP_CLAMP: LazyLock<Graph> =
    LazyLock::new(|| Graph::parse_glsl("void main() { result = clamp(a, b, c); }").unwrap());

fn op_clamp<'a>(nodes: &'a [Node], expr: &'a Expr) -> Option<(Operation, Vec<&'a Expr>)> {
    // TODO: also detect min -> max and max -> min.
    // TODO: convert to max and min?
    let result = query_nodes(expr, nodes, &OP_CLAMP.nodes)?;
    let a = result.get("a")?;
    let b = result.get("b")?;
    let c = result.get("c")?;
    Some((Operation::Clamp, vec![a, b, c]))
}

fn unary_op<'a>(
    expr: &'a Expr,
    fn_name: &str,
    op: Operation,
) -> Option<(Operation, Vec<&'a Expr>)> {
    if let Expr::Func { name, args, .. } = expr {
        if name == fn_name {
            return Some((op, vec![&args[0]]));
        }
    }
    None
}

static OP_SUB: LazyLock<Graph> =
    LazyLock::new(|| Graph::parse_glsl("void main() { result = a - b; }").unwrap());

static OP_SUB2: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
            void main() {
                neg_b = 0.0 - b;
                result = a + neg_b;
            }
        "};
    Graph::parse_glsl(query).unwrap()
});

fn op_sub<'a>(nodes: &'a [Node], expr: &'a Expr) -> Option<(Operation, Vec<&'a Expr>)> {
    // Some layers are simply subtracted like for xeno3/chr/chr/ch44000210.wimdo "ch45133501_body".
    let result = query_nodes(expr, nodes, &OP_SUB.nodes)
        .or_else(|| query_nodes(expr, nodes, &OP_SUB2.nodes))?;
    let a = result.get("a")?;
    let b = result.get("b")?;
    Some((Operation::Sub, vec![a, b]))
}

fn binary_op(
    expr: &Expr,
    binary_op: BinaryOp,
    operation: Operation,
) -> Option<(Operation, Vec<&Expr>)> {
    if let Expr::Binary(op, a0, a1) = expr {
        if *op == binary_op {
            return Some((operation, vec![a0, a1]));
        }
    }
    None
}

fn dependency_expr(e: &Expr, graph: &Graph, attributes: &Attributes) -> Option<Dependency> {
    texture_dependency(e, graph, attributes).or_else(|| {
        buffer_dependency(e)
            .map(Dependency::Buffer)
            .or_else(|| match e {
                Expr::Unary(UnaryOp::Negate, e) => {
                    if let Expr::Float(f) = **e {
                        Some(Dependency::Constant((-f).into()))
                    } else {
                        None
                    }
                }
                Expr::Float(f) => Some(Dependency::Constant((*f).into())),
                Expr::Global { name, channel } => {
                    // TODO: Also check if this matches a vertex input name?
                    Some(Dependency::Attribute(AttributeDependency {
                        name: name.into(),
                        channel: *channel,
                    }))
                }
                _ => None,
            })
    })
}

static OP_ADD_NORMAL: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
        void main() {
            n = n2 * temp1;
            neg_n = 0.0 - n;
            n = fma(temp2, temp3, neg_n);
            n_inv_sqrt = inversesqrt(temp4);
            neg_n1 = 0.0 - n1;
            r = fma(n, n_inv_sqrt, neg_n1);

            nom_work = nom_work;
            nom_work = fma(r, ratio, nom_work);
            inv_sqrt = inversesqrt(temp5);
            nom_work = nom_work * inv_sqrt;
        }
    "};
    Graph::parse_glsl(query).unwrap()
});

fn op_add_normal<'a>(nodes: &'a [Node], nom_work: &'a Expr) -> Option<(Operation, Vec<&'a Expr>)> {
    // getPixelCalcAddNormal in pcmdo shaders.
    // normalize(mix(nomWork, normalize(r), ratio))
    // XC2: ratio * (normalize(r) - nomWork) + nomWork
    // XC3: (normalize(r) - nomWork) * ratio + nomWork
    // TODO: Is it worth detecting the textures used for r?
    // TODO: nom_work and n1 are the same?
    // TODO: Reduce assignments to allow combining lines?
    // TODO: Allow 0.0 - x or -x
    let result = query_nodes(nom_work, nodes, &OP_ADD_NORMAL.nodes)?;
    let mut nom_work = *result.get("nom_work")?;
    let ratio = result.get("ratio")?;
    let n2 = result.get("n2")?;

    // Remove normal map channel remapping to avoid detecting this as a function.
    if let Some(new_nom_work) = normal_map_fma(nodes, nom_work) {
        nom_work = new_nom_work;
    }

    Some((Operation::AddNormal, vec![nom_work, n2, ratio]))
}

static OP_OVERLAY2: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
        void main() {
            ratio2 = b * b;
            ratio3 = ratio * ratio2;
            ratio4 = ratio * ratio3;
            ratio = clamp(ratio4, 0.0, 1.0);

            result4 = fma(a, -2.0, 2.0);
            neg_result4 = 0.0 - result4;
            result3 = fma(b, neg_result4, result4);
            neg_result3 = 0.0 - result3;
            result1 = fma(ratio, neg_result3, ratio);

            a_2 = a * 2.0;
            a_2_b = a_2 * b;
            neg_a_2_b = 0.0 - a_2_b;
            result2 = fma(ratio, neg_a_2_b, a_2_b);

            result = result1 + result2;
        }
    "};
    Graph::parse_glsl(query).unwrap()
});

fn op_overlay2<'a>(nodes: &'a [Node], nom_work: &'a Expr) -> Option<(Operation, Vec<&'a Expr>)> {
    let result = query_nodes(nom_work, nodes, &OP_OVERLAY2.nodes)?;
    let a = *result.get("a")?;
    let b = result.get("b")?;
    Some((Operation::Overlay2, vec![a, b]))
}

static NORMAL_MAP_FMA: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
        void main() {
            result = fma(result, 2.0, neg_one);
        }
    "};
    Graph::parse_glsl(query).unwrap()
});

fn normal_map_fma<'a>(nodes: &'a [Node], nom_work: &'a Expr) -> Option<&'a Expr> {
    // Extract the normal map texture if present.
    // This could be fma(x, 2.0, -1.0) or fma(x, 2.0, -1.0039216)
    let result = query_nodes(nom_work, nodes, &NORMAL_MAP_FMA.nodes)?;
    let neg_one = result.get("neg_one")?;
    match neg_one {
        Expr::Float(f) => {
            if f.abs_diff_eq(&-1.0, 1.0 / 128.0) {
                result.get("result").copied()
            } else {
                None
            }
        }
        Expr::Unary(UnaryOp::Negate, f) => {
            if matches!(**f, Expr::Float(f) if f.abs_diff_eq(&1.0, 1.0 / 128.0)) {
                result.get("result").copied()
            } else {
                None
            }
        }
        _ => None,
    }
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

static CALC_NORMAL_MAP_X: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
        void main() {
            inverse_length_tangent = inversesqrt(tangent_length);
            tangent = tangent.x;
            normalize_tangent = tangent * inverse_length_tangent;
            result_x = result_x;
            result = result_x * normalize_tangent;

            inverse_length_bitangent = inversesqrt(bitangent_length);
            bitangent = bitangent.x;
            normalize_bitangent = bitangent * inverse_length_bitangent;
            result_y = result_y;
            result = fma(result_y, normalize_bitangent, result);

            inverse_length_normal = inversesqrt(normal_length);
            normal = normal.x;
            normalize_normal = normal * inverse_length_normal;
            result_z = result_z;
            result = fma(result_z, normalize_normal, result);
        }
    "};
    Graph::parse_glsl(query).unwrap()
});

static CALC_NORMAL_MAP_Y: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
        void main() {
            inverse_length_tangent = inversesqrt(tangent_length);
            tangent = tangent.y;
            normalize_tangent = tangent * inverse_length_tangent;
            result_x = result_x;
            result = result_x * normalize_tangent;

            inverse_length_normal = inversesqrt(normal_length);
            normal = normal.y;
            normalize_normal = normal * inverse_length_normal;
            result_z = result_z;
            result = fma(result_z, normalize_normal, result);

            inverse_length_bitangent = inversesqrt(bitangent_length);
            bitangent = bitangent.y;
            normalize_bitangent = bitangent * inverse_length_bitangent;
            result_y = result_y;
            result = fma(result_y, normalize_bitangent, result);
        }
    "};
    Graph::parse_glsl(query).unwrap()
});

fn calc_normal_map<'a>(nodes: &'a [Node], expr: &'a Expr) -> Option<[&'a Expr; 3]> {
    let result = query_nodes(expr, nodes, &CALC_NORMAL_MAP_X.nodes)
        .or_else(|| query_nodes(expr, nodes, &CALC_NORMAL_MAP_Y.nodes))?;
    Some([
        result.get("result_x")?,
        result.get("result_y")?,
        result.get("result_z")?,
    ])
}

static CALC_NORMAL_MAP_W_INTENSITY_X: LazyLock<Graph> = LazyLock::new(|| {
    // normal.x with normal.w as normal map intensity.
    // TODO: Does intensity always use pow(intensity, 0.7)?
    let query = indoc! {"
        void main() {
            intensity = intensity;
            intensity = log2(intensity);
            intensity = intensity * 0.7;
            intensity = exp2(intensity);

            inverse_length_tangent = inversesqrt(tangent_length);
            tangent = tangent.x;
            normalize_tangent = tangent * inverse_length_tangent;
            result_x = result_x;
            result_x = result_x * normalize_tangent;
            result = result_x * intensity;

            inverse_length_normal = inversesqrt(normal_length);
            normal = normal.x;
            normalize_normal = normal * inverse_length_normal;
            result_z = result_z;
            result = fma(result_z, normalize_normal, result);

            inverse_length_bitangent = inversesqrt(bitangent_length);
            bitangent = bitangent.x;
            normalize_bitangent = bitangent * inverse_length_bitangent;
            result_y = result_y;
            result_y = normalize_bitangent * result_y;
            result = fma(intensity, result_y, result);
        }
    "};
    Graph::parse_glsl(query).unwrap()
});

static CALC_NORMAL_MAP_W_INTENSITY_Y: LazyLock<Graph> = LazyLock::new(|| {
    // normal.y with normal.w as normal map intensity.
    // TODO: Does intensity always use pow(intensity, 0.7)?
    let query = indoc! {"
        void main() {
            intensity = intensity;
            intensity = log2(intensity);
            intensity = intensity * 0.7;
            intensity = exp2(intensity);

            inverse_length_tangent = inversesqrt(tangent_length);
            tangent = tangent.y;
            normalize_tangent = tangent * inverse_length_tangent;
            result_x = result_x;
            result_x = result_x * normalize_tangent;
            result = result_x * intensity;

            inverse_length_normal = inversesqrt(normal_length);
            normal = normal.y;
            normalize_normal = normal * inverse_length_normal;
            result_z = result_z;
            result = fma(result_z, normalize_normal, result);

            inverse_length_bitangent = inversesqrt(bitangent_length);
            bitangent = bitangent.y;
            normalize_bitangent = bitangent * inverse_length_bitangent;
            result_y = result_y;
            result_y = normalize_bitangent * result_y;
            result = fma(intensity, result_y, result);
        }
    "};
    Graph::parse_glsl(query).unwrap()
});

fn calc_normal_map_w_intensity<'a>(
    nodes: &'a [Node],
    expr: &'a Expr,
) -> Option<([&'a Expr; 3], &'a Expr)> {
    let result = query_nodes(expr, nodes, &CALC_NORMAL_MAP_W_INTENSITY_X.nodes)
        .or_else(|| query_nodes(expr, nodes, &CALC_NORMAL_MAP_W_INTENSITY_Y.nodes))?;
    Some((
        [
            result.get("result_x")?,
            result.get("result_y")?,
            result.get("result_z")?,
        ],
        result.get("intensity")?,
    ))
}

static CALC_NORMAL_MAP_XCX_X: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
        void main() {
            inverse_length_tangent = inversesqrt(tangent_length);
            tangent = tangent.x;
            normalize_tangent = tangent * inverse_length_tangent;
            result_x = result_x;
            result = result_x * normalize_tangent;

            inverse_length_normal = inversesqrt(normal_length);
            normal = normal.x;
            normalize_normal = normal * inverse_length_normal;
            result_z = result_z;
            result = fma(result_z, normalize_normal, result);

            inverse_length_bitangent = inversesqrt(bitangent_length);
            bitangent = bitangent.x;
            normalize_bitangent = bitangent * inverse_length_bitangent;
            result_y = result_y;
            result = fma(result_y, normalize_bitangent, result);

            inverse_length_normal = inversesqrt(normal_length);
            result = result * inverse_length_normal;
            result = fma(normalize_val_inf, neg_dot_val_inf_normal, result);
        }
    "};
    Graph::parse_glsl(query).unwrap()
});

static CALC_NORMAL_MAP_XCX_Y: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
        void main() {
            inverse_length_tangent = inversesqrt(tangent_length);
            tangent = tangent.y;
            normalize_tangent = tangent * inverse_length_tangent;
            result_x = result_x;
            result = result_x * normalize_tangent;

            inverse_length_normal = inversesqrt(normal_length);
            normal = normal.y;
            normalize_normal = normal * inverse_length_normal;
            result_z = result_z;
            result = fma(result_z, normalize_normal, result);

            inverse_length_bitangent = inversesqrt(bitangent_length);
            bitangent = bitangent.y;
            normalize_bitangent = bitangent * inverse_length_bitangent;
            result_y = result_y;
            result = fma(result_y, normalize_bitangent, result);

            inverse_length_normal = inversesqrt(normal_length);
            result = result * inverse_length_normal;
            result = fma(normalize_val_inf, neg_dot_val_inf_normal, result);
        }
    "};
    Graph::parse_glsl(query).unwrap()
});

fn calc_normal_map_xcx<'a>(nodes: &'a [Node], expr: &'a Expr) -> Option<[&'a Expr; 3]> {
    let result = query_nodes(expr, nodes, &CALC_NORMAL_MAP_XCX_X.nodes)
        .or_else(|| query_nodes(expr, nodes, &CALC_NORMAL_MAP_XCX_Y.nodes))?;
    Some([
        result.get("result_x")?,
        result.get("result_y")?,
        result.get("result_z")?,
    ])
}

static GEOMETRIC_SPECULAR_AA: LazyLock<Graph> = LazyLock::new(|| {
    // calcGeometricSpecularAA in pcmdo shaders.
    // glossiness = 1.0 - sqrt(clamp((1.0 - glossiness)^2 + kernelRoughness2, 0.0, 1.0))
    // TODO: reduce assignments to allow combining lines
    // TODO: Allow 0.0 - x or -x
    let query = indoc! {"
        void main() {
            result = 0.0 - glossiness;
            result = 1.0 + result;
            result = fma(result, result, temp);
            result = clamp(result, 0.0, 1.0);
            result = sqrt(result);
            result = 0.0 - result;
            result = result + 1.0;
        }
    "};
    Graph::parse_glsl(query).unwrap()
});

fn geometric_specular_aa<'a>(nodes: &'a [Node], expr: &'a Expr) -> Option<&'a Expr> {
    let result = query_nodes(expr, nodes, &GEOMETRIC_SPECULAR_AA.nodes)?;
    result.get("glossiness").copied()
}

fn apply_vertex_uv_params(
    vertex: &Graph,
    vertex_attributes: &Attributes,
    fragment_attributes: &Attributes,
    dependency: &mut Dependency,
) {
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
                        &n.output.name == vertex_output_name && n.output.channel == texcoord.channel
                    }) {
                        // Detect common cases for transforming UV coordinates.
                        if let Some(new_texcoord) =
                            texcoord_params(vertex, &node.input, vertex_attributes)
                        {
                            *texcoord = new_texcoord;
                        }
                    }

                    // Also fix channels since the zw output may just be scaled vTex0.xy.
                    if let Some((actual_name, actual_channel)) = find_texcoord_input_name_channel(
                        vertex,
                        texcoord,
                        vertex_output_name,
                        vertex_attributes,
                    ) {
                        texcoord.name = actual_name.into();
                        texcoord.channel = actual_channel;
                    }
                }
            }
        }
    }
}

fn apply_expr_vertex_uv_params(
    value: &mut OutputExpr,
    vertex: &Graph,
    vertex_attributes: &Attributes,
    fragment_attributes: &Attributes,
) {
    // Add texture parameters used for the corresponding vertex output.
    // Most shaders apply UV transforms in the vertex shader.
    match value {
        OutputExpr::Value(d) => {
            apply_vertex_uv_params(vertex, vertex_attributes, fragment_attributes, d)
        }
        OutputExpr::Func { args, .. } => {
            for arg in args {
                apply_expr_vertex_uv_params(arg, vertex, vertex_attributes, fragment_attributes);
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
    use pretty_assertions::{assert_eq, assert_str_eq};

    macro_rules! assert_debug_eq {
        ($path:expr, $shader:expr) => {
            assert_str_eq!(include_str!($path), format!("{:#?}", $shader))
        };
    }

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
    fn shader_from_glsl_pyra_body() {
        // Test shaders from Pyra's metallic chest material.
        // xeno2/model/bl/bl000101, "ho_BL_TS2", shd0022
        let vert_glsl = include_str!("data/xc2/bl000101.22.vert");
        let frag_glsl = include_str!("data/xc2/bl000101.22.frag");
        let vertex = TranslationUnit::parse(vert_glsl).unwrap();
        let fragment = TranslationUnit::parse(frag_glsl).unwrap();

        let shader = shader_from_glsl(Some(&vertex), &fragment);
        assert_debug_eq!("data/xc2/bl000101.22.txt", shader);
    }

    #[test]
    fn shader_from_glsl_pyra_hair() {
        // xeno2/model/bl/bl000101, "_ho_hair_new", shd0008
        let vert_glsl = include_str!("data/xc2/bl000101.8.vert");
        let frag_glsl = include_str!("data/xc2/bl000101.8.frag");
        let vertex = TranslationUnit::parse(vert_glsl).unwrap();
        let fragment = TranslationUnit::parse(frag_glsl).unwrap();

        // Check that the color texture is multiplied by vertex color.
        let shader = shader_from_glsl(Some(&vertex), &fragment);
        assert_debug_eq!("data/xc2/bl000101.8.txt", shader);
    }

    #[test]
    fn shader_from_glsl_mio_skirt() {
        // xeno3/chr/ch/ch11021013, "body_skert2", shd0028
        let vert_glsl = include_str!("data/xc3/ch11021013.28.vert");
        let frag_glsl = include_str!("data/xc3/ch11021013.28.frag");
        let vertex = TranslationUnit::parse(vert_glsl).unwrap();
        let fragment = TranslationUnit::parse(frag_glsl).unwrap();

        // The pcmdo calcGeometricSpecularAA function compiles to the expression
        // glossiness = 1.0 - sqrt(clamp((1.0 - glossiness)^2 + kernelRoughness2 0.0, 1.0))
        // Consuming applications only care about the glossiness input.
        // This also avoids considering normal maps as a dependency.
        let shader = shader_from_glsl(Some(&vertex), &fragment);
        assert_debug_eq!("data/xc3/ch11021013.28.txt", shader);
    }

    #[test]
    fn shader_from_glsl_mio_metal() {
        // xeno3/chr/ch/ch11021013, "tlent_mio_metal1", shd0031
        let vert_glsl = include_str!("data/xc3/ch11021013.31.vert");
        let frag_glsl = include_str!("data/xc3/ch11021013.31.frag");
        let vertex = TranslationUnit::parse(vert_glsl).unwrap();
        let fragment = TranslationUnit::parse(frag_glsl).unwrap();

        // Test multiple calls to getPixelCalcAddNormal.
        let shader = shader_from_glsl(Some(&vertex), &fragment);
        assert_debug_eq!("data/xc3/ch11021013.31.txt", shader);
    }

    #[test]
    fn shader_from_glsl_mio_legs() {
        // xeno3/chr/ch/ch11021013, "body_stking1", shd0016
        let vert_glsl = include_str!("data/xc3/ch11021013.16.vert");
        let frag_glsl = include_str!("data/xc3/ch11021013.16.frag");
        let vertex = TranslationUnit::parse(vert_glsl).unwrap();
        let fragment = TranslationUnit::parse(frag_glsl).unwrap();

        // Test that color layers use the appropriate fresnel operation.
        let shader = shader_from_glsl(Some(&vertex), &fragment);
        assert_debug_eq!("data/xc3/ch11021013.16.txt", shader);
    }

    #[test]
    fn shader_from_glsl_mio_eyes() {
        // xeno3/chr/ch/ch01021011, "eye4", shd0063
        let vert_glsl = include_str!("data/xc3/ch01021011.63.vert");
        let frag_glsl = include_str!("data/xc3/ch01021011.63.frag");

        // Detect parallax mapping for texture coordinates.
        let vertex = TranslationUnit::parse(vert_glsl).unwrap();
        let fragment = TranslationUnit::parse(frag_glsl).unwrap();
        let shader = shader_from_glsl(Some(&vertex), &fragment);
        assert_debug_eq!("data/xc3/ch01021011.63.txt", shader);
    }

    #[test]
    fn shader_from_glsl_mio_ribbon() {
        // xeno3/chr/ch/ch01027000, "phong4", shd0044
        let vert_glsl = include_str!("data/xc3/ch01027000.44.vert");
        let frag_glsl = include_str!("data/xc3/ch01027000.44.frag");
        let vertex = TranslationUnit::parse(vert_glsl).unwrap();
        let fragment = TranslationUnit::parse(frag_glsl).unwrap();

        // Detect handling of gMatCol.
        let shader = shader_from_glsl(Some(&vertex), &fragment);
        assert_debug_eq!("data/xc3/ch01027000.44.txt", shader);
    }

    #[test]
    fn shader_from_glsl_wild_ride_body() {
        // xeno3/chr/ch/ch02010110, "body_m", shd0028
        let vert_glsl = include_str!("data/xc3/ch02010110.28.vert");
        let frag_glsl = include_str!("data/xc3/ch02010110.28.frag");
        let vertex = TranslationUnit::parse(vert_glsl).unwrap();
        let fragment = TranslationUnit::parse(frag_glsl).unwrap();

        // Some shaders use a simple mix() for normal blending.
        let shader = shader_from_glsl(Some(&vertex), &fragment);
        assert_debug_eq!("data/xc3/ch02010110.28.txt", shader);
    }

    #[test]
    fn shader_from_glsl_sena_body() {
        // xeno3/chr/ch/ch11061013, "bodydenim_toon", shd0009
        let vert_glsl = include_str!("data/xc3/ch11061013.9.vert");
        let frag_glsl = include_str!("data/xc3/ch11061013.9.frag");
        let vertex = TranslationUnit::parse(vert_glsl).unwrap();
        let fragment = TranslationUnit::parse(frag_glsl).unwrap();

        // Some shaders use multiple color blending modes.
        let shader = shader_from_glsl(Some(&vertex), &fragment);
        assert_debug_eq!("data/xc3/ch11061013.9.txt", shader);
    }

    #[test]
    fn shader_from_glsl_platform() {
        // xeno1/model/obj/oj110006, "ma14toride03", shd0003
        let vert_glsl = include_str!("data/xc1/oj110006.3.vert");
        let frag_glsl = include_str!("data/xc1/oj110006.3.frag");

        // Test detecting multiple normal layers with different blend modes.
        let vertex = TranslationUnit::parse(vert_glsl).unwrap();
        let fragment = TranslationUnit::parse(frag_glsl).unwrap();
        let shader = shader_from_glsl(Some(&vertex), &fragment);
        assert_debug_eq!("data/xc1/oj110006.3.txt", shader);
    }

    #[test]
    fn shader_from_glsl_xc1_normal_w_intensity() {
        // xeno1/model/pc/pc078702, "pc070702_body", shd0001
        let vert_glsl = include_str!("data/xc1/pc078702.1.vert");
        let frag_glsl = include_str!("data/xc1/pc078702.1.frag");

        // Test detecting xyz normal maps with vNormal.w intensity.
        let vertex = TranslationUnit::parse(vert_glsl).unwrap();
        let fragment = TranslationUnit::parse(frag_glsl).unwrap();
        let shader = shader_from_glsl(Some(&vertex), &fragment);
        assert_debug_eq!("data/xc1/pc078702.1.txt", shader);
    }

    #[test]
    fn shader_from_glsl_haze_body() {
        // xeno2/model/np/np001101, "body", shd0013
        let vert_glsl = include_str!("data/xc2/np001101.13.vert");
        let frag_glsl = include_str!("data/xc2/np001101.13.frag");
        let vertex = TranslationUnit::parse(vert_glsl).unwrap();
        let fragment = TranslationUnit::parse(frag_glsl).unwrap();

        // Test multiple normal layers with texture masks.
        let shader = shader_from_glsl(Some(&vertex), &fragment);
        assert_debug_eq!("data/xc2/np001101.13.txt", shader);
    }

    #[test]
    fn shader_from_glsl_pneuma_chest() {
        // xeno2/model/bl/bl000301, "tights_TS", shd0021
        let vert_glsl = include_str!("data/xc2/bl000301.21.vert");
        let frag_glsl = include_str!("data/xc2/bl000301.21.frag");

        // Test detecting the "PNEUMA" color layer.
        let vertex = TranslationUnit::parse(vert_glsl).unwrap();
        let fragment = TranslationUnit::parse(frag_glsl).unwrap();
        let shader = shader_from_glsl(Some(&vertex), &fragment);
        assert_debug_eq!("data/xc2/bl000301.21.txt", shader);
    }

    #[test]
    fn shader_from_glsl_tirkin_weapon() {
        // xeno2/model/we/we010402, "body_MT", shd0000
        let vert_glsl = include_str!("data/xc2/we010402.0.vert");
        let frag_glsl = include_str!("data/xc2/we010402.0.frag");
        let vertex = TranslationUnit::parse(vert_glsl).unwrap();
        let fragment = TranslationUnit::parse(frag_glsl).unwrap();

        // Test detecting layers for metalness.
        let shader = shader_from_glsl(Some(&vertex), &fragment);
        assert_debug_eq!("data/xc2/we010402.0.txt", shader);
    }

    #[test]
    fn shader_from_glsl_behemoth_fins() {
        // xeno2/model/en/en020601, "hire_a", shd0000
        let vert_glsl = include_str!("data/xc2/en020601.0.vert");
        let frag_glsl = include_str!("data/xc2/en020601.0.frag");
        let vertex = TranslationUnit::parse(vert_glsl).unwrap();
        let fragment = TranslationUnit::parse(frag_glsl).unwrap();

        // Test detecting layers for ambient occlusion.
        let shader = shader_from_glsl(Some(&vertex), &fragment);
        assert_debug_eq!("data/xc2/en020601.0.txt", shader);
    }

    #[test]
    fn shader_from_glsl_gramps_fur() {
        // xeno2/model/np/np000101, "_body_far_Fur", shd0009
        let vert_glsl = include_str!("data/xc2/np000101.9.vert");
        let frag_glsl = include_str!("data/xc2/np000101.9.frag");
        let vertex = TranslationUnit::parse(vert_glsl).unwrap();
        let fragment = TranslationUnit::parse(frag_glsl).unwrap();

        let shader = shader_from_glsl(Some(&vertex), &fragment);
        assert_debug_eq!("data/xc2/np000101.9.txt", shader);
    }

    #[test]
    fn shader_from_glsl_lysaat_eyes() {
        // xeno2/model/en/en030601, "phong3", shd0009
        let vert_glsl = include_str!("data/xc2/en030601.2.vert");
        let frag_glsl = include_str!("data/xc2/en030601.2.frag");
        let vertex = TranslationUnit::parse(vert_glsl).unwrap();
        let fragment = TranslationUnit::parse(frag_glsl).unwrap();

        // Detect parallax mapping for texture coordinates.
        let shader = shader_from_glsl(Some(&vertex), &fragment);
        assert_debug_eq!("data/xc2/en030601.2.txt", shader);
    }

    #[test]
    fn shader_from_glsl_noah_body_outline() {
        // xeno3/chr/ch/ch01011013, "body_outline", shd0000
        let vert_glsl = include_str!("data/xc3/ch01011013.0.vert");
        let frag_glsl = include_str!("data/xc3/ch01011013.0.frag");

        // Check for outline data.
        let vertex = TranslationUnit::parse(vert_glsl).unwrap();
        let fragment = TranslationUnit::parse(frag_glsl).unwrap();
        let shader = shader_from_glsl(Some(&vertex), &fragment);
        assert_debug_eq!("data/xc3/ch01011013.0.txt", shader);
    }

    #[test]
    fn shader_from_glsl_panacea_body() {
        // xeno3/chr/ch/ch44000210, "ch45133501_body", shd0029
        let vert_glsl = include_str!("data/xc3/ch44000210.29.vert");
        let frag_glsl = include_str!("data/xc3/ch44000210.29.frag");
        let vertex = TranslationUnit::parse(vert_glsl).unwrap();
        let fragment = TranslationUnit::parse(frag_glsl).unwrap();

        // Check for correct color layers
        let shader = shader_from_glsl(Some(&vertex), &fragment);
        assert_debug_eq!("data/xc3/ch44000210.29.txt", shader);
    }

    #[test]
    fn shader_from_latte_asm_elma_leg() {
        // xenox/chr_pc/pc221115.camdo, "leg_mat", shd0000
        let vert_asm = include_str!("data/xcx/pc221115.0.vert.txt");
        let frag_asm = include_str!("data/xcx/pc221115.0.frag.txt");

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
        let shader = shader_from_latte_asm(vert_asm, frag_asm, &fragment_shader);
        assert_debug_eq!("data/xcx/pc221115.0.txt", shader);
    }

    #[test]
    fn shader_from_glsl_l_face() {
        // xenoxde/chr/fc/fc181020, "facemat", shd0008
        let vert_glsl = include_str!("data/xcxde/fc181020.8.vert");
        let frag_glsl = include_str!("data/xcxde/fc181020.8.frag");

        // Check for overlay blending to make the face blue.
        let vertex = TranslationUnit::parse(vert_glsl).unwrap();
        let fragment = TranslationUnit::parse(frag_glsl).unwrap();
        let shader = shader_from_glsl(Some(&vertex), &fragment);
        assert_debug_eq!("data/xcxde/fc181020.8.txt", shader);
    }

    #[test]
    fn shader_from_glsl_elma_eye() {
        // xenoxde/chr/fc/fc281010, "eye_re", shd0002
        let vert_glsl = include_str!("data/xcxde/fc281010.2.vert");
        let frag_glsl = include_str!("data/xcxde/fc281010.2.frag");

        // Check reflection layers for the iris.
        let vertex = TranslationUnit::parse(vert_glsl).unwrap();
        let fragment = TranslationUnit::parse(frag_glsl).unwrap();
        let shader = shader_from_glsl(Some(&vertex), &fragment);
        assert_debug_eq!("data/xcxde/fc281010.2.txt", shader);
    }

    #[test]
    fn shader_from_glsl_elma_leg() {
        // xenoxde/chr/pc/pc221115, "leg_mat", shd0000
        let vert_glsl = include_str!("data/xcxde/pc221115.0.vert");
        let frag_glsl = include_str!("data/xcxde/pc221115.0.frag");

        // Check Xenoblade X specific normals and layering.
        let vertex = TranslationUnit::parse(vert_glsl).unwrap();
        let fragment = TranslationUnit::parse(frag_glsl).unwrap();
        let shader = shader_from_glsl(Some(&vertex), &fragment);
        assert_debug_eq!("data/xcxde/pc221115.0.txt", shader);
    }
}
