//! `.wismhd` files for map data that points to data in a corresponding `.wismda` files

use binrw::binread;

use crate::{parse_count_offset, parse_string_ptr32};

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

    unk1_1: [u32; 2],

    #[br(parse_with = parse_count_offset)]
    pub unk_models: Vec<SkyModel>,

    unk2: [u32; 6],

    /// References to [ModelData](crate::model::ModelData).
    #[br(parse_with = parse_count_offset)]
    pub prop_model_data: Vec<StreamEntry>,

    // TODO: What do these do?
    #[br(parse_with = parse_count_offset)]
    pub textures: Vec<Texture>,

    #[br(parse_with = parse_string_ptr32)]
    name: String,

    unk_21: u32,
    unk_22: u32,

    // Prop positions?
    #[br(parse_with = parse_count_offset)]
    pub prop_positions: Vec<StreamEntry>,

    unk3: [u32; 7],

    // low resolution texture?
    #[br(parse_with = parse_count_offset)]
    pub low_textures: Vec<StreamEntry>,

    unk3_1: [u32; 13],

    /// References to [ModelData](crate::model::ModelData).
    #[br(parse_with = parse_count_offset)]
    pub map_model_data: Vec<StreamEntry>,
}

/// A reference to an [Xbc1](crate::xbc1::Xbc1) in the `.wismda` file.
#[binread]
#[derive(Debug)]
pub struct StreamEntry {
    /// The offset of the [Xbc1](crate::xbc1::Xbc1) in the `.wismda` file.
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
    // bounding sphere?
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
    // bounding sphere?
    pub unk2: [f32; 4],
    /// Reference to [PropDefData](crate::map::PropDefData).
    pub entry: StreamEntry,
    pub unk3: u32,
}

#[binread]
#[derive(Debug)]
pub struct SkyModel {
    pub bounds: BoundingBox,
    // bounding sphere?
    pub unk2: [f32; 4],
    pub entry: StreamEntry,
}

// TODO: also in mxmd but without the center?
#[binread]
#[derive(Debug)]
pub struct BoundingBox {
    max: [f32; 3],
    min: [f32; 3],
    center: [f32; 3],
}
