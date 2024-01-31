//! Model data in `.wimdo` files.
//!
//! # File Paths
//! | Game | File Patterns |
//! | --- | --- |
//! | Xenoblade Chronicles 1 DE | `chr/{en,np,obj,pc,wp}/*.wimdo`, `monolib/shader/*.wimdo` |
//! | Xenoblade Chronicles 2 | `model/{bl,en,np,oj,pc,we,wp}/*.wimdo`, `monolib/shader/*.wimdo` |
//! | Xenoblade Chronicles 3 | `chr/{bt,ch,en,oj,wp}/*.wimdo`, `map/*.wimdo`, `monolib/shader/*.wimdo` |
use crate::{
    msrd::Streaming,
    parse_count32_offset32, parse_offset32_count32, parse_opt_ptr32, parse_ptr32,
    parse_string_opt_ptr32, parse_string_ptr32,
    spch::Spch,
    vertex::{DataType, VertexData},
    xc3_write_binwrite_impl,
};
use bilge::prelude::*;
use binrw::{args, binread, BinRead, BinWrite};
use xc3_write::{Xc3Write, Xc3WriteOffsets};

#[derive(Debug, BinRead, Xc3Write, Clone)]
#[br(magic(b"DMXM"))]
#[xc3(magic(b"DMXM"))]
pub struct Mxmd {
    // TODO: 10111 for xc2 has different fields
    #[br(assert(version == 10111 || version == 10112))]
    pub version: u32,

    // TODO: only aligned to 16 for 10112?
    // TODO: support expressions for alignment?
    /// A collection of [Model] and associated data.
    #[br(parse_with = parse_ptr32, args { inner: version })]
    #[xc3(offset(u32), align(16))]
    pub models: Models,

    /// A collection of [Material] and associated data.
    #[br(parse_with = parse_ptr32)]
    #[xc3(offset(u32), align(16))]
    pub materials: Materials,

    #[br(parse_with = parse_opt_ptr32)]
    #[xc3(offset(u32), align(16))]
    pub unk1: Option<Unk1>,

    /// Embedded vertex data for .wimdo only models with no .wismt.
    #[br(parse_with = parse_opt_ptr32)]
    #[xc3(offset(u32))]
    pub vertex_data: Option<VertexData>,

    /// Embedded shader data for .wimdo only models with no .wismt.
    #[br(parse_with = parse_opt_ptr32)]
    #[xc3(offset(u32))]
    pub spch: Option<Spch>,

    /// Textures included within this file.
    #[br(parse_with = parse_opt_ptr32)]
    #[xc3(offset(u32))]
    pub packed_textures: Option<PackedTextures>,

    pub unk5: u32,

    /// Streaming information for the `wismt` file or [None] if no `wismt` file.
    /// Identical to the same field in the corresponding [Msrd](crate::msrd::Msrd).
    #[br(parse_with = parse_opt_ptr32)]
    #[xc3(offset(u32))]
    pub streaming: Option<Streaming>,

    // TODO: padding?
    pub unk: [u32; 9],
}

// TODO: more strict alignment for xc3?
// TODO: 108 bytes for xc2 and 112 bytes for xc3?
/// A collection of [Material], [Sampler], and material parameters.
#[binread]
#[derive(Debug, Xc3Write, Clone)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct Materials {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    // TODO: Sometimes 108 and sometimes 112?
    #[br(parse_with = parse_offset32_count32, args { offset: base_offset, inner: base_offset })]
    #[xc3(offset_count(u32, u32), align(4))]
    pub materials: Vec<Material>,

    // offset?
    pub unk1: u32,
    pub unk2: u32,

    // TODO: Materials have offsets into these arrays for parameter values?
    // material body has a uniform at shader offset 64 but offset 48 in this floats buffer
    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32), align(4))]
    pub floats: Vec<f32>, // work values?

    // TODO: final number counts up from 0?
    // TODO: Some sort of index or offset?
    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub ints: Vec<(u16, u16)>, // shader vars (u8, u8, u16)?

    #[br(parse_with = parse_opt_ptr32)]
    #[br(args { offset: base_offset, inner: base_offset })]
    #[xc3(offset(u32))]
    pub material_unk1: Option<MaterialUnk1>, // callbacks?

    // TODO: is this ever not 0?
    pub unk4: u32,

    /// Info for each of the shaders in the associated [Spch](crate::spch::Spch).
    #[br(parse_with = parse_offset32_count32, args { offset: base_offset, inner: base_offset })]
    #[xc3(offset_count(u32, u32))]
    pub shader_programs: Vec<ShaderProgramInfo>,

    pub unks1: [u32; 2],

    #[br(parse_with = parse_count32_offset32, offset = base_offset)]
    #[xc3(count_offset(u32, u32))]
    pub alpha_test_textures: Vec<AlphaTestTexture>,

    // TODO: extra fields that go before samplers?
    pub unks3: [u32; 3],

    #[br(parse_with = parse_opt_ptr32)]
    #[br(args { offset: base_offset, inner: base_offset })]
    #[xc3(offset(u32))]
    pub material_unk2: Option<MaterialUnk2>,

    #[br(parse_with = parse_opt_ptr32)]
    #[br(args { offset: base_offset, inner: base_offset })]
    #[xc3(offset(u32))]
    pub material_unk3: Option<MaterialUnk3>,

    pub unks3_1: [u32; 2],

    #[br(parse_with = parse_opt_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub samplers: Option<Samplers>,

    // TODO: padding?
    pub unks4: [u32; 3],
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, Clone)]
pub struct AlphaTestTexture {
    // TODO: (_, 0, 1) has alpha testing?
    // TODO: Test different param values?
    pub texture_index: u16,
    pub unk1: u16,
    pub unk2: u32,
}

/// `ml::MdsMatTechnique` in the Xenoblade 2 binary.
#[derive(Debug, BinRead, Xc3Write, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct ShaderProgramInfo {
    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub attributes: Vec<VertexAttribute>,

    pub unk3: u32, // 0
    pub unk4: u32, // 0

    // work values?
    // TODO: matches up with uniform parameters for U_Mate?
    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub parameters: Vec<MaterialParameter>, // var table?

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub textures: Vec<u16>, // textures?

    // ssbos and then uniform buffers ordered by handle?
    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub uniform_blocks: Vec<(u16, u16)>, // uniform blocks?

    pub unk11: u32, // material texture count?

    pub unk12: u16, // counts up from 0?
    pub unk13: u16, // unk11 + unk12?

    // TODO: padding?
    pub padding: [u32; 5],
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, Clone)]
pub struct VertexAttribute {
    pub data_type: DataType,
    pub relative_offset: u16,
    pub buffer_index: u16,
    pub unk4: u16, // always 0?
}

/// `ml::MdsMatVariableTbl` in the Xenoblade 2 binary.
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, Clone)]
pub struct MaterialParameter {
    pub param_type: ParamType,
    pub floats_index_offset: u16, // added to floats start index?
    pub unk: u16,
    pub count: u16, // actual number of bytes depends on type?
}

#[derive(Debug, BinRead, BinWrite, Clone, Copy, PartialEq, Eq, Hash)]
#[brw(repr(u16))]
pub enum ParamType {
    Unk0 = 0,
    /// `gTexMat` uniform in the [Spch] and
    /// `ml::DrMdoSetup::unimate_texMatrix` in the Xenoblade 2 binary.
    TexMatrix = 1,
    /// `gWrkFl4[0]` uniform in the [Spch] and
    /// `ml::DrMdoSetup::unimate_workFloat4` in the Xenoblade 2 binary.
    WorkFloat4 = 2,
    /// `gWrkCol` uniform in the [Spch] and
    /// `ml::DrMdoSetup::unimate_workColor` in the Xenoblade 2 binary.
    WorkColor = 3,
    Unk4 = 4,
    /// `gAlInf` uniform in the [Spch] and
    /// `ml::DrMdoSetup::unimate_alphaInfo` in the Xenoblade 2 binary.
    Unk5 = 5,
    Unk6 = 6,
    Unk7 = 7,
    /// `gToonHeadMat` uniform in the [Spch].
    Unk10 = 10,
}

// TODO: Does this affect texture assignment order?
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct MaterialUnk1 {
    // count matches up with Material.unk_start_index?
    // TODO: affects material parameter assignment?
    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub unk1: Vec<(u16, u16)>,

    // 0 1 2 ... material_count - 1
    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub unk2: Vec<u16>,

    // TODO: padding?
    pub unk: [u32; 8],
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct MaterialUnk2 {
    #[br(parse_with = parse_count32_offset32, offset = base_offset)]
    #[xc3(count_offset(u32, u32))]
    pub unk1: Vec<[u32; 3]>,

    // TODO: padding?
    pub unk: [u32; 4],
}

#[derive(Debug, BinRead, Xc3Write, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct MaterialUnk3 {
    #[br(parse_with = parse_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub unk1: [u32; 8],

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub unk2: Vec<[f32; 5]>,

    // TODO: padding?
    pub unk: [u32; 4],
}

/// A collection of [Sampler].
#[binread]
#[derive(Debug, Xc3Write, Xc3WriteOffsets, Clone)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct Samplers {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    #[br(parse_with = parse_count32_offset32, offset = base_offset)]
    #[xc3(count_offset(u32, u32))]
    pub samplers: Vec<Sampler>,

    // TODO: padding?
    pub unk: [u32; 2],
}

/// State for controlling how textures are sampled.
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, Clone)]
pub struct Sampler {
    pub flags: SamplerFlags,

    // Is this actually a float?
    pub unk2: f32,
}

/// Texture sampler settings for addressing and filtering.
#[bitsize(32)]
#[derive(DebugBits, FromBits, BinRead, BinWrite, Clone, Copy)]
#[br(map = u32::into)]
#[bw(map = |&x| u32::from(x))]
pub struct SamplerFlags {
    /// Sets wrap U to repeat when `true`.
    pub repeat_u: bool,
    /// Sets wrap V to repeat when `true`.
    pub repeat_v: bool,
    /// Sets wrap U to mirrored repeat when `true` regardless of repeat U.
    pub mirror_u: bool,
    /// Sets wrap V to mirrored repeat when `true` regardless of repeat V.
    pub mirror_v: bool,
    /// Sets min and mag filter to nearest when `true`.
    /// The min filter also depends on disable_mipmap_filter.
    pub nearest: bool,
    /// Sets all wrap modes to clamp and min and mag filter to linear.
    /// Ignores the values of previous flags.
    pub force_clamp: bool,
    /// Removes the mipmap nearest from the min filter when `true`.
    pub disable_mipmap_filter: bool,
    pub unk1: bool,
    pub unk3: bool,
    pub unk: u23,
}

/// A single material assignable to a [Mesh].
/// `ml::mdsMatInfoHeader` in the Xenoblade 2 binary.
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct Material {
    #[br(parse_with = parse_string_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub name: String,

    pub flags: MaterialFlags,

    pub render_flags: u32,

    /// Color multiplier value assigned to the `gMatCol` shader uniform.
    pub color: [f32; 4],

    // TODO: final byte controls reference?
    pub alpha_test_ref: [u8; 4],

    // TODO: materials with zero textures?
    /// Defines the shader's sampler bindings in order for s0, s1, s2, ...
    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub textures: Vec<Texture>,

    // TODO: rename to pipeline state?
    pub state_flags: StateFlags,

    // group indices?
    pub m_unks1_1: u32,
    pub m_unks1_2: u32,
    pub m_unks1_3: u32,
    pub m_unks1_4: u32,

    pub floats_start_index: u32, // work value index?

    // TODO: starts with a small number and then some random ints?
    pub ints_start_index: u32,
    pub ints_count: u32,

    // always count 1?
    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub shader_programs: Vec<ShaderProgram>,

    pub unk5: u32,

    // index for MaterialUnk1.unk1?
    // work callbacks?
    pub unk_start_index: u16, // sum of previous unk_count?
    pub unk_count: u16,

    // TODO: alt textures offset for non opaque rendering?
    pub m_unks2: [u16; 3],

    /// Index into [alpha_test_textures](struct.Materials.html#structfield.alpha_test_textures).
    pub alpha_test_texture_index: u16,
    pub m_unks3: [u16; 8],
}

#[bitsize(32)]
#[derive(DebugBits, FromBits, BinRead, BinWrite, Clone, Copy)]
#[br(map = u32::into)]
#[bw(map = |&x| u32::from(x))]
pub struct MaterialFlags {
    pub unk1: bool,
    pub unk2: bool,
    /// Enables alpha testing from a texture when `true`.
    pub alpha_mask: bool,
    /// Samples `texture.x` from a dedicated mask texture when `true`.
    /// Otherwise, the alpha channel is used.
    pub separate_mask: bool,
    pub unk: u28,
}

/// Flags controlling pipeline state for rasterizer and fragment state.
#[derive(Debug, BinRead, BinWrite, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StateFlags {
    pub flag0: u8, // depth write?
    pub blend_state: BlendState,
    pub cull_mode: CullMode,
    pub flag3: u8, // unused?
    pub stencil_state1: StencilState1,
    pub stencil_state2: StencilState2,
    pub depth_func: DepthFunc,
    pub flag7: u8, // color writes?
}

// TODO: Convert these to equations for RGB and alpha for docs.
// TODO: Is it worth documenting this outside of xc3_wgpu?
// flag, col src, col dst, col op, alpha src, alpha dst, alpha op
// 0 = disabled
// 1, Src Alpha, 1 - Src Alpha, Add, Src Alpha, 1 - Src Alpha, Add
// 2, Src Alpha, One, Add, Src Alpha, One, Add
// 3, Zero, Src Col, Add, Zero, Src Col, Add
// 6, disabled + ???
#[derive(Debug, BinRead, BinWrite, Clone, Copy, PartialEq, Eq, Hash)]
#[brw(repr(u8))]
pub enum BlendState {
    Disabled = 0,
    AlphaBlend = 1,
    Additive = 2,
    Multiplicative = 3,
    Unk6 = 6, // also disabled?
}

// TODO: Get the actual stencil state from RenderDoc.
// 0 = disables hair blur stencil stuff?
// 4 = disables hair but different ref value?
// 16 = enables hair blur stencil stuff?
#[derive(Debug, BinRead, BinWrite, Clone, Copy, PartialEq, Eq, Hash)]
#[brw(repr(u8))]
pub enum StencilState1 {
    Always = 0,
    Unk1 = 1,
    Always2 = 4,
    Unk5 = 5,
    Unk8 = 8,
    Unk9 = 9,
    UnkHair = 16,
    Unk20 = 20,
}

// TODO: Does this flag actually disable stencil?
#[derive(Debug, BinRead, BinWrite, Clone, Copy, PartialEq, Eq, Hash)]
#[brw(repr(u8))]
pub enum StencilState2 {
    Disabled = 0,
    Enabled = 1,
    Unk2 = 2,
    Unk6 = 6,
    Unk7 = 7,
    Unk8 = 8,
}

#[derive(Debug, BinRead, BinWrite, Clone, Copy, PartialEq, Eq, Hash)]
#[brw(repr(u8))]
pub enum DepthFunc {
    Disabled = 0,
    LessEqual = 1,
    Equal = 3,
}

#[derive(Debug, BinRead, BinWrite, Clone, Copy, PartialEq, Eq, Hash)]
#[brw(repr(u8))]
pub enum CullMode {
    Back = 0,
    Front = 1,
    Disabled = 2,
    Unk3 = 3, // front + ???
}

/// `ml::MdsMatMaterialTechnique` in the Xenoblade 2 binary.
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, Clone)]
pub struct ShaderProgram {
    /// Index into [shader_programs](struct.Materials.html#structfield.shader_programs).
    pub program_index: u32,
    pub unk_type: RenderPassType,
    pub material_buffer_index: u16,
    pub flags: u32, // always 1?
}

/// Determines the render pass for an object.
// Each "pass" has different render targets?
// _trans = 1,
// _ope = 0,1,7
// _zpre = 0
// _outline = 0
#[derive(Debug, BinRead, BinWrite, PartialEq, Eq, Clone, Copy, Hash)]
#[brw(repr(u16))]
pub enum RenderPassType {
    Unk0 = 0, // main opaque + some transparent?
    Unk1 = 1, // second layer transparent?
    Unk6 = 6, // used for maps?
    Unk7 = 7, // additional eye effect layer?
    Unk9 = 9, // used for maps?
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, Clone)]
pub struct Texture {
    /// Index into the textures in [streaming](struct.Mxmd.html#structfield.streaming)
    /// or [packed_textures](struct.Mxmd.html#structfield.packed_textures).
    pub texture_index: u16,
    /// Index into the samplers in [samplers](struct.Materials.html#structfield.samplers).
    pub sampler_index: u16,
    pub unk2: u16,
    pub unk3: u16,
}

// TODO: variable size?
// xc1: 160, 164, 168 bytes
// xc2: 160 bytes
// xc3: 160, 164, 168, 200, 204 bytes
/// A collection of [Model] as well as skinning and animation information.
#[binread]
#[derive(Debug, Xc3Write, Clone)]
#[br(stream = r)]
#[br(import_raw(version: u32))]
#[xc3(base_offset)]
pub struct Models {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    // TODO: Default value for version arg?
    #[br(if(version != 10111))]
    pub models_flags: Option<ModelsFlags>,

    pub max_xyz: [f32; 3],
    pub min_xyz: [f32; 3],

    // TODO: temp?
    #[br(restore_position)]
    #[xc3(skip)]
    pub models_offset: u32,

    #[br(parse_with = parse_offset32_count32, args { offset: base_offset, inner: base_offset })]
    #[xc3(offset_count(u32, u32))]
    pub models: Vec<Model>,

    pub unk2: u32,

    #[br(parse_with = parse_opt_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub skinning: Option<Skinning>,

    pub unks3_1: [u32; 14],

    // offset 100
    #[br(parse_with = parse_offset32_count32, args { offset: base_offset, inner: base_offset })]
    #[xc3(offset_count(u32, u32), align(16))]
    pub ext_meshes: Vec<ExtMesh>,

    // TODO: always 0?
    // TODO: offset for 10111?
    pub unks3_2: [u32; 2],

    #[br(parse_with = parse_opt_ptr32)]
    #[br(args { offset: base_offset, inner: base_offset })]
    #[xc3(offset(u32))]
    pub model_unk8: Option<ModelUnk8>,

    pub unk3_3: u32,

    #[br(parse_with = parse_opt_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub model_unk7: Option<ModelUnk7>,

    // offset 128
    #[br(parse_with = parse_opt_ptr32, offset = base_offset)]
    #[xc3(offset(u32), align(16))]
    pub morph_controllers: Option<MorphControllers>,

    #[br(parse_with = parse_opt_ptr32, offset = base_offset)]
    #[xc3(offset(u32), align(16))]
    pub model_unk1: Option<ModelUnk1>,

    #[br(parse_with = parse_opt_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub model_unk3: Option<ModelUnk3>,

    // TODO: not always aligned to 16?
    #[br(parse_with = parse_opt_ptr32, offset = base_offset)]
    #[xc3(offset(u32), align(16))]
    pub lod_data: Option<LodData>,

    #[br(parse_with = parse_opt_ptr32, offset = base_offset)]
    #[xc3(offset(u32), align(16))]
    pub model_unk4: Option<ModelUnk4>,
    pub unk_field2: u32,

    // TODO: only for 10111?
    // TODO: offset for 10112?
    // #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    // #[xc3(offset_count(u32, u32))]
    // pub model_unk9: Vec<ModelUnk9>,
    pub model_unk9: [u32; 2],
    // TODO: What controls the up to 44 optional bytes?
    // TODO: How to estimate models offset from these fields?
    // offset 160
    // TODO: Investigate extra data for legacy mxmd files.
    #[br(args { size: models_offset, base_offset})]
    #[br(if(version > 10111))]
    pub extra: Option<ModelsExtraData>,
}

// Use an enum since even the largest size can have all offsets as null.
// i.e. the nullability of the offsets does not determine the size.
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, Clone)]
#[br(import { size: u32, base_offset: u64 })]
pub enum ModelsExtraData {
    #[br(pre_assert(size == 160))]
    Unk1,

    #[br(pre_assert(size == 164))]
    Unk2(#[br(args_raw(base_offset))] ModelsExtraDataUnk2),

    #[br(pre_assert(size == 168))]
    Unk3(#[br(args_raw(base_offset))] ModelsExtraDataUnk3),

    #[br(pre_assert(size == 200))]
    Unk4(#[br(args_raw(base_offset))] ModelsExtraDataUnk4),

    #[br(pre_assert(size == 204))]
    Unk5(#[br(args_raw(base_offset))] ModelsExtraDataUnk5),
}

// TODO: add asserts to all padding fields?
// 164 total bytes
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct ModelsExtraDataUnk2 {
    #[br(parse_with = parse_opt_ptr32)]
    #[br(args { offset: base_offset, inner: base_offset })]
    #[xc3(offset(u32))]
    pub model_unk10: Option<ModelUnk10>,
}

// 168 total bytes
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct ModelsExtraDataUnk3 {
    #[br(parse_with = parse_opt_ptr32)]
    #[br(args { offset: base_offset, inner: base_offset })]
    #[xc3(offset(u32))]
    pub model_unk10: Option<ModelUnk10>,

    #[br(parse_with = parse_opt_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub model_unk5: Option<ModelUnk5>,
}

// 200 total bytes
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct ModelsExtraDataUnk4 {
    #[br(parse_with = parse_opt_ptr32)]
    #[br(args { offset: base_offset, inner: base_offset })]
    #[xc3(offset(u32))]
    pub model_unk10: Option<ModelUnk10>,

    #[br(parse_with = parse_opt_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub model_unk5: Option<ModelUnk5>,

    #[br(parse_with = parse_opt_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub model_unk6: Option<ModelUnk6>,

    // TODO: padding?
    pub unk: Option<[u32; 7]>,
}

// 204 total bytes
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct ModelsExtraDataUnk5 {
    #[br(parse_with = parse_opt_ptr32)]
    #[br(args { offset: base_offset, inner: base_offset })]
    #[xc3(offset(u32))]
    pub model_unk10: Option<ModelUnk10>,

    #[br(parse_with = parse_opt_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub model_unk5: Option<ModelUnk5>,

    #[br(parse_with = parse_opt_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub model_unk6: Option<ModelUnk6>,

    // TODO: padding?
    pub unk: Option<[u32; 8]>,
}

/// A collection of meshes where each [Mesh] represents one draw call.
///
/// Each [Model] has an associated [VertexData] containing vertex and index buffers.
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct Model {
    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub meshes: Vec<Mesh>,

    pub unk1: u32,
    pub max_xyz: [f32; 3],
    pub min_xyz: [f32; 3],
    pub bounding_radius: f32,
    pub unks: [u32; 7],
}

/// Flags and resources associated with a single draw call.
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, Clone)]
pub struct Mesh {
    pub render_flags: u32,
    pub skin_flags: u32, // 0x1, 0x2, 0x4001 (16385), 0x4008 (16392)
    /// Index into [vertex_buffers](../vertex/struct.VertexData.html#structfield.vertex_buffers)
    /// for the associated [VertexData].
    pub vertex_buffer_index: u16,
    /// Index into [index_buffers](../vertex/struct.VertexData.html#structfield.index_buffers)
    /// for the associated [VertexData].
    pub index_buffer_index: u16,
    pub unk_index: u16,
    /// Index into [materials](struct.Materials.html#structfield.materials).
    pub material_index: u16,
    pub unk2: u32,
    pub unk3: u16,
    /// Index into [ext_meshes](struct.Models.html#structfield.ext_meshes).
    // TODO: enabled via a flag?
    pub ext_mesh_index: u16,
    pub unk4: u32,
    pub unk5: u16,
    /// The index of the level of detail typically starting from 1.
    pub lod: u16, // TODO: flags with one byte being lod?
    pub alpha_table_index: u16,
    pub unk6: u16,
    pub unk7: i32,
    pub unk8: i32,
    pub unk9: i32,
}

/// Flags to determine what data is present in [Models].
#[bitsize(32)]
#[derive(DebugBits, FromBits, BinRead, BinWrite, Clone, Copy)]
#[br(map = u32::into)]
#[bw(map = |&x| u32::from(x))]
pub struct ModelsFlags {
    pub unk1: bool,
    pub has_model_unk8: bool,
    pub unk3: bool,
    pub unk4: bool,
    pub unk5: bool,
    pub unk6: bool,
    pub has_model_unk7: bool,
    pub unk8: bool,
    pub unk9: bool,
    pub unk10: bool,
    pub has_morph_controllers: bool,
    pub has_model_unk1: bool,
    pub has_model_unk3: bool,
    pub unk14: bool,
    pub unk15: bool,
    pub has_skinning: bool,
    pub unk17: bool,
    pub has_lod_data: bool,
    pub has_model_unk4: bool,
    pub unk20: bool,
    pub unk21: bool,
    pub unk22: bool,
    pub unk23: bool,
    pub unk24: bool,
    pub unk25: bool,
    pub unk26: bool,
    pub unk27: bool,
    pub unk28: bool,
    pub unk29: bool,
    pub unk30: bool,
    pub unk31: bool,
    pub unk32: bool,
}

/// `ExtMesh` in the Xenoblade 2 binary.
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct ExtMesh {
    #[br(parse_with = parse_string_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub name1: String,

    // TODO: Always an empty string?
    #[br(parse_with = parse_string_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub name2: String,

    pub flags: ExtMeshFlags,
    pub unk2: u16,
    pub unk3: u32,
}

#[bitsize(16)]
#[derive(DebugBits, FromBits, BinRead, BinWrite, Clone, Copy)]
#[br(map = u16::into)]
#[bw(map = |&x| u16::from(x))]
pub struct ExtMeshFlags {
    pub unk1: bool, // true
    pub unk2: bool, // false
    pub unk3: bool, // false
    /// Whether to initially skip rendering assigned meshes.
    pub start_hidden: bool,
    pub unk5: bool,
    pub unk: u11, // 0
}

#[binread]
#[derive(Debug, Xc3Write, Xc3WriteOffsets, Clone)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct MorphControllers {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    // TODO: same count as morph targets per descriptor in vertex data?
    #[br(parse_with = parse_offset32_count32, args { offset: base_offset, inner: base_offset })]
    #[xc3(offset_count(u32, u32))]
    controllers: Vec<MorphController>,

    unk1: u32,

    // TODO: padding?
    unk: [u32; 3],
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct MorphController {
    #[br(parse_with = parse_string_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    name1: String,

    #[br(parse_with = parse_string_opt_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    name2: Option<String>,

    unk1: u16,
    unk2: u16, // index?
    unk3: u16, // 0?
    unk4: u16,

    // TODO: padding?
    unk: [u32; 3],
}

#[binread]
#[derive(Debug, Xc3Write, Xc3WriteOffsets, Clone)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct ModelUnk3 {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    #[br(parse_with = parse_count32_offset32, args { offset: base_offset, inner: base_offset })]
    #[xc3(count_offset(u32, u32))]
    pub items: Vec<ModelUnk3Item>,

    // TODO: padding?
    pub unk: [u32; 4],
}

#[derive(Debug, BinRead, Xc3Write, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct ModelUnk3Item {
    // DECL_GBL_CALC
    #[br(parse_with = parse_string_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub name: String,
    pub unk1: u32, // 0?
    pub unk2: u32,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub unk3: Vec<u16>,
}

#[binread]
#[derive(Debug, Xc3Write, Xc3WriteOffsets, Clone)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct ModelUnk4 {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    // (index, group index)?
    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub items: Vec<(u16, u16)>,

    // TODO: padding?
    pub unks: [u32; 4],
}

#[binread]
#[derive(Debug, Xc3Write, Xc3WriteOffsets, Clone)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct ModelUnk5 {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    // TODO: DS_ names?
    #[br(parse_with = parse_count32_offset32)]
    #[br(args { offset: base_offset, inner: base_offset })]
    #[xc3(count_offset(u32, u32))]
    pub items: Vec<StringOffset>,

    // TODO: padding?
    pub unks: [u32; 4],
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct StringOffset {
    #[br(parse_with = parse_string_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub name: String,
}

#[binread]
#[derive(Debug, Xc3Write, Xc3WriteOffsets, Clone)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct ModelUnk6 {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    // TODO: What type is this?
    #[br(parse_with = parse_count32_offset32, offset = base_offset)]
    #[xc3(count_offset(u32, u32))]
    pub items: Vec<[u32; 2]>,

    // TODO: padding?
    pub unks: [u32; 4],
}

#[binread]
#[derive(Debug, Xc3Write, Xc3WriteOffsets, Clone)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct ModelUnk7 {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    // TODO: What type is this?
    #[br(parse_with = parse_count32_offset32, offset = base_offset)]
    #[xc3(count_offset(u32, u32))]
    pub items: Vec<[f32; 9]>,

    // TODO: padding?
    pub unks: [u32; 4],
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct ModelUnk8 {
    // TODO: What type is this?
    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub unk1: Vec<[u32; 5]>,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub unk2: Vec<[f32; 4]>,

    // TODO: padding?
    pub unks: [u32; 2],
}

#[binread]
#[derive(Debug, Xc3Write, Xc3WriteOffsets, Clone)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct ModelUnk9 {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    #[br(parse_with = parse_count32_offset32, args { offset: base_offset, inner: base_offset })]
    #[xc3(count_offset(u32, u32))]
    pub items: Vec<ModelUnk9Item>,

    // TODO: padding?
    pub unk: [u32; 4],
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct ModelUnk10 {
    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub unk1: Vec<u32>,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct ModelUnk9Item {
    #[br(parse_with = parse_string_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub name: String,

    pub unk1: u32,
    pub unk2: u32,
    pub unk3: u32,
    pub unk4: u32,
}

// TODO: eye animations?
// TODO: Some sort of animation?
#[binread]
#[derive(Debug, Xc3Write, Clone)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct ModelUnk1 {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    #[br(parse_with = parse_offset32_count32)]
    #[br(args { offset: base_offset, inner: base_offset })]
    #[xc3(offset_count(u32, u32))]
    pub items1: Vec<ModelUnk1Item1>,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub items2: Vec<ModelUnk1Item2>,

    #[br(parse_with = parse_ptr32)]
    #[br(args { offset: base_offset, inner: args! { count: items1.len() }})]
    #[xc3(offset(u32))]
    pub items3: Vec<f32>,
    pub unk1: u32, // 0 or 1?

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub items4: Vec<[u32; 5]>,

    // flags?
    pub unk4: u32,
    pub unk5: u32,
    // TODO: not present for xc2?
    // TODO: Is this the correct check?
    #[br(if(unk4 != 0 || unk5 != 0))]
    #[br(args_raw(base_offset))]
    pub extra: Option<ModelUnk1Extra>,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct ModelUnk1Extra {
    #[br(parse_with = parse_opt_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub unk_inner: Option<ModelUnk1Inner>,

    // TODO: only 12 bytes for chr/ch/ch01022012.wimdo?
    pub unk: [u32; 4],
}

#[binread]
#[derive(Debug, Xc3Write, Xc3WriteOffsets, Clone)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct ModelUnk1Inner {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub items1: Vec<(u16, u16)>,

    // 0..N-1 arranged in a different order?
    #[br(parse_with = parse_ptr32)]
    #[br(args {
        offset: base_offset,
        inner: args! {
            count: items1.iter().map(|(a,_)| *a).max().unwrap_or_default() as usize
        }
    })]
    #[xc3(offset(u32))]
    pub unk_offset: Vec<u16>,

    // TODO: padding?
    pub unks: [u32; 5],
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct ModelUnk1Item1 {
    #[br(parse_with = parse_string_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub name: String,
    // TODO: padding?
    pub unk: [u32; 3],
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, Clone)]
pub struct ModelUnk1Item2 {
    pub unk1: u32,
    pub unk2: u32,
    pub unk3: u32,
    pub unk4: u32,
    pub unk5: u32,
}

#[binread]
#[derive(Debug, Xc3Write, Clone)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct LodData {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    pub unk1: u32, // 0?

    // TODO: Count related to number of mesh lod values?
    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32), align(8))]
    pub items1: Vec<LodItem1>,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub groups: Vec<LodGroup>,

    pub unks: [u32; 4],
}

// TODO: is lod: 0 in the mxmd special?
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, Clone)]
pub struct LodItem1 {
    pub unk1: [u32; 4],
    pub unk2: f32,
    // second element is index related to count in LodItem2?
    // [0,0,1,0], [0,1,1,0], [0,2,1,0], ...
    pub unk3: [u8; 4],
    pub unk4: [u32; 2],
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, Clone)]
pub struct LodGroup {
    /// One minus the [lod](struct.Mesh.html#structfield.lod) for [Mesh] with the highest level of detail.
    pub base_lod_index: u16,
    /// The number of LOD levels in this group.
    pub lod_count: u16,
    // TODO: padding?
    pub unk1: u32,
    pub unk2: u32,
}

/// A collection of [Mibl](crate::mibl::Mibl) textures embedded in the current file.
#[binread]
#[derive(Debug, Xc3Write, Clone)]
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

/// A single [Mibl](crate::mibl::Mibl) texture.
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct PackedTexture {
    pub usage: TextureUsage,

    #[br(parse_with = parse_count32_offset32, offset = base_offset)]
    #[xc3(count_offset(u32, u32), align(4096))]
    pub mibl_data: Vec<u8>,

    #[br(parse_with = parse_string_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub name: String,
}

/// References to [Mibl](crate::mibl::Mibl) textures in a separate file.
#[binread]
#[derive(Debug, Xc3Write, Clone, PartialEq)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct PackedExternalTextures {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    // TODO: Always identical to low textures in msrd?
    #[br(parse_with = parse_count32_offset32, args { offset: base_offset, inner: base_offset })]
    #[xc3(count_offset(u32, u32), align(2))]
    pub textures: Vec<PackedExternalTexture>,

    pub unk2: u32, // 0

    #[xc3(shared_offset)]
    pub strings_offset: u32,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, Clone, PartialEq)]
#[br(import_raw(base_offset: u64))]
pub struct PackedExternalTexture {
    pub usage: TextureUsage,

    pub mibl_length: u32,
    pub mibl_offset: u32,

    #[br(parse_with = parse_string_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub name: String,
}

// TODO: Are these some sort of flags?
// TODO: Use these for default assignments without database?
// TODO: Possible to guess temp texture channels?
/// Hints on how the texture is used.
/// Actual usage is determined by the shader.
#[derive(Debug, BinRead, BinWrite, Clone, Copy, PartialEq, Eq, Hash)]
#[brw(repr(u32))]
pub enum TextureUsage {
    Unk0 = 0,
    /// MTL, AMB, GLO, SHY, MASK, SPC, DPT, VEL, temp0001, ...
    Temp = 1048576,
    Unk6 = 1074790400,
    Nrm = 1179648,
    Unk13 = 131072,
    WavePlus = 136314882,
    Col = 2097152,
    Unk8 = 2162689,
    Alp = 2228224,
    Unk = 268435456,
    Alp2 = 269484032,
    Col2 = 270532608,
    Unk11 = 270663680,
    Unk9 = 272629760,
    Alp3 = 273678336,
    Nrm2 = 273809408,
    Col3 = 274726912,
    Unk3 = 274857984,
    Unk2 = 275775488,
    Unk20 = 287309824,
    Unk17 = 3276800,
    F01 = 403701762, // 3D?
    Unk4 = 4194304,
    Unk7 = 536870912,
    Unk15 = 537001984,
    /// AO, OCL2, temp0000, temp0001, ...
    Temp2 = 537919488,
    Unk14 = 538050560,
    Col4 = 538968064,
    Alp4 = 539099136,
    Unk12 = 540147712,
    Unk18 = 65537,
    Unk19 = 805306368,
    Unk5 = 807403520,
    Unk10 = 807534592,
    VolTex = 811597824,
    Unk16 = 811728896,
}

// xc1: 40 bytes
// xc2: 32, 36, 40 bytes
// xc3: 52, 60 bytes
/// Information for the skinned bones used by this model.
#[binread]
#[derive(Debug, Xc3Write, Clone)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct Skinning {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    pub count1: u32,
    pub count2: u32,

    // Estimate the struct size based on its first offset.
    #[br(temp, restore_position)]
    bones_offset: u32,

    /// Defines the name and ordering of the bones
    /// for the [BoneIndices](crate::vertex::DataType::BoneIndices) in the weights buffer.
    // TODO: Find a simpler way of writing this?
    // TODO: helper for separate count.
    #[br(parse_with = parse_ptr32)]
    #[br(args {
        offset: base_offset,
        inner: args! { count: count1 as usize, inner: base_offset }
    })]
    #[xc3(offset(u32))]
    pub bones: Vec<Bone>,

    /// Column-major inverse of the world transform for each bone in [bones](#structfield.bones).
    #[br(parse_with = parse_ptr32)]
    #[br(args { offset: base_offset, inner: args! { count: count1 as usize } })]
    #[xc3(offset(u32), align(16))]
    pub inverse_bind_transforms: Vec<[[f32; 4]; 4]>,

    // TODO: Possible to calculate count directly?
    #[br(temp, restore_position)]
    offsets: [u32; 2],

    // TODO: Count related to bone unk_type?
    // TODO: Count is 0, 2, or 4?
    #[br(parse_with = parse_opt_ptr32)]
    #[br(args {
        offset: base_offset,
        inner: args! {
            count: if offsets[1] > 0 { (offsets[1] - offsets[0]) as usize / 16 } else { 0 }
        }
    })]
    #[xc3(offset(u32))]
    pub transforms2: Option<Vec<[f32; 4]>>,

    // TODO: related to max unk index on bone?
    #[br(parse_with = parse_opt_ptr32)]
    #[br(args {
        offset: base_offset,
        inner: args! { count: bones.iter().map(|b| b.unk_index as usize + 1).max().unwrap_or_default() }
    })]
    #[xc3(offset(u32))]
    pub transforms3: Option<Vec<[[f32; 4]; 2]>>,

    // TODO: 0..count-1?
    #[br(parse_with = parse_count32_offset32, offset = base_offset)]
    #[xc3(count_offset(u32, u32))]
    pub bone_indices: Vec<u16>,

    // offset 32
    // Use nested options to skip fields entirely if not present.
    #[br(if(transforms2.is_some()))]
    #[br(args_raw(base_offset))]
    pub unk_offset4: Option<SkinningUnkBones>,

    #[br(if(transforms3.is_some()))]
    #[br(args_raw(base_offset))]
    pub unk_offset5: Option<SkinningUnk5>,

    // TODO: not present in xc2?
    // TODO: procedural bones?
    #[br(if(!bone_indices.is_empty()))]
    #[br(args_raw(base_offset))]
    pub as_bone_data: Option<SkinningAsBoneData>,

    // TODO: Optional padding for xc3?
    #[br(if(bones_offset == 60))]
    pub unk: Option<[u32; 4]>,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct SkinningUnkBones {
    #[br(parse_with = parse_opt_ptr32)]
    #[br(args { offset: base_offset, inner: base_offset })]
    #[xc3(offset(u32))]
    pub unk_offset4: Option<UnkBones>,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct SkinningUnk5 {
    #[br(parse_with = parse_opt_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub unk_offset5: Option<SkeletonUnk5>,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct SkinningAsBoneData {
    // TODO: procedural bones?
    #[br(parse_with = parse_opt_ptr32, args { offset: base_offset, inner: base_offset })]
    #[xc3(offset(u32))]
    pub as_bone_data: Option<AsBoneData>,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct Bone {
    #[br(parse_with = parse_string_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub name: String,
    pub unk1: f32,
    pub unk_type: (u16, u16),
    /// Index into [transforms3](struct.Skinning.html#structfield.transforms3).
    pub unk_index: u32,
    // TODO: padding?
    pub unk: [u32; 2],
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct UnkBones {
    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub bones: Vec<UnkBone>,

    #[br(parse_with = parse_ptr32)]
    #[br(args { offset: base_offset, inner: args! { count: bones.len() }})]
    #[xc3(offset(u32))]
    pub unk_offset: Vec<[[f32; 4]; 4]>,
    // TODO: no padding?
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, Clone)]
pub struct UnkBone {
    pub unk1: u32,
    /// The index in [bones](struct.Skeleton.html#structfield.bones).
    pub bone_index: u16,
    /// The index in [bones](struct.Skeleton.html#structfield.bones) of the parent bone.
    pub parent_index: u16,
    // TODO: padding?
    pub unks: [u32; 7],
}

#[binread]
#[derive(Debug, Xc3Write, Xc3WriteOffsets, Clone)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct SkeletonUnk5 {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    // TODO: element size?
    #[br(parse_with = parse_count32_offset32, offset = base_offset)]
    #[xc3(count_offset(u32, u32))]
    pub unk1: Vec<[u16; 105]>,

    // TODO: count?
    #[br(parse_with = parse_opt_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub unk_offset: Option<[f32; 12]>,

    // TODO: padding?
    pub unk: [u32; 5],
}

// TODO: Data for AS_ bones?
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct AsBoneData {
    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub bones: Vec<AsBone>,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub unk1: Vec<AsBoneValue>,

    #[br(parse_with = parse_ptr32)]
    #[br(args { offset: base_offset, inner: args! { count: bones.len() * 3 }})]
    #[xc3(offset(u32))]
    pub unk2: Vec<[[f32; 4]; 4]>,

    pub unk3: u32,

    // TODO: padding?
    pub unk: [u32; 2],
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, Clone)]
pub struct AsBone {
    /// The index in [bones](struct.Skeleton.html#structfield.bones).
    pub bone_index: u16,
    /// The index in [bones](struct.Skeleton.html#structfield.bones) of the parent bone.
    pub parent_index: u16,
    pub unk: [u32; 19],
}

// TODO: Some of these aren't floats?
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, Clone)]
pub struct AsBoneValue {
    unk1: [f32; 4],
    unk2: [f32; 4],
    unk3: [f32; 4],
    unk4: [f32; 2],
}

// TODO: pointer to decl_gbl_cac in ch001011011.wimdo?
#[binread]
#[derive(Debug, Xc3Write, Clone)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct Unk1 {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    #[br(parse_with = parse_count32_offset32, offset = base_offset)]
    #[xc3(count_offset(u32, u32))]
    pub unk1: Vec<Unk1Unk1>,

    #[br(parse_with = parse_count32_offset32, offset = base_offset)]
    #[xc3(count_offset(u32, u32))]
    pub unk2: Vec<Unk1Unk2>,

    #[br(parse_with = parse_count32_offset32, offset = base_offset)]
    #[xc3(count_offset(u32, u32))]
    pub unk3: Vec<Unk1Unk3>,

    // angle values?
    #[br(parse_with = parse_count32_offset32, offset = base_offset)]
    #[xc3(count_offset(u32, u32))]
    pub unk4: Vec<Unk1Unk4>,

    // TODO: padding?
    pub unk: [u32; 4],
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, Clone)]
pub struct Unk1Unk1 {
    pub index: u16,
    pub unk2: u16, // 1
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, Clone)]
pub struct Unk1Unk2 {
    pub unk1: u16, // 0
    pub index: u16,
    pub unk3: u16,
    pub unk4: u16,
    pub unk5: u32, // 0
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, Clone)]
pub struct Unk1Unk3 {
    pub unk1: u16,
    pub unk2: u16,
    pub unk3: u32,
    pub unk4: u16,
    pub unk5: u16,
    pub unk6: u16,
    pub unk7: u16,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, Clone)]
pub struct Unk1Unk4 {
    pub unk1: f32,
    pub unk2: f32,
    pub unk3: f32,
    pub unk4: u32,
}

xc3_write_binwrite_impl!(
    ParamType,
    RenderPassType,
    StateFlags,
    ModelsFlags,
    SamplerFlags,
    TextureUsage,
    ExtMeshFlags
);

impl Xc3Write for MaterialFlags {
    type Offsets<'a> = ();

    fn xc3_write<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        data_ptr: &mut u64,
    ) -> xc3_write::Xc3Result<Self::Offsets<'_>> {
        u32::from(*self).write_le(writer)?;
        *data_ptr = (*data_ptr).max(writer.stream_position()?);
        Ok(())
    }
}

impl<'a> Xc3WriteOffsets for SkinningOffsets<'a> {
    fn write_offsets<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        _base_offset: u64,
        data_ptr: &mut u64,
    ) -> xc3_write::Xc3Result<()> {
        let base_offset = self.base_offset;

        let bones = self.bones.write(writer, base_offset, data_ptr)?;

        if !self.bone_indices.data.is_empty() {
            self.bone_indices
                .write_full(writer, base_offset, data_ptr)?;
        }

        self.inverse_bind_transforms
            .write_full(writer, base_offset, data_ptr)?;

        self.transforms2.write_full(writer, base_offset, data_ptr)?;
        self.transforms3.write_full(writer, base_offset, data_ptr)?;

        self.unk_offset4
            .write_offsets(writer, base_offset, data_ptr)?;
        self.as_bone_data
            .write_offsets(writer, base_offset, data_ptr)?;
        self.unk_offset5
            .write_offsets(writer, base_offset, data_ptr)?;

        for bone in bones.0 {
            bone.name.write_full(writer, base_offset, data_ptr)?;
        }

        Ok(())
    }
}

impl<'a> Xc3WriteOffsets for ModelUnk1Offsets<'a> {
    fn write_offsets<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        _base_offset: u64,
        data_ptr: &mut u64,
    ) -> xc3_write::Xc3Result<()> {
        let base_offset = self.base_offset;

        let items1 = self.items1.write(writer, base_offset, data_ptr)?;

        self.items3.write_full(writer, base_offset, data_ptr)?;

        if !self.items2.data.is_empty() {
            self.items2.write_full(writer, base_offset, data_ptr)?;
        }

        // TODO: Set alignment at type level for Xc3Write?
        if !self.items4.data.is_empty() {
            self.items4.write_full(writer, base_offset, data_ptr)?;
        }

        for item in items1.0 {
            item.name.write_full(writer, base_offset, data_ptr)?;
        }

        self.extra.write_offsets(writer, base_offset, data_ptr)?;

        Ok(())
    }
}

impl<'a> Xc3WriteOffsets for LodDataOffsets<'a> {
    fn write_offsets<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        _base_offset: u64,
        data_ptr: &mut u64,
    ) -> xc3_write::Xc3Result<()> {
        let base_offset = self.base_offset;
        // Different order than field order.
        self.groups.write_full(writer, base_offset, data_ptr)?;
        self.items1.write_full(writer, base_offset, data_ptr)?;
        Ok(())
    }
}

// TODO: Add derive attribute for skipping empty vecs?
impl<'a> Xc3WriteOffsets for ModelsOffsets<'a> {
    fn write_offsets<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        _base_offset: u64,
        data_ptr: &mut u64,
    ) -> xc3_write::Xc3Result<()> {
        let base_offset = self.base_offset;

        self.models.write_full(writer, base_offset, data_ptr)?;
        self.skinning.write_full(writer, base_offset, data_ptr)?;
        if !self.ext_meshes.data.is_empty() {
            self.ext_meshes.write_full(writer, base_offset, data_ptr)?;
        }

        self.model_unk8.write_full(writer, base_offset, data_ptr)?;

        // TODO: Padding before this?
        self.morph_controllers
            .write_full(writer, base_offset, data_ptr)?;

        // Different order than field order.
        self.lod_data.write_full(writer, base_offset, data_ptr)?;
        self.model_unk7.write_full(writer, base_offset, data_ptr)?;
        self.model_unk1.write_full(writer, base_offset, data_ptr)?;
        self.model_unk4.write_full(writer, base_offset, data_ptr)?;
        self.model_unk3.write_full(writer, base_offset, data_ptr)?;
        self.extra.write_offsets(writer, base_offset, data_ptr)?;

        Ok(())
    }
}

impl<'a> Xc3WriteOffsets for ShaderProgramInfoOffsets<'a> {
    fn write_offsets<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
    ) -> xc3_write::Xc3Result<()> {
        // Different order than field order.
        self.attributes.write_full(writer, base_offset, data_ptr)?;
        if !self.textures.data.is_empty() {
            // TODO: Always skip offset for empty vec?
            self.textures.write_full(writer, base_offset, data_ptr)?;
        }
        self.uniform_blocks
            .write_full(writer, base_offset, data_ptr)?;

        // TODO: Why is there a variable amount of padding?
        self.parameters.write_full(writer, base_offset, data_ptr)?;
        *data_ptr += self.parameters.data.len() as u64 * 16;

        Ok(())
    }
}

// TODO: Add derive attribute for skipping empty vecs?
impl<'a> Xc3WriteOffsets for MaterialsOffsets<'a> {
    fn write_offsets<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        _base_offset: u64,
        data_ptr: &mut u64,
    ) -> xc3_write::Xc3Result<()> {
        let base_offset = self.base_offset;

        // Material fields get split up and written in a different order.
        let materials = self.materials.write(writer, base_offset, data_ptr)?;

        self.floats.write_full(writer, base_offset, data_ptr)?;
        self.ints.write_full(writer, base_offset, data_ptr)?;

        for material in &materials.0 {
            material
                .shader_programs
                .write_full(writer, base_offset, data_ptr)?;
        }

        for material in &materials.0 {
            material
                .textures
                .write_full(writer, base_offset, data_ptr)?;
        }

        // Different order than field order.
        if !self.alpha_test_textures.data.is_empty() {
            self.alpha_test_textures
                .write_full(writer, base_offset, data_ptr)?;
        }
        self.material_unk1
            .write_full(writer, base_offset, data_ptr)?;
        self.material_unk2
            .write_full(writer, base_offset, data_ptr)?;
        self.material_unk3
            .write_full(writer, base_offset, data_ptr)?;
        self.samplers.write_full(writer, base_offset, data_ptr)?;
        self.shader_programs
            .write_full(writer, base_offset, data_ptr)?;

        // TODO: Offset not large enough?
        for material in &materials.0 {
            material.name.write_full(writer, base_offset, data_ptr)?;
        }

        Ok(())
    }
}

impl<'a> Xc3WriteOffsets for MxmdOffsets<'a> {
    fn write_offsets<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
    ) -> xc3_write::Xc3Result<()> {
        self.models.write_full(writer, base_offset, data_ptr)?;
        self.materials.write_full(writer, base_offset, data_ptr)?;

        // Different order than field order.
        self.streaming.write_full(writer, base_offset, data_ptr)?;

        // TODO: 16 bytes of padding before this?
        // TODO: related to the optional 16 bytes before xbc1 in msrd?
        *data_ptr += 1;
        self.unk1.write_full(writer, base_offset, data_ptr)?;

        self.vertex_data.write_full(writer, base_offset, data_ptr)?;
        self.spch.write_full(writer, base_offset, data_ptr)?;
        self.packed_textures
            .write_full(writer, base_offset, data_ptr)?;

        Ok(())
    }
}

// TODO: Add derive attribute for skipping empty vecs?
impl<'a> Xc3WriteOffsets for Unk1Offsets<'a> {
    fn write_offsets<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        _base_offset: u64,
        data_ptr: &mut u64,
    ) -> xc3_write::Xc3Result<()> {
        let base_offset = self.base_offset;
        self.unk1.write_full(writer, base_offset, data_ptr)?;
        self.unk2.write_full(writer, base_offset, data_ptr)?;
        self.unk3.write_full(writer, base_offset, data_ptr)?;
        if !self.unk4.data.is_empty() {
            self.unk4.write_full(writer, base_offset, data_ptr)?;
        }
        Ok(())
    }
}

impl<'a> Xc3WriteOffsets for ModelUnk3ItemOffsets<'a> {
    fn write_offsets<W: std::io::prelude::Write + std::io::prelude::Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
    ) -> xc3_write::Xc3Result<()> {
        // Different order than field order.
        self.unk3.write_full(writer, base_offset, data_ptr)?;
        self.name.write_full(writer, base_offset, data_ptr)?;
        Ok(())
    }
}

impl<'a> Xc3WriteOffsets for MaterialUnk3Offsets<'a> {
    fn write_offsets<W: std::io::prelude::Write + std::io::prelude::Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
    ) -> xc3_write::Xc3Result<()> {
        // Different order than field order.
        self.unk2.write_full(writer, base_offset, data_ptr)?;
        self.unk1.write_full(writer, base_offset, data_ptr)?;
        Ok(())
    }
}

impl<'a> Xc3WriteOffsets for PackedTexturesOffsets<'a> {
    fn write_offsets<W: std::io::prelude::Write + std::io::prelude::Seek>(
        &self,
        writer: &mut W,
        _base_offset: u64,
        data_ptr: &mut u64,
    ) -> xc3_write::Xc3Result<()> {
        let base_offset = self.base_offset;

        // Names and data need to be written at the end.
        let textures = self.textures.write(writer, base_offset, data_ptr)?;

        self.strings_offset
            .write_full(writer, base_offset, data_ptr)?;
        for texture in &textures.0 {
            texture.name.write_full(writer, base_offset, data_ptr)?;
        }
        for texture in &textures.0 {
            texture
                .mibl_data
                .write_full(writer, base_offset, data_ptr)?;
        }
        Ok(())
    }
}

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
