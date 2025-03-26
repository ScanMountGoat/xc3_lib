//! Legacy types for Xenoblade Chronicles X.
use std::io::SeekFrom;

use crate::{
    msrd::StreamingDataLegacyInner, parse_count32_offset32, parse_offset, parse_offset32_count32,
    parse_opt_ptr32, parse_ptr32, parse_string_ptr32, vertex::VertexAttribute,
    xc3_write_binwrite_impl, StringOffset32,
};
use binrw::{args, binread, BinRead, BinWrite};
use xc3_write::{Xc3Write, Xc3WriteOffsets};

use super::{MaterialFlags, ModelUnk3, SamplerFlags, StateFlags};

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(magic(b"MXMD"))]
#[xc3(magic(b"MXMD"))]
pub struct MxmdLegacy {
    #[br(assert(version == 10040))]
    pub version: u32,

    /// A collection of [Model] and associated data.
    #[br(parse_with = parse_ptr32)]
    #[xc3(offset(u32))]
    pub models: Models,

    #[br(parse_with = parse_ptr32)]
    #[xc3(offset(u32))]
    pub materials: Materials,

    #[br(parse_with = parse_opt_ptr32)]
    #[xc3(offset(u32))]
    pub unk1: Option<Unk1>,

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
    pub streaming: Option<Streaming>,

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

    #[br(temp, restore_position)]
    models_offset: u32,

    #[br(parse_with = parse_offset32_count32)]
    #[br(args { offset: base_offset, inner: base_offset })]
    #[xc3(offset_count(u32, u32))]
    pub models: Vec<Model>,

    #[br(parse_with = parse_offset32_count32)]
    #[br(args { offset: base_offset, inner: base_offset })]
    #[xc3(offset_count(u32, u32))]
    pub skins: Vec<SkinningIndices>,

    pub unk1: [u32; 3],

    #[br(parse_with = parse_opt_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub unk_bones: Option<UnkBones>,

    pub unk1_2: [u32; 5],

    pub unk2: u32,

    // TODO: Will this work for writing?
    #[br(temp, restore_position)]
    bones_offset: u32,

    #[br(parse_with = parse_offset32_count32)]
    #[br(args { offset: base_offset, inner: base_offset + bones_offset as u64 })]
    #[xc3(offset_count(u32, u32))]
    pub bones: Vec<Bone>,

    #[br(parse_with = parse_opt_ptr32)]
    #[br(args { offset: base_offset, inner: args! { count: unk_float_count(&skins) } })]
    #[xc3(offset(u32))]
    pub floats: Option<Vec<f32>>,

    pub unk4: u32,

    #[br(parse_with = parse_opt_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub unk3: Option<ModelUnk3>,

    // TODO: Will this work for writing?
    #[br(temp, restore_position)]
    bone_names_offset: u32,

    #[br(parse_with = parse_offset32_count32)]
    #[br(args { offset: base_offset, inner: base_offset + bone_names_offset as u64 })]
    #[xc3(offset_count(u32, u32))]
    pub bone_names: Vec<StringOffset32>,

    pub unk5: u32,

    #[br(parse_with = parse_offset32_count32)]
    #[br(args { offset: base_offset, inner: base_offset })]
    #[xc3(offset_count(u32, u32))]
    pub unk6: Vec<Unk6>,

    // TODO: transforms?
    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub unk7: Vec<[[[f32; 4]; 4]; 2]>,

    pub unk8: [u32; 2],

    #[br(parse_with = parse_opt_ptr32)]
    #[br(args { offset: base_offset, inner: base_offset })]
    #[xc3(offset(u32))]
    pub unk10: Option<Unk10>,

    #[br(parse_with = parse_opt_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub unk9: Option<Unk9>,

    pub unks: [u32; 2],

    #[br(args { base_offset, size: models_offset })]
    pub extra: ModelsExtra,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import { base_offset:u64, size: u32 })]
pub enum ModelsExtra {
    // XCX has 152 total bytes.
    #[br(pre_assert(size == 152))]
    Unk0,

    // XCX can also have 156 total bytes.
    #[br(pre_assert(size == 156))]
    Unk1(ModelsExtraUnk1),

    // XCX DE has 172 total bytes.
    #[br(pre_assert(size == 172))]
    Unk2(#[br(args_raw(base_offset))] ModelsExtraUnk2),
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct ModelsExtraUnk1 {
    // TODO: padding?
    pub unk: u32,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct ModelsExtraUnk2 {
    #[br(parse_with = parse_opt_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub unk1: Option<ModelsExtraUnk1Inner>,

    // TODO: padding?
    pub unks: [u32; 4],
}

#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct ModelsExtraUnk1Inner {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    pub unk1: u32,

    #[br(parse_with = parse_ptr32)]
    #[br(args { offset: base_offset, inner: base_offset })]
    #[xc3(offset(u32))]
    pub unk2: ModelsExtraUnk1InnerUnk2,

    // TODO: padding?
    pub unks: [u32; 4],
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct ModelsExtraUnk1InnerUnk2 {
    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub unk1: Vec<[f32; 20]>,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub unk2: Vec<[f32; 14]>,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32), align(16))]
    pub unk3: Vec<[[[f32; 4]; 4]; 12]>,

    // TODO: padding?
    pub unks: [u32; 2],
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct Bone {
    #[br(parse_with = parse_string_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
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
pub struct SkinningIndices {
    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub indices: Vec<u16>,
}

#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct UnkBones {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    #[br(parse_with = parse_count32_offset32)]
    #[br(args { offset: base_offset, inner: base_offset })]
    #[xc3(count_offset(u32, u32))]
    pub bones: Vec<UnkBone>,

    // TODO: padding?
    pub unks: [u32; 5],
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct UnkBone {
    pub unk1: [f32; 3],

    #[br(parse_with = parse_string_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub name: String,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct Unk6 {
    #[br(parse_with = parse_string_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub name1: String,

    #[br(parse_with = parse_string_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub name2: String,

    pub unk1: u32, // TODO: count?
    pub unk2: u32, // TODO: offset into transforms?
}

#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct Unk9 {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    #[br(parse_with = parse_count32_offset32, offset = base_offset)]
    #[xc3(count_offset(u32, u32))]
    pub items: Vec<Unk9Item>,

    // TODO: padding?
    pub unks: [u32; 4],
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct Unk9Item {
    pub unk1: u32,
    pub unk2: u32,
    pub unk3: f32,
    pub unk4: f32,
    pub unk5: f32,
    pub unk6: f32,
    // TODO: padding?
    pub unk7: u32,
    pub unk8: u32,
    pub unk9: u32,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct Unk10 {
    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub unk1: Vec<u64>,

    // TODO: type?
    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub unk2: Vec<u64>,

    // TODO: type?
    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub unk3: Vec<[f32; 7]>,

    // TODO: padding?
    pub unks: [u32; 2],
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
    #[xc3(offset_count(u32, u32), align(8))]
    pub materials: Vec<Material>,

    pub unk1_1: u32,
    pub unk1_2: u32,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub work_values: Vec<f32>,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub shader_vars: Vec<(u16, u16)>,

    #[br(parse_with = parse_opt_ptr32)]
    #[br(args { offset: base_offset, inner: base_offset })]
    #[xc3(offset(u32))]
    pub callbacks: Option<MaterialCallbacks>,

    #[br(parse_with = parse_opt_ptr32)]
    #[br(args { offset: base_offset, inner: base_offset })]
    #[xc3(offset(u32))]
    pub unks1_3: Option<MaterialsUnk5>,

    #[br(parse_with = parse_offset32_count32, args { offset: base_offset, inner: base_offset })]
    #[xc3(offset_count(u32, u32))]
    pub techniques: Vec<Technique>,

    pub unks1_1: u32,

    #[br(parse_with = parse_opt_ptr32)]
    #[br(args { offset: base_offset, inner: base_offset })]
    #[xc3(offset(u32))]
    pub unk7: Option<MaterialsUnk7>,

    pub unk8: u32,

    // TODO: Is this the correct way to determine count?
    #[br(parse_with = parse_opt_ptr32)]
    #[br(args { offset: base_offset, inner: args! { count: alpha_texture_count(&materials) }})]
    #[xc3(offset(u32))]
    pub alpha_test_textures: Option<Vec<AlphaTestTexture>>,

    pub unks1_2_1: u32,
    pub unks1_2_2: u32,

    // TODO: is this always the overlay color parameter?
    #[br(parse_with = parse_opt_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub unks1_2_3: Option<[f32; 8]>,

    #[br(parse_with = parse_opt_ptr32)]
    #[br(args { offset: base_offset, inner: base_offset })]
    #[xc3(offset(u32))]
    pub unks1_2_4: Option<MaterialsUnk4>,

    #[br(parse_with = parse_opt_ptr32)]
    #[br(args {
        offset: base_offset,
        inner: args! { base_offset, count: materials.len() }
    })]
    #[xc3(offset(u32))]
    pub unks1_2_5: Option<MaterialsUnk6>,

    #[br(parse_with = parse_opt_ptr32)]
    #[br(args { offset: base_offset, inner: base_offset })]
    #[xc3(offset(u32))]
    pub unk2: Option<MaterialsUnk2>,

    #[br(parse_with = parse_opt_ptr32)]
    #[br(args { offset: base_offset, inner: base_offset })]
    #[xc3(offset(u32))]
    pub unk3: Option<MaterialsUnk3>,

    pub unk: [u32; 2],
}

// TODO: same as xc2?
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct MaterialCallbacks {
    // TODO: affects material parameter assignment?
    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub work_callbacks: Vec<(u16, u16)>,

    // TODO: Doesn't always include all materials?
    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub material_indices: Vec<u16>,

    // TODO: padding?
    pub unk: [u32; 6],
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

    pub flags: MaterialFlags,
    pub color: [f32; 4],
    pub unk2: [f32; 6],
    pub unk3: [f32; 3],

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub textures: Vec<Texture>,

    pub state_flags: StateFlags,

    pub m_unks1_1: u32,
    pub m_unks1_2: u32,
    pub m_unks1_3: u32,
    pub m_unks1_4: u32,

    pub work_value_start_index: u32,

    pub shader_var_start_index: u32,
    pub shader_var_count: u32,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub techniques: Vec<MaterialTechnique>,

    pub unk4: [u32; 4],

    pub unk5: u16,

    /// Index into [alpha_test_textures](struct.Materials.html#structfield.alpha_test_textures).
    pub alpha_test_texture_index: u16,

    pub unk7: u32,
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
    pub sampler: SamplerFlags,
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

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct MaterialsUnk3 {
    #[br(parse_with = parse_count32_offset32, offset = base_offset)]
    #[xc3(count_offset(u32, u32))]
    pub unk1: Vec<[u16; 4]>,

    #[br(parse_with = parse_count32_offset32, offset = base_offset)]
    #[xc3(count_offset(u32, u32))]
    pub unk2: Vec<[u16; 2]>,

    // TODO: one for each material?
    #[br(parse_with = parse_count32_offset32, offset = base_offset)]
    #[xc3(count_offset(u32, u32))]
    pub unk3: Vec<MaterialsUnk3Unk3>,

    pub unk: [u32; 4],
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct MaterialsUnk3Unk3 {
    pub unk1: u16,
    pub unk2: u16,
    pub unk3: u16,
    pub index: u16, // TODO: material index?
    pub unk5: u32,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct MaterialsUnk4 {
    #[br(parse_with = parse_count32_offset32, offset = base_offset)]
    #[xc3(count_offset(u32, u32))]
    pub unk1: Vec<[u16; 6]>,

    pub unk: [u32; 4],
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct MaterialsUnk5 {
    #[br(parse_with = parse_count32_offset32, offset = base_offset)]
    #[xc3(count_offset(u32, u32))]
    pub unk1: Vec<u32>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, PartialEq, Clone)]
#[br(import { base_offset: u64, count: usize })]
pub struct MaterialsUnk6 {
    // TODO: assigns items to each material?
    #[br(parse_with = parse_ptr32)]
    #[br(args { offset: base_offset, inner: args! { count } })]
    #[xc3(offset(u32))]
    pub unk1: Vec<u16>,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub unk2: Vec<MaterialsUnk6Unk2>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct MaterialsUnk6Unk2 {
    pub unk1: u32,
    pub unk2: [f32; 4],
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct MaterialsUnk7 {
    #[br(parse_with = parse_count32_offset32)]
    #[br(args { offset: base_offset, inner: base_offset })]
    #[xc3(count_offset(u32, u32))]
    pub items: Vec<MaterialsUnk7Item>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct MaterialsUnk7Item {
    pub unk1: u32,
    pub unk2: u32,
    pub unk3: u32,

    #[br(parse_with = parse_count32_offset32)]
    #[br(args { offset: base_offset, inner: base_offset })]
    #[xc3(count_offset(u32, u32))]
    pub unk4: Vec<MaterialsUnk7ItemUnk4>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct MaterialsUnk7ItemUnk4 {
    pub unk1: u32,

    #[br(parse_with = parse_count32_offset32, offset = base_offset)]
    #[xc3(count_offset(u32, u32))]
    pub unk2: Vec<[f32; 3]>,
}

// TODO: compare with decompiled shader data.
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[binread]
#[derive(Debug, Xc3Write, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct Technique {
    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub attributes: Vec<super::VertexAttribute>,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub unk2: Vec<[u16; 4]>,

    // offset1, offset2, count1, count2
    #[br(temp, restore_position)]
    offsets_counts: [u32; 4],

    #[br(parse_with = parse_opt_ptr32)]
    #[br(args { offset: base_offset, inner: args! { count: offsets_counts[2] as usize }})]
    #[xc3(offset(u32))]
    pub unk3: Option<Vec<u16>>,

    #[br(parse_with = parse_opt_ptr32)]
    #[br(args { offset: base_offset, inner: args! { count: offsets_counts[3] as usize }})]
    #[xc3(offset(u32))]
    pub unk4: Option<Vec<u16>>,

    pub unk3_count: u32,
    pub unk4_count: u32,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub unk7: Vec<[u16; 4]>,

    pub unk8: (u16, u16),
    pub unk9: (u16, u16),

    // TODO: padding?
    pub padding: [u32; 5],
}

#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, PartialEq, Clone)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct Unk1 {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    #[br(parse_with = parse_count32_offset32, offset = base_offset)]
    #[xc3(count_offset(u32, u32))]
    pub unk1: Vec<u32>,

    #[br(parse_with = parse_count32_offset32, offset = base_offset)]
    #[xc3(count_offset(u32, u32))]
    pub unk2: Vec<[u32; 3]>,

    #[br(parse_with = parse_count32_offset32, offset = base_offset)]
    #[xc3(count_offset(u32, u32))]
    pub unk3: Vec<[u32; 4]>,

    #[br(parse_with = parse_count32_offset32, offset = base_offset)]
    #[xc3(count_offset(u32, u32))]
    pub unk4: Vec<[f32; 4]>,

    pub unk5: [u32; 2],

    #[br(parse_with = parse_count32_offset32, offset = base_offset)]
    #[xc3(count_offset(u32, u32))]
    pub unk6: Vec<u32>,
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

    // TODO: Find a better way to handle buffer data?
    #[br(parse_with = parse_offset)]
    #[br(args {
        offset: base_offset + data_offset as u64,
        inner: args! { count: (vertex_count * vertex_size) as usize }
    })]
    #[xc3(save_position, skip)]
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

    // TODO: Find a better way to handle buffer data?
    #[br(parse_with = parse_offset)]
    #[br(args {
        offset: base_offset + data_offset as u64,
        inner: args! { count: (index_count * 2) as usize }
    })]
    #[xc3(save_position, skip)]
    pub data: Vec<u8>,
}

/// A collection of [Mtxt](crate::mtxt::Mtxt) textures embedded in the current file.
#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
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

// TODO: Is this actually identical to the one used for wimdo just read with a different endian?
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

// TODO: Nearly identical to legacy wimdo but not compressed?
#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct Streaming {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    pub unk1: u32,
    pub unk2: u32,

    #[br(args_raw(base_offset))]
    pub inner: StreamingDataLegacyInner<TextureUsage>,

    pub low_texture_data_offset: u32,
    pub low_texture_size: u32,
    pub texture_data_offset: u32,
    pub texture_size: u32,
}

#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
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

fn unk_float_count(skins: &[SkinningIndices]) -> usize {
    skins
        .iter()
        .flat_map(|s| s.indices.iter().map(|i| i + 1))
        .max()
        .unwrap_or_default() as usize
}

fn alpha_texture_count(materials: &[Material]) -> usize {
    materials
        .iter()
        .map(|m| m.alpha_test_texture_index as usize + 1)
        .max()
        .unwrap_or_default()
}

impl Xc3WriteOffsets for ModelsOffsets<'_> {
    type Args = ();

    fn write_offsets<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        _base_offset: u64,
        data_ptr: &mut u64,
        endian: xc3_write::Endian,
        _args: Self::Args,
    ) -> xc3_write::Xc3Result<()> {
        let base_offset = self.base_offset;
        self.models
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.skins
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.floats
            .write_full(writer, base_offset, data_ptr, endian, ())?;

        // TODO: handle this in a special type.
        let bone_name_base = *data_ptr;
        let bone_names = self
            .bone_names
            .write(writer, base_offset, data_ptr, endian)?;
        for n in bone_names.0 {
            n.name
                .write_full(writer, bone_name_base, data_ptr, endian, ())?;
        }

        // TODO: Are these two fields related?
        if !self.unk6.data.is_empty() {
            let unk6 = self.unk6.write(writer, base_offset, data_ptr, endian)?;
            self.unk7
                .write_full(writer, base_offset, data_ptr, endian, ())?;
            for u in unk6.0 {
                u.write_offsets(writer, base_offset, data_ptr, endian, ())?;
            }
        }

        // TODO: handle this in a special type.
        let bone_name_base = data_ptr.next_multiple_of(4);
        let bones = self.bones.write(writer, base_offset, data_ptr, endian)?;
        for b in bones.0 {
            b.name
                .write_full(writer, bone_name_base, data_ptr, endian, ())?;
        }

        // TODO: Where do these go?
        self.unk_bones
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.unk3
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.unk10
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.unk9
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.extra
            .write_offsets(writer, base_offset, data_ptr, endian, ())?;

        Ok(())
    }
}

impl Xc3WriteOffsets for MaterialsOffsets<'_> {
    type Args = ();

    fn write_offsets<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        _base_offset: u64,
        data_ptr: &mut u64,
        endian: xc3_write::Endian,
        _args: Self::Args,
    ) -> xc3_write::Xc3Result<()> {
        let base_offset = self.base_offset;

        let materials = self
            .materials
            .write(writer, base_offset, data_ptr, endian)?;

        self.work_values
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.shader_vars
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.callbacks
            .write_full(writer, base_offset, data_ptr, endian, ())?;

        for m in &materials.0 {
            m.techniques
                .write_full(writer, base_offset, data_ptr, endian, ())?;
        }
        for m in &materials.0 {
            m.textures
                .write_full(writer, base_offset, data_ptr, endian, ())?;
        }

        self.alpha_test_textures
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.unk3
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.unks1_2_3
            .write_full(writer, base_offset, data_ptr, endian, ())?;

        self.techniques
            .write_full(writer, base_offset, data_ptr, endian, ())?;

        self.unks1_3
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.unk7
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.unks1_2_4
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.unks1_2_5
            .write_full(writer, base_offset, data_ptr, endian, ())?;

        for m in materials.0 {
            m.name
                .write_full(writer, base_offset, data_ptr, endian, ())?;
        }
        Ok(())
    }
}

impl Xc3WriteOffsets for MaterialsUnk6Offsets<'_> {
    type Args = ();

    fn write_offsets<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
        endian: xc3_write::Endian,
        _args: Self::Args,
    ) -> xc3_write::Xc3Result<()> {
        // Different order than field order.
        self.unk2
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.unk1
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        Ok(())
    }
}

impl Xc3WriteOffsets for TechniqueOffsets<'_> {
    type Args = ();

    fn write_offsets<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
        endian: xc3_write::Endian,
        _args: Self::Args,
    ) -> xc3_write::Xc3Result<()> {
        // Different order than field order.
        self.attributes
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.unk3
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.unk4
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.unk7
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.unk2
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        Ok(())
    }
}

impl Xc3WriteOffsets for Unk1Offsets<'_> {
    type Args = ();

    fn write_offsets<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        _base_offset: u64,
        data_ptr: &mut u64,
        endian: xc3_write::Endian,
        _args: Self::Args,
    ) -> xc3_write::Xc3Result<()> {
        let base_offset = self.base_offset;
        self.unk1
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.unk2
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.unk3
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.unk4
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        if !self.unk6.data.is_empty() {
            self.unk6
                .write_full(writer, base_offset, data_ptr, endian, ())?;
        }
        Ok(())
    }
}

impl Xc3WriteOffsets for VertexDataOffsets<'_> {
    type Args = ();

    fn write_offsets<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        _base_offset: u64,
        data_ptr: &mut u64,
        endian: xc3_write::Endian,
        _args: Self::Args,
    ) -> xc3_write::Xc3Result<()> {
        let base_offset = self.base_offset;
        let vertex_buffers = self
            .vertex_buffers
            .write(writer, base_offset, data_ptr, endian)?;
        let index_buffers = self
            .index_buffers
            .write(writer, base_offset, data_ptr, endian)?;
        for b in &vertex_buffers.0 {
            b.attributes
                .write_full(writer, base_offset, data_ptr, endian, ())?;
        }

        // TODO: Store a shared buffer section and don't assume offset ordering?
        writer.seek(SeekFrom::Start(base_offset + 4096))?;
        for b in vertex_buffers.0 {
            writer.write_all(b.data.data)?;
        }
        for b in index_buffers.0 {
            writer.write_all(b.data.data)?;
        }
        *data_ptr = writer.stream_position()?;
        Ok(())
    }
}
