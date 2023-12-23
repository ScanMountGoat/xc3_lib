use std::{
    io::Cursor,
    path::{Path, PathBuf},
};

use clap::{Parser, Subcommand};
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
#[command(args_conflicts_with_subcommands = true)]
struct Cli {
    // Optional subcommands to still allow drag and drop if no subcommand.
    #[clap(subcommand)]
    pub subcommand: Option<Commands>,

    #[command(flatten)]
    pub args: Option<ConvertArgs>,
}

#[derive(Parser)]
struct ConvertArgs {
    /// The input dds, witex, witx, or wismt file.
    /// Most uncompressed image formats like png, tiff, or jpeg are also supported.
    // TODO: how to make this required?
    input: String,
    /// The output file or the output folder when the input is a wilay.
    /// All of the supported input formats also work as output formats.
    output: Option<String>,
    /// The compression format like BC7Unorm when saving as a file like dds or witex
    format: Option<ImageFormat>,
}

#[derive(Subcommand)]
enum Commands {
    /// Replace the Mibl and JPEG in a .wilay file.
    EditWilay {
        /// The original .wilay file.
        input: String,
        /// The folder containing images to replace with the input file name
        /// followed by the image index like "input.0.dds" or "input.3.jpeg".
        input_folder: String,
        /// The output file. Defaults to the same as the input when not specified.
        output: Option<String>,
    },
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

impl Wilay {
    // TODO: Move this to xc3_lib?
    fn from_file<P: AsRef<Path>>(path: P) -> Self {
        let path = path.as_ref();
        match Xbc1::from_file(path) {
            Ok(xbc1) => xbc1
                .extract()
                .map(Wilay::Dhal)
                .unwrap_or_else(|_| xbc1.extract().map(Wilay::Lagp).unwrap()),
            Err(_) => Dhal::from_file(path)
                .map(Wilay::Dhal)
                .unwrap_or_else(|_| Lagp::from_file(path).map(Wilay::Lagp).unwrap()),
        }
    }
}

impl File {
    fn to_dds(&self, format: Option<ImageFormat>) -> Dds {
        match self {
            File::Mibl(mibl) => mibl.to_dds().unwrap(),
            File::Dds(dds) => {
                // Handle changes in image format while preserving layers and mipmaps.
                // TODO: dds doesn't implement clone?
                match format {
                    Some(format) => Surface::from_dds(dds)
                        .unwrap()
                        .decode_rgba8()
                        .unwrap()
                        .encode(
                            format,
                            image_dds::Quality::Normal,
                            image_dds::Mipmaps::GeneratedAutomatic,
                        )
                        .unwrap()
                        .to_dds()
                        .unwrap(),
                    None => Dds {
                        header: dds.header.clone(),
                        header10: dds.header10.clone(),
                        data: dds.data.clone(),
                    },
                }
            }
            File::Image(image) => image_dds::dds_from_image(
                image,
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
                    image,
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

    if let Some(cmd) = cli.subcommand {
        match cmd {
            Commands::EditWilay {
                input,
                input_folder,
                output,
            } => update_wilay_from_folder(&input, &input_folder, output.as_ref().unwrap_or(&input)),
        }
    } else if let Some(args) = cli.args {
        let input = PathBuf::from(&args.input);

        // TODO: Support floating point images.
        // TODO: Specify quality and mipmaps?
        let input_file = match input.extension().unwrap().to_str().unwrap() {
            "witex" | "witx" => File::Mibl(Mibl::from_file(&input).unwrap()),
            "dds" => File::Dds(Dds::from_file(&input).unwrap()),
            "wismt" => File::Mibl(read_wismt_single_tex(&input)),
            "wilay" => File::Wilay(Wilay::from_file(&input)),
            _ => {
                // Assume other formats are image formats for now.
                File::Image(image::open(&input).unwrap().to_rgba8())
            }
        };

        // Default to DDS since it supports more formats.
        // Wilay can output their images to the current folder.
        let output = args
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
            if let Some(parent) = output.parent() {
                std::fs::create_dir_all(parent).unwrap();
            }

            // All other formats save to single files.
            match output.extension().unwrap().to_str().unwrap() {
                "dds" => {
                    input_file.to_dds(args.format).save(output).unwrap();
                }
                "witex" | "witx" => {
                    let mibl = input_file.to_mibl(args.format);
                    mibl.write_to_file(output).unwrap();
                }
                "wismt" => {
                    // TODO: Also create base level?
                    let mibl = input_file.to_mibl(args.format);
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
}

fn update_wilay_from_folder(input: &str, input_folder: &str, output: &str) {
    // Replace existing images in a .wilay file.
    // TODO: Error if indices are out of range?
    // TODO: match the original if it uses xbc1 compression?
    let mut wilay = Wilay::from_file(input);
    match &mut wilay {
        Wilay::Dhal(dhal) => {
            if let Some(textures) = &mut dhal.textures {
                replace_wilay_mibl(textures, input, input_folder);
            }
            if let Some(textures) = &mut dhal.uncompressed_textures {
                replace_wilay_jpeg(textures, input, input_folder);
            }
            dhal.write_to_file(output).unwrap();
        }
        Wilay::Lagp(lagp) => {
            if let Some(textures) = &mut lagp.textures {
                replace_wilay_mibl(textures, input, input_folder);
            }
            lagp.write_to_file(output).unwrap();
        }
    }
}

fn replace_wilay_mibl(textures: &mut xc3_lib::dhal::Textures, input: &str, input_folder: &str) {
    for entry in std::fs::read_dir(input_folder).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) == Some("dds") {
            if let Some(i) = image_index(&path, input) {
                // TODO: Add a to_bytes helper?
                let dds = Dds::from_file(path).unwrap();
                let mibl = Mibl::from_dds(&dds).unwrap();
                let mut writer = Cursor::new(Vec::new());
                mibl.write(&mut writer).unwrap();

                textures.textures[i].mibl_data = writer.into_inner();
            }
        }
    }
}

fn replace_wilay_jpeg(
    textures: &mut xc3_lib::dhal::UncompressedTextures,
    input: &str,
    input_folder: &str,
) {
    for entry in std::fs::read_dir(input_folder).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) == Some("jpeg") {
            if let Some(i) = image_index(&path, input) {
                textures.textures[i].jpeg_data = std::fs::read(path).unwrap();
            }
        }
    }
}

fn image_index(path: &Path, input: &str) -> Option<usize> {
    // Match the input file name in case the folder contains multiple wilay.
    // "mnu417_cont01.88.dds" -> 88
    let path = path.with_extension("");
    let file_name = path.file_name()?.to_str()?;
    let (file_name, index) = file_name.rsplit_once('.')?;

    let input_file_name = Path::new(input).with_extension("");
    let input_file_name = input_file_name.file_name()?.to_str()?;
    if file_name == input_file_name {
        index.parse().ok()
    } else {
        None
    }
}

fn save_wilay_to_folder(wilay: Wilay, input: &Path, output_folder: &Path) {
    let file_name = input.file_name().unwrap();
    match wilay {
        Wilay::Dhal(dhal) => {
            if let Some(textures) = dhal.textures {
                for (i, texture) in textures.textures.iter().enumerate() {
                    let dds = Mibl::from_bytes(&texture.mibl_data)
                        .unwrap()
                        .to_dds()
                        .unwrap();
                    let path = output_folder
                        .join(file_name)
                        .with_extension(format!("{i}.dds"));
                    dds.save(path).unwrap();
                }
            }
            if let Some(textures) = dhal.uncompressed_textures {
                for (i, texture) in textures.textures.iter().enumerate() {
                    let path = output_folder
                        .join(file_name)
                        .with_extension(format!("{i}.jpeg"));
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
                    let path = output_folder
                        .join(file_name)
                        .with_extension(format!("{i}.dds"));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_index_paths() {
        assert_eq!(
            Some(0),
            image_index(&Path::new("a/b/file.0.dds"), "b/c/file.wilay")
        );
        assert_eq!(
            Some(7),
            image_index(&Path::new("file.7.dds"), "b/c/file.wilay")
        );
        assert_eq!(Some(7), image_index(&Path::new("file.7.dds"), "file.wilay"));
        assert_eq!(
            None,
            image_index(&Path::new("file2.7.dds"), "b/c/file.wilay")
        );
        assert_eq!(
            None,
            image_index(&Path::new("a/b/file.0.dds"), "b/c/file2.wilay")
        );
    }
}
