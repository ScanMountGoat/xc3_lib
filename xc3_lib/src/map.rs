//! Map data stored in compressed sections in `.wismda` files.
//!
//! Many of these sections use the same formats as character models.

use binrw::{binread, FilePtr32};

use crate::{
    mxmd::{Materials, Mesh},
    parse_offset_count,
    spch::Spch,
};

// TODO: Same as mxmd?
// TODO: Link to appropriate fields with doc links.
/// The data for a [PropModel](crate::msmd::PropModel).
#[binread]
#[derive(Debug)]
pub struct PropModelData {
    pub unk1: [u32; 3],
    // TODO: nullable pointers?
    #[br(parse_with = FilePtr32::parse)]
    pub mesh: Mesh,
    #[br(parse_with = FilePtr32::parse)]
    pub materials: Materials,
    unk2: [u32; 3],

    #[br(parse_with = parse_offset_count)]
    pub textures: Vec<Texture>,

    unk3: [u32; 7],

    #[br(parse_with = FilePtr32::parse)]
    pub spch: Spch,

    unk4: u32,
    unk5: u32,
    // 16 bytes of padding?
}

// TODO: Link to appropriate fields with doc links.
/// The data for a [MapModel](crate::msmd::MapModel).
#[binread]
#[derive(Debug)]
pub struct MapModelData {
    unk1: [u32; 3],

    // TODO: nullable pointers?
    #[br(parse_with = FilePtr32::parse)]
    pub mesh: Mesh,

    #[br(parse_with = FilePtr32::parse)]
    pub materials: Materials,

    unk2: [u32; 6],

    #[br(parse_with = FilePtr32::parse)]
    pub spch: Spch,

    unk3: [u32; 3]
    // padding?
}

// TODO: Shared with other formats?
#[binread]
#[derive(Debug)]
pub struct Texture {
    low_texture_index: i16,
    low_texture_container_index: i16,
    texture_index: u16,
    texture_type: u16,
}
