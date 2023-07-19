//! `.wismhd` files for map data that points to data in a corresponding `.wismda` files
use std::{
    io::{Cursor, Read, Seek, SeekFrom},
    marker::PhantomData,
};

use binrw::{binread, BinRead};

use crate::{
    map::{
        EnvModelData, FoliageModelData, FoliageUnkData, FoliageVertexData, MapLowModelData,
        MapModelData, PropInstance, PropModelData, PropPositions,
    },
    mibl::Mibl,
    parse_count_offset, parse_offset_count, parse_opt_ptr32, parse_ptr32, parse_string_ptr32,
    vertex::VertexData,
    xbc1::Xbc1,
};

// TODO: Is it worth implementing serialize?
/// The main map data for a `.wismhd` file.
#[derive(BinRead, Debug)]
#[br(magic(b"DMSM"))]
pub struct Msmd {
    pub version: u32,
    // TODO: always 0?
    pub unk1: [u32; 4],

    #[br(parse_with = parse_count_offset)]
    pub map_models: Vec<MapModel>,

    #[br(parse_with = parse_count_offset)]
    pub prop_models: Vec<PropModel>,

    pub unk1_1: [u32; 2],

    #[br(parse_with = parse_count_offset)]
    pub env_models: Vec<EnvModel>,

    #[br(parse_with = parse_ptr32)]
    pub wismda_info: WismdaInfo,

    pub unk2_1: u32,

    #[br(parse_with = parse_opt_ptr32)]
    pub effects: Option<Effects>,

    pub unk2: [u32; 3],

    /// `.wismda` data with names like `/seamwork/inst/mdl/00003.te`.
    #[br(parse_with = parse_count_offset)]
    pub prop_vertex_data: Vec<StreamEntry<VertexData>>,

    // TODO: What do these do?
    #[br(parse_with = parse_count_offset)]
    pub textures: Vec<Texture>,

    pub strings_offset: u32,

    #[br(parse_with = parse_count_offset)]
    pub foliage_models: Vec<FoliageModel>,

    /// `.wismda` data with names like `/seamwork/inst/pos/00000.et`.
    #[br(parse_with = parse_count_offset)]
    pub prop_positions: Vec<StreamEntry<PropPositions>>,

    /// `.wismda` data with names like `/seamwork/mpfmap/poli//0022`.
    #[br(parse_with = parse_count_offset)]
    pub foliage_data: Vec<StreamEntry<FoliageVertexData>>,

    pub unk3_1: u32,
    pub unk3_2: u32,

    #[br(parse_with = parse_ptr32)]
    pub dlgt: Dlgt,

    #[br(parse_with = parse_count_offset)]
    pub unk_lights: Vec<UnkLight>,

    // low resolution packed textures?
    /// `.wismda` data with names like `/seamwork/texture/00000_wi`.
    #[br(parse_with = parse_count_offset)]
    pub low_textures: Vec<StreamEntry<LowTextures>>,

    // TODO: Document more of these fields.
    pub unk4: [u32; 6],

    #[br(parse_with = parse_opt_ptr32)]
    pub parts: Option<MapParts>,

    pub unk4_2: u32,

    #[br(parse_with = parse_count_offset)]
    pub low_models: Vec<MapLowModel>,

    pub env_flags: u32,

    /// `.wismda` data with names like `/seamwork/mpfmap/poli//0000`.
    #[br(parse_with = parse_count_offset)]
    pub unk_foliage_data: Vec<StreamEntry<FoliageUnkData>>,

    /// `.wismda` data with names like `/seamwork/basemap/poli//000`
    /// or `/seamwork/basemap/poli//001`.
    // TODO: Are all of these referenced by map models?
    // TODO: What references "poli/001"?
    #[br(parse_with = parse_count_offset)]
    pub map_vertex_data: Vec<StreamEntry<VertexData>>,

    #[br(parse_with = parse_ptr32)]
    #[br(args { inner: env_flags })]
    pub nerd: EnvironmentData,

    pub unk6: [u32; 3],

    #[br(parse_with = parse_ptr32)]
    pub ibl: Ibl,

    #[br(parse_with = parse_opt_ptr32)]
    pub cmld: Option<Cmld>,

    pub unk5_2: u32,
    pub unk5_3: u32,

    #[br(parse_with = parse_opt_ptr32)]
    pub unk5_4: Option<Doce>,

    pub unk5_5: u32,
    pub unk5_6: u32,

    // padding?
    pub unk7: [u32; 8],
}

/// References to medium and high resolution [Mibl](crate::mibl::Mibl) textures.
#[derive(BinRead, Debug)]
pub struct Texture {
    pub mid: StreamEntry<Mibl>,
    // TODO: This is just vec<u8>?
    pub high: StreamEntry<Mibl>,
    pub flags: u32,
}

// TODO: Better name for this?
#[derive(BinRead, Debug)]
pub struct MapModel {
    pub bounds: BoundingBox,
    // bounding sphere?
    pub unk2: [f32; 4],
    /// `.wismda` data with names like `bina_basefix.temp_wi`.
    pub entry: StreamEntry<MapModelData>,
    pub unk3: [f32; 4],
}

// TODO: Better name for this?

#[derive(BinRead, Debug)]
pub struct PropModel {
    pub bounds: BoundingBox,
    // bounding sphere?
    pub unk2: [f32; 4],
    /// `.wismda` data with names like `/seamwork/inst/out/00000.te`.
    pub entry: StreamEntry<PropModelData>,
    pub unk3: u32,
}

#[derive(BinRead, Debug)]
pub struct EnvModel {
    pub bounds: BoundingBox,
    // bounding sphere?
    pub unk2: [f32; 4],
    /// `.wismda` data with names like `/seamwork/envmap/ma00a/bina`.
    pub entry: StreamEntry<EnvModelData>,
}

// TODO: also in mxmd but without the center?

#[derive(BinRead, Debug)]
pub struct BoundingBox {
    pub max: [f32; 3],
    pub min: [f32; 3],
    pub center: [f32; 3],
}

#[derive(BinRead, Debug)]
pub struct MapLowModel {
    pub bounds: BoundingBox,
    pub unk1: f32,
    /// `.wismda` data with names like `/seamwork/lowmap/ma11a/bina`.
    pub entry: StreamEntry<MapLowModelData>,
    pub unk2: u16,
    pub unk3: u16,
    // TODO: padding?
    pub unk: [u32; 5],
}

#[derive(BinRead, Debug)]
pub struct FoliageModel {
    pub unk1: [f32; 9],
    pub unk: [u32; 3],
    pub unk2: f32,
    /// `.wismda` data with names like `/seamwork/mpfmap/ma11a/bina`.
    pub entry: StreamEntry<FoliageModelData>,
}

#[derive(BinRead, Debug)]
#[br(import_raw(flags: u32))]
pub enum EnvironmentData {
    #[br(pre_assert(flags == 0))]
    Cems(Cems),
    #[br(pre_assert(flags == 2))]
    Nerd(Nerd),
}

#[derive(BinRead, Debug)]
#[br(magic(b"DREN"))]
pub struct Nerd {
    pub version: u32,
    pub unk1: u32,
    pub unk2: u32,
    pub unk3: u32,
    pub unk4: u32,
    pub unk5: u32,
    // padding?
    pub unk6: [u32; 6],
}

// TODO: This contains a Nerd?

#[derive(BinRead, Debug)]
#[br(magic(b"SMEC"))]
pub struct Cems {
    pub unk1: [u32; 10],
    pub offset: u32,
}

// TODO: cloud data?
#[derive(BinRead, Debug)]
#[br(magic(b"CMLD"))]
pub struct Cmld {
    pub version: u32,
}

// TODO: Lighting data?
// TODO: .wilgt files?
#[derive(BinRead, Debug)]
#[br(magic(b"DLGT"))]
pub struct Dlgt {
    pub version: u32,
    pub unk1: u32,
    pub unk2: u32,
}

#[binread]
#[derive(Debug)]
#[br(stream = r)]
pub struct Ibl {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    #[br(parse_with = parse_count_offset, args { offset: base_offset, inner: base_offset })]
    pub unk1: Vec<IblInner>,

    pub unk3: u32,
    pub unk4: u32,
    pub unk5: u32,
    pub unk6: u32,
}

#[derive(BinRead, Debug)]
#[br(import_raw(base_offset: u64))]
pub struct IblInner {
    pub unk1: u32, // 0?
    #[br(parse_with = parse_string_ptr32, offset = base_offset)]
    pub map_name: String,
    #[br(parse_with = parse_ptr32, offset = base_offset)]
    pub gibl: Gibl,
    pub unk4: u32, // gibl section length?
    // padding?
    pub unk5: [u32; 6],
}

#[derive(BinRead, Debug)]
#[br(magic(b"GIBL"))]
pub struct Gibl {
    pub unk1: u32,
    pub unk2: u32,
    pub unk3: u32,
    pub unk4: u32, // offset to mibl?
    pub unk5: u32,
    // TODO: padding?
    pub unk6: [u32; 6],
}

#[derive(BinRead, Debug)]
pub struct WismdaInfo {
    pub compressed_length: u32,
    pub unk1: u32,
    pub decompressed_length: u32,
    pub streaming_buffer_length: u32,
    pub unks: [u32; 15],
}

#[binread]
#[derive(Debug)]
#[br(stream = r)]
pub struct Effects {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    #[br(parse_with = parse_count_offset, offset = base_offset)]
    pub unk1: Vec<Effect>,

    pub unk3: u32,
}

#[binread]
#[derive(Debug)]
#[br(stream = r)]
pub struct Effect {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    #[br(parse_with = parse_string_ptr32, offset = base_offset)]
    pub unk1: String,

    // TODO: xc2 has a string here instead?
    pub transform_count: u32,
    pub transform_offset: u32,

    pub unk4: u32,
    pub unk5: u32,
    pub unk6: f32,
    pub unk7: f32,
    pub unk8: f32,
    pub unk9: f32,
    pub unk10: u32,
    pub unk11: u32,
    pub unk12: u32,
    pub unk13: u32,
    pub unk14: u32,
    pub unk15: u32,
    pub unk16: u32,
}

// TODO: What does this do?
// 116 bytes including magic?

#[derive(BinRead, Debug)]
#[br(magic(b"DOCE"))]
pub struct Doce {
    pub version: u32,
    pub offset: u32,
    pub count: u32,
}

#[derive(BinRead, Debug)]
pub struct LowTextures {
    #[br(parse_with = parse_count_offset)]
    pub textures: Vec<LowTexture>,
    // TODO: Padding?
    pub unk: [u32; 5],
}

#[derive(BinRead, Debug)]
pub struct LowTexture {
    pub unk1: u32,
    // TODO: Optimized function for reading bytes?
    #[br(parse_with = parse_count_offset)]
    pub mibl_data: Vec<u8>,
    pub unk2: i32,
}

#[derive(BinRead, Debug)]
pub struct UnkLight {
    pub max: [f32; 3],
    pub min: [f32; 3],
    /// `.wismda` data with names like `/seamwork/lgt/bina/00000.wi`.
    pub entry: StreamEntry<Dlgt>,
    pub unk3: u32,
    // TODO: padding?
    pub unk4: [u32; 5],
}

#[binread]
#[derive(Debug)]
#[br(stream = r)]
pub struct MapParts {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    // TODO: Where do static parts index?
    #[br(parse_with = parse_offset_count, args { offset: base_offset, inner: base_offset })]
    pub parts: Vec<MapPart>,

    pub unk_count: u32,

    #[br(temp)]
    animated_parts_offset: u32,

    #[br(temp)]
    instance_animations_offset: u32,

    pub unk2: u32,

    #[br(temp)]
    instance_animations_count: u32,

    #[br(seek_before = std::io::SeekFrom::Start(base_offset + animated_parts_offset as u64))]
    #[br(args { count: instance_animations_count as usize })]
    #[br(restore_position)]
    pub animated_instances: Vec<PropInstance>,

    // TODO: Find a cleaner way of writing this?
    #[br(seek_before = std::io::SeekFrom::Start(base_offset + instance_animations_offset as u64))]
    #[br(args { count: instance_animations_count as usize, inner: base_offset })]
    #[br(restore_position)]
    pub instance_animations: Vec<MapPartInstanceAnimation>,

    pub unk4: u32,
    pub unk5: u32,
    pub unk6: u32,
    pub unk7: u32,

    #[br(parse_with = parse_offset_count, offset = base_offset)]
    pub transforms: Vec<[[f32; 4]; 4]>,
}

#[derive(BinRead, Debug)]
#[br(import_raw(base_offset: u64))]
pub struct MapPartInstanceAnimation {
    pub translation: [f32; 3],
    pub rotation: [f32; 3],
    pub scale: [f32; 3],
    pub unk1: u32,
    pub unk2: u32,
    pub unk3: u32,
    pub flags: u32,

    #[br(parse_with = parse_offset_count, args { offset: base_offset, inner: base_offset })]
    pub channels: Vec<MapPartInstanceAnimationChannel>,

    pub time_min: u16,
    pub time_max: u16,
    // TODO: padding?
    pub unks: [u32; 5],
}

#[derive(BinRead, Debug)]
#[br(import_raw(base_offset: u64))]
pub struct MapPartInstanceAnimationChannel {
    // TODO: Group this together into a single type?
    pub keyframes_offset: u32,
    pub channel_type: ChannelType,
    pub keyframe_count: u16,

    pub time_min: u16,
    pub time_max: u16,

    #[br(seek_before = std::io::SeekFrom::Start(base_offset + keyframes_offset as u64))]
    #[br(count = keyframe_count)]
    #[br(restore_position)]
    pub keyframes: Vec<MapPartInstanceAnimationKeyframe>,
}

#[derive(BinRead, Debug)]
#[br(repr(u16))]
pub enum ChannelType {
    TranslationX = 0,
    TranslationY = 1,
    TranslationZ = 2,
    RotationX = 3,
    RotationY = 4,
    RotationZ = 5,
    ScaleX = 6,
    ScaleY = 7,
    ScaleZ = 8,
}

#[derive(BinRead, Debug)]
pub struct MapPartInstanceAnimationKeyframe {
    pub slope_out: f32,
    pub slope_in: f32,
    pub value: f32,
    pub time: u16,
    pub flags: u16,
}

#[derive(BinRead, Debug)]
#[br(import_raw(base_offset: u64))]
pub struct MapPart {
    #[br(parse_with = parse_string_ptr32, offset = base_offset)]
    pub name: String,

    // TODO: The index of the instance in PropLods.instances?
    pub instance_index: u32,

    // TODO: matches with PropInstance part id?
    // TODO: Multiple MapPart can have the same ID?
    pub part_id: u16,

    pub flags: u16,
    pub animation_start: u8,
    pub animation_speed: u8,

    /// The transform from [transforms](struct.MapParts.html#structfield.transforms).
    pub transform_index: u16,

    pub node_animation_index: u16,
    pub instance_animation_index: u16,
    pub switch_group_index: u16,
    pub unk: u16,
}

/// A reference to an [Xbc1](crate::xbc1::Xbc1) in `.wismda` file.

#[derive(BinRead, Debug)]
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
    pub fn extract<R: Read + Seek>(&self, wismda: &mut R, is_compressed: bool) -> T {
        let bytes = self.decompress(wismda, is_compressed);
        T::read_le(&mut Cursor::new(bytes)).unwrap()
    }

    /// Decompress the data from a reader for a `.wismda` file.
    pub fn decompress<R: Read + Seek>(&self, wismda: &mut R, is_compressed: bool) -> Vec<u8> {
        // Not all wismda files use XBC1 archives to store data.
        wismda.seek(SeekFrom::Start(self.offset as u64)).unwrap();
        if is_compressed {
            Xbc1::read(wismda).unwrap().decompress().unwrap()
        } else {
            let mut bytes = vec![0u8; self.decompressed_size as usize];
            wismda.read_exact(&mut bytes).unwrap();
            bytes
        }
    }
}
