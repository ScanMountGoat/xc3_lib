use std::{
    error::Error,
    path::{Path, PathBuf},
};

use crate::annotation::{annotate_fragment, annotate_vertex};
use log::error;
use rayon::prelude::*;
use xc3_lib::spch::{vertex_fragment_binaries, Nvsd, Spch};

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
            let slct = slct_offset.read_slct(&spch.slct_section).unwrap();
            let nvsds: Vec<_> = slct
                .programs
                .iter()
                .map(|p| p.read_nvsd().unwrap())
                .collect();

            let binaries = vertex_fragment_binaries(&nvsds, &spch.xv4_section, slct.xv4_offset);

            for (nvsd_index, (vertex, fragment)) in binaries.into_iter().enumerate() {
                // Each NVSD has separate metadata since the shaders are different.
                let nvsd = &nvsds[nvsd_index];

                // Not all programs have associated names.
                // Generate the name to avoid any ambiguity.
                let name = match spch
                    .string_section
                    .as_ref()
                    .and_then(|s| s.program_names.get(slct_index).map(|n| &n.name))
                {
                    Some(program_name) => {
                        format!("slct{slct_index}_nvsd{nvsd_index}_{program_name}")
                    }
                    None => format!("slct{slct_index}_nvsd{nvsd_index}"),
                };

                // Metadata doesn't need to be parsed from strings later.
                // Just use the debug output for now.
                let txt_file = output_folder.join(&format!("{name}.txt"));
                let text = format!("{:#?}", &nvsd);
                std::fs::write(txt_file, text).unwrap();

                // TODO: Why are these binaries sometimes empty?
                if let Some(vertex) = vertex {
                    process_shader(
                        output_folder.join(&format!("{name}.vert.bin")),
                        output_folder.join(&format!("{name}.vert")),
                        vertex,
                        ryujinx_shader_tools,
                        nvsd,
                        save_binaries,
                        annotate_vertex,
                    );
                }

                if let Some(fragment) = fragment {
                    process_shader(
                        output_folder.join(&format!("{name}.frag.bin")),
                        output_folder.join(&format!("{name}.frag")),
                        fragment,
                        ryujinx_shader_tools,
                        nvsd,
                        save_binaries,
                        annotate_fragment,
                    );
                }
            }
        });
}

fn process_shader<F>(
    binary_path: PathBuf,
    glsl_path: PathBuf,
    binary: &[u8],
    ryujinx_shader_tools: Option<&str>,
    nvsd: &Nvsd,
    save_binaries: bool,
    annotate: F,
) where
    F: Fn(&str, &Nvsd) -> Result<String, Box<dyn Error>>,
{
    // Strip the xv4 headers to work with Ryujinx.ShaderTools.
    std::fs::write(&binary_path, &binary[48..]).unwrap();

    // Decompile using Ryujinx.ShaderTools.exe.
    // There isn't Rust code for this, so just take an exe path.
    if let Some(shader_tools) = &ryujinx_shader_tools {
        decompile_glsl_shader(shader_tools, &binary_path, &glsl_path, nvsd, annotate);
    }

    // We needed to temporarily create binaries for ShaderTools to decompile.
    // Delete them if they are no longer needed.
    if !save_binaries {
        std::fs::remove_file(binary_path).unwrap();
    }
}

fn decompile_glsl_shader<F>(
    shader_tools: &str,
    binary_path: &Path,
    glsl_path: &Path,
    nvsd: &Nvsd,
    annotate: F,
) where
    F: Fn(&str, &Nvsd) -> Result<String, Box<dyn Error>>,
{
    let process = extract_shader(shader_tools, binary_path);

    // TODO: Check exit code?
    let glsl = String::from_utf8(process.wait_with_output().unwrap().stdout).unwrap();

    match annotate(&glsl, nvsd) {
        Ok(glsl) => std::fs::write(glsl_path, glsl).unwrap(),
        Err(e) => {
            error!("Error annotating {binary_path:?}: {e}");
            std::fs::write(glsl_path, glsl).unwrap();
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
