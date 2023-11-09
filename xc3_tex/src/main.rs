use std::{
    io::BufReader,
    path::{Path, PathBuf},
    str::FromStr,
};

use clap::Parser;
use image_dds::{ddsfile::Dds, image};
use xc3_lib::{dds::save_dds, mibl::Mibl, xbc1::Xbc1};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    input: String,
    output: Option<String>,
    // TODO: Document available options.
    format: Option<String>,
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
            mibl.to_dds().unwrap()
        }
        // TODO: image and single tex wismt
        "dds" => {
            let mut reader = BufReader::new(std::fs::File::open(input).unwrap());
            Dds::read(&mut reader).unwrap()
        }
        "wismt" => {
            let mibl = read_wismt_single_tex(input);
            mibl.to_dds().unwrap()
        }
        _ => {
            // Assume other formats are image formats for now.
            // TODO: Support floating point images.
            // TODO: Specify quality and mipmaps?
            let image = image::open(input).unwrap().to_rgba8();
            let format = image_dds::ImageFormat::from_str(&cli.format.unwrap()).unwrap();
            image_dds::dds_from_image(
                &image,
                format,
                image_dds::Quality::Normal,
                image_dds::Mipmaps::GeneratedAutomatic,
            )
            .unwrap()
        }
    };

    match output.extension().unwrap().to_str().unwrap() {
        "dds" => {
            save_dds(output, &dds);
        }
        "witex" | "witx" => {
            let mibl = Mibl::from_dds(&dds).unwrap();
            mibl.write_to_file(output).unwrap();
        }
        // TODO: single tex wismt
        // TODO: Also create base level?
        "wismt" => {
            let mibl = Mibl::from_dds(&dds).unwrap();
            let xbc1 = create_wismt_single_tex(&mibl);
            xbc1.write_to_file(output).unwrap();
        }
        _ => {
            // Assume other formats are image formats for now.
            // TODO: properly flatten 3D images in image_dds.
            let image = image_dds::image_from_dds(&dds, 0).unwrap();
            image.save(output).unwrap();
        }
    }
}

// TODO: Move this to xc3_lib?
fn read_wismt_single_tex<P: AsRef<Path>>(path: P) -> Mibl {
    Xbc1::from_file(path).unwrap().extract().unwrap()
}

fn create_wismt_single_tex(mibl: &Mibl) -> Xbc1 {
    // TODO: Set the name properly.
    Xbc1::new("b2062367_middle.witx".to_string(), mibl)
}
