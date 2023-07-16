use std::path::Path;

use crate::annotation::{annotate_fragment, annotate_vertex};
use xc3_lib::spch::{Slct, Spch};

pub fn extract_shader_binaries<P: AsRef<Path>>(
    spch: &Spch,
    output_folder: P,
    ryujinx_shader_tools: Option<&str>,
    save_binaries: bool,
) {
    // TODO: Handle missing program names?
    for (program, name) in spch
        .shader_programs
        .iter()
        .zip(&spch.string_section.as_ref().unwrap().program_names)
    {
        let slct = program.read_slct(&spch.slct_section);

        let binaries = vertex_fragment_binaries(spch, &slct);

        for (i, (vertex, fragment)) in binaries.into_iter().enumerate() {
            // TODO: Only include i if above 0?
            let vert_file = output_folder.as_ref().join(&format!("{name}_VS{i}.bin"));
            std::fs::write(&vert_file, vertex).unwrap();

            let frag_file = output_folder.as_ref().join(&format!("{name}_FS{i}.bin"));
            std::fs::write(&frag_file, fragment).unwrap();

            // Each NVSD has separate metadata since the shaders are different.
            let metadata = &slct.nvsds[i].inner;

            // This doesn't need to be parsed, so just use debug output for now.
            let txt_file = output_folder.as_ref().join(&format!("{name}.txt"));
            let text = format!("{:#?}", &metadata);
            std::fs::write(txt_file, text).unwrap();

            // Decompile using Ryujinx.ShaderTools.exe.
            // There isn't Rust code for this, so just take an exe path.
            if let Some(shader_tools) = &ryujinx_shader_tools {
                std::process::Command::new(shader_tools)
                    .args([&vert_file, &vert_file.with_extension("glsl")])
                    .output()
                    .unwrap();

                std::process::Command::new(shader_tools)
                    .args([&frag_file, &frag_file.with_extension("glsl")])
                    .output()
                    .unwrap();

                // Perform annotation here since we need to know the file names.
                let mut vert_glsl =
                    std::fs::read_to_string(vert_file.with_extension("glsl")).unwrap();
                let mut frag_glsl =
                    std::fs::read_to_string(frag_file.with_extension("glsl")).unwrap();

                vert_glsl = annotate_vertex(vert_glsl, metadata);
                std::fs::write(vert_file.with_extension("glsl"), vert_glsl).unwrap();

                frag_glsl = annotate_fragment(frag_glsl, metadata);
                std::fs::write(frag_file.with_extension("glsl"), frag_glsl).unwrap();
            }

            // We need to temporarily create binaries for ShaderTools to decompile.
            // Delete them if they are no longer needed.
            if !save_binaries {
                std::fs::remove_file(vert_file).unwrap();
                std::fs::remove_file(frag_file).unwrap();
            }
        }
    }
}

fn vertex_fragment_binaries<'a>(spch: &'a Spch, slct: &Slct) -> Vec<(&'a [u8], &'a [u8])> {
    let mut offset = slct.xv4_offset as usize;

    // Each SLCT can have multiple NVSD.
    // Each NVSD has a vertex and fragment shader.
    let mut binaries = Vec::new();
    for nvsd in &slct.nvsds {
        // The first offset is the vertex shader.
        let vert_size = nvsd.inner.nvsd.vertex_xv4_size as usize;
        // Strip the xV4 header for easier decompilation.
        let vertex = &spch.xv4_section[offset..offset + vert_size][48..];

        // The fragment shader immediately follows the vertex shader.
        offset += vert_size;
        let frag_size = nvsd.inner.nvsd.fragment_xv4_size as usize;
        let fragment = &spch.xv4_section[offset..offset + frag_size][48..];
        offset += frag_size;

        binaries.push((vertex, fragment))
    }

    binaries
}
