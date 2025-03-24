use std::{
    path::{Path, PathBuf},
    str::FromStr,
};

use anyhow::Context;
use clap::{builder::PossibleValuesParser, Parser, Subcommand};
use convert::{
    batch_convert_files, extract_wilay_to_folder, extract_wimdo_to_folder, update_wifnt,
    update_wilay_from_folder, update_wimdo_from_folder, File, SaveImageExt, Wilay,
};
use image_dds::{ddsfile::Dds, image, ImageFormat, Quality};
use strum::IntoEnumIterator;
use xc3_lib::{
    bmn::Bmn,
    dds::DdsExt,
    fnt::Fnt,
    laft::Laft,
    mibl::Mibl,
    mtxt::Mtxt,
    mxmd::{legacy::MxmdLegacy, Mxmd},
    xbc1::MaybeXbc1,
};

use crate::convert::{extract_bmn_to_folder, extract_camdo_to_folder};

/// Convert texture files for Xenoblade X, Xenoblade 1 DE, Xenoblade 2, Xenoblade 3, and Xenoblade X DE.
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
    /// All of the supported input formats also work as output formats except for wismt.
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
    /// Convert a width x (height * 6) image into a square cube map.
    /// DDS inputs should instead use the appropriate flags.
    #[arg(long)]
    cube: bool,
    /// Convert a width x (height * depth) image into a square 3D texture. Does not apply to DDS.
    /// DDS inputs should instead use the appropriate flags.
    #[arg(long)]
    depth: bool,
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
        chr_folder: Option<String>,
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
    /// Recursively convert all files in a folder.
    BatchConvert {
        /// The root folder to search recursively for images.
        input_folder: String,
        /// The glob pattern for the input folder like "*.wilay" or "*.{witex, witx}".
        pattern: String,
        /// The output file extension like "dds".
        /// Most uncompressed image formats like png, tiff, or jpeg are also supported.
        /// This also selects the file format used for saving.
        /// Defaults to "png" if not specified.
        ext: Option<String>,
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

    // TODO: Count successes and failures?
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
                chr_folder,
            } => {
                let count = update_wimdo_from_folder(
                    &input,
                    &input_folder,
                    output.as_ref().unwrap_or(&input),
                    chr_folder,
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
            Commands::BatchConvert {
                input_folder,
                pattern,
                ext,
            } => {
                let count = batch_convert_files(&input_folder, &pattern, ext.as_deref())?;
                println!("Converted {count} file(s) in {:?}", start.elapsed());
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

        match input_file {
            File::Wilay(wilay) => {
                // Wilay contains multiple images that need to be saved.
                std::fs::create_dir_all(&output)
                    .with_context(|| format!("failed to create output directory {output:?}"))?;

                let count = extract_wilay_to_folder(*wilay, &input, &output)?;
                println!("Converted {count} file(s) in {:?}", start.elapsed());
            }
            File::Wimdo(wimdo) => {
                // wimdo and wismt contain multiple images that need to be saved.
                std::fs::create_dir_all(&output)
                    .with_context(|| format!("failed to create output directory {output:?}"))?;

                let count = extract_wimdo_to_folder(*wimdo, &input, &output)?;
                println!("Converted {count} file(s) in {:?}", start.elapsed());
            }
            File::Camdo(camdo) => {
                // camdo and casmt contain multiple images that need to be saved.
                std::fs::create_dir_all(&output)
                    .with_context(|| format!("failed to create output directory {output:?}"))?;

                let count = extract_camdo_to_folder(*camdo, &input, &output)?;
                println!("Converted {count} file(s) in {:?}", start.elapsed());
            }
            File::Bmn(bmn) => {
                // bmn contain multiple images that need to be saved.
                std::fs::create_dir_all(&output)
                    .with_context(|| format!("failed to create output directory {output:?}"))?;

                let count = extract_bmn_to_folder(bmn, &input, &output)?;
                println!("Converted {count} file(s) in {:?}", start.elapsed());
            }
            _ => {
                if let Some(parent) = output.parent() {
                    std::fs::create_dir_all(parent)
                        .with_context(|| format!("failed to create output directory {parent:?}"))?;
                }

                // All other formats save to single files.
                let format = args.format.map(|f| ImageFormat::from_str(&f)).transpose()?;
                let quality = args.quality.map(|f| Quality::from_str(&f)).transpose()?;
                let mipmaps = !args.no_mipmaps;
                let cube = args.cube;
                let depth = args.depth;

                match output.extension().unwrap().to_str().unwrap() {
                    "dds" => {
                        input_file
                            .to_dds(format, quality, mipmaps, cube, depth)?
                            .save(&output)
                            .with_context(|| format!("failed to save DDS to {output:?}"))?;
                    }
                    "witex" | "witx" => {
                        input_file
                            .to_mibl(format, quality, mipmaps)?
                            .save(&output)?;
                    }
                    "wismt" => {
                        anyhow::bail!("Creating .wismt files is not supported. Edit an existing model's textures with the edit-wimdo command.");
                    }
                    // TODO: Resave xenoblade x textures?
                    _ => {
                        // Assume other formats are image formats for now.
                        input_file
                            .to_image()?
                            .save_image(&output)
                            .with_context(|| format!("failed to save image to {output:?}"))?;
                    }
                }
                println!("Converted 1 file in {:?}", start.elapsed());
            }
        }
    }
    Ok(())
}

fn load_input_file(input: &Path) -> anyhow::Result<File> {
    match input.extension().unwrap().to_str().unwrap() {
        "witex" | "witx" => Mibl::from_file(input)
            .with_context(|| format!("{input:?} is not a valid .witex file"))
            .map(File::Mibl),
        "dds" => Dds::from_file(input)
            .with_context(|| format!("{input:?} is not a valid .dds file"))
            .map(File::Dds),
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
        "catex" | "calut" => Mtxt::from_file(input)
            .with_context(|| format!("{input:?} is not a valid .catex file"))
            .map(File::Mtxt),
        "bmn" => Bmn::from_file(input)
            .with_context(|| format!("{input:?} is not a valid .bmn file"))
            .map(File::Bmn),
        "wifnt" => MaybeXbc1::<Laft>::from_file(input)
            .with_context(|| format!("{input:?} is not a valid .wifnt file"))
            .map(File::Wifnt),
        "fnt" => Fnt::from_file(input)
            .with_context(|| format!("{input:?} is not a valid .fnt file"))
            .map(File::XcxFnt),
        "caavp" => {
            // caavp files have multiple embedded mtxt files.
            // TODO: Move this logic to xc3_lib?
            let bytes = std::fs::read(input)?;
            let mut mtxts = Vec::new();
            let mut start = 0;
            for i in (0..bytes.len()).step_by(4) {
                if matches!(bytes.get(i..i + 4), Some(b"MTXT")) {
                    let mtxt = Mtxt::from_bytes(&bytes[start..i + 4])?;
                    mtxts.push(mtxt);
                    start = i + 4;
                }
            }
            Ok(File::Caavp(mtxts))
        }
        _ => {
            // Assume other formats are image formats.
            let image = image::open(input)
                .with_context(|| format!("{input:?} is not a valid image file"))?
                .to_rgba8();
            Ok(File::Image(image))
        }
    }
}
