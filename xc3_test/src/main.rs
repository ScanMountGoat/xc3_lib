use std::{io::Cursor, path::Path};

use clap::Parser;
use rayon::prelude::*;
use xc3_lib::{
    dds::{create_dds, create_mibl},
    mibl::Mibl,
    model::ModelData,
    msrd::{DataItemType, Msrd},
    mxmd::Mxmd,
    sar1::Sar1,
    spch::Spch,
    xbc1::Xbc1,
};

fn check_all_mxmd<P: AsRef<Path>>(root: P) {
    // The map folder is a different format?
    globwalk::GlobWalkerBuilder::from_patterns(root, &["*.wimdo", "!map/**"])
        .build()
        .unwrap()
        .par_bridge()
        .for_each(|entry| {
            let path = entry.as_ref().unwrap().path();
            // TODO: How to validate this file?
            match Mxmd::from_file(path) {
                Ok(_) => (),
                Err(e) => println!("Error reading {path:?}: {e}"),
            }
        });
}

fn check_all_mibl<P: AsRef<Path>>(root: P) {
    // The h directory doesn't have mibl footers?
    let folder = root.as_ref().join("chr").join("tex").join("nx");
    globwalk::GlobWalkerBuilder::from_patterns(folder, &["*.wismt", "!h/**"])
        .build()
        .unwrap()
        .par_bridge()
        .for_each(|entry| {
            let path = entry.as_ref().unwrap().path();
            let (original_bytes, mibl) = read_wismt_single_tex(path);
            check_mibl(original_bytes, mibl, path);
        });

    let folder = root.as_ref().join("monolib").join("shader");
    globwalk::GlobWalkerBuilder::from_patterns(folder, &["*.{witex,witx}"])
        .build()
        .unwrap()
        .par_bridge()
        .for_each(|entry| {
            let path = entry.as_ref().unwrap().path();
            let original_bytes = std::fs::read(path).unwrap();
            let mibl = Mibl::from_file(path).unwrap();
            check_mibl(original_bytes, mibl, path);
        });
}

fn check_all_msrd<P: AsRef<Path>>(root: P) {
    let folder = root.as_ref().join("chr");

    // Skip the .wismt textures in the tex folder.
    globwalk::GlobWalkerBuilder::from_patterns(folder, &["*.wismt", "!tex/**"])
        .build()
        .unwrap()
        .par_bridge()
        .for_each(|entry| {
            let path = entry.as_ref().unwrap().path();
            match Msrd::from_file(path) {
                Ok(msrd) => {
                    check_msrd(msrd);
                }
                Err(e) => println!("Error reading {path:?}: {e}"),
            }
        });
}

fn check_msrd(msrd: Msrd) {
    let toc_streams: Vec<_> = msrd
        .tocs
        .iter()
        .map(|toc| toc.xbc1.decompress().unwrap())
        .collect();

    // Check parsing for any embedded files.
    for item in msrd.data_items {
        match item.item_type {
            DataItemType::ShaderBundle => {
                let stream = &toc_streams[item.toc_index as usize];
                let data = &stream[item.offset as usize..item.offset as usize + item.size as usize];

                Spch::read(&mut Cursor::new(data)).unwrap();
            }
            DataItemType::Model => {
                let stream = &toc_streams[item.toc_index as usize];
                let data = &stream[item.offset as usize..item.offset as usize + item.size as usize];

                ModelData::read(&mut Cursor::new(data)).unwrap();
            }
            // TODO: check textures
            DataItemType::CachedTexture => {}
            DataItemType::Texture => {}
        }
    }
}

fn check_mibl(original_bytes: Vec<u8>, mibl: Mibl, path: &Path) {
    let dds = create_dds(&mibl).unwrap();
    let new_mibl = create_mibl(&dds).unwrap();

    let mut writer = Cursor::new(Vec::new());
    new_mibl.write(&mut writer).unwrap();

    // DDS should support all MIBL image formats.
    // Check that read -> MIBL -> DDS -> MIBL -> write is 1:1.
    if original_bytes != writer.into_inner() {
        println!("Read/write not 1:1 for {path:?}");
    };
}

fn read_wismt_single_tex<P: AsRef<Path>>(path: P) -> (Vec<u8>, Mibl) {
    let xbc1 = Xbc1::from_file(path).unwrap();

    let decompressed = xbc1.decompress().unwrap();
    let mut reader = Cursor::new(decompressed.clone());
    (decompressed, Mibl::read(&mut reader).unwrap())
}

fn check_all_sar1<P: AsRef<Path>>(root: P) {
    let folder = root.as_ref().join("chr");
    globwalk::GlobWalkerBuilder::from_patterns(folder, &["*.chr"])
        .build()
        .unwrap()
        .par_bridge()
        .for_each(|entry| {
            // TODO: How to validate this file?
            let path = entry.as_ref().unwrap().path();
            match Sar1::from_file(path) {
                Ok(_) => (),
                Err(e) => println!("Error reading {path:?}: {e}"),
            }
        });
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    /// The root folder that contains folders like `chr/` and `monolib/`.
    root_folder: String,

    /// Process MIBL image files from .witex, .witx, .wismt
    #[arg(long)]
    mibl: bool,

    /// Process MXMD model files from .wimdo
    #[arg(long)]
    mxmd: bool,

    /// Process MSRD model files from .wismt
    #[arg(long)]
    msrd: bool,

    /// Process SAR1 model files from .chr
    #[arg(long)]
    sar1: bool,

    /// Process all file types
    #[arg(long)]
    all: bool,
}

fn main() {
    // Create a CLI for conversion testing instead of unit tests.
    // The main advantage is being able to avoid distributing assets.
    // The user can specify the path instead of hardcoding it.
    // It's also easier to apply optimizations like multithreading.

    let cli = Cli::parse();
    let root = Path::new(&cli.root_folder);

    let start = std::time::Instant::now();

    // Check conversions for various file types.
    if cli.mibl || cli.all {
        println!("Checking MIBL files ...");
        check_all_mibl(root);
    }

    if cli.mxmd || cli.all {
        println!("Checking MXMD files ...");
        check_all_mxmd(root);
    }

    if cli.msrd || cli.all {
        println!("Checking MSRD files ...");
        check_all_msrd(root);
    }

    if cli.sar1 || cli.all {
        println!("Checking SAR1 files ...");
        check_all_sar1(root);
    }

    // TODO: check shaders

    println!("Finished in {:?}", start.elapsed());
}
