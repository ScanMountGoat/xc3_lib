use std::{
    error::Error,
    path::{Path, PathBuf},
};

use crate::annotation::{annotate_fragment, annotate_vertex};
use log::error;
use rayon::prelude::*;
use xc3_lib::{
    mths::Mths,
    spch::{vertex_fragment_binaries, Nvsd, ShaderBinary, Spch},
};

// TODO: profile performance using a single thread and check threading with tracing?
pub fn extract_shaders<P: AsRef<Path>>(
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

            let binaries = vertex_fragment_binaries(
                &nvsds,
                &spch.xv4_section,
                slct.xv4_offset,
                &spch.unk_section,
                slct.unk_item_offset,
            );

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
    binary: ShaderBinary,
    ryujinx_shader_tools: Option<&str>,
    nvsd: &Nvsd,
    save_binaries: bool,
    annotate: F,
) where
    F: Fn(&str, &Nvsd, Option<&[[f32; 4]; 16]>) -> Result<String, Box<dyn Error>>,
{
    // Strip the xv4 headers to work with Ryujinx.ShaderTools.
    std::fs::write(&binary_path, &binary.program_binary[48..]).unwrap();

    // Decompile using Ryujinx.ShaderTools.exe.
    // There isn't Rust code for this, so just take an exe path.
    if let Some(shader_tools) = &ryujinx_shader_tools {
        decompile_glsl_shader(
            shader_tools,
            &binary_path,
            &glsl_path,
            nvsd,
            binary.constant_buffer.as_ref(),
            annotate,
        );
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
    constants: Option<&[[f32; 4]; 16]>,
    annotate: F,
) where
    F: Fn(&str, &Nvsd, Option<&[[f32; 4]; 16]>) -> Result<String, Box<dyn Error>>,
{
    let process = extract_shader(shader_tools, binary_path);

    // TODO: Check exit code?
    let glsl = String::from_utf8(process.wait_with_output().unwrap().stdout).unwrap();

    match annotate(&glsl, nvsd, constants) {
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

pub fn extract_legacy_shaders<P: AsRef<Path>>(
    mths: &Mths,
    mths_bytes: &[u8],
    output_folder: P,
    gfd_tool: &str,
    index: usize,
) {
    let output_folder = output_folder.as_ref();

    std::fs::write(output_folder.join(format!("{index}.cashd")), mths_bytes).unwrap();

    if let Ok(vert) = mths.vertex_shader() {
        let binary_path = output_folder.join(format!("{index}.vert.bin"));
        dissassemble_shader(&binary_path, &vert.inner.program_binary, gfd_tool);
    }

    if let Ok(frag) = mths.fragment_shader() {
        let binary_path = output_folder.join(format!("{index}.frag.bin"));
        dissassemble_shader(&binary_path, &frag.program_binary, gfd_tool);
    }
}

fn dissassemble_shader(binary_path: &Path, binary: &[u8], gfd_tool: &str) {
    std::fs::write(binary_path, binary).unwrap();

    let output = std::process::Command::new(gfd_tool)
        .arg("disassemble")
        .arg(binary_path)
        .output()
        .unwrap()
        .stdout;
    let text = String::from_utf8(output).unwrap();

    std::fs::write(binary_path.with_extension("txt"), text).unwrap();

    // TODO: add an option to preserve binaries?
    std::fs::remove_file(binary_path).unwrap();
}
