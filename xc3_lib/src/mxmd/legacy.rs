use std::io::SeekFrom;

use crate::{
    parse_count32_offset32, parse_offset32_count32, parse_opt_ptr32, parse_ptr32,
    parse_string_ptr32, vertex::VertexAttribute, xc3_write_binwrite_impl, StringOffset32,
};
use binrw::{args, binread, BinRead, BinWrite};
use xc3_write::{Xc3Write, Xc3WriteOffsets};

use super::StateFlags;

// TODO: How much code can be shared with non legacy types?
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, PartialEq, Clone)]
#[br(magic(b"MXMD"))]
#[xc3(magic(b"MXMD"))]
pub struct MxmdLegacy {
    #[br(assert(version == 10040))]
    pub version: u32,

    // TODO: This type is different for legacy.
    /// A collection of [Model] and associated data.
    #[br(parse_with = parse_ptr32)]
    #[xc3(offset(u32))]
    pub models: Models,

    #[br(parse_with = parse_ptr32)]
    #[xc3(offset(u32))]
    pub materials: Materials,

    pub unk1: u32,

    #[br(parse_with = parse_ptr32)]
    #[xc3(offset(u32))]
    pub vertex: VertexData,

    #[br(parse_with = parse_ptr32)]
    #[xc3(offset(u32))]
    pub shaders: Shaders,

    #[br(parse_with = parse_opt_ptr32)]
    #[xc3(offset(u32))]
    pub packed_textures: Option<PackedTextures>,

    pub unk3: u32,

    /// Streaming information for the .casmt file or [None] if no .casmt file.
    #[br(parse_with = parse_opt_ptr32)]
    #[xc3(offset(u32))]
    pub streaming: Option<StreamingLegacy>,

    // TODO: padding?
    pub unk: [u32; 7],
}

#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, PartialEq, Clone)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct Models {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    pub max_xyz: [f32; 3],
    pub min_xyz: [f32; 3],

    #[br(parse_with = parse_offset32_count32)]
    #[br(args { offset: base_offset, inner: base_offset })]
    #[xc3(offset_count(u32, u32))]
    pub models: Vec<Model>,

    #[br(parse_with = parse_offset32_count32)]
    #[br(args { offset: base_offset, inner: base_offset })]
    #[xc3(offset_count(u32, u32))]
    pub skins: Vec<Skinning>,

    pub unk1: [u32; 9],
    pub unk2: u32,

    // TODO: Will this work for writing?
    #[br(temp, restore_position)]
    bones_offset: u32,

    #[br(parse_with = parse_offset32_count32)]
    #[br(args { offset: base_offset, inner: base_offset + bones_offset as u64 })]
    #[xc3(offset_count(u32, u32))]
    pub bones: Vec<Bone>,

    // TODO: Why does this sometimes have a null offset but nonzero count?
    #[br(parse_with = parse_offset32_count32_unchecked, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub floats: Vec<f32>,

    pub unk3: u32,

    // TODO: Will this work for writing?
    #[br(temp, restore_position)]
    bone_names_offset: u32,

    #[br(parse_with = parse_offset32_count32)]
    #[br(args { offset: base_offset, inner: base_offset + bone_names_offset as u64 })]
    #[xc3(offset_count(u32, u32))]
    pub bone_names: Vec<StringOffset32>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct Bone {
    #[br(parse_with = parse_string_ptr32, offset = base_offset)]
    pub name: String,
    /// The index in [bones](struct.Models.html#structfield.bones) of the parent bone.
    pub parent_index: i32,
    pub descendants_start_index: i32,
    pub descendants_end_index: i32,
    pub unk3: i32, // TODO: bone index?
    pub translation: [f32; 3],
    /// XYZ rotation in radians.
    pub rotation_euler: [f32; 3],
    pub scale: [f32; 3],
    pub inverse_bind_transform: [[f32; 4]; 4],
    pub transform: [[f32; 4]; 4],
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct Model {
    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub meshes: Vec<Mesh>,

    // TODO: flags?
    pub unk1: u32, // 0, 64, 320

    // TODO: Slightly larger than a volume containing all vertex buffers?
    /// The minimum XYZ coordinates of the bounding volume.
    pub max_xyz: [f32; 3],
    /// The maximum XYZ coordinates of the bounding volume.
    pub min_xyz: [f32; 3],
    // TODO: how to calculate this?
    pub bounding_radius: f32,
    // TODO: padding?
    pub unks: [u32; 7],
}

/// Flags and resources associated with a single draw call.
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct Mesh {
    pub flags1: u32,
    pub flags2: u32, // TODO: are these actually the same as switch?
    /// Index into [vertex_buffers](struct.VertexData.html#structfield.vertex_buffers).
    pub vertex_buffer_index: u32,
    /// Index into [index_buffers](struct.VertexData.html#structfield.index_buffers).
    pub index_buffer_index: u32,
    pub unk2: u32, // 1
    /// Index into [materials](struct.Materials.html#structfield.materials).
    pub material_index: u32,
    pub unk3: u32,  // 0
    pub unk4: u32,  // 0
    pub unk5: u32,  // TODO: 0 to 58?
    pub unk6: u32,  // 0
    pub unk7: u32,  // TODO: 0 to 119?
    pub unk8: u32,  // 0
    pub unk9: u32,  // 0
    pub unk10: u32, // 0
    pub unk11: u32, // 0
    pub unk12: u32, // 0
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct Skinning {
    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub indices: Vec<u16>,
}

#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, PartialEq, Clone)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct Materials {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    #[br(parse_with = parse_offset32_count32)]
    #[br(args { offset: base_offset, inner: base_offset })]
    #[xc3(offset_count(u32, u32), align(4))]
    pub materials: Vec<Material>,

    pub unk1_1: u32,
    pub unk1_2: u32,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub work_values: Vec<f32>,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub shader_vars: Vec<(u16, u16)>,

    // TODO: Offset?
    pub unks1_3: [u32; 2],

    #[br(parse_with = parse_offset32_count32, args { offset: base_offset, inner: base_offset })]
    #[xc3(offset_count(u32, u32))]
    pub techniques: Vec<Technique>,

    pub unks1_1: [u32; 2],

    // TODO: Is this always implied to have count 1?
    #[br(parse_with = parse_count32_offset32_unchecked, offset = base_offset)]
    #[xc3(count_offset(u32, u32))]
    pub alpha_test_textures: Vec<AlphaTestTexture>,

    pub unks1_2: [u32; 5],

    // TODO: where are the samplers?
    #[br(parse_with = parse_opt_ptr32)]
    #[br(args { offset: base_offset, inner: base_offset })]
    #[xc3(offset(u32))]
    pub unk2: Option<MaterialsUnk2>,

    pub unk: [u32; 3],
}

// TODO: same as xc2?
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct AlphaTestTexture {
    pub texture_index: u16,
    pub unk1: u16,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct Material {
    #[br(parse_with = parse_string_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub name: String,

    pub unk1: u32,
    pub color: [f32; 4],
    pub unk2: [u32; 6],
    pub unk3: [f32; 3],

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub textures: Vec<Texture>,

    // TODO: same as xc2?
    pub state_flags: StateFlags,

    pub unks1: [u32; 7], // TODO: offset at index 3?

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub techniques: Vec<MaterialTechnique>,

    pub unk: [u32; 6],
}

// TODO: same as xc2?
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct MaterialTechnique {
    pub technique_index: u32,
    pub unk1: UnkPassType,
    pub material_buffer_index: u16,
    pub unk4: u32, // 0x01000000?
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, BinWrite, PartialEq, Eq, Clone, Copy, Hash)]
#[brw(repr(u16))]
pub enum UnkPassType {
    Unk0 = 0, // opaque?
    Unk1 = 1, // alpha?
    Unk2 = 2,
    Unk3 = 3,
    Unk5 = 5,
    Unk8 = 8,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct Texture {
    pub texture_index: u16,
    pub unk_index: u16, // TODO: where are the samplers?
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
#[xc3(base_offset)]
pub struct MaterialsUnk2 {
    #[br(parse_with = parse_count32_offset32, offset = base_offset)]
    #[xc3(count_offset(u32, u32))]
    pub unk1: Vec<u64>,

    #[br(parse_with = parse_count32_offset32, offset = base_offset)]
    #[xc3(count_offset(u32, u32))]
    pub unk2: Vec<u32>,

    #[br(parse_with = parse_count32_offset32, offset = base_offset)]
    #[xc3(count_offset(u32, u32))]
    pub unk3: Vec<[u32; 3]>,

    pub unk: [u32; 4],
}

// TODO: compare with decompiled shader data.
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct Technique {
    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub attributes: Vec<super::VertexAttribute>,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub unk2: Vec<[u16; 4]>,

    pub unk3: u32,

    #[br(parse_with = parse_opt_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub unk4: Option<u32>,

    pub unk5: u32,
    pub unk6: u32,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub unk7: Vec<[u16; 4]>,

    pub unk8: u32,
    pub unk9: u32,

    // TODO: padding?
    pub padding: [u32; 5],
}

#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, PartialEq, Clone)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct VertexData {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    #[br(parse_with = parse_offset32_count32)]
    #[br(args { offset: base_offset, inner: base_offset })]
    #[xc3(offset_count(u32, u32))]
    pub vertex_buffers: Vec<VertexBufferDescriptor>,

    #[br(parse_with = parse_offset32_count32)]
    #[br(args { offset: base_offset, inner: base_offset })]
    #[xc3(offset_count(u32, u32))]
    pub index_buffers: Vec<IndexBufferDescriptor>,

    // TODO: weight buffer index for different passe?
    pub weight_buffer_indices: [u16; 6],

    // TODO: padding?
    pub unk: [u32; 5],
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct VertexBufferDescriptor {
    pub data_offset: u32,
    pub vertex_count: u32,
    /// The size or stride of the vertex in bytes.
    pub vertex_size: u32,

    /// A tightly packed list of attributes for the data for this buffer.
    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub attributes: Vec<VertexAttribute>,

    pub unk1: u32,

    // TODO: Find a better way to handle this.
    #[br(seek_before = SeekFrom::Start(base_offset + data_offset as u64))]
    #[br(restore_position)]
    #[br(count = vertex_count * vertex_size)]
    pub data: Vec<u8>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct IndexBufferDescriptor {
    pub data_offset: u32,
    pub index_count: u32,
    pub unk1: u16, // TODO: primitive type?
    pub unk2: u16, // TODO: index format?

    // TODO: Find a better way to handle this.
    #[br(seek_before = SeekFrom::Start(base_offset + data_offset as u64))]
    #[br(restore_position)]
    #[br(count = index_count * 2)]
    pub data: Vec<u8>,
}

/// A collection of [Mtxt](crate::mtxt::Mtxt) textures embedded in the current file.
#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, PartialEq, Clone)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct PackedTextures {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    #[br(parse_with = parse_count32_offset32, args { offset: base_offset, inner: base_offset })]
    #[xc3(count_offset(u32, u32))]
    pub textures: Vec<PackedTexture>,

    pub unk2: u32,

    #[xc3(shared_offset)]
    pub strings_offset: u32,
}

/// A single [Mtxt](crate::mtxt::Mtxt) texture.
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct PackedTexture {
    pub usage: TextureUsage,

    #[br(parse_with = parse_count32_offset32, offset = base_offset)]
    #[xc3(count_offset(u32, u32), align(4096))]
    pub mtxt_data: Vec<u8>,

    #[br(parse_with = parse_string_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub name: String,
}

/// Hints on how the texture is used.
/// Actual usage is determined by the shader.
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, BinWrite, Clone, Copy, PartialEq, Eq, Hash)]
#[brw(repr(u32))]
pub enum TextureUsage {
    /// _GLO, _GLW, _GLM, _RFM, _SPM, _BLM, _OCL, _DEP
    Spm = 16, // temp?
    /// _NRM, _NM, or _NRM_cmk
    Nrm = 18,
    /// _RGB, _RFM, _COL
    Unk32 = 32,
    /// _AMB, _RGB
    Unk34 = 34,
    /// _COL, _DCL
    Unk48 = 48,
    /// _COL
    Col = 80,
    /// _COL, _AVA
    Unk96 = 96,
    Unk112 = 112,
    /// _SPM
    Spm2 = 528,
    /// _NRM
    Nrm2 = 530,
    /// _RGB
    Unk544 = 544,
    Unk1056 = 1056,
    Unk1120 = 1120,
    /// _CUBE, _ENV, _REFA
    Cube = 65569,
}

// TODO: Nearly identical to StreamingDataLegacy but not compressed?
#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, PartialEq, Clone)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct StreamingLegacy {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    pub unk1: u32,
    pub unk2: u32,

    #[br(parse_with = parse_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub low_textures: PackedExternalTextures,

    #[br(parse_with = parse_opt_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub textures: Option<PackedExternalTextures>,

    // TODO: Why are these necessary?
    #[br(parse_with = parse_ptr32)]
    #[br(args {
        offset: base_offset,
        inner: args! { count: low_textures.textures.len() }
    })]
    #[xc3(offset(u32))]
    pub low_texture_indices: Vec<u16>,

    #[br(parse_with = parse_opt_ptr32)]
    #[br(args {
        offset: base_offset,
        inner: args! { count: textures.as_ref().map(|t| t.textures.len()).unwrap_or_default() }
    })]
    #[xc3(offset(u32))]
    pub texture_indices: Option<Vec<u16>>,

    pub low_texture_data_offset: u32,
    pub low_texture_size: u32,
    pub texture_data_offset: u32,
    pub texture_size: u32,
}

// TODO: Share type by making this generic over the texture type?
/// References to [Mibl](crate::mibl::Mibl) textures in a separate file.
#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, PartialEq, Clone)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct PackedExternalTextures {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    #[br(parse_with = parse_count32_offset32, args { offset: base_offset, inner: base_offset })]
    #[xc3(count_offset(u32, u32))]
    pub textures: Vec<PackedExternalTexture>,

    pub unk2: u32, // 0

    #[xc3(shared_offset)]
    pub strings_offset: u32,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct PackedExternalTexture {
    pub usage: TextureUsage,

    pub mtxt_length: u32,
    pub mtxt_offset: u32,

    #[br(parse_with = parse_string_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub name: String,
}

#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, PartialEq, Clone)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct Shaders {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    #[br(parse_with = parse_offset32_count32)]
    #[br(args { offset: base_offset, inner: base_offset })]
    #[xc3(offset_count(u32, u32))]
    pub shaders: Vec<Shader>,

    pub unk2: u32,

    // TODO: padding?
    pub unks: [u32; 5],
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct Shader {
    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub mths_data: Vec<u8>,

    // TODO: padding?
    pub unks: [u32; 2],
}

xc3_write_binwrite_impl!(TextureUsage, UnkPassType);

impl<'a> Xc3WriteOffsets for PackedExternalTexturesOffsets<'a> {
    fn write_offsets<W: std::io::prelude::Write + std::io::prelude::Seek>(
        &self,
        writer: &mut W,
        _base_offset: u64,
        data_ptr: &mut u64,
    ) -> xc3_write::Xc3Result<()> {
        let base_offset = self.base_offset;

        // Names need to be written at the end.
        let textures = self.textures.write(writer, base_offset, data_ptr)?;

        self.strings_offset
            .write_full(writer, base_offset, data_ptr)?;
        for texture in &textures.0 {
            texture.name.write_full(writer, base_offset, data_ptr)?;
        }
        Ok(())
    }
}

fn parse_offset32_count32_unchecked<T, R, Args>(
    reader: &mut R,
    endian: binrw::Endian,
    args: binrw::file_ptr::FilePtrArgs<Args>,
) -> binrw::BinResult<Vec<T>>
where
    for<'a> T: BinRead<Args<'a> = Args> + 'static,
    R: std::io::Read + std::io::Seek,
    Args: Clone,
{
    let offset = u32::read_options(reader, endian, ())?;
    let count = u32::read_options(reader, endian, ())?;

    crate::parse_vec(reader, endian, args, offset as u64, count as usize)
}

fn parse_count32_offset32_unchecked<T, R, Args>(
    reader: &mut R,
    endian: binrw::Endian,
    args: binrw::file_ptr::FilePtrArgs<Args>,
) -> binrw::BinResult<Vec<T>>
where
    for<'a> T: BinRead<Args<'a> = Args> + 'static,
    R: std::io::Read + std::io::Seek,
    Args: Clone,
{
    let count = u32::read_options(reader, endian, ())?;
    let offset = u32::read_options(reader, endian, ())?;

    crate::parse_vec(reader, endian, args, offset as u64, count as usize)
}
