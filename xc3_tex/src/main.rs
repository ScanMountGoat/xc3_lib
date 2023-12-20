use std::path::{Path, PathBuf};

use clap::Parser;
use image_dds::{
    ddsfile::Dds,
    image::{self, RgbaImage},
    ImageFormat, Surface,
};
use xc3_lib::{dds::DdsExt, dhal::Dhal, lagp::Lagp, mibl::Mibl, xbc1::Xbc1};

/// Convert texture files for Xenoblade 1 DE, Xenoblade 2, and Xenoblade 3.
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    /// The input dds, witex, witx, or wismt file.
    /// Most uncompressed image formats like png, tiff, or jpeg are also supported.
    input: String,
    /// The output file or the output folder when the input is a wilay.
    /// All of the supported input formats also work as output formats.
    output: Option<String>,
    /// The compression format like BC7Unorm when saving as a file like dds or witex
    format: Option<ImageFormat>,
}

// TODO: Module for this?
enum File {
    Mibl(Mibl),
    Dds(Dds),
    Image(RgbaImage),
    Wilay(Wilay),
}

enum Wilay {
    Dhal(Dhal),
    Lagp(Lagp),
}

impl File {
    fn to_dds(&self, format: Option<ImageFormat>) -> Dds {
        match self {
            File::Mibl(mibl) => mibl.to_dds().unwrap(),
            File::Dds(dds) => {
                // Handle changes in image format while preserving layers and mipmaps.
                Surface::from_dds(&dds)
                    .unwrap()
                    .decode_rgba8()
                    .unwrap()
                    .encode(
                        format.unwrap(),
                        image_dds::Quality::Normal,
                        image_dds::Mipmaps::GeneratedAutomatic,
                    )
                    .unwrap()
                    .to_dds()
                    .unwrap()
            }
            File::Image(image) => image_dds::dds_from_image(
                &image,
                format.unwrap(),
                image_dds::Quality::Normal,
                image_dds::Mipmaps::GeneratedAutomatic,
            )
            .unwrap(),
            File::Wilay(_) => {
                panic!("Wilay must be saved to an output folder instead of a single image")
            }
        }
    }

    fn to_mibl(&self, format: Option<ImageFormat>) -> Mibl {
        match self {
            File::Mibl(mibl) => mibl.clone(),
            File::Dds(dds) => Mibl::from_dds(dds).unwrap(),
            File::Image(image) => {
                let dds = image_dds::dds_from_image(
                    &image,
                    format.unwrap(),
                    image_dds::Quality::Normal,
                    image_dds::Mipmaps::GeneratedAutomatic,
                )
                .unwrap();
                Mibl::from_dds(&dds).unwrap()
            }
            File::Wilay(_) => {
                panic!("Wilay must be saved to an output folder instead of a single image")
            }
        }
    }

    fn to_image(&self) -> RgbaImage {
        match self {
            File::Mibl(mibl) => image_dds::image_from_dds(&mibl.to_dds().unwrap(), 0).unwrap(),
            File::Dds(dds) => image_dds::image_from_dds(dds, 0).unwrap(),
            File::Image(image) => image.clone(),
            File::Wilay(_) => {
                panic!("Wilay must be saved to an output folder instead of a single image")
            }
        }
    }
}

fn main() {
    let cli = Cli::parse();

    let input = PathBuf::from(&cli.input);

    // TODO: Support floating point images.
    // TODO: Specify quality and mipmaps?
    let input_file = match input.extension().unwrap().to_str().unwrap() {
        "witex" | "witx" => File::Mibl(Mibl::from_file(&input).unwrap()),
        "dds" => File::Dds(Dds::from_file(&input).unwrap()),
        "wismt" => File::Mibl(read_wismt_single_tex(&input)),
        "wilay" => {
            // TODO: Move this to xc3_lib?
            let wilay = match Xbc1::from_file(&input) {
                Ok(xbc1) => xbc1
                    .extract()
                    .map(Wilay::Dhal)
                    .unwrap_or_else(|_| xbc1.extract().map(Wilay::Lagp).unwrap()),
                Err(_) => Dhal::from_file(&input)
                    .map(Wilay::Dhal)
                    .unwrap_or_else(|_| Lagp::from_file(&input).map(Wilay::Lagp).unwrap()),
            };
            File::Wilay(wilay)
        }
        _ => {
            // Assume other formats are image formats for now.
            File::Image(image::open(&input).unwrap().to_rgba8())
        }
    };

    // Default to DDS since it supports more formats.
    // Wilay can output their images to the current folder.
    let output = cli
        .output
        .map(PathBuf::from)
        .unwrap_or_else(|| match input_file {
            File::Wilay(_) => input.parent().unwrap().to_owned(),
            _ => input.with_extension("dds"),
        });

    if let File::Wilay(wilay) = input_file {
        // Wilay contains multiple images that need to be saved.
        std::fs::create_dir_all(&output).unwrap();
        save_wilay_to_folder(wilay, &input, &output);
    } else {
        // All other formats save to single files.
        match output.extension().unwrap().to_str().unwrap() {
            "dds" => {
                input_file.to_dds(cli.format).save(output).unwrap();
            }
            "witex" | "witx" => {
                let mibl = input_file.to_mibl(cli.format);
                mibl.write_to_file(output).unwrap();
            }
            "wismt" => {
                // TODO: Also create base level?
                let mibl = input_file.to_mibl(cli.format);
                let xbc1 = create_wismt_single_tex(&mibl);
                xbc1.write_to_file(output).unwrap();
            }
            _ => {
                // Assume other formats are image formats for now.
                let image = input_file.to_image();
                image.save(output).unwrap();
            }
        }
    }
}

fn save_wilay_to_folder(wilay: Wilay, input: &Path, output: &Path) {
    let file_name = input.file_name().unwrap();
    match wilay {
        Wilay::Dhal(dhal) => {
            if let Some(textures) = dhal.textures {
                for (i, texture) in textures.textures.iter().enumerate() {
                    let dds = Mibl::from_bytes(&texture.mibl_data)
                        .unwrap()
                        .to_dds()
                        .unwrap();
                    let path = output.join(file_name).with_extension(format!("{i}.dds"));
                    dds.save(path).unwrap();
                }
            }
            if let Some(textures) = dhal.uncompressed_textures {
                for (i, texture) in textures.textures.iter().enumerate() {
                    let path = output.join(file_name).with_extension(format!("{i}.jpeg"));
                    std::fs::write(path, &texture.jpeg_data).unwrap();
                }
            }
        }
        Wilay::Lagp(lagp) => {
            if let Some(textures) = lagp.textures {
                for (i, texture) in textures.textures.iter().enumerate() {
                    let dds = Mibl::from_bytes(&texture.mibl_data)
                        .unwrap()
                        .to_dds()
                        .unwrap();
                    let path = output.join(file_name).with_extension(format!("{i}.dds"));
                    dds.save(path).unwrap();
                }
            }
        }
    }
}

// TODO: Move this to xc3_lib?
fn read_wismt_single_tex<P: AsRef<Path>>(path: P) -> Mibl {
    Xbc1::from_file(path).unwrap().extract().unwrap()
}

fn create_wismt_single_tex(mibl: &Mibl) -> Xbc1 {
    // TODO: Set the name properly.
    Xbc1::new("b2062367_middle.witx".to_string(), mibl).unwrap()
}
