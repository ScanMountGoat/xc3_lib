use std::fmt::Write;
use std::{collections::BTreeMap, sync::LazyLock};

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
use indexmap::{IndexMap, IndexSet};
use indoc::{formatdoc, indoc};
use log::error;
use rayon::prelude::*;
use xc3_lib::{mths::Mths, spch::Spch};
use xc3_model::shader_database::{
    AttributeDependency, Dependency, Operation, OutputExpr, ProgramHash, ShaderDatabase,
    ShaderProgram,
};

use crate::{
    dependencies::{buffer_dependency, texture_dependency},
    extract::nvsd_glsl_name,
    graph::{
        glsl::shader_source_no_extensions,
        query::{assign_x, assign_x_recursive, fma_a_b_c, fma_half_half, normalize, query_nodes},
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

    // Create a combined graph that links vertex outputs to fragment inputs.
    // This effectively moves all shader logic to the fragment shader.
    // This simplifies generating shader code or material nodes in 3D applications.
    let graph = if let (Some(vert), Some(vert_attributes)) = (vert, vert_attributes) {
        merge_vertex_fragment(vert, &vert_attributes, frag, &frag_attributes)
    } else {
        frag
    };

    let mut output_dependencies = IndexMap::new();
    let mut normal_intensity = None;

    // Cache graph expr -> output expr index to visit nodes only once.
    let mut exprs = IndexSet::new();
    let mut expr_to_index = IndexMap::new();

    // Some shaders have up to 8 outputs.
    for i in frag_attributes.output_locations.right_values().copied() {
        for c in "xyzw".chars() {
            let name = format!("out_attr{i}");
            let dependent_lines = graph.dependencies_recursive(&name, Some(c), None);

            // TODO: Skip o3.xyw (depth) and o4.xyz (velocity)
            // TODO: skip using queries or use separate CLI command?
            let value;
            if i == 2 && (c == 'x' || c == 'y') {
                // The normals use XY for output index 2 for all games.
                let (new_value, intensity) =
                    normal_output_expr(&graph, &dependent_lines, &mut exprs, &mut expr_to_index)
                        .unzip();
                value = new_value;
                normal_intensity = intensity.flatten();
            } else if i == 2 && c == 'w' {
                // o2.w is n.z * 1000 + 0.5 for XC1 DE, XC2, and XC3.
                // This can be easily handled by consuming applications.
                // XCX and XCX DE only have 2 components.
                value = None;
            } else {
                // Xenoblade X DE uses different outputs than other games.
                // Detect color or params to handle different outputs and channels.
                // TODO: Detect if o2.x before remapping is used here?
                value = color_or_param_output_expr(
                    &graph,
                    &dependent_lines,
                    &mut exprs,
                    &mut expr_to_index,
                );
            };

            if let Some(value) = value {
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
        exprs: exprs.into_iter().collect(),
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

fn color_or_param_output_expr(
    frag: &Graph,
    dependent_lines: &[usize],
    exprs: &mut IndexSet<OutputExpr>,
    expr_to_index: &mut IndexMap<Expr, usize>,
) -> Option<usize> {
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

    if let Some(new_current) = geometric_specular_aa(&frag.nodes, current) {
        current = new_current;
    }

    Some(output_expr(current, frag, exprs, expr_to_index))
}

fn normal_output_expr(
    frag: &Graph,
    dependent_lines: &[usize],
    exprs: &mut IndexSet<OutputExpr>,
    expr_to_index: &mut IndexMap<Expr, usize>,
) -> Option<(usize, Option<usize>)> {
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
    view_normal = assign_x_recursive(&frag.nodes, view_normal);

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

    let value = output_expr(nom_work, frag, exprs, expr_to_index);

    let intensity = intensity.map(|i| output_expr(i, frag, exprs, expr_to_index));

    Some((value, intensity))
}

pub(crate) fn output_expr(
    expr: &Expr,
    graph: &Graph,
    exprs: &mut IndexSet<OutputExpr>,
    expr_to_index: &mut IndexMap<Expr, usize>,
) -> usize {
    // Cache graph input expressions to avoid processing nodes more than once while recursing.
    match expr_to_index.get(expr) {
        Some(i) => *i,
        None => {
            let original_expr = expr.clone();

            // Simplify any expressions that would interfere with queries.
            let mut expr = assign_x_recursive(&graph.nodes, expr);
            if let Some(new_expr) = normal_map_fma(&graph.nodes, expr) {
                expr = new_expr;
            }
            if let Some(new_expr) = normalize(&graph.nodes, expr) {
                expr = assign_x_recursive(&graph.nodes, new_expr);
            }

            // Detect attributes.
            // TODO: preserve the space for attributes like clip or view?
            if let Some(new_expr) = skin_attribute_xyzw(&graph.nodes, expr)
                .or_else(|| skin_attribute_xyz(&graph.nodes, expr))
                .or_else(|| skin_attribute_clip_space_xyzw(&graph.nodes, expr))
                .or_else(|| u_mdl_clip_attribute_xyzw(&graph.nodes, expr))
                .or_else(|| u_mdl_view_attribute_xyzw(&graph.nodes, expr))
                .or_else(|| u_mdl_attribute_xyz(&graph.nodes, expr))
            {
                expr = new_expr;
            }

            let mut expr = expr.clone();
            if let Some(new_expr) = skin_attribute_bitangent(&graph.nodes, &expr)
                .or_else(|| u_mdl_view_bitangent_xyz(&graph.nodes, &expr))
            {
                expr = new_expr;
            }

            let output = output_expr_inner(&expr, graph, exprs, expr_to_index);

            let index = exprs.insert_full(output).0;
            expr_to_index.insert(original_expr, index);

            index
        }
    }
}

fn output_expr_inner(
    expr: &Expr,
    graph: &Graph,
    exprs: &mut IndexSet<OutputExpr>,
    expr_to_index: &mut IndexMap<Expr, usize>,
) -> OutputExpr {
    if let Some(value) = extract_value(expr, graph, exprs, expr_to_index) {
        // The base case is a single value.
        OutputExpr::Value(value)
    } else {
        // Detect operations from most specific to least specific.
        // This results in fewer operations in many cases.
        if let Some((op, args)) = op_add_normal(&graph.nodes, expr)
            .or_else(|| op_monochrome(&graph.nodes, expr))
            .or_else(|| op_fresnel_ratio(&graph.nodes, expr))
            .or_else(|| op_overlay2(&graph.nodes, expr))
            .or_else(|| op_overlay_ratio(&graph.nodes, expr))
            .or_else(|| op_overlay(&graph.nodes, expr))
            .or_else(|| tex_parallax2(&graph.nodes, expr))
            .or_else(|| tex_parallax(&graph.nodes, expr))
            .or_else(|| tex_matrix(&graph.nodes, expr))
            .or_else(|| op_reflect(&graph.nodes, expr))
            .or_else(|| op_calc_normal_map(&graph.nodes, expr))
            .or_else(|| op_mix(&graph.nodes, expr))
            .or_else(|| op_mul_ratio(&graph.nodes, expr))
            .or_else(|| op_add_ratio(expr))
            .or_else(|| op_sub(&graph.nodes, expr))
            .or_else(|| op_div(&graph.nodes, expr))
            .or_else(|| binary_op(expr, BinaryOp::Mul, Operation::Mul))
            .or_else(|| binary_op(expr, BinaryOp::Add, Operation::Add))
            .or_else(|| op_pow(&graph.nodes, expr))
            .or_else(|| op_clamp(&graph.nodes, expr))
            .or_else(|| op_min(&graph.nodes, expr))
            .or_else(|| op_max(&graph.nodes, expr))
            .or_else(|| op_sqrt(&graph.nodes, expr))
            .or_else(|| op_dot(&graph.nodes, expr))
            .or_else(|| unary_op(expr, "abs", Operation::Abs))
            .or_else(|| unary_op(expr, "floor", Operation::Floor))
            .or_else(|| binary_op(expr, BinaryOp::Equal, Operation::Equal))
            .or_else(|| binary_op(expr, BinaryOp::NotEqual, Operation::NotEqual))
            .or_else(|| binary_op(expr, BinaryOp::Less, Operation::Less))
            .or_else(|| binary_op(expr, BinaryOp::Greater, Operation::Greater))
            .or_else(|| binary_op(expr, BinaryOp::LessEqual, Operation::LessEqual))
            .or_else(|| binary_op(expr, BinaryOp::GreaterEqual, Operation::GreaterEqual))
            .or_else(|| ternary(expr))
        {
            // Insert values that this operation depends on first.
            let args: Vec<_> = args
                .into_iter()
                .map(|arg| output_expr(arg, graph, exprs, expr_to_index))
                .collect();
            OutputExpr::Func { op, args }
        } else {
            // TODO: inversesqrt
            // TODO: exp2 should always be part of a pow expression
            // TODO: better fallback for unrecognized function or values?
            // TODO: log unsupported expr during database creation?
            // println!("{}", graph.expr_to_glsl(expr));
            OutputExpr::Func {
                op: Operation::Unk,
                args: Vec::new(),
            }
        }
    }
}

fn extract_value(
    expr: &Expr,
    graph: &Graph,
    exprs: &mut IndexSet<OutputExpr>,
    expr_to_index: &mut IndexMap<Expr, usize>,
) -> Option<Dependency> {
    let mut expr = assign_x_recursive(&graph.nodes, expr);
    if let Some(new_expr) = normalize(&graph.nodes, expr) {
        expr = new_expr;
    }
    if let Some(new_expr) = normal_map_fma(&graph.nodes, expr) {
        expr = new_expr;
    }

    dependency_expr(expr, graph, exprs, expr_to_index)
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

static FRESNEL_RATIO2: LazyLock<Graph> = LazyLock::new(|| {
    // Variant for XCX DE shaders with log2(abs()) instead of log2().
    // pow(1.0 - n_dot_v, ratio * 5.0)
    let query = indoc! {"
        void main() {
            n_dot_v = abs(n_dot_v);
            neg_n_dot_v = 0.0 - n_dot_v;
            one_minus_n_dot_v = neg_n_dot_v + 1.0;
            one_minus_n_dot_v = abs(one_minus_n_dot_v);
            result = log2(one_minus_n_dot_v);
            ratio = ratio * 5.0;
            result = ratio * result;
            result = exp2(result);
        }
    "};
    Graph::parse_glsl(query).unwrap()
});

fn op_fresnel_ratio<'a>(nodes: &'a [Node], expr: &'a Expr) -> Option<(Operation, Vec<&'a Expr>)> {
    let result = query_nodes(expr, nodes, &FRESNEL_RATIO.nodes)
        .or_else(|| query_nodes(expr, nodes, &FRESNEL_RATIO2.nodes))?;
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
    let result = query_nodes(expr, nodes, &OP_CLAMP.nodes)?;
    let a = result.get("a")?;
    let b = result.get("b")?;
    let c = result.get("c")?;
    Some((Operation::Clamp, vec![a, b, c]))
}

static OP_SQRT: LazyLock<Graph> = LazyLock::new(|| {
    // Equivalent to sqrt(result)
    let query = indoc! {"
        void main() {
            result = inversesqrt(result);
            result = 1.0 / result;
        }
    "};
    Graph::parse_glsl(query).unwrap()
});

static OP_SQRT2: LazyLock<Graph> =
    LazyLock::new(|| Graph::parse_glsl("void main() { result = sqrt(result); }").unwrap());

fn op_sqrt<'a>(nodes: &'a [Node], expr: &'a Expr) -> Option<(Operation, Vec<&'a Expr>)> {
    let result = query_nodes(expr, nodes, &OP_SQRT.nodes)
        .or_else(|| query_nodes(expr, nodes, &OP_SQRT2.nodes))?;
    let result = result.get("result")?;
    Some((Operation::Sqrt, vec![result]))
}

static OP_DOT4: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
        void main() {
            result = dot(vec4(ax, ay, az, aw), vec4(bx, by, bz, bw));
        }
    "};
    Graph::parse_glsl(query).unwrap()
});

fn op_dot<'a>(nodes: &'a [Node], expr: &'a Expr) -> Option<(Operation, Vec<&'a Expr>)> {
    let result = query_nodes(expr, nodes, &OP_DOT4.nodes)?;

    let ax = result.get("ax")?;
    let ay = result.get("ay")?;
    let az = result.get("az")?;
    let aw = result.get("aw")?;

    let bx = result.get("bx")?;
    let by = result.get("by")?;
    let bz = result.get("bz")?;
    let bw = result.get("bw")?;

    Some((Operation::Dot4, vec![ax, ay, az, aw, bx, by, bz, bw]))
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

fn ternary(expr: &Expr) -> Option<(Operation, Vec<&Expr>)> {
    if let Expr::Ternary(cond, a, b) = expr {
        Some((Operation::Select, vec![cond, a, b]))
    } else {
        None
    }
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

static OP_DIV: LazyLock<Graph> =
    LazyLock::new(|| Graph::parse_glsl("void main() { result = a / b; }").unwrap());

static OP_DIV2: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
            void main() {
                one_over_b = 1.0 / b;
                result = a * one_over_b;
            }
        "};
    Graph::parse_glsl(query).unwrap()
});

fn op_div<'a>(nodes: &'a [Node], expr: &'a Expr) -> Option<(Operation, Vec<&'a Expr>)> {
    let result = query_nodes(expr, nodes, &OP_DIV.nodes)
        .or_else(|| query_nodes(expr, nodes, &OP_DIV2.nodes))?;
    let a = result.get("a")?;
    let b = result.get("b")?;
    Some((Operation::Div, vec![a, b]))
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

fn dependency_expr(
    e: &Expr,
    graph: &Graph,
    exprs: &mut IndexSet<OutputExpr>,
    expr_to_index: &mut IndexMap<Expr, usize>,
) -> Option<Dependency> {
    texture_dependency(e, graph, exprs, expr_to_index).or_else(|| {
        buffer_dependency(e)
            .map(Dependency::Buffer)
            .or_else(|| match e {
                Expr::Unary(UnaryOp::Negate, e) => {
                    if let Expr::Float(f) = **e {
                        Some(Dependency::Constant(-f))
                    } else {
                        None
                    }
                }
                Expr::Float(f) => Some(Dependency::Constant(*f)),
                Expr::Global { name, channel } => {
                    // TODO: Also check if this matches a vertex input name?
                    Some(Dependency::Attribute(AttributeDependency {
                        name: name.clone(),
                        channel: *channel,
                    }))
                }
                _ => None,
            })
    })
}

static OP_MONOCHROME: LazyLock<Graph> = LazyLock::new(|| {
    // result = mix(color, dot(color, vec3(0.01, 0.01, 0.01), ratio))
    let query = indoc! {"
        void main() {
            b = x * 0.01;
            b = fma(y, 0.01, b);
            b = fma(z, 0.01, b);
            neg_a = 0.0 - a;
            b_minus_a = neg_a + b;
            result = fma(b_minus_a, ratio, a);
        }
    "};
    Graph::parse_glsl(query).unwrap()
});

static OP_MONOCHROME_XC1: LazyLock<Graph> = LazyLock::new(|| {
    // result = mix(color, dot(color, vec3(0.3, 0.59, 0.11), ratio))
    let query = indoc! {"
        void main() {
            b = x * 0.3;
            b = fma(y, 0.59, b);
            b = fma(z, 0.11, b);
            neg_a = 0.0 - a;
            b_minus_a = neg_a + b;
            result = fma(b_minus_a, ratio, a);
        }
    "};
    Graph::parse_glsl(query).unwrap()
});

fn op_monochrome<'a>(nodes: &'a [Node], expr: &'a Expr) -> Option<(Operation, Vec<&'a Expr>)> {
    // calcMonochrome in pcmdo fragment shaders for XC1 and XC3.
    // TODO: Create separate ops or include the RGB weights in the args?
    let result = query_nodes(expr, nodes, &OP_MONOCHROME.nodes)
        .or_else(|| query_nodes(expr, nodes, &OP_MONOCHROME_XC1.nodes))?;
    let a = result.get("a")?;
    let x = result.get("x")?;
    let y = result.get("y")?;
    let z = result.get("z")?;
    let ratio = result.get("ratio")?;

    let operation = if a == x {
        Operation::MonochromeX
    } else if a == y {
        Operation::MonochromeY
    } else if a == z {
        Operation::MonochromeZ
    } else {
        Operation::Unk
    };
    Some((operation, vec![x, y, z, ratio]))
}

static OP_ADD_NORMAL: LazyLock<Graph> = LazyLock::new(|| {
    // t = n1.xyz + vec3(0.0, 0.0, 1.0);
    // u = n2.xyz * vec3(-1.0, -1.0, 1.0);
    // r = t * dot(t, u) - u * t.z;
    // result = normalize(mix(n1, normalize(r), ratio));
    let query = indoc! {"
        void main() {
            n1_x = 0.0 + n1_x;
            neg_n1_x = 0.0 - n1_x;
            dot_t_u = n2_x * neg_n1_x;
            n1_y = 0.0 + n1_y;
            neg_n1_y = 0.0 - n1_y;
            dot_t_u = fma(n2_y, neg_n1_y, dot_t_u);
            one_plus_n1_z = n1_z + 1.0;
            dot_t_u = fma(n2_z, one_plus_n1_z, dot_t_u);
            temp6 = fma(temp2, dot_t_u, neg_n2);

            n_inv_sqrt = inversesqrt(temp4);
            r = fma(temp6, n_inv_sqrt, neg_n1);

            nom_work = fma(r, ratio, nom_work);
        }
    "};
    Graph::parse_glsl(query).unwrap()
});

static OP_ADD_NORMAL_OUTER: LazyLock<Graph> = LazyLock::new(|| {
    // Slightly different version of dot(t, u) for the outermost call.
    let query = indoc! {"
        void main() {
            n1_x = fma(n1_x, n1_inverse_sqrt, 0.0);
            n1_y = fma(n1_y, n1_inverse_sqrt, 0.0);
            n1_z_plus_one = fma(n1_z, n1_inverse_sqrt, 1.0);
            neg_n1_x = 0.0 - n1_x;
            dot_t_u = n2_x * neg_n1_x;
            neg_n1_y = 0.0 - n1_y;
            dot_t_u = fma(n2_y, neg_n1_y, dot_t_u);
            dot_t_u = fma(n2_z, n1_z_plus_one, dot_t_u);
            temp6 = fma(n1_x, dot_t_u, neg_n2);

            n_inv_sqrt = inversesqrt(temp4);
            r = fma(temp6, n_inv_sqrt, neg_n1);

            nom_work = fma(r, ratio, nom_work);
        }
    "};
    Graph::parse_glsl(query).unwrap()
});

fn op_add_normal<'a>(nodes: &'a [Node], expr: &'a Expr) -> Option<(Operation, Vec<&'a Expr>)> {
    // getPixelCalcAddNormal in pcmdo shaders.
    // normalize(mix(nomWork, normalize(r), ratio))
    // XC2: ratio * (normalize(r) - nomWork) + nomWork
    // XC3: (normalize(r) - nomWork) * ratio + nomWork

    // The normalize is baked into the outer query and might not be present.
    let mut expr = expr;
    if let Some(new_expr) = normalize(nodes, expr) {
        expr = assign_x_recursive(nodes, new_expr);
    }

    let result = query_nodes(expr, nodes, &OP_ADD_NORMAL_OUTER.nodes)
        .or_else(|| query_nodes(expr, nodes, &OP_ADD_NORMAL.nodes))?;

    let n1_x = result.get("n1_x")?;
    let n1_y = result.get("n1_y")?;

    let n2_x = result.get("n2_x")?;
    let n2_y = result.get("n2_y")?;

    let ratio = result.get("ratio")?;

    let mut nom_work = *result.get("nom_work")?;
    nom_work = assign_x_recursive(nodes, nom_work);
    if let Some(new_expr) = normalize(nodes, nom_work) {
        nom_work = assign_x_recursive(nodes, new_expr);
    }

    let op = if nom_work == assign_x_recursive(nodes, n1_x) {
        Operation::AddNormalX
    } else if nom_work == assign_x_recursive(nodes, n1_y) {
        Operation::AddNormalY
    } else {
        Operation::Unk
    };

    Some((op, vec![n1_x, n1_y, n2_x, n2_y, ratio]))
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

fn calc_normal_map_w_intensity_query(c: char) -> String {
    formatdoc! {"
        void main() {{
            intensity = intensity;
            intensity = log2(intensity);
            intensity = intensity * 0.7;
            intensity = exp2(intensity);

            inverse_length_tangent = inversesqrt(tangent_length);
            tangent = tangent.{c};
            normalize_tangent = tangent * inverse_length_tangent;
            result_x = result_x;
            result_x = result_x * normalize_tangent;
            result = result_x * intensity;

            inverse_length_normal = inversesqrt(normal_length);
            normal = normal.{c};
            normalize_normal = normal * inverse_length_normal;
            result_z = result_z;
            result = fma(result_z, normalize_normal, result);

            inverse_length_bitangent = inversesqrt(bitangent_length);
            bitangent = bitangent.{c};
            normalize_bitangent = bitangent * inverse_length_bitangent;
            result_y = result_y;
            result_y = normalize_bitangent * result_y;
            result = fma(intensity, result_y, result);
        }}
    "}
}

static CALC_NORMAL_MAP_W_INTENSITY_X: LazyLock<Graph> = LazyLock::new(|| {
    // normal.x with normal.w as normal map intensity.
    // TODO: Does intensity always use pow(intensity, 0.7)?
    let query = calc_normal_map_w_intensity_query('x');
    Graph::parse_glsl(&query).unwrap()
});

static CALC_NORMAL_MAP_W_INTENSITY_Y: LazyLock<Graph> = LazyLock::new(|| {
    // normal.y with normal.w as normal map intensity.
    // TODO: Does intensity always use pow(intensity, 0.7)?
    let query = calc_normal_map_w_intensity_query('y');
    Graph::parse_glsl(&query).unwrap()
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

fn calc_normal_map_val_inf_xcx_query(c: char) -> String {
    formatdoc! {"
        void main() {{
            inverse_length_tangent = inversesqrt(tangent_length);
            tangent = tangent.{c};
            normalize_tangent = tangent * inverse_length_tangent;
            result_x = result_x;
            result = result_x * normalize_tangent;

            inverse_length_normal = inversesqrt(normal_length);
            normal = normal.{c};
            normalize_normal = normal * inverse_length_normal;
            result_z = result_z;
            result = fma(result_z, normalize_normal, result);

            inverse_length_bitangent = inversesqrt(bitangent_length);
            bitangent = bitangent.{c};
            normalize_bitangent = bitangent * inverse_length_bitangent;
            result_y = result_y;
            result = fma(result_y, normalize_bitangent, result);

            inverse_length_normal = inversesqrt(normal_length);
            result = result * inverse_length_normal;
            result = fma(normalize_val_inf, neg_dot_val_inf_normal, result);
        }}
    "}
}

static CALC_NORMAL_MAP_VAL_INF_XCX_X: LazyLock<Graph> = LazyLock::new(|| {
    let query = calc_normal_map_val_inf_xcx_query('x');
    Graph::parse_glsl(&query).unwrap()
});

static CALC_NORMAL_MAP_VAL_INF_XCX_Y: LazyLock<Graph> = LazyLock::new(|| {
    let query = calc_normal_map_val_inf_xcx_query('y');
    Graph::parse_glsl(&query).unwrap()
});

static CALC_NORMAL_MAP_VAL_INF_XCX_Z: LazyLock<Graph> = LazyLock::new(|| {
    let query = calc_normal_map_val_inf_xcx_query('z');
    Graph::parse_glsl(&query).unwrap()
});

fn calc_normal_map_xcx<'a>(nodes: &'a [Node], expr: &'a Expr) -> Option<[&'a Expr; 3]> {
    let result = query_nodes(expr, nodes, &CALC_NORMAL_MAP_VAL_INF_XCX_X.nodes)
        .or_else(|| query_nodes(expr, nodes, &CALC_NORMAL_MAP_VAL_INF_XCX_Y.nodes))?;
    Some([
        result.get("result_x")?,
        result.get("result_y")?,
        result.get("result_z")?,
    ])
}

fn calc_normal_map_xcx_query(c: char) -> String {
    formatdoc! {"
        void main() {{
            inverse_length_tangent = inversesqrt(tangent_length);
            tangent = tangent.{c};
            normalize_tangent = tangent * inverse_length_tangent;
            result_x = result_x;
            result = result_x * normalize_tangent;

            inverse_length_normal = inversesqrt(normal_length);
            normal = normal.{c};
            normalize_normal = normal * inverse_length_normal;
            result_z = result_z;
            result = fma(result_z, normalize_normal, result);

            inverse_length_bitangent = inversesqrt(bitangent_length);
            bitangent = bitangent.{c};
            normalize_bitangent = bitangent * inverse_length_bitangent;
            result_y = result_y;
            result = fma(result_y, normalize_bitangent, result);
        }}
    "}
}

static CALC_NORMAL_MAP_XCX_X: LazyLock<Graph> = LazyLock::new(|| {
    let query = calc_normal_map_xcx_query('x');
    Graph::parse_glsl(&query).unwrap()
});

static CALC_NORMAL_MAP_XCX_Y: LazyLock<Graph> = LazyLock::new(|| {
    let query = calc_normal_map_xcx_query('y');
    Graph::parse_glsl(&query).unwrap()
});

static CALC_NORMAL_MAP_XCX_Z: LazyLock<Graph> = LazyLock::new(|| {
    let query = calc_normal_map_xcx_query('z');
    Graph::parse_glsl(&query).unwrap()
});

fn op_calc_normal_map<'a>(nodes: &'a [Node], expr: &'a Expr) -> Option<(Operation, Vec<&'a Expr>)> {
    // TODO: Detect normal mapping from other games.
    let (op, result) = query_nodes(expr, nodes, &CALC_NORMAL_MAP_XCX_X.nodes)
        .or_else(|| query_nodes(expr, nodes, &CALC_NORMAL_MAP_VAL_INF_XCX_X.nodes))
        .map(|r| (Operation::NormalMapX, r))
        .or_else(|| {
            query_nodes(expr, nodes, &CALC_NORMAL_MAP_XCX_Y.nodes)
                .or_else(|| query_nodes(expr, nodes, &CALC_NORMAL_MAP_VAL_INF_XCX_Y.nodes))
                .map(|r| (Operation::NormalMapY, r))
        })
        .or_else(|| {
            query_nodes(expr, nodes, &CALC_NORMAL_MAP_XCX_Z.nodes)
                .or_else(|| query_nodes(expr, nodes, &CALC_NORMAL_MAP_VAL_INF_XCX_Z.nodes))
                .map(|r| (Operation::NormalMapZ, r))
        })?;

    // Don't store result_z since it can be calculated from result_x and result_y.
    Some((op, vec![result.get("result_x")?, result.get("result_y")?]))
}

static GEOMETRIC_SPECULAR_AA: LazyLock<Graph> = LazyLock::new(|| {
    // calcGeometricSpecularAA in pcmdo shaders.
    // glossiness = 1.0 - sqrt(clamp((1.0 - glossiness)^2 + kernelRoughness2, 0.0, 1.0))
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

static SKIN_ATTRIBUTE_XYZ_X: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
        void main() {
            temp_0 = nWgtIdx.x;
            temp_1 = floatBitsToInt(temp_0) & 65535;
            temp_2 = temp_1 * 48;
            temp_3 = result_x;
            temp_4 = floatBitsToUint(temp_0) >> 16;
            temp_5 = int(temp_4) * 48;
            temp_6 = temp_5 << 16;
            temp_7 = temp_6 + temp_2;
            temp_14 = result_y;
            temp_17 = result_z;
            temp_30 = uint(temp_7) >> 2;
            temp_31 = uintBitsToFloat(U_Bone.data[int(temp_30)]);
            temp_32 = temp_7 + 4;
            temp_33 = uint(temp_32) >> 2;
            temp_34 = uintBitsToFloat(U_Bone.data[int(temp_33)]);
            temp_35 = temp_7 + 8;
            temp_36 = uint(temp_35) >> 2;
            temp_37 = uintBitsToFloat(U_Bone.data[int(temp_36)]);
            temp_59 = temp_31 * temp_3;
            temp_69 = fma(temp_34, temp_14, temp_59);
            temp_73 = fma(temp_37, temp_17, temp_69);
        }
    "};
    Graph::parse_glsl(query).unwrap()
});

static SKIN_ATTRIBUTE_XYZ_Y: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
        void main() {
            temp_0 = nWgtIdx.x;
            temp_1 = floatBitsToInt(temp_0) & 65535;
            temp_2 = temp_1 * 48;
            temp_3 = result_x;
            temp_4 = floatBitsToUint(temp_0) >> 16;
            temp_5 = int(temp_4) * 48;
            temp_6 = temp_5 << 16;
            temp_7 = temp_6 + temp_2;
            temp_13 = temp_7 + 16;
            temp_14 = result_y;
            temp_17 = result_z;
            temp_41 = uint(temp_13) >> 2;
            temp_42 = uintBitsToFloat(U_Bone.data[int(temp_41)]);
            temp_43 = temp_13 + 4;
            temp_44 = uint(temp_43) >> 2;
            temp_45 = uintBitsToFloat(U_Bone.data[int(temp_44)]);
            temp_46 = temp_13 + 8;
            temp_47 = uint(temp_46) >> 2;
            temp_48 = uintBitsToFloat(U_Bone.data[int(temp_47)]);
            temp_64 = temp_42 * temp_3;
            temp_80 = fma(temp_45, temp_14, temp_64);
            temp_88 = fma(temp_48, temp_17, temp_80);
        }
    "};
    Graph::parse_glsl(query).unwrap()
});

static SKIN_ATTRIBUTE_XYZ_Z: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
        void main() {
            temp_0 = nWgtIdx.x;
            temp_1 = floatBitsToInt(temp_0) & 65535;
            temp_2 = temp_1 * 48;
            temp_3 = result_x;
            temp_4 = floatBitsToUint(temp_0) >> 16;
            temp_5 = int(temp_4) * 48;
            temp_6 = temp_5 << 16;
            temp_7 = temp_6 + temp_2;
            temp_10 = temp_7 + 32;
            temp_14 = result_y;
            temp_17 = result_z;
            temp_18 = uint(temp_10) >> 2;
            temp_19 = uintBitsToFloat(U_Bone.data[int(temp_18)]);
            temp_20 = temp_10 + 4;
            temp_21 = uint(temp_20) >> 2;
            temp_22 = uintBitsToFloat(U_Bone.data[int(temp_21)]);
            temp_23 = temp_10 + 8;
            temp_24 = uint(temp_23) >> 2;
            temp_25 = uintBitsToFloat(U_Bone.data[int(temp_24)]);
            temp_62 = temp_19 * temp_3;
            temp_68 = fma(temp_22, temp_14, temp_62);
            temp_83 = fma(temp_25, temp_17, temp_68);
        }
    "};
    Graph::parse_glsl(query).unwrap()
});

fn skin_attribute_xyz<'a>(nodes: &'a [Node], expr: &'a Expr) -> Option<&'a Expr> {
    query_nodes(expr, nodes, &SKIN_ATTRIBUTE_XYZ_X.nodes)
        .and_then(|r| r.get("result_x").copied())
        .or_else(|| {
            query_nodes(expr, nodes, &SKIN_ATTRIBUTE_XYZ_Y.nodes)
                .and_then(|r| r.get("result_y").copied())
        })
        .or_else(|| {
            query_nodes(expr, nodes, &SKIN_ATTRIBUTE_XYZ_Z.nodes)
                .and_then(|r| r.get("result_z").copied())
        })
}

static SKIN_ATTRIBUTE_XYZW_X: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
        void main() {
            temp_0 = nWgtIdx.x;
            temp_1 = floatBitsToInt(temp_0) & 65535;
            temp_2 = temp_1 * 48;
            temp_4 = floatBitsToUint(temp_0) >> 16;
            temp_5 = int(temp_4) * 48;
            temp_6 = temp_5 << 16;
            temp_7 = temp_6 + temp_2;
            temp_9 = result_x;
            temp_16 = result_y;
            temp_30 = uint(temp_7) >> 2;
            temp_31 = uintBitsToFloat(U_Bone.data[int(temp_30)]);
            temp_32 = temp_7 + 4;
            temp_33 = uint(temp_32) >> 2;
            temp_34 = uintBitsToFloat(U_Bone.data[int(temp_33)]);
            temp_35 = temp_7 + 8;
            temp_36 = uint(temp_35) >> 2;
            temp_37 = uintBitsToFloat(U_Bone.data[int(temp_36)]);
            temp_38 = temp_7 + 12;
            temp_39 = uint(temp_38) >> 2;
            temp_40 = uintBitsToFloat(U_Bone.data[int(temp_39)]);
            temp_52 = result_z;
            temp_53 = result_w;
            temp_61 = temp_31 * temp_9;
            temp_70 = fma(temp_34, temp_16, temp_61);
            temp_75 = fma(temp_37, temp_52, temp_70);
            temp_79 = fma(temp_40, temp_53, temp_75);
        }
    "};
    Graph::parse_glsl(query).unwrap()
});

static SKIN_ATTRIBUTE_XYZW_Y: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
        void main() {
            temp_0 = nWgtIdx.x;
            temp_1 = floatBitsToInt(temp_0) & 65535;
            temp_2 = temp_1 * 48;
            temp_4 = floatBitsToUint(temp_0) >> 16;
            temp_5 = int(temp_4) * 48;
            temp_6 = temp_5 << 16;
            temp_7 = temp_6 + temp_2;
            temp_9 = result_x;
            temp_13 = temp_7 + 16;
            temp_16 = result_y;
            temp_41 = uint(temp_13) >> 2;
            temp_42 = uintBitsToFloat(U_Bone.data[int(temp_41)]);
            temp_43 = temp_13 + 4;
            temp_44 = uint(temp_43) >> 2;
            temp_45 = uintBitsToFloat(U_Bone.data[int(temp_44)]);
            temp_46 = temp_13 + 8;
            temp_47 = uint(temp_46) >> 2;
            temp_48 = uintBitsToFloat(U_Bone.data[int(temp_47)]);
            temp_49 = temp_13 + 12;
            temp_50 = uint(temp_49) >> 2;
            temp_51 = uintBitsToFloat(U_Bone.data[int(temp_50)]);
            temp_52 = result_z;
            temp_53 = result_w;
            temp_63 = temp_42 * temp_9;
            temp_72 = fma(temp_45, temp_16, temp_63);
            temp_78 = fma(temp_48, temp_52, temp_72);
            temp_84 = fma(temp_51, temp_53, temp_78);
        }
    "};
    Graph::parse_glsl(query).unwrap()
});

static SKIN_ATTRIBUTE_XYZW_Z: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
        void main() {
            temp_0 = nWgtIdx.x;
            temp_1 = floatBitsToInt(temp_0) & 65535;
            temp_2 = temp_1 * 48;
            temp_4 = floatBitsToUint(temp_0) >> 16;
            temp_5 = int(temp_4) * 48;
            temp_6 = temp_5 << 16;
            temp_7 = temp_6 + temp_2;
            temp_9 = result_x;
            temp_10 = temp_7 + 32;
            temp_16 = result_y;
            temp_18 = uint(temp_10) >> 2;
            temp_19 = uintBitsToFloat(U_Bone.data[int(temp_18)]);
            temp_20 = temp_10 + 4;
            temp_21 = uint(temp_20) >> 2;
            temp_22 = uintBitsToFloat(U_Bone.data[int(temp_21)]);
            temp_23 = temp_10 + 8;
            temp_24 = uint(temp_23) >> 2;
            temp_25 = uintBitsToFloat(U_Bone.data[int(temp_24)]);
            temp_26 = temp_10 + 12;
            temp_27 = uint(temp_26) >> 2;
            temp_28 = uintBitsToFloat(U_Bone.data[int(temp_27)]);
            temp_52 = result_z;
            temp_53 = result_w;
            temp_58 = temp_19 * temp_9;
            temp_76 = fma(temp_22, temp_16, temp_58);
            temp_85 = fma(temp_25, temp_52, temp_76);
            temp_89 = fma(temp_28, temp_53, temp_85);
        }
    "};
    Graph::parse_glsl(query).unwrap()
});

fn skin_attribute_xyzw<'a>(nodes: &'a [Node], expr: &'a Expr) -> Option<&'a Expr> {
    query_nodes(expr, nodes, &SKIN_ATTRIBUTE_XYZW_X.nodes)
        .and_then(|r| r.get("result_x").copied())
        .or_else(|| {
            query_nodes(expr, nodes, &SKIN_ATTRIBUTE_XYZW_Y.nodes)
                .and_then(|r| r.get("result_y").copied())
        })
        .or_else(|| {
            query_nodes(expr, nodes, &SKIN_ATTRIBUTE_XYZW_Z.nodes)
                .and_then(|r| r.get("result_z").copied())
        })
}

static SKIN_ATTRIBUTE_CLIP_XYZW: LazyLock<Graph> = LazyLock::new(|| {
    // TODO: Detect this as matrix multiplication and regular skinning?
    let query = indoc! {"
        void main() {
            temp_3 = result_x;
            temp_8 = result_y;
            temp_9 = result_z;
            temp_11 = result_w;
            temp_15 = uintBitsToFloat(U_Bone.data[int(temp_14)]);
            temp_18 = uintBitsToFloat(U_Bone.data[int(temp_17)]);
            temp_21 = uintBitsToFloat(U_Bone.data[int(temp_20)]);
            temp_24 = uintBitsToFloat(U_Bone.data[int(temp_23)]);
            temp_30 = uintBitsToFloat(U_Bone.data[int(temp_29)]);
            temp_33 = uintBitsToFloat(U_Bone.data[int(temp_32)]);
            temp_36 = uintBitsToFloat(U_Bone.data[int(temp_35)]);
            temp_39 = uintBitsToFloat(U_Bone.data[int(temp_38)]);
            temp_41 = uintBitsToFloat(U_Bone.data[int(temp_40)]);
            temp_44 = uintBitsToFloat(U_Bone.data[int(temp_43)]);
            temp_47 = uintBitsToFloat(U_Bone.data[int(temp_46)]);
            temp_50 = uintBitsToFloat(U_Bone.data[int(temp_49)]);
            temp_58 = temp_15 * temp_3;
            temp_59 = fma(temp_18, temp_8, temp_58);
            temp_61 = fma(temp_21, temp_9, temp_59);
            temp_62 = fma(temp_24, temp_11, temp_61);
            temp_63 = temp_30 * temp_3;
            temp_64 = temp_41 * temp_3;
            temp_65 = fma(temp_33, temp_8, temp_63);
            temp_66 = fma(temp_36, temp_9, temp_65);
            temp_67 = fma(temp_44, temp_8, temp_64);
            temp_68 = fma(temp_39, temp_11, temp_66);
            temp_70 = fma(temp_47, temp_9, temp_67);
            temp_72 = fma(temp_50, temp_11, temp_70);
            temp_139 = temp_62 * U_Static.gmProj[i].x;
            temp_155 = fma(temp_68, U_Static.gmProj[i].y, temp_139);
            temp_160 = fma(temp_72, U_Static.gmProj[i].z, temp_155);
            temp_168 = temp_160 + U_Static.gmProj[i].w;
        }
    "};
    Graph::parse_glsl(query).unwrap()
});

static SKIN_ATTRIBUTE_CLIP_XYZW_Z: LazyLock<Graph> = LazyLock::new(|| {
    // TODO: Detect this as matrix multiplication and regular skinning?
    let query = indoc! {"
        void main() {
            temp_1 = result_x;
            temp_2 = result_y;
            temp_3 = result_z;
            temp_4 = result_w;
            temp_17 = uintBitsToFloat(U_Bone.data[int(temp_16)]);
            temp_20 = uintBitsToFloat(U_Bone.data[int(temp_19)]);
            temp_23 = uintBitsToFloat(U_Bone.data[int(temp_22)]);
            temp_26 = uintBitsToFloat(U_Bone.data[int(temp_25)]);
            temp_28 = uintBitsToFloat(U_Bone.data[int(temp_27)]);
            temp_31 = uintBitsToFloat(U_Bone.data[int(temp_30)]);
            temp_34 = uintBitsToFloat(U_Bone.data[int(temp_33)]);
            temp_37 = uintBitsToFloat(U_Bone.data[int(temp_36)]);
            temp_39 = uintBitsToFloat(U_Bone.data[int(temp_38)]);
            temp_42 = uintBitsToFloat(U_Bone.data[int(temp_41)]);
            temp_45 = uintBitsToFloat(U_Bone.data[int(temp_44)]);
            temp_48 = uintBitsToFloat(U_Bone.data[int(temp_47)]);
            temp_49 = temp_17 * temp_1;
            temp_51 = temp_28 * temp_1;
            temp_52 = fma(temp_20, temp_2, temp_49);
            temp_53 = temp_39 * temp_1;
            temp_56 = fma(temp_31, temp_2, temp_51);
            temp_57 = fma(temp_23, temp_3, temp_52);
            temp_58 = fma(temp_42, temp_2, temp_53);
            temp_59 = fma(temp_34, temp_3, temp_56);
            temp_60 = fma(temp_26, temp_4, temp_57);
            temp_61 = fma(temp_45, temp_3, temp_58);
            temp_63 = fma(temp_37, temp_4, temp_59);
            temp_65 = fma(temp_48, temp_4, temp_61);
            temp_128 = temp_60 * U_Static.gmProj[i].x;
            temp_143 = fma(temp_63, U_Static.gmProj[i].y, temp_128);
            temp_152 = fma(temp_65, U_Static.gmProj[i].z, temp_143);
            temp_160 = temp_152 + U_Static.gmProj[i].w;
            temp_165 = 0.0 - U_Static.gCDep.y;
            temp_166 = temp_160 + temp_165;
            temp_177 = temp_166 * U_Static.gCDep.z;
        }
    "};
    Graph::parse_glsl(query).unwrap()
});

fn skin_attribute_clip_space_xyzw<'a>(nodes: &'a [Node], expr: &'a Expr) -> Option<&'a Expr> {
    let result = query_nodes(expr, nodes, &SKIN_ATTRIBUTE_CLIP_XYZW.nodes)
        .or_else(|| query_nodes(expr, nodes, &SKIN_ATTRIBUTE_CLIP_XYZW_Z.nodes))?;
    let index = result.get("i")?;
    match index {
        Expr::Int(0) => result.get("result_x").copied(),
        Expr::Int(1) => result.get("result_y").copied(),
        Expr::Int(2) => result.get("result_z").copied(),
        Expr::Int(3) => result.get("result_w").copied(),
        _ => None,
    }
}

static SKIN_ATTRIBUTE_BITANGENT_X: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
        void main() {
            temp_0 = nWgtIdx.x;
            temp_1 = floatBitsToInt(temp_0) & 65535;
            temp_2 = temp_1 * 48;
            temp_3 = vNormal.x;
            temp_4 = floatBitsToUint(temp_0) >> 16;
            temp_5 = int(temp_4) * 48;
            temp_6 = temp_5 << 16;
            temp_7 = temp_6 + temp_2;
            temp_8 = vTan.x;
            temp_10 = temp_7 + 32;
            temp_13 = temp_7 + 16;
            temp_14 = vNormal.y;
            temp_15 = vTan.y;
            temp_17 = vNormal.z;
            temp_18 = uint(temp_10) >> 2;
            temp_19 = uintBitsToFloat(U_Bone.data[int(temp_18)]);
            temp_20 = temp_10 + 4;
            temp_21 = uint(temp_20) >> 2;
            temp_22 = uintBitsToFloat(U_Bone.data[int(temp_21)]);
            temp_23 = temp_10 + 8;
            temp_24 = uint(temp_23) >> 2;
            temp_25 = uintBitsToFloat(U_Bone.data[int(temp_24)]);
            temp_29 = vTan.z;
            temp_41 = uint(temp_13) >> 2;
            temp_42 = uintBitsToFloat(U_Bone.data[int(temp_41)]);
            temp_43 = temp_13 + 4;
            temp_44 = uint(temp_43) >> 2;
            temp_45 = uintBitsToFloat(U_Bone.data[int(temp_44)]);
            temp_46 = temp_13 + 8;
            temp_47 = uint(temp_46) >> 2;
            temp_48 = uintBitsToFloat(U_Bone.data[int(temp_47)]);
            temp_54 = vTan.w;
            temp_62 = temp_19 * temp_3;
            temp_64 = temp_42 * temp_3;
            temp_65 = temp_19 * temp_8;
            temp_66 = temp_42 * temp_8;
            temp_68 = fma(temp_22, temp_14, temp_62);
            temp_77 = fma(temp_22, temp_15, temp_65);
            temp_80 = fma(temp_45, temp_14, temp_64);
            temp_81 = fma(temp_45, temp_15, temp_66);
            temp_82 = fma(temp_25, temp_29, temp_77);
            temp_83 = fma(temp_25, temp_17, temp_68);
            temp_87 = fma(temp_48, temp_29, temp_81);
            temp_88 = fma(temp_48, temp_17, temp_80);
            temp_92 = temp_83 * temp_87;
            temp_97 = 0.0 - temp_92;
            temp_98 = fma(temp_82, temp_88, temp_97);
            temp_101 = temp_98 * temp_54;
        }
    "};
    Graph::parse_glsl(query).unwrap()
});

static SKIN_ATTRIBUTE_BITANGENT_Y: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
        void main() {
            temp_0 = nWgtIdx.x;
            temp_1 = floatBitsToInt(temp_0) & 65535;
            temp_2 = temp_1 * 48;
            temp_3 = vNormal.x;
            temp_4 = floatBitsToUint(temp_0) >> 16;
            temp_5 = int(temp_4) * 48;
            temp_6 = temp_5 << 16;
            temp_7 = temp_6 + temp_2;
            temp_8 = vTan.x;
            temp_10 = temp_7 + 32;
            temp_14 = vNormal.y;
            temp_15 = vTan.y;
            temp_17 = vNormal.z;
            temp_18 = uint(temp_10) >> 2;
            temp_19 = uintBitsToFloat(U_Bone.data[int(temp_18)]);
            temp_20 = temp_10 + 4;
            temp_21 = uint(temp_20) >> 2;
            temp_22 = uintBitsToFloat(U_Bone.data[int(temp_21)]);
            temp_23 = temp_10 + 8;
            temp_24 = uint(temp_23) >> 2;
            temp_25 = uintBitsToFloat(U_Bone.data[int(temp_24)]);
            temp_29 = vTan.z;
            temp_30 = uint(temp_7) >> 2;
            temp_31 = uintBitsToFloat(U_Bone.data[int(temp_30)]);
            temp_32 = temp_7 + 4;
            temp_33 = uint(temp_32) >> 2;
            temp_34 = uintBitsToFloat(U_Bone.data[int(temp_33)]);
            temp_35 = temp_7 + 8;
            temp_36 = uint(temp_35) >> 2;
            temp_37 = uintBitsToFloat(U_Bone.data[int(temp_36)]);
            temp_54 = vTan.w;
            temp_59 = temp_31 * temp_3;
            temp_60 = temp_31 * temp_8;
            temp_62 = temp_19 * temp_3;
            temp_65 = temp_19 * temp_8;
            temp_68 = fma(temp_22, temp_14, temp_62);
            temp_69 = fma(temp_34, temp_14, temp_59);
            temp_71 = fma(temp_34, temp_15, temp_60);
            temp_73 = fma(temp_37, temp_17, temp_69);
            temp_74 = fma(temp_37, temp_29, temp_71);
            temp_77 = fma(temp_22, temp_15, temp_65);
            temp_82 = fma(temp_25, temp_29, temp_77);
            temp_83 = fma(temp_25, temp_17, temp_68);
            temp_94 = temp_73 * temp_82;
            temp_104 = 0.0 - temp_94;
            temp_105 = fma(temp_74, temp_83, temp_104);
            temp_109 = temp_105 * temp_54;
        }
    "};
    Graph::parse_glsl(query).unwrap()
});

static SKIN_ATTRIBUTE_BITANGENT_Z: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
        void main() {
            temp_0 = nWgtIdx.x;
            temp_1 = floatBitsToInt(temp_0) & 65535;
            temp_2 = temp_1 * 48;
            temp_3 = vNormal.x;
            temp_4 = floatBitsToUint(temp_0) >> 16;
            temp_5 = int(temp_4) * 48;
            temp_6 = temp_5 << 16;
            temp_7 = temp_6 + temp_2;
            temp_8 = vTan.x;
            temp_13 = temp_7 + 16;
            temp_14 = vNormal.y;
            temp_15 = vTan.y;
            temp_17 = vNormal.z;
            temp_29 = vTan.z;
            temp_30 = uint(temp_7) >> 2;
            temp_31 = uintBitsToFloat(U_Bone.data[int(temp_30)]);
            temp_32 = temp_7 + 4;
            temp_33 = uint(temp_32) >> 2;
            temp_34 = uintBitsToFloat(U_Bone.data[int(temp_33)]);
            temp_35 = temp_7 + 8;
            temp_36 = uint(temp_35) >> 2;
            temp_37 = uintBitsToFloat(U_Bone.data[int(temp_36)]);
            temp_41 = uint(temp_13) >> 2;
            temp_42 = uintBitsToFloat(U_Bone.data[int(temp_41)]);
            temp_43 = temp_13 + 4;
            temp_44 = uint(temp_43) >> 2;
            temp_45 = uintBitsToFloat(U_Bone.data[int(temp_44)]);
            temp_46 = temp_13 + 8;
            temp_47 = uint(temp_46) >> 2;
            temp_48 = uintBitsToFloat(U_Bone.data[int(temp_47)]);
            temp_54 = vTan.w;
            temp_59 = temp_31 * temp_3;
            temp_60 = temp_31 * temp_8;
            temp_64 = temp_42 * temp_3;
            temp_66 = temp_42 * temp_8;
            temp_69 = fma(temp_34, temp_14, temp_59);
            temp_71 = fma(temp_34, temp_15, temp_60);
            temp_73 = fma(temp_37, temp_17, temp_69);
            temp_74 = fma(temp_37, temp_29, temp_71);
            temp_80 = fma(temp_45, temp_14, temp_64);
            temp_81 = fma(temp_45, temp_15, temp_66);
            temp_87 = fma(temp_48, temp_29, temp_81);
            temp_88 = fma(temp_48, temp_17, temp_80);
            temp_91 = temp_74 * temp_88;
            temp_95 = 0.0 - temp_91;
            temp_96 = fma(temp_73, temp_87, temp_95);
            temp_116 = temp_96 * temp_54;
        }
    "};
    Graph::parse_glsl(query).unwrap()
});

fn skin_attribute_bitangent<'a>(nodes: &'a [Node], expr: &'a Expr) -> Option<Expr> {
    let channel = query_nodes(expr, nodes, &SKIN_ATTRIBUTE_BITANGENT_X.nodes)
        .map(|_| 'x')
        .or_else(|| query_nodes(expr, nodes, &SKIN_ATTRIBUTE_BITANGENT_Y.nodes).map(|_| 'y'))
        .or_else(|| query_nodes(expr, nodes, &SKIN_ATTRIBUTE_BITANGENT_Z.nodes).map(|_| 'z'))?;
    Some(Expr::Global {
        name: "vBitan".into(),
        channel: Some(channel),
    })
}

static U_MDL_ATTRIBUTE_XYZW_X: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
        void main() {
            temp_0 = result_x;
            temp_1 = result_y;
            temp_2 = result_z;
            temp_3 = result_w;
            temp_24 = temp_0 * U_Mdl.gmWorldView[0].x;
            temp_28 = fma(temp_1, U_Mdl.gmWorldView[0].y, temp_24);
            temp_34 = fma(temp_2, U_Mdl.gmWorldView[0].z, temp_28);
            temp_40 = fma(temp_3, U_Mdl.gmWorldView[0].w, temp_34);
        }
    "};
    Graph::parse_glsl(query).unwrap()
});

static U_MDL_ATTRIBUTE_XYZW_Y: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
        void main() {
            temp_0 = result_x;
            temp_1 = result_y;
            temp_2 = result_z;
            temp_3 = result_w;
            temp_23 = temp_0 * U_Mdl.gmWorldView[1].x;
            temp_27 = fma(temp_1, U_Mdl.gmWorldView[1].y, temp_23);
            temp_33 = fma(temp_2, U_Mdl.gmWorldView[1].z, temp_27);
            temp_39 = fma(temp_3, U_Mdl.gmWorldView[1].w, temp_33);
        }
    "};
    Graph::parse_glsl(query).unwrap()
});

static U_MDL_ATTRIBUTE_XYZW_Z: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
        void main() {
            temp_0 = result_x;
            temp_1 = result_y;
            temp_2 = result_z;
            temp_3 = result_w;
            temp_22 = temp_0 * U_Mdl.gmWorldView[2].x;
            temp_26 = fma(temp_1, U_Mdl.gmWorldView[2].y, temp_22);
            temp_31 = fma(temp_2, U_Mdl.gmWorldView[2].z, temp_26);
            temp_37 = fma(temp_3, U_Mdl.gmWorldView[2].w, temp_31);
        }
    "};
    Graph::parse_glsl(query).unwrap()
});

fn u_mdl_view_attribute_xyzw<'a>(nodes: &'a [Node], expr: &'a Expr) -> Option<&'a Expr> {
    query_nodes(expr, nodes, &U_MDL_ATTRIBUTE_XYZW_X.nodes)
        .and_then(|r| r.get("result_x").copied())
        .or_else(|| {
            query_nodes(expr, nodes, &U_MDL_ATTRIBUTE_XYZW_Y.nodes)
                .and_then(|r| r.get("result_y").copied())
        })
        .or_else(|| {
            query_nodes(expr, nodes, &U_MDL_ATTRIBUTE_XYZW_Z.nodes)
                .and_then(|r| r.get("result_z").copied())
        })
}

static U_MDL_VIEW_BITANGENT_X: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
        void main() {
            temp_10 = vNormal.x;
            temp_12 = vNormal.y;
            temp_14 = vNormal.z;
            temp_18 = vTan.x;
            temp_19 = vTan.y;
            temp_21 = vTan.z;
            temp_23 = vTan.w;
            temp_49 = temp_10 * U_Mdl.gmWorldView[2].x;
            temp_55 = temp_10 * U_Mdl.gmWorldView[1].x;
            temp_56 = temp_18 * U_Mdl.gmWorldView[2].x;
            temp_57 = temp_18 * U_Mdl.gmWorldView[1].x;
            temp_58 = fma(temp_12, U_Mdl.gmWorldView[2].y, temp_49);
            temp_61 = fma(temp_19, U_Mdl.gmWorldView[2].y, temp_56);
            temp_62 = fma(temp_19, U_Mdl.gmWorldView[1].y, temp_57);
            temp_64 = fma(temp_12, U_Mdl.gmWorldView[1].y, temp_55);
            temp_66 = fma(temp_14, U_Mdl.gmWorldView[2].z, temp_58);
            temp_67 = fma(temp_21, U_Mdl.gmWorldView[2].z, temp_61);
            temp_68 = fma(temp_21, U_Mdl.gmWorldView[1].z, temp_62);
            temp_70 = fma(temp_14, U_Mdl.gmWorldView[1].z, temp_64);
            temp_74 = temp_66 * temp_68;
            temp_77 = 0.0 - temp_74;
            temp_78 = fma(temp_67, temp_70, temp_77);
            temp_84 = temp_78 * temp_23;
        }
    "};
    Graph::parse_glsl(query).unwrap()
});

static U_MDL_VIEW_BITANGENT_Y: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
        void main() {
            temp_10 = vNormal.x;
            temp_12 = vNormal.y;
            temp_14 = vNormal.z;
            temp_18 = vTan.x;
            temp_19 = vTan.y;
            temp_21 = vTan.z;
            temp_23 = vTan.w;
            temp_49 = temp_10 * U_Mdl.gmWorldView[2].x;
            temp_56 = temp_18 * U_Mdl.gmWorldView[2].x;
            temp_58 = fma(temp_12, U_Mdl.gmWorldView[2].y, temp_49);
            temp_59 = temp_10 * U_Mdl.gmWorldView[0].x;
            temp_60 = temp_18 * U_Mdl.gmWorldView[0].x;
            temp_61 = fma(temp_19, U_Mdl.gmWorldView[2].y, temp_56);
            temp_63 = fma(temp_12, U_Mdl.gmWorldView[0].y, temp_59);
            temp_65 = fma(temp_19, U_Mdl.gmWorldView[0].y, temp_60);
            temp_66 = fma(temp_14, U_Mdl.gmWorldView[2].z, temp_58);
            temp_67 = fma(temp_21, U_Mdl.gmWorldView[2].z, temp_61);
            temp_69 = fma(temp_14, U_Mdl.gmWorldView[0].z, temp_63);
            temp_71 = fma(temp_21, U_Mdl.gmWorldView[0].z, temp_65);
            temp_75 = temp_67 * temp_69;
            temp_79 = 0.0 - temp_75;
            temp_80 = fma(temp_66, temp_71, temp_79);
            temp_85 = temp_80 * temp_23;
        }
    "};
    Graph::parse_glsl(query).unwrap()
});

static U_MDL_VIEW_BITANGENT_Z: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
        void main() {
            temp_10 = vNormal.x;
            temp_12 = vNormal.y;
            temp_14 = vNormal.z;
            temp_18 = vTan.x;
            temp_19 = vTan.y;
            temp_21 = vTan.z;
            temp_23 = vTan.w;
            temp_55 = temp_10 * U_Mdl.gmWorldView[1].x;
            temp_57 = temp_18 * U_Mdl.gmWorldView[1].x;
            temp_59 = temp_10 * U_Mdl.gmWorldView[0].x;
            temp_60 = temp_18 * U_Mdl.gmWorldView[0].x;
            temp_62 = fma(temp_19, U_Mdl.gmWorldView[1].y, temp_57);
            temp_63 = fma(temp_12, U_Mdl.gmWorldView[0].y, temp_59);
            temp_64 = fma(temp_12, U_Mdl.gmWorldView[1].y, temp_55);
            temp_65 = fma(temp_19, U_Mdl.gmWorldView[0].y, temp_60);
            temp_68 = fma(temp_21, U_Mdl.gmWorldView[1].z, temp_62);
            temp_69 = fma(temp_14, U_Mdl.gmWorldView[0].z, temp_63);
            temp_70 = fma(temp_14, U_Mdl.gmWorldView[1].z, temp_64);
            temp_71 = fma(temp_21, U_Mdl.gmWorldView[0].z, temp_65);
            temp_76 = temp_70 * temp_71;
            temp_81 = 0.0 - temp_76;
            temp_82 = fma(temp_69, temp_68, temp_81);
            temp_86 = temp_82 * temp_23;
        }
    "};
    Graph::parse_glsl(query).unwrap()
});

fn u_mdl_view_bitangent_xyz<'a>(nodes: &'a [Node], expr: &'a Expr) -> Option<Expr> {
    let channel = query_nodes(expr, nodes, &U_MDL_VIEW_BITANGENT_X.nodes)
        .map(|_| 'x')
        .or_else(|| query_nodes(expr, nodes, &U_MDL_VIEW_BITANGENT_Y.nodes).map(|_| 'y'))
        .or_else(|| query_nodes(expr, nodes, &U_MDL_VIEW_BITANGENT_Z.nodes).map(|_| 'z'))?;
    Some(Expr::Global {
        name: "vBitan".into(),
        channel: Some(channel),
    })
}

static U_MDL_CLIP_ATTRIBUTE_XYZW_X: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
        void main() {
            temp_0 = result_x;
            temp_1 = result_y;
            temp_2 = result_z;
            temp_3 = result_w;
            temp_8 = temp_0 * U_Mdl.gmWVP[0].x;
            temp_16 = fma(temp_1, U_Mdl.gmWVP[0].y, temp_8);
            temp_29 = fma(temp_2, U_Mdl.gmWVP[0].z, temp_16);
            temp_36 = fma(temp_3, U_Mdl.gmWVP[0].w, temp_29);
        }
    "};
    Graph::parse_glsl(query).unwrap()
});

static U_MDL_CLIP_ATTRIBUTE_XYZW_Y: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
        void main() {
            temp_0 = result_x;
            temp_1 = result_y;
            temp_2 = result_z;
            temp_3 = result_w;
            temp_12 = temp_0 * U_Mdl.gmWVP[1].x;
            temp_21 = fma(temp_1, U_Mdl.gmWVP[1].y, temp_12);
            temp_32 = fma(temp_2, U_Mdl.gmWVP[1].z, temp_21);
            temp_38 = fma(temp_3, U_Mdl.gmWVP[1].w, temp_32);
        }
    "};
    Graph::parse_glsl(query).unwrap()
});

static U_MDL_CLIP_ATTRIBUTE_XYZW_Z: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
        void main() {
            temp_0 = result_x;
            temp_1 = result_y;
            temp_2 = result_z;
            temp_3 = result_w;
            temp_20 = temp_0 * U_Mdl.gmWVP[2].x;
            temp_25 = fma(temp_1, U_Mdl.gmWVP[2].y, temp_20);
            temp_30 = fma(temp_2, U_Mdl.gmWVP[2].z, temp_25);
            temp_35 = fma(temp_3, U_Mdl.gmWVP[2].w, temp_30);
            temp_42 = 0.0 - U_Static.gCDep.y;
            temp_43 = temp_35 + temp_42;
            temp_49 = temp_43 * U_Static.gCDep.z;
        }
    "};
    Graph::parse_glsl(query).unwrap()
});

fn u_mdl_clip_attribute_xyzw<'a>(nodes: &'a [Node], expr: &'a Expr) -> Option<&'a Expr> {
    query_nodes(expr, nodes, &U_MDL_CLIP_ATTRIBUTE_XYZW_X.nodes)
        .and_then(|r| r.get("result_x").copied())
        .or_else(|| {
            query_nodes(expr, nodes, &U_MDL_CLIP_ATTRIBUTE_XYZW_Y.nodes)
                .and_then(|r| r.get("result_y").copied())
        })
        .or_else(|| {
            query_nodes(expr, nodes, &U_MDL_CLIP_ATTRIBUTE_XYZW_Z.nodes)
                .and_then(|r| r.get("result_z").copied())
        })
}

static U_MDL_ATTRIBUTE_XYZ_X: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
        void main() {
            temp_0 = result_x;
            temp_1 = result_y;
            temp_2 = result_z;
            temp_24 = temp_0 * U_Mdl.gmWorldView[0].x;
            temp_28 = fma(temp_1, U_Mdl.gmWorldView[0].y, temp_24);
            temp_34 = fma(temp_2, U_Mdl.gmWorldView[0].z, temp_28);
        }
    "};
    Graph::parse_glsl(query).unwrap()
});

static U_MDL_ATTRIBUTE_XYZ_Y: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
        void main() {
            temp_0 = result_x;
            temp_1 = result_y;
            temp_2 = result_z;
            temp_23 = temp_0 * U_Mdl.gmWorldView[1].x;
            temp_27 = fma(temp_1, U_Mdl.gmWorldView[1].y, temp_23);
            temp_33 = fma(temp_2, U_Mdl.gmWorldView[1].z, temp_27);
        }
    "};
    Graph::parse_glsl(query).unwrap()
});

static U_MDL_ATTRIBUTE_XYZ_Z: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
        void main() {
            temp_0 = result_x;
            temp_1 = result_y;
            temp_2 = result_z;
            temp_22 = temp_0 * U_Mdl.gmWorldView[2].x;
            temp_26 = fma(temp_1, U_Mdl.gmWorldView[2].y, temp_22);
            temp_31 = fma(temp_2, U_Mdl.gmWorldView[2].z, temp_26);
        }
    "};
    Graph::parse_glsl(query).unwrap()
});

fn u_mdl_attribute_xyz<'a>(nodes: &'a [Node], expr: &'a Expr) -> Option<&'a Expr> {
    query_nodes(expr, nodes, &U_MDL_ATTRIBUTE_XYZ_X.nodes)
        .and_then(|r| r.get("result_x").copied())
        .or_else(|| {
            query_nodes(expr, nodes, &U_MDL_ATTRIBUTE_XYZ_Y.nodes)
                .and_then(|r| r.get("result_y").copied())
        })
        .or_else(|| {
            query_nodes(expr, nodes, &U_MDL_ATTRIBUTE_XYZ_Z.nodes)
                .and_then(|r| r.get("result_z").copied())
        })
}

static TEX_MATRIX: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
        void main() {
            result = u * param_x;
            result = fma(v, param_y, result);
            result = fma(0.0, param_z, result);
            result = result + param_w;
        }
    "};
    Graph::parse_glsl(query).unwrap()
});

fn tex_matrix<'a>(nodes: &'a [Node], expr: &'a Expr) -> Option<(Operation, Vec<&'a Expr>)> {
    // Detect matrix multiplication for the mat4x2 "gTexMat * vec4(u, v, 0.0, 1.0)".
    // U and V have the same pattern but use a different row of the matrix.
    let result = query_nodes(expr, nodes, &TEX_MATRIX.nodes)?;
    let u = result.get("u")?;
    let v = result.get("v")?;
    let x = result.get("param_x")?;
    let y = result.get("param_y")?;
    let z = result.get("param_z")?;
    let w = result.get("param_w")?;

    Some((Operation::TexMatrix, vec![u, v, x, y, z, w]))
}

static TEX_PARALLAX: LazyLock<Graph> = LazyLock::new(|| {
    // uv = ratio * 0.7 * (nrm.x * tan.xy - norm.y * bitan.xy) + vTex0.xy
    let query = indoc! {"
        void main() {
            nrm_result = fma(temp1, 0.7, temp2);
            result = fma(nrm_result, ratio, coord);
        }
    "};
    Graph::parse_glsl(query).unwrap()
});

static TEX_PARALLAX2: LazyLock<Graph> = LazyLock::new(|| {
    // uv = ratio * 0.7 * (nrm.x * tan.xy - norm.y * bitan.xy) + vTex0.xy
    let query = indoc! {"
        void main() {
            coord = coord;
            mask = mask;
            nrm_result = fma(temp1, 0.7, temp2);
            result = fma(ratio, nrm_result, coord);
            // Generated for some shaders.
            result = abs(result);
            result = result + -0.0;
        }
    "};
    Graph::parse_glsl(query).unwrap()
});

fn tex_parallax<'a>(nodes: &'a [Node], expr: &'a Expr) -> Option<(Operation, Vec<&'a Expr>)> {
    let expr = assign_x_recursive(nodes, expr);

    // Some eye shaders use some form of parallax mapping.
    let result = query_nodes(expr, nodes, &TEX_PARALLAX.nodes)
        .or_else(|| query_nodes(expr, nodes, &TEX_PARALLAX2.nodes))?;

    let ratio = result.get("ratio")?;
    let coord = result.get("coord")?;

    // TODO: Detect x vs y
    Some((Operation::TexParallaxX, vec![coord, ratio]))
}

static TEX_PARALLAX3_X: LazyLock<Graph> = LazyLock::new(|| {
    // u = ratio * (2 * normal.y * bitangent.x - 2 * normal.x * tangent.x) + vTex0.x
    let query = indoc! {"
        void main() {
            temp_30 = vNormal.x;
            temp_31 = vBitan.x;
            temp_32 = vTan.x;
            temp_33 = vNormal.y;
            temp_34 = vBitan.y;
            temp_35 = vTan.y;
            temp_36 = vNormal.z;
            temp_37 = vBitan.z;
            temp_38 = vTan.z;
            temp_39 = temp_30 * temp_30;
            temp_40 = temp_31 * temp_31;
            temp_41 = temp_32 * temp_32;
            temp_42 = fma(temp_33, temp_33, temp_39);
            temp_43 = fma(temp_34, temp_34, temp_40);
            temp_44 = fma(temp_35, temp_35, temp_41);
            temp_45 = fma(temp_36, temp_36, temp_42);
            temp_46 = fma(temp_37, temp_37, temp_43);
            temp_47 = inversesqrt(temp_45);
            temp_48 = fma(temp_38, temp_38, temp_44);
            temp_49 = inversesqrt(temp_46);
            temp_50 = inversesqrt(temp_48);
            temp_51 = temp_30 * temp_47;
            temp_52 = temp_33 * temp_47;
            temp_53 = temp_31 * temp_49;
            temp_55 = temp_32 * temp_50;
            temp_71 = temp_51 * 2.0;
            temp_77 = temp_52 * -2.0;
            temp_79 = temp_55 * temp_71;
            temp_84 = fma(temp_53, temp_77, temp_79);
            temp_89 = temp_84 * ratio;
            temp_92 = fma(temp_89, 2.0, coord);
        }
    "};
    Graph::parse_glsl(query).unwrap()
});

static TEX_PARALLAX3_Y: LazyLock<Graph> = LazyLock::new(|| {
    // v = ratio * (2 * normal.y * bitangent.y - 2 * normal.x * tangent.y) + vTex0.x
    let query = indoc! {"
        void main() {
            temp_30 = vNormal.x;
            temp_31 = vBitan.x;
            temp_32 = vTan.x;
            temp_33 = vNormal.y;
            temp_34 = vBitan.y;
            temp_35 = vTan.y;
            temp_36 = vNormal.z;
            temp_37 = vBitan.z;
            temp_38 = vTan.z;
            temp_39 = temp_30 * temp_30;
            temp_40 = temp_31 * temp_31;
            temp_41 = temp_32 * temp_32;
            temp_42 = fma(temp_33, temp_33, temp_39);
            temp_43 = fma(temp_34, temp_34, temp_40);
            temp_44 = fma(temp_35, temp_35, temp_41);
            temp_45 = fma(temp_36, temp_36, temp_42);
            temp_46 = fma(temp_37, temp_37, temp_43);
            temp_47 = inversesqrt(temp_45);
            temp_48 = fma(temp_38, temp_38, temp_44);
            temp_49 = inversesqrt(temp_46);
            temp_50 = inversesqrt(temp_48);
            temp_51 = temp_30 * temp_47;
            temp_52 = temp_33 * temp_47;
            temp_65 = temp_34 * temp_49;
            temp_66 = temp_35 * temp_50;
            temp_71 = temp_51 * 2.0;
            temp_77 = temp_52 * -2.0;
            temp_82 = temp_66 * temp_71;
            temp_87 = fma(temp_65, temp_77, temp_82);
            temp_91 = temp_87 * ratio;
            temp_100 = fma(temp_91, 2.0, coord);
        }
    "};
    Graph::parse_glsl(query).unwrap()
});

fn tex_parallax2<'a>(nodes: &'a [Node], expr: &'a Expr) -> Option<(Operation, Vec<&'a Expr>)> {
    // Some eye shaders use some form of parallax mapping.
    let result = query_nodes(expr, nodes, &TEX_PARALLAX3_X.nodes)
        .or_else(|| query_nodes(expr, nodes, &TEX_PARALLAX3_Y.nodes))?;

    let ratio = result.get("ratio")?;
    let coord = result.get("coord")?;

    // TODO: New operation for this since the math is different.
    Some((Operation::TexParallaxX, vec![coord, ratio]))
}

static REFLECT_X: LazyLock<Graph> = LazyLock::new(|| {
    // reflect(I, N) = I - 2.0 * dot(N, I) * N
    let query = indoc! {"
        void main() {
            dot_n_i = n_x * i_x;
            dot_n_i = fma(n_y, i_y, dot_n_i);
            dot_n_i = fma(n_z, i_z, dot_n_i);
            temp_127 = n_x * dot_n_i;
            temp_129 = fma(temp_127, -2.0, i_x);
        }
    "};
    Graph::parse_glsl(query).unwrap()
});

static REFLECT_Y: LazyLock<Graph> = LazyLock::new(|| {
    // reflect(I, N) = I - 2.0 * dot(N, I) * N
    let query = indoc! {"
        void main() {
            dot_n_i = n_x * i_x;
            dot_n_i = fma(n_y, i_y, dot_n_i);
            dot_n_i = fma(n_z, i_z, dot_n_i);
            temp_127 = n_y * dot_n_i;
            temp_129 = fma(temp_127, -2.0, i_y);
        }
    "};
    Graph::parse_glsl(query).unwrap()
});

static REFLECT_Z: LazyLock<Graph> = LazyLock::new(|| {
    // reflect(I, N) = I - 2.0 * dot(N, I) * N
    let query = indoc! {"
        void main() {
            dot_n_i = n_x * i_x;
            dot_n_i = fma(n_y, i_y, dot_n_i);
            dot_n_i = fma(n_z, i_z, dot_n_i);
            temp_127 = n_z * dot_n_i;
            temp_129 = fma(temp_127, -2.0, i_z);
        }
    "};
    Graph::parse_glsl(query).unwrap()
});

fn op_reflect<'a>(nodes: &'a [Node], expr: &'a Expr) -> Option<(Operation, Vec<&'a Expr>)> {
    let (op, result) = query_nodes(expr, nodes, &REFLECT_X.nodes)
        .map(|r| (Operation::ReflectX, r))
        .or_else(|| query_nodes(expr, nodes, &REFLECT_Y.nodes).map(|r| (Operation::ReflectY, r)))
        .or_else(|| query_nodes(expr, nodes, &REFLECT_Z.nodes).map(|r| (Operation::ReflectZ, r)))?;

    let n_x = result.get("n_x")?;
    let n_y = result.get("n_y")?;
    let n_z = result.get("n_z")?;

    let i_x = result.get("i_x")?;
    let i_y = result.get("i_y")?;
    let i_z = result.get("i_z")?;

    Some((op, vec![i_x, i_y, i_z, n_x, n_y, n_z]))
}

pub fn create_shader_database(input: &str) -> ShaderDatabase {
    // Collect unique programs.
    let mut programs = BTreeMap::new();

    for path in globwalk::GlobWalkerBuilder::from_patterns(input, &["*.wishp"])
        .build()
        .unwrap()
        .filter_map(|e| e.map(|e| e.path().to_owned()).ok())
    {
        add_programs(&mut programs, path);
    }

    // Process programs in parallel since this is CPU heavy.
    let programs = programs
        .into_par_iter()
        .map(|(hash, (vert, frag))| {
            let vertex = vert.and_then(|s| {
                let source = shader_source_no_extensions(&s);
                match TranslationUnit::parse(source) {
                    Ok(vertex) => Some(vertex),
                    Err(e) => {
                        error!("Error parsing shader: {e}");
                        None
                    }
                }
            });

            let shader_program = frag
                .map(|s| {
                    let source = shader_source_no_extensions(&s);
                    match TranslationUnit::parse(source) {
                        Ok(fragment) => shader_from_glsl(vertex.as_ref(), &fragment),
                        Err(e) => {
                            error!("Error parsing shader: {e}");
                            ShaderProgram::default()
                        }
                    }
                })
                .unwrap_or_default();

            (hash, shader_program)
        })
        .collect();

    ShaderDatabase::from_programs(programs)
}

fn add_programs(
    programs: &mut BTreeMap<ProgramHash, (Option<String>, Option<String>)>,
    spch_path: std::path::PathBuf,
) {
    if let Ok(spch) = Spch::from_file(&spch_path) {
        for (i, slct_offset) in spch.slct_offsets.iter().enumerate() {
            let slct = slct_offset.read_slct(&spch.slct_section).unwrap();

            // Only check the first shader for now.
            // TODO: What do additional nvsd shader entries do?
            if let Some((p, vert, frag)) = spch.program_data_vertex_fragment_binaries(&slct).first()
            {
                let hash = ProgramHash::from_spch_program(p, vert, frag);

                programs.entry(hash).or_insert_with(|| {
                    let path = spch_path
                        .with_file_name(nvsd_glsl_name(&spch, i, 0))
                        .with_extension("frag");

                    // TODO: Should the vertex shader be mandatory?
                    let vertex_source = std::fs::read_to_string(path.with_extension("vert")).ok();
                    let frag_source = std::fs::read_to_string(path).ok();
                    (vertex_source, frag_source)
                });
            }
        }
    }
}

pub fn create_shader_database_legacy(input: &str) -> ShaderDatabase {
    let mut programs = BTreeMap::new();

    for path in globwalk::GlobWalkerBuilder::from_patterns(input, &["*.cashd"])
        .build()
        .unwrap()
        .filter_map(|e| e.map(|e| e.path().to_owned()).ok())
    {
        add_programs_legacy(&mut programs, path);
    }

    // Process programs in parallel since this is CPU heavy.
    let programs = programs
        .into_par_iter()
        .map(|(hash, shader)| {
            let vertex = match TranslationUnit::parse(&shader.vertex_source) {
                Ok(vertex) => Some(vertex),
                Err(e) => {
                    error!("Error parsing shader: {e}");
                    None
                }
            };

            let fragment = match TranslationUnit::parse(&shader.fragment_source) {
                Ok(vertex) => Some(vertex),
                Err(e) => {
                    error!("Error parsing shader: {e}");
                    None
                }
            };

            let shader_program = fragment
                .map(|fragment| shader_from_glsl(vertex.as_ref(), &fragment))
                .unwrap_or_default();

            (hash, shader_program)
        })
        .collect();

    ShaderDatabase::from_programs(programs)
}

struct LegacyProgram {
    vertex_source: String,
    fragment_source: String,
}

fn add_programs_legacy(
    programs: &mut BTreeMap<ProgramHash, LegacyProgram>,
    path: std::path::PathBuf,
) {
    // Avoid processing the same program more than once.
    let mths = Mths::from_file(&path).unwrap();
    let hash = ProgramHash::from_mths(&mths);
    programs.entry(hash).or_insert_with(|| {
        // TODO: Should both shaders be mandatory?
        let vertex_source = std::fs::read_to_string(path.with_extension("vert")).unwrap();
        let fragment_source = std::fs::read_to_string(path.with_extension("frag")).unwrap();
        LegacyProgram {
            vertex_source,
            fragment_source,
        }
    });
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

fn remap_expr_node_indices(input: &mut Expr, start_index: usize) {
    // Recursively shift node indices to match their new position.
    match input {
        Expr::Node { node_index, .. } => *node_index += start_index,
        Expr::Float(_) => (),
        Expr::Int(_) => (),
        Expr::Uint(_) => (),
        Expr::Bool(_) => (),
        Expr::Parameter { index, .. } => {
            if let Some(index) = index {
                remap_expr_node_indices(index, start_index);
            }
        }
        Expr::Global { .. } => (),
        Expr::Unary(_, a) => {
            remap_expr_node_indices(a, start_index);
        }
        Expr::Binary(_, lh, rh) => {
            remap_expr_node_indices(lh, start_index);
            remap_expr_node_indices(rh, start_index);
        }
        Expr::Ternary(a, b, c) => {
            remap_expr_node_indices(a, start_index);
            remap_expr_node_indices(b, start_index);
            remap_expr_node_indices(c, start_index);
        }
        Expr::Func { args, .. } => {
            for arg in args {
                remap_expr_node_indices(arg, start_index);
            }
        }
    }
}

fn merge_vertex_fragment(
    vert: Graph,
    vert_attributes: &Attributes,
    frag: Graph,
    frag_attributes: &Attributes,
) -> Graph {
    let mut graph = vert;

    // Make sure fragment nodes only refer to other fragment nodes.
    let start = graph.nodes.len();
    for n in frag.nodes {
        let mut new_node = n;
        remap_expr_node_indices(&mut new_node.input, start);

        if let Expr::Global { name, channel } = &mut new_node.input {
            // Convert a fragment input like "in_attr4" to its vertex output like "out_attr4".
            if let Some(fragment_location) =
                frag_attributes.input_locations.get_by_left(name.as_str())
            {
                if let Some(vertex_output_name) = vert_attributes
                    .output_locations
                    .get_by_right(fragment_location)
                {
                    // This will search vertex nodes first even if a fragment output has the same name.
                    if let Some(node_index) = graph.nodes.iter().position(|n| {
                        n.output.name == vertex_output_name && n.output.channel == *channel
                    }) {
                        // Link fragment inputs to vertex outputs.
                        new_node.input = Expr::Node {
                            node_index,
                            channel: *channel,
                        };
                    }
                }
            }
        }

        graph.nodes.push(new_node);
    }

    graph
}

pub fn shader_str(s: &ShaderProgram) -> String {
    // Use a condensed representation similar to GLSL for nicer diffs.
    let mut output = String::new();
    for (k, v) in &s.output_dependencies {
        writeln!(&mut output, "{k:?}: {:?}", expr_str(s, *v)).unwrap();
    }
    writeln!(
        &mut output,
        "outline_width: {}",
        s.outline_width
            .as_ref()
            .map(|d| d.to_string())
            .unwrap_or("None".to_string())
    )
    .unwrap();
    match s.normal_intensity {
        Some(i) => {
            writeln!(&mut output, "normal_intensity: {:?}", expr_str(s, i)).unwrap();
        }
        None => writeln!(&mut output, "normal_intensity: None").unwrap(),
    }

    output
}

fn expr_str(s: &ShaderProgram, v: usize) -> String {
    // Substitute all args to produce a single line of condensed output.
    match &s.exprs[v] {
        OutputExpr::Value(Dependency::Texture(t)) => {
            let args: Vec<_> = t.texcoords.iter().map(|a| expr_str(s, *a)).collect();
            format!(
                "Texture({}, {}){}",
                t.name,
                args.join(", "),
                t.channel.map(|c| format!(".{c}")).unwrap_or_default()
            )
        }
        OutputExpr::Func { op, args } => {
            let args: Vec<_> = args.iter().map(|a| expr_str(s, *a)).collect();
            format!("{op}({})", args.join(", "))
        }
        OutputExpr::Value(v) => v.to_string(),
    }
}

pub fn shader_graphviz(shader: &ShaderProgram) -> String {
    let mut text = String::new();
    writeln!(&mut text, "digraph {{").unwrap();
    for (i, expr) in shader.exprs.iter().enumerate() {
        let label = match expr {
            OutputExpr::Func { op, .. } => op.to_string(),
            OutputExpr::Value(Dependency::Texture(t)) => {
                format!(
                    "{}{}",
                    &t.name,
                    t.channel.map(|c| format!(".{c}")).unwrap_or_default()
                )
            }
            OutputExpr::Value(d) => d.to_string(),
        };
        writeln!(&mut text, "    {i} [label={:?}]", label).unwrap();
    }
    for (i, expr) in shader.exprs.iter().enumerate() {
        match expr {
            OutputExpr::Func { args, .. } => {
                for arg in args {
                    writeln!(&mut text, "    {arg} -> {i}").unwrap();
                }
            }
            OutputExpr::Value(Dependency::Texture(t)) => {
                for arg in &t.texcoords {
                    writeln!(&mut text, "    {arg} -> {i}").unwrap();
                }
            }
            _ => (),
        }
    }
    for (name, i) in &shader.output_dependencies {
        writeln!(&mut text, "    {i} -> {name:?}").unwrap();
    }
    writeln!(&mut text, "}}").unwrap();
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    use indoc::indoc;
    use insta::assert_snapshot;
    use pretty_assertions::assert_eq;

    macro_rules! assert_shader_snapshot {
        ($folder:expr, $name: expr, $index:expr) => {
            let vert_glsl =
                include_str!(concat!("data/", $folder, "/", $name, ".", $index, ".vert"));
            let frag_glsl =
                include_str!(concat!("data/", $folder, "/", $name, ".", $index, ".frag"));
            let vertex = TranslationUnit::parse(vert_glsl).unwrap();
            let fragment = TranslationUnit::parse(frag_glsl).unwrap();

            let shader = shader_from_glsl(Some(&vertex), &fragment);

            let mut settings = insta::Settings::new();
            settings.set_prepend_module_to_snapshot(false);
            settings.set_omit_expression(true);
            settings.bind(|| {
                // Use names like "xc2 bl000101.22"
                assert_snapshot!(
                    concat!($folder, " ", $name, ".", $index),
                    shader_str(&shader)
                );
            });
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
        assert_shader_snapshot!("xc2", "bl000101", "22");
    }

    #[test]
    fn shader_from_glsl_pyra_hair() {
        // xeno2/model/bl/bl000101, "_ho_hair_new", shd0008
        // Check that the color texture is multiplied by vertex color.
        assert_shader_snapshot!("xc2", "bl000101", "8");
    }

    #[test]
    fn shader_from_glsl_mio_skirt() {
        // xeno3/chr/ch/ch11021013, "body_skert2", shd0028
        // The pcmdo calcGeometricSpecularAA function compiles to the expression
        // glossiness = 1.0 - sqrt(clamp((1.0 - glossiness)^2 + kernelRoughness2 0.0, 1.0))
        // Consuming applications only care about the glossiness input.
        // This also avoids considering normal maps as a dependency.
        assert_shader_snapshot!("xc3", "ch11021013", "28");
    }

    #[test]
    fn shader_from_glsl_mio_metal() {
        // xeno3/chr/ch/ch11021013, "tlent_mio_metal1", shd0031
        // Test multiple calls to getPixelCalcAddNormal.
        assert_shader_snapshot!("xc3", "ch11021013", "31");
    }

    #[test]
    fn shader_from_glsl_mio_legs() {
        // xeno3/chr/ch/ch11021013, "body_stking1", shd0016
        // Test that color layers use the appropriate fresnel operation.
        assert_shader_snapshot!("xc3", "ch11021013", "16");
    }

    #[test]
    fn shader_from_glsl_mio_eyes() {
        // xeno3/chr/ch/ch01021011, "eye4", shd0063
        // Detect parallax mapping for texture coordinates.
        assert_shader_snapshot!("xc3", "ch01021011", "63");
    }

    #[test]
    fn shader_from_glsl_mio_ribbon() {
        // xeno3/chr/ch/ch01027000, "phong4", shd0044
        // Detect handling of gMatCol.
        assert_shader_snapshot!("xc3", "ch01027000", "44");
    }

    #[test]
    fn shader_from_glsl_wild_ride_body() {
        // xeno3/chr/ch/ch02010110, "body_m", shd0028
        // Some shaders use a simple mix() for normal blending.
        assert_shader_snapshot!("xc3", "ch02010110", "28");
    }

    #[test]
    fn shader_from_glsl_sena_body() {
        // xeno3/chr/ch/ch11061013, "bodydenim_toon", shd0009
        // Some shaders use multiple color blending modes.
        assert_shader_snapshot!("xc3", "ch11061013", "9");
    }

    #[test]
    fn shader_from_glsl_platform() {
        // xeno1/model/obj/oj110006, "ma14toride03", shd0003
        // Test detecting multiple normal layers with different blend modes.
        assert_shader_snapshot!("xc1", "oj110006", "3");
    }

    #[test]
    fn shader_from_glsl_xc1_normal_w_intensity() {
        // xeno1/model/pc/pc078702, "pc070702_body", shd0001
        // Test detecting xyz normal maps with vNormal.w intensity.
        assert_shader_snapshot!("xc1", "pc078702", "1");
    }

    #[test]
    fn shader_from_glsl_haze_body() {
        // xeno2/model/np/np001101, "body", shd0013
        // Test multiple normal layers with texture masks.
        assert_shader_snapshot!("xc2", "np001101", "13");
    }

    #[test]
    fn shader_from_glsl_pneuma_chest() {
        // xeno2/model/bl/bl000301, "tights_TS", shd0021
        // Test detecting the "PNEUMA" color layer.
        assert_shader_snapshot!("xc2", "bl000301", "21");
    }

    #[test]
    fn shader_from_glsl_tirkin_weapon() {
        // xeno2/model/we/we010402, "body_MT", shd0000
        // Test detecting layers for metalness.
        assert_shader_snapshot!("xc2", "we010402", "0");
    }

    #[test]
    fn shader_from_glsl_behemoth_fins() {
        // xeno2/model/en/en020601, "hire_a", shd0000
        // Test detecting layers for ambient occlusion.
        assert_shader_snapshot!("xc2", "en020601", "0");
    }

    #[test]
    fn shader_from_glsl_lysaat_eyes() {
        // xeno2/model/en/en030601, "phong3", shd0009
        // Detect parallax mapping for texture coordinates.
        assert_shader_snapshot!("xc2", "en030601", "2");
    }

    #[test]
    fn shader_from_glsl_noah_body_outline() {
        // xeno3/chr/ch/ch01011013, "body_outline", shd0000
        // Check for outline data.
        assert_shader_snapshot!("xc3", "ch01011013", "0");
    }

    #[test]
    fn shader_from_glsl_panacea_body() {
        // xeno3/chr/ch/ch44000210, "ch45133501_body", shd0029
        // Check for correct color layers
        assert_shader_snapshot!("xc3", "ch44000210", "29");
    }

    #[test]
    fn shader_from_glsl_l_face() {
        // xenoxde/chr/fc/fc181020, "facemat", shd0008
        // Check for overlay blending to make the face blue.
        assert_shader_snapshot!("xcxde", "fc181020", "8");
    }

    #[test]
    fn shader_from_glsl_elma_eye() {
        // xenoxde/chr/fc/fc281010, "eye_re", shd0002
        // Check reflection layers for the iris.
        assert_shader_snapshot!("xcxde", "fc281010", "2");
    }

    #[test]
    fn shader_from_glsl_elma_leg() {
        // xenoxde/chr/pc/pc221115, "leg_mat", shd0000
        // Check Xenoblade X specific normals and layering.
        assert_shader_snapshot!("xcxde", "pc221115", "0");
    }

    #[test]
    fn shader_from_glsl_elma_hair() {
        // xenoxde/chr/fc/fc282010, "fc282010hair", shd0001
        // Check Xenoblade X hair forward shading.
        assert_shader_snapshot!("xcxde", "fc282010", "1");
    }
}
