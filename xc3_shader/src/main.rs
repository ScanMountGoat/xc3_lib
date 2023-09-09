use std::io::BufReader;
use std::path::Path;

use clap::{Parser, Subcommand};
use rayon::prelude::*;
use xc3_lib::msmd::Msmd;
use xc3_lib::msrd::Msrd;
use xc3_lib::mxmd::Mxmd;
use xc3_shader::extract::extract_shader_binaries;
use xc3_shader::gbuffer_database::create_shader_database;

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
    GBufferDatabase {
        /// The output folder from decompiling shaders.
        input_folder: String,
        /// The output JSON file.
        output_file: String,
    },
}

fn main() {
    let cli = Cli::parse();

    let start = std::time::Instant::now();
    // TODO: make annotation optional
    match cli.command {
        Commands::DecompileShaders {
            input_folder,
            output_folder,
            shader_tools,
        } => extract_and_decompile_shaders(&input_folder, &output_folder, shader_tools.as_deref()),
        Commands::GBufferDatabase {
            input_folder,
            output_file,
        } => {
            let files = create_shader_database(&input_folder);
            let json = serde_json::to_string(&files).unwrap();
            std::fs::write(output_file, json).unwrap()
        }
    }

    println!("Finished in {:?}", start.elapsed());
}

fn extract_and_decompile_shaders(input: &str, output: &str, shader_tools: Option<&str>) {
    globwalk::GlobWalkerBuilder::from_patterns(input, &["*.wimdo"])
        .build()
        .unwrap()
        .par_bridge()
        .for_each(|entry| {
            let path = entry.as_ref().unwrap().path();

            // Assume that file names are unique even across different folders.
            // This simplifies the output directory structure.
            // TODO: Preserve the original folder structure instead?
            let output_folder = decompiled_output_folder(output, path);
            std::fs::create_dir_all(&output_folder).unwrap();

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
                    let spch = msrd.extract_shader_data();
                    extract_shader_binaries(&spch, &output_folder, shader_tools, false);
                }
                Err(e) => println!("Error reading {path:?}: {e}"),
            }
        });

    let map_folder = Path::new(input).join("map");
    globwalk::GlobWalkerBuilder::from_patterns(map_folder, &["*.wismhd"])
        .build()
        .unwrap()
        .par_bridge()
        .for_each(|entry| {
            let path = entry.as_ref().unwrap().path();
            match Msmd::from_file(path) {
                Ok(msmd) => {
                    // Get the embedded shaders from the map files.
                    let output_folder = decompiled_output_folder(output, path);
                    std::fs::create_dir_all(&output_folder).unwrap();

                    extract_and_decompile_msmd_shaders(path, msmd, output_folder, shader_tools);
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

    for (i, model) in msmd.map_models.iter().enumerate() {
        let data = model.entry.extract(&mut wismda, true);

        let model_folder = output_folder.join("map").join(i.to_string());
        std::fs::create_dir_all(&model_folder).unwrap();

        extract_shader_binaries(&data.spch, &model_folder, shader_tools, false);
    }

    for (i, model) in msmd.prop_models.iter().enumerate() {
        let data = model.entry.extract(&mut wismda, true);

        let model_folder = output_folder.join("prop").join(i.to_string());
        std::fs::create_dir_all(&model_folder).unwrap();

        extract_shader_binaries(&data.spch, &model_folder, shader_tools, false);
    }

    for (i, model) in msmd.env_models.iter().enumerate() {
        let data = model.entry.extract(&mut wismda, true);

        let model_folder = output_folder.join("env").join(i.to_string());
        std::fs::create_dir_all(&model_folder).unwrap();

        extract_shader_binaries(&data.spch, &model_folder, shader_tools, false);
    }
}
