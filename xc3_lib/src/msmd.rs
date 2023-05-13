use std::io::SeekFrom;

use binrw::{args, binread, BinRead, BinResult, FilePtr32, NamedArgs, NullString};
use serde::Serialize;

/// .wismhd files
#[binread]
#[derive(Debug, Serialize)]
#[br(magic(b"DMSM"))]
pub struct Msmd {
    // TODO: implement enough to extract textures
    // TODO: textures in wismda files as well?
}
