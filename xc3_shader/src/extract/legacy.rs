use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write,
    fs::File,
    path::Path,
};

use crate::graph::{Expr, Graph};
use log::error;
use xc3_lib::mths::Mths;

pub fn extract_legacy_shaders<P: AsRef<Path>>(
    mths: &Mths,
    mths_bytes: &[u8],
    output_folder: P,
    gfd_tool: &str,
    index: usize,
) {
    let output_folder = output_folder.as_ref();

    // Save the binary for creating the database later.
    std::fs::write(output_folder.join(format!("{index}.cashd")), mths_bytes).unwrap();

    if let Ok(vert) = mths.vertex_shader() {
        let binary_path = output_folder.join(format!("{index}.vert.bin"));
        dissassemble_shader(&binary_path, &vert.program_binary, gfd_tool);
    }

    if let Ok(frag) = mths.pixel_shader() {
        let binary_path = output_folder.join(format!("{index}.frag.bin"));
        dissassemble_shader(&binary_path, &frag.program_binary, gfd_tool);
    }
}

pub fn annotate_legacy_shaders<P: AsRef<Path>>(
    mths: &Mths,
    mths_bytes: &[u8],
    output_folder: P,
    index: usize,
    technique: Option<&xc3_lib::mxmd::legacy::Technique>,
) {
    let output_folder = output_folder.as_ref();

    // Save the binary for creating the database later.
    std::fs::write(output_folder.join(format!("{index}.cashd")), mths_bytes).unwrap();

    let mut vertex_output_locations = Vec::new();

    if let Ok(vert) = mths.vertex_shader() {
        let binary_path = output_folder.join(format!("{index}.vert.bin"));
        annotate_vertex_shader(&binary_path, &vert, technique, &mut vertex_output_locations);
    }

    if let Ok(frag) = mths.pixel_shader() {
        let binary_path = output_folder.join(format!("{index}.frag.bin"));
        annotate_fragment_shader(&binary_path, &frag, technique, &vertex_output_locations);
    }
}

fn dissassemble_shader(binary_path: &Path, binary: &[u8], gfd_tool: &str) {
    std::fs::write(binary_path, binary).unwrap();

    std::process::Command::new(gfd_tool)
        .arg("disassemble")
        .arg(binary_path)
        .stdout(File::create(binary_path.with_extension("txt")).unwrap())
        .spawn()
        .unwrap()
        .wait()
        .unwrap();

    // TODO: add an option to preserve binaries?
    std::fs::remove_file(binary_path).unwrap();
}

// TODO: Share code with fragment.
// TODO: Tests for annotation
fn annotate_vertex_shader(
    binary_path: &Path,
    shader: &xc3_lib::mths::Gx2VertexShader,
    technique: Option<&xc3_lib::mxmd::legacy::Technique>,
    vertex_output_locations: &mut Vec<usize>,
) {
    // TODO: perform annotation here and output glsl?
    // TODO: annotation will require the technique since attributes and params are just "Q"?
    // TODO: Construct syntatically valid GLSL for parsing later?
    let text = std::fs::read_to_string(binary_path.with_extension("txt")).unwrap();
    let mut graph = Graph::from_latte_asm(&text);

    let output_count = shader
        .registers
        .spi_vs_out_id
        .iter()
        .flat_map(|id| id.to_be_bytes())
        .filter(|i| *i != 0xFF)
        .count();

    // Vertex output locations be remapped by registers.
    // https://github.com/decaf-emu/decaf-emu/blob/e6c528a20a41c34e0f9eb91dd3da40f119db2dee/src/libgpu/src/spirv/spirv_transpiler.cpp#L280-L301
    for output_index in 0..output_count {
        let mut i = 0;
        for register in &shader.registers.spi_vs_out_id {
            // The order is [id3, id2, id1, id0];
            for id in &register.to_le_bytes() {
                if *id as usize == output_index {
                    vertex_output_locations.push(i);
                }

                i += 1;
            }
        }
    }

    for node in &mut graph.nodes {
        if node.output.name.starts_with("PARAM") {
            let index: usize = node
                .output
                .name
                .trim_start_matches("PARAM")
                .parse()
                .unwrap();

            node.output.name = format!("out_attr{index}").into();
        }

        replace_uniform_blocks(&mut node.input, &shader.uniform_blocks);
    }
    let glsl = graph.to_glsl();

    let mut annotated = String::new();

    write_uniform_blocks(&mut annotated, &shader.uniform_blocks);

    // TODO: Create metadata and annotate the GLSL instead?
    let mut attribute_names = BTreeMap::new();
    if let Some(technique) = technique {
        for attribute in &shader.attributes {
            let technique_attribute = technique
                .attributes
                .get(attribute.location as usize)
                .unwrap();

            let name = attribute_name(technique_attribute.data_type);

            // TODO: var type isn't always vec4?
            writeln!(
                &mut annotated,
                "layout(location = {}) in vec4 {};",
                attribute.location, name
            )
            .unwrap();

            attribute_names.insert(attribute.location, name);
        }
    }

    for i in 0..vertex_output_locations.len() {
        // TODO: is the type always vec4?
        writeln!(
            &mut annotated,
            "layout(location = {i}) out vec4 out_attr{i};"
        )
        .unwrap();
    }

    writeln!(&mut annotated, "void main() {{").unwrap();

    // Vertex input attribute registers can also be remapped.
    for (i, location) in shader
        .registers
        .sq_vtx_semantic
        .iter()
        .enumerate()
        .take(shader.registers.num_sq_vtx_semantic as usize)
    {
        if *location != 0xFF {
            if let Some(name) = attribute_names.get(location) {
                // Register 0 is special, so we need to start with register 1.
                for c in "xyzw".chars() {
                    writeln!(&mut annotated, "    R{}.{c} = {name}.{c};", i + 1).unwrap();
                }
            } else {
                error!("Unable to find name for attribute location {location}");
            }
        }
    }

    for line in glsl.lines() {
        writeln!(&mut annotated, "    {line}").unwrap();
    }

    writeln!(&mut annotated, "}}").unwrap();

    std::fs::write(binary_path.with_extension(""), annotated).unwrap();
}

fn write_uniform_blocks(annotated: &mut String, blocks: &[xc3_lib::mths::UniformBlock]) {
    for block in blocks {
        writeln!(
            annotated,
            "layout(binding = {}, std140) uniform _{} {{",
            block.offset, &block.name
        )
        .unwrap();
        // TODO: Add uniform variables.
        writeln!(annotated, "    vec4 values[{}];", block.size / 16).unwrap();
        writeln!(annotated, "}} {};", &block.name).unwrap();
    }
}

fn replace_uniform_blocks(e: &mut Expr, blocks: &[xc3_lib::mths::UniformBlock]) {
    // TODO: Create iterator that visits mutable expressions?
    match e {
        Expr::Node { .. } => (),
        Expr::Float(_) => (),
        Expr::Int(_) => (),
        Expr::Uint(_) => (),
        Expr::Bool(_) => (),
        Expr::Parameter { name, field, .. } => match name.as_str() {
            "KC0" => {
                // TODO: What is the correct way to map KC0 to uniform blocks?
                if let Some(block) = blocks.iter().find(|b| b.offset == 1) {
                    *field = Some("values".into());
                    *name = (&block.name).into();
                }
            }
            "KC1" => {
                // TODO: What is the correct way to map KC1 to uniform blocks?
                if let Some(block) = blocks.iter().find(|b| b.offset == 2) {
                    *field = Some("values".into());
                    *name = (&block.name).into();
                }
            }
            _ => (),
        },
        Expr::Global { .. } => (),
        Expr::Unary(_, expr) => replace_uniform_blocks(expr, blocks),
        Expr::Binary(_, expr, expr1) => {
            replace_uniform_blocks(expr, blocks);
            replace_uniform_blocks(expr1, blocks);
        }
        Expr::Ternary(expr, expr1, expr2) => {
            replace_uniform_blocks(expr, blocks);
            replace_uniform_blocks(expr1, blocks);
            replace_uniform_blocks(expr2, blocks);
        }
        Expr::Func { args, .. } => {
            for arg in args {
                replace_uniform_blocks(arg, blocks);
            }
        }
    }
}

fn attribute_name(d: xc3_lib::vertex::DataType) -> &'static str {
    match d {
        xc3_lib::vertex::DataType::Position => "vPos",
        xc3_lib::vertex::DataType::SkinWeights2 => "fWeight",
        xc3_lib::vertex::DataType::BoneIndices2 => todo!(),
        xc3_lib::vertex::DataType::WeightIndex => "nWgtIdx",
        xc3_lib::vertex::DataType::WeightIndex2 => "nWgtIdx",
        xc3_lib::vertex::DataType::TexCoord0 => "vTex0",
        xc3_lib::vertex::DataType::TexCoord1 => "vTex1",
        xc3_lib::vertex::DataType::TexCoord2 => "vTex2",
        xc3_lib::vertex::DataType::TexCoord3 => "vTex3",
        xc3_lib::vertex::DataType::TexCoord4 => "vTex4",
        xc3_lib::vertex::DataType::TexCoord5 => "vTex5",
        xc3_lib::vertex::DataType::TexCoord6 => "vTex6",
        xc3_lib::vertex::DataType::TexCoord7 => "vTex7",
        xc3_lib::vertex::DataType::TexCoord8 => "vTex8",
        xc3_lib::vertex::DataType::Blend => "vBlend",
        xc3_lib::vertex::DataType::Unk15 => "Unk15",
        xc3_lib::vertex::DataType::Unk16 => "Unk16",
        xc3_lib::vertex::DataType::VertexColor => "vColor",
        xc3_lib::vertex::DataType::Unk18 => "Unk18",
        xc3_lib::vertex::DataType::Unk24 => "vGmCal1",
        xc3_lib::vertex::DataType::Unk25 => "vGmCal2",
        xc3_lib::vertex::DataType::Unk26 => "vGmCal3",
        xc3_lib::vertex::DataType::Normal => "vNormal",
        xc3_lib::vertex::DataType::Tangent => "vTan",
        xc3_lib::vertex::DataType::Unk30 => "fGmAL",
        xc3_lib::vertex::DataType::Unk31 => "Unk31",
        xc3_lib::vertex::DataType::Normal2 => "vNormal",
        xc3_lib::vertex::DataType::ValInf => "vValInf",
        xc3_lib::vertex::DataType::Normal3 => "vNormal",
        xc3_lib::vertex::DataType::VertexColor3 => "vColor",
        xc3_lib::vertex::DataType::Position2 => "vPos",
        xc3_lib::vertex::DataType::Normal4 => "vNormal",
        xc3_lib::vertex::DataType::OldPosition => "vOldPos",
        xc3_lib::vertex::DataType::Tangent2 => "vTan",
        xc3_lib::vertex::DataType::SkinWeights => todo!(),
        xc3_lib::vertex::DataType::BoneIndices => todo!(),
        xc3_lib::vertex::DataType::Flow => "vFlow",
    }
}

fn annotate_fragment_shader(
    binary_path: &Path,
    shader: &xc3_lib::mths::Gx2PixelShader,
    _technique: Option<&xc3_lib::mxmd::legacy::Technique>,
    vertex_output_locations: &[usize],
) {
    // TODO: perform annotation here and output glsl?
    // TODO: annotation will require the technique since attributes and params are just "Q"?
    // TODO: Construct syntatically valid GLSL for parsing later?
    let text = std::fs::read_to_string(binary_path.with_extension("txt")).unwrap();
    let mut graph = Graph::from_latte_asm(&text);

    for node in &mut graph.nodes {
        if let Expr::Func { name, args, .. } = &mut node.input {
            if name.starts_with("texture") {
                if let Some(Expr::Global { name, .. }) = args.first_mut() {
                    // texture(t1, ...) -> texture(s1, ...)
                    *name = name.replace("t", "s").into();
                }
            }
        }

        replace_uniform_blocks(&mut node.input, &shader.uniform_blocks);
    }

    let mut fragment_outputs = BTreeSet::new();
    for node in &mut graph.nodes {
        if node.output.name.starts_with("PIX") {
            let index: usize = node.output.name.trim_start_matches("PIX").parse().unwrap();
            fragment_outputs.insert(index);

            node.output.name = format!("out_attr{index}").into();
        }
    }

    let glsl = graph.to_glsl();

    let mut annotated = String::new();

    write_uniform_blocks(&mut annotated, &shader.uniform_blocks);

    for (i, location) in vertex_output_locations.iter().enumerate() {
        writeln!(
            &mut annotated,
            "layout(location = {location}) in vec4 in_attr{i};"
        )
        .unwrap();
    }

    for i in fragment_outputs.iter() {
        writeln!(
            &mut annotated,
            "layout(location = {i}) out vec4 out_attr{i};"
        )
        .unwrap();
    }

    writeln!(&mut annotated, "void main() {{").unwrap();

    // Fragment input attributes initialize R0, R1, ...?
    for i in 0..vertex_output_locations.len() {
        for c in "xyzw".chars() {
            writeln!(&mut annotated, "    R{i}.{c} = in_attr{i}.{c};").unwrap();
        }
    }

    for line in glsl.lines() {
        writeln!(&mut annotated, "    {line}").unwrap();
    }

    writeln!(&mut annotated, "}}").unwrap();

    std::fs::write(binary_path.with_extension(""), annotated).unwrap();
}
