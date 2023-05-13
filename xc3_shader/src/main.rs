use std::{io::Cursor, path::Path};

use clap::Parser;
use extract::extract_shader_binaries;
use rayon::prelude::*;
use xc3_lib::{msrd::Msrd, spch::Spch};

mod extract;

// TODO: subcommands for decompilation, annotation, etc
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    input_folder: String,
    output_folder: String,
    /// The path to the Ryujinx.ShaderTools executable
    shader_tools: String,
}

fn main() {
    // TODO: port dependency analysis from smush_materials
    // 1. extract all shaders from wismt files in a folder
    // 2. find fragment shaders using the wimdo material for each mesh
    // 3. find the dependencies for each G-Buffer texture
    // 4. store these dependencies into a JSON "database"
    // 5. optimize size/performance if needed

    let cli = Cli::parse();
    extract_and_decompile_wismt_shaders(&cli.input_folder, &cli.output_folder, &cli.shader_tools);

    // TODO: add an annotation option
}

fn extract_and_decompile_wismt_shaders(input: &str, output: &str, shader_tools: &str) {
    globwalk::GlobWalkerBuilder::from_patterns(input, &["*.wismt"])
        .build()
        .unwrap()
        .take(1) // TODO: only do a few for now for performance reasons
        .par_bridge()
        .for_each(|entry| {
            let path = entry.as_ref().unwrap().path();
            // TODO: How to validate this file?
            match Msrd::from_file(path) {
                Ok(msrd) => {
                    // Get the shaders from the corresponding file.
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

fn extract_and_decompile_shaders<P: AsRef<Path>>(msrd: Msrd, shader_tools: &str, output_folder: P) {
    let toc_streams: Vec<_> = msrd
        .tocs
        .iter()
        .map(|toc| toc.xbc1.decompress().unwrap())
        .collect();

    for item in msrd.data_items {
        match item.item_type {
            xc3_lib::msrd::DataItemType::ShaderBundle => {
                let stream = &toc_streams[item.toc_index as usize];
                let data = &stream[item.offset as usize..item.offset as usize + item.size as usize];

                let spch = Spch::read(&mut Cursor::new(data)).unwrap();

                // TODO: Will shaders always have names like "shd0004"?
                // TODO: Include the program index in the name to avoid ambiguities?
                extract_shader_binaries(
                    &spch,
                    data,
                    output_folder.as_ref(),
                    Some(shader_tools.to_string()),
                    false,
                );

                // TODO: Annotate the source for each generated shader.
                // This relies on the naming conventions of the step above?
                // TODO: add a separate module and tests for this in annotation.rs
            }
            _ => (),
        }
    }
}
