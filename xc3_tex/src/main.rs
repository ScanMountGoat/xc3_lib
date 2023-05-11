use std::{
    io::{BufReader, BufWriter},
    path::PathBuf,
};

use clap::Parser;
use xc3_lib::{
    dds::{create_dds, create_mibl},
    mibl::Mibl,
};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    input: String,
    output: Option<String>,
}

fn main() {
    let cli = Cli::parse();

    let input = PathBuf::from(&cli.input);

    // Default to DDS since it supports more formats.
    let output = cli
        .output
        .map(PathBuf::from)
        .unwrap_or(input.with_extension("dds"));

    // Handle all conversions by using DDS as an intermediate format.
    let dds = match input.extension().unwrap().to_str().unwrap() {
        "witex" | "witx" => {
            let mibl = Mibl::from_file(input).unwrap();
            create_dds(&mibl).unwrap()
        }
        // TODO: image and single tex wismt
        "dds" => {
            let mut reader = BufReader::new(std::fs::File::open(input).unwrap());
            ddsfile::Dds::read(&mut reader).unwrap()
        }
        _ => todo!(),
    };

    match output.extension().unwrap().to_str().unwrap() {
        "dds" => {
            let mut writer = BufWriter::new(std::fs::File::create(output).unwrap());
            dds.write(&mut writer).unwrap();
        }
        "witex" | "witx" => {
            let mibl = create_mibl(&dds).unwrap();
            mibl.write_to_file(output).unwrap();
        }
        // TODO: single tex wismt
        _ => {
            // Assume other formats are image formats for now.
            // TODO: properly flatten 3D images in image_dds.
            let image = image_dds::image_from_dds(&dds, 0).unwrap();
            image.save(output).unwrap();
        }
    }
}
