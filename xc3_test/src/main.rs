use std::{
    io::{BufReader, Cursor},
    path::Path,
};

use clap::Parser;
use rayon::prelude::*;
use xc3_lib::{
    dds::{create_dds, create_mibl},
    dhal::Dhal,
    mibl::Mibl,
    msmd::Msmd,
    msrd::{write_msrd, Msrd},
    mxmd::Mxmd,
    sar1::Sar1,
    spch::Spch,
    vertex::write_vertex_data,
    xbc1::Xbc1,
};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    /// The root folder that contains folders like `map/` and `monolib/`.
    /// Supports Xenoblade 2 and Xenoblade 3.
    root_folder: String,

    /// Process LIBM image files from .witex, .witx, .wismt
    #[arg(long)]
    mibl: bool,

    /// Process DMXM model files from .wimdo
    #[arg(long)]
    mxmd: bool,

    /// Process DRSM model files from .wismt
    #[arg(long)]
    msrd: bool,

    /// Process DMSM map files from .wismhd
    #[arg(long)]
    msmd: bool,

    /// Process 1RAS model files from .chr
    #[arg(long)]
    sar1: bool,

    /// Process HCPS shader files from .wishp
    #[arg(long)]
    spch: bool,

    /// Process LAHD texture files from .wilay
    #[arg(long)]
    dhal: bool,

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

    // Check parsing and conversions for various file types.
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

    if cli.msmd || cli.all {
        println!("Checking MSMD files ...");
        check_all_msmd(root);
    }

    if cli.sar1 || cli.all {
        println!("Checking SAR1 files ...");
        check_all_sar1(root);
    }

    if cli.spch || cli.all {
        println!("Checking SPCH files ...");
        check_all_spch(root);
    }

    if cli.dhal || cli.all {
        println!("Checking DHAL files ...");
        check_all_dhal(root);
    }

    println!("Finished in {:?}", start.elapsed());
}

fn check_all_mxmd<P: AsRef<Path>>(root: P) {
    // TODO: The map folder .wimdo files for XC3 are a different format?
    // TODO: b"APMD" magic in "chr/oj/oj03010100.wimdo"?
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
    // Only XC3 has a dedicated tex directory.
    // TODO: Test joining the medium and low textures?
    let folder = root.as_ref().join("chr").join("tex").join("nx");
    if folder.exists() {
        globwalk::GlobWalkerBuilder::from_patterns(folder, &["*.wismt", "!h/**"])
            .build()
            .unwrap()
            .par_bridge()
            .for_each(|entry| {
                let path = entry.as_ref().unwrap().path();
                let (original_bytes, mibl) = read_wismt_single_tex(path);
                check_mibl(original_bytes, mibl, path);
            });
    }

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
    let folder = root.as_ref();

    // Skip the .wismt textures in the XC3 tex folder.
    // TODO: Some XC2 .wismt files are other formats?
    // model/oj/oj108004.wismt - XBC1 for packed MIBL files
    // model/we/we010601.wismt - packed MIBL files (uncompressed)
    // model/we/we010602.wismt - packed MIBL files (uncompressed)
    globwalk::GlobWalkerBuilder::from_patterns(folder, &["*.wismt", "!**/tex/**"])
        .build()
        .unwrap()
        .par_bridge()
        .for_each(|entry| {
            let path = entry.as_ref().unwrap().path();
            match Msrd::from_file(path) {
                Ok(msrd) => {
                    check_msrd(msrd, path);
                }
                Err(e) => println!("Error reading {path:?}: {e}"),
            }
        });
}

fn check_msrd(msrd: Msrd, path: &Path) {
    msrd.extract_shader_data();
    let vertex_data = msrd.extract_vertex_data();
    msrd.extract_low_texture_data();
    // TODO: High textures?

    let original = std::fs::read(path).unwrap();
    let mut writer = Cursor::new(Vec::new());
    write_msrd(&msrd, &mut writer).unwrap();
    if writer.into_inner() != original {
        println!("Read write not 1:1 for {path:?}");
    }

    let original = msrd.decompress_stream(0, msrd.vertex_data_entry_index);
    let mut writer = Cursor::new(Vec::new());
    write_vertex_data(&vertex_data, &mut writer).unwrap();
    if writer.into_inner() != original {
        println!("VertexData Read write not 1:1 for {path:?}");
    }
}

fn check_all_msmd<P: AsRef<Path>>(root: P) {
    let folder = root.as_ref().join("map");

    globwalk::GlobWalkerBuilder::from_patterns(folder, &["*.wismhd"])
        .build()
        .unwrap()
        .par_bridge()
        .for_each(|entry| {
            let path = entry.as_ref().unwrap().path();
            // TODO: Also check xc3_model loading?
            match Msmd::from_file(path) {
                Ok(msmd) => {
                    check_msmd(msmd, path);
                }
                Err(e) => println!("Error reading {path:?}: {e}"),
            }
        });
}

fn check_msmd(msmd: Msmd, path: &Path) {
    // Parse all the data from the .wismda
    let mut reader = BufReader::new(std::fs::File::open(path.with_extension("wismda")).unwrap());

    let compressed = msmd.wismda_info.compressed_length != msmd.wismda_info.decompressed_length;

    for model in msmd.map_models {
        model.entry.extract(&mut reader, compressed);
    }

    for model in msmd.prop_models {
        model.entry.extract(&mut reader, compressed);
    }

    for model in msmd.env_models {
        model.entry.extract(&mut reader, compressed);
    }

    for entry in msmd.prop_vertex_data {
        entry.extract(&mut reader, compressed);
    }

    for texture in msmd.textures {
        // TODO: Test combining med and high files?
        texture.mid.extract(&mut reader, compressed);
        // texture.high.extract(&mut reader, compressed);
    }

    for model in msmd.foliage_models {
        model.entry.extract(&mut reader, compressed);
    }

    for entry in msmd.prop_positions {
        entry.extract(&mut reader, compressed);
    }

    for entry in msmd.low_textures {
        entry.extract(&mut reader, compressed);
    }

    for model in msmd.low_models {
        model.entry.extract(&mut reader, compressed);
    }

    for entry in msmd.unk_foliage_data {
        entry.extract(&mut reader, compressed);
    }

    for entry in msmd.map_vertex_data {
        entry.extract(&mut reader, compressed);
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
    let folder = root.as_ref();
    globwalk::GlobWalkerBuilder::from_patterns(folder, &["*.chr", "*.mot"])
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

fn check_all_spch<P: AsRef<Path>>(root: P) {
    let folder = root.as_ref();
    globwalk::GlobWalkerBuilder::from_patterns(folder, &["*.wishp"])
        .build()
        .unwrap()
        .par_bridge()
        .for_each(|entry| {
            // TODO: How to validate this file?
            let path = entry.as_ref().unwrap().path();
            match Spch::from_file(path) {
                Ok(_) => (),
                Err(e) => println!("Error reading {path:?}: {e}"),
            }
        });
}

fn check_all_dhal<P: AsRef<Path>>(root: P) {
    let folder = root.as_ref();
    globwalk::GlobWalkerBuilder::from_patterns(folder, &["*.wilay"])
        .build()
        .unwrap()
        .par_bridge()
        .for_each(|entry| {
            // TODO: How to validate this file?
            let path = entry.as_ref().unwrap().path();
            match Dhal::from_file(path) {
                Ok(dhal) => {
                    if let Some(textures) = dhal.textures {
                        for texture in textures.textures {
                            Mibl::read(&mut Cursor::new(&texture.mibl_data)).unwrap();
                        }
                    }
                }
                Err(e) => println!("Error reading {path:?}: {e}"),
            }
        });
}
