use std::{
    io::{BufReader, Cursor},
    path::Path,
};

use binrw::{BinRead, BinReaderExt};
use clap::Parser;
use rayon::prelude::*;
use xc3_lib::{
    bc::Bc,
    dhal::Dhal,
    eva::Eva,
    ltpc::Ltpc,
    mibl::Mibl,
    msmd::Msmd,
    msrd::Msrd,
    mxmd::Mxmd,
    sar1::{ChCl, Csvb, Sar1},
    spch::Spch,
    xbc1::Xbc1,
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

    /// Check that read/write is 1:1 for all files and embedded files.
    #[arg(long)]
    rw: bool,
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
        check_all_mibl(root, cli.rw);
    }

    if cli.mxmd || cli.all {
        // TODO: The map folder .wimdo files for XC3 are a different format?
        // TODO: b"APMD" magic in "chr/oj/oj03010100.wimdo"?
        println!("Checking MXMD files ...");
        check_all(root, &["*.wimdo", "!map/**"], check_mxmd, cli.rw);
    }

    // TODO: Check apmd separately by checking the initial magic?

    if cli.msrd || cli.all {
        // Skip the .wismt textures in the XC3 tex folder.
        // TODO: Some XC2 .wismt files are other formats?
        // model/oj/oj108004.wismt - XBC1 for packed MIBL files
        // model/we/we010601.wismt - packed MIBL files (uncompressed)
        // model/we/we010602.wismt - packed MIBL files (uncompressed)
        println!("Checking MSRD files ...");
        check_all(root, &["*.wismt", "!**/tex/**"], check_msrd, cli.rw);
    }

    if cli.msmd || cli.all {
        println!("Checking MSMD files ...");
        check_all(root, &["*.wismhd"], check_msmd, cli.rw);
    }

    if cli.sar1 || cli.all {
        println!("Checking SAR1 files ...");
        check_all(root, &["*.arc", "*.chr", "*.mot"], check_sar1_data, cli.rw);
    }

    if cli.spch || cli.all {
        println!("Checking SPCH files ...");
        check_all(root, &["*.wishp"], check_spch, cli.rw);
    }

    if cli.dhal || cli.all {
        println!("Checking DHAL files ...");
        check_all(root, &["*.wilay"], check_dhal_data, cli.rw);
    }

    if cli.ltpc || cli.all {
        println!("Checking LTPC files ...");
        check_all(root, &["*.wiltp"], check_ltpc, cli.rw);
    }

    if cli.bc || cli.all {
        println!("Checking BC files ...");
        check_all(root, &["*.anm", "*.motstm_data"], check_bc, cli.rw);
    }

    if cli.eva || cli.all {
        println!("Checking EVA files ...");
        check_all(root, &["*.eva"], check_eva, cli.rw);
    }

    println!("Finished in {:?}", start.elapsed());
}

fn check_all_mibl<P: AsRef<Path>>(root: P, check_read_write: bool) {
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
                check_mibl(mibl, path, &original_bytes, check_read_write);
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
            check_mibl(mibl, path, &original_bytes, check_read_write);
        });
}

#[derive(BinRead)]
enum MaybeXbc1<T>
where
    for<'a> T: BinRead<Args<'a> = ()>,
{
    Uncompressed(T),
    Xbc1(Xbc1),
}

fn check_maybe_xbc1_data<T, F>(
    data: MaybeXbc1<T>,
    path: &Path,
    check_read_write: bool,
    original_bytes: &[u8],
    check_file: F,
) where
    for<'a> T: BinRead<Args<'a> = ()>,
    F: Fn(T, &Path, &[u8], bool),
{
    // TODO: Still check read/write for xbc1 data?
    match data {
        MaybeXbc1::Uncompressed(data) => check_file(data, path, original_bytes, check_read_write),
        MaybeXbc1::Xbc1(xbc1) => match xbc1.extract() {
            Ok(data) => check_file(data, path, &xbc1.decompress().unwrap(), check_read_write),
            Err(e) => println!("Error extracting from {path:?}: {e}"),
        },
    }
}

fn check_sar1_data(
    data: MaybeXbc1<Sar1>,
    path: &Path,
    original_bytes: &[u8],
    check_read_write: bool,
) {
    check_maybe_xbc1_data(data, path, check_read_write, original_bytes, check_sar1);
}

fn check_msrd(msrd: Msrd, path: &Path, original_bytes: &[u8], check_read_write: bool) {
    msrd.extract_shader_data();
    let vertex_data = msrd.extract_vertex_data().unwrap();
    msrd.extract_low_texture_data();

    // TODO: Check mibl?
    match msrd.extract_middle_textures() {
        Ok(_textures) => (),
        Err(e) => println!("Error extracting middle textures {path:?}: {e}"),
    }
    // TODO: High textures?

    if check_read_write {
        let mut writer = Cursor::new(Vec::new());
        msrd.write(&mut writer).unwrap();
        if writer.into_inner() != original_bytes {
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

fn check_msmd(msmd: Msmd, path: &Path, _original_bytes: &[u8], _check_read_write: bool) {
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

fn check_mibl(mibl: Mibl, path: &Path, original_bytes: &[u8], check_read_write: bool) {
    // DDS should support all MIBL image formats.
    // MIBL <-> DDS should be 1:1.
    let dds = mibl.to_dds().unwrap();
    let new_mibl = Mibl::from_dds(&dds).unwrap();
    if mibl != new_mibl {
        println!("Mibl/DDS conversion not 1:1 for {path:?}");
    }

    if check_read_write {
        let mut writer = Cursor::new(Vec::new());
        mibl.write(&mut writer).unwrap();

        if original_bytes != writer.into_inner() {
            println!("Mibl read/write not 1:1 for {path:?}");
        };
    }
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

fn check_dhal_data(
    data: MaybeXbc1<Dhal>,
    path: &Path,
    original_bytes: &[u8],
    check_read_write: bool,
) {
    check_maybe_xbc1_data(data, path, check_read_write, original_bytes, check_dhal);
}

fn check_dhal(dhal: Dhal, path: &Path, original_bytes: &[u8], check_read_write: bool) {
    if check_read_write {
        if let Some(textures) = &dhal.textures {
            for texture in &textures.textures {
                let mibl = Mibl::from_bytes(&texture.mibl_data).unwrap();
                check_mibl(mibl, path, &texture.mibl_data, check_read_write);
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
        let mut writer = Cursor::new(Vec::new());
        dhal.write(&mut writer).unwrap();
        if writer.into_inner() != original_bytes {
            println!("Dhal read/write not 1:1 for {path:?}");
        }
    }
}

fn check_mxmd(mxmd: Mxmd, path: &Path, original_bytes: &[u8], check_read_write: bool) {
    if !is_valid_models_flags(&mxmd) {
        println!("Inconsistent ModelsFlags for {path:?}");
    }

    if check_read_write {
        // Check read/write.
        let mut writer = Cursor::new(Vec::new());
        mxmd.write(&mut writer).unwrap();
        if writer.into_inner() != original_bytes {
            println!("Mxmd read/write not 1:1 for {path:?}");
        }
    }

    if let Some(spch) = mxmd.spch {
        // TODO: Check read/write for inner data?
        check_spch(spch, path, &[], false);
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

fn check_spch(spch: Spch, path: &Path, original_bytes: &[u8], check_read_write: bool) {
    // TODO: Check reading other sections.
    for (i, program) in spch.shader_programs.iter().enumerate() {
        if let Err(e) = program.read_slct(&spch.slct_section) {
            println!("Error reading Slct {i} for {path:?}: {e}");
        }
    }

    if check_read_write {
        let mut writer = Cursor::new(Vec::new());
        spch.write(&mut writer).unwrap();
        if writer.into_inner() != original_bytes {
            println!("Spch read/write not 1:1 for {path:?}");
        }
    }
}

fn check_ltpc(ltpc: Ltpc, path: &Path, original_bytes: &[u8], check_read_write: bool) {
    if check_read_write {
        // Check read/write.
        let mut writer = Cursor::new(Vec::new());
        ltpc.write(&mut writer).unwrap();
        if writer.into_inner() != original_bytes {
            println!("Ltpc read/write not 1:1 for {path:?}");
        }
    }
}

// Use an enum for better error reporting.
#[derive(BinRead)]
enum Sar1EntryData {
    Bc(Bc),
    ChCl(ChCl),
    Csvb(Csvb),
    Eva(Eva),
}

fn check_sar1(sar1: Sar1, path: &Path, original_bytes: &[u8], check_read_write: bool) {
    for entry in &sar1.entries {
        if xc3_lib::hash::hash_str_crc(&entry.name) != entry.name_hash {
            println!("Incorrect hash for {:?}", entry.name);
        }

        // Check read/write for the inner data.
        let mut reader = Cursor::new(&entry.entry_data);
        match reader.read_le() {
            Ok(data) => match data {
                Sar1EntryData::Bc(bc) => {
                    if check_read_write {
                        let mut writer = Cursor::new(Vec::new());
                        xc3_write::write_full(&bc, &mut writer, 0, &mut 0).unwrap();
                        if writer.into_inner() != entry.entry_data {
                            println!("Bc read/write not 1:1 for {:?} in {path:?}", entry.name);
                        }
                    }
                }
                Sar1EntryData::ChCl(chcl) => {
                    if check_read_write {
                        let mut writer = Cursor::new(Vec::new());
                        xc3_write::write_full(&chcl, &mut writer, 0, &mut 0).unwrap();
                        if writer.into_inner() != entry.entry_data {
                            println!("ChCl read/write not 1:1 for {:?} in {path:?}", entry.name);
                        }
                    }
                }
                Sar1EntryData::Csvb(csvb) => {
                    if check_read_write {
                        let mut writer = Cursor::new(Vec::new());
                        xc3_write::write_full(&csvb, &mut writer, 0, &mut 0).unwrap();
                        if writer.into_inner() != entry.entry_data {
                            println!("Csvb read/write not 1:1 for {:?} in {path:?}", entry.name);
                        }
                    }
                }
                Sar1EntryData::Eva(eva) => {
                    if check_read_write {
                        let mut writer = Cursor::new(Vec::new());
                        xc3_write::write_full(&eva, &mut writer, 0, &mut 0).unwrap();
                        if writer.into_inner() != entry.entry_data {
                            println!("Eva read/write not 1:1 for {:?} in {path:?}", entry.name);
                        }
                    }
                }
            },
            Err(e) => println!("Error reading {:?} in {path:?}: {e}", entry.name,),
        }
    }

    if check_read_write {
        // Check read/write for the archive.
        let mut writer = Cursor::new(Vec::new());
        sar1.write(&mut writer).unwrap();
        if writer.into_inner() != original_bytes {
            println!("Sar1 read/write not 1:1 for {path:?}");
        };
    }
}

fn check_bc(bc: Bc, path: &Path, original_bytes: &[u8], check_read_write: bool) {
    if check_read_write {
        // Check read/write.
        let mut writer = Cursor::new(Vec::new());
        bc.write(&mut writer).unwrap();
        if writer.into_inner() != original_bytes {
            println!("Bc read/write not 1:1 for {path:?}");
        }
    }
}

fn check_eva(eva: Eva, path: &Path, original_bytes: &[u8], check_read_write: bool) {
    if check_read_write {
        // Check read/write.
        let mut writer = Cursor::new(Vec::new());
        eva.write(&mut writer).unwrap();
        if writer.into_inner() != original_bytes {
            println!("Eva read/write not 1:1 for {path:?}");
        }
    }
}

fn check_all<P, T, F>(root: P, patterns: &[&str], check_file: F, check_read_write: bool)
where
    P: AsRef<Path>,
    F: Fn(T, &Path, &[u8], bool) + Sync,
    for<'a> T: BinRead<Args<'a> = ()>,
{
    globwalk::GlobWalkerBuilder::from_patterns(root, patterns)
        .build()
        .unwrap()
        .par_bridge()
        .for_each(|entry| {
            let path = entry.as_ref().unwrap().path();
            let original_bytes = std::fs::read(path).unwrap();
            let mut reader = Cursor::new(&original_bytes);
            match reader.read_le() {
                Ok(file) => check_file(file, path, &original_bytes, check_read_write),
                Err(e) => println!("Error reading {path:?}: {e}"),
            }
        });
}
