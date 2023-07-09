use binrw::BinRead;

/// `chr/oj/oj03010100.wimdo` for Xenoblade 3.
#[derive(BinRead, Debug)]
#[br(magic(b"DMPA"))]
pub struct Apmd {
    version: u32,
}
