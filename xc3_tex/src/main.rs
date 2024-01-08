use std::path::PathBuf;

use clap::{
    builder::{PossibleValue, PossibleValuesParser},
    Parser, Subcommand,
};
use convert::{
    create_wismt_single_tex, read_wismt_single_tex, save_wilay_to_folder, update_wilay_from_folder,
    update_wimdo_from_folder, File, Wilay,
};
use image_dds::{
    ddsfile::Dds,
    image::{self},
    ImageFormat,
};
use strum::IntoEnumIterator;
use xc3_lib::{dds::DdsExt, mibl::Mibl};

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
    #[arg(value_parser = PossibleValuesParser::new(ImageFormat::iter().map(|f| f.to_string())))]
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
    /// Replace the Mibl in a .wimdo file and its associated .wismt file.
    EditWimdo {
        /// The original .wimdo file.
        input: String,
        /// The folder containing images to replace with the input file name
        /// followed by the image index like "input.0.dds".
        input_folder: String,
        /// The output file. Defaults to the same as the input when not specified.
        output: Option<String>,
    },
}

mod convert;

fn main() {
    let cli = Cli::parse();

    if let Some(cmd) = cli.subcommand {
        match cmd {
            Commands::EditWilay {
                input,
                input_folder,
                output,
            } => update_wilay_from_folder(&input, &input_folder, output.as_ref().unwrap_or(&input)),
            Commands::EditWimdo {
                input,
                input_folder,
                output,
            } => update_wimdo_from_folder(&input, &input_folder, output.as_ref().unwrap_or(&input)),
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
                    mibl.save(output).unwrap();
                }
                "wismt" => {
                    // TODO: Also create base level?
                    let mibl = input_file.to_mibl(args.format);
                    let xbc1 = create_wismt_single_tex(&mibl);
                    xbc1.save(output).unwrap();
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
