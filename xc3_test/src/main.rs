use std::{
    io::{BufReader, Cursor},
    path::Path,
};

use binrw::{BinRead, BinReaderExt};
use rayon::prelude::*;
use xc3_lib::{
    dds::{create_dds, create_mibl},
    mibl::Mibl,
    msrd::{DataItemType, Msrd},
    mxmd::Mxmd,
    scph::Spch,
    xcb1::Xbc1,
};

fn check_all_mxmd<P: AsRef<Path>>(root: P) {
    // The map folder is a different format?
    globwalk::GlobWalkerBuilder::from_patterns(root, &["*.wimdo", "!map/**"])
        .build()
        .unwrap()
        .par_bridge()
        .for_each(|entry| {
            let path = entry.as_ref().unwrap().path();
            let mut reader = BufReader::new(std::fs::File::open(path).unwrap());
            // TODO: How to validate this file?
            match Mxmd::read_le(&mut reader) {
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
            let mibl = read_wismt_single_tex(&path);
            check_mibl(mibl);
        });

    let folder = root.as_ref().join("monolib").join("shader");
    globwalk::GlobWalkerBuilder::from_patterns(folder, &["*.{witex,witx}"])
        .build()
        .unwrap()
        .par_bridge()
        .for_each(|entry| {
            let path = entry.as_ref().unwrap().path();
            let mibl = Mibl::from_file(&path).unwrap();
            check_mibl(mibl);
        });
}

fn check_all_msrd<P: AsRef<Path>>(root: P) {
    let folder = root.as_ref().join("chr");

    // The .wismt in the tex folder are just for textures.
    globwalk::GlobWalkerBuilder::from_patterns(folder, &["*.wismt", "!tex/**"])
        .build()
        .unwrap()
        .par_bridge()
        .for_each(|entry| {
            let path = entry.as_ref().unwrap().path();
            let mut reader = BufReader::new(std::fs::File::open(path).unwrap());
            match Msrd::read_le(&mut reader) {
                Ok(msrd) => {
                    let toc_streams: Vec<_> = msrd
                        .tocs
                        .iter()
                        .map(|toc| toc.xbc1.decompress().unwrap())
                        .collect();

                    // TODO: parse remaining embedded files as well
                    for item in msrd.data_items {
                        match item.item_type {
                            DataItemType::ShaderBundle => {
                                let stream = &toc_streams[item.toc_index as usize];
                                let data = &stream[item.offset as usize
                                    ..item.offset as usize + item.size as usize];

                                Spch::read_le(&mut Cursor::new(data)).unwrap();
                            }
                            _ => (),
                        }
                    }
                }
                Err(e) => println!("Error reading {path:?}: {e}"),
            }
        });
}

fn check_mibl(mibl: Mibl) {
    // Check that the mibl can be reconstructed from the dds.
    let dds = create_dds(&mibl).unwrap();
    let new_mibl = create_mibl(&dds).unwrap();

    // Check that the description of the image data remains unchanged.
    if mibl.footer != new_mibl.footer {
        println!("{:?} != {:?}", mibl.footer, new_mibl.footer);
    };

    // TODO: Why does this not work?
    // assert_eq!(mibl.image_data.len(), new_mibl.image_data.len());
}

fn read_wismt_single_tex<P: AsRef<Path>>(path: P) -> Mibl {
    let mut reader = BufReader::new(std::fs::File::open(path).unwrap());
    let xbc1: Xbc1 = reader.read_le().unwrap();

    let decompressed = xbc1.decompress().unwrap();
    let mut reader = Cursor::new(&decompressed);
    reader.read_le_args((decompressed.len(),)).unwrap()
}

fn main() {
    // TODO: clap for args to enable/disable different tests?
    let args: Vec<_> = std::env::args().collect();

    let root = Path::new(&args[1]);

    let start = std::time::Instant::now();

    println!("Checking MIBL files ...");
    check_all_mibl(root);

    println!("Checking MXMD files ...");
    check_all_mxmd(root);

    println!("Checking MSRD files ...");
    check_all_msrd(root);

    // TODO: check shaders

    println!("Finished in {:?}", start.elapsed());
}
