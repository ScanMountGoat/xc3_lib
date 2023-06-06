use std::path::Path;

use xc3_lib::spch::{ShaderProgram, Spch};

use crate::annotation::{annotate_fragment, annotate_vertex};

pub fn extract_shader_binaries<P: AsRef<Path>>(
    spch: &Spch,
    file_data: &[u8],
    output_folder: P,
    ryujinx_shader_tools: Option<&str>,
    save_binaries: bool,
) {
    for (program, name) in spch
        .shader_programs
        .iter()
        .zip(&spch.string_section.program_names)
    {
        let binaries = vertex_fragment_binaries(spch, program, file_data);

        for (i, (vertex, fragment)) in binaries.into_iter().enumerate() {
            // TODO: Only include i if above 0?
            let vert_file = output_folder.as_ref().join(&format!("{name}_VS{i}.bin"));
            std::fs::write(&vert_file, vertex).unwrap();

            let frag_file = output_folder.as_ref().join(&format!("{name}_FS{i}.bin"));
            std::fs::write(&frag_file, fragment).unwrap();

            // Each NVSD has separate metadata since the shaders are different.
            let metadata = &program.slct.nvsds[i].inner;

            for sampler in &metadata.samplers {
                println!("{:?}", sampler.name);
            }

            let json_file = output_folder.as_ref().join(&format!("{name}.json"));
            let json = serde_json::to_string_pretty(&metadata).unwrap();
            std::fs::write(json_file, json).unwrap();

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

fn vertex_fragment_binaries<'a>(
    spch: &Spch,
    program: &ShaderProgram,
    file_data: &'a [u8],
) -> Vec<(&'a [u8], &'a [u8])> {
    let mut offset = spch.xv4_base_offset as usize + program.slct.xv4_offset as usize;

    // Each SLCT can have multiple NVSD.
    // Each NVSD has a vertex and fragment shader.
    let mut binaries = Vec::new();
    for nvsd in &program.slct.nvsds {
        // The first offset is the vertex shader.
        let vert_size = nvsd.inner.nvsd.vertex_xv4_size as usize;
        // Strip the xV4 header for easier decompilation.
        let vertex = &file_data[offset..offset + vert_size][48..];

        // The fragment shader immediately follows the vertex shader.
        offset += vert_size;
        let frag_size = nvsd.inner.nvsd.fragment_xv4_size as usize;
        let fragment = &file_data[offset..offset + frag_size][48..];
        offset += frag_size;

        binaries.push((vertex, fragment))
    }

    binaries
}
