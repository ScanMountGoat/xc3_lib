use std::{path::PathBuf, str::FromStr};

use anyhow::Context;
use clap::{builder::PossibleValuesParser, Parser, Subcommand};
use convert::{
    create_wismt_single_tex, extract_wilay_to_folder, extract_wimdo_to_folder,
    read_wismt_single_tex, update_wifnt, update_wilay_from_folder, update_wimdo_from_folder, File,
    Wilay,
};
use image_dds::{ddsfile::Dds, image, ImageFormat, Quality};
use strum::IntoEnumIterator;
use xc3_lib::{
    bmn::Bmn,
    dds::DdsExt,
    laft::Laft,
    mibl::Mibl,
    mtxt::Mtxt,
    mxmd::{legacy::MxmdLegacy, Mxmd},
    xbc1::MaybeXbc1,
};

use crate::convert::{extract_bmn_to_folder, extract_camdo_to_folder};

/// Convert texture files for Xenoblade X, Xenoblade 1 DE, Xenoblade 2, and Xenoblade 3.
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
    /// The input dds, witex, witx, wimdo, wismt, camdo, catex, or calut file.
    /// Most uncompressed image formats like png, tiff, or jpeg are also supported.
    // TODO: how to make this required?
    input: String,
    /// The output file or the output folder when the input is a wimdo or wilay.
    /// All of the supported input formats also work as output formats.
    output: Option<String>,
    /// The compression format when saving as a file like dds or witex
    #[arg(long, value_parser = PossibleValuesParser::new(ImageFormat::iter().map(|f| f.to_string())))]
    format: Option<String>,
    /// The compression quality when saving as a file like dds or witex
    #[arg(long, value_parser = PossibleValuesParser::new(Quality::iter().map(|f| f.to_string())))]
    quality: Option<String>,
    /// Don't include any mipmaps when saving as a file like dds or witex
    #[arg(long)]
    no_mipmaps: bool,
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
        /// The "chr/tex/nx" texture folder for the input's external wismt textures.
        /// Required for most Xenoblade 3 models if the folder
        /// cannot be inferred from the input path.
        chr_tex_nx: Option<String>,
    },
    /// Replace the Mibl in a .wifnt file.
    EditWifnt {
        /// The original .wifnt file.
        input: String,
        /// The DDS font texture to use for the .wifnt file.
        input_image: String,
        /// The output file. Defaults to the same as the input when not specified.
        output: Option<String>,
    },
}

mod convert;

fn main() -> anyhow::Result<()> {
    simple_logger::SimpleLogger::new()
        .with_level(log::LevelFilter::Warn)
        .init()
        .unwrap();

    let cli = Cli::parse();

    let start = std::time::Instant::now();

    if let Some(cmd) = cli.subcommand {
        match cmd {
            Commands::EditWilay {
                input,
                input_folder,
                output,
            } => {
                let count = update_wilay_from_folder(
                    &input,
                    &input_folder,
                    output.as_ref().unwrap_or(&input),
                )?;
                println!("Converted {count} file(s) in {:?}", start.elapsed());
            }
            Commands::EditWimdo {
                input,
                input_folder,
                output,
                chr_tex_nx,
            } => {
                let count = update_wimdo_from_folder(
                    &input,
                    &input_folder,
                    output.as_ref().unwrap_or(&input),
                    chr_tex_nx,
                )?;
                println!("Converted {count} file(s) in {:?}", start.elapsed());
            }
            Commands::EditWifnt {
                input,
                input_image,
                output,
            } => {
                update_wifnt(&input, &input_image, output.as_ref().unwrap_or(&input))?;
                println!("Converted 1 file in {:?}", start.elapsed());
            }
        }
    } else if let Some(args) = cli.args {
        let input = PathBuf::from(&args.input);

        // TODO: Support floating point images.
        // TODO: Specify quality and mipmaps?
        let input_file = load_input_file(&input)?;

        // Default to DDS since it supports more formats.
        // Wilay can output their images to the current folder.
        let output = args
            .output
            .map(PathBuf::from)
            .unwrap_or_else(|| match input_file {
                File::Wilay(_) | File::Wimdo(_) => input.parent().unwrap().to_owned(),
                _ => input.with_extension("dds"),
            });

        if let File::Wilay(wilay) = input_file {
            // Wilay contains multiple images that need to be saved.
            std::fs::create_dir_all(&output)
                .with_context(|| format!("failed to create output directory {output:?}"))?;

            let count = extract_wilay_to_folder(*wilay, &input, &output)?;
            println!("Converted {count} file(s) in {:?}", start.elapsed());
        } else if let File::Wimdo(wimdo) = input_file {
            // wimdo and wismt contain multiple images that need to be saved.
            std::fs::create_dir_all(&output)
                .with_context(|| format!("failed to create output directory {output:?}"))?;

            let count = extract_wimdo_to_folder(*wimdo, &input, &output)?;
            println!("Converted {count} file(s) in {:?}", start.elapsed());
        } else if let File::Camdo(camdo) = input_file {
            // camdo and casmt contain multiple images that need to be saved.
            std::fs::create_dir_all(&output)
                .with_context(|| format!("failed to create output directory {output:?}"))?;

            let count = extract_camdo_to_folder(*camdo, &input, &output)?;
            println!("Converted {count} file(s) in {:?}", start.elapsed());
        } else if let File::Bmn(bmn) = input_file {
            // bmn contain multiple images that need to be saved.
            std::fs::create_dir_all(&output)
                .with_context(|| format!("failed to create output directory {output:?}"))?;

            let count = extract_bmn_to_folder(bmn, &input, &output)?;
            println!("Converted {count} file(s) in {:?}", start.elapsed());
        } else {
            if let Some(parent) = output.parent() {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("failed to create output directory {parent:?}"))?;
            }

            // All other formats save to single files.
            let format = args.format.map(|f| ImageFormat::from_str(&f)).transpose()?;
            let quality = args.quality.map(|f| Quality::from_str(&f)).transpose()?;
            let mipmaps = !args.no_mipmaps;

            match output.extension().unwrap().to_str().unwrap() {
                "dds" => {
                    input_file
                        .to_dds(format, quality, mipmaps)?
                        .save(&output)
                        .with_context(|| format!("failed to save DDS to {output:?}"))?;
                }
                "witex" | "witx" => {
                    input_file
                        .to_mibl(format, quality, mipmaps)?
                        .save(&output)?;
                }
                "wismt" => {
                    // TODO: Also create base level?
                    let mibl = input_file.to_mibl(format, quality, mipmaps)?;
                    let xbc1 = create_wismt_single_tex(&mibl)?;
                    xbc1.save(&output)?;
                }
                // TODO: Resave xenoblade x textures?
                _ => {
                    // Assume other formats are image formats for now.
                    input_file
                        .to_image()?
                        .save(&output)
                        .with_context(|| format!("failed to save image to {output:?}"))?;
                }
            }
            println!("Converted 1 file in {:?}", start.elapsed());
        }
    }
    Ok(())
}

fn load_input_file(input: &PathBuf) -> anyhow::Result<File> {
    match input.extension().unwrap().to_str().unwrap() {
        "witex" | "witx" => Mibl::from_file(input)
            .with_context(|| format!("{input:?} is not a valid .witex file"))
            .map(File::Mibl),
        "dds" => Dds::from_file(input)
            .with_context(|| format!("{input:?} is not a valid .dds file"))
            .map(File::Dds),
        "wismt" => read_wismt_single_tex(input)
            .with_context(|| format!("{input:?} is not a valid .wismt file"))
            .map(File::Mibl),
        "wilay" => Ok(File::Wilay(Box::new(
            MaybeXbc1::<Wilay>::from_file(input)
                .with_context(|| format!("{input:?} is not a valid .wilay file"))?,
        ))),
        "wimdo" => Mxmd::from_file(input)
            .with_context(|| format!("{input:?} is not a valid .wimdo file"))
            .map(Box::new)
            .map(File::Wimdo),
        "camdo" => MxmdLegacy::from_file(input)
            .with_context(|| format!("{input:?} is not a valid .camdo file"))
            .map(Box::new)
            .map(File::Camdo),
        "catex" | "calut" | "caavp" => Mtxt::from_file(input)
            .with_context(|| format!("{input:?} is not a valid .catex file"))
            .map(File::Mtxt),
        "bmn" => Bmn::from_file(input)
            .with_context(|| format!("{input:?} is not a valid .bmn file"))
            .map(File::Bmn),
        "wifnt" => MaybeXbc1::<Laft>::from_file(input)
            .with_context(|| format!("{input:?} is not a valid .wifnt file"))
            .map(File::Wifnt),
        _ => {
            // Assume other formats are image formats.
            let image = image::open(input)
                .with_context(|| format!("{input:?} is not a valid image file"))?
                .to_rgba8();
            Ok(File::Image(image))
        }
    }
}
