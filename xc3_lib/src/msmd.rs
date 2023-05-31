use binrw::binread;
use serde::Serialize;

/// .wismhd files
#[binread]
#[derive(Debug, Serialize)]
#[br(magic(b"DMSM"))]
pub struct Msmd {
    // TODO: implement enough to extract textures
    // TODO: textures in wismda files as well?
}
