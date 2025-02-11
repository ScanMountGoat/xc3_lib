use std::{
    io::{BufReader, Cursor},
    path::{Path, PathBuf},
};

use approx::RelativeEq;
use binrw::{BinRead, BinReaderExt, Endian};
use clap::Parser;
use glam::Mat4;
use rayon::prelude::*;
use xc3_lib::{
    apmd::Apmd,
    bc::Bc,
    beb::{Beb, BebData},
    beh::Beh,
    bmn::Bmn,
    dhal::Dhal,
    efb0::Efb0,
    eva::Eva,
    fnt::Fnt,
    idcm::Idcm,
    laft::Laft,
    lagp::Lagp,
    laps::Laps,
    last::Last,
    ltpc::Ltpc,
    mibl::Mibl,
    msmd::Msmd,
    msrd::{streaming::chr_tex_nx_folder, Msrd},
    mths::Mths,
    mtxt::Mtxt,
    mxmd::{legacy::MxmdLegacy, Mxmd},
    sar1::{ChCl, Csvb, Sar1},
    spch::Spch,
    xbc1::{MaybeXbc1, Xbc1},
};
use xc3_model::{load_skel, monolib::ShaderTextures, ModelRoot};
use xc3_write::{Xc3Write, Xc3WriteOffsets};

// TODO: Avoid redundant loads for wimdo and wismhd
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

    /// Process Beb files from .beb
    #[arg(long)]
    beb: bool,

    /// Process Beh files from .beh
    #[arg(long)]
    beh: bool,

    /// Process efb0 files from .wiefb
    #[arg(long)]
    efb0: bool,

    /// Process XCX fnt files from .fnt
    #[arg(long)]
    fnt: bool,

    /// Process IDCM files from .idcm or .wiidcm
    #[arg(long)]
    idcm: bool,

    /// Process LAFT files from .wifnt
    #[arg(long)]
    laft: bool,

    /// Process LAST files from .wisty
    #[arg(long)]
    last: bool,

    /// Process MTXT files from .catex, .calut, and .caavp
    #[arg(long)]
    mtxt: bool,

    /// Process MXMD files from .camdo
    #[arg(long)]
    camdo: bool,

    /// Process BMN files from .bmn
    #[arg(long)]
    bmn: bool,

    /// Process MTHS files from .cashd
    #[arg(long)]
    mths: bool,

    /// Process all file types except gltf and wimdo-model.
    #[arg(long)]
    all: bool,

    /// Convert wimdo and wismhd to gltf without saving.
    #[arg(long)]
    gltf: bool,

    /// Convert wimdo models to and from xc3_model types.
    #[arg(long)]
    wimdo_model: bool,

    /// Load animations from .mot, .anm, and .motsm_data files.
    #[arg(long)]
    animation: bool,

    /// Load collisions from .wiidcm and .idcm files.
    #[arg(long)]
    collision: bool,

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

    simple_logger::SimpleLogger::new()
        .with_level(log::LevelFilter::Error)
        .init()
        .unwrap();

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

    if cli.beb || cli.all {
        println!("Checking Beb files ...");
        check_all(root, &["*.beb"], check_beb, Endian::Little, cli.rw);
    }

    if cli.beh || cli.all {
        println!("Checking Beh files ...");
        check_all(root, &["*.beh"], check_beh, Endian::Little, cli.rw);
    }

    if cli.efb0 || cli.all {
        println!("Checking Efb0 files ...");
        check_all(root, &["*.wiefb"], check_efb0, Endian::Little, cli.rw);
    }

    if cli.fnt || cli.all {
        println!("Checking fnt files ...");
        check_all(root, &["*.fnt"], check_fnt, Endian::Big, cli.rw);
    }

    if cli.idcm || cli.all {
        println!("Checking Idcm files ...");
        check_all(
            root,
            &["*.idcm", "*.wiidcm"],
            check_idcm,
            Endian::Little,
            cli.rw,
        );
    }

    if cli.laft || cli.all {
        println!("Checking Laft files ...");
        check_all(root, &["*.wifnt"], check_laft_data, Endian::Little, cli.rw);
    }

    if cli.last || cli.all {
        println!("Checking Last files ...");
        check_all(root, &["*.wisty"], check_last_data, Endian::Little, cli.rw);
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

    if cli.mths || cli.all {
        println!("Checking Mths files ...");
        check_all(root, &["*.cashd"], check_mths, Endian::Big, cli.rw);
    }

    if cli.gltf || cli.all {
        println!("Checking glTF export ...");
        check_all_gltf(root);
    }

    if cli.wimdo_model || cli.all {
        println!("Checking wimdo model conversions ...");
        check_all_wimdo_model(root, cli.rw);
    }

    if cli.animation || cli.all {
        println!("Checking animations ...");
        check_all_animations(root, cli.rw);
    }

    if cli.collision || cli.all {
        println!("Checking collisions ...");
        check_all_collisions(root, cli.rw);
    }

    println!("Finished in {:?}", start.elapsed());
}

fn check_all_mibl<P: AsRef<Path>>(root: P, check_read_write: bool) {
    // Only Xenoblade 3 has a dedicated tex directory with shared textures.
    let folder = root.as_ref().join("chr").join("tex").join("nx");
    if folder.exists() {
        globwalk::GlobWalkerBuilder::from_patterns(folder, &["*.wismt", "!h/**"])
            .build()
            .unwrap()
            .par_bridge()
            .for_each(|entry| {
                let path = entry.as_ref().unwrap().path();
                let (original_bytes, mibl) = read_wismt_single_tex(path);

                if let Some(base_mip_path) = base_mip_path(path) {
                    if let Ok(base_mip) = Xbc1::from_file(base_mip_path) {
                        // Test joining and splitting base mip levels.
                        let base_mip = base_mip.decompress().unwrap();
                        let combined = mibl.to_surface_with_base_mip(&base_mip).unwrap();
                        let combined_mibl = Mibl::from_surface(combined).unwrap();

                        let (new_mibl, new_base_mip) = combined_mibl.split_base_mip();
                        if new_base_mip != base_mip || new_mibl != mibl {
                            println!("Join/split Mibl not 1:1 for {path:?}");
                        }
                    }
                }

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

fn base_mip_path(path: &Path) -> Option<PathBuf> {
    // chr/tex/nx/m/file.wismt -> chr/tex/nx/h/file.wismt
    Some(path.parent()?.parent()?.join("h").join(path.file_name()?))
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
    match data {
        MaybeXbc1::Uncompressed(data) => check_file(data, path, original_bytes, check_read_write),
        MaybeXbc1::Xbc1(xbc1) => {
            if check_read_write && !write_le_bytes_equals(&xbc1, original_bytes) {
                println!("Xbc1 read/write not 1:1 for {path:?}");
            }

            match xbc1.extract() {
                Ok(data) => check_file(data, path, &xbc1.decompress().unwrap(), check_read_write),
                Err(e) => println!("Error extracting from {path:?}: {e}"),
            }
        }
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

    if check_read_write && !write_le_bytes_equals(&msrd, original_bytes) {
        println!("Msrd read/write not 1:1 for {path:?}");
    }

    match &msrd.streaming.inner {
        xc3_lib::msrd::StreamingInner::StreamingLegacy(_) => todo!(),
        xc3_lib::msrd::StreamingInner::Streaming(data) => {
            // Check embedded data.
            let vertex_bytes = data
                .decompress_stream_entry(0, data.vertex_data_entry_index, &msrd.data)
                .unwrap();
            check_vertex_data(vertex, path, &vertex_bytes, check_read_write);

            let spch_bytes = data
                .decompress_stream_entry(0, data.shader_entry_index, &msrd.data)
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
    if check_read_write && !write_le_bytes_equals(&vertex_data, original_bytes) {
        println!("VertexData read/write not 1:1 for {path:?}");
    }

    for buffer in vertex_data.vertex_buffers {
        for a in buffer.attributes {
            if a.data_type.size_in_bytes() != a.data_size as usize {
                println!("Unexpected size {} for {:?}", a.data_size, a.data_type);
            }
        }
    }
}

fn check_msmd(msmd: Msmd, path: &Path, _original_bytes: &[u8], check_read_write: bool) {
    // Parse all the data from the .wismda
    let mut reader = BufReader::new(std::fs::File::open(path.with_extension("wismda")).unwrap());

    let compressed = msmd.wismda_info.compressed_length != msmd.wismda_info.decompressed_length;

    for (i, model) in msmd.map_models.iter().enumerate() {
        match model.entry.extract(&mut reader, compressed) {
            Ok(model) => {
                check_spch(model.spch, path, &[], false);
            }
            Err(e) => println!("Error extracting map model {i} in {path:?}: {e}"),
        }
    }

    for (i, model) in msmd.prop_models.iter().enumerate() {
        match model.entry.extract(&mut reader, compressed) {
            Ok(model) => {
                check_spch(model.spch, path, &[], false);
            }
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
                check_vertex_data(model.vertex_data, path, &[], false);
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
            Ok(model) => {
                check_vertex_data(model.vertex_data, path, &[], false);
                check_spch(model.spch, path, &[], false);
            }
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

    if check_read_write && !write_le_bytes_equals(&mibl, original_bytes) {
        println!("Mibl read/write not 1:1 for {path:?}");
    }
}

fn read_wismt_single_tex(path: &Path) -> (Vec<u8>, Mibl) {
    let xbc1 = Xbc1::from_file(path).unwrap();
    let decompressed = xbc1.decompress().unwrap();
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

    if check_read_write && !write_le_bytes_equals(&dhal, original_bytes) {
        println!("Dhal read/write not 1:1 for {path:?}");
    }
}

fn check_lagp(lagp: Lagp, path: &Path, original_bytes: &[u8], check_read_write: bool) {
    if let Some(textures) = &lagp.textures {
        for texture in &textures.textures {
            let mibl = Mibl::from_bytes(&texture.mibl_data).unwrap();
            check_mibl(mibl, path, &texture.mibl_data, check_read_write);
        }
    }

    if check_read_write && !write_le_bytes_equals(&lagp, original_bytes) {
        println!("Lagp read/write not 1:1 for {path:?}");
    }
}

fn check_laps(laps: Laps, path: &Path, original_bytes: &[u8], check_read_write: bool) {
    if check_read_write && !write_le_bytes_equals(&laps, original_bytes) {
        println!("Laps read/write not 1:1 for {path:?}");
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
                            check_mxmd(*mxmd, path, &entry.entry_data, check_read_write)
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

            if check_read_write && !write_le_bytes_equals(&apmd, original_bytes) {
                println!("Apmd read/write not 1:1 for {path:?}");
            }
        }
    }
}

fn check_mxmd(mxmd: Mxmd, path: &Path, original_bytes: &[u8], check_read_write: bool) {
    if !is_valid_models_flags(&mxmd) {
        println!("Inconsistent ModelsFlags for {path:?}");
    }

    if check_read_write && !write_le_bytes_equals(&mxmd, original_bytes) {
        println!("Mxmd read/write not 1:1 for {path:?}");
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
            && flags.has_alpha_table() == mxmd.models.alpha_table.is_some()
    } else {
        true
    }
}

fn check_spch(spch: Spch, path: &Path, original_bytes: &[u8], check_read_write: bool) {
    for (i, slct_offset) in spch.slct_offsets.iter().enumerate() {
        match slct_offset.read_slct(&spch.slct_section) {
            Ok(_) => {
                // TODO: Check that the extracted binary sizes add up to the total size.
                // TODO: check constant buffer size.
            }
            Err(e) => println!("Error reading Slct {i} for {path:?}: {e}"),
        }
    }

    if check_read_write && !write_le_bytes_equals(&spch, original_bytes) {
        println!("Spch read/write not 1:1 for {path:?}");
    }
}

fn check_ltpc(ltpc: Ltpc, path: &Path, original_bytes: &[u8], check_read_write: bool) {
    if check_read_write && !write_le_bytes_equals(&ltpc, original_bytes) {
        println!("Ltpc read/write not 1:1 for {path:?}");
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
                    if check_read_write && !write_le_bytes_equals(&bc, &entry.entry_data) {
                        println!("Bc read/write not 1:1 for {:?} in {path:?}", entry.name);
                    }
                    check_bc(*bc, path, &[], false);
                }
                Sar1EntryData::ChCl(chcl) => {
                    if check_read_write && !write_le_bytes_equals(&chcl, original_bytes) {
                        println!("ChCl read/write not 1:1 for {:?} in {path:?}", entry.name);
                    }
                }
                Sar1EntryData::Csvb(csvb) => {
                    if check_read_write && !write_le_bytes_equals(&csvb, original_bytes) {
                        println!("Csvb read/write not 1:1 for {:?} in {path:?}", entry.name);
                    }
                }
                Sar1EntryData::Eva(eva) => {
                    if check_read_write && !write_le_bytes_equals(&eva, original_bytes) {
                        println!("Eva read/write not 1:1 for {:?} in {path:?}", entry.name);
                    }
                }
            },
            Err(e) => println!("Error reading {:?} in {path:?}: {e}", entry.name),
        }
    }

    if check_read_write && !write_le_bytes_equals(&sar1, original_bytes) {
        println!("Sar1 read/write not 1:1 for {path:?}");
    }
}

fn check_bc(bc: Bc, path: &Path, original_bytes: &[u8], check_read_write: bool) {
    if check_read_write && !write_le_bytes_equals(&bc, original_bytes) {
        println!("Bc read/write not 1:1 for {path:?}");
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
    if check_read_write && !write_le_bytes_equals(&eva, original_bytes) {
        println!("Eva read/write not 1:1 for {path:?}");
    }
}

fn check_beb(beb: Beb, path: &Path, original_bytes: &[u8], check_read_write: bool) {
    if check_read_write && !write_le_bytes_equals(&beb, original_bytes) {
        println!("Beb read/write not 1:1 for {path:?}");
    }

    for (i, offset) in beb.xbc1_offsets.iter().enumerate() {
        match offset.value.decompress() {
            Ok(bytes) => match BebData::read_le(&mut Cursor::new(&bytes)) {
                Ok(data) => {
                    for (offset, size) in data.offsets.iter().zip(&data.lengths) {
                        // Skip the 4 floats at the start of each entry.
                        let start = *offset as usize + 16;
                        let entry_bytes = &bytes[start..start + *size as usize];
                        // TODO: detect item type?
                        if entry_bytes.get(..4) == Some(b"BC\x00\x00") {
                            match Bc::from_bytes(entry_bytes) {
                                Ok(bc) => check_bc(bc, path, entry_bytes, check_read_write),
                                Err(e) => {
                                    println!("Error reading BC in archive {i} in {path:?}: {e}")
                                }
                            }
                        }
                    }
                }
                Err(e) => println!("Error reading data in archive {i} in {path:?}: {e}"),
            },
            Err(e) => println!("Error decompressing archive {i} in {path:?}: {e}"),
        }
    }
}

fn check_beh(beh: Beh, path: &Path, original_bytes: &[u8], check_read_write: bool) {
    if check_read_write && !write_le_bytes_equals(&beh, original_bytes) {
        println!("Beh read/write not 1:1 for {path:?}");
    }
}

fn check_efb0(efb0: Efb0, path: &Path, original_bytes: &[u8], check_read_write: bool) {
    if check_read_write && !write_le_bytes_equals(&efb0, original_bytes) {
        println!("efb0 read/write not 1:1 for {path:?}");
    }
}

fn check_idcm(idcm: Idcm, path: &Path, original_bytes: &[u8], check_read_write: bool) {
    if check_read_write && !write_le_bytes_equals(&idcm, original_bytes) {
        println!("Idcm read/write not 1:1 for {path:?}");
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

    for shader in mxmd.shaders.shaders {
        match Mths::from_bytes(&shader.mths_data) {
            Ok(mths) => {
                check_mths(mths, path, &shader.mths_data, check_read_write);
            }
            Err(e) => println!("Error reading Mths in {path:?}: {e}"),
        }
    }

    // TODO: check read/write for camdo?
    // TODO: Also test loading casmt data?

    for buffer in mxmd.vertex.vertex_buffers {
        for a in buffer.attributes {
            if a.data_type.size_in_bytes() != a.data_size as usize {
                println!("Unexpected size {} for {:?}", a.data_size, a.data_type);
            }
        }
    }
}

fn check_mths(mths: Mths, path: &Path, _original_bytes: &[u8], _check_read_write: bool) {
    if let Err(e) = mths.vertex_shader() {
        println!("Error reading vertex shader in {path:?}: {e}")
    }
    if let Err(e) = mths.fragment_shader() {
        println!("Error reading fragment shader in {path:?}: {e}")
    }
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
    if let Err(e) = mtxt.deswizzled_image_data() {
        println!(
            "Error deswizzling surface for {path:?}: {e}\n{:#?}",
            mtxt.footer
        );
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

fn check_laft_data(
    data: MaybeXbc1<Laft>,
    path: &Path,
    original_bytes: &[u8],
    check_read_write: bool,
) {
    check_maybe_xbc1(data, path, check_read_write, original_bytes, check_laft);
}

fn check_laft(laft: Laft, path: &Path, original_bytes: &[u8], check_read_write: bool) {
    if !laft.mappings.len().is_power_of_two() {
        println!("Laft: mappings len not power of 2 for {path:?}");
    }
    let read_glyphs = (0..u16::MAX)
        .filter(|&code| laft.get_glyph(code).is_some_and(|g| g.1.codepoint == code))
        .count();
    if read_glyphs != laft.font_info.len() {
        println!("Laft: found unreachable glyphs in {path:?}");
    }
    if let Some(texture) = laft.texture.clone() {
        check_mibl(texture, path, &[], false);
    }

    if check_read_write && !write_le_bytes_equals(&laft, original_bytes) {
        println!("Laft read/write not 1:1 for {path:?}");
    }
}

fn check_last_data(
    data: MaybeXbc1<Last>,
    path: &Path,
    original_bytes: &[u8],
    check_read_write: bool,
) {
    check_maybe_xbc1(data, path, check_read_write, original_bytes, check_last);
}

fn check_last(last: Last, path: &Path, original_bytes: &[u8], check_read_write: bool) {
    if check_read_write && !write_le_bytes_equals(&last, original_bytes) {
        println!("Last read/write not 1:1 for {path:?}");
    }
}

fn check_fnt(fnt: Fnt, path: &Path, original_bytes: &[u8], check_read_write: bool) {
    if fnt.font.get_glyph_by_utf16(0x2a).is_none() {
        println!("{path:?} has no \"*\" character registered! The game will crash on unsupported characters.");
    }
    if check_read_write && !write_be_bytes_equals(&fnt, original_bytes) {
        println!("Fnt read/write not 1:1 for {path:?}");
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
    // Assume root is the dump root path.
    let shader_textures = ShaderTextures::from_folder(root.as_ref().join("monolib/shader"));

    globwalk::GlobWalkerBuilder::from_patterns(root.as_ref(), &["*.{wimdo}"])
        .build()
        .unwrap()
        .par_bridge()
        .for_each(|entry| {
            let path = entry.as_ref().unwrap().path();
            match xc3_model::load_model(path, None) {
                Ok(root) => {
                    if let Err(e) = xc3_model::gltf::GltfFile::from_model(
                        "model",
                        &[root],
                        &[],
                        &shader_textures,
                        false,
                    ) {
                        println!("Error converting {path:?}: {e}");
                    }
                }
                Err(e) => println!("Error loading {path:?}: {e}"),
            }
        });

    globwalk::GlobWalkerBuilder::from_patterns(root.as_ref(), &["*.{camdo}"])
        .build()
        .unwrap()
        .par_bridge()
        .for_each(|entry| {
            let path = entry.as_ref().unwrap().path();
            match xc3_model::load_model_legacy(path, None) {
                Ok(root) => {
                    if let Err(e) = xc3_model::gltf::GltfFile::from_model(
                        "model",
                        &[root],
                        &[],
                        &shader_textures,
                        true,
                    ) {
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
                    if let Err(e) = xc3_model::gltf::GltfFile::from_map(
                        "model",
                        &roots,
                        &shader_textures,
                        false,
                    ) {
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
            // Avoid compressing or decompressing data more than once for performance.
            match Mxmd::from_file(path) {
                Ok(mxmd) => {
                    let streaming_data = xc3_model::StreamingData::from_files(
                        &mxmd,
                        &path.with_extension("wismt"),
                        false,
                        None,
                    )
                    .unwrap();

                    let model_name = path.file_stem().unwrap_or_default().to_string_lossy();
                    let skel = load_skel(path, &model_name);

                    match ModelRoot::from_mxmd_model(&mxmd, skel, &streaming_data, None) {
                        Ok(root) => {
                            if check_read_write {
                                check_model(root, &mxmd, &streaming_data.vertex, path);
                            }
                        }
                        Err(e) => println!("Error loading {path:?}: {e}"),
                    }
                }
                Err(e) => println!("Error reading {path:?}: {e}"),
            }
        });
}

fn check_model(
    root: xc3_model::ModelRoot,
    mxmd: &Mxmd,
    vertex: &xc3_lib::vertex::VertexData,
    path: &Path,
) {
    let (new_mxmd, new_vertex, _) = root.to_mxmd_model_files(mxmd).unwrap();
    if new_vertex.buffer != vertex.buffer {
        println!("VertexData buffer not 1:1 for {path:?}");
    } else if &new_vertex != vertex {
        println!("VertexData not 1:1 for {path:?}");
    }

    // TODO: How many of these fields should be preserved?
    if new_mxmd.models.alpha_table != mxmd.models.alpha_table {
        println!("Alpha table not 1:1 for {path:?}");
    }
    if new_mxmd.models.models != mxmd.models.models {
        println!("Model list not 1:1 for {path:?}");
    }
    if let Some(skinning) = &mxmd.models.skinning {
        if let Some(new_skinning) = &new_mxmd.models.skinning {
            if new_skinning.bones != skinning.bones {
                println!("Skinning bones not 1:1 for {path:?}");
            }
            if new_skinning.constraints != skinning.constraints {
                println!("Skinning constraints not 1:1 for {path:?}");
            }
            if new_skinning.bounds != skinning.bounds {
                println!("Skinning bounds not 1:1 for {path:?}");
            }
            // Use a generous tolerance to allow for inaccuracies in computations.
            // TODO: Find a more accurate method for inverting bone transforms.
            let count = new_skinning
                .inverse_bind_transforms
                .iter()
                .zip(&skinning.inverse_bind_transforms)
                .filter(|(m1, m2)| {
                    !Mat4::from_cols_array_2d(m1).relative_eq(
                        &Mat4::from_cols_array_2d(m2),
                        0.5,
                        0.1,
                    )
                })
                .count();
            if count > 0 {
                println!(
                    "Skinning transforms not within tolerances for {count} of {} bones for {path:?}",
                    skinning.bones.len()
                );
            }
        }
    }
}

fn check_all_animations<P: AsRef<Path>>(root: P, _check_read_write: bool) {
    globwalk::GlobWalkerBuilder::from_patterns(root.as_ref(), &["*.{mot, anm, motstm_data}"])
        .build()
        .unwrap()
        .par_bridge()
        .for_each(|entry| {
            let path = entry.as_ref().unwrap().path();

            match xc3_model::load_animations(path) {
                Ok(_) => (),
                Err(e) => println!("Error loading {path:?}: {e:?}"),
            }
        });
}

fn check_all_collisions<P: AsRef<Path>>(root: P, _check_read_write: bool) {
    globwalk::GlobWalkerBuilder::from_patterns(root.as_ref(), &["*.{wiidcm, idcm}"])
        .build()
        .unwrap()
        .par_bridge()
        .for_each(|entry| {
            let path = entry.as_ref().unwrap().path();

            match xc3_model::load_collisions(path) {
                Ok(_) => (),
                Err(e) => println!("Error loading {path:?}: {e:?}"),
            }
        });
}

fn write_le_bytes_equals<T>(value: &T, original_bytes: &[u8]) -> bool
where
    T: Xc3Write + 'static,
    for<'a> T::Offsets<'a>: Xc3WriteOffsets<Args = ()>,
{
    let mut writer = Cursor::new(Vec::new());
    xc3_write::write_full(value, &mut writer, 0, &mut 0, xc3_write::Endian::Little, ()).unwrap();
    writer.into_inner() == original_bytes
}

fn write_be_bytes_equals<T>(value: &T, original_bytes: &[u8]) -> bool
where
    T: Xc3Write + 'static,
    for<'a> T::Offsets<'a>: Xc3WriteOffsets<Args = ()>,
{
    let mut writer = Cursor::new(Vec::new());
    xc3_write::write_full(value, &mut writer, 0, &mut 0, xc3_write::Endian::Big, ()).unwrap();
    writer.into_inner() == original_bytes
}
