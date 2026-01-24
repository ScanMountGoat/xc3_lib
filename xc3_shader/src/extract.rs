use std::{
    error::Error,
    io::BufReader,
    path::{Path, PathBuf},
};

use crate::{
    annotation::{annotate_fragment, annotate_vertex},
    extract::legacy::{annotate_legacy_shaders, extract_legacy_shaders},
};
use log::error;
use rayon::prelude::*;
use xc3_lib::{
    msmd::Msmd,
    msrd::Msrd,
    mths::Mths,
    mxmd::{Mxmd, legacy::MxmdLegacy},
    spch::{Nvsd, ShaderBinary, Spch},
};

pub mod legacy;

pub fn extract_and_decompile_shaders(input: &str, output: &str, shader_tools: Option<&str>) {
    globwalk::GlobWalkerBuilder::from_patterns(input, &["*.wimdo"])
        .build()
        .unwrap()
        .for_each(|entry| {
            let path = entry.as_ref().unwrap().path();

            // Assume that file names are unique even across different folders.
            // This simplifies the output directory structure.
            let output_folder = shader_output_folder(output, path);
            std::fs::create_dir_all(&output_folder).unwrap();
            println!("{output_folder:?}");

            // Shaders can be embedded in the wimdo or wismt file.
            match Mxmd::from_file(path) {
                Ok(mxmd) => match mxmd.inner {
                    xc3_lib::mxmd::MxmdInner::V40(mxmd) => {
                        // TODO: Which spch should be used?
                        if let Some(spch) = mxmd
                            .shaders
                            .as_ref()
                            .and_then(|s| s.items.first().map(|s| &s.spch))
                        {
                            extract_shaders(spch, &output_folder, shader_tools, false);
                        }

                        if mxmd.streaming.is_some() {
                            match Msrd::from_file(path.with_extension("wismt")) {
                                Ok(msrd) => {
                                    if let Ok(files) = msrd.extract_files_legacy(None)
                                        && let Some(spch) = files.shader.spch()
                                    {
                                        extract_shaders(spch, &output_folder, shader_tools, false);
                                    }
                                }
                                Err(e) => println!("Error reading {path:?}: {e}"),
                            }
                        }
                    }
                    xc3_lib::mxmd::MxmdInner::V111(mxmd) => {
                        if let Some(spch) = mxmd.spch {
                            extract_shaders(&spch, &output_folder, shader_tools, false);
                        }

                        if mxmd.streaming.is_some() {
                            match Msrd::from_file(path.with_extension("wismt")) {
                                Ok(msrd) => {
                                    let files = msrd.extract_files(None).unwrap();
                                    extract_shaders(
                                        &files.shader,
                                        &output_folder,
                                        shader_tools,
                                        false,
                                    );
                                }
                                Err(e) => println!("Error reading {path:?}: {e}"),
                            }
                        }
                    }
                    xc3_lib::mxmd::MxmdInner::V112(mxmd) => {
                        if let Some(spch) = mxmd.spch {
                            extract_shaders(&spch, &output_folder, shader_tools, false);
                        }

                        if mxmd.streaming.is_some() {
                            match Msrd::from_file(path.with_extension("wismt")) {
                                Ok(msrd) => {
                                    let files = msrd.extract_files(None).unwrap();
                                    extract_shaders(
                                        &files.shader,
                                        &output_folder,
                                        shader_tools,
                                        false,
                                    );
                                }
                                Err(e) => println!("Error reading {path:?}: {e}"),
                            }
                        }
                    }
                },
                Err(e) => println!("Error reading {path:?}: {e}"),
            }
        });

    globwalk::GlobWalkerBuilder::from_patterns(input, &["*.wismhd"])
        .build()
        .unwrap()
        .for_each(|entry| {
            let path = entry.as_ref().unwrap().path();
            match Msmd::from_file(path) {
                Ok(msmd) => {
                    // Get the embedded shaders from the map files.
                    let output_folder = shader_output_folder(output, path);
                    std::fs::create_dir_all(&output_folder).unwrap();
                    println!("{output_folder:?}");

                    extract_and_decompile_msmd_shaders(path, msmd, output_folder, shader_tools);
                }
                Err(e) => println!("Error reading {path:?}: {e}"),
            }
        });

    globwalk::GlobWalkerBuilder::from_patterns(input, &["*.wishp"])
        .build()
        .unwrap()
        .for_each(|entry| {
            let path = entry.as_ref().unwrap().path();
            match Spch::from_file(path) {
                Ok(spch) => {
                    // Get the embedded shaders from the map files.
                    let output_folder = shader_output_folder(output, path);
                    std::fs::create_dir_all(&output_folder).unwrap();
                    println!("{output_folder:?}");

                    extract_shaders(&spch, &output_folder, shader_tools, false);
                }
                Err(e) => println!("Error reading {path:?}: {e}"),
            }
        });
}

fn extract_and_decompile_msmd_shaders(
    path: &Path,
    msmd: Msmd,
    output_folder: std::path::PathBuf,
    shader_tools: Option<&str>,
) {
    match &msmd.inner {
        xc3_lib::msmd::MsmdInner::V11(msmd_v111) => todo!(),
        xc3_lib::msmd::MsmdInner::V112(msmd) => {
            let mut wismda =
                BufReader::new(std::fs::File::open(path.with_extension("wismda")).unwrap());
            let compressed =
                msmd.wismda_info.compressed_length != msmd.wismda_info.decompressed_length;

            for (i, model) in msmd.map_models.iter().enumerate() {
                let data = model.entry.extract(&mut wismda, compressed).unwrap();

                let model_folder = output_folder.join("map").join(i.to_string());
                std::fs::create_dir_all(&model_folder).unwrap();

                extract_shaders(&data.spch, &model_folder, shader_tools, false);
            }

            for (i, model) in msmd.prop_models.iter().enumerate() {
                let data = model.entry.extract(&mut wismda, compressed).unwrap();

                let model_folder = output_folder.join("prop").join(i.to_string());
                std::fs::create_dir_all(&model_folder).unwrap();

                extract_shaders(&data.spch, &model_folder, shader_tools, false);
            }

            for (i, model) in msmd.env_models.iter().enumerate() {
                let data = model.entry.extract(&mut wismda, compressed).unwrap();

                let model_folder = output_folder.join("env").join(i.to_string());
                std::fs::create_dir_all(&model_folder).unwrap();

                extract_shaders(&data.spch, &model_folder, shader_tools, false);
            }

            // TODO: Foliage shaders?
        }
    }
}

pub fn extract_all_legacy_shaders(input: &str, output: &str, gfd_tool: &str) {
    globwalk::GlobWalkerBuilder::from_patterns(input, &["*.camdo"])
        .build()
        .unwrap()
        .par_bridge()
        .for_each(|entry| {
            let path = entry.as_ref().unwrap().path();

            // Assume that file names are unique even across different folders.
            // This simplifies the output directory structure.
            let output_folder = shader_output_folder(output, path);
            std::fs::create_dir_all(&output_folder).unwrap();

            // Shaders are embedded in the camdo file.
            match MxmdLegacy::from_file(path) {
                Ok(mxmd) => {
                    mxmd.shaders
                        .shaders
                        .iter()
                        .enumerate()
                        .for_each(|(i, shader)| match Mths::from_bytes(&shader.mths_data) {
                            Ok(mths) => extract_legacy_shaders(
                                &mths,
                                &shader.mths_data,
                                &output_folder,
                                gfd_tool,
                                i,
                            ),
                            Err(e) => println!("Error extracting Mths from {path:?}: {e}"),
                        });
                }
                Err(e) => println!("Error reading {path:?}: {e}"),
            }
        });

    globwalk::GlobWalkerBuilder::from_patterns(input, &["*.cashd"])
        .build()
        .unwrap()
        .par_bridge()
        .for_each(|entry| {
            let path = entry.as_ref().unwrap().path();

            // Assume that file names are unique even across different folders.
            // This simplifies the output directory structure.
            let output_folder = shader_output_folder(output, path);
            std::fs::create_dir_all(&output_folder).unwrap();

            let bytes = std::fs::read(path).unwrap();
            match Mths::from_bytes(&bytes) {
                Ok(mths) => extract_legacy_shaders(&mths, &bytes, &output_folder, gfd_tool, 0),
                Err(e) => println!("Error reading {path:?}: {e}"),
            }
        });
}

pub fn annotate_all_legacy_shaders(input: &str, output: &str) {
    globwalk::GlobWalkerBuilder::from_patterns(input, &["*.camdo"])
        .build()
        .unwrap()
        .par_bridge()
        .for_each(|entry| {
            let path = entry.as_ref().unwrap().path();

            // Assume that file names are unique even across different folders.
            // This simplifies the output directory structure.
            let output_folder = shader_output_folder(output, path);
            std::fs::create_dir_all(&output_folder).unwrap();

            // Shaders are embedded in the camdo file.
            match MxmdLegacy::from_file(path) {
                Ok(mxmd) => {
                    mxmd.shaders
                        .shaders
                        .iter()
                        .zip(&mxmd.materials.techniques)
                        .enumerate()
                        .for_each(|(i, (shader, technique))| {
                            match Mths::from_bytes(&shader.mths_data) {
                                Ok(mths) => annotate_legacy_shaders(
                                    &mths,
                                    &shader.mths_data,
                                    &output_folder,
                                    i,
                                    Some(technique),
                                ),
                                Err(e) => println!("Error extracting Mths from {path:?}: {e}"),
                            }
                        });
                }
                Err(e) => println!("Error reading {path:?}: {e}"),
            }
        });

    globwalk::GlobWalkerBuilder::from_patterns(input, &["*.cashd"])
        .build()
        .unwrap()
        .par_bridge()
        .for_each(|entry| {
            let path = entry.as_ref().unwrap().path();

            // Assume that file names are unique even across different folders.
            // This simplifies the output directory structure.
            let output_folder = shader_output_folder(output, path);
            std::fs::create_dir_all(&output_folder).unwrap();

            let bytes = std::fs::read(path).unwrap();
            match Mths::from_bytes(&bytes) {
                Ok(mths) => annotate_legacy_shaders(&mths, &bytes, &output_folder, 0, None),
                Err(e) => println!("Error reading {path:?}: {e}"),
            }
        });
}

fn extract_shaders<P: AsRef<Path>>(
    spch: &Spch,
    output_folder: P,
    ryujinx_shader_tools: Option<&str>,
    save_binaries: bool,
) {
    let output_folder = output_folder.as_ref();

    // Save the binary for creating the database later.
    spch.save(output_folder.join("shaders.wishp")).unwrap();

    spch.slct_offsets
        .par_iter()
        .enumerate()
        .for_each(|(slct_index, slct_offset)| {
            let slct = slct_offset.read_slct(&spch.slct_section).unwrap();

            let binaries = spch.nvsd_vertex_fragment_binaries(&slct);

            for (nvsd_index, (nvsd, vertex, fragment)) in binaries.into_iter().enumerate() {
                // Each NVSD has separate metadata since the shaders are different.
                let name = nvsd_glsl_name(spch, slct_index, nvsd_index);

                // Metadata doesn't need to be parsed from strings later.
                // Just use the debug output for now.
                let txt_file = output_folder.join(format!("{name}.txt"));
                let text = format!("{:#?}", &nvsd);
                std::fs::write(txt_file, text).unwrap();

                // TODO: Why are these binaries sometimes empty?
                if let Some(vertex) = vertex {
                    process_shader(
                        output_folder.join(format!("{name}.vert.bin")),
                        output_folder.join(format!("{name}.vert")),
                        vertex,
                        ryujinx_shader_tools,
                        &nvsd,
                        save_binaries,
                        annotate_vertex,
                    );
                }

                if let Some(fragment) = fragment {
                    process_shader(
                        output_folder.join(format!("{name}.frag.bin")),
                        output_folder.join(format!("{name}.frag")),
                        fragment,
                        ryujinx_shader_tools,
                        &nvsd,
                        save_binaries,
                        annotate_fragment,
                    );
                }
            }
        });
}

pub fn nvsd_glsl_name(spch: &Spch, slct_index: usize, nvsd_index: usize) -> String {
    // Not all programs have associated names.
    // Generate the name to avoid any ambiguity.
    match spch
        .string_section
        .as_ref()
        .and_then(|s| s.program_names.get(slct_index).map(|n| &n.name))
    {
        Some(program_name) => {
            format!("slct{slct_index}_nvsd{nvsd_index}_{program_name}")
        }
        None => format!("slct{slct_index}_nvsd{nvsd_index}"),
    }
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

fn shader_output_folder(output_folder: &str, path: &Path) -> std::path::PathBuf {
    // Use the name as a folder like "ch01011010.wismt" -> "output/ch01011010/".
    let name = path.file_stem().unwrap();
    Path::new(output_folder).join(name)
}
