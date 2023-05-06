use std::{
    io::{BufReader, BufWriter, Cursor, Read, Seek, SeekFrom},
    path::Path,
};

use anyhow::Result;
use binrw::BinReaderExt;
use drsm::{DataItemType, Drsm, Xbc1};
use flate2::bufread::ZlibDecoder;
use lbim::Libm;

use crate::{dds::create_dds, hpcs::Hpcs, mot::Sar1, model::ModelData};

mod dds;
// TODO: formats module.
// TODO: consistent naming for magics/extensions?
// mibl instead of lbim?
// TODO: Is the pointer placement algorithm similar enough to SSBH?
mod drsm;
mod hpcs;
mod lbim;
mod mot;
// TODO: naming for wismt vertex data?
mod model;

// TODO: xc3_test program to run against the dump using Rayon?
// add basic tests like read/write, surface sizes, etc
// TODO: separate binary project that can export to JSON, PNG, DDS, etc
fn main() {
    // let start = std::time::Instant::now();
    // read_hpcs("shdr.bin");
    read_model("model.bin");
    // read_wismt("ch01012013.wismt").unwrap();
    // read_mot("ch01011000_wp01_reaction.mot");
    // eprintln!("{:?}", start.elapsed());
}

// TODO: Create dedicated error types using thiserror instead of anyhow.

fn read_mot<P: AsRef<Path>>(path: P) {
    let mut reader = BufReader::new(std::fs::File::open(path).unwrap());
    let sar1: Sar1 = reader.read_le().unwrap();
    println!("{:#?}", sar1);
}

fn read_model<P: AsRef<Path>>(path: P) {
    let mut reader = BufReader::new(std::fs::File::open(path).unwrap());
    let hpcs: ModelData = reader.read_le().unwrap();
    println!("{:#?}", hpcs);
}


fn read_hpcs<P: AsRef<Path>>(path: P) {
    let mut reader = BufReader::new(std::fs::File::open(path).unwrap());
    let hpcs: Hpcs = reader.read_le().unwrap();
    println!("{:#?}", hpcs);
}

fn process_chr_wismt<P: AsRef<Path>>(chr_ch: P) {
    for e in std::fs::read_dir(chr_ch).unwrap() {
        let path = e.unwrap().path();
        if path.extension().unwrap().to_str() == Some("wismt") {
            if let Err(e) = read_wismt(&path) {
                println!("Error reading {path:?}: {e}")
            }
        }
    }
}

fn process_tex_nx_wismt<P: AsRef<Path>>(chr_tex_nx_m: P) {
    // TODO: the h directory doesn't have lbim footers?
    for e in std::fs::read_dir(chr_tex_nx_m).unwrap() {
        let path = e.unwrap().path();
        if path.extension().unwrap().to_str() == Some("wismt") {
            let lbim = read_wismt_single_tex(&path);
            println!("{:?},{:?}", path, lbim.footer);
        }
    }
}

fn process_monolib_shader_witex<P: AsRef<Path>>(monolib_shader: P) {
    for entry in std::fs::read_dir(monolib_shader).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().as_ref().and_then(|e| e.to_str()) == Some("witex") {
            match read_witex(&path) {
                Ok(libm) => {
                    let _dds = create_dds(&libm).unwrap();
                    println!("{:?},{:?}", path, libm.footer);
                }
                Err(e) => eprintln!("Error reading {path:?}: {e}"),
            }
        }
    }
}

fn read_witex<P: AsRef<Path>>(path: P) -> Result<Libm> {
    let bytes = std::fs::read(path)?;
    let len = bytes.len();
    let mut reader = Cursor::new(bytes);
    reader.read_le_args((len,)).map_err(Into::into)
}

fn read_wismt_single_tex<P: AsRef<Path>>(path: P) -> Libm {
    let mut reader = BufReader::new(std::fs::File::open(path).unwrap());
    let xbc1: Xbc1 = reader.read_le().unwrap();

    let decompressed = decompress_xbc1(&xbc1);
    // std::fs::write("out.bin", &decompressed).unwrap();
    let mut reader = Cursor::new(&decompressed);
    let lbim: Libm = reader.read_le_args((decompressed.len(),)).unwrap();
    lbim
}

fn read_wismt<P: AsRef<Path>>(path: P) -> Result<()> {
    let path = path.as_ref();

    let mut reader = BufReader::new(std::fs::File::open(path).unwrap());
    let drsm: Drsm = reader.read_le()?;

    let toc_streams: Vec<_> = drsm
        .tocs
        .iter()
        .map(|toc| decompress_xbc1(&toc.xbc1))
        .collect();

    // TODO: add an option to convert textures to PNG or DDS?
    for item in drsm.data_items {
        match item.item_type {
            DataItemType::Model => {
                // TODO
                let stream = &toc_streams[item.toc_index as usize];
                let data = &stream[item.offset as usize..item.offset as usize + item.size as usize];
                std::fs::write("model.bin", data).unwrap();
            }
            DataItemType::ShaderBundle => {
                // TODO: apply hpcs code
                let stream = &toc_streams[item.toc_index as usize];
                let data = &stream[item.offset as usize..item.offset as usize + item.size as usize];
                std::fs::write("shdr.bin", data).unwrap();
            }
            DataItemType::CachedTexture => {
                for info in &drsm.texture_name_table.textures {
                    let mut reader = Cursor::new(&toc_streams[item.toc_index as usize]);

                    let offset = item.offset + info.offset;
                    reader.seek(SeekFrom::Start(offset as u64))?;

                    let size = info.size as usize;

                    let lbim: Libm = reader.read_le_args((size,))?;

                    let estimate = tegra_swizzle::surface::swizzled_surface_size(
                        lbim.footer.width as usize,
                        lbim.footer.height as usize,
                        lbim.footer.depth as usize,
                        lbim.footer.image_format.block_dim(),
                        None,
                        lbim.footer.image_format.bytes_per_pixel(),
                        lbim.footer.mipmap_count as usize,
                        1, // TODO: cube maps?
                    );
                    let estimate_deswizzled = tegra_swizzle::surface::deswizzled_surface_size(
                        lbim.footer.width as usize,
                        lbim.footer.height as usize,
                        lbim.footer.depth as usize,
                        lbim.footer.image_format.block_dim(),
                        lbim.footer.image_format.bytes_per_pixel(),
                        lbim.footer.mipmap_count as usize,
                        1, // TODO: cube maps?
                    );

                    // TODO: is this always rounded up to a multiple of 4096?
                    if estimate != lbim.footer.image_size as usize {
                        println!(
                            "{} != {}, {}, {:?}",
                            estimate,
                            lbim.footer.image_size as usize,
                            estimate_deswizzled,
                            lbim.footer
                        );
                    }

                    if lbim.footer.depth > 1 {
                        // println!("{:?},{:?}", lbim.footer, path);
                        let name = format!(
                            "{}x{}x{}_{:?}.dds",
                            lbim.footer.width,
                            lbim.footer.height,
                            lbim.footer.depth,
                            lbim.footer.image_format
                        );
                        let dds = dds::create_dds(&lbim).unwrap();
                        let mut writer = BufWriter::new(std::fs::File::create(name).unwrap());
                        dds.write(&mut writer).unwrap();
                    }

                    // 0 to 526336 = 526336 bytes
                    // 532480 to 534528 = 2048 bytes
                    // 540672 to 542720 = 2048 bytes
                    // println!("{} == {}", estimate, lbim.footer.image_size);
                }
            }
            DataItemType::Texture => {
                // TODO: Why do we subtract 1 here?
                let mut reader = Cursor::new(&toc_streams[item.toc_index as usize - 1]);

                let offset = item.offset;
                reader.seek(SeekFrom::Start(offset as u64))?;

                let size = item.size as usize;

                // TODO: No header?
                // println!("not cached: {:?}", size);
                // let _lbim: lbim = reader.read_le_args((size,))?;
            }
        }
    }
    Ok(())
}

fn decompress_xbc1(xbc1: &Xbc1) -> Vec<u8> {
    let mut decoder = ZlibDecoder::new(&xbc1.deflate_stream[..]);
    let mut decompressed = vec![0u8; xbc1.decomp_size as usize];
    decoder.read_exact(&mut decompressed).unwrap();
    decompressed
}
