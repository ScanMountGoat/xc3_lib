//! `.wismhd` files for map data that points to data in a corresponding `.wismda` files
use std::{
    io::{Cursor, Read, Seek, SeekFrom},
    marker::PhantomData,
};

use binrw::{args, binread, BinRead, FilePtr32};

use crate::{
    map::{
        EnvModelData, FoliageModelData, FoliageUnkData, FoliageVertexData, MapLowModelData,
        MapModelData, PropInstance, PropModelData, PropPositions,
    },
    mibl::Mibl,
    parse_count_offset, parse_count_offset2, parse_offset_count, parse_offset_count2, parse_ptr32,
    parse_string_ptr32,
    vertex::VertexData,
    xbc1::Xbc1,
};

// TODO: Is it worth implementing serialize?
/// The main map data for a `.wismhd` file.
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
    pub env_models: Vec<EnvModel>,

    #[br(parse_with = FilePtr32::parse)]
    unk_offset: Unk,

    unk2_1: u32,

    #[br(parse_with = parse_ptr32)]
    effects: Option<Effects>,

    unk2: [u32; 3],

    /// `.wismda` data with names like `/seamwork/inst/mdl/00003.te`.
    #[br(parse_with = parse_count_offset)]
    pub prop_vertex_data: Vec<StreamEntry<VertexData>>,

    // TODO: What do these do?
    #[br(parse_with = parse_count_offset)]
    pub textures: Vec<Texture>,

    strings_offset: u32,

    #[br(parse_with = parse_count_offset)]
    pub foliage_models: Vec<FoliageModel>,

    /// `.wismda` data with names like `/seamwork/inst/pos/00000.et`.
    #[br(parse_with = parse_count_offset)]
    pub prop_positions: Vec<StreamEntry<PropPositions>>,

    /// `.wismda` data with names like `/seamwork/mpfmap/poli//0022`.
    #[br(parse_with = parse_count_offset)]
    pub foliage_data: Vec<StreamEntry<FoliageVertexData>>,

    unk3_1: u32,
    unk3_2: u32,

    #[br(parse_with = FilePtr32::parse)]
    tgld: Tgld,

    #[br(parse_with = parse_count_offset)]
    pub unk_lights: Vec<UnkLight>,

    // low resolution packed textures?
    /// `.wismda` data with names like `/seamwork/texture/00000_wi`.
    #[br(parse_with = parse_count_offset)]
    pub low_textures: Vec<StreamEntry<LowTextures>>,

    // TODO: Document more of these fields.
    unk4: [u32; 6],

    #[br(parse_with = parse_ptr32)]
    pub parts: Option<MapParts>,

    unk4_2: u32,

    #[br(parse_with = parse_count_offset)]
    pub low_models: Vec<MapLowModel>,

    unk_flags: u32,

    /// `.wismda` data with names like `/seamwork/mpfmap/poli//0000`.
    #[br(parse_with = parse_count_offset)]
    pub unk_foliage_data: Vec<StreamEntry<FoliageUnkData>>,

    /// `.wismda` data with names like `/seamwork/basemap/poli//000`
    /// or `/seamwork/basemap/poli//001`.
    // TODO: Are all of these referenced by map models?
    // TODO: What references "poli/001"?
    #[br(parse_with = parse_count_offset)]
    pub map_vertex_data: Vec<StreamEntry<VertexData>>,

    // TODO: only nerd if flags is 2?
    #[br(parse_with = FilePtr32::parse)]
    nerd: Nerd,

    unk6: [u32; 3],

    #[br(parse_with = FilePtr32::parse)]
    ibl: Ibl,

    #[br(parse_with = parse_ptr32)]
    cmld: Option<Cmld>,

    unk5_2: u32,
    unk5_3: u32,

    #[br(parse_with = parse_ptr32)]
    unk5_4: Option<Doce>,

    unk5_5: u32,
    unk5_6: u32,

    // padding?
    unk7: [u32; 8],
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
    /// `.wismda` data with names like `bina_basefix.temp_wi`.
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
    /// `.wismda` data with names like `/seamwork/inst/out/00000.te`.
    pub entry: StreamEntry<PropModelData>,
    pub unk3: u32,
}

#[binread]
#[derive(Debug)]
pub struct EnvModel {
    pub bounds: BoundingBox,
    // bounding sphere?
    pub unk2: [f32; 4],
    /// `.wismda` data with names like `/seamwork/envmap/ma00a/bina`.
    pub entry: StreamEntry<EnvModelData>,
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
pub struct MapLowModel {
    pub bounds: BoundingBox,
    unk1: f32,
    /// `.wismda` data with names like `/seamwork/lowmap/ma11a/bina`.
    pub entry: StreamEntry<MapLowModelData>,
    unk2: u16,
    unk3: u16,
    // TODO: padding?
    unk: [u32; 5],
}

#[binread]
#[derive(Debug)]
pub struct FoliageModel {
    unk1: [f32; 9],
    unk: [u32; 3],
    unk2: f32,
    /// `.wismda` data with names like `/seamwork/mpfmap/ma11a/bina`.
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

// TODO: cloud data?
#[binread]
#[derive(Debug)]
#[br(magic(b"CMLD"))]
pub struct Cmld {
    version: u32,
}

// TODO: Lighting data?
#[binread]
#[derive(Debug)]
#[br(magic(b"DLGT"))]
pub struct Tgld {
    version: u32,
    unk1: u32,
    unk2: u32,
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
    #[br(parse_with = parse_string_ptr32, args_raw(base_offset))]
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

    #[br(parse_with = parse_string_ptr32, args_raw(base_offset))]
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

#[binread]
#[derive(Debug)]
pub struct LowTextures {
    #[br(parse_with = parse_count_offset)]
    pub textures: Vec<LowTexture>,
    // TODO: Padding?
    unk: [u32; 5],
}

#[binread]
#[derive(Debug)]
pub struct LowTexture {
    unk1: u32,
    // TODO: Optimized function for reading bytes?
    #[br(parse_with = parse_count_offset)]
    pub mibl_data: Vec<u8>,
    unk2: i32,
}

#[binread]
#[derive(Debug)]
pub struct UnkLight {
    max: [f32; 3],
    min: [f32; 3],
    /// `.wismda` data with names like `/seamwork/lgt/bina/00000.wi`.
    pub entry: StreamEntry<Tgld>,
    unk3: u32,
    // TODO: padding?
    unk4: [u32; 5],
}

#[binread]
#[derive(Debug)]
#[br(stream = r)]
pub struct MapParts {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    // TODO: Where do static parts index?
    #[br(parse_with = parse_offset_count2, args_raw(base_offset))]
    pub parts: Vec<MapPart>,

    // TODO: How to select the appropriate prop model?
    #[br(parse_with = parse_count_offset, args_raw(base_offset))]
    pub animated_parts: Vec<PropInstance>,

    #[br(parse_with = FilePtr32::parse)]
    #[br(args { offset: base_offset, inner: args! { count: animated_parts.len(), inner: base_offset }})]
    pub animations: Vec<MapPartInstanceAnimation>,

    unk2: u32,
    unk3: u32,
    unk4: u32,
    unk5: u32,
    unk6: u32,
    unk7: u32,

    #[br(parse_with = parse_offset_count, args_raw(base_offset))]
    pub transforms: Vec<[[f32; 4]; 4]>,
}

#[binread]
#[derive(Debug)]
#[br(import_raw(base_offset: u64))]
pub struct MapPartInstanceAnimation {
    translation: [f32; 3],
    rotation: [f32; 3],
    scale: [f32; 3],
    unk1: u32,
    unk2: u32,
    unk3: u32,
    flags: u32,
    channels_offset: u32,
    channels_count: u32,
    time_min: u16,
    time_max: u16,
    // TODO: padding?
    unks: [u32; 5],
}

#[binread]
#[derive(Debug)]
#[br(import_raw(base_offset: u64))]
pub struct MapPart {
    #[br(parse_with = parse_string_ptr32, args_raw(base_offset))]
    pub name: String,

    // TODO: The index of the instance in PropLods.instances?
    pub instance_index: u32,

    // TODO: matches with PropInstance part id?
    // TODO: Multiple MapPart can have the same ID?
    pub part_id: u16,

    flags: u16,
    animation_start: u8,
    animation_speed: u8,

    /// The transform from [transforms](struct.MapParts.html#structfield.transforms).
    pub transform_index: u16,

    node_animation_index: u16,
    instance_animation_index: u16,
    switch_group_index: u16,
    unk: u16,
}

/// A reference to an [Xbc1](crate::xbc1::Xbc1) in `.wismda` file.
#[binread]
#[derive(Debug)]
pub struct StreamEntry<T> {
    /// The offset of the [Xbc1](crate::xbc1::Xbc1) in `.wismda` file.
    pub offset: u32,
    pub decompressed_size: u32,
    phantom: PhantomData<T>,
}

impl<T> StreamEntry<T>
where
    for<'a> T: BinRead<Args<'a> = ()>,
{
    /// Decompress and read the data from a reader for a `.wismda` file.
    pub fn extract<R: Read + Seek>(&self, wismda: &mut R) -> T {
        let bytes = self.decompress(wismda);
        T::read_le(&mut Cursor::new(bytes)).unwrap()
    }

    /// Decompress the data from a reader for a `.wismda` file.
    pub fn decompress<R: Read + Seek>(&self, wismda: &mut R) -> Vec<u8> {
        wismda.seek(SeekFrom::Start(self.offset as u64)).unwrap();
        Xbc1::read(wismda).unwrap().decompress().unwrap()
    }
}
