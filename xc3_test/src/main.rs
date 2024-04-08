use std::{
    io::{BufReader, Cursor},
    path::Path,
};

use binrw::{BinRead, BinReaderExt, Endian};
use clap::Parser;
use rayon::prelude::*;
use xc3_lib::{
    apmd::Apmd,
    bc::Bc,
    bmn::Bmn,
    dhal::Dhal,
    eva::Eva,
    lagp::Lagp,
    laps::Laps,
    ltpc::Ltpc,
    mibl::Mibl,
    msmd::Msmd,
    msrd::{streaming::chr_tex_nx_folder, Msrd},
    mtxt::Mtxt,
    mxmd::{legacy::MxmdLegacy, Mxmd},
    sar1::{ChCl, Csvb, Sar1},
    spch::Spch,
    xbc1::{MaybeXbc1, Xbc1},
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

    /// Process DMXM or DMPA model files from .wimdo
    #[arg(long)]
    wimdo: bool,

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
    wilay: bool,

    /// Process LTPC texture files from .wiltp
    #[arg(long)]
    ltpc: bool,

    /// Process BC files from .anm and .motstm_data
    #[arg(long)]
    bc: bool,

    /// Process EVA files from .eva
    #[arg(long)]
    eva: bool,

    /// Process MTXT files from .catex, .calut, and .caavp
    #[arg(long)]
    mtxt: bool,

    /// Process MXMD files from .camdo
    #[arg(long)]
    camdo: bool,

    /// Process BMN files from .bmn
    #[arg(long)]
    bmn: bool,

    /// Process all file types except gltf and wimdo-model.
    #[arg(long)]
    all: bool,

    /// Convert wimdo and wismhd to gltf without saving.
    #[arg(long)]
    gltf: bool,

    /// Convert wimdo models to and from xc3_model types.
    #[arg(long)]
    wimdo_model: bool,

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
        println!("Checking Mibl files ...");
        check_all_mibl(root, cli.rw);
    }

    if cli.wimdo || cli.all {
        println!("Checking Mxmd and Apmd files ...");
        check_all(
            root,
            &["*.wimdo", "*.pcmdo"],
            check_wimdo,
            Endian::Little,
            cli.rw,
        );
    }

    if cli.msrd || cli.all {
        // Skip the .wismt textures in the XC3 tex folder.
        println!("Checking Msrd files ...");
        check_all(
            root,
            &["*.wismt", "!**/tex/**"],
            check_msrd,
            Endian::Little,
            cli.rw,
        );
    }

    if cli.msmd || cli.all {
        println!("Checking Msmd files ...");
        check_all(root, &["*.wismhd"], check_msmd, Endian::Little, cli.rw);
    }

    if cli.sar1 || cli.all {
        println!("Checking Sar1 files ...");
        check_all(
            root,
            &["*.arc", "*.chr", "*.mot"],
            check_sar1_data,
            Endian::Little,
            cli.rw,
        );
    }

    if cli.spch || cli.all {
        println!("Checking Spch files ...");
        check_all(root, &["*.wishp"], check_spch, Endian::Little, cli.rw);
    }

    if cli.wilay || cli.all {
        println!("Checking Dhal and Lagp files ...");
        check_all(root, &["*.wilay"], check_wilay_data, Endian::Little, cli.rw);
    }

    if cli.ltpc || cli.all {
        println!("Checking Ltpc files ...");
        check_all(root, &["*.wiltp"], check_ltpc, Endian::Little, cli.rw);
    }

    if cli.bc || cli.all {
        println!("Checking Bc files ...");
        check_all(
            root,
            &["*.anm", "*.motstm_data"],
            check_bc,
            Endian::Little,
            cli.rw,
        );
    }

    if cli.eva || cli.all {
        println!("Checking Eva files ...");
        check_all(root, &["*.eva"], check_eva, Endian::Little, cli.rw);
    }

    if cli.mtxt || cli.all {
        println!("Checking Mtxt files ...");
        check_all(
            root,
            &["*.catex", "*.calut", "*.caavp"],
            check_mtxt,
            Endian::Big,
            cli.rw,
        );
    }

    if cli.camdo || cli.all {
        println!("Checking Mxmd files ...");
        check_all(root, &["*.camdo"], check_mxmd_legacy, Endian::Big, cli.rw);
    }

    if cli.bmn || cli.all {
        println!("Checking Bmn files ...");
        check_all(root, &["*.bmn"], check_bmn, Endian::Big, cli.rw);
    }

    if cli.gltf {
        check_all_gltf(root);
    }

    if cli.wimdo_model {
        check_all_wimdo_model(root, cli.rw);
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

fn check_maybe_xbc1<T, F>(
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
    check_maybe_xbc1(data, path, check_read_write, original_bytes, check_sar1);
}

fn check_msrd(msrd: Msrd, path: &Path, original_bytes: &[u8], check_read_write: bool) {
    // TODO: check stream flags?
    let chr_tex_nx = chr_tex_nx_folder(path);
    let (vertex, spch, textures) = msrd.extract_files(chr_tex_nx.as_deref()).unwrap();

    if check_read_write {
        let mut writer = Cursor::new(Vec::new());
        msrd.write(&mut writer).unwrap();
        if writer.into_inner() != original_bytes {
            println!("Msrd read/write not 1:1 for {path:?}");
        }
    }

    match &msrd.streaming.inner {
        xc3_lib::msrd::StreamingInner::StreamingLegacy(_) => todo!(),
        xc3_lib::msrd::StreamingInner::Streaming(data) => {
            // Check embedded data.
            let vertex_bytes = msrd
                .decompress_stream_entry(0, data.vertex_data_entry_index)
                .unwrap();
            check_vertex_data(vertex, path, &vertex_bytes, check_read_write);

            let spch_bytes = msrd
                .decompress_stream_entry(0, data.shader_entry_index)
                .unwrap();
            check_spch(spch, path, &spch_bytes, check_read_write);
        }
    }

    for texture in textures {
        check_mibl(texture.low, path, &[], false);
        if let Some(high) = texture.high {
            check_mibl(high.mid, path, &[], false);
        }
    }
}

fn check_vertex_data(
    vertex_data: xc3_lib::vertex::VertexData,
    path: &Path,
    original_bytes: &[u8],
    check_read_write: bool,
) {
    if check_read_write {
        let mut writer = Cursor::new(Vec::new());
        vertex_data.write(&mut writer).unwrap();
        if writer.into_inner() != original_bytes {
            println!("VertexData read/write not 1:1 for {path:?}");
        }
    }
}

fn check_msmd(msmd: Msmd, path: &Path, _original_bytes: &[u8], check_read_write: bool) {
    // Parse all the data from the .wismda
    let mut reader = BufReader::new(std::fs::File::open(path.with_extension("wismda")).unwrap());

    let compressed = msmd.wismda_info.compressed_length != msmd.wismda_info.decompressed_length;

    for (i, model) in msmd.map_models.iter().enumerate() {
        match model.entry.extract(&mut reader, compressed) {
            Ok(_) => (),
            Err(e) => println!("Error extracting map model {i} in {path:?}: {e}"),
        }
    }

    for (i, model) in msmd.prop_models.iter().enumerate() {
        match model.entry.extract(&mut reader, compressed) {
            Ok(_) => (),
            Err(e) => println!("Error extracting prop model {i} in {path:?}: {e}"),
        }
    }

    for (i, model) in msmd.env_models.iter().enumerate() {
        match model.entry.extract(&mut reader, compressed) {
            Ok(model) => {
                for texture in model.textures.textures {
                    let mibl = Mibl::from_bytes(&texture.mibl_data).unwrap();
                    check_mibl(mibl, path, &texture.mibl_data, check_read_write);
                }
            }
            Err(e) => println!("Error extracting env model {i} in {path:?}: {e}"),
        }
    }

    for (i, entry) in msmd.prop_vertex_data.iter().enumerate() {
        match entry.extract(&mut reader, compressed) {
            Ok(vertex_data) => {
                let original_bytes = entry.decompress(&mut reader, compressed).unwrap();
                check_vertex_data(vertex_data, path, &original_bytes, check_read_write);
            }
            Err(e) => println!("Error extracting prop VertexData {i} in {path:?}: {e}"),
        }
    }

    for (i, model) in msmd.foliage_models.iter().enumerate() {
        match model.entry.extract(&mut reader, compressed) {
            Ok(model) => {
                for texture in model.textures.textures {
                    let mibl = Mibl::from_bytes(&texture.mibl_data).unwrap();
                    check_mibl(mibl, path, &texture.mibl_data, check_read_write);
                }
            }
            Err(e) => println!("Error extracting foliage model {i} in {path:?}: {e}"),
        }
    }

    for entry in msmd.prop_positions {
        entry.extract(&mut reader, compressed).unwrap();
    }

    for entry in msmd.low_textures {
        let entry = entry.extract(&mut reader, compressed).unwrap();
        for texture in entry.textures {
            Mibl::from_bytes(&texture.mibl_data).unwrap();
        }
    }

    for (i, model) in msmd.low_models.iter().enumerate() {
        match model.entry.extract(&mut reader, compressed) {
            Ok(_) => (),
            Err(e) => println!("Error extracting low model {i} in {path:?}: {e}"),
        }
    }

    for entry in msmd.unk_foliage_data {
        entry.extract(&mut reader, compressed).unwrap();
    }

    for (i, entry) in msmd.map_vertex_data.iter().enumerate() {
        match entry.extract(&mut reader, compressed) {
            Ok(vertex_data) => {
                let original_bytes = entry.decompress(&mut reader, compressed).unwrap();
                check_vertex_data(vertex_data, path, &original_bytes, check_read_write);
            }
            Err(e) => println!("Error extracting map VertexData {i} in {path:?}: {e}"),
        }
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

#[derive(BinRead)]
enum Wilay {
    Dhal(Dhal),
    Lagp(Lagp),
    Laps(Laps),
}

fn check_wilay_data(
    data: MaybeXbc1<Wilay>,
    path: &Path,
    original_bytes: &[u8],
    check_read_write: bool,
) {
    check_maybe_xbc1(data, path, check_read_write, original_bytes, check_wilay);
}

fn check_wilay(data: Wilay, path: &Path, original_bytes: &[u8], check_read_write: bool) {
    match data {
        Wilay::Dhal(dhal) => check_dhal(dhal, path, original_bytes, check_read_write),
        Wilay::Lagp(lagp) => check_lagp(lagp, path, original_bytes, check_read_write),
        Wilay::Laps(laps) => check_laps(laps, path, original_bytes, check_read_write),
    }
}

fn check_dhal(dhal: Dhal, path: &Path, original_bytes: &[u8], check_read_write: bool) {
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

    if check_read_write {
        let mut writer = Cursor::new(Vec::new());
        dhal.write(&mut writer).unwrap();
        if writer.into_inner() != original_bytes {
            println!("Dhal read/write not 1:1 for {path:?}");
        }
    }
}

fn check_lagp(lagp: Lagp, path: &Path, original_bytes: &[u8], check_read_write: bool) {
    if let Some(textures) = &lagp.textures {
        for texture in &textures.textures {
            let mibl = Mibl::from_bytes(&texture.mibl_data).unwrap();
            check_mibl(mibl, path, &texture.mibl_data, check_read_write);
        }
    }

    if check_read_write {
        let mut writer = Cursor::new(Vec::new());
        lagp.write(&mut writer).unwrap();
        if writer.into_inner() != original_bytes {
            println!("Lagp read/write not 1:1 for {path:?}");
        }
    }
}

fn check_laps(laps: Laps, path: &Path, original_bytes: &[u8], check_read_write: bool) {
    if check_read_write {
        let mut writer = Cursor::new(Vec::new());
        laps.write(&mut writer).unwrap();
        if writer.into_inner() != original_bytes {
            println!("Laps read/write not 1:1 for {path:?}");
        }
    }
}

#[derive(BinRead)]
enum Wimdo {
    Mxmd(Box<Mxmd>),
    Apmd(Apmd),
}

fn check_wimdo(data: Wimdo, path: &Path, original_bytes: &[u8], check_read_write: bool) {
    match data {
        Wimdo::Mxmd(mxmd) => {
            check_mxmd(*mxmd, path, original_bytes, check_read_write);
        }
        Wimdo::Apmd(apmd) => {
            for entry in &apmd.entries {
                // TODO: check inner data.
                match entry.read_data() {
                    Ok(data) => match data {
                        xc3_lib::apmd::EntryData::Mxmd(mxmd) => {
                            check_mxmd(mxmd, path, &entry.entry_data, check_read_write)
                        }
                        xc3_lib::apmd::EntryData::Dmis => (),
                        xc3_lib::apmd::EntryData::Dlgt(_) => (),
                        xc3_lib::apmd::EntryData::Gibl(_) => (),
                        xc3_lib::apmd::EntryData::Nerd(_) => (),
                        xc3_lib::apmd::EntryData::Dlgt2(_) => (),
                    },
                    Err(e) => println!("Error reading entry in {path:?}: {e}"),
                }
            }

            if check_read_write {
                let mut writer = Cursor::new(Vec::new());
                apmd.write(&mut writer).unwrap();
                if writer.into_inner() != original_bytes {
                    println!("Apmd read/write not 1:1 for {path:?}");
                }
            }
        }
    }
}

fn check_mxmd(mxmd: Mxmd, path: &Path, original_bytes: &[u8], check_read_write: bool) {
    if !is_valid_models_flags(&mxmd) {
        println!("Inconsistent ModelsFlags for {path:?}");
    }

    if check_read_write {
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
            match Mibl::from_bytes(&texture.mibl_data) {
                Ok(mibl) => check_mibl(mibl, path, &texture.mibl_data, check_read_write),
                Err(e) => println!("Error reading Mibl in {path:?}: {e}"),
            }
        }
    }
}

fn is_valid_models_flags(mxmd: &Mxmd) -> bool {
    // Check that flags are consistent with nullability of offsets.
    if let Some(flags) = mxmd.models.models_flags {
        flags.has_model_unk8() == mxmd.models.model_unk8.is_some()
            && flags.has_model_unk7() == mxmd.models.model_unk7.is_some()
            && flags.has_morph_controllers() == mxmd.models.morph_controllers.is_some()
            && flags.has_model_unk1() == mxmd.models.model_unk1.is_some()
            && flags.has_skinning() == mxmd.models.skinning.is_some()
            && flags.has_lod_data() == mxmd.models.lod_data.is_some()
            && flags.has_model_unk4() == mxmd.models.model_unk4.is_some()
    } else {
        true
    }
}

fn check_spch(spch: Spch, path: &Path, original_bytes: &[u8], check_read_write: bool) {
    for (i, slct) in spch.slct_offsets.iter().enumerate() {
        match slct.read_slct(&spch.slct_section) {
            Ok(slct) => {
                for (p, program) in slct.programs.iter().enumerate() {
                    // TODO: Check that the extracted binary sizes add up to the total size.
                    if let Err(e) = program.read_nvsd() {
                        println!("Error reading Slct {i} and Nvsd {p} for {path:?}: {e}");
                    }
                }
            }
            Err(e) => println!("Error reading Slct {i} for {path:?}: {e}"),
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
    Bc(Box<Bc>),
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
                    check_bc(*bc, path, &entry.entry_data, check_read_write);
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
        let mut writer = Cursor::new(Vec::new());
        sar1.write(&mut writer).unwrap();
        if writer.into_inner() != original_bytes {
            println!("Sar1 read/write not 1:1 for {path:?}");
        };
    }
}

fn check_bc(bc: Bc, path: &Path, original_bytes: &[u8], check_read_write: bool) {
    if check_read_write {
        let mut writer = Cursor::new(Vec::new());
        bc.write(&mut writer).unwrap();
        if writer.into_inner() != original_bytes {
            println!("Bc read/write not 1:1 for {path:?}");
        }
    }

    match bc.data {
        xc3_lib::bc::BcData::Skdy(_) => (),
        xc3_lib::bc::BcData::Anim(_) => (),
        xc3_lib::bc::BcData::Skel(_) => (),
        xc3_lib::bc::BcData::Asmb(asmb) => match asmb.inner {
            xc3_lib::bc::asmb::AsmbInner::V1(_) => (),
            xc3_lib::bc::asmb::AsmbInner::V2(v2) => {
                for entry in v2.unk2.elements {
                    for e1 in entry.unk1.elements {
                        if xc3_lib::hash::murmur3(e1.value.name.as_bytes()) != e1.value.name_hash {
                            println!("Incorrect hash for {:?}", e1.value.name);
                        }

                        for e8 in e1.value.children.elements {
                            if xc3_lib::hash::murmur3(e8.value.name2.as_bytes())
                                != e8.value.name2_hash
                            {
                                println!("Incorrect hash for {:?}", e8.value.name2);
                            }
                        }
                    }

                    for e2 in entry.unk2.elements {
                        if xc3_lib::hash::murmur3(e2.value.name2.as_bytes()) != e2.value.name2_hash
                        {
                            println!("Incorrect hash for {:?}", e2.value.name2);
                        }
                    }
                }
            }
        },
    }
}

fn check_eva(eva: Eva, path: &Path, original_bytes: &[u8], check_read_write: bool) {
    if check_read_write {
        let mut writer = Cursor::new(Vec::new());
        eva.write(&mut writer).unwrap();
        if writer.into_inner() != original_bytes {
            println!("Eva read/write not 1:1 for {path:?}");
        }
    }
}

fn check_mxmd_legacy(
    mxmd: MxmdLegacy,
    path: &Path,
    _original_bytes: &[u8],
    check_read_write: bool,
) {
    if let Some(textures) = mxmd.packed_textures {
        for texture in textures.textures {
            match Mtxt::from_bytes(&texture.mtxt_data) {
                Ok(mtxt) => check_mtxt(mtxt, path, &texture.mtxt_data, check_read_write),
                Err(e) => println!("Error reading Mtxt in {path:?}: {e}"),
            }
        }
    }
    // TODO: check read/write for camdo?
    // TODO: Also test loading casmt data?
}

fn check_mtxt(mtxt: Mtxt, path: &Path, original_bytes: &[u8], check_read_write: bool) {
    if check_read_write {
        let mut writer = Cursor::new(Vec::new());
        mtxt.write(&mut writer).unwrap();
        if writer.into_inner() != original_bytes {
            println!("Mtxt read/write not 1:1 for {path:?}");
        }
        // TODO: Check read/write for dds?
    }
}

fn check_bmn(bmn: Bmn, path: &Path, _original_bytes: &[u8], check_read_write: bool) {
    if let Some(unk16) = bmn.unk16 {
        for texture in unk16.textures {
            if !texture.mtxt_data.is_empty() {
                match Mtxt::from_bytes(&texture.mtxt_data) {
                    Ok(mtxt) => check_mtxt(mtxt, path, &texture.mtxt_data, check_read_write),
                    Err(e) => println!("Error reading Mtxt in {path:?}: {e}"),
                }
            }
        }
    }
}

fn check_all<P, T, F>(
    root: P,
    patterns: &[&str],
    check_file: F,
    endian: Endian,
    check_read_write: bool,
) where
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
            match reader.read_type(endian) {
                Ok(file) => check_file(file, path, &original_bytes, check_read_write),
                Err(e) => println!("Error reading {path:?}: {e}"),
            }
        });
}

fn check_all_gltf<P: AsRef<Path>>(root: P) {
    globwalk::GlobWalkerBuilder::from_patterns(root.as_ref(), &["*.{wimdo}"])
        .build()
        .unwrap()
        .par_bridge()
        .for_each(|entry| {
            let path = entry.as_ref().unwrap().path();
            match xc3_model::load_model(path, None) {
                Ok(root) => {
                    if let Err(e) = xc3_model::gltf::GltfFile::new("model", &[root]) {
                        println!("Error converting {path:?}: {e}");
                    }
                }
                Err(e) => println!("Error loading {path:?}: {e}"),
            }
        });

    // Process files sequentially since gltf processing is already highly threaded.
    globwalk::GlobWalkerBuilder::from_patterns(root.as_ref(), &["*.{wismhd}"])
        .build()
        .unwrap()
        .for_each(|entry| {
            let path = entry.as_ref().unwrap().path();
            match xc3_model::load_map(path, None) {
                Ok(roots) => {
                    if let Err(e) = xc3_model::gltf::GltfFile::new("model", &roots) {
                        println!("Error converting {path:?}: {e}");
                    }
                }
                Err(e) => println!("Error loading {path:?}: {e}"),
            }
        });
}

fn check_all_wimdo_model<P: AsRef<Path>>(root: P, check_read_write: bool) {
    globwalk::GlobWalkerBuilder::from_patterns(root.as_ref(), &["*.{wimdo}"])
        .build()
        .unwrap()
        .par_bridge()
        .for_each(|entry| {
            let path = entry.as_ref().unwrap().path();

            // Test reimporting models without any changes.
            let mxmd = Mxmd::from_file(path).unwrap();
            let msrd = Msrd::from_file(path.with_extension("wismt")).unwrap();
            let streaming_data =
                xc3_model::StreamingData::new(&mxmd, &path.with_extension("wismt"), false, None)
                    .unwrap();

            match xc3_model::ModelRoot::from_mxmd_model(&mxmd, None, &streaming_data, None) {
                Ok(root) => {
                    // TODO: Create a function that loads files from wimdo path?
                    // TODO: Should this take the msrd or streaming?
                    // TODO: Is it worth being able to test this without compression?
                    if check_read_write {
                        let (_new_mxmd, new_msrd) = root.to_mxmd_model(&mxmd, &msrd);
                        let (new_vertex, _, _) = new_msrd.extract_files(None).unwrap();
                        if &new_vertex != streaming_data.vertex.as_ref() {
                            println!("VertexData not 1:1 for {path:?}")
                        }
                    }
                }
                Err(e) => println!("Error loading {path:?}: {e}"),
            }
        });
}
