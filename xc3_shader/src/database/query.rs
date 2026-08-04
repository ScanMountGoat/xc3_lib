use std::sync::LazyLock;

use approx::AbsDiffEq;
use indoc::{formatdoc, indoc};
use xc3_model::shader_database::Operation;

use crate::graph::{
    BinaryOp, Expr, Graph, UnaryOp,
    query::{assign_x_recursive, dot3_a_b, fma_a_b_c, normalize, query_nodes},
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
        neg_a = 0.0 - a;
        b_minus_a = neg_a + b;
        result = fma(b_minus_a, ratio, a);
    "};
    Graph::parse_glsl_query(query).unwrap().simplify()
});

static OP_OVER2: LazyLock<Graph> = LazyLock::new(|| {
    // Alternative form used for some shaders.
    let query = indoc! {"
        neg_ratio = 0.0 - ratio;
        a_inv_ratio = fma(a, neg_ratio, a);
        result = fma(b, ratio, a_inv_ratio);
    "};
    Graph::parse_glsl_query(query).unwrap().simplify()
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
        neg_a = 0.0 - a;
        ab_minus_a = fma(a, b, neg_a);
        result = fma(ab_minus_a, ratio, a);
    "};
    Graph::parse_glsl_query(query).unwrap().simplify()
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
    "};
    Graph::parse_glsl_query(query).unwrap().simplify()
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
    "};
    Graph::parse_glsl_query(query).unwrap().simplify()
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
        n_dot_v = abs(n_dot_v);
        neg_n_dot_v = 0.0 - n_dot_v;
        one_minus_n_dot_v = neg_n_dot_v + 1.0;
        result = log2(one_minus_n_dot_v);
        ratio = ratio * 5.0;
        result = ratio * result;
        result = exp2(result);
    "};
    Graph::parse_glsl_query(query).unwrap().simplify()
});

static FRESNEL_RATIO2: LazyLock<Graph> = LazyLock::new(|| {
    // Variant for XCX DE shaders with log2(abs()) instead of log2().
    // pow(1.0 - n_dot_v, ratio * 5.0)
    let query = indoc! {"
        n_dot_v = abs(n_dot_v);
        neg_n_dot_v = 0.0 - n_dot_v;
        one_minus_n_dot_v = neg_n_dot_v + 1.0;
        one_minus_n_dot_v = abs(one_minus_n_dot_v);
        result = log2(one_minus_n_dot_v);
        ratio = ratio * 5.0;
        result = ratio * result;
        result = exp2(result);
    "};
    Graph::parse_glsl_query(query).unwrap().simplify()
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
        a = abs(a);
        a = log2(a);
        a = a * b;
        a = exp2(a);
    "};
    Graph::parse_glsl_query(query).unwrap().simplify()
});

static OP_POW2: LazyLock<Graph> = LazyLock::new(|| {
    // Equivalent to pow(a, b)
    let query = indoc! {"
        a = log2(a);
        a = a * b;
        a = exp2(a);
    "};
    Graph::parse_glsl_query(query).unwrap().simplify()
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
        result = inversesqrt(result);
        result = 1.0 / result;
    "};
    Graph::parse_glsl_query(query).unwrap().simplify()
});

static OP_SQRT2: LazyLock<Graph> = LazyLock::new(|| {
    Graph::parse_glsl_query("result = sqrt(result);")
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
    let query = "result = dot(vec4(ax, ay, az, aw), vec4(bx, by, bz, bw));";
    Graph::parse_glsl_query(query).unwrap().simplify()
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
    Graph::parse_glsl_query("result = a - b;")
        .unwrap()
        .simplify()
});

static OP_SUB2: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
        neg_b = 0.0 - b;
        result = a + neg_b;
    "};
    Graph::parse_glsl_query(query).unwrap().simplify()
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
    Graph::parse_glsl_query("result = a / b;")
        .unwrap()
        .simplify()
});

static OP_DIV2: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
        one_over_b = 1.0 / b;
        result = a * one_over_b;
    "};
    Graph::parse_glsl_query(query).unwrap().simplify()
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
        b = x * 0.01;
        b = fma(y, 0.01, b);
        b = fma(z, 0.01, b);
        neg_a = 0.0 - a;
        b_minus_a = neg_a + b;
        result = fma(b_minus_a, ratio, a);
    "};
    Graph::parse_glsl_query(query).unwrap().simplify()
});

static OP_MONOCHROME_XC1: LazyLock<Graph> = LazyLock::new(|| {
    // result = mix(color, dot(color, vec3(0.3, 0.59, 0.11), ratio))
    let query = indoc! {"
        b = x * 0.3;
        b = fma(y, 0.59, b);
        b = fma(z, 0.11, b);
        neg_a = 0.0 - a;
        b_minus_a = neg_a + b;
        result = fma(b_minus_a, ratio, a);
    "};
    Graph::parse_glsl_query(query).unwrap().simplify()
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
    // xeno3/chr/ch/ch01027000, "body_toon", shd0087
    // The normal maps have only XY channels.
    // The t and u values are negated here for some reason.
    // The the final result is still equivalent to the pcmdo code.
    // t = n1.xyz + vec3(0.0, 0.0, 1.0);
    // u = n2.xyz * vec3(-1.0, -1.0, 1.0);
    // r = t * dot(t, u) - u * t.z;
    // result = normalize(mix(n1, normalize(r), ratio));
    // TODO: include the normal map fma for n2 here?
    // TODO: Assume n2 is a normal map?
    // TODO: detect t.z for r?
    let query = indoc! {"
        t_x = 0.0 + n1_x;
        t_y = 0.0 + n1_y;
        t_z = n1_z + 1.0;
        u_x = n2_x;
        u_y = n2_y;

        neg_t_x = 0.0 - t_x;
        neg_t_y = 0.0 - t_y;

        dot_t_u = u_x * neg_t_x;
        dot_t_u = fma(u_y, neg_t_y, dot_t_u);
        dot_t_u = fma(u_z, t_z, dot_t_u);

        temp6 = fma(temp2, dot_t_u, neg_n2);

        neg_n1 = 0.0 - n1;
        n_inv_sqrt = inversesqrt(temp4);
        r = fma(temp6, n_inv_sqrt, neg_n1);

        nom_work = fma(r, ratio, nom_work);
    "};
    Graph::parse_glsl_query(query).unwrap().simplify()
});

static OP_ADD_NORMAL2: LazyLock<Graph> = LazyLock::new(|| {
    // xeno2/map/ma30a, props, "TR0502d_BaseTrunkA", shd0001
    // Unlike most shaders, the normal maps have XYZ channels.
    // Assume n2 is a normal map texture.
    // t = n1.xyz + vec3(0.0, 0.0, 1.0);
    // u = n2.xyz * vec3(-1.0, -1.0, 1.0);
    // r = t * dot(t, u) - u * t.z;
    // result = normalize(mix(n1, normalize(r), ratio));
    // TODO: detect t.z for r?
    let query = indoc! {"
        t_x = fma(n1_x, n1_inverse_sqrt, 0.0);
        t_y = fma(n1_y, n1_inverse_sqrt, 0.0);
        t_z = fma(n1_z, n1_inverse_sqrt, 1.0);
        u_x = fma(n2_x, -2.0, 1.0);
        u_y = fma(n2_y, -2.0, 1.0);
        u_z = fma(n2_z, 2.0, -1.0);

        dot_t_u = t_x * u_x;
        dot_t_u = fma(t_y, u_y, dot_t_u);
        dot_t_u = fma(t_z, u_z, dot_t_u);

        temp6 = fma(t_x, dot_t_u, neg_n2);

        n_inv_sqrt = inversesqrt(temp4);
        r = fma(temp6, n_inv_sqrt, neg_n1);

        nom_work = fma(r, ratio, nom_work);
    "};
    Graph::parse_glsl_query(query).unwrap().simplify()
});

static OP_ADD_NORMAL3: LazyLock<Graph> = LazyLock::new(|| {
    // xeno2/model/np/np001101, "body", shd0013
    // Slightly different version of dot(t, u) for the outermost call.
    // TODO: Figure out why this needs a separate query.
    let query = indoc! {"
        n1_x = fma(n1_x, n1_inverse_sqrt, 0.0);
        n1_y = fma(n1_y, n1_inverse_sqrt, 0.0);
        n1_z_plus_one = fma(n1_z, n1_inverse_sqrt, 1.0);
        neg_n1_x = 0.0 - n1_x;
        neg_n1_y = 0.0 - n1_y;

        dot_t_u = n2_x * neg_n1_x;
        dot_t_u = fma(n2_y, neg_n1_y, dot_t_u);
        dot_t_u = fma(n2_z, n1_z_plus_one, dot_t_u);

        temp6 = fma(n1_x, dot_t_u, neg_n2);

        n_inv_sqrt = inversesqrt(temp4);
        r = fma(temp6, n_inv_sqrt, neg_n1);

        nom_work = fma(r, ratio, nom_work);
    "};
    Graph::parse_glsl_query(query).unwrap().simplify()
});

pub fn op_add_normal<'a>(graph: &'a Graph, expr: &'a Expr) -> Option<(Operation, Vec<&'a Expr>)> {
    // getPixelCalcAddNormal in pcmdo shaders.
    // normalize(mix(nomWork, normalize(r), ratio))
    // XC2: ratio * (normalize(r) - nomWork) + nomWork
    // XC3: (normalize(r) - nomWork) * ratio + nomWork

    // The normalize is baked into the outer query and might not be present.
    let mut expr = expr;
    if let Some(new_expr) = normalize(graph, expr) {
        expr = new_expr;
    }
    let result = query_nodes(expr, graph, &OP_ADD_NORMAL3)
        .or_else(|| query_nodes(expr, graph, &OP_ADD_NORMAL2))
        .or_else(|| query_nodes(expr, graph, &OP_ADD_NORMAL))?;

    let n1_x = result.get("n1_x")?;
    let n1_y = result.get("n1_y")?;

    let n2_x = result.get("n2_x")?;
    let n2_y = result.get("n2_y")?;

    let ratio = result.get("ratio")?;

    let mut nom_work = *result.get("nom_work")?;
    if let Some(new_expr) = normalize(graph, nom_work) {
        nom_work = new_expr;
    }

    let op = if nom_work == *n1_x {
        Operation::AddNormalX
    } else if nom_work == *n1_y {
        Operation::AddNormalY
    } else {
        Operation::Unk
    };

    Some((op, vec![n1_x, n1_y, n2_x, n2_y, ratio]))
}

static OP_OVERLAY2: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
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
    "};
    Graph::parse_glsl_query(query).unwrap().simplify()
});

pub fn op_overlay2<'a>(graph: &'a Graph, nom_work: &'a Expr) -> Option<(Operation, Vec<&'a Expr>)> {
    let result = query_nodes(nom_work, graph, &OP_OVERLAY2)?;
    let a = *result.get("a")?;
    let b = result.get("b")?;
    Some((Operation::Overlay2, vec![a, b]))
}

static NORMAL_MAP_FMA: LazyLock<Graph> = LazyLock::new(|| {
    let query = "result = fma(result, 2.0, neg_one);";
    Graph::parse_glsl_query(query).unwrap().simplify()
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

// TODO: This can also have the order swapped with tan -> normal -> bitan
// TODO: better channel detection?
static CALC_NORMAL_MAP_X: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
        inverse_length_tangent = inversesqrt(tangent_length);
        normalize_tangent = tangent.x * inverse_length_tangent;
        result = result_x * normalize_tangent;

        inverse_length_bitangent = inversesqrt(bitangent_length);
        normalize_bitangent = bitangent.x * inverse_length_bitangent;
        result = fma(result_y, normalize_bitangent, result);

        inverse_length_normal = inversesqrt(normal_length);
        normalize_normal = normal.x * inverse_length_normal;
        result = fma(result_z, normalize_normal, result);
    "};
    Graph::parse_glsl_query(query).unwrap().simplify()
});

static CALC_NORMAL_MAP_Y: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
        inverse_length_tangent = inversesqrt(tangent_length);
        normalize_tangent = tangent.y * inverse_length_tangent;
        result = result_x * normalize_tangent;

        inverse_length_normal = inversesqrt(normal_length);
        normalize_normal = normal.y * inverse_length_normal;
        result = fma(result_z, normalize_normal, result);

        inverse_length_bitangent = inversesqrt(bitangent_length);
        normalize_bitangent = bitangent.y * inverse_length_bitangent;
        result = fma(result_y, normalize_bitangent, result);
    "};
    Graph::parse_glsl_query(query).unwrap().simplify()
});

pub fn calc_normal_map<'a>(graph: &'a Graph, expr: &'a Expr) -> Option<[&'a Expr; 3]> {
    let result = query_nodes(expr, graph, &CALC_NORMAL_MAP_X)
        .or_else(|| query_nodes(expr, graph, &CALC_NORMAL_MAP_Y))?;
    // TODO: detect TBN vectors to properly differentiate result_xyz
    Some([
        result.get("result_x")?,
        result.get("result_y")?,
        result.get("result_z")?,
    ])
}

fn calc_normal_map_w_intensity_query(c: char) -> String {
    formatdoc! {"
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
    "}
}

static CALC_NORMAL_MAP_W_INTENSITY_X: LazyLock<Graph> = LazyLock::new(|| {
    // normal.x with normal.w as normal map intensity.
    // TODO: Does intensity always use pow(intensity, 0.7)?
    let query = calc_normal_map_w_intensity_query('x');
    Graph::parse_glsl_query(&query).unwrap().simplify()
});

static CALC_NORMAL_MAP_W_INTENSITY_Y: LazyLock<Graph> = LazyLock::new(|| {
    // normal.y with normal.w as normal map intensity.
    // TODO: Does intensity always use pow(intensity, 0.7)?
    let query = calc_normal_map_w_intensity_query('y');
    Graph::parse_glsl_query(&query).unwrap().simplify()
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

fn calc_normal_map_val_inf_query(c: char) -> String {
    // A fix for mirrored normal map seams used for XCX, XCXDE, and XC3.
    // TODO: is this gram-schmidt process to make bump normal and view(vValInf) orthogonal?
    // TODO: neg_dot_val_inf_normal is multipled by intensity?
    // TODO: intensity is only 1.0 for seams?
    // TODO: is vValInf some sort of seam adjustment?
    // intensity = clamp(1.0 - sqrt(normal.w), 0.0, 1.0)
    // TODO: should this be its own operation?
    formatdoc! {"
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

        intensity = sqrt(intensity);
        intensity = 0.0 - intensity;
        intensity = intensity + 1.0;
        intensity = clamp(intensity, 0.0, 1.0);
        dot_val_inf_normal = dot_val_inf_normal * intensity;
        neg_dot_val_inf_normal = 0.0 - dot_val_inf_normal;

        inverse_length_normal = inversesqrt(normal_length);
        result = result * inverse_length_normal;
        result = fma(normalize_val_inf, neg_dot_val_inf_normal, result);
    "}
}

static CALC_NORMAL_MAP_VAL_INF_XCX_X: LazyLock<Graph> = LazyLock::new(|| {
    let query = calc_normal_map_val_inf_query('x');
    Graph::parse_glsl_query(&query).unwrap().simplify()
});

static CALC_NORMAL_MAP_VAL_INF_XCX_Y: LazyLock<Graph> = LazyLock::new(|| {
    let query = calc_normal_map_val_inf_query('y');
    Graph::parse_glsl_query(&query).unwrap().simplify()
});

static CALC_NORMAL_MAP_VAL_INF_XCX_Z: LazyLock<Graph> = LazyLock::new(|| {
    let query = calc_normal_map_val_inf_query('z');
    Graph::parse_glsl_query(&query).unwrap().simplify()
});

pub fn calc_normal_map_val_inf<'a>(
    graph: &'a Graph,
    expr: &'a Expr,
) -> Option<([&'a Expr; 3], &'a Expr)> {
    let result = query_nodes(expr, graph, &CALC_NORMAL_MAP_VAL_INF_XCX_X)
        .or_else(|| query_nodes(expr, graph, &CALC_NORMAL_MAP_VAL_INF_XCX_Y))?;
    Some((
        [
            result.get("result_x")?,
            result.get("result_y")?,
            result.get("result_z")?,
        ],
        result.get("intensity")?,
    ))
}

fn calc_normal_map_xcx_query(c: char) -> String {
    formatdoc! {"
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
    "}
}

static CALC_NORMAL_MAP_XCX_X: LazyLock<Graph> = LazyLock::new(|| {
    let query = calc_normal_map_xcx_query('x');
    Graph::parse_glsl_query(&query).unwrap().simplify()
});

static CALC_NORMAL_MAP_XCX_Y: LazyLock<Graph> = LazyLock::new(|| {
    let query = calc_normal_map_xcx_query('y');
    Graph::parse_glsl_query(&query).unwrap().simplify()
});

static CALC_NORMAL_MAP_XCX_Z: LazyLock<Graph> = LazyLock::new(|| {
    let query = calc_normal_map_xcx_query('z');
    Graph::parse_glsl_query(&query).unwrap().simplify()
});

pub fn op_calc_normal_map<'a>(
    graph: &'a Graph,
    expr: &'a Expr,
) -> Option<(Operation, Vec<&'a Expr>)> {
    // TODO: Detect normal mapping from other games.
    let mut expr = expr;
    if let Some(new_expr) = normalize(graph, expr) {
        expr = new_expr;
    }

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
        result = 0.0 - glossiness;
        result = 1.0 + result;
        result = fma(result, result, temp);
        result = clamp(result, 0.0, 1.0);
        result = sqrt(result);
        result = 0.0 - result;
        result = result + 1.0;
    "};
    Graph::parse_glsl_query(query).unwrap().simplify()
});

pub fn geometric_specular_aa<'a>(graph: &'a Graph, expr: &'a Expr) -> Option<&'a Expr> {
    let result = query_nodes(expr, graph, &GEOMETRIC_SPECULAR_AA)?;
    result.get("glossiness").copied()
}

static SKIN_ATTRIBUTE_XYZ_X: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
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
    "};
    Graph::parse_glsl_query(query).unwrap().simplify()
});

static SKIN_ATTRIBUTE_XYZ_Y: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
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
    "};
    Graph::parse_glsl_query(query).unwrap().simplify()
});

static SKIN_ATTRIBUTE_XYZ_Z: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
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
    "};
    Graph::parse_glsl_query(query).unwrap().simplify()
});

static SKIN_ATTRIBUTE_XYZ_X2: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
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
    "};
    Graph::parse_glsl_query(query).unwrap().simplify()
});

static SKIN_ATTRIBUTE_XYZ_Y2: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
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
    "};
    Graph::parse_glsl_query(query).unwrap().simplify()
});

static SKIN_ATTRIBUTE_XYZ_Z2: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
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
    "};
    Graph::parse_glsl_query(query).unwrap().simplify()
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
    "};
    Graph::parse_glsl_query(query).unwrap().simplify()
});

static SKIN_ATTRIBUTE_XYZW_YZ: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
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
    "};
    Graph::parse_glsl_query(query).unwrap().simplify()
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
    "};
    Graph::parse_glsl_query(query).unwrap().simplify()
});

static SKIN_ATTRIBUTE_CLIP_XYZW_Z: LazyLock<Graph> = LazyLock::new(|| {
    // TODO: Detect this as matrix multiplication and regular skinning?
    let query = indoc! {"
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
    "};
    Graph::parse_glsl_query(query).unwrap().simplify()
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

// TODO: reduce repetition?
static SKIN_ATTRIBUTE_BITANGENT_XC3_X: LazyLock<Graph> = LazyLock::new(|| {
    // TODO: This can be U_OdB (XC3) or U_Bone (XC1)
    let query = indoc! {"
        temp_0 = nWgtIdx.x;
        temp_2 = vNormal.x;
        temp_3 = vTan.x;
        temp_5 = vNormal.y;
        temp_6 = floatBitsToInt(temp_0) & 65535;
        temp_7 = temp_6 * 48;
        temp_8 = vTan.y;
        temp_9 = floatBitsToUint(temp_0) >> 16;
        temp_10 = int(temp_9) * 48;
        temp_11 = temp_10 << 16;
        temp_12 = temp_11 + temp_7;
        temp_13 = vNormal.z;
        temp_14 = vTan.z;
        temp_15 = temp_12 + 16;
        temp_18 = vTan.w;
        temp_19 = temp_12 + 32;
        temp_34 = uint(temp_15) >> 2;
        temp_35 = uintBitsToFloat(U_OdB.data[int(temp_34)]);
        temp_36 = temp_15 + 4;
        temp_37 = uint(temp_36) >> 2;
        temp_38 = uintBitsToFloat(U_OdB.data[int(temp_37)]);
        temp_39 = temp_15 + 8;
        temp_40 = uint(temp_39) >> 2;
        temp_41 = uintBitsToFloat(U_OdB.data[int(temp_40)]);
        temp_46 = uint(temp_19) >> 2;
        temp_47 = uintBitsToFloat(U_OdB.data[int(temp_46)]);
        temp_48 = temp_19 + 4;
        temp_49 = uint(temp_48) >> 2;
        temp_50 = uintBitsToFloat(U_OdB.data[int(temp_49)]);
        temp_51 = temp_19 + 8;
        temp_52 = uint(temp_51) >> 2;
        temp_53 = uintBitsToFloat(U_OdB.data[int(temp_52)]);
        temp_94 = temp_35 * temp_2;
        temp_95 = temp_35 * temp_3;
        temp_100 = temp_47 * temp_2;
        temp_104 = temp_47 * temp_3;
        temp_107 = fma(temp_38, temp_5, temp_94);
        temp_109 = fma(temp_50, temp_5, temp_100);
        temp_110 = fma(temp_38, temp_8, temp_95);
        temp_113 = fma(temp_50, temp_8, temp_104);
        temp_114 = fma(temp_53, temp_13, temp_109);
        temp_115 = fma(temp_41, temp_14, temp_110);
        temp_117 = fma(temp_41, temp_13, temp_107);
        temp_118 = fma(temp_53, temp_14, temp_113);
        temp_119 = temp_114 * temp_115;
        temp_126 = 0.0 - temp_119;
        temp_127 = fma(temp_118, temp_117, temp_126);
        temp_177 = temp_127 * temp_18;
    "};
    Graph::parse_glsl_query(query).unwrap().simplify()
});

static SKIN_ATTRIBUTE_BITANGENT_XC3_Y: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
        temp_0 = nWgtIdx.x;
        temp_2 = vNormal.x;
        temp_3 = vTan.x;
        temp_5 = vNormal.y;
        temp_6 = floatBitsToInt(temp_0) & 65535;
        temp_7 = temp_6 * 48;
        temp_8 = vTan.y;
        temp_9 = floatBitsToUint(temp_0) >> 16;
        temp_10 = int(temp_9) * 48;
        temp_11 = temp_10 << 16;
        temp_12 = temp_11 + temp_7;
        temp_13 = vNormal.z;
        temp_14 = vTan.z;
        temp_18 = vTan.w;
        temp_19 = temp_12 + 32;
        temp_46 = uint(temp_19) >> 2;
        temp_47 = uintBitsToFloat(U_OdB.data[int(temp_46)]);
        temp_48 = temp_19 + 4;
        temp_49 = uint(temp_48) >> 2;
        temp_50 = uintBitsToFloat(U_OdB.data[int(temp_49)]);
        temp_51 = temp_19 + 8;
        temp_52 = uint(temp_51) >> 2;
        temp_53 = uintBitsToFloat(U_OdB.data[int(temp_52)]);
        temp_58 = uint(temp_12) >> 2;
        temp_59 = uintBitsToFloat(U_OdB.data[int(temp_58)]);
        temp_60 = temp_12 + 4;
        temp_61 = uint(temp_60) >> 2;
        temp_62 = uintBitsToFloat(U_OdB.data[int(temp_61)]);
        temp_63 = temp_12 + 8;
        temp_64 = uint(temp_63) >> 2;
        temp_65 = uintBitsToFloat(U_OdB.data[int(temp_64)]);
        temp_96 = temp_59 * temp_2;
        temp_98 = temp_59 * temp_3;
        temp_99 = fma(temp_62, temp_5, temp_96);
        temp_100 = temp_47 * temp_2;
        temp_103 = fma(temp_62, temp_8, temp_98);
        temp_104 = temp_47 * temp_3;
        temp_109 = fma(temp_50, temp_5, temp_100);
        temp_113 = fma(temp_50, temp_8, temp_104);
        temp_114 = fma(temp_53, temp_13, temp_109);
        temp_118 = fma(temp_53, temp_14, temp_113);
        temp_120 = fma(temp_65, temp_14, temp_103);
        temp_123 = fma(temp_65, temp_13, temp_99);
        temp_132 = temp_118 * temp_123;
        temp_139 = 0.0 - temp_132;
        temp_140 = fma(temp_114, temp_120, temp_139);
        temp_152 = temp_140 * temp_18;
    "};
    Graph::parse_glsl_query(query).unwrap().simplify()
});

static SKIN_ATTRIBUTE_BITANGENT_XC3_Z: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
        temp_0 = nWgtIdx.x;
        temp_2 = vNormal.x;
        temp_3 = vTan.x;
        temp_5 = vNormal.y;
        temp_6 = floatBitsToInt(temp_0) & 65535;
        temp_7 = temp_6 * 48;
        temp_8 = vTan.y;
        temp_9 = floatBitsToUint(temp_0) >> 16;
        temp_10 = int(temp_9) * 48;
        temp_11 = temp_10 << 16;
        temp_12 = temp_11 + temp_7;
        temp_13 = vNormal.z;
        temp_14 = vTan.z;
        temp_15 = temp_12 + 16;
        temp_18 = vTan.w;
        temp_34 = uint(temp_15) >> 2;
        temp_35 = uintBitsToFloat(U_OdB.data[int(temp_34)]);
        temp_36 = temp_15 + 4;
        temp_37 = uint(temp_36) >> 2;
        temp_38 = uintBitsToFloat(U_OdB.data[int(temp_37)]);
        temp_39 = temp_15 + 8;
        temp_40 = uint(temp_39) >> 2;
        temp_41 = uintBitsToFloat(U_OdB.data[int(temp_40)]);
        temp_58 = uint(temp_12) >> 2;
        temp_59 = uintBitsToFloat(U_OdB.data[int(temp_58)]);
        temp_60 = temp_12 + 4;
        temp_61 = uint(temp_60) >> 2;
        temp_62 = uintBitsToFloat(U_OdB.data[int(temp_61)]);
        temp_63 = temp_12 + 8;
        temp_64 = uint(temp_63) >> 2;
        temp_65 = uintBitsToFloat(U_OdB.data[int(temp_64)]);
        temp_94 = temp_35 * temp_2;
        temp_95 = temp_35 * temp_3;
        temp_96 = temp_59 * temp_2;
        temp_98 = temp_59 * temp_3;
        temp_99 = fma(temp_62, temp_5, temp_96);
        temp_103 = fma(temp_62, temp_8, temp_98);
        temp_107 = fma(temp_38, temp_5, temp_94);
        temp_110 = fma(temp_38, temp_8, temp_95);
        temp_115 = fma(temp_41, temp_14, temp_110);
        temp_117 = fma(temp_41, temp_13, temp_107);
        temp_120 = fma(temp_65, temp_14, temp_103);
        temp_123 = fma(temp_65, temp_13, temp_99);
        temp_128 = temp_117 * temp_120;
        temp_133 = 0.0 - temp_128;
        temp_134 = fma(temp_115, temp_123, temp_133);
        temp_156 = temp_134 * temp_18;
    "};
    Graph::parse_glsl_query(query).unwrap().simplify()
});

pub fn skin_attribute_bitangent(graph: &Graph, expr: &Expr) -> Option<Expr> {
    let channel = query_nodes(expr, graph, &SKIN_ATTRIBUTE_BITANGENT_XC3_X)
        .map(|_| 'x')
        .or_else(|| query_nodes(expr, graph, &SKIN_ATTRIBUTE_BITANGENT_XC3_Y).map(|_| 'y'))
        .or_else(|| query_nodes(expr, graph, &SKIN_ATTRIBUTE_BITANGENT_XC3_Z).map(|_| 'z'))?;
    Some(Expr::Global {
        name: "vBitan".into(),
        channel: Some(channel),
    })
}

pub fn attribute_gm_cal_xyz<'a>(graph: &'a Graph, expr: &'a Expr) -> Option<&'a Expr> {
    // vGmCal1.xyz, vGmCal2.xyz, vGmCal3.xyz make up the 3x3 instance transform matrix.
    // TODO: Add a way for queries to match identifiers exactly like "#vGmCal1"?
    let (a, b) = dot3_a_b(graph, expr)?;
    match (a, b) {
        (
            [
                Expr::Global {
                    name: n1,
                    channel: Some('x'),
                },
                Expr::Global {
                    name: n2,
                    channel: Some('y'),
                },
                Expr::Global {
                    name: n3,
                    channel: Some('z'),
                },
            ],
            [x, y, z],
        ) => {
            // TODO: find a nicer way of writing this
            if n1 == n2 && n2 == n3 {
                if n1 == "vGmCal1" {
                    Some(x)
                } else if n1 == "vGmCal2" {
                    Some(y)
                } else if n1 == "vGmCal3" {
                    Some(z)
                } else {
                    None
                }
            } else {
                None
            }
        }
        (
            [x, y, z],
            [
                Expr::Global {
                    name: n1,
                    channel: Some('x'),
                },
                Expr::Global {
                    name: n2,
                    channel: Some('y'),
                },
                Expr::Global {
                    name: n3,
                    channel: Some('z'),
                },
            ],
        ) => {
            if n1 == n2 && n2 == n3 {
                if n1 == "vGmCal1" {
                    Some(x)
                } else if n1 == "vGmCal2" {
                    Some(y)
                } else if n1 == "vGmCal3" {
                    Some(z)
                } else {
                    None
                }
            } else {
                None
            }
        }
        _ => None,
    }
}

static BITANGENT_GM_CAL_XYZ: LazyLock<Graph> = LazyLock::new(|| {
    // The channels differ only in their gmCal names.
    let query = indoc! {"
        temp_5 = vGmCal_A.x;
        temp_6 = vGmCal_B.x;
        temp_7 = vNormal.x;
        temp_8 = vTan.x;
        temp_9 = vGmCal_B.y;
        temp_10 = vNormal.y;
        temp_14 = vGmCal_A.y;
        temp_15 = vTan.y;
        temp_17 = vGmCal_A.z;
        temp_18 = vTan.z;
        temp_22 = vGmCal_B.z;
        temp_24 = vNormal.z;
        temp_29 = vTan.w;
        temp_35 = temp_6 * temp_8;
        temp_36 = temp_6 * temp_7;
        temp_37 = temp_5 * temp_7;
        temp_39 = temp_5 * temp_8;
        temp_41 = fma(temp_9, temp_10, temp_36);
        temp_44 = fma(temp_14, temp_10, temp_37);
        temp_46 = fma(temp_9, temp_15, temp_35);
        temp_47 = fma(temp_14, temp_15, temp_39);
        temp_50 = fma(temp_22, temp_24, temp_41);
        temp_52 = fma(temp_17, temp_18, temp_47);
        temp_53 = fma(temp_22, temp_18, temp_46);
        temp_55 = fma(temp_17, temp_24, temp_44);
        temp_58 = temp_50 * temp_52;
        temp_62 = 0.0 - temp_58;
        temp_63 = fma(temp_55, temp_53, temp_62);
        temp_70 = temp_63 * temp_29;
    "};
    Graph::parse_glsl_query(query).unwrap().simplify()
});

pub fn bitangent_gm_cal_xyz(graph: &Graph, expr: &Expr) -> Option<Expr> {
    // vGmCal1.xyz, vGmCal2.xyz, vGmCal3.xyz make up the 3x3 instance transform matrix.
    let result = query_nodes(expr, graph, &BITANGENT_GM_CAL_XYZ)?;

    let a = result.get("vGmCal_A")?;
    let b = result.get("vGmCal_B")?;

    let channel = match (a, b) {
        (Expr::Global { name: n1, .. }, Expr::Global { name: n2, .. }) => {
            match (n1.as_str(), n2.as_str()) {
                ("vGmCal2", "vGmCal3") => Some('x'),
                ("vGmCal3", "vGmCal1") => Some('y'),
                ("vGmCal1", "vGmCal2") => Some('z'),
                _ => None,
            }
        }
        _ => None,
    }?;
    Some(Expr::Global {
        name: "vBitan".into(),
        channel: Some(channel),
    })
}

// TODO: Detect gmProj separately.
static GM_CAL_CLIP_ATTRIBUTE_XYZW_X: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
        temp_3 = vGmCal1.x;
        temp_4 = result_x;
        temp_5 = vGmCal2.x;
        temp_6 = vGmCal3.x;
        temp_9 = vGmCal3.y;
        temp_12 = vGmCal1.y;
        temp_13 = result_y;
        temp_14 = vGmCal2.y;
        temp_16 = result_z;
        temp_17 = vGmCal2.z;
        temp_20 = vGmCal1.z;
        temp_22 = vGmCal3.z;
        temp_23 = temp_3 * temp_4;
        temp_25 = vGmCal1.w;
        temp_26 = result_w;
        temp_27 = vGmCal2.w;
        temp_28 = vGmCal3.w;
        temp_33 = temp_4 * temp_6;
        temp_34 = temp_4 * temp_5;
        temp_42 = fma(temp_12, temp_13, temp_23);
        temp_43 = fma(temp_13, temp_9, temp_33);
        temp_45 = fma(temp_13, temp_14, temp_34);
        temp_51 = fma(temp_16, temp_17, temp_45);
        temp_57 = fma(temp_20, temp_16, temp_42);
        temp_61 = fma(temp_25, temp_26, temp_57);
        temp_68 = fma(temp_16, temp_22, temp_43);
        temp_69 = fma(temp_26, temp_27, temp_51);
        temp_74 = fma(temp_26, temp_28, temp_68);
        temp_82 = temp_61 * U_Static.gmProj[0].x;
        temp_89 = fma(temp_69, U_Static.gmProj[0].y, temp_82);
        temp_95 = fma(temp_74, U_Static.gmProj[0].z, temp_89);
        temp_105 = temp_95 + U_Static.gmProj[0].w;
    "};
    Graph::parse_glsl_query(query).unwrap().simplify()
});

static GM_CAL_CLIP_ATTRIBUTE_XYZW_Y: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
        temp_3 = vGmCal1.x;
        temp_4 = result_x;
        temp_5 = vGmCal2.x;
        temp_6 = vGmCal3.x;
        temp_9 = vGmCal3.y;
        temp_12 = vGmCal1.y;
        temp_13 = result_y;
        temp_14 = vGmCal2.y;
        temp_16 = result_z;
        temp_17 = vGmCal2.z;
        temp_20 = vGmCal1.z;
        temp_22 = vGmCal3.z;
        temp_23 = temp_3 * temp_4;
        temp_25 = vGmCal1.w;
        temp_26 = result_w;
        temp_27 = vGmCal2.w;
        temp_28 = vGmCal3.w;
        temp_33 = temp_4 * temp_6;
        temp_34 = temp_4 * temp_5;
        temp_42 = fma(temp_12, temp_13, temp_23);
        temp_43 = fma(temp_13, temp_9, temp_33);
        temp_45 = fma(temp_13, temp_14, temp_34);
        temp_51 = fma(temp_16, temp_17, temp_45);
        temp_57 = fma(temp_20, temp_16, temp_42);
        temp_61 = fma(temp_25, temp_26, temp_57);
        temp_68 = fma(temp_16, temp_22, temp_43);
        temp_69 = fma(temp_26, temp_27, temp_51);
        temp_73 = temp_61 * U_Static.gmProj[1].x;
        temp_74 = fma(temp_26, temp_28, temp_68);
        temp_75 = fma(temp_69, U_Static.gmProj[1].y, temp_73);
        temp_77 = fma(temp_74, U_Static.gmProj[1].z, temp_75);
        temp_106 = temp_77 + U_Static.gmProj[1].w;
    "};
    Graph::parse_glsl_query(query).unwrap().simplify()
});

static GM_CAL_CLIP_ATTRIBUTE_XYZW_Z: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
        temp_3 = vGmCal1.x;
        temp_4 = result_x;
        temp_5 = vGmCal2.x;
        temp_6 = vGmCal3.x;
        temp_9 = vGmCal3.y;
        temp_12 = vGmCal1.y;
        temp_13 = result_y;
        temp_14 = vGmCal2.y;
        temp_16 = result_z;
        temp_17 = vGmCal2.z;
        temp_20 = vGmCal1.z;
        temp_22 = vGmCal3.z;
        temp_23 = temp_3 * temp_4;
        temp_25 = vGmCal1.w;
        temp_26 = result_w;
        temp_27 = vGmCal2.w;
        temp_28 = vGmCal3.w;
        temp_33 = temp_4 * temp_6;
        temp_34 = temp_4 * temp_5;
        temp_42 = fma(temp_12, temp_13, temp_23);
        temp_43 = fma(temp_13, temp_9, temp_33);
        temp_45 = fma(temp_13, temp_14, temp_34);
        temp_51 = fma(temp_16, temp_17, temp_45);
        temp_57 = fma(temp_20, temp_16, temp_42);
        temp_61 = fma(temp_25, temp_26, temp_57);
        temp_68 = fma(temp_16, temp_22, temp_43);
        temp_69 = fma(temp_26, temp_27, temp_51);
        temp_74 = fma(temp_26, temp_28, temp_68);
        temp_78 = temp_61 * U_Static.gmProj[2].x;
        temp_81 = fma(temp_69, U_Static.gmProj[2].y, temp_78);
        temp_87 = fma(temp_74, U_Static.gmProj[2].z, temp_81);
        temp_93 = temp_87 + U_Static.gmProj[2].w;
        temp_100 = 0.0 - U_Static.gCDep.y;
        temp_101 = temp_93 + temp_100;
        temp_111 = temp_101 * U_Static.gCDep.z;
    "};
    Graph::parse_glsl_query(query).unwrap().simplify()
});

static GM_CAL_CLIP_ATTRIBUTE_XYZW_W: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
        temp_3 = vGmCal1.x;
        temp_4 = result_x;
        temp_5 = vGmCal2.x;
        temp_6 = vGmCal3.x;
        temp_9 = vGmCal3.y;
        temp_12 = vGmCal1.y;
        temp_13 = result_y;
        temp_14 = vGmCal2.y;
        temp_16 = result_z;
        temp_17 = vGmCal2.z;
        temp_20 = vGmCal1.z;
        temp_22 = vGmCal3.z;
        temp_23 = temp_3 * temp_4;
        temp_25 = vGmCal1.w;
        temp_26 = result_w;
        temp_27 = vGmCal2.w;
        temp_28 = vGmCal3.w;
        temp_33 = temp_4 * temp_6;
        temp_34 = temp_4 * temp_5;
        temp_42 = fma(temp_12, temp_13, temp_23);
        temp_43 = fma(temp_13, temp_9, temp_33);
        temp_45 = fma(temp_13, temp_14, temp_34);
        temp_51 = fma(temp_16, temp_17, temp_45);
        temp_57 = fma(temp_20, temp_16, temp_42);
        temp_61 = fma(temp_25, temp_26, temp_57);
        temp_68 = fma(temp_16, temp_22, temp_43);
        temp_69 = fma(temp_26, temp_27, temp_51);
        temp_74 = fma(temp_26, temp_28, temp_68);
        temp_76 = temp_61 * U_Static.gmProj[3].x;
        temp_79 = fma(temp_69, U_Static.gmProj[3].y, temp_76);
        temp_88 = fma(temp_74, U_Static.gmProj[3].z, temp_79);
        temp_94 = temp_88 + U_Static.gmProj[3].w;
    "};
    Graph::parse_glsl_query(query).unwrap().simplify()
});

static GM_CAL_CLIP_ATTRIBUTE_XYZW: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
        temp_5 = result.x;
        temp_9 = result.y;
        temp_13 = result.z;
        temp_17 = result.w;
        temp_26 = vGmCal.x;
        temp_27 = vGmCal.y;
        temp_28 = vGmCal.z;
        temp_29 = vGmCal.w;
        temp_75 = temp_5 * temp_26;
        temp_80 = fma(temp_9, temp_27, temp_75);
        temp_86 = fma(temp_13, temp_28, temp_80);
        temp_92 = fma(temp_17, temp_29, temp_86);
    "};
    Graph::parse_glsl_query(query).unwrap().simplify()
});

pub fn gm_cal_clip_attribute_xyzw<'a>(graph: &'a Graph, expr: &'a Expr) -> Option<Expr> {
    // TODO: This should match the attribute names exactly to be able to return &Expr?
    // TODO: Don't assume vPos?
    query_nodes(expr, graph, &GM_CAL_CLIP_ATTRIBUTE_XYZW)
        .and_then(|result| {
            // TODO: Detect names in the query itself to make this simpler.
            let gm_cal = result.get("vGmCal")?;
            let pos = result.get("result")?;
            if let Expr::Global { name, .. } = gm_cal {
                gm_cal_position(name).or_else(|| {
                    if let Expr::Global { name, .. } = pos {
                        gm_cal_position(name)
                    } else {
                        None
                    }
                })
            } else {
                None
            }
        })
        .or_else(|| {
            query_nodes(expr, graph, &GM_CAL_CLIP_ATTRIBUTE_XYZW_X)
                .map(|_| Expr::Global {
                    name: "vPos".into(),
                    channel: Some('x'),
                })
                .or_else(|| {
                    query_nodes(expr, graph, &GM_CAL_CLIP_ATTRIBUTE_XYZW_Y).map(|_| Expr::Global {
                        name: "vPos".into(),
                        channel: Some('y'),
                    })
                })
                .or_else(|| {
                    query_nodes(expr, graph, &GM_CAL_CLIP_ATTRIBUTE_XYZW_Z).map(|_| Expr::Global {
                        name: "vPos".into(),
                        channel: Some('z'),
                    })
                })
                .or_else(|| {
                    query_nodes(expr, graph, &GM_CAL_CLIP_ATTRIBUTE_XYZW_W).map(|_| Expr::Global {
                        name: "vPos".into(),
                        channel: Some('w'),
                    })
                })
        })
}

fn gm_cal_position(name: &str) -> Option<Expr> {
    match name {
        "vGmCal1" => Some(Expr::Global {
            name: "vPos".into(),
            channel: Some('x'),
        }),
        "vGmCal2" => Some(Expr::Global {
            name: "vPos".into(),
            channel: Some('y'),
        }),
        "vGmCal3" => Some(Expr::Global {
            name: "vPos".into(),
            channel: Some('z'),
        }),
        _ => None,
    }
}

// TODO: Detect gmProj separately.
static GM_CAL_U_BILL_ATTRIBUTE_XYZW_X: LazyLock<Graph> = LazyLock::new(|| {
    // xc2/ma02a/prop/1/slct48_nvsd0_shd0048.vert
    let query = indoc! {"
        temp_0 = nWgtIdx.x;
        temp_1 = floatBitsToInt(temp_0) << 4;
        temp_2 = vGmCal1.x;
        temp_3 = uint(temp_1) >> 2;
        temp_5 = int(temp_3) & 3;
        temp_6 = U_BILL.data[int(temp_4)][temp_5];
        temp_7 = int(temp_3) + 1;
        temp_9 = temp_7 & 3;
        temp_10 = U_BILL.data[int(temp_8)][temp_9];
        temp_11 = temp_1 + 8;
        temp_12 = uint(temp_11) >> 2;
        temp_14 = int(temp_12) & 3;
        temp_15 = U_BILL.data[int(temp_13)][temp_14];
        temp_16 = vGmCal1.y;
        temp_17 = vGmCal2.x;
        temp_18 = vGmCal3.x;
        temp_19 = vGmCal2.y;
        temp_20 = vGmCal1.z;
        temp_21 = vGmCal3.y;
        temp_22 = vGmCal2.z;
        temp_23 = vGmCal1.w;
        temp_24 = vGmCal3.z;
        temp_25 = vGmCal2.w;
        temp_26 = vGmCal3.w;
        temp_27 = temp_6 * temp_2;
        temp_29 = temp_6 * temp_17;
        temp_30 = fma(temp_10, temp_16, temp_27);
        temp_31 = temp_6 * temp_18;
        temp_32 = fma(temp_10, temp_19, temp_29);
        temp_34 = fma(temp_15, temp_20, temp_30);
        temp_35 = fma(temp_10, temp_21, temp_31);
        temp_36 = vColor.w;
        temp_37 = fma(temp_15, temp_22, temp_32);
        temp_38 = vPos.z;
        temp_39 = temp_34 + temp_23;
        temp_40 = vPos.y;
        temp_41 = fma(temp_15, temp_24, temp_35);
        temp_42 = temp_37 + temp_25;
        temp_43 = temp_39 * temp_39;
        temp_44 = temp_41 + temp_26;
        temp_45 = vPos.x;
        temp_46 = fma(temp_42, temp_42, temp_43);
        temp_47 = temp_2 * temp_2;
        temp_49 = fma(temp_44, temp_44, temp_46);
        temp_50 = sqrt(temp_49);
        temp_51 = fma(temp_17, temp_17, temp_47);
        temp_52 = fma(temp_18, temp_18, temp_51);
        temp_54 = temp_50 * U_Mate.gWrkFl4[1].x;
        temp_55 = sqrt(temp_52);
        temp_56 = 0.0 - U_Mate.gWrkFl4[0].x;
        temp_57 = temp_50 + temp_56;
        temp_58 = temp_50 * U_Mate.gWrkFl4[1].y;
        temp_59 = fma(temp_54, temp_45, temp_45);
        temp_61 = fma(temp_58, temp_40, temp_40);
        temp_62 = abs(temp_57);
        temp_63 = 0.0 - temp_62;
        temp_64 = temp_63 + -0.0;
        temp_65 = fma(temp_59, temp_55, temp_39);
        temp_66 = fma(temp_61, temp_55, temp_42);
        temp_67 = fma(temp_64, U_Mate.gWrkFl4[0].y, 1.0);
        temp_68 = clamp(temp_67, 0.0, 1.0);
        temp_70 = temp_44 + U_Mate.gWrkFl4[0].z;
        temp_71 = temp_68 * temp_36;
        temp_73 = temp_70 * U_Mate.gWrkFl4[0].w;
        temp_74 = clamp(temp_73, 0.0, 1.0);
        temp_76 = temp_65 * U_Static.gmProj[0].x;
        temp_77 = fma(temp_55, temp_38, temp_44);
        temp_79 = 0.0 - temp_74;
        temp_80 = fma(temp_71, temp_79, temp_71);
        temp_82 = fma(temp_66, U_Static.gmProj[0].y, temp_76);
        temp_86 = fma(temp_77, U_Static.gmProj[0].z, temp_82);
        temp_89 = temp_80 <= vp_c1.data[0].x;
        temp_90 = temp_89 ? 1.0 : 0.0;
        temp_91 = temp_86 + U_Static.gmProj[0].w;
        temp_96 = 0.0 - vp_c1.data[0].y;
        temp_97 = fma(temp_90, temp_96, temp_91);
    "};
    Graph::parse_glsl_query(query).unwrap().simplify()
});

static GM_CAL_U_BILL_ATTRIBUTE_XYZW_Y: LazyLock<Graph> = LazyLock::new(|| {
    // xc2/ma02a/prop/1/slct48_nvsd0_shd0048.vert
    let query = indoc! {"
        temp_0 = nWgtIdx.x;
        temp_1 = floatBitsToInt(temp_0) << 4;
        temp_2 = vGmCal1.x;
        temp_3 = uint(temp_1) >> 2;
        temp_5 = int(temp_3) & 3;
        temp_6 = U_BILL.data[int(temp_4)][temp_5];
        temp_7 = int(temp_3) + 1;
        temp_9 = temp_7 & 3;
        temp_10 = U_BILL.data[int(temp_8)][temp_9];
        temp_11 = temp_1 + 8;
        temp_12 = uint(temp_11) >> 2;
        temp_14 = int(temp_12) & 3;
        temp_15 = U_BILL.data[int(temp_13)][temp_14];
        temp_16 = vGmCal1.y;
        temp_17 = vGmCal2.x;
        temp_18 = vGmCal3.x;
        temp_19 = vGmCal2.y;
        temp_20 = vGmCal1.z;
        temp_21 = vGmCal3.y;
        temp_22 = vGmCal2.z;
        temp_23 = vGmCal1.w;
        temp_24 = vGmCal3.z;
        temp_25 = vGmCal2.w;
        temp_26 = vGmCal3.w;
        temp_27 = temp_6 * temp_2;
        temp_29 = temp_6 * temp_17;
        temp_30 = fma(temp_10, temp_16, temp_27);
        temp_31 = temp_6 * temp_18;
        temp_32 = fma(temp_10, temp_19, temp_29);
        temp_34 = fma(temp_15, temp_20, temp_30);
        temp_35 = fma(temp_10, temp_21, temp_31);
        temp_36 = vColor.w;
        temp_37 = fma(temp_15, temp_22, temp_32);
        temp_38 = vPos.z;
        temp_39 = temp_34 + temp_23;
        temp_40 = vPos.y;
        temp_41 = fma(temp_15, temp_24, temp_35);
        temp_42 = temp_37 + temp_25;
        temp_43 = temp_39 * temp_39;
        temp_44 = temp_41 + temp_26;
        temp_45 = vPos.x;
        temp_46 = fma(temp_42, temp_42, temp_43);
        temp_47 = temp_2 * temp_2;
        temp_49 = fma(temp_44, temp_44, temp_46);
        temp_50 = sqrt(temp_49);
        temp_51 = fma(temp_17, temp_17, temp_47);
        temp_52 = fma(temp_18, temp_18, temp_51);
        temp_54 = temp_50 * U_Mate.gWrkFl4[1].x;
        temp_55 = sqrt(temp_52);
        temp_56 = 0.0 - U_Mate.gWrkFl4[0].x;
        temp_57 = temp_50 + temp_56;
        temp_58 = temp_50 * U_Mate.gWrkFl4[1].y;
        temp_59 = fma(temp_54, temp_45, temp_45);
        temp_61 = fma(temp_58, temp_40, temp_40);
        temp_62 = abs(temp_57);
        temp_63 = 0.0 - temp_62;
        temp_64 = temp_63 + -0.0;
        temp_65 = fma(temp_59, temp_55, temp_39);
        temp_66 = fma(temp_61, temp_55, temp_42);
        temp_67 = fma(temp_64, U_Mate.gWrkFl4[0].y, 1.0);
        temp_68 = clamp(temp_67, 0.0, 1.0);
        temp_70 = temp_44 + U_Mate.gWrkFl4[0].z;
        temp_71 = temp_68 * temp_36;
        temp_73 = temp_70 * U_Mate.gWrkFl4[0].w;
        temp_74 = clamp(temp_73, 0.0, 1.0);
        temp_77 = fma(temp_55, temp_38, temp_44);
        temp_78 = temp_65 * U_Static.gmProj[1].x;
        temp_79 = 0.0 - temp_74;
        temp_80 = fma(temp_71, temp_79, temp_71);
        temp_83 = fma(temp_66, U_Static.gmProj[1].y, temp_78);
        temp_87 = fma(temp_77, U_Static.gmProj[1].z, temp_83);
        temp_89 = temp_80 <= vp_c1.data[0].x;
        temp_90 = temp_89 ? 1.0 : 0.0;
        temp_92 = temp_87 + U_Static.gmProj[1].w;
        temp_98 = 0.0 - vp_c1.data[0].y;
        temp_99 = fma(temp_90, temp_98, temp_92);
    "};
    Graph::parse_glsl_query(query).unwrap().simplify()
});

static GM_CAL_U_BILL_ATTRIBUTE_XYZW_Z: LazyLock<Graph> = LazyLock::new(|| {
    // xc2/ma02a/prop/1/slct48_nvsd0_shd0048.vert
    let query = indoc! {"
        temp_0 = nWgtIdx.x;
        temp_1 = floatBitsToInt(temp_0) << 4;
        temp_2 = vGmCal1.x;
        temp_3 = uint(temp_1) >> 2;
        temp_5 = int(temp_3) & 3;
        temp_6 = U_BILL.data[int(temp_4)][temp_5];
        temp_7 = int(temp_3) + 1;
        temp_9 = temp_7 & 3;
        temp_10 = U_BILL.data[int(temp_8)][temp_9];
        temp_11 = temp_1 + 8;
        temp_12 = uint(temp_11) >> 2;
        temp_14 = int(temp_12) & 3;
        temp_15 = U_BILL.data[int(temp_13)][temp_14];
        temp_16 = vGmCal1.y;
        temp_17 = vGmCal2.x;
        temp_18 = vGmCal3.x;
        temp_19 = vGmCal2.y;
        temp_20 = vGmCal1.z;
        temp_21 = vGmCal3.y;
        temp_22 = vGmCal2.z;
        temp_23 = vGmCal1.w;
        temp_24 = vGmCal3.z;
        temp_25 = vGmCal2.w;
        temp_26 = vGmCal3.w;
        temp_27 = temp_6 * temp_2;
        temp_29 = temp_6 * temp_17;
        temp_30 = fma(temp_10, temp_16, temp_27);
        temp_31 = temp_6 * temp_18;
        temp_32 = fma(temp_10, temp_19, temp_29);
        temp_34 = fma(temp_15, temp_20, temp_30);
        temp_35 = fma(temp_10, temp_21, temp_31);
        temp_37 = fma(temp_15, temp_22, temp_32);
        temp_38 = vPos.z;
        temp_39 = temp_34 + temp_23;
        temp_40 = vPos.y;
        temp_41 = fma(temp_15, temp_24, temp_35);
        temp_42 = temp_37 + temp_25;
        temp_43 = temp_39 * temp_39;
        temp_44 = temp_41 + temp_26;
        temp_45 = vPos.x;
        temp_46 = fma(temp_42, temp_42, temp_43);
        temp_47 = temp_2 * temp_2;
        temp_49 = fma(temp_44, temp_44, temp_46);
        temp_50 = sqrt(temp_49);
        temp_51 = fma(temp_17, temp_17, temp_47);
        temp_52 = fma(temp_18, temp_18, temp_51);
        temp_54 = temp_50 * U_Mate.gWrkFl4[1].x;
        temp_55 = sqrt(temp_52);
        temp_58 = temp_50 * U_Mate.gWrkFl4[1].y;
        temp_59 = fma(temp_54, temp_45, temp_45);
        temp_61 = fma(temp_58, temp_40, temp_40);
        temp_65 = fma(temp_59, temp_55, temp_39);
        temp_66 = fma(temp_61, temp_55, temp_42);
        temp_75 = temp_65 * U_Static.gmProj[2].x;
        temp_77 = fma(temp_55, temp_38, temp_44);
        temp_81 = fma(temp_66, U_Static.gmProj[2].y, temp_75);
        temp_84 = fma(temp_77, U_Static.gmProj[2].z, temp_81);
        temp_88 = temp_84 + U_Static.gmProj[2].w;
        temp_93 = 0.0 - U_Static.gCDep.y;
        temp_94 = temp_88 + temp_93;
        temp_100 = temp_94 * U_Static.gCDep.z;
    "};
    Graph::parse_glsl_query(query).unwrap().simplify()
});

static GM_CAL_U_BILL_ATTRIBUTE_XYZW_W: LazyLock<Graph> = LazyLock::new(|| {
    // xc2/ma02a/prop/1/slct48_nvsd0_shd0048.vert
    let query = indoc! {"
        temp_0 = nWgtIdx.x;
        temp_1 = floatBitsToInt(temp_0) << 4;
        temp_2 = vGmCal1.x;
        temp_3 = uint(temp_1) >> 2;
        temp_5 = int(temp_3) & 3;
        temp_6 = U_BILL.data[int(temp_4)][temp_5];
        temp_7 = int(temp_3) + 1;
        temp_9 = temp_7 & 3;
        temp_10 = U_BILL.data[int(temp_8)][temp_9];
        temp_11 = temp_1 + 8;
        temp_12 = uint(temp_11) >> 2;
        temp_14 = int(temp_12) & 3;
        temp_15 = U_BILL.data[int(temp_13)][temp_14];
        temp_16 = vGmCal1.y;
        temp_17 = vGmCal2.x;
        temp_18 = vGmCal3.x;
        temp_19 = vGmCal2.y;
        temp_20 = vGmCal1.z;
        temp_21 = vGmCal3.y;
        temp_22 = vGmCal2.z;
        temp_23 = vGmCal1.w;
        temp_24 = vGmCal3.z;
        temp_25 = vGmCal2.w;
        temp_26 = vGmCal3.w;
        temp_27 = temp_6 * temp_2;
        temp_29 = temp_6 * temp_17;
        temp_30 = fma(temp_10, temp_16, temp_27);
        temp_31 = temp_6 * temp_18;
        temp_32 = fma(temp_10, temp_19, temp_29);
        temp_34 = fma(temp_15, temp_20, temp_30);
        temp_35 = fma(temp_10, temp_21, temp_31);
        temp_37 = fma(temp_15, temp_22, temp_32);
        temp_38 = vPos.z;
        temp_39 = temp_34 + temp_23;
        temp_40 = vPos.y;
        temp_41 = fma(temp_15, temp_24, temp_35);
        temp_42 = temp_37 + temp_25;
        temp_43 = temp_39 * temp_39;
        temp_44 = temp_41 + temp_26;
        temp_45 = vPos.x;
        temp_46 = fma(temp_42, temp_42, temp_43);
        temp_47 = temp_2 * temp_2;
        temp_49 = fma(temp_44, temp_44, temp_46);
        temp_50 = sqrt(temp_49);
        temp_51 = fma(temp_17, temp_17, temp_47);
        temp_52 = fma(temp_18, temp_18, temp_51);
        temp_54 = temp_50 * U_Mate.gWrkFl4[1].x;
        temp_55 = sqrt(temp_52);
        temp_58 = temp_50 * U_Mate.gWrkFl4[1].y;
        temp_59 = fma(temp_54, temp_45, temp_45);
        temp_61 = fma(temp_58, temp_40, temp_40);
        temp_65 = fma(temp_59, temp_55, temp_39);
        temp_66 = fma(temp_61, temp_55, temp_42);
        temp_69 = temp_65 * U_Static.gmProj[3].x;
        temp_72 = fma(temp_66, U_Static.gmProj[3].y, temp_69);
        temp_77 = fma(temp_55, temp_38, temp_44);
        temp_85 = fma(temp_77, U_Static.gmProj[3].z, temp_72);
        temp_95 = temp_85 + U_Static.gmProj[3].w;
    "};
    Graph::parse_glsl_query(query).unwrap().simplify()
});

pub fn gm_cal_u_bill_attribute_xyzw<'a>(graph: &'a Graph, expr: &'a Expr) -> Option<Expr> {
    // TODO: This should match the attribute names exactly to be able to return &Expr?
    // TODO: Don't assume vPos?
    query_nodes(expr, graph, &GM_CAL_U_BILL_ATTRIBUTE_XYZW_X)
        .map(|_| Expr::Global {
            name: "vPos".into(),
            channel: Some('x'),
        })
        .or_else(|| {
            query_nodes(expr, graph, &GM_CAL_U_BILL_ATTRIBUTE_XYZW_Y).map(|_| Expr::Global {
                name: "vPos".into(),
                channel: Some('y'),
            })
        })
        .or_else(|| {
            query_nodes(expr, graph, &GM_CAL_U_BILL_ATTRIBUTE_XYZW_Z).map(|_| Expr::Global {
                name: "vPos".into(),
                channel: Some('z'),
            })
        })
        .or_else(|| {
            query_nodes(expr, graph, &GM_CAL_U_BILL_ATTRIBUTE_XYZW_W).map(|_| Expr::Global {
                name: "vPos".into(),
                channel: Some('w'),
            })
        })
}

// TODO: Detect gmProj separately.
static GM_CAL_U_NAM_ATTRIBUTE_XYZW_X: LazyLock<Graph> = LazyLock::new(|| {
    // xc2/ma01a/prop/1/slct18_nvsd0_shd0018.vert
    let query = indoc! {"
        temp_0 = nWgtIdx.y;
        temp_1 = intBitsToFloat(gl_InstanceID);
        temp_2 = trunc(U_NAM.gAnmIdx.x);
        temp_3 = int(temp_2);
        temp_4 = temp_3 & 65535;
        temp_5 = floatBitsToInt(temp_1) & 65535;
        temp_6 = temp_4 * temp_5;
        temp_7 = temp_6 + floatBitsToInt(temp_0);
        temp_9 = temp_3 & 65535;
        temp_10 = floatBitsToUint(temp_1) >> 16;
        temp_11 = temp_9 * int(temp_10);
        temp_12 = temp_11 & 65535;
        temp_13 = floatBitsToInt(temp_1) << 16;
        temp_14 = temp_12 | temp_13;
        temp_16 = uint(temp_3) >> 16;
        temp_17 = uint(temp_14) >> 16;
        temp_18 = int(temp_16) * int(temp_17);
        temp_19 = temp_18 << 16;
        temp_20 = temp_14 << 16;
        temp_21 = temp_7 + temp_20;
        temp_22 = temp_19 + temp_21;
        temp_25 = temp_22 & 65535;
        temp_26 = temp_25 * 48;
        temp_27 = vPos.x;
        temp_28 = uint(temp_22) >> 16;
        temp_29 = int(temp_28) * 48;
        temp_30 = temp_29 << 16;
        temp_31 = temp_30 + temp_26;
        temp_34 = temp_31 + 16;
        temp_35 = uint(temp_34) >> 2;
        temp_37 = int(temp_35) & 3;
        temp_38 = U_NAM.data[int(temp_36)][temp_37];
        temp_39 = int(temp_35) + 1;
        temp_41 = temp_39 & 3;
        temp_42 = U_NAM.data[int(temp_40)][temp_41];
        temp_45 = temp_31 + 48;
        temp_46 = uint(temp_45) >> 2;
        temp_48 = int(temp_46) & 3;
        temp_49 = U_NAM.data[int(temp_47)][temp_48];
        temp_50 = int(temp_46) + 1;
        temp_52 = temp_50 & 3;
        temp_53 = U_NAM.data[int(temp_51)][temp_52];
        temp_58 = temp_31 + 32;
        temp_59 = uint(temp_58) >> 2;
        temp_61 = int(temp_59) & 3;
        temp_62 = U_NAM.data[int(temp_60)][temp_61];
        temp_63 = int(temp_59) + 1;
        temp_65 = temp_63 & 3;
        temp_66 = U_NAM.data[int(temp_64)][temp_65];
        temp_77 = vPos.y;
        temp_96 = temp_31 + 24;
        temp_97 = uint(temp_96) >> 2;
        temp_99 = int(temp_97) & 3;
        temp_100 = U_NAM.data[int(temp_98)][temp_99];
        temp_101 = int(temp_97) + 1;
        temp_103 = temp_101 & 3;
        temp_104 = U_NAM.data[int(temp_102)][temp_103];
        temp_105 = temp_31 + 56;
        temp_106 = uint(temp_105) >> 2;
        temp_108 = int(temp_106) & 3;
        temp_109 = U_NAM.data[int(temp_107)][temp_108];
        temp_110 = int(temp_106) + 1;
        temp_112 = temp_110 & 3;
        temp_113 = U_NAM.data[int(temp_111)][temp_112];
        temp_115 = temp_31 + 40;
        temp_116 = uint(temp_115) >> 2;
        temp_118 = int(temp_116) & 3;
        temp_119 = U_NAM.data[int(temp_117)][temp_118];
        temp_120 = int(temp_116) + 1;
        temp_122 = temp_120 & 3;
        temp_123 = U_NAM.data[int(temp_121)][temp_122];
        temp_127 = vPos.z;
        temp_130 = temp_49 * temp_27;
        temp_131 = vPos.w;
        temp_133 = vGmCal2.x;
        temp_139 = temp_62 * temp_27;
        temp_143 = vGmCal1.y;
        temp_144 = fma(temp_66, temp_77, temp_139);
        temp_154 = temp_38 * temp_27;
        temp_165 = fma(temp_42, temp_77, temp_154);
        temp_176 = vGmCal2.y;
        temp_178 = vGmCal3.x;
        temp_179 = fma(temp_53, temp_77, temp_130);
        temp_180 = vGmCal1.x;
        temp_183 = vGmCal3.y;
        temp_185 = vGmCal2.z;
        temp_186 = fma(temp_109, temp_127, temp_179);
        temp_190 = fma(temp_100, temp_127, temp_165);
        temp_191 = vGmCal3.z;
        temp_192 = fma(temp_119, temp_127, temp_144);
        temp_194 = vGmCal1.z;
        temp_201 = fma(temp_123, temp_131, temp_192);
        temp_205 = fma(temp_113, temp_131, temp_186);
        temp_207 = fma(temp_104, temp_131, temp_190);
        temp_210 = vGmCal1.w;
        temp_212 = vGmCal2.w;
        temp_214 = vGmCal3.w;
        temp_224 = temp_207 * temp_180;
        temp_225 = temp_207 * temp_133;
        temp_226 = temp_207 * temp_178;
        temp_230 = fma(temp_201, temp_143, temp_224);
        temp_231 = fma(temp_201, temp_176, temp_225);
        temp_232 = fma(temp_201, temp_183, temp_226);
        temp_242 = fma(temp_205, temp_194, temp_230);
        temp_248 = fma(temp_205, temp_185, temp_231);
        temp_249 = temp_242 + temp_210;
        temp_257 = temp_248 + temp_212;
        temp_260 = temp_249 * U_Static.gmProj[0].x;
        temp_261 = fma(temp_205, temp_191, temp_232);
        temp_267 = fma(temp_257, U_Static.gmProj[0].y, temp_260);
        temp_269 = temp_261 + temp_214;
        temp_288 = fma(temp_269, U_Static.gmProj[0].z, temp_267);
        temp_298 = temp_288 + U_Static.gmProj[0].w;
    "};
    Graph::parse_glsl_query(query).unwrap().simplify()
});

static GM_CAL_U_NAM_ATTRIBUTE_XYZW_Y: LazyLock<Graph> = LazyLock::new(|| {
    // xc2/ma01a/prop/1/slct18_nvsd0_shd0018.vert
    let query = indoc! {"
        temp_0 = nWgtIdx.y;
        temp_1 = intBitsToFloat(gl_InstanceID);
        temp_2 = trunc(U_NAM.gAnmIdx.x);
        temp_3 = int(temp_2);
        temp_4 = temp_3 & 65535;
        temp_5 = floatBitsToInt(temp_1) & 65535;
        temp_6 = temp_4 * temp_5;
        temp_7 = temp_6 + floatBitsToInt(temp_0);
        temp_9 = temp_3 & 65535;
        temp_10 = floatBitsToUint(temp_1) >> 16;
        temp_11 = temp_9 * int(temp_10);
        temp_12 = temp_11 & 65535;
        temp_13 = floatBitsToInt(temp_1) << 16;
        temp_14 = temp_12 | temp_13;
        temp_16 = uint(temp_3) >> 16;
        temp_17 = uint(temp_14) >> 16;
        temp_18 = int(temp_16) * int(temp_17);
        temp_19 = temp_18 << 16;
        temp_20 = temp_14 << 16;
        temp_21 = temp_7 + temp_20;
        temp_22 = temp_19 + temp_21;
        temp_25 = temp_22 & 65535;
        temp_26 = temp_25 * 48;
        temp_27 = vPos.x;
        temp_28 = uint(temp_22) >> 16;
        temp_29 = int(temp_28) * 48;
        temp_30 = temp_29 << 16;
        temp_31 = temp_30 + temp_26;
        temp_34 = temp_31 + 16;
        temp_35 = uint(temp_34) >> 2;
        temp_37 = int(temp_35) & 3;
        temp_38 = U_NAM.data[int(temp_36)][temp_37];
        temp_39 = int(temp_35) + 1;
        temp_41 = temp_39 & 3;
        temp_42 = U_NAM.data[int(temp_40)][temp_41];
        temp_45 = temp_31 + 48;
        temp_46 = uint(temp_45) >> 2;
        temp_48 = int(temp_46) & 3;
        temp_49 = U_NAM.data[int(temp_47)][temp_48];
        temp_50 = int(temp_46) + 1;
        temp_52 = temp_50 & 3;
        temp_53 = U_NAM.data[int(temp_51)][temp_52];
        temp_58 = temp_31 + 32;
        temp_59 = uint(temp_58) >> 2;
        temp_61 = int(temp_59) & 3;
        temp_62 = U_NAM.data[int(temp_60)][temp_61];
        temp_63 = int(temp_59) + 1;
        temp_65 = temp_63 & 3;
        temp_66 = U_NAM.data[int(temp_64)][temp_65];
        temp_77 = vPos.y;
        temp_96 = temp_31 + 24;
        temp_97 = uint(temp_96) >> 2;
        temp_99 = int(temp_97) & 3;
        temp_100 = U_NAM.data[int(temp_98)][temp_99];
        temp_101 = int(temp_97) + 1;
        temp_103 = temp_101 & 3;
        temp_104 = U_NAM.data[int(temp_102)][temp_103];
        temp_105 = temp_31 + 56;
        temp_106 = uint(temp_105) >> 2;
        temp_108 = int(temp_106) & 3;
        temp_109 = U_NAM.data[int(temp_107)][temp_108];
        temp_110 = int(temp_106) + 1;
        temp_112 = temp_110 & 3;
        temp_113 = U_NAM.data[int(temp_111)][temp_112];
        temp_115 = temp_31 + 40;
        temp_116 = uint(temp_115) >> 2;
        temp_118 = int(temp_116) & 3;
        temp_119 = U_NAM.data[int(temp_117)][temp_118];
        temp_120 = int(temp_116) + 1;
        temp_122 = temp_120 & 3;
        temp_123 = U_NAM.data[int(temp_121)][temp_122];
        temp_127 = vPos.z;
        temp_130 = temp_49 * temp_27;
        temp_131 = vPos.w;
        temp_133 = vGmCal2.x;
        temp_139 = temp_62 * temp_27;
        temp_143 = vGmCal1.y;
        temp_144 = fma(temp_66, temp_77, temp_139);
        temp_154 = temp_38 * temp_27;
        temp_165 = fma(temp_42, temp_77, temp_154);
        temp_176 = vGmCal2.y;
        temp_178 = vGmCal3.x;
        temp_179 = fma(temp_53, temp_77, temp_130);
        temp_180 = vGmCal1.x;
        temp_183 = vGmCal3.y;
        temp_185 = vGmCal2.z;
        temp_186 = fma(temp_109, temp_127, temp_179);
        temp_190 = fma(temp_100, temp_127, temp_165);
        temp_191 = vGmCal3.z;
        temp_192 = fma(temp_119, temp_127, temp_144);
        temp_194 = vGmCal1.z;
        temp_201 = fma(temp_123, temp_131, temp_192);
        temp_205 = fma(temp_113, temp_131, temp_186);
        temp_207 = fma(temp_104, temp_131, temp_190);
        temp_210 = vGmCal1.w;
        temp_212 = vGmCal2.w;
        temp_214 = vGmCal3.w;
        temp_224 = temp_207 * temp_180;
        temp_225 = temp_207 * temp_133;
        temp_226 = temp_207 * temp_178;
        temp_230 = fma(temp_201, temp_143, temp_224);
        temp_231 = fma(temp_201, temp_176, temp_225);
        temp_232 = fma(temp_201, temp_183, temp_226);
        temp_242 = fma(temp_205, temp_194, temp_230);
        temp_248 = fma(temp_205, temp_185, temp_231);
        temp_249 = temp_242 + temp_210;
        temp_257 = temp_248 + temp_212;
        temp_259 = temp_249 * U_Static.gmProj[1].x;
        temp_261 = fma(temp_205, temp_191, temp_232);
        temp_265 = fma(temp_257, U_Static.gmProj[1].y, temp_259);
        temp_269 = temp_261 + temp_214;
        temp_287 = fma(temp_269, U_Static.gmProj[1].z, temp_265);
        temp_297 = temp_287 + U_Static.gmProj[1].w;
    "};
    Graph::parse_glsl_query(query).unwrap().simplify()
});

static GM_CAL_U_NAM_ATTRIBUTE_XYZW_Z: LazyLock<Graph> = LazyLock::new(|| {
    // xc2/ma01a/prop/1/slct18_nvsd0_shd0018.vert
    let query = indoc! {"
        temp_0 = nWgtIdx.y;
        temp_1 = intBitsToFloat(gl_InstanceID);
        temp_2 = trunc(U_NAM.gAnmIdx.x);
        temp_3 = int(temp_2);
        temp_4 = temp_3 & 65535;
        temp_5 = floatBitsToInt(temp_1) & 65535;
        temp_6 = temp_4 * temp_5;
        temp_7 = temp_6 + floatBitsToInt(temp_0);
        temp_9 = temp_3 & 65535;
        temp_10 = floatBitsToUint(temp_1) >> 16;
        temp_11 = temp_9 * int(temp_10);
        temp_12 = temp_11 & 65535;
        temp_13 = floatBitsToInt(temp_1) << 16;
        temp_14 = temp_12 | temp_13;
        temp_16 = uint(temp_3) >> 16;
        temp_17 = uint(temp_14) >> 16;
        temp_18 = int(temp_16) * int(temp_17);
        temp_19 = temp_18 << 16;
        temp_20 = temp_14 << 16;
        temp_21 = temp_7 + temp_20;
        temp_22 = temp_19 + temp_21;
        temp_25 = temp_22 & 65535;
        temp_26 = temp_25 * 48;
        temp_27 = vPos.x;
        temp_28 = uint(temp_22) >> 16;
        temp_29 = int(temp_28) * 48;
        temp_30 = temp_29 << 16;
        temp_31 = temp_30 + temp_26;
        temp_34 = temp_31 + 16;
        temp_35 = uint(temp_34) >> 2;
        temp_37 = int(temp_35) & 3;
        temp_38 = U_NAM.data[int(temp_36)][temp_37];
        temp_39 = int(temp_35) + 1;
        temp_41 = temp_39 & 3;
        temp_42 = U_NAM.data[int(temp_40)][temp_41];
        temp_45 = temp_31 + 48;
        temp_46 = uint(temp_45) >> 2;
        temp_48 = int(temp_46) & 3;
        temp_49 = U_NAM.data[int(temp_47)][temp_48];
        temp_50 = int(temp_46) + 1;
        temp_52 = temp_50 & 3;
        temp_53 = U_NAM.data[int(temp_51)][temp_52];
        temp_58 = temp_31 + 32;
        temp_59 = uint(temp_58) >> 2;
        temp_61 = int(temp_59) & 3;
        temp_62 = U_NAM.data[int(temp_60)][temp_61];
        temp_63 = int(temp_59) + 1;
        temp_65 = temp_63 & 3;
        temp_66 = U_NAM.data[int(temp_64)][temp_65];
        temp_77 = vPos.y;
        temp_96 = temp_31 + 24;
        temp_97 = uint(temp_96) >> 2;
        temp_99 = int(temp_97) & 3;
        temp_100 = U_NAM.data[int(temp_98)][temp_99];
        temp_101 = int(temp_97) + 1;
        temp_103 = temp_101 & 3;
        temp_104 = U_NAM.data[int(temp_102)][temp_103];
        temp_105 = temp_31 + 56;
        temp_106 = uint(temp_105) >> 2;
        temp_108 = int(temp_106) & 3;
        temp_109 = U_NAM.data[int(temp_107)][temp_108];
        temp_110 = int(temp_106) + 1;
        temp_112 = temp_110 & 3;
        temp_113 = U_NAM.data[int(temp_111)][temp_112];
        temp_115 = temp_31 + 40;
        temp_116 = uint(temp_115) >> 2;
        temp_118 = int(temp_116) & 3;
        temp_119 = U_NAM.data[int(temp_117)][temp_118];
        temp_120 = int(temp_116) + 1;
        temp_122 = temp_120 & 3;
        temp_123 = U_NAM.data[int(temp_121)][temp_122];
        temp_127 = vPos.z;
        temp_130 = temp_49 * temp_27;
        temp_131 = vPos.w;
        temp_133 = vGmCal2.x;
        temp_139 = temp_62 * temp_27;
        temp_143 = vGmCal1.y;
        temp_144 = fma(temp_66, temp_77, temp_139);
        temp_154 = temp_38 * temp_27;
        temp_165 = fma(temp_42, temp_77, temp_154);
        temp_176 = vGmCal2.y;
        temp_178 = vGmCal3.x;
        temp_179 = fma(temp_53, temp_77, temp_130);
        temp_180 = vGmCal1.x;
        temp_183 = vGmCal3.y;
        temp_185 = vGmCal2.z;
        temp_186 = fma(temp_109, temp_127, temp_179);
        temp_190 = fma(temp_100, temp_127, temp_165);
        temp_191 = vGmCal3.z;
        temp_192 = fma(temp_119, temp_127, temp_144);
        temp_194 = vGmCal1.z;
        temp_201 = fma(temp_123, temp_131, temp_192);
        temp_205 = fma(temp_113, temp_131, temp_186);
        temp_207 = fma(temp_104, temp_131, temp_190);
        temp_210 = vGmCal1.w;
        temp_212 = vGmCal2.w;
        temp_214 = vGmCal3.w;
        temp_224 = temp_207 * temp_180;
        temp_225 = temp_207 * temp_133;
        temp_226 = temp_207 * temp_178;
        temp_230 = fma(temp_201, temp_143, temp_224);
        temp_231 = fma(temp_201, temp_176, temp_225);
        temp_232 = fma(temp_201, temp_183, temp_226);
        temp_242 = fma(temp_205, temp_194, temp_230);
        temp_248 = fma(temp_205, temp_185, temp_231);
        temp_249 = temp_242 + temp_210;
        temp_257 = temp_248 + temp_212;
        temp_261 = fma(temp_205, temp_191, temp_232);
        temp_262 = temp_249 * U_Static.gmProj[2].x;
        temp_269 = temp_261 + temp_214;
        temp_272 = fma(temp_257, U_Static.gmProj[2].y, temp_262);
        temp_279 = fma(temp_269, U_Static.gmProj[2].z, temp_272);
        temp_285 = temp_279 + U_Static.gmProj[2].w;
        temp_293 = 0.0 - U_Static.gCDep.y;
        temp_294 = temp_285 + temp_293;
        temp_303 = temp_294 * U_Static.gCDep.z;
    "};
    Graph::parse_glsl_query(query).unwrap().simplify()
});

static GM_CAL_U_NAM_ATTRIBUTE_XYZW_W: LazyLock<Graph> = LazyLock::new(|| {
    // xc2/ma01a/prop/1/slct18_nvsd0_shd0018.vert
    let query = indoc! {"
        temp_0 = nWgtIdx.y;
        temp_1 = intBitsToFloat(gl_InstanceID);
        temp_2 = trunc(U_NAM.gAnmIdx.x);
        temp_3 = int(temp_2);
        temp_4 = temp_3 & 65535;
        temp_5 = floatBitsToInt(temp_1) & 65535;
        temp_6 = temp_4 * temp_5;
        temp_7 = temp_6 + floatBitsToInt(temp_0);
        temp_9 = temp_3 & 65535;
        temp_10 = floatBitsToUint(temp_1) >> 16;
        temp_11 = temp_9 * int(temp_10);
        temp_12 = temp_11 & 65535;
        temp_13 = floatBitsToInt(temp_1) << 16;
        temp_14 = temp_12 | temp_13;
        temp_16 = uint(temp_3) >> 16;
        temp_17 = uint(temp_14) >> 16;
        temp_18 = int(temp_16) * int(temp_17);
        temp_19 = temp_18 << 16;
        temp_20 = temp_14 << 16;
        temp_21 = temp_7 + temp_20;
        temp_22 = temp_19 + temp_21;
        temp_25 = temp_22 & 65535;
        temp_26 = temp_25 * 48;
        temp_27 = vPos.x;
        temp_28 = uint(temp_22) >> 16;
        temp_29 = int(temp_28) * 48;
        temp_30 = temp_29 << 16;
        temp_31 = temp_30 + temp_26;
        temp_34 = temp_31 + 16;
        temp_35 = uint(temp_34) >> 2;
        temp_37 = int(temp_35) & 3;
        temp_38 = U_NAM.data[int(temp_36)][temp_37];
        temp_39 = int(temp_35) + 1;
        temp_41 = temp_39 & 3;
        temp_42 = U_NAM.data[int(temp_40)][temp_41];
        temp_45 = temp_31 + 48;
        temp_46 = uint(temp_45) >> 2;
        temp_48 = int(temp_46) & 3;
        temp_49 = U_NAM.data[int(temp_47)][temp_48];
        temp_50 = int(temp_46) + 1;
        temp_52 = temp_50 & 3;
        temp_53 = U_NAM.data[int(temp_51)][temp_52];
        temp_58 = temp_31 + 32;
        temp_59 = uint(temp_58) >> 2;
        temp_61 = int(temp_59) & 3;
        temp_62 = U_NAM.data[int(temp_60)][temp_61];
        temp_63 = int(temp_59) + 1;
        temp_65 = temp_63 & 3;
        temp_66 = U_NAM.data[int(temp_64)][temp_65];
        temp_77 = vPos.y;
        temp_96 = temp_31 + 24;
        temp_97 = uint(temp_96) >> 2;
        temp_99 = int(temp_97) & 3;
        temp_100 = U_NAM.data[int(temp_98)][temp_99];
        temp_101 = int(temp_97) + 1;
        temp_103 = temp_101 & 3;
        temp_104 = U_NAM.data[int(temp_102)][temp_103];
        temp_105 = temp_31 + 56;
        temp_106 = uint(temp_105) >> 2;
        temp_108 = int(temp_106) & 3;
        temp_109 = U_NAM.data[int(temp_107)][temp_108];
        temp_110 = int(temp_106) + 1;
        temp_112 = temp_110 & 3;
        temp_113 = U_NAM.data[int(temp_111)][temp_112];
        temp_115 = temp_31 + 40;
        temp_116 = uint(temp_115) >> 2;
        temp_118 = int(temp_116) & 3;
        temp_119 = U_NAM.data[int(temp_117)][temp_118];
        temp_120 = int(temp_116) + 1;
        temp_122 = temp_120 & 3;
        temp_123 = U_NAM.data[int(temp_121)][temp_122];
        temp_127 = vPos.z;
        temp_130 = temp_49 * temp_27;
        temp_131 = vPos.w;
        temp_133 = vGmCal2.x;
        temp_139 = temp_62 * temp_27;
        temp_143 = vGmCal1.y;
        temp_144 = fma(temp_66, temp_77, temp_139);
        temp_154 = temp_38 * temp_27;
        temp_165 = fma(temp_42, temp_77, temp_154);
        temp_176 = vGmCal2.y;
        temp_178 = vGmCal3.x;
        temp_179 = fma(temp_53, temp_77, temp_130);
        temp_180 = vGmCal1.x;
        temp_183 = vGmCal3.y;
        temp_185 = vGmCal2.z;
        temp_186 = fma(temp_109, temp_127, temp_179);
        temp_190 = fma(temp_100, temp_127, temp_165);
        temp_191 = vGmCal3.z;
        temp_192 = fma(temp_119, temp_127, temp_144);
        temp_194 = vGmCal1.z;
        temp_201 = fma(temp_123, temp_131, temp_192);
        temp_205 = fma(temp_113, temp_131, temp_186);
        temp_207 = fma(temp_104, temp_131, temp_190);
        temp_210 = vGmCal1.w;
        temp_212 = vGmCal2.w;
        temp_214 = vGmCal3.w;
        temp_224 = temp_207 * temp_180;
        temp_225 = temp_207 * temp_133;
        temp_226 = temp_207 * temp_178;
        temp_230 = fma(temp_201, temp_143, temp_224);
        temp_231 = fma(temp_201, temp_176, temp_225);
        temp_232 = fma(temp_201, temp_183, temp_226);
        temp_242 = fma(temp_205, temp_194, temp_230);
        temp_248 = fma(temp_205, temp_185, temp_231);
        temp_249 = temp_242 + temp_210;
        temp_257 = temp_248 + temp_212;
        temp_258 = temp_249 * U_Static.gmProj[3].x;
        temp_261 = fma(temp_205, temp_191, temp_232);
        temp_264 = fma(temp_257, U_Static.gmProj[3].y, temp_258);
        temp_269 = temp_261 + temp_214;
        temp_286 = fma(temp_269, U_Static.gmProj[3].z, temp_264);
        temp_296 = temp_286 + U_Static.gmProj[3].w;
    "};
    Graph::parse_glsl_query(query).unwrap().simplify()
});

pub fn gm_cal_u_nam_attribute_xyzw<'a>(graph: &'a Graph, expr: &'a Expr) -> Option<Expr> {
    // TODO: This should match the attribute names exactly to be able to return &Expr?
    // TODO: Don't assume vPos?
    query_nodes(expr, graph, &GM_CAL_U_NAM_ATTRIBUTE_XYZW_X)
        .map(|_| Expr::Global {
            name: "vPos".into(),
            channel: Some('x'),
        })
        .or_else(|| {
            query_nodes(expr, graph, &GM_CAL_U_NAM_ATTRIBUTE_XYZW_Y).map(|_| Expr::Global {
                name: "vPos".into(),
                channel: Some('y'),
            })
        })
        .or_else(|| {
            query_nodes(expr, graph, &GM_CAL_U_NAM_ATTRIBUTE_XYZW_Z).map(|_| Expr::Global {
                name: "vPos".into(),
                channel: Some('z'),
            })
        })
        .or_else(|| {
            query_nodes(expr, graph, &GM_CAL_U_NAM_ATTRIBUTE_XYZW_W).map(|_| Expr::Global {
                name: "vPos".into(),
                channel: Some('w'),
            })
        })
}

static GM_CAL_U_NAM2_ATTRIBUTE_XYZW_X: LazyLock<Graph> = LazyLock::new(|| {
    // xc2/ma01a/prop/1/slct18_nvsd0_shd0018.vert
    let query = indoc! {"
        temp_0 = nWgtIdx.y;
        temp_1 = intBitsToFloat(gl_InstanceID);
        temp_2 = trunc(U_NAM.gAnmIdx.x);
        temp_3 = int(temp_2);
        temp_4 = temp_3 & 65535;
        temp_5 = floatBitsToInt(temp_1) & 65535;
        temp_6 = temp_4 * temp_5;
        temp_7 = temp_6 + floatBitsToInt(temp_0);
        temp_9 = temp_3 & 65535;
        temp_10 = floatBitsToUint(temp_1) >> 16;
        temp_11 = temp_9 * int(temp_10);
        temp_12 = temp_11 & 65535;
        temp_13 = floatBitsToInt(temp_1) << 16;
        temp_14 = temp_12 | temp_13;
        temp_16 = uint(temp_3) >> 16;
        temp_17 = uint(temp_14) >> 16;
        temp_18 = int(temp_16) * int(temp_17);
        temp_19 = temp_18 << 16;
        temp_20 = temp_14 << 16;
        temp_21 = temp_7 + temp_20;
        temp_22 = temp_19 + temp_21;
        temp_23 = trunc(U_NAM.gAnmIdx.y);
        temp_24 = int(temp_23);
        temp_27 = vPos.x;
        temp_33 = temp_22 + temp_24;
        temp_43 = temp_33 & 65535;
        temp_44 = temp_43 * 48;
        temp_54 = uint(temp_33) >> 16;
        temp_55 = int(temp_54) * 48;
        temp_56 = temp_55 << 16;
        temp_57 = temp_56 + temp_44;
        temp_67 = temp_57 + 16;
        temp_68 = uint(temp_67) >> 2;
        temp_70 = int(temp_68) & 3;
        temp_71 = U_NAM.data[int(temp_69)][temp_70];
        temp_72 = int(temp_68) + 1;
        temp_74 = temp_72 & 3;
        temp_75 = U_NAM.data[int(temp_73)][temp_74];
        temp_77 = vPos.y;
        temp_78 = temp_57 + 32;
        temp_79 = uint(temp_78) >> 2;
        temp_81 = int(temp_79) & 3;
        temp_82 = U_NAM.data[int(temp_80)][temp_81];
        temp_83 = int(temp_79) + 1;
        temp_85 = temp_83 & 3;
        temp_86 = U_NAM.data[int(temp_84)][temp_85];
        temp_87 = temp_57 + 48;
        temp_88 = uint(temp_87) >> 2;
        temp_90 = int(temp_88) & 3;
        temp_91 = U_NAM.data[int(temp_89)][temp_90];
        temp_92 = int(temp_88) + 1;
        temp_94 = temp_92 & 3;
        temp_95 = U_NAM.data[int(temp_93)][temp_94];
        temp_127 = vPos.z;
        temp_131 = vPos.w;
        temp_133 = vGmCal2.x;
        temp_140 = temp_71 * temp_27;
        temp_143 = vGmCal1.y;
        temp_145 = temp_57 + 40;
        temp_146 = uint(temp_145) >> 2;
        temp_148 = int(temp_146) & 3;
        temp_149 = U_NAM.data[int(temp_147)][temp_148];
        temp_150 = int(temp_146) + 1;
        temp_152 = temp_150 & 3;
        temp_153 = U_NAM.data[int(temp_151)][temp_152];
        temp_155 = fma(temp_75, temp_77, temp_140);
        temp_156 = temp_57 + 56;
        temp_157 = uint(temp_156) >> 2;
        temp_159 = int(temp_157) & 3;
        temp_160 = U_NAM.data[int(temp_158)][temp_159];
        temp_161 = int(temp_157) + 1;
        temp_163 = temp_161 & 3;
        temp_164 = U_NAM.data[int(temp_162)][temp_163];
        temp_166 = temp_57 + 24;
        temp_167 = uint(temp_166) >> 2;
        temp_169 = int(temp_167) & 3;
        temp_170 = U_NAM.data[int(temp_168)][temp_169];
        temp_171 = int(temp_167) + 1;
        temp_173 = temp_171 & 3;
        temp_174 = U_NAM.data[int(temp_172)][temp_173];
        temp_175 = temp_82 * temp_27;
        temp_176 = vGmCal2.y;
        temp_177 = temp_91 * temp_27;
        temp_178 = vGmCal3.x;
        temp_180 = vGmCal1.x;
        temp_181 = fma(temp_86, temp_77, temp_175);
        temp_182 = fma(temp_95, temp_77, temp_177);
        temp_183 = vGmCal3.y;
        temp_185 = vGmCal2.z;
        temp_191 = vGmCal3.z;
        temp_194 = vGmCal1.z;
        temp_195 = fma(temp_149, temp_127, temp_181);
        temp_197 = fma(temp_160, temp_127, temp_182);
        temp_199 = fma(temp_170, temp_127, temp_155);
        temp_203 = fma(temp_153, temp_131, temp_195);
        temp_210 = vGmCal1.w;
        temp_212 = vGmCal2.w;
        temp_214 = vGmCal3.w;
        temp_216 = fma(temp_174, temp_131, temp_199);
        temp_219 = fma(temp_164, temp_131, temp_197);
        temp_233 = temp_216 * temp_180;
        temp_234 = temp_216 * temp_133;
        temp_235 = temp_216 * temp_178;
        temp_244 = fma(temp_203, temp_176, temp_234);
        temp_247 = fma(temp_203, temp_143, temp_233);
        temp_256 = fma(temp_219, temp_194, temp_247);
        temp_263 = fma(temp_219, temp_185, temp_244);
        temp_266 = temp_256 + temp_210;
        temp_268 = fma(temp_203, temp_183, temp_235);
        temp_273 = temp_263 + temp_212;
        temp_277 = fma(temp_219, temp_191, temp_268);
        temp_278 = temp_266 * U_Static.gmDiffPreMat[0].x;
        temp_283 = temp_277 + temp_214;
        temp_284 = fma(temp_273, U_Static.gmDiffPreMat[0].y, temp_278);
        temp_292 = fma(temp_283, U_Static.gmDiffPreMat[0].z, temp_284);
        temp_302 = temp_292 + U_Static.gmDiffPreMat[0].w;
    "};
    Graph::parse_glsl_query(query).unwrap().simplify()
});

static GM_CAL_U_NAM2_ATTRIBUTE_XYZW_Y: LazyLock<Graph> = LazyLock::new(|| {
    // xc2/ma01a/prop/1/slct18_nvsd0_shd0018.vert
    let query = indoc! {"
        temp_0 = nWgtIdx.y;
        temp_1 = intBitsToFloat(gl_InstanceID);
        temp_2 = trunc(U_NAM.gAnmIdx.x);
        temp_3 = int(temp_2);
        temp_4 = temp_3 & 65535;
        temp_5 = floatBitsToInt(temp_1) & 65535;
        temp_6 = temp_4 * temp_5;
        temp_7 = temp_6 + floatBitsToInt(temp_0);
        temp_9 = temp_3 & 65535;
        temp_10 = floatBitsToUint(temp_1) >> 16;
        temp_11 = temp_9 * int(temp_10);
        temp_12 = temp_11 & 65535;
        temp_13 = floatBitsToInt(temp_1) << 16;
        temp_14 = temp_12 | temp_13;
        temp_16 = uint(temp_3) >> 16;
        temp_17 = uint(temp_14) >> 16;
        temp_18 = int(temp_16) * int(temp_17);
        temp_19 = temp_18 << 16;
        temp_20 = temp_14 << 16;
        temp_21 = temp_7 + temp_20;
        temp_22 = temp_19 + temp_21;
        temp_23 = trunc(U_NAM.gAnmIdx.y);
        temp_24 = int(temp_23);
        temp_27 = vPos.x;
        temp_33 = temp_22 + temp_24;
        temp_43 = temp_33 & 65535;
        temp_44 = temp_43 * 48;
        temp_54 = uint(temp_33) >> 16;
        temp_55 = int(temp_54) * 48;
        temp_56 = temp_55 << 16;
        temp_57 = temp_56 + temp_44;
        temp_67 = temp_57 + 16;
        temp_68 = uint(temp_67) >> 2;
        temp_70 = int(temp_68) & 3;
        temp_71 = U_NAM.data[int(temp_69)][temp_70];
        temp_72 = int(temp_68) + 1;
        temp_74 = temp_72 & 3;
        temp_75 = U_NAM.data[int(temp_73)][temp_74];
        temp_77 = vPos.y;
        temp_78 = temp_57 + 32;
        temp_79 = uint(temp_78) >> 2;
        temp_81 = int(temp_79) & 3;
        temp_82 = U_NAM.data[int(temp_80)][temp_81];
        temp_83 = int(temp_79) + 1;
        temp_85 = temp_83 & 3;
        temp_86 = U_NAM.data[int(temp_84)][temp_85];
        temp_87 = temp_57 + 48;
        temp_88 = uint(temp_87) >> 2;
        temp_90 = int(temp_88) & 3;
        temp_91 = U_NAM.data[int(temp_89)][temp_90];
        temp_92 = int(temp_88) + 1;
        temp_94 = temp_92 & 3;
        temp_95 = U_NAM.data[int(temp_93)][temp_94];
        temp_127 = vPos.z;
        temp_131 = vPos.w;
        temp_133 = vGmCal2.x;
        temp_140 = temp_71 * temp_27;
        temp_143 = vGmCal1.y;
        temp_145 = temp_57 + 40;
        temp_146 = uint(temp_145) >> 2;
        temp_148 = int(temp_146) & 3;
        temp_149 = U_NAM.data[int(temp_147)][temp_148];
        temp_150 = int(temp_146) + 1;
        temp_152 = temp_150 & 3;
        temp_153 = U_NAM.data[int(temp_151)][temp_152];
        temp_155 = fma(temp_75, temp_77, temp_140);
        temp_156 = temp_57 + 56;
        temp_157 = uint(temp_156) >> 2;
        temp_159 = int(temp_157) & 3;
        temp_160 = U_NAM.data[int(temp_158)][temp_159];
        temp_161 = int(temp_157) + 1;
        temp_163 = temp_161 & 3;
        temp_164 = U_NAM.data[int(temp_162)][temp_163];
        temp_166 = temp_57 + 24;
        temp_167 = uint(temp_166) >> 2;
        temp_169 = int(temp_167) & 3;
        temp_170 = U_NAM.data[int(temp_168)][temp_169];
        temp_171 = int(temp_167) + 1;
        temp_173 = temp_171 & 3;
        temp_174 = U_NAM.data[int(temp_172)][temp_173];
        temp_175 = temp_82 * temp_27;
        temp_176 = vGmCal2.y;
        temp_177 = temp_91 * temp_27;
        temp_178 = vGmCal3.x;
        temp_180 = vGmCal1.x;
        temp_181 = fma(temp_86, temp_77, temp_175);
        temp_182 = fma(temp_95, temp_77, temp_177);
        temp_183 = vGmCal3.y;
        temp_185 = vGmCal2.z;
        temp_191 = vGmCal3.z;
        temp_194 = vGmCal1.z;
        temp_195 = fma(temp_149, temp_127, temp_181);
        temp_197 = fma(temp_160, temp_127, temp_182);
        temp_199 = fma(temp_170, temp_127, temp_155);
        temp_203 = fma(temp_153, temp_131, temp_195);
        temp_210 = vGmCal1.w;
        temp_212 = vGmCal2.w;
        temp_214 = vGmCal3.w;
        temp_216 = fma(temp_174, temp_131, temp_199);
        temp_219 = fma(temp_164, temp_131, temp_197);
        temp_233 = temp_216 * temp_180;
        temp_234 = temp_216 * temp_133;
        temp_235 = temp_216 * temp_178;
        temp_244 = fma(temp_203, temp_176, temp_234);
        temp_247 = fma(temp_203, temp_143, temp_233);
        temp_256 = fma(temp_219, temp_194, temp_247);
        temp_263 = fma(temp_219, temp_185, temp_244);
        temp_266 = temp_256 + temp_210;
        temp_268 = fma(temp_203, temp_183, temp_235);
        temp_273 = temp_263 + temp_212;
        temp_276 = temp_266 * U_Static.gmDiffPreMat[1].x;
        temp_277 = fma(temp_219, temp_191, temp_268);
        temp_282 = fma(temp_273, U_Static.gmDiffPreMat[1].y, temp_276);
        temp_283 = temp_277 + temp_214;
        temp_291 = fma(temp_283, U_Static.gmDiffPreMat[1].z, temp_282);
        temp_301 = temp_291 + U_Static.gmDiffPreMat[1].w;
    "};
    Graph::parse_glsl_query(query).unwrap().simplify()
});

static GM_CAL_U_NAM2_ATTRIBUTE_XYZW_Z: LazyLock<Graph> = LazyLock::new(|| {
    // xc2/ma01a/prop/1/slct18_nvsd0_shd0018.vert
    let query = indoc! {"
        temp_0 = nWgtIdx.y;
        temp_1 = intBitsToFloat(gl_InstanceID);
        temp_2 = trunc(U_NAM.gAnmIdx.x);
        temp_3 = int(temp_2);
        temp_4 = temp_3 & 65535;
        temp_5 = floatBitsToInt(temp_1) & 65535;
        temp_6 = temp_4 * temp_5;
        temp_7 = temp_6 + floatBitsToInt(temp_0);
        temp_9 = temp_3 & 65535;
        temp_10 = floatBitsToUint(temp_1) >> 16;
        temp_11 = temp_9 * int(temp_10);
        temp_12 = temp_11 & 65535;
        temp_13 = floatBitsToInt(temp_1) << 16;
        temp_14 = temp_12 | temp_13;
        temp_16 = uint(temp_3) >> 16;
        temp_17 = uint(temp_14) >> 16;
        temp_18 = int(temp_16) * int(temp_17);
        temp_19 = temp_18 << 16;
        temp_20 = temp_14 << 16;
        temp_21 = temp_7 + temp_20;
        temp_22 = temp_19 + temp_21;
        temp_23 = trunc(U_NAM.gAnmIdx.y);
        temp_24 = int(temp_23);
        temp_27 = vPos.x;
        temp_33 = temp_22 + temp_24;
        temp_43 = temp_33 & 65535;
        temp_44 = temp_43 * 48;
        temp_54 = uint(temp_33) >> 16;
        temp_55 = int(temp_54) * 48;
        temp_56 = temp_55 << 16;
        temp_57 = temp_56 + temp_44;
        temp_67 = temp_57 + 16;
        temp_68 = uint(temp_67) >> 2;
        temp_70 = int(temp_68) & 3;
        temp_71 = U_NAM.data[int(temp_69)][temp_70];
        temp_72 = int(temp_68) + 1;
        temp_74 = temp_72 & 3;
        temp_75 = U_NAM.data[int(temp_73)][temp_74];
        temp_77 = vPos.y;
        temp_78 = temp_57 + 32;
        temp_79 = uint(temp_78) >> 2;
        temp_81 = int(temp_79) & 3;
        temp_82 = U_NAM.data[int(temp_80)][temp_81];
        temp_83 = int(temp_79) + 1;
        temp_85 = temp_83 & 3;
        temp_86 = U_NAM.data[int(temp_84)][temp_85];
        temp_87 = temp_57 + 48;
        temp_88 = uint(temp_87) >> 2;
        temp_90 = int(temp_88) & 3;
        temp_91 = U_NAM.data[int(temp_89)][temp_90];
        temp_92 = int(temp_88) + 1;
        temp_94 = temp_92 & 3;
        temp_95 = U_NAM.data[int(temp_93)][temp_94];
        temp_127 = vPos.z;
        temp_131 = vPos.w;
        temp_133 = vGmCal2.x;
        temp_140 = temp_71 * temp_27;
        temp_143 = vGmCal1.y;
        temp_145 = temp_57 + 40;
        temp_146 = uint(temp_145) >> 2;
        temp_148 = int(temp_146) & 3;
        temp_149 = U_NAM.data[int(temp_147)][temp_148];
        temp_150 = int(temp_146) + 1;
        temp_152 = temp_150 & 3;
        temp_153 = U_NAM.data[int(temp_151)][temp_152];
        temp_155 = fma(temp_75, temp_77, temp_140);
        temp_156 = temp_57 + 56;
        temp_157 = uint(temp_156) >> 2;
        temp_159 = int(temp_157) & 3;
        temp_160 = U_NAM.data[int(temp_158)][temp_159];
        temp_161 = int(temp_157) + 1;
        temp_163 = temp_161 & 3;
        temp_164 = U_NAM.data[int(temp_162)][temp_163];
        temp_166 = temp_57 + 24;
        temp_167 = uint(temp_166) >> 2;
        temp_169 = int(temp_167) & 3;
        temp_170 = U_NAM.data[int(temp_168)][temp_169];
        temp_171 = int(temp_167) + 1;
        temp_173 = temp_171 & 3;
        temp_174 = U_NAM.data[int(temp_172)][temp_173];
        temp_175 = temp_82 * temp_27;
        temp_176 = vGmCal2.y;
        temp_177 = temp_91 * temp_27;
        temp_178 = vGmCal3.x;
        temp_180 = vGmCal1.x;
        temp_181 = fma(temp_86, temp_77, temp_175);
        temp_182 = fma(temp_95, temp_77, temp_177);
        temp_183 = vGmCal3.y;
        temp_185 = vGmCal2.z;
        temp_191 = vGmCal3.z;
        temp_194 = vGmCal1.z;
        temp_195 = fma(temp_149, temp_127, temp_181);
        temp_197 = fma(temp_160, temp_127, temp_182);
        temp_199 = fma(temp_170, temp_127, temp_155);
        temp_203 = fma(temp_153, temp_131, temp_195);
        temp_210 = vGmCal1.w;
        temp_212 = vGmCal2.w;
        temp_214 = vGmCal3.w;
        temp_216 = fma(temp_174, temp_131, temp_199);
        temp_219 = fma(temp_164, temp_131, temp_197);
        temp_233 = temp_216 * temp_180;
        temp_234 = temp_216 * temp_133;
        temp_235 = temp_216 * temp_178;
        temp_244 = fma(temp_203, temp_176, temp_234);
        temp_247 = fma(temp_203, temp_143, temp_233);
        temp_256 = fma(temp_219, temp_194, temp_247);
        temp_263 = fma(temp_219, temp_185, temp_244);
        temp_266 = temp_256 + temp_210;
        temp_268 = fma(temp_203, temp_183, temp_235);
        temp_273 = temp_263 + temp_212;
        temp_274 = temp_266 * U_Static.gmDiffPreMat[2].x;
        temp_277 = fma(temp_219, temp_191, temp_268);
        temp_280 = fma(temp_273, U_Static.gmDiffPreMat[2].y, temp_274);
        temp_283 = temp_277 + temp_214;
        temp_289 = fma(temp_283, U_Static.gmDiffPreMat[2].z, temp_280);
        temp_299 = temp_289 + U_Static.gmDiffPreMat[2].w;
    "};
    Graph::parse_glsl_query(query).unwrap().simplify()
});

static GM_CAL_U_NAM2_ATTRIBUTE_XYZW_W: LazyLock<Graph> = LazyLock::new(|| {
    // xc2/ma01a/prop/1/slct18_nvsd0_shd0018.vert
    let query = indoc! {"
        temp_0 = nWgtIdx.y;
        temp_1 = intBitsToFloat(gl_InstanceID);
        temp_2 = trunc(U_NAM.gAnmIdx.x);
        temp_3 = int(temp_2);
        temp_4 = temp_3 & 65535;
        temp_5 = floatBitsToInt(temp_1) & 65535;
        temp_6 = temp_4 * temp_5;
        temp_7 = temp_6 + floatBitsToInt(temp_0);
        temp_9 = temp_3 & 65535;
        temp_10 = floatBitsToUint(temp_1) >> 16;
        temp_11 = temp_9 * int(temp_10);
        temp_12 = temp_11 & 65535;
        temp_13 = floatBitsToInt(temp_1) << 16;
        temp_14 = temp_12 | temp_13;
        temp_16 = uint(temp_3) >> 16;
        temp_17 = uint(temp_14) >> 16;
        temp_18 = int(temp_16) * int(temp_17);
        temp_19 = temp_18 << 16;
        temp_20 = temp_14 << 16;
        temp_21 = temp_7 + temp_20;
        temp_22 = temp_19 + temp_21;
        temp_23 = trunc(U_NAM.gAnmIdx.y);
        temp_24 = int(temp_23);
        temp_27 = vPos.x;
        temp_33 = temp_22 + temp_24;
        temp_43 = temp_33 & 65535;
        temp_44 = temp_43 * 48;
        temp_54 = uint(temp_33) >> 16;
        temp_55 = int(temp_54) * 48;
        temp_56 = temp_55 << 16;
        temp_57 = temp_56 + temp_44;
        temp_67 = temp_57 + 16;
        temp_68 = uint(temp_67) >> 2;
        temp_70 = int(temp_68) & 3;
        temp_71 = U_NAM.data[int(temp_69)][temp_70];
        temp_72 = int(temp_68) + 1;
        temp_74 = temp_72 & 3;
        temp_75 = U_NAM.data[int(temp_73)][temp_74];
        temp_77 = vPos.y;
        temp_78 = temp_57 + 32;
        temp_79 = uint(temp_78) >> 2;
        temp_81 = int(temp_79) & 3;
        temp_82 = U_NAM.data[int(temp_80)][temp_81];
        temp_83 = int(temp_79) + 1;
        temp_85 = temp_83 & 3;
        temp_86 = U_NAM.data[int(temp_84)][temp_85];
        temp_87 = temp_57 + 48;
        temp_88 = uint(temp_87) >> 2;
        temp_90 = int(temp_88) & 3;
        temp_91 = U_NAM.data[int(temp_89)][temp_90];
        temp_92 = int(temp_88) + 1;
        temp_94 = temp_92 & 3;
        temp_95 = U_NAM.data[int(temp_93)][temp_94];
        temp_127 = vPos.z;
        temp_131 = vPos.w;
        temp_133 = vGmCal2.x;
        temp_140 = temp_71 * temp_27;
        temp_143 = vGmCal1.y;
        temp_145 = temp_57 + 40;
        temp_146 = uint(temp_145) >> 2;
        temp_148 = int(temp_146) & 3;
        temp_149 = U_NAM.data[int(temp_147)][temp_148];
        temp_150 = int(temp_146) + 1;
        temp_152 = temp_150 & 3;
        temp_153 = U_NAM.data[int(temp_151)][temp_152];
        temp_155 = fma(temp_75, temp_77, temp_140);
        temp_156 = temp_57 + 56;
        temp_157 = uint(temp_156) >> 2;
        temp_159 = int(temp_157) & 3;
        temp_160 = U_NAM.data[int(temp_158)][temp_159];
        temp_161 = int(temp_157) + 1;
        temp_163 = temp_161 & 3;
        temp_164 = U_NAM.data[int(temp_162)][temp_163];
        temp_166 = temp_57 + 24;
        temp_167 = uint(temp_166) >> 2;
        temp_169 = int(temp_167) & 3;
        temp_170 = U_NAM.data[int(temp_168)][temp_169];
        temp_171 = int(temp_167) + 1;
        temp_173 = temp_171 & 3;
        temp_174 = U_NAM.data[int(temp_172)][temp_173];
        temp_175 = temp_82 * temp_27;
        temp_176 = vGmCal2.y;
        temp_177 = temp_91 * temp_27;
        temp_178 = vGmCal3.x;
        temp_180 = vGmCal1.x;
        temp_181 = fma(temp_86, temp_77, temp_175);
        temp_182 = fma(temp_95, temp_77, temp_177);
        temp_183 = vGmCal3.y;
        temp_185 = vGmCal2.z;
        temp_191 = vGmCal3.z;
        temp_194 = vGmCal1.z;
        temp_195 = fma(temp_149, temp_127, temp_181);
        temp_197 = fma(temp_160, temp_127, temp_182);
        temp_199 = fma(temp_170, temp_127, temp_155);
        temp_203 = fma(temp_153, temp_131, temp_195);
        temp_210 = vGmCal1.w;
        temp_212 = vGmCal2.w;
        temp_214 = vGmCal3.w;
        temp_216 = fma(temp_174, temp_131, temp_199);
        temp_219 = fma(temp_164, temp_131, temp_197);
        temp_233 = temp_216 * temp_180;
        temp_234 = temp_216 * temp_133;
        temp_235 = temp_216 * temp_178;
        temp_244 = fma(temp_203, temp_176, temp_234);
        temp_247 = fma(temp_203, temp_143, temp_233);
        temp_256 = fma(temp_219, temp_194, temp_247);
        temp_263 = fma(temp_219, temp_185, temp_244);
        temp_266 = temp_256 + temp_210;
        temp_268 = fma(temp_203, temp_183, temp_235);
        temp_273 = temp_263 + temp_212;
        temp_275 = temp_266 * U_Static.gmDiffPreMat[3].x;
        temp_277 = fma(temp_219, temp_191, temp_268);
        temp_281 = fma(temp_273, U_Static.gmDiffPreMat[3].y, temp_275);
        temp_283 = temp_277 + temp_214;
        temp_290 = fma(temp_283, U_Static.gmDiffPreMat[3].z, temp_281);
        temp_300 = temp_290 + U_Static.gmDiffPreMat[3].w;
    "};
    Graph::parse_glsl_query(query).unwrap().simplify()
});

pub fn gm_cal_u_nam2_attribute_xyzw<'a>(graph: &'a Graph, expr: &'a Expr) -> Option<Expr> {
    // TODO: This should match the attribute names exactly to be able to return &Expr?
    // TODO: Don't assume vPos?
    query_nodes(expr, graph, &GM_CAL_U_NAM2_ATTRIBUTE_XYZW_X)
        .map(|_| Expr::Global {
            name: "vPos".into(),
            channel: Some('x'),
        })
        .or_else(|| {
            query_nodes(expr, graph, &GM_CAL_U_NAM2_ATTRIBUTE_XYZW_Y).map(|_| Expr::Global {
                name: "vPos".into(),
                channel: Some('y'),
            })
        })
        .or_else(|| {
            query_nodes(expr, graph, &GM_CAL_U_NAM2_ATTRIBUTE_XYZW_Z).map(|_| Expr::Global {
                name: "vPos".into(),
                channel: Some('z'),
            })
        })
        .or_else(|| {
            query_nodes(expr, graph, &GM_CAL_U_NAM2_ATTRIBUTE_XYZW_W).map(|_| Expr::Global {
                name: "vPos".into(),
                channel: Some('w'),
            })
        })
}

static GM_CAL_U_BILL_COLOR_ATTRIBUTE_W: LazyLock<Graph> = LazyLock::new(|| {
    // TODO: Why does the vertex color alpha need the instance transform from vGmCal?
    // TODO: U_BILL contains some sort of position?
    // xc2/ma02a/prop/1/slct48_nvsd0_shd0048.vert
    let query = indoc! {"
        temp_0 = nWgtIdx.x;
        temp_1 = floatBitsToInt(temp_0) << 4;
        temp_2 = vGmCal1.x;
        temp_3 = uint(temp_1) >> 2;
        temp_5 = int(temp_3) & 3;
        temp_6 = U_BILL.data[int(temp_4)][temp_5];
        temp_7 = int(temp_3) + 1;
        temp_9 = temp_7 & 3;
        temp_10 = U_BILL.data[int(temp_8)][temp_9];
        temp_11 = temp_1 + 8;
        temp_12 = uint(temp_11) >> 2;
        temp_14 = int(temp_12) & 3;
        temp_15 = U_BILL.data[int(temp_13)][temp_14];
        temp_16 = vGmCal1.y;
        temp_17 = vGmCal2.x;
        temp_18 = vGmCal3.x;
        temp_19 = vGmCal2.y;
        temp_20 = vGmCal1.z;
        temp_21 = vGmCal3.y;
        temp_22 = vGmCal2.z;
        temp_23 = vGmCal1.w;
        temp_24 = vGmCal3.z;
        temp_25 = vGmCal2.w;
        temp_26 = vGmCal3.w;
        temp_27 = temp_6 * temp_2;
        temp_29 = temp_6 * temp_17;
        temp_30 = fma(temp_10, temp_16, temp_27);
        temp_31 = temp_6 * temp_18;
        temp_32 = fma(temp_10, temp_19, temp_29);
        temp_34 = fma(temp_15, temp_20, temp_30);
        temp_35 = fma(temp_10, temp_21, temp_31);
        temp_36 = color_w;
        temp_37 = fma(temp_15, temp_22, temp_32);
        temp_39 = temp_34 + temp_23;
        temp_41 = fma(temp_15, temp_24, temp_35);
        temp_42 = temp_37 + temp_25;
        temp_43 = temp_39 * temp_39;
        temp_44 = temp_41 + temp_26;
        temp_46 = fma(temp_42, temp_42, temp_43);
        temp_49 = fma(temp_44, temp_44, temp_46);
        temp_50 = sqrt(temp_49);
        temp_56 = 0.0 - U_Mate.gWrkFl4[0].x;
        temp_57 = temp_50 + temp_56;
        temp_62 = abs(temp_57);
        temp_63 = 0.0 - temp_62;
        temp_64 = temp_63 + -0.0;
        temp_67 = fma(temp_64, U_Mate.gWrkFl4[0].y, 1.0);
        temp_68 = clamp(temp_67, 0.0, 1.0);
        temp_70 = temp_44 + U_Mate.gWrkFl4[0].z;
        temp_71 = temp_68 * temp_36;
        temp_73 = temp_70 * U_Mate.gWrkFl4[0].w;
        temp_74 = clamp(temp_73, 0.0, 1.0);
        temp_79 = 0.0 - temp_74;
        temp_80 = fma(temp_71, temp_79, temp_71);
    "};
    Graph::parse_glsl_query(query).unwrap().simplify()
});

pub fn gm_cal_u_bill_color_attribute_w<'a>(graph: &'a Graph, expr: &'a Expr) -> Option<&'a Expr> {
    let result = query_nodes(expr, graph, &GM_CAL_U_BILL_COLOR_ATTRIBUTE_W)?;
    result.get("color_w").copied()
}

static U_MDL_ATTRIBUTE_XYZW: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
        temp_0 = result_x;
        temp_1 = result_y;
        temp_2 = result_z;
        temp_3 = result_w;
        temp_24 = temp_0 * U_Mdl.gmWorldView[index].x;
        temp_28 = fma(temp_1, U_Mdl.gmWorldView[index].y, temp_24);
        temp_34 = fma(temp_2, U_Mdl.gmWorldView[index].z, temp_28);
        temp_40 = fma(temp_3, U_Mdl.gmWorldView[index].w, temp_34);
    "};
    Graph::parse_glsl_query(query).unwrap().simplify()
});

pub fn u_mdl_view_attribute_xyzw<'a>(graph: &'a Graph, expr: &'a Expr) -> Option<&'a Expr> {
    // TODO: return an operation for this conversion.
    // TODO: match the buffer name and field.
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
    "};
    Graph::parse_glsl_query(query).unwrap().simplify()
});

static U_MDL_VIEW_BITANGENT_Y: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
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
    "};
    Graph::parse_glsl_query(query).unwrap().simplify()
});

static U_MDL_VIEW_BITANGENT_Z: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
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
    "};
    Graph::parse_glsl_query(query).unwrap().simplify()
});

pub fn u_mdl_view_bitangent_xyz(graph: &Graph, expr: &Expr) -> Option<Expr> {
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
        temp_0 = result_x;
        temp_1 = result_y;
        temp_2 = result_z;
        temp_3 = result_w;
        temp_8 = temp_0 * U_Mdl.gmWVP[0].x;
        temp_16 = fma(temp_1, U_Mdl.gmWVP[0].y, temp_8);
        temp_29 = fma(temp_2, U_Mdl.gmWVP[0].z, temp_16);
        temp_36 = fma(temp_3, U_Mdl.gmWVP[0].w, temp_29);
    "};
    Graph::parse_glsl_query(query).unwrap().simplify()
});

static U_MDL_CLIP_ATTRIBUTE_XYZW_Y: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
        temp_0 = result_x;
        temp_1 = result_y;
        temp_2 = result_z;
        temp_3 = result_w;
        temp_12 = temp_0 * U_Mdl.gmWVP[1].x;
        temp_21 = fma(temp_1, U_Mdl.gmWVP[1].y, temp_12);
        temp_32 = fma(temp_2, U_Mdl.gmWVP[1].z, temp_21);
        temp_38 = fma(temp_3, U_Mdl.gmWVP[1].w, temp_32);
    "};
    Graph::parse_glsl_query(query).unwrap().simplify()
});

static U_MDL_CLIP_ATTRIBUTE_XYZW_Z: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
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
    "};
    Graph::parse_glsl_query(query).unwrap().simplify()
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
        temp_0 = result_x;
        temp_1 = result_y;
        temp_2 = result_z;
        temp_24 = temp_0 * U_Mdl.gmWorldView[index].x;
        temp_28 = fma(temp_1, U_Mdl.gmWorldView[index].y, temp_24);
        temp_34 = fma(temp_2, U_Mdl.gmWorldView[index].z, temp_28);
    "};
    Graph::parse_glsl_query(query).unwrap().simplify()
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
        result = u * param_x;
        result = fma(v, param_y, result);
        result = fma(0.0, param_z, result);
        result = result + param_w;
    "};
    Graph::parse_glsl_query(query).unwrap().simplify()
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
        nrm_result = fma(temp1, 0.7, temp2);
        result = fma(nrm_result, ratio, coord);
    "};
    Graph::parse_glsl_query(query).unwrap().simplify()
});

static TEX_PARALLAX2: LazyLock<Graph> = LazyLock::new(|| {
    // uv = ratio * 0.7 * (nrm.x * tan.xy - norm.y * bitan.xy) + vTex0.xy
    let query = indoc! {"
        coord = coord;
        mask = mask;
        nrm_result = fma(temp1, 0.7, temp2);
        result = fma(ratio, nrm_result, coord);
        // Generated for some shaders.
        result = abs(result);
        result = result + -0.0;
    "};
    Graph::parse_glsl_query(query).unwrap().simplify()
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
    "};
    Graph::parse_glsl_query(query).unwrap().simplify()
});

static TEX_PARALLAX3_Y: LazyLock<Graph> = LazyLock::new(|| {
    // v = ratio * (2 * normal.y * bitangent.y - 2 * normal.x * tangent.y) + vTex0.x
    let query = indoc! {"
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
    "};
    Graph::parse_glsl_query(query).unwrap().simplify()
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

static REFLECT: LazyLock<Graph> = LazyLock::new(|| {
    // reflect(I, N) = I - 2.0 * dot(N, I) * N
    let query = indoc! {"
        dot_n_i = n_x * i_x;
        dot_n_i = fma(n_y, i_y, dot_n_i);
        dot_n_i = fma(n_z, i_z, dot_n_i);
        temp_127 = n_c * dot_n_i;
        temp_129 = fma(temp_127, -2.0, i_c);
    "};
    Graph::parse_glsl_query(query).unwrap().simplify()
});

pub fn op_reflect<'a>(graph: &'a Graph, expr: &'a Expr) -> Option<(Operation, Vec<&'a Expr>)> {
    let result = query_nodes(expr, graph, &REFLECT)?;

    let n_c = result.get("n_c")?;
    let _i_c = result.get("i_c")?;

    let n_x = result.get("n_x")?;
    let n_y = result.get("n_y")?;
    let n_z = result.get("n_z")?;

    // TODO: Why does this match the position and not the view vector?
    let i_x = result.get("i_x")?;
    let i_y = result.get("i_y")?;
    let i_z = result.get("i_z")?;

    let args = vec![*i_x, *i_y, *i_z, *n_x, *n_y, *n_z];
    if n_c == n_x {
        Some((Operation::ReflectX, args))
    } else if n_c == n_y {
        Some((Operation::ReflectY, args))
    } else {
        // TODO: Why does matching n_z not work as expected?
        Some((Operation::ReflectZ, args))
    }
}

static FUR_INSTANCE_ALPHA: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
        temp_3 = intBitsToFloat(gl_InstanceID);
        temp_14 = float(floatBitsToInt(temp_3));
        temp_135 = temp_14 * param;
        temp_136 = clamp(temp_135, 0.0, 1.0);
        temp_140 = 0.0 - temp_136;
        temp_141 = temp_140 + 1.0;
        result = temp_141;
    "};
    Graph::parse_glsl_query(query).unwrap().simplify()
});

pub fn op_fur_instance_alpha<'a>(
    graph: &'a Graph,
    expr: &'a Expr,
) -> Option<(Operation, Vec<&'a Expr>)> {
    let result = query_nodes(expr, graph, &FUR_INSTANCE_ALPHA)?;
    let param = result.get("param")?;
    Some((Operation::FurInstanceAlpha, vec![param]))
}

static OP_NORMALIZE_XYZ: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
        void main() {
            length2 = x * x;
            length2 = fma(y, y, length2);
            length2 = fma(z, z, length2);
            inverse_length = inversesqrt(length2);
            result = value * inverse_length;
        }
    "};
    Graph::parse_glsl(query).unwrap().simplify()
});

static OP_NORMALIZE_XCX_XYZ: LazyLock<Graph> = LazyLock::new(|| {
    let query = indoc! {"
        void main() {
            length2 = dot(vec4(x, y, z, 0.0), vec4(x, y, z, 0.0));
            inverse_length = inversesqrt(length2);
            result = value * inverse_length;
        }
    "};
    Graph::parse_glsl(query).unwrap().simplify()
});

pub fn op_normalize<'a>(graph: &'a Graph, expr: &'a Expr) -> Option<(Operation, Vec<&'a Expr>)> {
    let result = query_nodes(expr, graph, &OP_NORMALIZE_XYZ)
        .or_else(|| query_nodes(expr, graph, &OP_NORMALIZE_XCX_XYZ))?;

    let value = result.get("value")?;
    let x = result.get("x")?;
    let y = result.get("y")?;
    let z = result.get("z")?;

    let op = if value == x {
        Operation::NormalizeX
    } else if value == y {
        Operation::NormalizeY
    } else if value == z {
        Operation::NormalizeZ
    } else {
        return None;
    };

    Some((op, vec![x, y, z]))
}

fn latte_texture_cube_query(c: char) -> String {
    // cube.xyzw = cube(R.zzxy, R.yxzz)
    // texture(s0, cube.yx / abs(cube.z) + 1.5))
    formatdoc! {"
        cube_z = 1.0 / abs(cube_z);
        cube_x = cube(R_z, R_y); 
        cube_y = cube(R_z, R_x); 
        result_s = fma(cube_y, cube_z, 1.5);
        result_t = fma(cube_x, cube_z, 1.5);
        result = texture(tex, vec2(result_s, result_t)).{c};
    "}
}

static LATTE_TEXTURE_CUBE_X: LazyLock<Graph> = LazyLock::new(|| {
    let query = latte_texture_cube_query('x');
    Graph::parse_glsl_query(&query).unwrap().simplify()
});

static LATTE_TEXTURE_CUBE_Y: LazyLock<Graph> = LazyLock::new(|| {
    let query = latte_texture_cube_query('y');
    Graph::parse_glsl_query(&query).unwrap().simplify()
});

static LATTE_TEXTURE_CUBE_Z: LazyLock<Graph> = LazyLock::new(|| {
    let query = latte_texture_cube_query('z');
    Graph::parse_glsl_query(&query).unwrap().simplify()
});

static LATTE_TEXTURE_CUBE_W: LazyLock<Graph> = LazyLock::new(|| {
    let query = latte_texture_cube_query('w');
    Graph::parse_glsl_query(&query).unwrap().simplify()
});

pub fn latte_texture_cube_coords(graph: &Graph, expr: &Expr) -> Option<Expr> {
    // Find the reflection vector R from latte cube coordinates.
    let (result, channel) = query_nodes(expr, graph, &LATTE_TEXTURE_CUBE_X)
        .map(|r| (r, 'x'))
        .or_else(|| query_nodes(expr, graph, &LATTE_TEXTURE_CUBE_Y).map(|r| (r, 'y')))
        .or_else(|| query_nodes(expr, graph, &LATTE_TEXTURE_CUBE_Z).map(|r| (r, 'z')))
        .or_else(|| query_nodes(expr, graph, &LATTE_TEXTURE_CUBE_W).map(|r| (r, 'w')))?;

    let args = [
        result.get("tex")?,
        result.get("R_x")?,
        result.get("R_y")?,
        result.get("R_z")?,
    ];

    // Convert a latte specific cube map lookup to a standard GLSL call.
    // TODO: add the vec3 call?
    Some(Expr::Func {
        name: "textureCube".into(),
        args: args
            .into_iter()
            .map(|a| graph.exprs.iter().position(|e| e == *a).unwrap())
            .collect(),
        channel: Some(channel),
    })
}
