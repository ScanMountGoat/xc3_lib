use std::io::BufReader;
use std::path::Path;

use clap::{Parser, Subcommand};
use extract::extract_shader_binaries;
use shader_database::create_shader_database;
use xc3_lib::msmd::Msmd;
use xc3_lib::msrd::Msrd;
use xc3_lib::mxmd::Mxmd;
use xc3_lib::spch::Spch;

use crate::dependencies::glsl_dependencies;

mod annotation;
mod dependencies;
mod extract;
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
    /// Extract and decompile shaders into a folder for each .wismt file.
    /// JSON metadata for each program will also be saved in the output folder.
    DecompileShaders {
        /// The dump root folder for Xenoblade 2 or Xenoblade 3.
        input_folder: String,
        /// The output folder for the decompiled shaders.
        output_folder: String,
        /// The path to the Ryujinx.ShaderTools executable
        shader_tools: Option<String>,
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
    /// Find all lines of GLSL code influencing the final assignment of a variable.
    GlslDependencies {
        /// The input GLSL file.
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
        Commands::ShaderDatabase {
            input_folder,
            output_file,
            pretty,
        } => {
            let database = create_shader_database(&input_folder);
            database.save(output_file, pretty).unwrap();
        }
        Commands::GlslDependencies { input, output, var } => {
            let source = std::fs::read_to_string(input).unwrap();
            let source_out = glsl_dependencies(&source, &var);
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
            let output_folder = decompiled_output_folder(output, path);
            std::fs::create_dir_all(&output_folder).unwrap();
            println!("{output_folder:?}");

            // Shaders can be embedded in the wimdo or wismt file.
            match Mxmd::from_file(path) {
                Ok(mxmd) => {
                    if let Some(spch) = mxmd.spch {
                        extract_shader_binaries(&spch, &output_folder, shader_tools, false);
                    }
                }
                Err(e) => println!("Error reading {path:?}: {e}"),
            }

            match Msrd::from_file(path.with_extension("wismt")) {
                Ok(msrd) => {
                    let (_, spch, _) = msrd.extract_files(None).unwrap();
                    extract_shader_binaries(&spch, &output_folder, shader_tools, false);
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
                    let output_folder = decompiled_output_folder(output, path);
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
                    let output_folder = decompiled_output_folder(output, path);
                    std::fs::create_dir_all(&output_folder).unwrap();
                    println!("{output_folder:?}");

                    extract_shader_binaries(&spch, &output_folder, shader_tools, false);
                }
                Err(e) => println!("Error reading {path:?}: {e}"),
            }
        });
}

fn decompiled_output_folder(output_folder: &str, path: &Path) -> std::path::PathBuf {
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

        extract_shader_binaries(&data.spch, &model_folder, shader_tools, false);
    }

    for (i, model) in msmd.prop_models.iter().enumerate() {
        let data = model.entry.extract(&mut wismda, compressed).unwrap();

        let model_folder = output_folder.join("prop").join(i.to_string());
        std::fs::create_dir_all(&model_folder).unwrap();

        extract_shader_binaries(&data.spch, &model_folder, shader_tools, false);
    }

    for (i, model) in msmd.env_models.iter().enumerate() {
        let data = model.entry.extract(&mut wismda, compressed).unwrap();

        let model_folder = output_folder.join("env").join(i.to_string());
        std::fs::create_dir_all(&model_folder).unwrap();

        extract_shader_binaries(&data.spch, &model_folder, shader_tools, false);
    }

    // TODO: Foliage shaders?
}
