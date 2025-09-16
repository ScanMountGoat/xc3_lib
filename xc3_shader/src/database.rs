use std::borrow::Cow;
use std::fmt::Write;
use std::{collections::BTreeMap, sync::LazyLock};

use glsl_lang::{ast::TranslationUnit, parse::DefaultParse};
use indexmap::{IndexMap, IndexSet};
use indoc::indoc;
use log::error;
use rayon::prelude::*;
use xc3_lib::{mths::Mths, spch::Spch};
use xc3_model::shader_database::{
    Dependency, Operation, ProgramHash, ShaderDatabase, ShaderProgram,
};

use crate::expr::{output_expr, OutputExpr, Value};
use crate::graph::glsl::{find_attribute_locations, merge_vertex_fragment};
use crate::graph::UnaryOp;
use crate::{
    dependencies::buffer_dependency,
    extract::nvsd_glsl_name,
    graph::{
        glsl::shader_source_no_extensions,
        query::{assign_x, assign_x_recursive, fma_half_half, normalize, query_nodes},
        BinaryOp, Expr, Graph,
    },
};

mod query;
use query::*;

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
        outline_width: outline_width.map(Into::into),
        normal_intensity,
        exprs: exprs
            .into_iter()
            .map(|e| match e {
                OutputExpr::Value(value) => {
                    xc3_model::shader_database::OutputExpr::Value(value.into())
                }
                OutputExpr::Func { op, args } => {
                    xc3_model::shader_database::OutputExpr::Func { op, args }
                }
            })
            .collect(),
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

fn outline_width_parameter(vert: &Graph) -> Option<Value> {
    vert.nodes.iter().find_map(|n| {
        // TODO: Add a way to match identifiers like "vColor" exactly.
        let result = query_nodes(&vert.exprs[n.input], vert, &OUTLINE_WIDTH_PARAMETER)?;
        let param = result.get("param")?;
        let vcolor = result.get("vColor")?;

        if matches!(vcolor, Expr::Global { name, channel } if name == "vColor" && *channel == Some('w')) {
            // TODO: Handle other dependency types?
            buffer_dependency(vert, param).map(Value::Parameter)
        } else {
            None
        }
    })
}

fn color_or_param_output_expr(
    frag: &Graph,
    dependent_lines: &[usize],
    exprs: &mut IndexSet<OutputExpr<Operation>>,
    expr_to_index: &mut IndexMap<Expr, usize>,
) -> Option<usize> {
    let last_node_index = *dependent_lines.last()?;
    let last_node = frag.nodes.get(last_node_index)?;

    // matCol.xyz in pcmdo shaders.
    let mut current = &frag.exprs[last_node.input];

    // Remove some redundant float -> int float -> conversions found in some shaders.
    if let Expr::Func { name, args, .. } = current {
        if name == "intBitsToFloat" {
            let new_current = assign_x_recursive(frag, &frag.exprs[args[0]]);

            if let Expr::Func { name, args, .. } = new_current {
                if name == "floatBitsToInt" {
                    current = &frag.exprs[args[0]];
                }
            }
        }
    }

    current = assign_x_recursive(frag, current);

    if let Some(new_current) = geometric_specular_aa(frag, current) {
        current = new_current;
    }

    Some(output_expr(current, frag, exprs, expr_to_index))
}

fn normal_output_expr(
    frag: &Graph,
    dependent_lines: &[usize],
    exprs: &mut IndexSet<OutputExpr<Operation>>,
    expr_to_index: &mut IndexMap<Expr, usize>,
) -> Option<(usize, Option<usize>)> {
    let last_node_index = *dependent_lines.last()?;
    let last_node = frag.nodes.get(last_node_index)?;

    let mut view_normal = assign_x(frag, &frag.exprs[last_node.input])?;

    // setMrtNormal in pcmdo shaders.
    // Xenoblade X uses RG16Float and doesn't require remapping the value range.
    if let Some(new_view_normal) = fma_half_half(frag, view_normal) {
        view_normal = new_view_normal;
    }
    view_normal = assign_x_recursive(frag, view_normal);
    view_normal = normalize(frag, view_normal)?;
    view_normal = assign_x_recursive(frag, view_normal);

    // TODO: front facing in calcNormalZAbs in pcmdo?

    // nomWork input for getCalcNormalMap in pcmdo shaders.
    let (nom_work, intensity) = calc_normal_map(frag, view_normal)
        .map(|n| (n, None))
        .or_else(|| calc_normal_map_xcx(frag, view_normal).map(|n| (n, None)))
        .or_else(|| calc_normal_map_w_intensity(frag, view_normal).map(|(n, i)| (n, Some(i))))?;

    let nom_work = match last_node.output.channel {
        Some('x') => nom_work[0],
        Some('y') => nom_work[1],
        Some('z') => nom_work[2],
        _ => nom_work[0],
    };
    let nom_work = assign_x_recursive(frag, nom_work);

    let value = output_expr(nom_work, frag, exprs, expr_to_index);

    let intensity = intensity.map(|i| output_expr(i, frag, exprs, expr_to_index));

    Some((value, intensity))
}

impl crate::expr::Operation for Operation {
    fn query_operation_args<'a>(graph: &'a Graph, expr: &'a Expr) -> Option<(Self, Vec<&'a Expr>)> {
        // Detect operations from most specific to least specific.
        // This results in fewer operations in many cases.
        // TODO: inversesqrt
        // TODO: exp2 should always be part of a pow expression
        op_add_normal(graph, expr)
            .or_else(|| op_monochrome(graph, expr))
            .or_else(|| op_fresnel_ratio(graph, expr))
            .or_else(|| op_overlay2(graph, expr))
            .or_else(|| op_overlay_ratio(graph, expr))
            .or_else(|| op_overlay(graph, expr))
            .or_else(|| tex_parallax2(graph, expr))
            .or_else(|| tex_parallax(graph, expr))
            .or_else(|| tex_matrix(graph, expr))
            .or_else(|| op_reflect(graph, expr))
            .or_else(|| op_calc_normal_map(graph, expr))
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
            .or_else(|| ternary(graph, expr))
            .or_else(|| unary_op(graph, expr, UnaryOp::Negate, Operation::Negate))
    }

    fn preprocess_expr<'a>(graph: &'a Graph, expr: &'a Expr) -> Cow<'a, Expr> {
        // Simplify any expressions that would interfere with queries.
        let mut expr = assign_x_recursive(graph, expr);
        if let Some(new_expr) = normal_map_fma(graph, expr) {
            expr = new_expr;
        }
        if let Some(new_expr) = normalize(graph, expr) {
            expr = assign_x_recursive(graph, new_expr);
        }

        // Detect attributes.
        // TODO: preserve the space for attributes like clip or view?
        if let Some(new_expr) = skin_attribute_xyzw(graph, expr)
            .or_else(|| skin_attribute_xyz(graph, expr))
            .or_else(|| skin_attribute_clip_space_xyzw(graph, expr))
            .or_else(|| u_mdl_clip_attribute_xyzw(graph, expr))
            .or_else(|| u_mdl_view_attribute_xyzw(graph, expr))
            .or_else(|| u_mdl_attribute_xyz(graph, expr))
        {
            expr = new_expr;
        }

        let mut expr = expr.clone();
        if let Some(new_expr) = skin_attribute_bitangent(graph, &expr)
            .or_else(|| u_mdl_view_bitangent_xyz(graph, &expr))
        {
            expr = new_expr;
        }

        Cow::Owned(expr)
    }

    fn preprocess_value_expr<'a>(graph: &'a Graph, expr: &'a Expr) -> Cow<'a, Expr> {
        let mut expr = assign_x_recursive(graph, expr);
        if let Some(new_expr) = normalize(graph, expr) {
            expr = new_expr;
        }
        if let Some(new_expr) = normal_map_fma(graph, expr) {
            expr = new_expr;
        }

        Cow::Borrowed(expr)
    }
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
        xc3_model::shader_database::OutputExpr::Value(Dependency::Texture(t)) => {
            let args: Vec<_> = t.texcoords.iter().map(|a| expr_str(s, *a)).collect();
            format!(
                "Texture({}, {}){}",
                t.name,
                args.join(", "),
                t.channel.map(|c| format!(".{c}")).unwrap_or_default()
            )
        }
        xc3_model::shader_database::OutputExpr::Func { op, args } => {
            let args: Vec<_> = args.iter().map(|a| expr_str(s, *a)).collect();
            format!("{op}({})", args.join(", "))
        }
        xc3_model::shader_database::OutputExpr::Value(v) => v.to_string(),
    }
}

pub fn shader_graphviz(shader: &ShaderProgram) -> String {
    let mut text = String::new();
    writeln!(&mut text, "digraph {{").unwrap();
    for (i, expr) in shader.exprs.iter().enumerate() {
        let label = match expr {
            xc3_model::shader_database::OutputExpr::Func { op, .. } => op.to_string(),
            xc3_model::shader_database::OutputExpr::Value(Dependency::Texture(t)) => {
                format!(
                    "{}{}",
                    &t.name,
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
            xc3_model::shader_database::OutputExpr::Value(Dependency::Texture(t)) => {
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

    use insta::assert_snapshot;

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
