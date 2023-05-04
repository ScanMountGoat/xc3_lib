use std::{
    io::{BufReader, BufWriter, Cursor, Read, Seek, SeekFrom},
    path::Path,
};

use anyhow::Result;
use binrw::BinReaderExt;
use drsm::{DataItemType, Drsm, Xbc1};
use flate2::bufread::ZlibDecoder;
use lbim::Libm;

use crate::hpcs::Hpcs;

mod dds;
// TODO: formats module.
mod drsm;
mod hpcs;
mod lbim;

fn main() {
    read_hpcs("shdr.bin");
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
    // TODO: the h directory doesn't have mibl footers?
    for e in std::fs::read_dir(chr_tex_nx_m).unwrap() {
        let path = e.unwrap().path();
        if path.extension().unwrap().to_str() == Some("wismt") {
            let mibl = read_wismt_single_tex(&path);
            println!("{:?},{:?}", path, mibl.footer);
        }
    }
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
            }
            DataItemType::ShaderBundle => {
                // TODO: apply hpcs code
            }
            DataItemType::CachedTexture => {
                for info in &drsm.texture_name_table.textures {
                    let mut reader = Cursor::new(&toc_streams[item.toc_index as usize]);

                    let offset = item.offset + info.offset;
                    reader.seek(SeekFrom::Start(offset as u64))?;

                    let size = info.size as usize;

                    let mibl: Libm = reader.read_le_args((size,))?;

                    let estimate = tegra_swizzle::surface::swizzled_surface_size(
                        mibl.footer.width as usize,
                        mibl.footer.height as usize,
                        mibl.footer.depth as usize,
                        mibl.footer.image_format.block_dim(),
                        None,
                        mibl.footer.image_format.bytes_per_pixel(),
                        mibl.footer.mipmap_count as usize,
                        1, // TODO: cube maps?
                    );
                    let estimate_deswizzled = tegra_swizzle::surface::deswizzled_surface_size(
                        mibl.footer.width as usize,
                        mibl.footer.height as usize,
                        mibl.footer.depth as usize,
                        mibl.footer.image_format.block_dim(),
                        mibl.footer.image_format.bytes_per_pixel(),
                        mibl.footer.mipmap_count as usize,
                        1, // TODO: cube maps?
                    );

                    // TODO: is this always rounded up to a multiple of 4096?
                    if estimate != mibl.footer.image_size as usize {
                        println!(
                            "{} != {}, {}, {:?}",
                            estimate,
                            mibl.footer.image_size as usize,
                            estimate_deswizzled,
                            mibl.footer
                        );
                    }

                    if mibl.footer.depth > 1 {
                        // println!("{:?},{:?}", mibl.footer, path);
                        let name = format!(
                            "{}x{}x{}_{:?}.dds",
                            mibl.footer.width,
                            mibl.footer.height,
                            mibl.footer.depth,
                            mibl.footer.image_format
                        );
                        let dds = dds::create_dds(&mibl).unwrap();
                        let mut writer = BufWriter::new(std::fs::File::create(name).unwrap());
                        dds.write(&mut writer).unwrap();
                    }

                    // 0 to 526336 = 526336 bytes
                    // 532480 to 534528 = 2048 bytes
                    // 540672 to 542720 = 2048 bytes
                    // println!("{} == {}", estimate, mibl.footer.image_size);
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
                // let _mibl: Mibl = reader.read_le_args((size,))?;
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
