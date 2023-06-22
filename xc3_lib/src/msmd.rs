//! `.wismhd` files for map data that points to data in a corresponding `.wismda` files
use std::{
    io::{Cursor, Read, Seek, SeekFrom},
    marker::PhantomData,
};

use binrw::{binread, BinRead, FilePtr32};

use crate::{
    map::{FoliageModelData, MapModelData, PropModelData, SkyModelData},
    mibl::Mibl,
    parse_count_offset, parse_count_offset2, parse_ptr32, parse_string_ptr32,
    vertex::VertexData,
    xbc1::Xbc1,
};

// TODO: Is it worth implementing serialize?
#[binread]
#[derive(Debug)]
#[br(magic(b"DMSM"))]
pub struct Msmd {
    version: u32,
    // TODO: always 0?
    unk1: [u32; 4],

    #[br(parse_with = parse_count_offset)]
    pub map_models: Vec<MapModel>,

    #[br(parse_with = parse_count_offset)]
    pub prop_models: Vec<PropModel>,

    unk1_1: [u32; 2],

    #[br(parse_with = parse_count_offset)]
    pub unk_models: Vec<SkyModel>,

    #[br(parse_with = FilePtr32::parse)]
    unk_offset: Unk,

    unk2_1: u32,

    #[br(parse_with = parse_ptr32)]
    effects: Option<Effects>,

    unk2: [u32; 3],

    /// References to [VertexData](crate::vertex::VertexData).
    // TODO: xbc1 names like "/seamwork/inst/mdl/00003.te"
    #[br(parse_with = parse_count_offset)]
    pub prop_vertex_data: Vec<StreamEntry<VertexData>>,

    // TODO: What do these do?
    #[br(parse_with = parse_count_offset)]
    pub textures: Vec<Texture>,

    strings_offset: u32,

    #[br(parse_with = parse_count_offset)]
    pub foliage_models: Vec<FoliageModel>,

    // Prop positions?
    /// TODO: xbc1 names like "/seamwork/inst/pos/00000.et"
    #[br(parse_with = parse_count_offset)]
    pub prop_positions: Vec<StreamEntry<()>>, // TODO: type for this?

    // TODO: xbc1 names like "/seamwork/mpfmap/poli//0022"?
    // TODO: Buffers of floats but not structured like VertexData?
    #[br(parse_with = parse_count_offset)]
    pub foliage_data: Vec<StreamEntry<()>>, // TODO: type for this?

    unk3: [u32; 5],

    // low resolution texture?
    #[br(parse_with = parse_count_offset)]
    pub low_textures: Vec<StreamEntry<()>>, // TODO: type for this?

    unk3_1: [u32; 8],

    #[br(parse_with = parse_count_offset)]
    pub unk_models2: Vec<UnkModel2>,

    unk3_2: u32,

    // TODO: xbc1 names like "/seamwork/mpfmap/poli//0000"?
    // TODO: Small buffers with offsets ands counts?
    #[br(parse_with = parse_count_offset)]
    pub unk_foliage_data: Vec<StreamEntry<()>>,

    /// References to [VertexData](crate::vertex::VertexData).
    // TODO: xbc1 names like "/seamwork/basemap/poli//000"
    #[br(parse_with = parse_count_offset)]
    pub map_vertex_data: Vec<StreamEntry<VertexData>>,

    #[br(parse_with = FilePtr32::parse)]
    nerd: Nerd,

    unk4: [u32; 3],

    #[br(parse_with = FilePtr32::parse)]
    ibl: Ibl,

    unk5_1: u32,
    unk5_2: u32,
    unk5_3: u32,

    #[br(parse_with = parse_ptr32)]
    unk5_4: Option<Doce>,

    unk5_5: u32,
    unk5_6: u32,

    // padding?
    unk6: [u32; 8],
}

/// References to medium and high resolution [Mibl](crate::mibl::Mibl) textures.
#[binread]
#[derive(Debug)]
pub struct Texture {
    pub mid: StreamEntry<Mibl>,
    pub high: StreamEntry<Mibl>,
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
    // TODO: xbc1 names like "bina_basefix.temp_wi"?
    pub entry: StreamEntry<MapModelData>,
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
    // TODO: xbc1 names like "/seamwork/inst/out/00000.te"?
    pub entry: StreamEntry<PropModelData>,
    pub unk3: u32,
}

#[binread]
#[derive(Debug)]
pub struct SkyModel {
    pub bounds: BoundingBox,
    // bounding sphere?
    pub unk2: [f32; 4],
    // TODO: xbc1 names like "/seamwork/envmap/ma00a/bina"
    pub entry: StreamEntry<SkyModelData>,
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
pub struct UnkModel2 {
    unk1: [f32; 10],
    // TODO: xbc1 names like "/seamwork/lowmap/ma11a/bina"
    pub entry: StreamEntry<()>, // TODO: type for this?
    unk2: u32,
    // TODO: padding?
    unk: [u32; 5],
}

#[binread]
#[derive(Debug)]
pub struct FoliageModel {
    unk1: [f32; 9],
    unk: [u32; 3],
    unk2: f32,
    // TODO: xbc1 names like "/seamwork/mpfmap/ma11a/bina"
    pub entry: StreamEntry<FoliageModelData>,
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
pub struct Ibl {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    #[br(parse_with = parse_count_offset2, args_raw(base_offset))]
    unk1: Vec<IblInner>,

    unk3: u32,
    unk4: u32,
    unk5: u32,
    unk6: u32,
}

#[binread]
#[derive(Debug)]
#[br(import_raw(base_offset: u64))]
pub struct IblInner {
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

#[binread]
#[derive(Debug)]
pub struct Unk {
    wismda_length: u32,
    unks: [u32; 18],
}

#[binread]
#[derive(Debug)]
#[br(stream = r)]
pub struct Effects {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    #[br(parse_with = parse_count_offset, args_raw(base_offset))]
    unk1: Vec<Effect>,

    unk3: u32,
}

#[binread]
#[derive(Debug)]
#[br(stream = r)]
pub struct Effect {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    #[br(parse_with = parse_string_ptr32, args(base_offset))]
    unk1: String,

    #[br(parse_with = parse_count_offset, args_raw(base_offset))]
    unk2: Vec<[[f32; 4]; 4]>, // TODO: transforms?

    unk4: u32,
    unk5: u32,
    unk6: f32,
    unk7: f32,
    unk8: f32,
    unk9: f32,
    unk10: u32,
    unk11: u32,
    unk12: u32,
    unk13: u32,
    unk14: u32,
    unk15: u32,
    unk16: u32,
}

// TODO: What does this do?
// 116 bytes including magic?
#[binread]
#[derive(Debug)]
#[br(magic(b"DOCE"))]
pub struct Doce {
    version: u32,
    offset: u32,
    count: u32,
}

/// A reference to an [Xbc1](crate::xbc1::Xbc1) in the `.wismda` file.
#[binread]
#[derive(Debug)]
pub struct StreamEntry<T> {
    /// The offset of the [Xbc1](crate::xbc1::Xbc1) in the `.wismda` file.
    pub offset: u32,
    pub decompressed_size: u32,
    phantom: PhantomData<T>,
}

impl<T> StreamEntry<T>
where
    for<'a> T: BinRead<Args<'a> = ()>,
{
    /// Decompress and read the data from a reader for a `.wismda` file.
    pub fn extract<R: Read + Seek>(&self, reader: &mut R) -> T {
        reader.seek(SeekFrom::Start(self.offset as u64)).unwrap();
        let bytes = Xbc1::read(reader).unwrap().decompress().unwrap();
        T::read_le(&mut Cursor::new(bytes)).unwrap()
    }
}
