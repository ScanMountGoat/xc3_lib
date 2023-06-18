//! `.wismhd` files for map data that points to data in a corresponding `.wismda` files

use binrw::{binread, FilePtr32};

use crate::{parse_count_offset, parse_count_offset2, parse_string_ptr32};

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

    unk_offset: u32,
    unk2: [u32; 5],

    /// References to [VertexData](crate::vertex::VertexData).
    #[br(parse_with = parse_count_offset)]
    pub prop_vertex_data: Vec<StreamEntry>,

    // TODO: What do these do?
    #[br(parse_with = parse_count_offset)]
    pub textures: Vec<Texture>,

    // TODO: This section can have multiple strings?
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

    /// References to [VertexData](crate::vertex::VertexData).
    #[br(parse_with = parse_count_offset)]
    pub map_vertex_data: Vec<StreamEntry>,

    #[br(parse_with = FilePtr32::parse)]
    nerd: Nerd,

    unk4: [u32; 3],

    // some section before the map name?
    #[br(parse_with = FilePtr32::parse)]
    unk_offset2: Unk2,

    // padding?
    unk5: [u32; 14],
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
    /// Reference to [MapModelData](crate::map::MapModelData).
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
    /// Reference to [PropModelData](crate::map::PropModelData).
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

#[binread]
#[derive(Debug)]
#[br(magic(b"DREN"))]
pub struct Nerd {
    version: u32,
    unk1: u32,
    unk2: u32,
    unk3: u32,
    unk4: u32,
    unk5: u32,
    // padding?
    unk6: [u32; 6],
}

#[binread]
#[derive(Debug)]
#[br(stream = r)]
pub struct Unk2 {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    #[br(parse_with = parse_count_offset2, args_raw(base_offset))]
    unk1: Vec<Unk2Inner>,

    unk3: u32,
    unk4: u32,
    unk5: u32,
    unk6: u32,
}

#[binread]
#[derive(Debug)]
#[br(import_raw(base_offset: u64))]
pub struct Unk2Inner {
    unk1: u32, // 0?
    #[br(parse_with = parse_string_ptr32, args(base_offset))]
    map_name: String,
    #[br(parse_with = FilePtr32::parse, offset = base_offset)]
    gibl: Gibl,
    unk4: u32, // gibl section length?
    // padding?
    unk5: [u32; 6],
}

#[binread]
#[derive(Debug)]
#[br(magic(b"GIBL"))]
pub struct Gibl {
    unk1: u32,
    unk2: u32,
    unk3: u32,
    unk4: u32, // offset to mibl?
    unk5: u32,
    // TODO: padding?
    unk6: [u32; 6],
}
