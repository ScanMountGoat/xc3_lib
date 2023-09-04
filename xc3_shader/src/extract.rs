use std::path::Path;

use crate::annotation::{annotate_fragment, annotate_vertex};
use xc3_lib::spch::{NvsdMetadata, Slct, Spch};

// TODO: profile performance using a single thread and check threading with tracing?
pub fn extract_shader_binaries<P: AsRef<Path>>(
    spch: &Spch,
    output_folder: P,
    ryujinx_shader_tools: Option<&str>,
    save_binaries: bool,
) {
    let output_folder = output_folder.as_ref();

    spch.shader_programs
        .iter()
        .enumerate()
        .for_each(|(program_index, program)| {
            // Not all programs have associated names.
            // Generate the name to avoid any ambiguity.
            let name = format!("nvsd{program_index}");

            let slct = program.read_slct(&spch.slct_section);
            let binaries = vertex_fragment_binaries(spch, &slct);

            for (i, (vertex, fragment)) in binaries.into_iter().enumerate() {
                // Strip the xv4 headers to work with Ryujinx.ShaderTools.
                let vert_file = output_folder.join(&format!("{name}_VS{i}.bin"));
                std::fs::write(&vert_file, &vertex[48..]).unwrap();

                let frag_file = output_folder.join(&format!("{name}_FS{i}.bin"));
                std::fs::write(&frag_file, &fragment[48..]).unwrap();

                // Each NVSD has separate metadata since the shaders are different.
                let metadata = &slct.nvsds[i].inner;

                // This doesn't need to be parsed, so just use debug output for now.
                let txt_file = output_folder.join(&format!("{name}.txt"));
                let text = format!("{:#?}", &metadata);
                std::fs::write(txt_file, text).unwrap();

                // Decompile using Ryujinx.ShaderTools.exe.
                // There isn't Rust code for this, so just take an exe path.
                if let Some(shader_tools) = &ryujinx_shader_tools {
                    decompile_glsl_shaders(shader_tools, &frag_file, &vert_file, metadata);
                }

                // We needed to temporarily create binaries for ShaderTools to decompile.
                // Delete them if they are no longer needed.
                if !save_binaries {
                    std::fs::remove_file(vert_file).unwrap();
                    std::fs::remove_file(frag_file).unwrap();
                }
            }
        });
}

fn decompile_glsl_shaders(
    shader_tools: &str,
    frag_file: &Path,
    vert_file: &Path,
    metadata: &NvsdMetadata,
) {
    let mut frag_glsl = extract_shader(shader_tools, frag_file);
    let mut vert_glsl = extract_shader(shader_tools, vert_file);

    // Perform annotation here since we need to know the file names.
    vert_glsl = annotate_vertex(vert_glsl, metadata);
    std::fs::write(vert_file.with_extension("glsl"), vert_glsl).unwrap();

    frag_glsl = annotate_fragment(frag_glsl, metadata);
    std::fs::write(frag_file.with_extension("glsl"), frag_glsl).unwrap();
}

fn extract_shader(shader_tools: &str, binary_file: &Path) -> String {
    // TODO: Check exit code?
    String::from_utf8(
        std::process::Command::new(shader_tools)
            .args([binary_file])
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap()
}

fn vertex_fragment_binaries<'a>(spch: &'a Spch, slct: &Slct) -> Vec<(&'a [u8], &'a [u8])> {
    // Each SLCT can have multiple NVSD.
    // Each NVSD has a vertex and fragment shader.
    let offset = slct.xv4_offset;

    let mut binaries = Vec::new();
    for nvsd in &slct.nvsds {
        let vertex = nvsd.inner.vertex_binary(offset, &spch.xv4_section);
        let fragment = nvsd.inner.fragment_binary(offset, &spch.xv4_section);
        binaries.push((vertex, fragment));
    }

    binaries
}
