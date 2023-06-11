//! `.wismhd` files for map data that points to data in a corresponding `.wismda` files

use binrw::binread;

use crate::parse_count_offset;

// TODO: Is it worth implementing serialize?
#[binread]
#[derive(Debug)]
#[br(magic(b"DMSM"))]
pub struct Msmd {
    version: u32,
    unk1: [u32; 4],

    #[br(parse_with = parse_count_offset)]
    pub map_models: Vec<MapModel>,

    #[br(parse_with = parse_count_offset)]
    pub prop_models: Vec<PropModel>,

    unk2: [u32; 10],

    /// References to [ModelData](crate::model::ModelData).
    #[br(parse_with = parse_count_offset)]
    pub prop_model_data: Vec<StreamEntry>,

    #[br(parse_with = parse_count_offset)]
    pub textures: Vec<Texture>,

    unk3: [u32; 27],

    /// References to [ModelData](crate::model::ModelData).
    #[br(parse_with = parse_count_offset)]
    pub map_model_data: Vec<StreamEntry>,
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
pub struct MapModel {
    pub bounds: BoundingBox,
    pub unk2: [f32; 4],
    /// Reference to [MapDefData](crate::map::MapDefData).
    pub entry: StreamEntry,
    pub unk3: [f32; 4],
}

// TODO: Better name for this?
#[binread]
#[derive(Debug)]
pub struct PropModel {
    pub bounds: BoundingBox,
    pub unk2: [f32; 4],
    /// Reference to [PropDefData](crate::map::PropDefData).
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
