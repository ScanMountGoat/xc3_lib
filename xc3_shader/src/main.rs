use std::{io::Cursor, path::Path};

use clap::{Parser, Subcommand};
use rayon::prelude::*;
use xc3_lib::{
    msrd::{EntryType, Msrd},
    spch::Spch,
};
use xc3_shader::extract::extract_shader_binaries;
use xc3_shader::gbuffer_database::create_shader_database;

// TODO: subcommands for decompilation, annotation, etc
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
        /// The folder containing the .wismt files.
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
        } => extract_and_decompile_wismt_shaders(
            &input_folder,
            &output_folder,
            shader_tools.as_ref().map(|s| s.as_str()),
        ),
        Commands::GBufferDatabase {
            input_folder,
            output_file,
        } => {
            let files = create_shader_database(&input_folder);
            let json = serde_json::to_string_pretty(&files).unwrap();
            std::fs::write(output_file, json).unwrap()
        }
    }

    println!("Finished in {:?}", start.elapsed());
}

fn extract_and_decompile_wismt_shaders(input: &str, output: &str, shader_tools: Option<&str>) {
    globwalk::GlobWalkerBuilder::from_patterns(input, &["*.wismt"])
        .build()
        .unwrap()
        .par_bridge()
        .for_each(|entry| {
            let path = entry.as_ref().unwrap().path();
            match Msrd::from_file(path) {
                Ok(msrd) => {
                    // Get the embedded shaders from the wismt file.
                    let output_folder = decompiled_output_folder(output, path);
                    std::fs::create_dir_all(&output_folder).unwrap();

                    extract_and_decompile_shaders(msrd, shader_tools, output_folder);
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

fn extract_and_decompile_shaders<P: AsRef<Path>>(
    msrd: Msrd,
    shader_tools: Option<&str>,
    output_folder: P,
) {
    let decompressed_streams: Vec<_> = msrd
        .streams
        .iter()
        .map(|stream| stream.xbc1.decompress().unwrap())
        .collect();

    for item in msrd.stream_entries {
        if item.item_type == EntryType::ShaderBundle {
            let stream = &decompressed_streams[item.stream_index as usize];
            let data = &stream[item.offset as usize..item.offset as usize + item.size as usize];

            let spch = Spch::read(&mut Cursor::new(data)).unwrap();

            // TODO: Will shaders always have names like "shd0004"?
            // TODO: Include the program index in the name to avoid ambiguities?
            extract_shader_binaries(&spch, output_folder.as_ref(), shader_tools, false);
        }
    }
}
