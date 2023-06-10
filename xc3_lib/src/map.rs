//! Map data stored in compressed sections in `.wismda` files.
//!
//! Many of these sections use the same formats as character models.

use binrw::{binread, FilePtr32};

use crate::mxmd::{Materials, Mesh};

// TODO: Link to appropriate fields with doc links.
/// The data for [PropDef](crate::msmd::PropDef).
#[binread]
#[derive(Debug)]
pub struct PropDefData {
    pub unk1: [u32; 3],
    #[br(parse_with = FilePtr32::parse)]
    pub mesh: Mesh,
    #[br(parse_with = FilePtr32::parse)]
    pub materials: Materials,
}
