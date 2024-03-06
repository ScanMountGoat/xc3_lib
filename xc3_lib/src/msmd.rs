//! `.wismhd` files for map data that points to data in a corresponding `.wismda` files
//!
//! # File Paths
//! | Game | File Patterns |
//! | --- | --- |
//! | Xenoblade Chronicles 1 DE | `map/*.wismhd` |
//! | Xenoblade Chronicles 2 | `map/*.wismhd` |
//! | Xenoblade Chronicles 3 | `map/*.wismhd` |
use std::{
    io::{Cursor, Read, Seek, SeekFrom},
    marker::PhantomData,
};

use binrw::{binread, BinRead, BinWrite};
use xc3_write::{Xc3Write, Xc3WriteOffsets};

use crate::{
    error::DecompressStreamError,
    map::{
        EnvModelData, FoliageModelData, FoliageUnkData, FoliageVertexData, MapLowModelData,
        MapModelData, PropInstance, PropModelData, PropPositions,
    },
    mibl::Mibl,
    mxmd::TextureUsage,
    parse_count32_offset32, parse_offset32_count32, parse_opt_ptr32, parse_ptr32,
    parse_string_ptr32,
    vertex::VertexData,
    xbc1::Xbc1,
    xc3_write_binwrite_impl,
};

/// The main map data for a `.wismhd` file.
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(magic(b"DMSM"))]
#[xc3(magic(b"DMSM"))]
pub struct Msmd {
    /// 10112
    pub version: u32,
    // TODO: always 0?
    pub unk1: [u32; 4],

    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub map_models: Vec<MapModel>,

    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub prop_models: Vec<PropModel>,

    pub unk1_1: [u32; 2],

    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub env_models: Vec<EnvModel>,

    #[br(parse_with = parse_ptr32)]
    #[xc3(offset(u32))]
    pub wismda_info: WismdaInfo,

    pub unk2_1: u32,

    #[br(parse_with = parse_opt_ptr32)]
    #[xc3(offset(u32))]
    pub effects: Option<Effects>,

    pub unk2: [u32; 3],

    /// `.wismda` data with names like `/seamwork/inst/mdl/00003.te`.
    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub prop_vertex_data: Vec<StreamEntry<VertexData>>,

    /// High resolution textures.
    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub textures: Vec<Texture>,

    pub strings_offset: u32,

    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub foliage_models: Vec<FoliageModel>,

    /// `.wismda` data with names like `/seamwork/inst/pos/00000.et`.
    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub prop_positions: Vec<StreamEntry<PropPositions>>,

    /// `.wismda` data with names like `/seamwork/mpfmap/poli//0022`.
    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub foliage_data: Vec<StreamEntry<FoliageVertexData>>,

    pub unk3_1: u32,
    pub unk3_2: u32,

    #[br(parse_with = parse_ptr32)]
    #[xc3(offset(u32))]
    pub dlgt: Dlgt,

    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub unk_lights: Vec<UnkLight>,

    // low resolution packed textures?
    /// `.wismda` data with names like `/seamwork/texture/00000_wi`.
    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub low_textures: Vec<StreamEntry<LowTextures>>,

    // TODO: Document more of these fields.
    pub unk4: [u32; 6],

    #[br(parse_with = parse_opt_ptr32)]
    #[xc3(offset(u32))]
    pub parts: Option<MapParts>,

    pub unk4_2: u32,

    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub low_models: Vec<MapLowModel>,

    pub env_flags: u32,

    /// `.wismda` data with names like `/seamwork/mpfmap/poli//0000`.
    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub unk_foliage_data: Vec<StreamEntry<FoliageUnkData>>,

    /// `.wismda` data with names like `/seamwork/basemap/poli//000`
    /// or `/seamwork/basemap/poli//001`.
    // TODO: Are all of these referenced by map models?
    // TODO: What references "poli/001"?
    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub map_vertex_data: Vec<StreamEntry<VertexData>>,

    #[br(parse_with = parse_ptr32)]
    #[xc3(offset(u32))]
    #[br(args { inner: env_flags })]
    pub nerd: EnvironmentData,

    pub unk6: [u32; 3],

    #[br(parse_with = parse_ptr32)]
    #[xc3(offset(u32))]
    pub ibl: Ibl,

    #[br(parse_with = parse_opt_ptr32)]
    #[xc3(offset(u32))]
    pub cmld: Option<Cmld>,

    pub unk5_2: u32,
    pub unk5_3: u32,

    #[br(parse_with = parse_opt_ptr32)]
    #[xc3(offset(u32))]
    pub unk5_4: Option<Doce>,

    pub unk5_5: u32,
    pub unk5_6: u32,

    // padding?
    pub unk7: [u32; 8],
}

/// References to medium and high resolution [Mibl] textures.
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct Texture {
    pub mid: StreamEntry<Mibl>,
    // TODO: This isn't always used?
    pub base_mip: StreamEntry<Vec<u8>>,
    pub flags: u32, // TODO: What do these do?
}

// TODO: Better name for this?
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct MapModel {
    pub bounds: BoundingBox,
    // bounding sphere?
    pub unk2: [f32; 4],
    /// `.wismda` data with names like `bina_basefix.temp_wi`.
    pub entry: StreamEntry<MapModelData>,
    pub unk3: [f32; 4],
}

// TODO: Better name for this?
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct PropModel {
    pub bounds: BoundingBox,
    // bounding sphere?
    pub unk2: [f32; 4],
    /// `.wismda` data with names like `/seamwork/inst/out/00000.te`.
    pub entry: StreamEntry<PropModelData>,
    pub unk3: u32,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct EnvModel {
    pub bounds: BoundingBox,
    // bounding sphere?
    pub unk2: [f32; 4],
    /// `.wismda` data with names like `/seamwork/envmap/ma00a/bina`.
    pub entry: StreamEntry<EnvModelData>,
}

// TODO: also in mxmd but without the center?

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct BoundingBox {
    pub max: [f32; 3],
    pub min: [f32; 3],
    pub center: [f32; 3],
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
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

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct FoliageModel {
    pub unk1: [f32; 9],
    pub unk: [u32; 3],
    pub unk2: f32,
    /// `.wismda` data with names like `/seamwork/mpfmap/ma11a/bina`.
    pub entry: StreamEntry<FoliageModelData>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(flags: u32))]
pub enum EnvironmentData {
    #[br(pre_assert(flags == 0))]
    Cems(Cems),
    #[br(pre_assert(flags == 2))]
    Nerd(Nerd),
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(magic(b"DREN"))]
#[xc3(magic(b"DREN"))]
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

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(magic(b"SMEC"))]
#[xc3(magic(b"SMEC"))]
pub struct Cems {
    pub unk1: [u32; 10],
    pub offset: u32,
}

// TODO: cloud data?
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(magic(b"CMLD"))]
#[xc3(magic(b"CMLD"))]
pub struct Cmld {
    pub version: u32,
}

// TODO: Lighting data?
// TODO: .wilgt files?
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(magic(b"DLGT"))]
#[xc3(magic(b"DLGT"))]
pub struct Dlgt {
    pub version: u32,
    pub unk1: u32,
    pub unk2: u32,
}

#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct Ibl {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    #[br(parse_with = parse_count32_offset32, args { offset: base_offset, inner: base_offset })]
    #[xc3(count_offset(u32, u32))]
    pub unk1: Vec<IblInner>,

    pub unk3: u32,
    pub unk4: u32,
    pub unk5: u32,
    pub unk6: u32,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct IblInner {
    pub unk1: u32, // 0?

    #[br(parse_with = parse_string_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub map_name: String,

    #[br(parse_with = parse_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub gibl: Gibl,

    pub unk4: u32, // gibl section length?
    // padding?
    pub unk5: [u32; 6],
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(magic(b"GIBL"))]
#[xc3(magic(b"GIBL"))]
pub struct Gibl {
    pub unk1: u32,
    pub unk2: u32,
    pub unk3: u32,
    pub unk4: u32, // offset to mibl?
    pub unk5: u32,
    // TODO: padding?
    pub unk6: [u32; 6],
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct WismdaInfo {
    pub compressed_length: u32,
    pub unk1: u32,
    pub decompressed_length: u32,
    pub streaming_buffer_length: u32,
    pub unks: [u32; 15],
}

#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct Effects {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    #[br(parse_with = parse_count32_offset32, offset = base_offset)]
    #[xc3(count_offset(u32, u32))]
    pub unk1: Vec<Effect>,

    pub unk3: u32,
}

#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct Effect {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    #[br(parse_with = parse_string_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
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
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(magic(b"DOCE"))]
#[xc3(magic(b"DOCE"))]
pub struct Doce {
    pub version: u32,
    pub offset: u32,
    pub count: u32,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct LowTextures {
    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub textures: Vec<LowTexture>,
    // TODO: Padding?
    pub unk: [u32; 5],
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct LowTexture {
    pub usage: TextureUsage,
    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub mibl_data: Vec<u8>,
    pub unk2: i32, // TODO: always -1?
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct UnkLight {
    pub max: [f32; 3],
    pub min: [f32; 3],
    /// `.wismda` data with names like `/seamwork/lgt/bina/00000.wi`.
    pub entry: StreamEntry<Dlgt>,
    pub unk3: u32,
    // TODO: padding?
    pub unk4: [u32; 5],
}

// TODO: How to get writing working?
#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct MapParts {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    // TODO: Where do static parts index?
    #[br(parse_with = parse_offset32_count32, args { offset: base_offset, inner: base_offset })]
    #[xc3(offset_count(u32, u32))]
    pub parts: Vec<MapPart>,

    pub unk_count: u32,

    // TODO: How to handle this for writing?
    #[br(temp)]
    animated_parts_offset: u32,

    #[br(temp)]
    instance_animations_offset: u32,

    pub unk2: u32,

    #[br(temp)]
    instance_animations_count: u32,

    // TODO: Find a cleaner way of handling these offsets.
    #[br(seek_before = std::io::SeekFrom::Start(base_offset + animated_parts_offset as u64))]
    #[br(args { count: instance_animations_count as usize })]
    #[br(restore_position)]
    pub animated_instances: Vec<PropInstance>,

    #[br(seek_before = std::io::SeekFrom::Start(base_offset + instance_animations_offset as u64))]
    #[br(args { count: instance_animations_count as usize, inner: base_offset })]
    #[br(restore_position)]
    pub instance_animations: Vec<MapPartInstanceAnimation>,

    pub unk4: u32,
    pub unk5: u32,
    pub unk6: u32,
    pub unk7: u32,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub transforms: Vec<[[f32; 4]; 4]>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct MapPartInstanceAnimation {
    pub translation: [f32; 3],
    pub rotation: [f32; 3],
    pub scale: [f32; 3],
    pub unk1: u32,
    pub unk2: u32,
    pub unk3: u32,
    pub flags: u32,

    #[br(parse_with = parse_offset32_count32, args { offset: base_offset, inner: base_offset })]
    #[xc3(offset_count(u32, u32))]
    pub channels: Vec<MapPartInstanceAnimationChannel>,

    pub time_min: u16,
    pub time_max: u16,
    // TODO: padding?
    pub unks: [u32; 5],
}

// TODO: Derive xc3write?
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct MapPartInstanceAnimationChannel {
    // TODO: Group this together into a single type?
    pub keyframes_offset: u32,
    pub channel_type: ChannelType,
    pub keyframe_count: u16,

    pub time_min: u16,
    pub time_max: u16,

    // TODO: Write offset?
    #[br(seek_before = std::io::SeekFrom::Start(base_offset + keyframes_offset as u64))]
    #[br(count = keyframe_count as usize)]
    #[br(restore_position)]
    pub keyframes: Vec<MapPartInstanceAnimationKeyframe>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, BinWrite, PartialEq, Clone)]
#[brw(repr(u16))]
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

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct MapPartInstanceAnimationKeyframe {
    pub slope_out: f32,
    pub slope_in: f32,
    pub value: f32,
    pub time: u16,
    pub flags: u16,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct MapPart {
    #[br(parse_with = parse_string_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
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

/// A reference to an [Xbc1] in the `.wismda` file.
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, BinWrite, PartialEq, Clone)]
pub struct StreamEntry<T> {
    /// The offset of the [Xbc1] in the `.wismda` file.
    pub offset: u32,
    pub decompressed_size: u32,
    #[bw(ignore)]
    phantom: PhantomData<T>,
}

impl<T> StreamEntry<T> {
    /// Decompress the data from a reader for a `.wismda` file.
    pub fn decompress<R: Read + Seek>(
        &self,
        wismda: &mut R,
        is_compressed: bool,
    ) -> Result<Vec<u8>, DecompressStreamError> {
        // Not all wismda files use XBC1 archives to store data.
        wismda.seek(SeekFrom::Start(self.offset as u64))?;
        if is_compressed {
            let bytes = Xbc1::read(wismda)?.decompress()?;
            Ok(bytes)
        } else {
            let mut bytes = vec![0u8; self.decompressed_size as usize];
            wismda.read_exact(&mut bytes)?;
            Ok(bytes)
        }
    }
}

impl<T> StreamEntry<T>
where
    for<'a> T: BinRead<Args<'a> = ()>,
{
    /// Decompress and read the data from a reader for a `.wismda` file.
    pub fn extract<R: Read + Seek>(
        &self,
        wismda: &mut R,
        is_compressed: bool,
    ) -> Result<T, DecompressStreamError> {
        let bytes = self.decompress(wismda, is_compressed)?;
        T::read_le(&mut Cursor::new(bytes)).map_err(Into::into)
    }
}

// TODO: Find a way to derive this?
impl<T> Xc3Write for StreamEntry<T> {
    type Offsets<'a> = () where T: 'a;

    fn xc3_write<W: std::io::Write + Seek>(
        &self,
        writer: &mut W,
    ) -> xc3_write::Xc3Result<Self::Offsets<'_>> {
        self.write_le(writer)?;
        Ok(())
    }
}

xc3_write_binwrite_impl!(ChannelType);
