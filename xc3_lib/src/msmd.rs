//! `.wismhd` files for map data that points to data in a corresponding `.wismda` files

use binrw::binread;

use crate::parse_count_offset;

// TODO: Is it worth implementing serialize?
#[binread]
#[derive(Debug)]
#[br(magic(b"DMSM"))]
pub struct Msmd {
    version: u32,
    unk1: [u32; 6],

    // TODO: Better name for this?
    // Objects?
    #[br(parse_with = parse_count_offset)]
    pub prop_defs: Vec<PropDef>,

    unk2: [u32; 12],

    #[br(parse_with = parse_count_offset)]
    pub textures: Vec<Texture>,
    unk3: [u32; 27],

    /// References to [ModelData](crate::model::ModelData).
    #[br(parse_with = parse_count_offset)]
    pub model_data: Vec<StreamEntry>,
}

/// A reference to an [Xbc1](crate::xbc1::Xbc1) in the `.wismda` file.
#[binread]
#[derive(Debug)]
pub struct StreamEntry {
    /// The offset of [Xbc1](crate::xbc1::Xbc1) in the `.wismda` file.
    pub offset: u32,
    pub decompressed_size: u32,
}

/// References to medium and high resolution [Mibl](crate::mibl::Mibl) textures.
#[binread]
#[derive(Debug)]
pub struct Texture {
    pub mid: StreamEntry,
    pub high: StreamEntry,
    unk1: u32,
}

// TODO: Better name for this?
#[binread]
#[derive(Debug)]
pub struct PropDef {
    pub bounds: BoundingBox,
    pub unk2: [f32; 4],
    // TODO: What kind of data is this?
    pub entry: StreamEntry,
    pub unk3: u32,
}

// TODO: also in mxmd but without the center?
#[binread]
#[derive(Debug)]
pub struct BoundingBox {
    max: [f32; 3],
    min: [f32; 3],
    center: [f32; 3],
}
