use std::sync::LazyLock;

use approx::AbsDiffEq;
use indoc::{formatdoc, indoc};
use xc3_model::shader_database::Operation;

use crate::graph::{
    BinaryOp, Expr, Graph, UnaryOp,
    query::{assign_x_recursive, fma_a_b_c, normalize, query_nodes},
};

pub fn op_func<'a>(
    graph: &'a Graph,
    expr: &'a Expr,
    func: &str,
    op: Operation,
) -> Option<(Operation, Vec<&'a Expr>)> {
    match expr {
        Expr::Func { name, args, .. } => {
            if name == func {
                Some((op, args.iter().map(|a| &graph.exprs[*a]).collect()))
            } else {
                None
            }
        }
        _ => None,
    }
}

static OP_OVER: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
        void main() {
            neg_a = 0.0 - a;
            b_minus_a = neg_a + b;
            result = fma(b_minus_a, ratio, a);
        }
    "};
    Graph::parse_glsl(query).unwrap().simplify()
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
    Graph::parse_glsl(query).unwrap().simplify()
});

pub fn op_mix<'a>(graph: &'a Graph, expr: &'a Expr) -> Option<(Operation, Vec<&'a Expr>)> {
    // getPixelCalcOver in pcmdo fragment shaders for XC1 and XC3.
    let result =
        query_nodes(expr, graph, &OP_OVER).or_else(|| query_nodes(expr, graph, &OP_OVER2))?;
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
    Graph::parse_glsl(query).unwrap().simplify()
});

// TODO: Is it better to just detect this as mix -> mul?
pub fn op_mul_ratio<'a>(graph: &'a Graph, expr: &'a Expr) -> Option<(Operation, Vec<&'a Expr>)> {
    // getPixelCalcRatioBlend in pcmdo fragment shaders for XC1 and XC3.
    let result = query_nodes(expr, graph, &OP_RATIO)?;
    let a = result.get("a")?;
    let b = result.get("b")?;
    let ratio = result.get("ratio")?;
    Some((Operation::MulRatio, vec![a, b, ratio]))
}

pub fn op_fma<'a>(graph: &'a Graph, expr: &'a Expr) -> Option<(Operation, Vec<&'a Expr>)> {
    // += getPixelCalcRatio in pcmdo fragment shaders for XC1 and XC3.
    let (a, b, c) = fma_a_b_c(graph, expr)?;
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
    Graph::parse_glsl(query).unwrap().simplify()
});

// TODO: This can just be detected as mix -> overlay2?
pub fn op_overlay_ratio<'a>(
    graph: &'a Graph,
    expr: &'a Expr,
) -> Option<(Operation, Vec<&'a Expr>)> {
    // Overlay combines multiply and screen blend modes.
    // Some XC2 models use overlay blending for metalness.
    let result = query_nodes(expr, graph, &OP_OVERLAY_XC2)?;
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
    Graph::parse_glsl(query).unwrap().simplify()
});

pub fn op_overlay<'a>(graph: &'a Graph, expr: &'a Expr) -> Option<(Operation, Vec<&'a Expr>)> {
    // Overlay combines multiply and screen blend modes.
    // Some XCX DE models use overlay for face coloring.
    let result = query_nodes(expr, graph, &OP_OVERLAY_XCX_DE)?;
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
    Graph::parse_glsl(query).unwrap().simplify()
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
    Graph::parse_glsl(query).unwrap().simplify()
});

pub fn op_fresnel_ratio<'a>(
    graph: &'a Graph,
    expr: &'a Expr,
) -> Option<(Operation, Vec<&'a Expr>)> {
    let result = query_nodes(expr, graph, &FRESNEL_RATIO)
        .or_else(|| query_nodes(expr, graph, &FRESNEL_RATIO2))?;
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
    Graph::parse_glsl(query).unwrap().simplify()
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
    Graph::parse_glsl(query).unwrap().simplify()
});

pub fn op_pow<'a>(graph: &'a Graph, expr: &'a Expr) -> Option<(Operation, Vec<&'a Expr>)> {
    let result =
        query_nodes(expr, graph, &OP_POW).or_else(|| query_nodes(expr, graph, &OP_POW2))?;
    let a = result.get("a")?;
    let b = result.get("b")?;
    Some((Operation::Power, vec![a, b]))
}

static OP_SQRT: LazyLock<Graph> = LazyLock::new(|| {
    // Equivalent to sqrt(result)
    let query = indoc! {"
        void main() {
            result = inversesqrt(result);
            result = 1.0 / result;
        }
    "};
    Graph::parse_glsl(query).unwrap().simplify()
});

static OP_SQRT2: LazyLock<Graph> = LazyLock::new(|| {
    Graph::parse_glsl("void main() { result = sqrt(result); }")
        .unwrap()
        .simplify()
});

pub fn op_sqrt<'a>(graph: &'a Graph, expr: &'a Expr) -> Option<(Operation, Vec<&'a Expr>)> {
    let result =
        query_nodes(expr, graph, &OP_SQRT).or_else(|| query_nodes(expr, graph, &OP_SQRT2))?;
    let result = result.get("result")?;
    Some((Operation::Sqrt, vec![result]))
}

static OP_DOT4: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
        void main() {
            result = dot(vec4(ax, ay, az, aw), vec4(bx, by, bz, bw));
        }
    "};
    Graph::parse_glsl(query).unwrap().simplify()
});

pub fn op_dot<'a>(graph: &'a Graph, expr: &'a Expr) -> Option<(Operation, Vec<&'a Expr>)> {
    let result = query_nodes(expr, graph, &OP_DOT4)?;

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

pub fn ternary<'a>(graph: &'a Graph, expr: &'a Expr) -> Option<(Operation, Vec<&'a Expr>)> {
    if let Expr::Ternary(cond, a, b) = expr {
        Some((
            Operation::Select,
            vec![&graph.exprs[*cond], &graph.exprs[*a], &graph.exprs[*b]],
        ))
    } else {
        None
    }
}

static OP_SUB: LazyLock<Graph> = LazyLock::new(|| {
    Graph::parse_glsl("void main() { result = a - b; }")
        .unwrap()
        .simplify()
});

static OP_SUB2: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
            void main() {
                neg_b = 0.0 - b;
                result = a + neg_b;
            }
        "};
    Graph::parse_glsl(query).unwrap().simplify()
});

pub fn op_sub<'a>(graph: &'a Graph, expr: &'a Expr) -> Option<(Operation, Vec<&'a Expr>)> {
    // Some layers are simply subtracted like for xeno3/chr/chr/ch44000210.wimdo "ch45133501_body".
    let result =
        query_nodes(expr, graph, &OP_SUB).or_else(|| query_nodes(expr, graph, &OP_SUB2))?;
    let a = result.get("a")?;
    let b = result.get("b")?;
    Some((Operation::Sub, vec![a, b]))
}

static OP_DIV: LazyLock<Graph> = LazyLock::new(|| {
    Graph::parse_glsl("void main() { result = a / b; }")
        .unwrap()
        .simplify()
});

static OP_DIV2: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
            void main() {
                one_over_b = 1.0 / b;
                result = a * one_over_b;
            }
        "};
    Graph::parse_glsl(query).unwrap().simplify()
});

pub fn op_div<'a>(graph: &'a Graph, expr: &'a Expr) -> Option<(Operation, Vec<&'a Expr>)> {
    let result =
        query_nodes(expr, graph, &OP_DIV).or_else(|| query_nodes(expr, graph, &OP_DIV2))?;
    let a = result.get("a")?;
    let b = result.get("b")?;
    Some((Operation::Div, vec![a, b]))
}

pub fn unary_op<'a>(
    graph: &'a Graph,
    expr: &'a Expr,
    unary_op: UnaryOp,
    operation: Operation,
) -> Option<(Operation, Vec<&'a Expr>)> {
    if let Expr::Unary(op, e) = expr
        && *op == unary_op
    {
        return Some((operation, vec![&graph.exprs[*e]]));
    }
    None
}

pub fn binary_op<'a>(
    graph: &'a Graph,
    expr: &'a Expr,
    binary_op: BinaryOp,
    operation: Operation,
) -> Option<(Operation, Vec<&'a Expr>)> {
    if let Expr::Binary(op, a0, a1) = expr
        && *op == binary_op
    {
        return Some((operation, vec![&graph.exprs[*a0], &graph.exprs[*a1]]));
    }
    None
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
    Graph::parse_glsl(query).unwrap().simplify()
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
    Graph::parse_glsl(query).unwrap().simplify()
});

pub fn op_monochrome<'a>(graph: &'a Graph, expr: &'a Expr) -> Option<(Operation, Vec<&'a Expr>)> {
    // calcMonochrome in pcmdo fragment shaders for XC1 and XC3.
    // TODO: Create separate ops or include the RGB weights in the args?
    let result = query_nodes(expr, graph, &OP_MONOCHROME)
        .or_else(|| query_nodes(expr, graph, &OP_MONOCHROME_XC1))?;
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
    Graph::parse_glsl(query).unwrap().simplify()
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
    Graph::parse_glsl(query).unwrap().simplify()
});

pub fn op_add_normal<'a>(graph: &'a Graph, expr: &'a Expr) -> Option<(Operation, Vec<&'a Expr>)> {
    // getPixelCalcAddNormal in pcmdo shaders.
    // normalize(mix(nomWork, normalize(r), ratio))
    // XC2: ratio * (normalize(r) - nomWork) + nomWork
    // XC3: (normalize(r) - nomWork) * ratio + nomWork

    // The normalize is baked into the outer query and might not be present.
    let mut expr = expr;
    if let Some(new_expr) = normalize(graph, expr) {
        expr = assign_x_recursive(graph, new_expr);
    }

    let result = query_nodes(expr, graph, &OP_ADD_NORMAL_OUTER)
        .or_else(|| query_nodes(expr, graph, &OP_ADD_NORMAL))?;

    let n1_x = result.get("n1_x")?;
    let n1_y = result.get("n1_y")?;

    let n2_x = result.get("n2_x")?;
    let n2_y = result.get("n2_y")?;

    let ratio = result.get("ratio")?;

    let mut nom_work = *result.get("nom_work")?;
    nom_work = assign_x_recursive(graph, nom_work);
    if let Some(new_expr) = normalize(graph, nom_work) {
        nom_work = assign_x_recursive(graph, new_expr);
    }

    let op = if nom_work == assign_x_recursive(graph, n1_x) {
        Operation::AddNormalX
    } else if nom_work == assign_x_recursive(graph, n1_y) {
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
    Graph::parse_glsl(query).unwrap().simplify()
});

pub fn op_overlay2<'a>(graph: &'a Graph, nom_work: &'a Expr) -> Option<(Operation, Vec<&'a Expr>)> {
    let result = query_nodes(nom_work, graph, &OP_OVERLAY2)?;
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
    Graph::parse_glsl(query).unwrap().simplify()
});

pub fn normal_map_fma<'a>(graph: &'a Graph, nom_work: &'a Expr) -> Option<&'a Expr> {
    // Extract the normal map texture if present.
    // This could be fma(x, 2.0, -1.0) or fma(x, 2.0, -1.0039216)
    let result = query_nodes(nom_work, graph, &NORMAL_MAP_FMA)?;
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
            if matches!(&graph.exprs[*f], Expr::Float(f) if f.abs_diff_eq(&1.0, 1.0 / 128.0)) {
                result.get("result").copied()
            } else {
                None
            }
        }
        _ => None,
    }
}

static CALC_NORMAL_MAP_X: LazyLock<Graph> = LazyLock::new(|| {
    // TODO: remove lines like x = x;
    // TODO: detect normalize on attributes to properly differentiate channels
    let query = indoc! {"
        void main() {
            inverse_length_tangent = inversesqrt(tangent_length);
            tangent = tangent.x;
            normalize_tangent = tangent * inverse_length_tangent;
            result_x = result_x;
            result = result_x * normalize_tangent;

            inverse_length_bitangent = inversesqrt(bitangent_length);
            bitangent = bitangent_x;
            normalize_bitangent = bitangent * inverse_length_bitangent;
            result_y = result_y;
            result = fma(result_y, normalize_bitangent, result);

            inverse_length_normal = inversesqrt(normal_length);
            normal = normal_x;
            normalize_normal = normal * inverse_length_normal;
            result_z = result_z;
            result = fma(result_z, normalize_normal, result);
        }
    "};
    Graph::parse_glsl(query).unwrap().simplify()
});

static CALC_NORMAL_MAP_Y: LazyLock<Graph> = LazyLock::new(|| {
    // TODO: remove lines like x = x;
    // TODO: detect normalize on attributes to properly differentiate channels
    let query = indoc! {"
        void main() {
            inverse_length_tangent = inversesqrt(tangent_length);
            tangent = tangent.y;
            normalize_tangent = tangent * inverse_length_tangent;
            result_x = result_x;
            result = result_x * normalize_tangent;

            inverse_length_normal = inversesqrt(normal_length);
            normal = normal_y;
            normalize_normal = normal * inverse_length_normal;
            result_z = result_z;
            result = fma(result_z, normalize_normal, result);

            inverse_length_bitangent = inversesqrt(bitangent_length);
            bitangent = bitangent_y;
            normalize_bitangent = bitangent * inverse_length_bitangent;
            result_y = result_y;
            result = fma(result_y, normalize_bitangent, result);
        }
    "};
    Graph::parse_glsl(query).unwrap().simplify()
});

pub fn calc_normal_map<'a>(graph: &'a Graph, expr: &'a Expr) -> Option<[&'a Expr; 3]> {
    let result = query_nodes(expr, graph, &CALC_NORMAL_MAP_X)
        .or_else(|| query_nodes(expr, graph, &CALC_NORMAL_MAP_Y))?;
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
    Graph::parse_glsl(&query).unwrap().simplify()
});

static CALC_NORMAL_MAP_W_INTENSITY_Y: LazyLock<Graph> = LazyLock::new(|| {
    // normal.y with normal.w as normal map intensity.
    // TODO: Does intensity always use pow(intensity, 0.7)?
    let query = calc_normal_map_w_intensity_query('y');
    Graph::parse_glsl(&query).unwrap().simplify()
});

pub fn calc_normal_map_w_intensity<'a>(
    graph: &'a Graph,
    expr: &'a Expr,
) -> Option<([&'a Expr; 3], &'a Expr)> {
    let result = query_nodes(expr, graph, &CALC_NORMAL_MAP_W_INTENSITY_X)
        .or_else(|| query_nodes(expr, graph, &CALC_NORMAL_MAP_W_INTENSITY_Y))?;
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
    // TODO: Fix this
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
    Graph::parse_glsl(&query).unwrap().simplify()
});

static CALC_NORMAL_MAP_VAL_INF_XCX_Y: LazyLock<Graph> = LazyLock::new(|| {
    let query = calc_normal_map_val_inf_xcx_query('y');
    Graph::parse_glsl(&query).unwrap().simplify()
});

static CALC_NORMAL_MAP_VAL_INF_XCX_Z: LazyLock<Graph> = LazyLock::new(|| {
    let query = calc_normal_map_val_inf_xcx_query('z');
    Graph::parse_glsl(&query).unwrap().simplify()
});

pub fn calc_normal_map_xcx<'a>(graph: &'a Graph, expr: &'a Expr) -> Option<[&'a Expr; 3]> {
    let result = query_nodes(expr, graph, &CALC_NORMAL_MAP_VAL_INF_XCX_X)
        .or_else(|| query_nodes(expr, graph, &CALC_NORMAL_MAP_VAL_INF_XCX_Y))?;
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
    Graph::parse_glsl(&query).unwrap().simplify()
});

static CALC_NORMAL_MAP_XCX_Y: LazyLock<Graph> = LazyLock::new(|| {
    let query = calc_normal_map_xcx_query('y');
    Graph::parse_glsl(&query).unwrap().simplify()
});

static CALC_NORMAL_MAP_XCX_Z: LazyLock<Graph> = LazyLock::new(|| {
    let query = calc_normal_map_xcx_query('z');
    Graph::parse_glsl(&query).unwrap().simplify()
});

pub fn op_calc_normal_map<'a>(
    graph: &'a Graph,
    expr: &'a Expr,
) -> Option<(Operation, Vec<&'a Expr>)> {
    // TODO: Detect normal mapping from other games.
    let (op, result) = query_nodes(expr, graph, &CALC_NORMAL_MAP_XCX_X)
        .or_else(|| query_nodes(expr, graph, &CALC_NORMAL_MAP_VAL_INF_XCX_X))
        .map(|r| (Operation::NormalMapX, r))
        .or_else(|| {
            query_nodes(expr, graph, &CALC_NORMAL_MAP_XCX_Y)
                .or_else(|| query_nodes(expr, graph, &CALC_NORMAL_MAP_VAL_INF_XCX_Y))
                .map(|r| (Operation::NormalMapY, r))
        })
        .or_else(|| {
            query_nodes(expr, graph, &CALC_NORMAL_MAP_XCX_Z)
                .or_else(|| query_nodes(expr, graph, &CALC_NORMAL_MAP_VAL_INF_XCX_Z))
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
    Graph::parse_glsl(query).unwrap().simplify()
});

pub fn geometric_specular_aa<'a>(graph: &'a Graph, expr: &'a Expr) -> Option<&'a Expr> {
    let result = query_nodes(expr, graph, &GEOMETRIC_SPECULAR_AA)?;
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
    Graph::parse_glsl(query).unwrap().simplify()
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
    Graph::parse_glsl(query).unwrap().simplify()
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
    Graph::parse_glsl(query).unwrap().simplify()
});

static SKIN_ATTRIBUTE_XYZ_X2: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
        void main() {
            temp_0 = nWgtIdx.x;
            temp_3 = result_x;
            temp_6 = floatBitsToInt(temp_0) & 65535;
            temp_7 = temp_6 * 48;
            temp_8 = result_y;
            temp_9 = floatBitsToUint(temp_0) >> 16;
            temp_10 = int(temp_9) * 48;
            temp_11 = temp_10 << 16;
            temp_12 = temp_11 + temp_7;
            temp_14 = result_z;
            temp_58 = uint(temp_12) >> 2;
            temp_59 = uintBitsToFloat(U_OdB.data[int(temp_58)]);
            temp_60 = temp_12 + 4;
            temp_61 = uint(temp_60) >> 2;
            temp_62 = uintBitsToFloat(U_OdB.data[int(temp_61)]);
            temp_63 = temp_12 + 8;
            temp_64 = uint(temp_63) >> 2;
            temp_65 = uintBitsToFloat(U_OdB.data[int(temp_64)]);
            temp_98 = temp_59 * temp_3;
            temp_103 = fma(temp_62, temp_8, temp_98);
            temp_120 = fma(temp_65, temp_14, temp_103);
        }
    "};
    Graph::parse_glsl(query).unwrap().simplify()
});

static SKIN_ATTRIBUTE_XYZ_Y2: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
        void main() {
            temp_0 = nWgtIdx.x;
            temp_3 = result_x;
            temp_6 = floatBitsToInt(temp_0) & 65535;
            temp_7 = temp_6 * 48;
            temp_8 = result_y;
            temp_9 = floatBitsToUint(temp_0) >> 16;
            temp_10 = int(temp_9) * 48;
            temp_11 = temp_10 << 16;
            temp_12 = temp_11 + temp_7;
            temp_14 = result_z;
            temp_15 = temp_12 + 16;
            temp_34 = uint(temp_15) >> 2;
            temp_35 = uintBitsToFloat(U_OdB.data[int(temp_34)]);
            temp_36 = temp_15 + 4;
            temp_37 = uint(temp_36) >> 2;
            temp_38 = uintBitsToFloat(U_OdB.data[int(temp_37)]);
            temp_39 = temp_15 + 8;
            temp_40 = uint(temp_39) >> 2;
            temp_41 = uintBitsToFloat(U_OdB.data[int(temp_40)]);
            temp_95 = temp_35 * temp_3;
            temp_110 = fma(temp_38, temp_8, temp_95);
            temp_115 = fma(temp_41, temp_14, temp_110);
        }
    "};
    Graph::parse_glsl(query).unwrap().simplify()
});

static SKIN_ATTRIBUTE_XYZ_Z2: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
        void main() {
            temp_0 = nWgtIdx.x;
            temp_3 = result_x;
            temp_6 = floatBitsToInt(temp_0) & 65535;
            temp_7 = temp_6 * 48;
            temp_8 = result_y;
            temp_9 = floatBitsToUint(temp_0) >> 16;
            temp_10 = int(temp_9) * 48;
            temp_11 = temp_10 << 16;
            temp_12 = temp_11 + temp_7;
            temp_14 = result_z;
            temp_19 = temp_12 + 32;
            temp_46 = uint(temp_19) >> 2;
            temp_47 = uintBitsToFloat(U_OdB.data[int(temp_46)]);
            temp_48 = temp_19 + 4;
            temp_49 = uint(temp_48) >> 2;
            temp_50 = uintBitsToFloat(U_OdB.data[int(temp_49)]);
            temp_51 = temp_19 + 8;
            temp_52 = uint(temp_51) >> 2;
            temp_53 = uintBitsToFloat(U_OdB.data[int(temp_52)]);
            temp_104 = temp_47 * temp_3;
            temp_113 = fma(temp_50, temp_8, temp_104);
            temp_118 = fma(temp_53, temp_14, temp_113);
        }
    "};
    Graph::parse_glsl(query).unwrap().simplify()
});

pub fn skin_attribute_xyz<'a>(graph: &'a Graph, expr: &'a Expr) -> Option<&'a Expr> {
    query_nodes(expr, graph, &SKIN_ATTRIBUTE_XYZ_X)
        .or_else(|| query_nodes(expr, graph, &SKIN_ATTRIBUTE_XYZ_X2))
        .and_then(|r| r.get("result_x").copied())
        .or_else(|| {
            query_nodes(expr, graph, &SKIN_ATTRIBUTE_XYZ_Y)
                .or_else(|| query_nodes(expr, graph, &SKIN_ATTRIBUTE_XYZ_Y2))
                .and_then(|r| r.get("result_y").copied())
        })
        .or_else(|| {
            query_nodes(expr, graph, &SKIN_ATTRIBUTE_XYZ_Z)
                .or_else(|| query_nodes(expr, graph, &SKIN_ATTRIBUTE_XYZ_Z2))
                .and_then(|r| r.get("result_z").copied())
        })
}

// TODO: combine these queries and only check the integer values?
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
    Graph::parse_glsl(query).unwrap().simplify()
});

static SKIN_ATTRIBUTE_XYZW_YZ: LazyLock<Graph> = LazyLock::new(|| {
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
            temp_13 = temp_7 + offset;
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
    Graph::parse_glsl(query).unwrap().simplify()
});

pub fn skin_attribute_xyzw<'a>(graph: &'a Graph, expr: &'a Expr) -> Option<&'a Expr> {
    // TODO: Combine these queries
    query_nodes(expr, graph, &SKIN_ATTRIBUTE_XYZW_X)
        .and_then(|r| r.get("result_x").copied())
        .or_else(|| {
            query_nodes(expr, graph, &SKIN_ATTRIBUTE_XYZW_YZ).and_then(|r| {
                let offset = r.get("offset")?;
                match offset {
                    Expr::Int(16) => r.get("result_y").copied(),
                    Expr::Int(32) => r.get("result_z").copied(),
                    _ => None,
                }
            })
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
    Graph::parse_glsl(query).unwrap().simplify()
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
    Graph::parse_glsl(query).unwrap().simplify()
});

pub fn skin_attribute_clip_space_xyzw<'a>(graph: &'a Graph, expr: &'a Expr) -> Option<&'a Expr> {
    let result = query_nodes(expr, graph, &SKIN_ATTRIBUTE_CLIP_XYZW)
        .or_else(|| query_nodes(expr, graph, &SKIN_ATTRIBUTE_CLIP_XYZW_Z))?;
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
    Graph::parse_glsl(query).unwrap().simplify()
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
    Graph::parse_glsl(query).unwrap().simplify()
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
    Graph::parse_glsl(query).unwrap().simplify()
});

pub fn skin_attribute_bitangent<'a>(graph: &'a Graph, expr: &'a Expr) -> Option<Expr> {
    let channel = query_nodes(expr, graph, &SKIN_ATTRIBUTE_BITANGENT_X)
        .map(|_| 'x')
        .or_else(|| query_nodes(expr, graph, &SKIN_ATTRIBUTE_BITANGENT_Y).map(|_| 'y'))
        .or_else(|| query_nodes(expr, graph, &SKIN_ATTRIBUTE_BITANGENT_Z).map(|_| 'z'))?;
    Some(Expr::Global {
        name: "vBitan".into(),
        channel: Some(channel),
    })
}

static U_MDL_ATTRIBUTE_XYZW: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
        void main() {
            temp_0 = result_x;
            temp_1 = result_y;
            temp_2 = result_z;
            temp_3 = result_w;
            temp_24 = temp_0 * U_Mdl.gmWorldView[index].x;
            temp_28 = fma(temp_1, U_Mdl.gmWorldView[index].y, temp_24);
            temp_34 = fma(temp_2, U_Mdl.gmWorldView[index].z, temp_28);
            temp_40 = fma(temp_3, U_Mdl.gmWorldView[index].w, temp_34);
        }
    "};
    Graph::parse_glsl(query).unwrap().simplify()
});

pub fn u_mdl_view_attribute_xyzw<'a>(graph: &'a Graph, expr: &'a Expr) -> Option<&'a Expr> {
    let result = query_nodes(expr, graph, &U_MDL_ATTRIBUTE_XYZW)?;
    let index = result.get("index")?;
    match index {
        Expr::Int(0) => result.get("result_x").copied(),
        Expr::Int(1) => result.get("result_y").copied(),
        Expr::Int(2) => result.get("result_z").copied(),
        _ => None,
    }
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
    Graph::parse_glsl(query).unwrap().simplify()
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
    Graph::parse_glsl(query).unwrap().simplify()
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
    Graph::parse_glsl(query).unwrap().simplify()
});

pub fn u_mdl_view_bitangent_xyz<'a>(graph: &'a Graph, expr: &'a Expr) -> Option<Expr> {
    let channel = query_nodes(expr, graph, &U_MDL_VIEW_BITANGENT_X)
        .map(|_| 'x')
        .or_else(|| query_nodes(expr, graph, &U_MDL_VIEW_BITANGENT_Y).map(|_| 'y'))
        .or_else(|| query_nodes(expr, graph, &U_MDL_VIEW_BITANGENT_Z).map(|_| 'z'))?;
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
    Graph::parse_glsl(query).unwrap().simplify()
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
    Graph::parse_glsl(query).unwrap().simplify()
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
    Graph::parse_glsl(query).unwrap().simplify()
});

pub fn u_mdl_clip_attribute_xyzw<'a>(graph: &'a Graph, expr: &'a Expr) -> Option<&'a Expr> {
    query_nodes(expr, graph, &U_MDL_CLIP_ATTRIBUTE_XYZW_X)
        .and_then(|r| r.get("result_x").copied())
        .or_else(|| {
            query_nodes(expr, graph, &U_MDL_CLIP_ATTRIBUTE_XYZW_Y)
                .and_then(|r| r.get("result_y").copied())
        })
        .or_else(|| {
            query_nodes(expr, graph, &U_MDL_CLIP_ATTRIBUTE_XYZW_Z)
                .and_then(|r| r.get("result_z").copied())
        })
}

static U_MDL_ATTRIBUTE_XYZ: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
        void main() {
            temp_0 = result_x;
            temp_1 = result_y;
            temp_2 = result_z;
            temp_24 = temp_0 * U_Mdl.gmWorldView[index].x;
            temp_28 = fma(temp_1, U_Mdl.gmWorldView[index].y, temp_24);
            temp_34 = fma(temp_2, U_Mdl.gmWorldView[index].z, temp_28);
        }
    "};
    Graph::parse_glsl(query).unwrap().simplify()
});

pub fn u_mdl_attribute_xyz<'a>(graph: &'a Graph, expr: &'a Expr) -> Option<&'a Expr> {
    let result = query_nodes(expr, graph, &U_MDL_ATTRIBUTE_XYZ)?;
    let index = result.get("index")?;
    match index {
        Expr::Int(0) => result.get("result_x").copied(),
        Expr::Int(1) => result.get("result_y").copied(),
        Expr::Int(2) => result.get("result_z").copied(),
        _ => None,
    }
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
    Graph::parse_glsl(query).unwrap().simplify()
});

pub fn tex_matrix<'a>(graph: &'a Graph, expr: &'a Expr) -> Option<(Operation, Vec<&'a Expr>)> {
    // Detect matrix multiplication for the mat4x2 "gTexMat * vec4(u, v, 0.0, 1.0)".
    // U and V have the same pattern but use a different row of the matrix.
    let result = query_nodes(expr, graph, &TEX_MATRIX)?;
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
    Graph::parse_glsl(query).unwrap().simplify()
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
    Graph::parse_glsl(query).unwrap().simplify()
});

pub fn tex_parallax<'a>(graph: &'a Graph, expr: &'a Expr) -> Option<(Operation, Vec<&'a Expr>)> {
    let expr = assign_x_recursive(graph, expr);

    // Some eye shaders use some form of parallax mapping.
    let result = query_nodes(expr, graph, &TEX_PARALLAX)
        .or_else(|| query_nodes(expr, graph, &TEX_PARALLAX2))?;

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
    Graph::parse_glsl(query).unwrap().simplify()
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
    Graph::parse_glsl(query).unwrap().simplify()
});

pub fn tex_parallax2<'a>(graph: &'a Graph, expr: &'a Expr) -> Option<(Operation, Vec<&'a Expr>)> {
    // Some eye shaders use some form of parallax mapping.
    let result = query_nodes(expr, graph, &TEX_PARALLAX3_X)
        .or_else(|| query_nodes(expr, graph, &TEX_PARALLAX3_Y))?;

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
    Graph::parse_glsl(query).unwrap().simplify()
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
    Graph::parse_glsl(query).unwrap().simplify()
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
    Graph::parse_glsl(query).unwrap().simplify()
});

pub fn op_reflect<'a>(graph: &'a Graph, expr: &'a Expr) -> Option<(Operation, Vec<&'a Expr>)> {
    let (op, result) = query_nodes(expr, graph, &REFLECT_X)
        .map(|r| (Operation::ReflectX, r))
        .or_else(|| query_nodes(expr, graph, &REFLECT_Y).map(|r| (Operation::ReflectY, r)))
        .or_else(|| query_nodes(expr, graph, &REFLECT_Z).map(|r| (Operation::ReflectZ, r)))?;

    let n_x = result.get("n_x")?;
    let n_y = result.get("n_y")?;
    let n_z = result.get("n_z")?;

    let i_x = result.get("i_x")?;
    let i_y = result.get("i_y")?;
    let i_z = result.get("i_z")?;

    Some((op, vec![i_x, i_y, i_z, n_x, n_y, n_z]))
}

static FUR_INSTANCE_ALPHA: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
        void main() {
            temp_3 = intBitsToFloat(gl_InstanceID);
            temp_14 = float(floatBitsToInt(temp_3));
            temp_135 = temp_14 * param;
            temp_136 = clamp(temp_135, 0.0, 1.0);
            temp_140 = 0.0 - temp_136;
            temp_141 = temp_140 + 1.0;
            result = temp_141;
        }
    "};
    Graph::parse_glsl(query).unwrap().simplify()
});

pub fn op_fur_instance_alpha<'a>(
    graph: &'a Graph,
    expr: &'a Expr,
) -> Option<(Operation, Vec<&'a Expr>)> {
    let result = query_nodes(expr, graph, &FUR_INSTANCE_ALPHA)?;
    let param = result.get("param")?;
    Some((Operation::FurInstanceAlpha, vec![param]))
}
