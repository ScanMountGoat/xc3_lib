use std::path::Path;

use crate::annotation::{annotate_fragment, annotate_vertex};
use log::error;
use rayon::prelude::*;
use xc3_lib::spch::{Nvsd, Spch};

// TODO: profile performance using a single thread and check threading with tracing?
pub fn extract_shader_binaries<P: AsRef<Path>>(
    spch: &Spch,
    output_folder: P,
    ryujinx_shader_tools: Option<&str>,
    save_binaries: bool,
) {
    let output_folder = output_folder.as_ref();

    spch.slct_offsets
        .par_iter()
        .enumerate()
        .for_each(|(slct_index, slct_offset)| {
            // Not all programs have associated names.
            // Generate the name to avoid any ambiguity.
            let name = format!("slct{slct_index}");

            let slct = slct_offset.read_slct(&spch.slct_section).unwrap();
            let nvsds: Vec<_> = slct
                .programs
                .iter()
                .map(|p| p.read_nvsd().unwrap())
                .collect();

            let binaries = vertex_fragment_binaries(&nvsds, &spch.xv4_section, slct.xv4_offset);

            // TODO: Why do additional binaries sometimes fail to decompile?
            for (i, (vertex, fragment)) in binaries.into_iter().enumerate().take(1) {
                // Strip the xv4 headers to work with Ryujinx.ShaderTools.
                let vert_file = output_folder.join(&format!("{name}_VS{i}.bin"));
                std::fs::write(&vert_file, &vertex[48..]).unwrap();

                let frag_file = output_folder.join(&format!("{name}_FS{i}.bin"));
                std::fs::write(&frag_file, &fragment[48..]).unwrap();

                // Each NVSD has separate metadata since the shaders are different.
                let nvsd = slct.programs[i].read_nvsd().unwrap();

                // This doesn't need to be parsed, so just use debug output for now.
                let txt_file = output_folder.join(&format!("{name}.txt"));
                let text = format!("{:#?}", &nvsd);
                std::fs::write(txt_file, text).unwrap();

                // Decompile using Ryujinx.ShaderTools.exe.
                // There isn't Rust code for this, so just take an exe path.
                if let Some(shader_tools) = &ryujinx_shader_tools {
                    decompile_glsl_shaders(shader_tools, &frag_file, &vert_file, &nvsd);
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

fn decompile_glsl_shaders(shader_tools: &str, frag_file: &Path, vert_file: &Path, nvsd: &Nvsd) {
    // Spawn multiple process to increase utilization and boost performance.
    let frag_process = extract_shader(shader_tools, frag_file);
    let vert_process = extract_shader(shader_tools, vert_file);

    // TODO: Check exit code?
    let frag_glsl = String::from_utf8(frag_process.wait_with_output().unwrap().stdout).unwrap();
    let vert_glsl = String::from_utf8(vert_process.wait_with_output().unwrap().stdout).unwrap();

    // Perform annotation here since we need to know the file names.
    match annotate_vertex(&vert_glsl, nvsd) {
        Ok(glsl) => std::fs::write(vert_file.with_extension("glsl"), glsl).unwrap(),
        Err(e) => {
            error!("Error annotating {vert_file:?}: {e}");
            std::fs::write(vert_file.with_extension("glsl"), vert_glsl).unwrap();
        }
    }

    match annotate_fragment(&frag_glsl, nvsd) {
        Ok(glsl) => std::fs::write(frag_file.with_extension("glsl"), glsl).unwrap(),
        Err(e) => {
            error!("Error annotating {frag_file:?}: {e}");
            std::fs::write(frag_file.with_extension("glsl"), frag_glsl).unwrap();
        }
    }
}

fn extract_shader(shader_tools: &str, binary_file: &Path) -> std::process::Child {
    std::process::Command::new(shader_tools)
        .args([binary_file])
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap()
}

fn vertex_fragment_binaries<'a>(
    nvsds: &[Nvsd],
    xv4_section: &'a [u8],
    xv4_offset: u32,
) -> Vec<(&'a [u8], &'a [u8])> {
    // Each SLCT can have multiple NVSD.
    // Each NVSD has a vertex and fragment shader.
    nvsds
        .iter()
        .filter_map(|nvsd| {
            // TODO: Why is this sometimes none?
            let vertex = nvsd.vertex_binary(xv4_offset, xv4_section).or_else(|| {
                error!("No vertex binary for NVSD");
                None
            })?;
            let fragment = nvsd.fragment_binary(xv4_offset, xv4_section).or_else(|| {
                error!("No vertex binary for NVSD");
                None
            })?;
            Some((vertex, fragment))
        })
        .collect()
}
