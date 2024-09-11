use std::io::BufReader;
use std::path::Path;

use clap::{Parser, Subcommand};
use dependencies::latte_dependencies;
use extract::{extract_legacy_shaders, extract_shaders};
use rayon::prelude::*;
use shader_database::{create_shader_database, create_shader_database_legacy};
use xc3_lib::msmd::Msmd;
use xc3_lib::msrd::Msrd;
use xc3_lib::mths::Mths;
use xc3_lib::mxmd::legacy::MxmdLegacy;
use xc3_lib::mxmd::Mxmd;
use xc3_lib::spch::Spch;

use crate::dependencies::glsl_dependencies;

mod annotation;
mod dependencies;
mod extract;
mod graph;
mod shader_database;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Extract and decompile shaders into a folder for each .wimdo or .wismhd file.
    /// JSON metadata for each program will also be saved in the output folder.
    DecompileShaders {
        /// The root folder for Xenoblade 1 DE, Xenoblade 2, or Xenoblade 3.
        input_folder: String,
        /// The output folder for the decompiled shaders.
        output_folder: String,
        /// The path to the Ryujinx.ShaderTools executable
        shader_tools: Option<String>,
    },
    /// Extract and disassemble shaders into a folder for each .camdo file.
    DisassembleLegacyShaders {
        /// The root folder for Xenoblade X.
        input_folder: String,
        /// The output folder for the disassembled shaders.
        output_folder: String,
        /// The path to the gfd-tool executable
        gfd_tool: String,
    },
    /// Create a JSON file containing textures used for fragment output attributes.
    ShaderDatabase {
        /// The output folder from decompiling shaders.
        input_folder: String,
        /// The output JSON file.
        output_file: String,
        /// Pretty print the JSON file
        #[arg(long)]
        pretty: bool,
    },
    /// Create a JSON file containing textures used for fragment output attributes for Xenoblade X.
    ShaderDatabaseLegacy {
        /// The output folder from decompiling shaders.
        input_folder: String,
        /// The output JSON file.
        output_file: String,
        /// Pretty print the JSON file
        #[arg(long)]
        pretty: bool,
    },
    /// Find all lines of GLSL code influencing the final assignment of a variable.
    GlslDependencies {
        /// The input GLSL file.
        input: String,
        /// The output GLSL file.
        output: String,
        /// The name of the variable to analyze.
        var: String,
    },
    /// Find all lines of GLSL code influencing the final assignment of a variable.
    LatteDependencies {
        /// The input Latte ASM file.
        input: String,
        /// The output GLSL file.
        output: String,
        /// The name of the variable to analyze.
        var: String,
    },
}

fn main() {
    simple_logger::SimpleLogger::new()
        .with_level(log::LevelFilter::Warn)
        .init()
        .unwrap();

    let cli = Cli::parse();

    let start = std::time::Instant::now();
    // TODO: make annotation optional
    match cli.command {
        Commands::DecompileShaders {
            input_folder,
            output_folder,
            shader_tools,
        } => extract_and_decompile_shaders(&input_folder, &output_folder, shader_tools.as_deref()),
        Commands::DisassembleLegacyShaders {
            input_folder,
            output_folder,
            gfd_tool,
        } => extract_and_disassemble_shaders(&input_folder, &output_folder, &gfd_tool),
        Commands::ShaderDatabase {
            input_folder,
            output_file,
            pretty,
        } => {
            let database = create_shader_database(&input_folder);
            database.save(output_file, pretty).unwrap();
        }
        Commands::ShaderDatabaseLegacy {
            input_folder,
            output_file,
            pretty,
        } => {
            let database = create_shader_database_legacy(&input_folder);
            database.save(output_file, pretty).unwrap();
        }
        Commands::GlslDependencies { input, output, var } => {
            let source = std::fs::read_to_string(input).unwrap();
            let (var, channels) = var.split_once('.').unwrap();
            let source_out = glsl_dependencies(&source, var, channels.chars().next());
            std::fs::write(output, source_out).unwrap();
        }
        Commands::LatteDependencies { input, output, var } => {
            let source = std::fs::read_to_string(input).unwrap();
            let (var, channels) = var.split_once('.').unwrap();
            let source_out = latte_dependencies(&source, var, channels.chars().next());
            std::fs::write(output, source_out).unwrap();
        }
    }

    println!("Finished in {:?}", start.elapsed());
}

fn extract_and_decompile_shaders(input: &str, output: &str, shader_tools: Option<&str>) {
    globwalk::GlobWalkerBuilder::from_patterns(input, &["*.wimdo"])
        .build()
        .unwrap()
        .for_each(|entry| {
            let path = entry.as_ref().unwrap().path();

            // Assume that file names are unique even across different folders.
            // This simplifies the output directory structure.
            // TODO: Preserve the original folder structure instead?
            let output_folder = shader_output_folder(output, path);
            std::fs::create_dir_all(&output_folder).unwrap();
            println!("{output_folder:?}");

            // Shaders can be embedded in the wimdo or wismt file.
            match Mxmd::from_file(path) {
                Ok(mxmd) => {
                    if let Some(spch) = mxmd.spch {
                        extract_shaders(&spch, &output_folder, shader_tools, false);
                    }
                }
                Err(e) => println!("Error reading {path:?}: {e}"),
            }

            match Msrd::from_file(path.with_extension("wismt")) {
                Ok(msrd) => {
                    let (_, spch, _) = msrd.extract_files(None).unwrap();
                    extract_shaders(&spch, &output_folder, shader_tools, false);
                }
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

fn shader_output_folder(output_folder: &str, path: &Path) -> std::path::PathBuf {
    // Use the name as a folder like "ch01011010.wismt" -> "ch01011010/".
    let name = path.with_extension("");
    let name = name.file_name().unwrap();
    Path::new(output_folder).join(name)
}

fn extract_and_decompile_msmd_shaders(
    path: &Path,
    msmd: Msmd,
    output_folder: std::path::PathBuf,
    shader_tools: Option<&str>,
) {
    let mut wismda = BufReader::new(std::fs::File::open(path.with_extension("wismda")).unwrap());
    let compressed = msmd.wismda_info.compressed_length != msmd.wismda_info.decompressed_length;

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

pub fn extract_and_disassemble_shaders(input: &str, output: &str, gfd_tool: &str) {
    globwalk::GlobWalkerBuilder::from_patterns(input, &["*.camdo"])
        .build()
        .unwrap()
        .par_bridge()
        .for_each(|entry| {
            let path = entry.as_ref().unwrap().path();

            // Assume that file names are unique even across different folders.
            // This simplifies the output directory structure.
            // TODO: Preserve the original folder structure instead?
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
            // TODO: Preserve the original folder structure instead?
            let output_folder = shader_output_folder(output, path);
            std::fs::create_dir_all(&output_folder).unwrap();

            let bytes = std::fs::read(path).unwrap();
            match Mths::from_bytes(&bytes) {
                Ok(mths) => extract_legacy_shaders(&mths, &bytes, &output_folder, gfd_tool, 0),
                Err(e) => println!("Error reading {path:?}: {e}"),
            }
        });
}
