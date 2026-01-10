use std::{
    io::Cursor,
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
    msrd::{Msrd, streaming::chr_folder},
    mths::Mths,
    mtxt::Mtxt,
    mxmd::{Mxmd, MxmdV112, legacy::MxmdLegacy, legacy2::MxmdV40},
    offset::{OffsetRange, OffsetValidationError, read_type_get_offsets},
    sar1::{ChCl, Csvb, Sar1},
    spch::Spch,
    xbc1::{MaybeXbc1, Xbc1},
};
use xc3_model::{
    ModelRoot, load_skel,
    model::import::{ModelFilesV40, ModelFilesV111, ModelFilesV112},
    monolib::ShaderTextures,
    shader_database::ShaderDatabase,
};
use xc3_write::WriteFull;

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

    /// Process DMXM files from .wimdo
    #[arg(long)]
    wimdo2: bool,

    /// Check that read/write is 1:1 for all files and embedded files.
    #[arg(long)]
    rw: bool,

    /// Load a shader database from a .bin file
    #[arg(long)]
    database: Option<String>,
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
        check_all::<Wimdo>(root, &["*.wimdo", "*.pcmdo"], Endian::Little, cli.rw);
    }

    if cli.msrd || cli.all {
        // Skip the .wismt textures in the XC3 tex folder.
        println!("Checking Msrd files ...");
        check_all::<Msrd>(root, &["*.wismt", "!**/tex/**"], Endian::Little, cli.rw);
    }

    if cli.msmd || cli.all {
        println!("Checking Msmd files ...");
        check_all::<Msmd>(root, &["*.wismhd"], Endian::Little, cli.rw);
    }

    if cli.sar1 || cli.all {
        println!("Checking Sar1 files ...");
        check_all::<MaybeXbc1<Sar1>>(root, &["*.arc", "*.chr", "*.mot"], Endian::Little, cli.rw);
    }

    if cli.spch || cli.all {
        println!("Checking Spch files ...");
        check_all::<Spch>(root, &["*.wishp"], Endian::Little, cli.rw);
    }

    if cli.wilay || cli.all {
        println!("Checking Dhal and Lagp files ...");
        check_all::<MaybeXbc1<Wilay>>(root, &["*.wilay"], Endian::Little, cli.rw);
    }

    if cli.ltpc || cli.all {
        println!("Checking Ltpc files ...");
        check_all::<Ltpc>(root, &["*.wiltp"], Endian::Little, cli.rw);
    }

    if cli.bc || cli.all {
        println!("Checking Bc files ...");
        check_all::<MaybeXbc1<Bc>>(root, &["*.anm", "*.motstm_data"], Endian::Little, cli.rw);
    }

    if cli.eva || cli.all {
        println!("Checking Eva files ...");
        check_all::<Eva>(root, &["*.eva"], Endian::Little, cli.rw);
    }

    if cli.beb || cli.all {
        println!("Checking Beb files ...");
        check_all::<Beb>(root, &["*.beb"], Endian::Little, cli.rw);
    }

    if cli.beh || cli.all {
        println!("Checking Beh files ...");
        check_all::<Beh>(root, &["*.beh"], Endian::Little, cli.rw);
    }

    if cli.efb0 || cli.all {
        println!("Checking Efb0 files ...");
        check_all::<Efb0>(root, &["*.wiefb"], Endian::Little, cli.rw);
    }

    if cli.fnt || cli.all {
        println!("Checking fnt files ...");
        check_all::<Fnt>(root, &["*.fnt"], Endian::Big, cli.rw);
    }

    if cli.idcm || cli.all {
        println!("Checking Idcm files ...");
        check_all::<Idcm>(root, &["*.idcm", "*.wiidcm"], Endian::Little, cli.rw);
    }

    if cli.laft || cli.all {
        println!("Checking Laft files ...");
        check_all::<MaybeXbc1<Laft>>(root, &["*.wifnt"], Endian::Little, cli.rw);
    }

    if cli.last || cli.all {
        println!("Checking Last files ...");
        check_all::<MaybeXbc1<Last>>(root, &["*.wisty"], Endian::Little, cli.rw);
    }

    if cli.mtxt || cli.all {
        println!("Checking Mtxt files ...");
        check_all::<Mtxt>(
            root,
            &["*.catex", "*.calut", "*.caavp"],
            Endian::Big,
            cli.rw,
        );
    }

    // Xenoblade X.
    if cli.camdo || cli.all {
        println!("Checking Mxmd files ...");
        check_all::<MxmdLegacy>(root, &["*.camdo"], Endian::Big, cli.rw);
    }

    if cli.bmn || cli.all {
        println!("Checking Bmn files ...");
        check_all::<Bmn>(root, &["*.bmn"], Endian::Big, cli.rw);
    }

    if cli.mths || cli.all {
        println!("Checking Mths files ...");
        check_all::<Mths>(root, &["*.cashd"], Endian::Big, cli.rw);
    }

    if cli.gltf || cli.all {
        println!("Checking glTF export ...");
        check_all_gltf(root);
    }

    if cli.wimdo_model || cli.all {
        println!("Checking wimdo model conversions ...");
        check_all_wimdo_model(root, cli.rw, cli.database);
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

trait CheckFile {
    fn check_file(
        self,
        path: &Path,
        original_bytes: &[u8],
        ranges: &[OffsetRange],
        check_read_write: bool,
    );
}

fn check_all_mibl(root: &Path, check_read_write: bool) {
    // Only Xenoblade 3 has a dedicated tex directory with shared textures.
    let folder = root.join("chr").join("tex").join("nx");
    if folder.exists() {
        globwalk::GlobWalkerBuilder::from_patterns(folder, &["*.wismt", "!h/**"])
            .build()
            .unwrap()
            .par_bridge()
            .for_each(|entry| {
                let path = entry.as_ref().unwrap().path();
                let (original_bytes, mibl) = read_wismt_single_tex(path);

                if let Some(base_mip_path) = base_mip_path(path)
                    && let Ok(base_mip) = Xbc1::from_file(base_mip_path)
                {
                    // Test joining and splitting base mip levels.
                    let base_mip = base_mip.decompress().unwrap();
                    let combined = mibl.to_surface_with_base_mip(&base_mip).unwrap();
                    let combined_mibl = Mibl::from_surface(combined).unwrap();

                    let (new_mibl, new_base_mip) = combined_mibl.split_base_mip();
                    if new_base_mip != base_mip || new_mibl != mibl {
                        println!("Join/split Mibl not 1:1 for {path:?}");
                    }
                }

                mibl.check_file(path, &original_bytes, &[], check_read_write);
            });
    }

    let folder = root.join("monolib").join("shader");
    globwalk::GlobWalkerBuilder::from_patterns(folder, &["*.{witex,witx}"])
        .build()
        .unwrap()
        .par_bridge()
        .for_each(|entry| {
            let path = entry.as_ref().unwrap().path();
            let original_bytes = std::fs::read(path).unwrap();
            let mibl = Mibl::from_file(path).unwrap();
            mibl.check_file(path, &original_bytes, &[], check_read_write);
        });
}

fn base_mip_path(path: &Path) -> Option<PathBuf> {
    // chr/tex/nx/m/file.wismt -> chr/tex/nx/h/file.wismt
    Some(path.parent()?.parent()?.join("h").join(path.file_name()?))
}

impl<T> CheckFile for MaybeXbc1<T>
where
    T: CheckFile,
    for<'a> T: BinRead<Args<'a> = ()>,
{
    fn check_file(
        self,
        path: &Path,
        original_bytes: &[u8],
        original_ranges: &[OffsetRange],
        check_read_write: bool,
    ) {
        match self {
            MaybeXbc1::Uncompressed(data) => T::check_file(
                data,
                path,
                original_bytes,
                original_ranges,
                check_read_write,
            ),
            MaybeXbc1::Xbc1(xbc1) => {
                if check_read_write && xbc1.to_bytes().unwrap() != original_bytes {
                    println!("Xbc1 read/write not 1:1 for {path:?}");
                }

                match xbc1.extract() {
                    Ok(data) => T::check_file(
                        data,
                        path,
                        &xbc1.decompress().unwrap(),
                        original_ranges,
                        check_read_write,
                    ),
                    Err(e) => println!("Error extracting from {path:?}: {e}"),
                }
            }
        }
    }
}

impl CheckFile for Msrd {
    fn check_file(
        self,
        path: &Path,
        original_bytes: &[u8],
        _original_ranges: &[OffsetRange],
        check_read_write: bool,
    ) {
        // TODO: check stream flags?
        if check_read_write && self.to_bytes().unwrap() != original_bytes {
            println!("Msrd read/write not 1:1 for {path:?}");
        }

        let chr_folder = chr_folder(path);
        if let Ok(files) = self.extract_files(chr_folder.as_deref()) {
            match &self.streaming.inner {
                xc3_lib::msrd::StreamingInner::StreamingLegacy(_) => todo!(),
                xc3_lib::msrd::StreamingInner::Streaming(data) => {
                    // Check embedded data.
                    let vertex_bytes = data
                        .decompress_stream_entry(0, data.vertex_data_entry_index, &self.data)
                        .unwrap();
                    files
                        .vertex
                        .check_file(path, &vertex_bytes, &[], check_read_write);

                    let spch_bytes = data
                        .decompress_stream_entry(0, data.shader_entry_index, &self.data)
                        .unwrap();
                    files
                        .shader
                        .check_file(path, &spch_bytes, &[], check_read_write);
                }
            }

            for texture in files.textures {
                texture.low.check_file(path, &[], &[], false);
                if let Some(high) = texture.high {
                    high.mid.check_file(path, &[], &[], false);
                }
            }
        } else if let Ok(_files) = self.extract_files_legacy(chr_folder.as_deref()) {
            // TODO: test XCX DE rebuilding once implemented
        } else {
            println!("Failed to extract {path:?}");
        }
    }
}

impl CheckFile for xc3_lib::vertex::VertexData {
    fn check_file(
        self,
        path: &Path,
        original_bytes: &[u8],
        _original_ranges: &[OffsetRange],
        check_read_write: bool,
    ) {
        if check_read_write && self.to_bytes().unwrap() != original_bytes {
            println!("VertexData read/write not 1:1 for {path:?}");
        }

        for buffer in self.vertex_buffers {
            for a in buffer.attributes {
                if a.data_type.size_in_bytes() != a.data_size as usize {
                    println!("Unexpected size {} for {:?}", a.data_size, a.data_type);
                }
            }
        }
    }
}

impl CheckFile for Msmd {
    fn check_file(
        self,
        path: &Path,
        _original_bytes: &[u8],
        _original_ranges: &[OffsetRange],
        check_read_write: bool,
    ) {
        // Parse all the data from the .wismda
        let mut reader = Cursor::new(std::fs::read(path.with_extension("wismda")).unwrap());

        let compressed = self.wismda_info.compressed_length != self.wismda_info.decompressed_length;

        for (i, model) in self.map_models.iter().enumerate() {
            match model.entry.extract(&mut reader, compressed) {
                Ok(model) => {
                    model.spch.check_file(path, &[], &[], false);
                }
                Err(e) => println!("Error extracting map model {i} in {path:?}: {e}"),
            }
        }

        for (i, model) in self.prop_models.iter().enumerate() {
            match model.entry.extract(&mut reader, compressed) {
                Ok(model) => {
                    model.spch.check_file(path, &[], &[], false);
                }
                Err(e) => println!("Error extracting prop model {i} in {path:?}: {e}"),
            }
        }

        for (i, model) in self.env_models.iter().enumerate() {
            match model.entry.extract(&mut reader, compressed) {
                Ok(model) => {
                    for texture in model.textures.textures {
                        let mibl = Mibl::from_bytes(&texture.mibl_data).unwrap();
                        mibl.check_file(path, &texture.mibl_data, &[], check_read_write);
                    }
                }
                Err(e) => println!("Error extracting env model {i} in {path:?}: {e}"),
            }
        }

        for (i, entry) in self.prop_vertex_data.iter().enumerate() {
            match entry.extract(&mut reader, compressed) {
                Ok(vertex_data) => {
                    let original_bytes = entry.decompress(&mut reader, compressed).unwrap();

                    vertex_data.check_file(path, &original_bytes, &[], check_read_write);
                }
                Err(e) => println!("Error extracting prop VertexData {i} in {path:?}: {e}"),
            }
        }

        for (i, model) in self.foliage_models.iter().enumerate() {
            match model.entry.extract(&mut reader, compressed) {
                Ok(model) => {
                    model.vertex_data.check_file(path, &[], &[], false);
                    for texture in model.textures.textures {
                        let mibl = Mibl::from_bytes(&texture.mibl_data).unwrap();
                        mibl.check_file(path, &texture.mibl_data, &[], check_read_write);
                    }
                }
                Err(e) => println!("Error extracting foliage model {i} in {path:?}: {e}"),
            }
        }

        for entry in self.prop_positions {
            entry.extract(&mut reader, compressed).unwrap();
        }

        for entry in self.low_textures {
            let entry = entry.extract(&mut reader, compressed).unwrap();
            for texture in entry.textures {
                Mibl::from_bytes(&texture.mibl_data).unwrap();
            }
        }

        for (i, model) in self.low_models.iter().enumerate() {
            match model.entry.extract(&mut reader, compressed) {
                Ok(model) => {
                    model.vertex_data.check_file(path, &[], &[], false);
                    model.spch.check_file(path, &[], &[], false);
                }
                Err(e) => println!("Error extracting low model {i} in {path:?}: {e}"),
            }
        }

        for entry in self.unk_foliage_data {
            entry.extract(&mut reader, compressed).unwrap();
        }

        for (i, entry) in self.map_vertex_data.iter().enumerate() {
            match entry.extract(&mut reader, compressed) {
                Ok(vertex_data) => {
                    let original_bytes = entry.decompress(&mut reader, compressed).unwrap();
                    vertex_data.check_file(path, &original_bytes, &[], check_read_write);
                }
                Err(e) => println!("Error extracting map VertexData {i} in {path:?}: {e}"),
            }
        }
    }
}

impl CheckFile for Mibl {
    fn check_file(
        self,
        path: &Path,
        original_bytes: &[u8],
        _original_ranges: &[OffsetRange],
        check_read_write: bool,
    ) {
        // DDS should support all MIBL image formats.
        // MIBL <-> DDS should be 1:1.
        let dds = self.to_dds().unwrap();
        let new_mibl = Mibl::from_dds(&dds).unwrap();
        if self != new_mibl {
            println!("Mibl/DDS conversion not 1:1 for {path:?}");
        }

        if check_read_write && self.to_bytes().unwrap() != original_bytes {
            println!("Mibl read/write not 1:1 for {path:?}");
        }
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

impl CheckFile for Wilay {
    fn check_file(
        self,
        path: &Path,
        original_bytes: &[u8],
        original_ranges: &[OffsetRange],
        check_read_write: bool,
    ) {
        match self {
            Wilay::Dhal(dhal) => check_dhal(
                dhal,
                path,
                original_bytes,
                original_ranges,
                check_read_write,
            ),
            Wilay::Lagp(lagp) => check_lagp(
                lagp,
                path,
                original_bytes,
                original_ranges,
                check_read_write,
            ),
            Wilay::Laps(laps) => check_laps(
                laps,
                path,
                original_bytes,
                original_ranges,
                check_read_write,
            ),
        }
    }
}

fn check_dhal(
    dhal: Dhal,
    path: &Path,
    original_bytes: &[u8],
    original_ranges: &[OffsetRange],
    check_read_write: bool,
) {
    if let Some(textures) = &dhal.textures {
        for texture in &textures.textures {
            let mibl = Mibl::from_bytes(&texture.mibl_data).unwrap();
            mibl.check_file(path, &texture.mibl_data, &[], check_read_write);
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
        let bytes = dhal.to_bytes().unwrap();
        if bytes != original_bytes {
            println!("Dhal read/write not 1:1 for {path:?}");

            // Compare offset ranges to better explain differences.
            let (_, new_ranges) = read_type_get_offsets::<Dhal>(&bytes, Endian::Little);
            validate_offset_write_order(original_ranges, &new_ranges, path);
        }
    }
}

fn check_lagp(
    lagp: Lagp,
    path: &Path,
    original_bytes: &[u8],
    original_ranges: &[OffsetRange],
    check_read_write: bool,
) {
    if let Some(textures) = &lagp.textures {
        for texture in &textures.textures {
            let mibl = Mibl::from_bytes(&texture.mibl_data).unwrap();
            mibl.check_file(path, &texture.mibl_data, &[], check_read_write);
        }
    }

    if check_read_write {
        let bytes = lagp.to_bytes().unwrap();
        if bytes != original_bytes {
            println!("Lagp read/write not 1:1 for {path:?}");

            // Compare offset ranges to better explain differences.
            let (_, new_ranges) = read_type_get_offsets::<Lagp>(&bytes, Endian::Little);
            validate_offset_write_order(original_ranges, &new_ranges, path);
        }
    }
}

fn check_laps(
    laps: Laps,
    path: &Path,
    original_bytes: &[u8],
    _original_ranges: &[OffsetRange],
    check_read_write: bool,
) {
    if check_read_write && laps.to_bytes().unwrap() != original_bytes {
        println!("Laps read/write not 1:1 for {path:?}");
    }
}

#[derive(BinRead)]
enum Wimdo {
    Mxmd(Box<Mxmd>),
    Apmd(Apmd),
}

impl CheckFile for Wimdo {
    fn check_file(
        self,
        path: &Path,
        original_bytes: &[u8],
        _original_ranges: &[OffsetRange],
        check_read_write: bool,
    ) {
        match self {
            Wimdo::Mxmd(mxmd) => {
                mxmd.check_file(path, original_bytes, &[], check_read_write);
            }
            Wimdo::Apmd(apmd) => {
                for entry in &apmd.entries {
                    // TODO: check inner data.
                    match entry.read_data() {
                        Ok(data) => match data {
                            xc3_lib::apmd::EntryData::Mxmd(mxmd) => {
                                mxmd.check_file(path, &entry.entry_data, &[], check_read_write)
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

                if check_read_write && apmd.to_bytes().unwrap() != original_bytes {
                    println!("Apmd read/write not 1:1 for {path:?}");
                }
            }
        }
    }
}

impl CheckFile for Mxmd {
    fn check_file(
        self,
        path: &Path,
        original_bytes: &[u8],
        original_ranges: &[OffsetRange],
        check_read_write: bool,
    ) {
        if check_read_write {
            let bytes = self.to_bytes().unwrap();
            if bytes != original_bytes {
                println!("Mxmd read/write not 1:1 for {path:?}");

                // Compare offset ranges to better explain differences.
                let (_, new_ranges) = read_type_get_offsets::<Self>(&bytes, Endian::Little);
                validate_offset_write_order(original_ranges, &new_ranges, path);
            }
        }

        match self.inner {
            xc3_lib::mxmd::MxmdInner::V40(mxmd) => {
                if let Some(spco) = mxmd.shaders {
                    // TODO: Check read/write for inner data?
                    for item in spco.items {
                        item.spch.check_file(path, &[], &[], false);
                    }
                }

                if let Some(packed_textures) = &mxmd.packed_textures {
                    for texture in &packed_textures.textures {
                        match Mibl::from_bytes(&texture.mibl_data) {
                            Ok(mibl) => {
                                mibl.check_file(path, &texture.mibl_data, &[], check_read_write)
                            }
                            Err(e) => println!("Error reading Mibl in {path:?}: {e}"),
                        }
                    }
                }
            }
            xc3_lib::mxmd::MxmdInner::V111(_mxmd) => {}
            xc3_lib::mxmd::MxmdInner::V112(mxmd) => {
                if !is_valid_models_flags(&mxmd) {
                    println!("Inconsistent ModelsFlags for {path:?}");
                }

                if let Some(spch) = mxmd.spch {
                    // TODO: Check read/write for inner data?
                    spch.check_file(path, &[], &[], false);
                }

                if let Some(packed_textures) = &mxmd.packed_textures {
                    for texture in &packed_textures.textures {
                        match Mibl::from_bytes(&texture.mibl_data) {
                            Ok(mibl) => {
                                mibl.check_file(path, &texture.mibl_data, &[], check_read_write)
                            }
                            Err(e) => println!("Error reading Mibl in {path:?}: {e}"),
                        }
                    }
                }
            }
        }
    }
}

fn is_valid_models_flags(mxmd: &MxmdV112) -> bool {
    // Check that flags are consistent with nullability of offsets.
    let flags = mxmd.models.models_flags;
    flags.has_model_unk8() == mxmd.models.model_unk8.is_some()
        && flags.has_model_unk7() == mxmd.models.model_unk7.is_some()
        && flags.has_morph_controllers() == mxmd.models.morph_controllers.is_some()
        && flags.has_model_unk1() == mxmd.models.model_unk1.is_some()
        && flags.has_skinning() == mxmd.models.skinning.is_some()
        && flags.has_lod_data() == mxmd.models.lod_data.is_some()
        && flags.has_alpha_table() == mxmd.models.alpha_table.is_some()
}

impl CheckFile for Spch {
    fn check_file(
        self,
        path: &Path,
        original_bytes: &[u8],
        _original_ranges: &[OffsetRange],
        check_read_write: bool,
    ) {
        for (i, slct_offset) in self.slct_offsets.iter().enumerate() {
            match slct_offset.read_slct(&self.slct_section) {
                Ok(_) => {
                    // TODO: Check that the extracted binary sizes add up to the total size.
                    // TODO: check constant buffer size.
                }
                Err(e) => println!("Error reading Slct {i} for {path:?}: {e}"),
            }
        }

        if check_read_write && self.to_bytes().unwrap() != original_bytes {
            println!("Spch read/write not 1:1 for {path:?}");
        }
    }
}

impl CheckFile for Ltpc {
    fn check_file(
        self,
        path: &Path,
        original_bytes: &[u8],
        _original_ranges: &[OffsetRange],
        check_read_write: bool,
    ) {
        if check_read_write && self.to_bytes().unwrap() != original_bytes {
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

impl CheckFile for Sar1 {
    fn check_file(
        self,
        path: &Path,
        original_bytes: &[u8],
        _original_ranges: &[OffsetRange],
        check_read_write: bool,
    ) {
        for entry in &self.entries {
            if xc3_lib::hash::hash_str_crc(&entry.name) != entry.name_hash {
                println!("Incorrect hash for {:?}", entry.name);
            }

            // Check read/write for the inner data.
            let mut reader = Cursor::new(&entry.entry_data);
            match reader.read_le() {
                Ok(data) => match data {
                    Sar1EntryData::Bc(bc) => {
                        if check_read_write && bc.to_bytes().unwrap() != entry.entry_data {
                            println!("Bc read/write not 1:1 for {:?} in {path:?}", entry.name);
                        }
                        bc.check_file(path, &[], &[], false);
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
                        if check_read_write && eva.to_bytes().unwrap() != original_bytes {
                            println!("Eva read/write not 1:1 for {:?} in {path:?}", entry.name);
                        }
                    }
                },
                Err(e) => println!("Error reading {:?} in {path:?}: {e}", entry.name),
            }
        }

        if check_read_write && self.to_bytes().unwrap() != original_bytes {
            println!("Sar1 read/write not 1:1 for {path:?}");
        }
    }
}

impl CheckFile for Bc {
    fn check_file(
        self,
        path: &Path,
        original_bytes: &[u8],
        _original_ranges: &[OffsetRange],
        check_read_write: bool,
    ) {
        if check_read_write && self.to_bytes().unwrap() != original_bytes {
            println!("Bc read/write not 1:1 for {path:?}");
        }

        match self.data {
            xc3_lib::bc::BcData::Skdy(_) => (),
            xc3_lib::bc::BcData::Anim(_) => (),
            xc3_lib::bc::BcData::Skel(_) => (),
            xc3_lib::bc::BcData::Asmb(asmb) => match asmb.inner {
                xc3_lib::bc::asmb::AsmbInner::V1(_) => (),
                xc3_lib::bc::asmb::AsmbInner::V2(v2) => {
                    for entry in v2.unk2.elements {
                        for e1 in entry.unk1.elements {
                            if xc3_lib::hash::murmur3(e1.value.name.as_bytes())
                                != e1.value.name_hash
                            {
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
                            if xc3_lib::hash::murmur3(e2.value.name2.as_bytes())
                                != e2.value.name2_hash
                            {
                                println!("Incorrect hash for {:?}", e2.value.name2);
                            }
                        }
                    }
                }
            },
        }
    }
}

impl CheckFile for Eva {
    fn check_file(
        self,
        path: &Path,
        original_bytes: &[u8],
        _original_ranges: &[OffsetRange],
        check_read_write: bool,
    ) {
        if check_read_write && self.to_bytes().unwrap() != original_bytes {
            println!("Eva read/write not 1:1 for {path:?}");
        }
    }
}

impl CheckFile for Beb {
    fn check_file(
        self,
        path: &Path,
        original_bytes: &[u8],
        _original_ranges: &[OffsetRange],
        check_read_write: bool,
    ) {
        if check_read_write && self.to_bytes().unwrap() != original_bytes {
            println!("Beb read/write not 1:1 for {path:?}");
        }

        for (i, offset) in self.xbc1_offsets.iter().enumerate() {
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
                                    Ok(bc) => {
                                        bc.check_file(path, entry_bytes, &[], check_read_write)
                                    }
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
}

impl CheckFile for Beh {
    fn check_file(
        self,
        path: &Path,
        original_bytes: &[u8],
        _original_ranges: &[OffsetRange],
        check_read_write: bool,
    ) {
        if check_read_write && self.to_bytes().unwrap() != original_bytes {
            println!("Beh read/write not 1:1 for {path:?}");
        }
    }
}

impl CheckFile for Efb0 {
    fn check_file(
        self,
        path: &Path,
        original_bytes: &[u8],
        _original_ranges: &[OffsetRange],
        check_read_write: bool,
    ) {
        if check_read_write && self.to_bytes().unwrap() != original_bytes {
            println!("efb0 read/write not 1:1 for {path:?}");
        }
    }
}

impl CheckFile for Idcm {
    fn check_file(
        self,
        path: &Path,
        original_bytes: &[u8],
        _original_ranges: &[OffsetRange],
        check_read_write: bool,
    ) {
        if check_read_write && self.to_bytes().unwrap() != original_bytes {
            println!("Idcm read/write not 1:1 for {path:?}");
        }
    }
}

impl CheckFile for MxmdLegacy {
    fn check_file(
        self,
        path: &Path,
        _original_bytes: &[u8],
        _original_ranges: &[OffsetRange],
        check_read_write: bool,
    ) {
        if let Some(textures) = self.packed_textures {
            for texture in textures.textures {
                match Mtxt::from_bytes(&texture.mtxt_data) {
                    Ok(mtxt) => mtxt.check_file(path, &texture.mtxt_data, &[], check_read_write),
                    Err(e) => println!("Error reading Mtxt in {path:?}: {e}"),
                }
            }
        }

        for shader in self.shaders.shaders {
            match Mths::from_bytes(&shader.mths_data) {
                Ok(mths) => {
                    mths.check_file(path, &shader.mths_data, &[], check_read_write);
                }
                Err(e) => println!("Error reading Mths in {path:?}: {e}"),
            }
        }

        // TODO: check read/write for camdo?
        // TODO: Also test loading casmt data?

        for buffer in self.vertex.vertex_buffers {
            for a in buffer.attributes {
                if a.data_type.size_in_bytes() != a.data_size as usize {
                    println!("Unexpected size {} for {:?}", a.data_size, a.data_type);
                }
            }
        }
    }
}

impl CheckFile for Mths {
    fn check_file(
        self,
        path: &Path,
        _original_bytes: &[u8],
        _original_ranges: &[OffsetRange],
        _check_read_write: bool,
    ) {
        if let Err(e) = self.vertex_shader() {
            println!("Error reading vertex shader in {path:?}: {e}")
        }
        if let Err(e) = self.pixel_shader() {
            println!("Error reading fragment shader in {path:?}: {e}")
        }
    }
}

impl CheckFile for Mtxt {
    fn check_file(
        self,
        path: &Path,
        original_bytes: &[u8],
        _original_ranges: &[OffsetRange],
        check_read_write: bool,
    ) {
        // TODO: Multiple mtxt for caavp files.
        if check_read_write {
            let mut writer = Cursor::new(Vec::new());
            self.write(&mut writer).unwrap();
            if writer.into_inner() != original_bytes {
                println!("Mtxt read/write not 1:1 for {path:?}");
            }
            // TODO: Check read/write for dds?
        }
        if let Err(e) = self.deswizzled_image_data() {
            println!(
                "Error deswizzling surface for {path:?}: {e}\n{:#?}",
                self.footer
            );
        }
    }
}

impl CheckFile for Bmn {
    fn check_file(
        self,
        path: &Path,
        _original_bytes: &[u8],
        _original_ranges: &[OffsetRange],
        check_read_write: bool,
    ) {
        if let Some(unk16) = self.unk16 {
            for texture in unk16.textures {
                if !texture.mtxt_data.is_empty() {
                    match Mtxt::from_bytes(&texture.mtxt_data) {
                        Ok(mtxt) => {
                            mtxt.check_file(path, &texture.mtxt_data, &[], check_read_write)
                        }
                        Err(e) => println!("Error reading Mtxt in {path:?}: {e}"),
                    }
                }
            }
        }
    }
}

impl CheckFile for Laft {
    fn check_file(
        self,
        path: &Path,
        original_bytes: &[u8],
        _original_ranges: &[OffsetRange],
        check_read_write: bool,
    ) {
        if !self.mappings.len().is_power_of_two() {
            println!("Laft: mappings len not power of 2 for {path:?}");
        }
        let read_glyphs = (0..u16::MAX)
            .filter(|&code| self.get_glyph(code).is_some_and(|g| g.1.codepoint == code))
            .count();
        if read_glyphs != self.font_info.len() {
            println!("Laft: found unreachable glyphs in {path:?}");
        }
        if let Some(texture) = self.texture.clone() {
            texture.check_file(path, &[], &[], false);
        }

        if check_read_write && self.to_bytes().unwrap() != original_bytes {
            println!("Laft read/write not 1:1 for {path:?}");
        }
    }
}

impl CheckFile for Last {
    fn check_file(
        self,
        path: &Path,
        original_bytes: &[u8],
        _original_ranges: &[OffsetRange],
        check_read_write: bool,
    ) {
        if check_read_write && self.to_bytes().unwrap() != original_bytes {
            println!("Last read/write not 1:1 for {path:?}");
        }
    }
}

impl CheckFile for Fnt {
    fn check_file(
        self,
        path: &Path,
        original_bytes: &[u8],
        _original_ranges: &[OffsetRange],
        check_read_write: bool,
    ) {
        if self.font.get_glyph_by_utf16(0x2a).is_none() {
            println!(
                "{path:?} has no \"*\" character registered! The game will crash on unsupported characters."
            );
        }
        if check_read_write && !write_be_bytes_equals(&self, original_bytes) {
            println!("Fnt read/write not 1:1 for {path:?}");
        }
    }
}

fn check_all<T>(root: &Path, patterns: &[&str], endian: Endian, check_read_write: bool)
where
    T: CheckFile,
    for<'a> T: BinRead<Args<'a> = ()>,
{
    globwalk::GlobWalkerBuilder::from_patterns(root, patterns)
        .build()
        .unwrap()
        .par_bridge()
        .for_each(|entry| {
            let path = entry.as_ref().unwrap().path();
            let original_bytes = std::fs::read(path).unwrap();

            let (result, ranges) = read_type_get_offsets(&original_bytes, endian);

            match result {
                Ok(file) => T::check_file(file, path, &original_bytes, &ranges, check_read_write),
                Err(e) => println!("Error reading {path:?}: {e}"),
            }

            // There may be many validation errors, so only print a summary.
            let errors = xc3_lib::offset::validate_ranges(&ranges, &original_bytes);
            if !errors.is_empty() {
                let mut gap_count = 0;
                let mut overlap_count = 0;
                for e in errors {
                    match e {
                        OffsetValidationError::OverlappingRange { .. } => overlap_count += 1,
                        OffsetValidationError::GapWithNonPaddingBytes { .. } => gap_count += 1,
                    }
                }
                if gap_count > 0 {
                    println!("GapWithNonPaddingBytes: {gap_count}, {path:?}");
                }
                if overlap_count > 0 {
                    println!("OverlappingRange: {overlap_count}, {path:?}");
                }
            }
        });
}

fn check_all_gltf(root: &Path) {
    // Assume root is the dump root path.
    let shader_textures = ShaderTextures::from_folder(root.join("monolib/shader"));

    globwalk::GlobWalkerBuilder::from_patterns(root, &["*.{wimdo}"])
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

    globwalk::GlobWalkerBuilder::from_patterns(root, &["*.{camdo}"])
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
    globwalk::GlobWalkerBuilder::from_patterns(root, &["*.{wismhd}"])
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

fn check_all_wimdo_model(root: &Path, check_read_write: bool, database: Option<String>) {
    let database = database.map(|p| ShaderDatabase::from_file(p).unwrap());

    globwalk::GlobWalkerBuilder::from_patterns(root, &["*.{wimdo}"])
        .build()
        .unwrap()
        .par_bridge()
        .for_each(|entry| {
            let path = entry.as_ref().unwrap().path();
            let wismt_path = path.with_extension("wismt");
            let chr = chr_folder(path);

            let model_name = path.file_stem().unwrap_or_default().to_string_lossy();
            let skel = load_skel(path, &model_name);

            // Test reimporting models without any changes.
            // Avoid compressing or decompressing data more than once for performance.
            match Mxmd::from_file(path) {
                Ok(mxmd) => match mxmd.inner {
                    xc3_lib::mxmd::MxmdInner::V40(mxmd) => {
                        match ModelFilesV40::from_files(&mxmd, &wismt_path, chr.as_deref()) {
                            Ok(files) => {
                                match ModelRoot::from_mxmd_v40(&files, skel, database.as_ref()) {
                                    Ok(root) => {
                                        check_shader_dependencies(&root, path);

                                        if check_read_write {
                                            check_wimdo_v40_export(
                                                root,
                                                &mxmd,
                                                &files.vertex,
                                                path,
                                            );
                                        }
                                    }
                                    Err(e) => println!("Error loading {path:?}: {e}"),
                                }
                            }
                            Err(e) => println!("Error loading files from {path:?}: {e}"),
                        }
                    }
                    xc3_lib::mxmd::MxmdInner::V111(mxmd) => {
                        match ModelFilesV111::from_files(&mxmd, &wismt_path, chr.as_deref(), false)
                        {
                            Ok(files) => match ModelRoot::from_mxmd_v111(&files, skel, None) {
                                Ok(_root) => {
                                    // v111 is rarely used and not worth rebuilding with xc3_model.
                                }
                                Err(e) => println!("Error loading {path:?}: {e}"),
                            },
                            Err(e) => println!("Error loading files from {path:?}: {e}"),
                        }
                    }
                    xc3_lib::mxmd::MxmdInner::V112(mxmd) => {
                        match ModelFilesV112::from_files(&mxmd, &wismt_path, chr.as_deref(), false)
                        {
                            Ok(files) => {
                                match ModelRoot::from_mxmd_v112(&files, skel, database.as_ref()) {
                                    Ok(root) => {
                                        check_shader_dependencies(&root, path);

                                        if check_read_write {
                                            check_wimdo_v112_export(
                                                root,
                                                &mxmd,
                                                &files.vertex,
                                                path,
                                            );
                                        }
                                    }
                                    Err(e) => println!("Error loading {path:?}: {e}"),
                                }
                            }
                            Err(e) => println!("Error loading files from {path:?}: {e}"),
                        }
                    }
                },
                Err(e) => println!("Error reading {path:?}: {e}"),
            }
        });
}

fn check_shader_dependencies(root: &ModelRoot, path: &Path) {
    for m in &root.models.materials {
        if let Some(shader) = &m.shader {
            for (k, v) in &shader.output_dependencies {
                if has_unsupported_values(&shader.exprs, *v) {
                    println!(
                        "Unsupported operations for {:?}, {k:?}, technique {}, {path:?}",
                        &m.name, m.technique_index
                    );
                }
            }
        }
    }
}

fn has_unsupported_values(exprs: &[xc3_model::shader_database::OutputExpr], i: usize) -> bool {
    match &exprs[i] {
        xc3_model::shader_database::OutputExpr::Value(_) => false,
        xc3_model::shader_database::OutputExpr::Func { op, args } => {
            *op == xc3_model::shader_database::Operation::Unk
                || args.iter().any(|a| has_unsupported_values(exprs, *a))
        }
    }
}

fn check_wimdo_v40_export(
    root: xc3_model::ModelRoot,
    mxmd: &MxmdV40,
    vertex: &xc3_lib::mxmd::legacy::VertexData,
    path: &Path,
) {
    let (new_mxmd, new_vertex, _) = root.to_mxmd_v40_model_files(mxmd).unwrap();
    // TODO: Check rebuilding vertex and mxmd fields 1:1
    if &new_vertex != vertex {
        println!("VertexData not 1:1 for {path:?}");
    }

    // TODO: How many of these fields should be preserved?
    if new_mxmd.models.models != mxmd.models.models {
        println!("Model list not 1:1 for {path:?}");
    }
}

fn check_wimdo_v112_export(
    root: xc3_model::ModelRoot,
    mxmd: &MxmdV112,
    vertex: &xc3_lib::vertex::VertexData,
    path: &Path,
) {
    let (new_mxmd, new_vertex, _) = root.to_mxmd_v112_model_files(mxmd).unwrap();
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
    if new_mxmd.models.lod_data != mxmd.models.lod_data {
        println!("Model LODs not 1:1 for {path:?}");
    }
    if let Some(skinning) = &mxmd.models.skinning
        && let Some(new_skinning) = &new_mxmd.models.skinning
    {
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
                !Mat4::from_cols_array_2d(m1).relative_eq(&Mat4::from_cols_array_2d(m2), 0.5, 0.1)
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

fn check_all_animations(root: &Path, _check_read_write: bool) {
    globwalk::GlobWalkerBuilder::from_patterns(root, &["*.{mot, anm, motstm_data}"])
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

fn check_all_collisions(root: &Path, _check_read_write: bool) {
    globwalk::GlobWalkerBuilder::from_patterns(root, &["*.{wiidcm, idcm}"])
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
    T: WriteFull<Args = ()>,
{
    let mut writer = Cursor::new(Vec::new());
    value
        .write_full(&mut writer, 0, &mut 0, xc3_write::Endian::Little, ())
        .unwrap();
    writer.into_inner() == original_bytes
}

fn write_be_bytes_equals<T>(value: &T, original_bytes: &[u8]) -> bool
where
    T: WriteFull<Args = ()>,
{
    let mut writer = Cursor::new(Vec::new());
    value
        .write_full(&mut writer, 0, &mut 0, xc3_write::Endian::Big, ())
        .unwrap();
    writer.into_inner() == original_bytes
}

fn validate_offset_write_order(
    original_ranges: &[OffsetRange],
    new_ranges: &[OffsetRange],
    path: &Path,
) {
    // Ranges are already sorted by start offset, so we can validate write order.
    for (old, new) in original_ranges.iter().zip(new_ranges) {
        if old.type_name != new.type_name || old.parent_type_names != new.parent_type_names {
            println!("Incorrect offset write order for {path:?}");
            break;
        }
    }
}
