use std::{
    io::{BufReader, Cursor},
    path::Path,
};

use binrw::BinReaderExt;
use xc3_lib::{
    dds::{create_dds, create_mibl},
    mibl::Mibl,
    xcb1::Xbc1,
};

fn check_tex_nx_wismt<P: AsRef<Path>>(chr_tex_nx_m: P) {
    // TODO: the h directory doesn't have mibl footers?
    // TODO: rayon
    for e in std::fs::read_dir(chr_tex_nx_m).unwrap() {
        let path = e.unwrap().path();
        if path.extension().unwrap().to_str() == Some("wismt") {
            let mibl = read_wismt_single_tex(&path);

            // Check that the mibl can be reconstructed from the dds.
            let dds = create_dds(&mibl).unwrap();

            let new_mibl = create_mibl(&dds).unwrap();
            assert_eq!(mibl.footer, new_mibl.footer);

            // TODO: Why does this not work?
            // assert_eq!(mibl.image_data.len(), new_mibl.image_data.len());
        }
    }
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
    // TODO: batch test file parsing using glob + rayon
    let args: Vec<_> = std::env::args().collect();

    let root = Path::new(&args[1]);

    let start = std::time::Instant::now();

    let chr_tex_nx_m = root.join("chr").join("tex").join("nx").join("m");

    println!("Checking chr/tex/nx/m/*.wismt...");
    check_tex_nx_wismt(chr_tex_nx_m);

    println!("{:?}", start.elapsed());
}
