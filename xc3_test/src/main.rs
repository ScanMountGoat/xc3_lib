use std::{
    error::Error,
    io::{BufReader, Cursor},
    path::Path,
};

use clap::Parser;
use rayon::prelude::*;
use xc3_lib::{
    dds::{create_dds, create_mibl},
    dhal::Dhal,
    ltpc::{write_ltpc, Ltpc},
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

    /// Process LTPC texture files from .wiltp
    #[arg(long)]
    ltpc: bool,

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
        // TODO: The map folder .wimdo files for XC3 are a different format?
        // TODO: b"APMD" magic in "chr/oj/oj03010100.wimdo"?
        println!("Checking MXMD files ...");
        check_all(root, &["*.wimdo", "!map/**"], check_mxmd);
    }

    if cli.msrd || cli.all {
        // Skip the .wismt textures in the XC3 tex folder.
        // TODO: Some XC2 .wismt files are other formats?
        // model/oj/oj108004.wismt - XBC1 for packed MIBL files
        // model/we/we010601.wismt - packed MIBL files (uncompressed)
        // model/we/we010602.wismt - packed MIBL files (uncompressed)
        println!("Checking MSRD files ...");
        check_all(root, &["*.wismt", "!**/tex/**"], check_msrd);
    }

    if cli.msmd || cli.all {
        println!("Checking MSMD files ...");
        check_all(root, &["*.wismhd"], check_msmd);
    }

    if cli.sar1 || cli.all {
        println!("Checking SAR1 files ...");
        check_all(root, &["*.chr", "*.mot"], check_sar1);
    }

    if cli.spch || cli.all {
        println!("Checking SPCH files ...");
        check_all(root, &["*.wishp"], check_spch);
    }

    if cli.dhal || cli.all {
        println!("Checking DHAL files ...");
        check_all(root, &["*.wilay"], check_dhal);
    }

    if cli.ltpc || cli.all {
        println!("Checking LTPC files ...");
        check_all(root, &["*.wiltp"], check_ltpc);
    }

    println!("Finished in {:?}", start.elapsed());
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

fn check_msrd(msrd: Msrd, path: &Path) {
    msrd.extract_shader_data();
    let vertex_data = msrd.extract_vertex_data();
    msrd.extract_low_texture_data();
    // TODO: High textures?
    // TODO: Check mibl?

    // Check read/write for embedded data.
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

    // TODO: Test mibl read/write?
    for model in msmd.env_models {
        let entry = model.entry.extract(&mut reader, compressed);
        for texture in entry.textures.textures {
            Mibl::from_bytes(&texture.mibl_data).unwrap();
        }
    }

    for entry in msmd.prop_vertex_data {
        entry.extract(&mut reader, compressed);
    }

    for texture in msmd.textures {
        // TODO: Test combining mid and high files?
        texture.mid.extract(&mut reader, compressed);
        // texture.high.extract(&mut reader, compressed);
    }

    for model in msmd.foliage_models {
        let entry = model.entry.extract(&mut reader, compressed);
        for texture in entry.textures.textures {
            Mibl::from_bytes(&texture.mibl_data).unwrap();
        }
    }

    for entry in msmd.prop_positions {
        entry.extract(&mut reader, compressed);
    }

    for entry in msmd.low_textures {
        let entry = entry.extract(&mut reader, compressed);
        for texture in entry.textures {
            Mibl::from_bytes(&texture.mibl_data).unwrap();
        }
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
    let mibl = Mibl::from_bytes(&decompressed).unwrap();
    (decompressed, mibl)
}

fn check_dhal(dhal: Dhal, _path: &Path) {
    if let Some(textures) = dhal.textures {
        for texture in textures.textures {
            Mibl::from_bytes(&texture.mibl_data).unwrap();
        }
    }
}

fn check_mxmd(mxmd: Mxmd, path: &Path) {
    if let Some(spch) = mxmd.spch {
        check_spch(spch, path);
    }

    if let Some(packed_textures) = mxmd.packed_textures {
        for texture in packed_textures.textures {
            if let Err(e) = Mibl::from_bytes(&texture.mibl_data) {
                println!("Error reading Mibl for {path:?}: {e}");
            }
        }
    }
}

fn check_spch(spch: Spch, _path: &Path) {
    for program in spch.shader_programs {
        program.read_slct(&spch.slct_section);
    }
}

fn check_ltpc(ltpc: Ltpc, path: &Path) {
    // Check read/write.
    let original = std::fs::read(path).unwrap();
    let mut writer = Cursor::new(Vec::new());
    write_ltpc(&ltpc, &mut writer).unwrap();
    if writer.into_inner() != original {
        println!("Read write not 1:1 for {path:?}");
    }
}

fn check_sar1(sar1: Sar1, path: &Path) {
    for entry in sar1.entries {
        if let Err(e) = entry.read_data() {
            println!("Error reading entry for {path:?}: {e}");
        }
    }
}

trait Xc3File
where
    Self: Sized,
{
    fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn Error>>;
}

macro_rules! file_impl {
    ($($type_name:path),*) => {
        $(
            impl Xc3File for $type_name {
                fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn Error>> {
                    Self::from_file(path)
                }
            }
        )*
    };
}
file_impl!(Mxmd, Msrd, Msmd, Spch, Dhal, Sar1, Ltpc);

fn check_all<P, T, F>(root: P, patterns: &[&str], check_file: F)
where
    P: AsRef<Path>,
    T: Xc3File,
    F: Fn(T, &Path) + Sync,
{
    globwalk::GlobWalkerBuilder::from_patterns(root, patterns)
        .build()
        .unwrap()
        .par_bridge()
        .for_each(|entry| {
            let path = entry.as_ref().unwrap().path();
            match T::from_file(path) {
                Ok(file) => check_file(file, path),
                Err(e) => println!("Error reading {path:?}: {e}"),
            }
        });
}
