use std::{
    error::Error,
    io::{BufReader, Cursor},
    path::Path,
};

use clap::Parser;
use rayon::prelude::*;
use xc3_lib::{
    bc::Bc, dhal::Dhal, eva::Eva, ltpc::Ltpc, mibl::Mibl, msmd::Msmd, msrd::Msrd, mxmd::Mxmd,
    sar1::Sar1, spch::Spch, xbc1::Xbc1,
};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    /// The root folder that contains folders like `map/` and `monolib/`.
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

    /// Process BC files from .anm and .motstm_data
    #[arg(long)]
    bc: bool,

    /// Process EVA files from .eva
    #[arg(long)]
    eva: bool,

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

    // TODO: Check apmd separately by checking the initial magic?

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
        check_all(root, &["*.arc", "*.chr", "*.mot"], check_sar1);
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

    if cli.bc || cli.all {
        println!("Checking BC files ...");
        check_all(root, &["*.anm", "*.motstm_data"], check_bc);
    }

    if cli.eva || cli.all {
        println!("Checking EVA files ...");
        check_all(root, &["*.eva"], check_eva);
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
                check_mibl(&original_bytes, mibl, path);
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
            check_mibl(&original_bytes, mibl, path);
        });
}

fn check_msrd(msrd: Msrd, path: &Path, check_read_write: bool) {
    msrd.extract_shader_data();
    let vertex_data = msrd.extract_vertex_data();
    msrd.extract_low_texture_data();

    // TODO: High textures?
    // TODO: Check mibl?

    if check_read_write {
        let original = std::fs::read(path).unwrap();
        let mut writer = Cursor::new(Vec::new());
        msrd.write(&mut writer).unwrap();
        if writer.into_inner() != original {
            println!("Msrd read/write not 1:1 for {path:?}");
        }
    }

    // Check read/write for embedded data.
    let original = msrd.decompress_stream(0, msrd.vertex_data_entry_index);
    let mut writer = Cursor::new(Vec::new());
    vertex_data.write(&mut writer).unwrap();
    if writer.into_inner() != original {
        println!("VertexData read/write not 1:1 for {path:?}");
    }
}

fn check_msmd(msmd: Msmd, path: &Path, check_read_write: bool) {
    // Parse all the data from the .wismda
    if check_read_write {
        let mut reader =
            BufReader::new(std::fs::File::open(path.with_extension("wismda")).unwrap());

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
}

fn check_mibl(original_bytes: &[u8], mibl: Mibl, path: &Path) {
    let dds = mibl.to_dds().unwrap();
    let new_mibl = Mibl::from_dds(&dds).unwrap();

    let mut writer = Cursor::new(Vec::new());
    new_mibl.write(&mut writer).unwrap();

    // DDS should support all MIBL image formats.
    // Check that read -> MIBL -> DDS -> MIBL -> write is 1:1.
    if original_bytes != writer.into_inner() {
        println!("Mibl read/write not 1:1 for {path:?}");
    };
}

fn read_wismt_single_tex(path: &Path) -> (Vec<u8>, Mibl) {
    let xbc1 = Xbc1::from_file(path).unwrap();

    let decompressed = xbc1.decompress().unwrap();

    if xc3_lib::hash::hash_crc(&decompressed) != xbc1.decompressed_hash {
        println!("Incorrect xbc1 hash for {path:?}");
    }

    // TODO: Test merging.
    let mibl_m = Mibl::from_bytes(&decompressed).unwrap();
    (decompressed, mibl_m)
}

fn check_dhal(dhal: Dhal, path: &Path, check_read_write: bool) {
    if check_read_write {
        if let Some(textures) = &dhal.textures {
            for texture in &textures.textures {
                let mibl = Mibl::from_bytes(&texture.mibl_data).unwrap();
                check_mibl(&texture.mibl_data, mibl, path);
            }
        }

        if let Some(textures) = &dhal.uncompressed_textures {
            for texture in &textures.textures {
                // Check for valid JFIF/JPEG data.
                if let Err(e) = texture.to_image() {
                    println!("Error decoding JPEG for {path:?}: {e}");
                }
            }
        }

        // Check read/write.
        let original = std::fs::read(path).unwrap();
        let mut writer = Cursor::new(Vec::new());
        dhal.write(&mut writer).unwrap();
        if writer.into_inner() != original {
            println!("Dhal read/write not 1:1 for {path:?}");
        }
    }
}

fn check_mxmd(mxmd: Mxmd, path: &Path, check_read_write: bool) {
    if !is_valid_models_flags(&mxmd) {
        println!("Inconsistent ModelsFlags for {path:?}");
    }

    if check_read_write {
        // Check read/write.
        let original = std::fs::read(path).unwrap();
        let mut writer = Cursor::new(Vec::new());
        mxmd.write(&mut writer).unwrap();
        if writer.into_inner() != original {
            println!("Mxmd read/write not 1:1 for {path:?}");
        }
    }

    if let Some(spch) = mxmd.spch {
        // TODO: Check read/write for inner data?
        check_spch(spch, path, false);
    }

    if let Some(packed_textures) = &mxmd.packed_textures {
        for texture in &packed_textures.textures {
            if let Err(e) = Mibl::from_bytes(&texture.mibl_data) {
                println!("Error reading Mibl for {path:?}: {e}");
            }
        }
    }
}

fn is_valid_models_flags(mxmd: &Mxmd) -> bool {
    // Check that flags are consistent with nullability of offsets.
    let flags = mxmd.models.models_flags;
    flags.has_model_unk8() == mxmd.models.model_unk8.is_some()
        && flags.has_model_unk7() == mxmd.models.model_unk7.is_some()
        && flags.has_morph_controllers() == mxmd.models.morph_controllers.is_some()
        && flags.has_model_unk1() == mxmd.models.model_unk1.is_some()
        && flags.has_skinning() == mxmd.models.skinning.is_some()
        && flags.has_lod_data() == mxmd.models.lod_data.is_some()
        && flags.has_model_unk4() == mxmd.models.model_unk4.is_some()
}

fn check_spch(spch: Spch, path: &Path, check_read_write: bool) {
    // TODO: Check reading other sections.
    for (i, program) in spch.shader_programs.iter().enumerate() {
        if let Err(e) = program.read_slct(&spch.slct_section) {
            println!("Error reading Slct {i} for {path:?}: {e}");
        }
    }

    if check_read_write {
        let original = std::fs::read(path).unwrap();
        let mut writer = Cursor::new(Vec::new());
        spch.write(&mut writer).unwrap();
        if writer.into_inner() != original {
            println!("Spch read/write not 1:1 for {path:?}");
        }
    }
}

fn check_ltpc(ltpc: Ltpc, path: &Path, check_read_write: bool) {
    if check_read_write {
        // Check read/write.
        let original = std::fs::read(path).unwrap();
        let mut writer = Cursor::new(Vec::new());
        ltpc.write(&mut writer).unwrap();
        if writer.into_inner() != original {
            println!("Ltpc read/write not 1:1 for {path:?}");
        }
    }
}

fn check_sar1(sar1: Sar1, path: &Path, check_read_write: bool) {
    for entry in &sar1.entries {
        if xc3_lib::hash::hash_str_crc(&entry.name) != entry.name_hash {
            println!("Incorrect hash for {:?}", entry.name);
        }

        // Check read/write for the inner data.
        if let Ok(bc) = entry.read_data::<xc3_lib::bc::Bc>() {
            let mut writer = Cursor::new(Vec::new());
            xc3_write::write_full(&bc, &mut writer, 0, &mut 0).unwrap();
            if writer.into_inner() != entry.entry_data {
                println!("Bc read/write not 1:1 for {:?} in {path:?}", entry.name);
            }
        } else if let Ok(chcl) = entry.read_data::<xc3_lib::sar1::ChCl>() {
            let mut writer = Cursor::new(Vec::new());
            xc3_write::write_full(&chcl, &mut writer, 0, &mut 0).unwrap();
            if writer.into_inner() != entry.entry_data {
                println!("ChCl read/write not 1:1 for {:?} in {path:?}", entry.name);
            }
        } else if let Ok(csvb) = entry.read_data::<xc3_lib::sar1::Csvb>() {
            let mut writer = Cursor::new(Vec::new());
            xc3_write::write_full(&csvb, &mut writer, 0, &mut 0).unwrap();
            if writer.into_inner() != entry.entry_data {
                println!("Csvb read/write not 1:1 for {:?} in {path:?}", entry.name);
            }
        } else if let Ok(eva) = entry.read_data::<xc3_lib::eva::Eva>() {
            let mut writer = Cursor::new(Vec::new());
            xc3_write::write_full(&eva, &mut writer, 0, &mut 0).unwrap();
            if writer.into_inner() != entry.entry_data {
                println!("Eva read/write not 1:1 for {:?} in {path:?}", entry.name);
            }
        } else {
            println!("Error reading {:?} in {path:?}", entry.name);
        }
    }

    if check_read_write {
        // Check read/write for the archive.
        let original = std::fs::read(path).unwrap();
        let mut writer = Cursor::new(Vec::new());
        sar1.write(&mut writer).unwrap();
        if writer.into_inner() != original {
            println!("Sar1 read/write not 1:1 for {path:?}");
        };
    }
}

fn check_bc(bc: Bc, path: &Path, check_read_write: bool) {
    if check_read_write {
        // Check read/write.
        let original = std::fs::read(path).unwrap();
        let mut writer = Cursor::new(Vec::new());
        bc.write(&mut writer).unwrap();
        if writer.into_inner() != original {
            println!("Bc read/write not 1:1 for {path:?}");
        }
    }
}

fn check_eva(eva: Eva, path: &Path, check_read_write: bool) {
    if check_read_write {
        // Check read/write.
        let original = std::fs::read(path).unwrap();
        let mut writer = Cursor::new(Vec::new());
        eva.write(&mut writer).unwrap();
        if writer.into_inner() != original {
            println!("Eva read/write not 1:1 for {path:?}");
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
file_impl!(Mxmd, Msrd, Msmd, Spch, Dhal, Sar1, Ltpc, Bc, Eva);

fn check_all<P, T, F>(root: P, patterns: &[&str], check_file: F)
where
    P: AsRef<Path>,
    T: Xc3File,
    F: Fn(T, &Path, bool) + Sync,
{
    globwalk::GlobWalkerBuilder::from_patterns(root, patterns)
        .build()
        .unwrap()
        .par_bridge()
        .for_each(|entry| {
            let path = entry.as_ref().unwrap().path();
            match T::from_file(path) {
                Ok(file) => check_file(file, path, true),
                Err(e) => println!("Error reading {path:?}: {e}"),
            }
        });
}
