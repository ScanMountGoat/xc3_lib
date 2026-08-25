use std::borrow::Cow;
use std::collections::BTreeSet;
use std::fmt::Write;
use std::path::PathBuf;
use std::{collections::BTreeMap, sync::LazyLock};

use ahash::HashSet;
use clap::ValueEnum;
use indoc::indoc;
use rayon::prelude::*;
use smol_str::{SmolStr, format_smolstr};
use tracing::{Level, error, span};
use xc3_lib::{mths::Mths, spch::Spch};
use xc3_model::shader_database::{
    AttributeXyz, Operation, ParameterXyz, ProgramHash, ShaderDatabase, ShaderProgram, Value,
};

use crate::expr::xyz::{ExprCacheXyz, merge_xyz_exprs};
use crate::expr::{ExprCache, OutputExpr, Texture, output_expr, parameter};
use crate::extract::nvsd_glsl_name;
use crate::graph::{
    BinaryOp, Expr, Graph, UnaryOp,
    glsl::{GlslGraph, merge_vertex_fragment, shader_source_no_extensions},
    query::{assign_x_recursive, fma_half_half, fma_normalize, query_nodes},
};

mod query;
mod xyz;
use query::*;

// Faster than the default hash implementation.
type IndexMap<K, V> = indexmap::IndexMap<K, V, ahash::RandomState>;
type IndexSet<T> = indexmap::IndexSet<T, ahash::RandomState>;

pub fn shader_from_glsl(
    vertex: Option<GlslGraph>,
    fragment: GlslGraph,
    version: GameVersion,
) -> ShaderProgram {
    let mut exprs = ExprCache::default();

    // This doesn't work with a simplified graph.
    let outline_width = vertex
        .as_ref()
        .map(|v| outline_width_parameter(&v.graph, &mut exprs))
        .unwrap_or_default();

    let frag_attributes = fragment.attributes.clone();

    // Create a combined graph that links vertex outputs to fragment inputs.
    // This effectively moves all shader logic to the fragment shader.
    // This simplifies generating shader code or material nodes in 3D applications.
    let graph = if let Some(vert) = vertex {
        // TODO: How much does it cost to simplify the vertex shader before and after merging?
        merge_vertex_fragment(
            GlslGraph {
                graph: vert.graph.simplify(),
                attributes: vert.attributes.clone(),
            },
            fragment,
            modify_attributes,
        )
    } else {
        fragment.graph
    };
    let graph = graph.simplify();

    let mut output_dependencies = IndexMap::default();
    let mut normal_intensity = None;
    let mut val_inf_intensity = None;

    // Some shaders have up to 8 outputs.
    for i in frag_attributes.output_locations.right_values().copied() {
        for c in "xyzw".chars() {
            let name = format!("out_attr{i}");

            // Search from the end to find fragment outputs instead of vertex outputs.
            // TODO: Label the vertex outputs differently to avoid conflicts?
            let node_index = graph
                .nodes
                .iter()
                .rposition(|n| n.output.name == name && n.output.channel == Some(c))
                .unwrap_or_default();

            // TODO: Skip o3.xyw (depth) and o4.xyz (velocity)
            // TODO: skip using queries or use separate CLI command?
            let value;
            if i == 2 && (c == 'x' || c == 'y') {
                // The normals use XY for output index 2 for all games.
                if let Some((new_value, intensity, inf_intensity)) =
                    normal_output_expr(&graph, node_index, &mut exprs)
                {
                    value = Some(new_value);
                    normal_intensity = intensity;
                    val_inf_intensity = inf_intensity;
                } else {
                    value = None;
                }
            } else if i == 2 && c == 'z' && version == GameVersion::Xcx {
                // XCX and XCX DE only have 2 components for this G-Buffer texture.
                value = None
            } else if i == 2 && c == 'w' {
                // o2.w is n.z * 1000 + 0.5 for XC1 DE, XC2, and XC3.
                // This can be easily handled by consuming applications.
                // XCX and XCX DE only have 2 components.
                value = None;
            } else {
                // Xenoblade X DE uses different outputs than other games.
                // Detect color or params to handle different outputs and channels.
                // TODO: Detect if o2.x before remapping is used here?
                value = color_or_param_output_expr(&graph, node_index, &mut exprs);
            };

            if let Some(value) = value {
                // Simplify the output name to save space.
                output_dependencies.insert(format_smolstr!("o{i}.{c}"), value);
            }
        }
    }

    let discard_condition = graph
        .discard_condition
        .map(|i| output_expr(&graph.exprs[i], &graph, &mut exprs));

    let exprs = exprs.into_exprs();

    for e in &exprs {
        if matches!(e, OutputExpr::Value(crate::expr::Value::Attribute(a)) if a.name.starts_with("vGmCal"))
        {
            error!("Unexpected attribute vGmCal instance transform.");
            break;
        }
    }

    // Don't infer ambient occlusion for shaders with only a color output.
    if version == GameVersion::Xcx && frag_attributes.output_locations.len() > 1 {
        insert_xcx_de_inferred_ambient_occlusion(&mut output_dependencies, &exprs);
    }

    // Merge XYZ channels during database creation to simplify consuming code.
    let mut output_dependencies_xyz = IndexMap::default();

    let mut exprs_xyz = ExprCacheXyz::default();

    for i in frag_attributes.output_locations.right_values() {
        if let (Some(x), Some(y), Some(z)) = (
            output_dependencies.get(&format_smolstr!("o{i}.x")),
            output_dependencies.get(&format_smolstr!("o{i}.y")),
            output_dependencies.get(&format_smolstr!("o{i}.z")),
        ) && let Some(xyz) = merge_xyz_exprs(*x, *y, *z, &exprs, &mut exprs_xyz)
        {
            output_dependencies_xyz.insert(format_smolstr!("o{i}.xyz"), xyz);
        }
    }

    let exprs: Vec<_> = exprs
        .into_iter()
        .map(|e| match e {
            OutputExpr::Value(value) => {
                xc3_model::shader_database::OutputExpr::Value(xc3_value(value))
            }
            OutputExpr::Func { op, args } => {
                xc3_model::shader_database::OutputExpr::Func { op, args }
            }
        })
        .collect();

    let exprs_xyz: Vec<_> = exprs_xyz
        .into_exprs()
        .into_iter()
        .map(|e| match e {
            crate::expr::xyz::OutputExprXyz::Value(value) => {
                xc3_model::shader_database::OutputExprXyz::Value(xc3_value_xyz(value))
            }
            crate::expr::xyz::OutputExprXyz::Func { op, args, channel } => {
                xc3_model::shader_database::OutputExprXyz::Func {
                    op,
                    args,
                    channel: channel.map(xc3_channel_xyz),
                }
            }
        })
        .collect();

    ShaderProgram {
        output_dependencies,
        outline_width: outline_width.map(xc3_value),
        normal_intensity,
        val_inf_intensity,
        discard_condition,
        exprs,
        output_dependencies_xyz,
        exprs_xyz,
    }
}

fn insert_xcx_de_inferred_ambient_occlusion(
    output_dependencies: &mut IndexMap<SmolStr, usize>,
    exprs: &[OutputExpr<Operation>],
) {
    // Simply using a single ambient lighting output channel does not work as ambient occlusion.
    // Assume a single channel texture used for ambient lighting is ambient occlusion.
    // This removes the "lighting" portion of the ambient lighting output.
    // TODO: Is this there a more reliable way to do this using queries?
    let x = output_textures(&*output_dependencies, exprs, "o0.x");
    let y = output_textures(&*output_dependencies, exprs, "o0.y");
    let z = output_textures(&*output_dependencies, exprs, "o0.z");

    let xy: HashSet<_> = x.intersection(&y).cloned().collect();
    let xyz: HashSet<_> = xy.intersection(&z).collect();
    if !xyz.is_empty() {
        let mut channels_by_name = BTreeMap::<_, BTreeSet<_>>::new();
        for t in &xyz {
            channels_by_name
                .entry(t.name.clone())
                .or_default()
                .insert(t.channel);
        }
        let filtered: Vec<_> = channels_by_name
            .iter()
            .filter(|(n, cs)| is_material_texture(n) && cs.len() == 1)
            .collect();
        if let &[(name, _)] = &filtered[..]
            && let Some(ao_tex) = xyz.iter().find(|t| &t.name == name)
            && let Some(expr_index) = exprs.iter().position(|e| {
                e == &OutputExpr::Value(crate::expr::Value::Texture((*ao_tex).clone()))
            })
        {
            output_dependencies.insert("o2.z".into(), expr_index);
        }
    }
}

static OUTLINE_WIDTH_PARAMETER: LazyLock<Graph> = LazyLock::new(|| {
    // This query won't work on a simplified graph, so don't simplify the query.
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

fn outline_width_parameter(
    vert: &Graph,
    exprs: &mut ExprCache<Operation>,
) -> Option<crate::expr::Value> {
    vert.nodes.iter().find_map(|n| {
        // TODO: Add a way to match identifiers like "vColor" exactly.
        let result = query_nodes(&vert.exprs[n.input], vert, &OUTLINE_WIDTH_PARAMETER)?;
        let param = result.get("param")?;
        let vcolor = result.get("vColor")?;

        if matches!(vcolor, Expr::Global { name, channel } if name == "vColor" && *channel == Some('w')) {
            // TODO: Handle other value types?
            parameter(vert, param, exprs).map(crate::expr::Value::Parameter)
        } else {
            None
        }
    })
}

fn color_or_param_output_expr(
    frag: &Graph,
    node_index: usize,
    exprs: &mut ExprCache<Operation>,
) -> Option<usize> {
    let node = frag.nodes.get(node_index)?;

    // matCol.xyz in pcmdo shaders.
    let mut current = &frag.exprs[node.input];

    if let Some(new_current) = geometric_specular_aa(frag, current) {
        current = new_current;
    }

    Some(output_expr(current, frag, exprs))
}

fn normal_output_expr(
    frag: &Graph,
    node_index: usize,
    exprs: &mut ExprCache<Operation>,
) -> Option<(usize, Option<usize>, Option<usize>)> {
    let last_node = frag.nodes.get(node_index)?;

    let mut view_normal = &frag.exprs[last_node.input];

    // setMrtNormal in pcmdo shaders.
    // Xenoblade X uses RG16Float and doesn't require remapping the value range.
    if let Some(new_view_normal) = fma_half_half(frag, view_normal) {
        view_normal = new_view_normal;
    }

    // TODO: Preserve the normal flip for double-sided lighting.
    if let Some(new_view_normal) = flip_backfacing(frag, view_normal) {
        view_normal = new_view_normal;
    }

    // The normal map result is always normalized, so we can infer the channel here.
    let (op, args) = op_normalize(frag, view_normal)?;

    // nomWork input for getCalcNormalMap in pcmdo shaders.
    // TODO: Find a cleaner way to detect separate normal map channels.
    let (nom_work, intensity, val_inf_intensity) = match op {
        Operation::NormalizeX => calc_normal_map_x(frag, args[0])
            .map(|n| (n, None, None))
            .or_else(|| calc_normal_map_val_inf_x(frag, args[0]).map(|(n, i)| (n, None, Some(i))))
            .or_else(|| {
                calc_normal_map_w_intensity_x(frag, args[0]).map(|(n, i)| (n, Some(i), None))
            }),
        Operation::NormalizeY => calc_normal_map_y(frag, args[1])
            .map(|n| (n, None, None))
            .or_else(|| calc_normal_map_val_inf_y(frag, args[1]).map(|(n, i)| (n, None, Some(i))))
            .or_else(|| {
                calc_normal_map_w_intensity_y(frag, args[1]).map(|(n, i)| (n, Some(i), None))
            }),
        _ => {
            // TODO: fix normal map detection for XCX WiiU
            return None;
        }
    }?;

    let value = output_expr(nom_work, frag, exprs);

    let intensity = intensity.map(|i| output_expr(i, frag, exprs));
    let val_inf_intensity = val_inf_intensity.map(|i| output_expr(i, frag, exprs));

    Some((value, intensity, val_inf_intensity))
}

impl crate::expr::Operation for Operation {
    fn query_operation_args<'a>(graph: &'a Graph, expr: &'a Expr) -> Option<(Self, Vec<&'a Expr>)> {
        // Detect operations from most specific to least specific.
        // This results in fewer operations in many cases.
        // TODO: should exp2 should always be part of a pow expression?
        op_add_normal(graph, expr)
            .or_else(|| op_calc_normal_map(graph, expr))
            .or_else(|| op_matmul_proj(graph, expr))
            .or_else(|| op_skin_point_xyzw(graph, expr))
            .or_else(|| op_skin_xyz(graph, expr))
            .or_else(|| op_normalize(graph, expr))
            .or_else(|| op_monochrome(graph, expr))
            .or_else(|| op_fresnel_ratio(graph, expr))
            .or_else(|| op_overlay2(graph, expr))
            .or_else(|| op_overlay_ratio(graph, expr))
            .or_else(|| op_overlay(graph, expr))
            .or_else(|| tex_parallax2(graph, expr))
            .or_else(|| tex_parallax(graph, expr))
            .or_else(|| tex_matrix(graph, expr))
            .or_else(|| op_reflect(graph, expr))
            .or_else(|| op_fur_instance_alpha(graph, expr))
            .or_else(|| op_mix(graph, expr))
            .or_else(|| op_mul_ratio(graph, expr))
            .or_else(|| op_fma(graph, expr))
            .or_else(|| op_sub(graph, expr))
            .or_else(|| op_div(graph, expr))
            .or_else(|| binary_op(graph, expr, BinaryOp::Mul, Operation::Mul))
            .or_else(|| binary_op(graph, expr, BinaryOp::Add, Operation::Add))
            .or_else(|| op_pow(graph, expr))
            .or_else(|| op_func(graph, expr, "clamp", Operation::Clamp))
            .or_else(|| op_func(graph, expr, "min", Operation::Min))
            .or_else(|| op_func(graph, expr, "max", Operation::Max))
            .or_else(|| op_sqrt(graph, expr))
            .or_else(|| op_dot(graph, expr))
            .or_else(|| op_func(graph, expr, "abs", Operation::Abs))
            .or_else(|| op_func(graph, expr, "floor", Operation::Floor))
            .or_else(|| binary_op(graph, expr, BinaryOp::Equal, Operation::Equal))
            .or_else(|| binary_op(graph, expr, BinaryOp::NotEqual, Operation::NotEqual))
            .or_else(|| binary_op(graph, expr, BinaryOp::Less, Operation::Less))
            .or_else(|| binary_op(graph, expr, BinaryOp::Greater, Operation::Greater))
            .or_else(|| binary_op(graph, expr, BinaryOp::LessEqual, Operation::LessEqual))
            .or_else(|| binary_op(graph, expr, BinaryOp::GreaterEqual, Operation::GreaterEqual))
            .or_else(|| binary_op(graph, expr, BinaryOp::LeftShift, Operation::LeftShift))
            .or_else(|| binary_op(graph, expr, BinaryOp::RightShift, Operation::RightShift))
            .or_else(|| binary_op(graph, expr, BinaryOp::And, Operation::And))
            .or_else(|| binary_op(graph, expr, BinaryOp::Or, Operation::Or))
            .or_else(|| binary_op(graph, expr, BinaryOp::BitAnd, Operation::BitAnd))
            .or_else(|| binary_op(graph, expr, BinaryOp::BitOr, Operation::BitOr))
            .or_else(|| binary_op(graph, expr, BinaryOp::BitXor, Operation::BitXor))
            .or_else(|| ternary(graph, expr))
            .or_else(|| unary_op(graph, expr, UnaryOp::Negate, Operation::Negate))
            .or_else(|| unary_op(graph, expr, UnaryOp::Not, Operation::Not))
            .or_else(|| op_func(graph, expr, "float", Operation::Float))
            .or_else(|| op_func(graph, expr, "int", Operation::Int))
            .or_else(|| op_func(graph, expr, "uint", Operation::Uint))
            .or_else(|| op_func(graph, expr, "trunc", Operation::Truncate))
            .or_else(|| op_func(graph, expr, "floatBitsToInt", Operation::FloatBitsToInt))
            .or_else(|| op_func(graph, expr, "floatBitsToUint", Operation::FloatBitsToUint))
            .or_else(|| op_func(graph, expr, "intBitsToFloat", Operation::IntBitsToFloat))
            .or_else(|| op_func(graph, expr, "uintBitsToFloat", Operation::UintBitsToFloat))
            .or_else(|| op_func(graph, expr, "inversesqrt", Operation::InverseSqrt))
            .or_else(|| op_func(graph, expr, "dFdx", Operation::PartialDerivativeX))
            .or_else(|| op_func(graph, expr, "dFdy", Operation::PartialDerivativeY))
            .or_else(|| op_func(graph, expr, "log2", Operation::Log2))
            .or_else(|| op_func(graph, expr, "exp2", Operation::Exp2))
            .or_else(|| op_func(graph, expr, "sin", Operation::Sin))
            .or_else(|| op_func(graph, expr, "cos", Operation::Cos))
            .or_else(|| op_func(graph, expr, "isnan", Operation::IsNaN))
            .or_else(|| {
                error!("Unsupported expression {expr:?}");
                None
            })
    }

    fn preprocess_expr<'a>(graph: &'a Graph, expr: &'a Expr) -> Cow<'a, Expr> {
        // Simplify any expressions that would interfere with queries.
        let mut expr = expr;
        // TODO: Preserve the normal flip for double-sided lighting.
        if let Some(new_expr) = normal_map_fma(graph, expr).or_else(|| flip_backfacing(graph, expr))
        {
            // TODO: Only check for normal map fma for normal maps?
            expr = new_expr;
        }
        // TODO: Detect these as operations.
        // if let Some(new_expr) = attribute_gm_cal_xyz(graph, expr)
        //     .or_else(|| gm_cal_u_bill_color_attribute_w(graph, expr))
        // {
        //     expr = new_expr;
        // }

        if let Some(new_expr) = latte_texture_cube_coords(graph, expr)
            // TODO: Detect these as operations.
            .or_else(|| fma_normalize(graph, expr))
        // .or_else(|| u_mdl_view_bitangent_xyz(graph, expr))
        // .or_else(|| gm_cal_u_bill_attribute_xyzw(graph, expr))
        // .or_else(|| gm_cal_u_nam_attribute_xyzw(graph, expr))
        // .or_else(|| gm_cal_u_nam2_attribute_xyzw(graph, expr))
        // .or_else(|| gm_cal_clip_attribute_xyzw(graph, expr))
        {
            Cow::Owned(new_expr)
        } else {
            Cow::Borrowed(expr)
        }
    }

    fn preprocess_value_expr<'a>(graph: &'a Graph, expr: &'a Expr) -> Cow<'a, Expr> {
        let mut expr = expr;
        if let Some(new_expr) = normal_map_fma(graph, expr) {
            expr = new_expr;
        }

        Cow::Borrowed(expr)
    }
}

pub fn modify_attributes(graph: &Graph, expr: &Expr) -> Expr {
    // Remove attribute skinning if present, so queries can detect globals like "vNormal.x".
    // TODO: preserve the space for attributes like clip or view?

    // TODO: Finish converting these into operations.
    let mut expr = assign_x_recursive(graph, expr);
    // if let Some(new_expr) = skin_attribute_xyzw(graph, expr)
    //     .or_else(|| skin_attribute_xyz(graph, expr))
    //     .or_else(|| skin_attribute_clip_space_xyzw(graph, expr))
    //     .or_else(|| u_mdl_clip_attribute_xyzw(graph, expr))
    //     .or_else(|| u_mdl_view_attribute_xyzw(graph, expr))
    //     .or_else(|| u_mdl_attribute_xyz(graph, expr))
    //     .or_else(|| attribute_gm_cal_xyz(graph, expr))
    //     .or_else(|| gm_cal_u_bill_color_attribute_w(graph, expr))
    // {
    //     expr = new_expr;
    // }

    let mut expr = expr.clone();
    if let Some(new_expr) = skin_attribute_bitangent(graph, &expr)
    //     .or_else(|| u_mdl_view_bitangent_xyz(graph, &expr))
    //     .or_else(|| bitangent_gm_cal_xyz(graph, &expr))
    //     .or_else(|| gm_cal_u_bill_attribute_xyzw(graph, &expr))
    //     .or_else(|| gm_cal_u_nam_attribute_xyzw(graph, &expr))
    //     .or_else(|| gm_cal_u_nam2_attribute_xyzw(graph, &expr))
    //     .or_else(|| gm_cal_clip_attribute_xyzw(graph, &expr))
    {
        expr = new_expr;
    }

    expr
}

struct SpchProgram {
    // Only store one path for now even though different files can have the same hash.
    fragment_path: PathBuf,
    vertex_source: Option<String>,
    fragment_source: Option<String>,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, ValueEnum)]
pub enum GameVersion {
    Xc1,
    Xc2,
    Xc3,
    Xcx,
}

pub fn create_shader_database(input: &str, version: GameVersion) -> ShaderDatabase {
    // Collect unique programs.
    let mut programs = BTreeMap::new();

    // TODO: collect all the file names for a particular hash?
    // The folder structure doesn't matter since we hash the wishp data.
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
        .map(|(hash, p)| {
            let vertex = p.vertex_source.and_then(|s| {
                let source = shader_source_no_extensions(&s);
                match GlslGraph::parse_glsl(source) {
                    Ok(vertex) => Some(vertex),
                    Err(e) => {
                        error!("Error parsing shader: {e}");
                        None
                    }
                }
            });

            // TODO: span level?
            // TODO: set hash as field on the span?
            let span = span!(
                Level::ERROR,
                "shader",
                path = p.fragment_path.display().to_string()
            );

            let shader_program = span.in_scope(|| {
                p.fragment_source
                    .map(|s| {
                        let source = shader_source_no_extensions(&s);
                        match GlslGraph::parse_glsl(source) {
                            Ok(fragment) => shader_from_glsl(vertex, fragment, version),
                            Err(e) => {
                                error!("Error parsing shader: {e}");
                                ShaderProgram::default()
                            }
                        }
                    })
                    .unwrap_or_default()
            });

            (hash, shader_program)
        })
        .collect();

    ShaderDatabase::from_programs(programs)
}

fn add_programs(programs: &mut BTreeMap<ProgramHash, SpchProgram>, spch_path: std::path::PathBuf) {
    if let Ok(spch) = Spch::from_file(&spch_path) {
        for (slct_index, slct_offset) in spch.slct_offsets.iter().enumerate() {
            let slct = slct_offset.read_slct(&spch.slct_section).unwrap();

            for (nvsd_index, (p, vert, frag)) in spch
                .program_data_vertex_fragment_binaries(&slct)
                .iter()
                .enumerate()
            {
                let hash = ProgramHash::from_spch_program(p, vert, frag);

                programs.entry(hash).or_insert_with(|| {
                    let path = spch_path
                        .with_file_name(nvsd_glsl_name(&spch, slct_index, nvsd_index))
                        .with_extension("frag");

                    // TODO: Should the vertex shader be mandatory?
                    let vertex_source = std::fs::read_to_string(path.with_extension("vert")).ok();
                    let fragment_source = std::fs::read_to_string(path.clone()).ok();
                    SpchProgram {
                        fragment_path: path,
                        vertex_source,
                        fragment_source,
                    }
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
            let vertex = match GlslGraph::parse_glsl(&shader.vertex_source) {
                Ok(vertex) => Some(vertex),
                Err(e) => {
                    error!("Error parsing shader: {e}");
                    None
                }
            };

            let fragment = match GlslGraph::parse_glsl(&shader.fragment_source) {
                Ok(vertex) => Some(vertex),
                Err(e) => {
                    error!("Error parsing shader: {e}");
                    None
                }
            };

            let shader_program = fragment
                .map(|fragment| shader_from_glsl(vertex, fragment, GameVersion::Xcx))
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

pub fn shader_str(s: &ShaderProgram) -> String {
    // Use a condensed representation similar to GLSL for nicer diffs.
    // Reindex exprs for each output to produce fewer changes in diffs.
    let mut output = String::new();
    for (k, v) in &s.output_dependencies {
        let mut visited = IndexSet::default();
        write_expr_dependencies_recursive(&mut output, s, *v, &mut visited);
        write_assignment(&mut output, s, k, *v, &mut visited);
        writeln!(&mut output).unwrap();
    }
    if let Some(v) = &s.outline_width {
        let mut visited = IndexSet::default();
        writeln!(
            &mut output,
            "outline_width = {};",
            arg_inlined_value_inner(v, s, &mut visited)
        )
        .unwrap();
        writeln!(&mut output).unwrap();
    }
    if let Some(i) = s.normal_intensity {
        let mut visited = IndexSet::default();
        write_expr_dependencies_recursive(&mut output, s, i, &mut visited);
        write_assignment(&mut output, s, "normal_intensity", i, &mut visited);
        writeln!(&mut output).unwrap();
    }
    if let Some(i) = s.val_inf_intensity {
        let mut visited = IndexSet::default();
        write_expr_dependencies_recursive(&mut output, s, i, &mut visited);
        write_assignment(&mut output, s, "val_inf_intensity", i, &mut visited);
        writeln!(&mut output).unwrap();
    }
    if let Some(i) = s.discard_condition {
        let mut visited = IndexSet::default();
        write_expr_dependencies_recursive(&mut output, s, i, &mut visited);
        write_assignment(&mut output, s, "discard", i, &mut visited);
        writeln!(&mut output).unwrap();
    }
    for (k, v) in &s.output_dependencies_xyz {
        let mut visited = IndexSet::default();
        let mut visited_xyz = IndexSet::default();
        write_expr_xyz_dependencies_recursive(&mut output, s, *v, &mut visited, &mut visited_xyz);
        write_assignment_xyz(&mut output, s, k, *v, &mut visited, &mut visited_xyz);
        writeln!(&mut output).unwrap();
    }

    output
}

// TODO: assume the index is used exactly once because of SSA and never write var{i}?
fn write_assignment(
    output: &mut String,
    s: &ShaderProgram,
    var: &str,
    i: usize,
    old_to_new_index: &mut IndexSet<usize>,
) {
    writeln!(
        output,
        "{var} = {};",
        arg_inlined_value(s, i, old_to_new_index)
    )
    .unwrap();
}

fn write_assignment_xyz(
    output: &mut String,
    s: &ShaderProgram,
    var: &str,
    i: usize,
    old_to_new_index: &mut IndexSet<usize>,
    old_to_new_index_xyz: &mut IndexSet<usize>,
) {
    writeln!(
        output,
        "{var} = {};",
        arg_inlined_value_xyz(s, i, old_to_new_index, old_to_new_index_xyz)
    )
    .unwrap();
}

fn write_expr_dependencies_recursive(
    output: &mut String,
    s: &ShaderProgram,
    i: usize,
    old_to_new_index: &mut IndexSet<usize>,
) {
    // Write all values that this value depends on first.
    if !old_to_new_index.contains(&i) {
        let expr = &s.exprs[i];
        match expr {
            xc3_model::shader_database::OutputExpr::Value(
                xc3_model::shader_database::Value::Texture(t),
            ) => {
                for arg in &t.texcoords {
                    write_expr_dependencies_recursive(output, s, *arg, old_to_new_index);
                }
            }
            xc3_model::shader_database::OutputExpr::Func { op, args } => {
                for arg in args {
                    write_expr_dependencies_recursive(output, s, *arg, old_to_new_index);
                }

                // Write values inline to make the output easier to read.
                let args = args_inlined_values(s, args, old_to_new_index);
                let new_index = old_to_new_index.insert_full(i).0;
                writeln!(output, "var{new_index} = {op}({});", args.join(", ")).unwrap();
            }
            xc3_model::shader_database::OutputExpr::Value(_) => (),
        }
    }
}

fn args_inlined_values(
    s: &ShaderProgram,
    args: &[usize],
    old_to_new_index: &mut IndexSet<usize>,
) -> Vec<String> {
    args.iter()
        .map(|a| arg_inlined_value(s, *a, old_to_new_index))
        .collect()
}

fn arg_inlined_value(
    s: &ShaderProgram,
    i: usize,
    old_to_new_index: &mut IndexSet<usize>,
) -> String {
    match &s.exprs[i] {
        xc3_model::shader_database::OutputExpr::Value(v) => {
            arg_inlined_value_inner(v, s, old_to_new_index)
        }
        _ => format!("var{}", old_to_new_index.insert_full(i).0),
    }
}

fn arg_inlined_value_inner(
    v: &Value,
    s: &ShaderProgram,
    old_to_new_index: &mut indexmap::IndexSet<usize, ahash::RandomState>,
) -> String {
    match v {
        Value::Parameter(p) => {
            format!(
                "{}{}{}{}{}",
                p.name,
                if !p.field.is_empty() {
                    format!(".{}", p.field)
                } else {
                    String::new()
                },
                p.index
                    .map(|i| format!("[{}]", arg_inlined_value(s, i, old_to_new_index)))
                    .unwrap_or_default(),
                p.index2
                    .map(|i| format!("[{}]", arg_inlined_value(s, i, old_to_new_index)))
                    .unwrap_or_default(),
                p.channel.map(|c| format!(".{c}")).unwrap_or_default()
            )
        }
        Value::Texture(t) => {
            let coords: Vec<_> = args_inlined_values(s, &t.texcoords, old_to_new_index);
            format!(
                "Texture({}, {}){}",
                t.name,
                coords.join(", "),
                t.channel.map(|c| format!(".{c}")).unwrap_or_default()
            )
        }
        v => v.to_string(),
    }
}

fn write_expr_xyz_dependencies_recursive(
    output: &mut String,
    s: &crate::database::ShaderProgram,
    i: usize,
    old_to_new_index: &mut IndexSet<usize>,
    old_to_new_index_xyz: &mut IndexSet<usize>,
) {
    // Write all values that this value depends on first.
    if !old_to_new_index_xyz.contains(&i) {
        let expr = &s.exprs_xyz[i];
        match expr {
            xc3_model::shader_database::OutputExprXyz::Value(
                xc3_model::shader_database::ValueXyz::Texture(t),
            ) => {
                for arg in &t.texcoords {
                    write_expr_dependencies_recursive(output, s, *arg, old_to_new_index);
                }
            }
            xc3_model::shader_database::OutputExprXyz::Func { op, args, channel } => {
                for arg in args {
                    write_expr_xyz_dependencies_recursive(
                        output,
                        s,
                        *arg,
                        old_to_new_index,
                        old_to_new_index_xyz,
                    );
                }

                // Write values inline to make the output easier to read.
                let args = args_inlined_values_xyz(s, args, old_to_new_index, old_to_new_index_xyz);
                let new_index = old_to_new_index_xyz.insert_full(i).0;
                writeln!(
                    output,
                    "var{new_index} = {op}({}){};",
                    args.join(", "),
                    channel.map(|c| format!(".{c}")).unwrap_or_default()
                )
                .unwrap();
            }
            xc3_model::shader_database::OutputExprXyz::Value(_) => (),
        }
    }
}

fn args_inlined_values_xyz(
    s: &ShaderProgram,
    args: &[usize],
    old_to_new_index: &mut IndexSet<usize>,
    old_to_new_index_xyz: &mut IndexSet<usize>,
) -> Vec<String> {
    args.iter()
        .map(|a| arg_inlined_value_xyz(s, *a, old_to_new_index, old_to_new_index_xyz))
        .collect()
}

fn arg_inlined_value_xyz(
    s: &ShaderProgram,
    i: usize,
    old_to_new_index: &mut IndexSet<usize>,
    old_to_new_index_xyz: &mut IndexSet<usize>,
) -> String {
    match &s.exprs_xyz[i] {
        xc3_model::shader_database::OutputExprXyz::Value(v) => match v {
            xc3_model::shader_database::ValueXyz::Texture(t) => {
                let coords: Vec<_> = args_inlined_values(s, &t.texcoords, old_to_new_index);
                format!(
                    "Texture({}, {}){}",
                    t.name,
                    coords.join(", "),
                    t.channel.map(|c| format!(".{c}")).unwrap_or_default()
                )
            }
            xc3_model::shader_database::ValueXyz::Parameter(p) => {
                format!(
                    "{}{}{}{}{}",
                    p.name,
                    if !p.field.is_empty() {
                        format!(".{}", p.field)
                    } else {
                        String::new()
                    },
                    p.index
                        .map(|i| format!("[{}]", arg_inlined_value(s, i, old_to_new_index)))
                        .unwrap_or_default(),
                    p.index2
                        .map(|i| format!("[{}]", arg_inlined_value(s, i, old_to_new_index)))
                        .unwrap_or_default(),
                    p.channel.map(|c| format!(".{c}")).unwrap_or_default()
                )
            }
            v => v.to_string(),
        },
        _ => format!("var{}", old_to_new_index_xyz.insert_full(i).0),
    }
}

pub fn shader_graphviz(shader: &ShaderProgram) -> String {
    let mut text = String::new();
    writeln!(&mut text, "digraph {{").unwrap();
    for (i, expr) in shader.exprs.iter().enumerate() {
        let label = match expr {
            xc3_model::shader_database::OutputExpr::Func { op, .. } => op.to_string(),
            xc3_model::shader_database::OutputExpr::Value(Value::Texture(t)) => {
                format!(
                    "{}{}",
                    t.name,
                    t.channel.map(|c| format!(".{c}")).unwrap_or_default()
                )
            }
            xc3_model::shader_database::OutputExpr::Value(d) => d.to_string(),
        };
        writeln!(&mut text, "    {i} [label={label:?}]").unwrap();
    }
    for (i, expr) in shader.exprs.iter().enumerate() {
        match expr {
            xc3_model::shader_database::OutputExpr::Func { args, .. } => {
                for arg in args {
                    writeln!(&mut text, "    {arg} -> {i}").unwrap();
                }
            }
            xc3_model::shader_database::OutputExpr::Value(Value::Texture(t)) => {
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

fn xc3_value(value: crate::expr::Value) -> xc3_model::shader_database::Value {
    match value {
        crate::expr::Value::Int(i) => xc3_model::shader_database::Value::Int(i),
        crate::expr::Value::Uint(u) => xc3_model::shader_database::Value::Uint(u),
        crate::expr::Value::Float(f) => xc3_model::shader_database::Value::Float(f),
        crate::expr::Value::Bool(b) => xc3_model::shader_database::Value::Bool(b),
        crate::expr::Value::Parameter(parameter) => {
            xc3_model::shader_database::Value::Parameter(xc3_model::shader_database::Parameter {
                name: parameter.name,
                field: parameter.field,
                index: parameter.index,
                index2: parameter.index2,
                channel: parameter.channel,
            })
        }
        crate::expr::Value::Texture(texture) => {
            xc3_model::shader_database::Value::Texture(xc3_model::shader_database::Texture {
                name: texture.name,
                channel: texture.channel,
                texcoords: texture.texcoords,
            })
        }
        crate::expr::Value::Attribute(attribute) => {
            xc3_model::shader_database::Value::Attribute(xc3_model::shader_database::Attribute {
                name: attribute.name,
                channel: attribute.channel,
            })
        }
    }
}

fn xc3_value_xyz(value: crate::expr::xyz::ValueXyz) -> xc3_model::shader_database::ValueXyz {
    match value {
        crate::expr::xyz::ValueXyz::Texture(t) => {
            xc3_model::shader_database::ValueXyz::Texture(xc3_model::shader_database::TextureXyz {
                name: t.name,
                texcoords: t.texcoords,
                channel: t.channel.map(xc3_channel_xyz),
            })
        }
        crate::expr::xyz::ValueXyz::Attribute(a) => {
            xc3_model::shader_database::ValueXyz::Attribute(AttributeXyz {
                name: a.name,
                channel: a.channel.map(xc3_channel_xyz),
            })
        }
        crate::expr::xyz::ValueXyz::Parameter(p) => {
            xc3_model::shader_database::ValueXyz::Parameter(ParameterXyz {
                name: p.name,
                field: p.field,
                index: p.index,
                index2: p.index2,
                channel: p.channel.map(xc3_channel_xyz),
            })
        }
        crate::expr::xyz::ValueXyz::Float(f) => xc3_model::shader_database::ValueXyz::Float(f),
    }
}

fn xc3_channel_xyz(value: crate::expr::xyz::ChannelXyz) -> xc3_model::shader_database::ChannelXyz {
    match value {
        crate::expr::xyz::ChannelXyz::Xyz => xc3_model::shader_database::ChannelXyz::Xyz,
        crate::expr::xyz::ChannelXyz::X => xc3_model::shader_database::ChannelXyz::X,
        crate::expr::xyz::ChannelXyz::Y => xc3_model::shader_database::ChannelXyz::Y,
        crate::expr::xyz::ChannelXyz::Z => xc3_model::shader_database::ChannelXyz::Z,
        crate::expr::xyz::ChannelXyz::W => xc3_model::shader_database::ChannelXyz::W,
    }
}

fn is_material_texture(name: &str) -> bool {
    // "s11" -> true, "s_tex1" -> false
    name.strip_prefix("s")
        .map(|n| n.parse::<usize>().is_ok())
        .unwrap_or_default()
}

fn output_textures(
    output_dependencies: &IndexMap<SmolStr, usize>,
    exprs: &[OutputExpr<Operation>],
    name: &str,
) -> HashSet<Texture> {
    let mut textures = HashSet::default();
    if let Some(i) = output_dependencies.get(name) {
        add_textures(*i, exprs, &mut textures);
    }
    textures
}

fn add_textures(i: usize, exprs: &[OutputExpr<Operation>], textures: &mut HashSet<Texture>) {
    match &exprs[i] {
        OutputExpr::Value(value) => {
            if let crate::expr::Value::Texture(t) = value {
                textures.insert(t.clone());
            }
        }
        OutputExpr::Func { args, .. } => {
            for a in args {
                add_textures(*a, exprs, textures);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use insta::assert_snapshot;

    macro_rules! assert_shader_snapshot {
        ($folder:expr, $name: expr, $index:expr) => {
            let vert_glsl =
                include_str!(concat!("data/", $folder, "/", $name, ".", $index, ".vert"));
            let frag_glsl =
                include_str!(concat!("data/", $folder, "/", $name, ".", $index, ".frag"));
            let vertex = GlslGraph::parse_glsl(vert_glsl).unwrap();
            let fragment = GlslGraph::parse_glsl(frag_glsl).unwrap();

            let version = match $folder {
                "xc1" => GameVersion::Xc1,
                "xc2" => GameVersion::Xc2,
                "xc3" => GameVersion::Xc3,
                "xcx" | "xcxde" => GameVersion::Xcx,
                _ => todo!(),
            };

            let shader = shader_from_glsl(Some(vertex), fragment, version);

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

    // TODO: bl302101 9

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
    fn shader_from_glsl_mio_dress() {
        // xeno3/chr/ch/ch01027000, "body_toon", shd0087
        // Detect color and normal layering.
        assert_shader_snapshot!("xc3", "ch01027000", "87");
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
    fn shader_from_glsl_dromarch_fur() {
        // xeno2/bl/bl000501, "fur_Fur", shd0006
        // Check instanced fur shell rendering
        assert_shader_snapshot!("xc2", "bl000501", "6");
    }

    #[test]
    fn shader_from_glsl_xc2_ma30a_branches() {
        // xeno2/map/ma30a, props, "TR0502d_BaseTrunkA", shd0001
        // Test multiple normal layers and prop instancing.
        assert_shader_snapshot!("xc2", "ma30a.props", "1");
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

    #[test]
    fn shader_from_glsl_vandham_eye() {
        // xenox/chr_np/np002101, "np002101hair", 8
        // Check latte cube map instructions.
        assert_shader_snapshot!("xcx", "np002101", "8");
    }
}
