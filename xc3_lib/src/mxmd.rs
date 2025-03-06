//! Model data in `.wimdo` files.
//!
//! [Mxmd] files contain the main model data like the mesh hierarchy and materials
//! as well as information on the streaming data in the optional `.wismt` file.
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
    xc3_write_binwrite_impl, StringOffset32,
};
use bilge::prelude::*;
use binrw::{args, binread, BinRead, BinWrite};
use xc3_write::{Xc3Write, Xc3WriteOffsets};

pub mod legacy;

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, PartialEq, Clone)]
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

    /// Streaming information for the .wismt file or [None] if no .wismt file.
    /// Identical to the same field in the corresponding [Msrd](crate::msrd::Msrd).
    #[br(parse_with = parse_opt_ptr32)]
    #[xc3(offset(u32), align(4))]
    pub streaming: Option<Streaming>,

    pub unk6: u32,
    pub unk7: u32,

    #[br(parse_with = parse_opt_ptr32)]
    #[xc3(offset(u32), align(16))]
    pub unk8: Option<Unk8>,

    // TODO: padding?
    pub unk: [u32; 6],
}

// TODO: more strict alignment for xc3?
// TODO: 108 bytes for xc2 and 112 bytes for xc3?
/// A collection of [Material], [Sampler], and material parameters.
/// `ml::MdsMatTopHeader` in the Xenoblade 2 binary.
#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, PartialEq, Clone)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct Materials {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    #[br(temp, restore_position)]
    material_offset: u32,

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
    #[xc3(offset_count(u32, u32), align(16))]
    pub work_values: Vec<f32>,

    // TODO: final number counts up from 0?
    // TODO: Some sort of index or offset?
    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub shader_vars: Vec<(u16, u16)>, // shader vars (u8, u8, u16)?

    #[br(parse_with = parse_opt_ptr32)]
    #[br(args { offset: base_offset, inner: base_offset })]
    #[xc3(offset(u32))]
    pub callbacks: Option<MaterialCallbacks>,

    // TODO: is this ever not 0?
    pub unk4: u32,

    /// Info for each of the shaders in the associated [Spch](crate::spch::Spch).
    #[br(parse_with = parse_offset32_count32)]
    #[br(args { offset: base_offset, inner: base_offset })]
    #[xc3(offset_count(u32, u32))]
    pub techniques: Vec<Technique>,

    pub unks1: u32,

    #[br(parse_with = parse_opt_ptr32)]
    #[br(args { offset: base_offset, inner: base_offset })]
    #[xc3(offset(u32))]
    pub unk6: Option<MaterialUnk6>,

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
    #[br(args { offset: base_offset, inner: args! { base_offset, count: materials.len() } })]
    #[xc3(offset(u32))]
    pub fur_shells: Option<FurShells>,

    pub unks3_1: [u32; 2],

    #[br(parse_with = parse_opt_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub samplers: Option<Samplers>,

    // TODO: padding?
    pub unks4: [u32; 3],

    #[br(if(material_offset >= 112))]
    #[br(args_raw(base_offset))]
    pub unk5: Option<MaterialUnk5>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct AlphaTestTexture {
    // TODO: (_, 0, 1) has alpha testing?
    // TODO: Test different param values?
    pub texture_index: u16,
    pub unk1: u16,
    pub unk2: u32,
}

/// `ml::MdsMatTechnique` in the Xenoblade 2 binary.
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct Technique {
    /// The input attributes for the vertex shader.
    /// The order defined here should also be used for the vertex buffer attributes to work properly in game.
    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub attributes: Vec<VertexAttribute>,

    pub unk3: u32, // 0
    pub unk4: u32, // 0

    // work values?
    // TODO: matches up with uniform parameters for U_Mate?
    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub parameters: Vec<MaterialParameter>,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub textures: Vec<u16>,

    // ssbos and then uniform buffers ordered by handle?
    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub uniform_blocks: Vec<UniformBlock>, // uniform blocks?

    pub material_texture_count: u32,

    // first texture param index?
    pub unk12: u16, // counts up from 0?
    // first global param index?
    pub unk13: u16, // unk11 + unk12?

    pub unk14: u32,

    // TODO: type?
    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub unk15: Vec<[u32; 5]>,

    // TODO: padding?
    pub padding: [u32; 2],
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct UniformBlock {
    pub unk1: u16,
    pub unk2: u8,
    pub unk3: u8,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct VertexAttribute {
    pub data_type: DataType,
    pub relative_offset: u16,
    pub buffer_index: u16,
    pub unk4: u16, // always 0?
}

/// `ml::MdsMatVariableTbl` in the Xenoblade 2 binary.
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct MaterialParameter {
    pub param_type: ParamType,
    pub work_value_index: u16, // added to work value start index?
    pub unk: u16,
    pub count: u16, // actual number of bytes depends on type?
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
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
    AlphaInfo = 5,
    /// `gMatCol` uniform in the [Spch].
    /// Takes the value of [color](struct.Material.html#structfield.color).
    MaterialColor = 6,
    Unk7 = 7,
    /// `gToonHeadMat` uniform in the [Spch].
    ToonHeadMatrix = 10,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct MaterialCallbacks {
    // TODO: affects material parameter assignment?
    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub work_callbacks: Vec<WorkCallback>,

    // 0 ... material_count - 1
    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub material_indices: Vec<u16>,

    // TODO: [index, ???]
    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub unk1: Vec<[u32; 2]>,

    // TODO: padding?
    pub unk: [u32; 6],
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct WorkCallback {
    // TODO: enum 0, 12, 22, 25, 26, 27, 28, 35, 36, 38, 40, 41, 42, 43, 45, 46, 47, 58, 50
    // 25 outline width
    // 26 next value / 255.0
    pub unk1: u16,
    // TODO: index?
    pub unk2: u16,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct MaterialUnk2 {
    #[br(parse_with = parse_count32_offset32, offset = base_offset)]
    #[xc3(count_offset(u32, u32))]
    pub unk1: Vec<[u32; 3]>,

    // TODO: padding?
    pub unk: [u32; 4],
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, PartialEq, Clone)]
#[br(import { base_offset: u64, count: usize })]
pub struct FurShells {
    /// Index into [params](#structfield.params) for each of the elements in
    /// [materials](struct.Materials.html#structfield.materials).
    #[br(parse_with = parse_ptr32)]
    #[br(args { offset: base_offset, inner: args! { count }})]
    #[xc3(offset(u32))]
    pub material_param_indices: Vec<u16>,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub params: Vec<FurShellParams>,

    // TODO: padding?
    pub unk: [u32; 4],
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct FurShellParams {
    /// The number of fur shells to render.
    pub instance_count: u32,
    /// The distance at which shell count starts to lower.
    pub view_distance: f32,
    /// The width applied increasingly to each fur shell.
    pub shell_width: f32,
    /// The vertical offset applied increasingly to each fur shell.
    pub y_offset: f32,
    /// The alpha transparency applied increasingly to each fur shell.
    // TODO: alpha of 0.0 is not fully transparent?
    pub alpha: f32,
}

/// A collection of [Sampler].
#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
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
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct Sampler {
    pub flags: SamplerFlags,
    pub unk2: u16,

    // Is this actually a float?
    pub unk3: f32,
}

/// Texture sampler settings for addressing and filtering.
#[bitsize(16)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(DebugBits, FromBits, BinRead, BinWrite, PartialEq, Clone, Copy)]
#[br(map = u16::into)]
#[bw(map = |&x| u16::from(x))]
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
    /// Disables 4x anisotropic filtering when `true`
    /// The min filter also depends on disable_mipmap_filter.
    pub nearest: bool,
    /// Sets all wrap modes to clamp and min and mag filter to linear.
    /// Ignores the values of previous flags.
    pub force_clamp: bool,
    /// Removes the mipmap nearest from the min filter when `true`.
    /// Disables 4x anisotropic filtering when `true`
    pub disable_mipmap_filter: bool,
    pub unk1: bool,
    pub unk3: bool,
    pub unk: u7,
}

/// A single material assignable to a [Mesh].
/// `ml::MdsMatInfoHeader` in the Xenoblade 2 binary.
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct Material {
    #[br(parse_with = parse_string_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub name: String,

    pub flags: MaterialFlags,

    pub render_flags: MaterialRenderFlags,

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

    // TODO: group indices for animations?
    pub m_unks1_1: u32,
    pub m_unks1_2: u32,
    pub m_unks1_3: u32,
    pub m_unks1_4: u32,

    // TODO: each material has its own unique range of values?
    /// Index into [work_values](struct.Materials.html#structfield.work_values).
    pub work_value_start_index: u32,

    // TODO: each material has its own unique range of values?
    /// Index into [shader_vars](struct.Materials.html#structfield.shader_vars).
    pub shader_var_start_index: u32,
    pub shader_var_count: u32,

    // TODO: always count 1?
    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub techniques: Vec<MaterialTechnique>,

    pub unk5: u32, // 0

    /// Index into [work_callbacks](struct.MaterialCallbacks.html#structfield.work_callbacks).
    pub callback_start_index: u16,
    pub callback_count: u16,

    // TODO: alt textures offset for non opaque rendering?
    pub m_unks2: [u16; 3],

    /// Index into [alpha_test_textures](struct.Materials.html#structfield.alpha_test_textures).
    pub alpha_test_texture_index: u16,
    // TODO: [???, gbuffer flags?, ...]
    pub m_unks3: [u16; 8],
}

#[bitsize(32)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(DebugBits, FromBits, BinRead, BinWrite, PartialEq, Clone, Copy)]
#[br(map = u32::into)]
#[bw(map = |&x| u32::from(x))]
pub struct MaterialFlags {
    pub unk1: bool,
    pub unk2: bool,
    /// Enables alpha testing from a texture in a prepass when `true`.
    pub alpha_mask: bool,
    /// Samples `texture.x` from a dedicated mask texture when `true`.
    /// Otherwise, the alpha channel is used.
    pub separate_mask: bool,
    pub unk5: bool,
    pub unk6: bool,
    pub unk7: bool,
    pub unk8: bool,
    pub unk9: bool,
    // TODO: Extra draw calls for fur rendering?
    pub fur: bool,
    pub unk11: u17,
    // TODO: Actually draw the instanced shells?
    pub fur_shells: bool,
    pub unk: u4,
}

#[bitsize(32)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(DebugBits, FromBits, BinRead, BinWrite, PartialEq, Clone, Copy)]
#[br(map = u32::into)]
#[bw(map = |&x| u32::from(x))]
pub struct MaterialRenderFlags {
    pub unk1: bool,
    pub unk2: bool,
    pub unk3: bool,
    pub unk4: bool,
    pub unk5: bool,
    pub unk6: bool,
    /// Render in a depth only z-prepass.
    /// Used exlusively for speff_zpre materials for Xenoblade 3.
    pub speff_zpre: bool, // TODO: start of gbuffer flags?
    pub unk8: bool,
    pub unk9: bool,
    pub unk10: bool, // TODO: fur shading temp tex for xc2?
    pub unk11: bool,
    // TODO: Is this an enum?
    pub specular: bool, // TODO: specular for out_attr5?
    pub unk13: bool,    // TODO: true for core crystals?
    pub unk14: bool,    // TODO: true for core crystals?
    pub unk15: bool,
    pub unk16: bool, // false for characters
    pub unk17: bool,
    pub unk18: bool,
    pub unk19: bool,
    pub unk20: bool,
    /// Used exclusively for speff_ope materials for Xenoblade 3.
    // TODO: what does this toggle?
    pub speff_ope: bool,
    pub unk: u11,
}

/// Flags controlling pipeline state for rasterizer and fragment state.
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, BinWrite, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StateFlags {
    pub depth_write_mode: u8, // TODO: 0, 1, 2, 7
    pub blend_mode: BlendMode,
    pub cull_mode: CullMode,
    pub unk4: u8, // unused?
    pub stencil_value: StencilValue,
    pub stencil_mode: StencilMode,
    pub depth_func: DepthFunc,
    pub color_write_mode: ColorWriteMode,
}

// TODO: 0, 10 write to all outputs and 1,11 write to just color?
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, BinWrite, Clone, Copy, PartialEq, Eq, Hash)]
#[brw(repr(u8))]
pub enum ColorWriteMode {
    Unk0 = 0,   // TODO: all outputs?
    Unk1 = 1,   // TODO: single output?
    Unk2 = 2,   // TODO: xcx only?
    Unk3 = 3,   // TODO: xcx only?
    Unk5 = 5,   // TODO: xc2 efb0 only?
    Unk6 = 6,   // TODO: xcx only?
    Unk9 = 9,   // TODO: xcx only?
    Unk10 = 10, // TODO: all outputs but blends with previous color output texture?
    Unk11 = 11, // TODO: single output?
    Unk12 = 12, // TODO: xcx only?
}

/// | Value | Col Src | Col Dst | Col Op | Alpha Src | Alpha Dst | Alpha Op |
/// | --- | --- | --- | --- | --- | --- | --- |
/// | 0 |  |  |  |  |  |  |
/// | 1 | Src Alpha | 1 - Src Alpha | Add | Src Alpha | 1 - Src Alpha | Add |
/// | 2 | Src Alpha | One | Add | Src Alpha | One | Add |
/// | 3 | Zero | Src Col | Add | Zero | Src Col | Add |
/// | 4 | 1 - Dst Col | Zero | Add | 1 - Dst Col | Zero | Add |
/// | 5 | One | One | Add | One | One | Add |
/// | 6 |  |  |  |  |  |  |
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, BinWrite, Clone, Copy, PartialEq, Eq, Hash)]
#[brw(repr(u8))]
pub enum BlendMode {
    Disabled = 0,
    Blend = 1,
    Unk2 = 2,
    Multiply = 3,
    MultiplyInverted = 4,
    Add = 5,
    Disabled2 = 6,
}

// TODO: manually test stencil values in renderdoc.
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, BinWrite, Clone, Copy, PartialEq, Eq, Hash)]
#[brw(repr(u8))]
pub enum StencilValue {
    /// 10 (0xA)
    Unk0 = 0,
    Unk1 = 1,
    /// 14 (0xE)
    Unk4 = 4,
    Unk5 = 5,
    Unk8 = 8,
    Unk9 = 9,
    Unk12 = 12,
    /// 74 (0x4A)
    Unk16 = 16,
    Unk20 = 20,
    // TODO: test Xenoblade X values in RenderDoc
    Unk33 = 33,
    Unk37 = 37,
    Unk41 = 41,
    Unk49 = 49,
    Unk97 = 97,
    Unk105 = 105,
}

// TODO: Does this flag actually disable stencil?
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, BinWrite, Clone, Copy, PartialEq, Eq, Hash)]
#[brw(repr(u8))]
pub enum StencilMode {
    // func, write mask, comp mask, ref value
    Unk0 = 0, // completely disabled?
    Unk1 = 1, // always, ff, ff, 0a
    Unk2 = 2, // equals, 0a, 0a, 0a
    Unk6 = 6, // equals, 4b, 04, 0a
    Unk7 = 7, // always, 0e, 04, 0a
    Unk8 = 8, // nequal, 02, 02, 02
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, BinWrite, Clone, Copy, PartialEq, Eq, Hash)]
#[brw(repr(u8))]
pub enum DepthFunc {
    Disabled = 0,
    LessEqual = 1,
    Equal = 3,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, BinWrite, Clone, Copy, PartialEq, Eq, Hash)]
#[brw(repr(u8))]
pub enum CullMode {
    Back = 0,
    Front = 1,
    Disabled = 2,
    Unk3 = 3, // front + ???
}

/// `ml::MdsMatMaterialTechnique` in the Xenoblade 2 binary.
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct MaterialTechnique {
    /// Index into [techniques](struct.Materials.html#structfield.techniques).
    /// This can also be assumed to be the index into the [Spch] programs.
    pub technique_index: u32,
    pub pass_type: RenderPassType,
    pub material_buffer_index: u16,
    pub flags: u32, // always 1?
}

// TODO: Use in combination with mesh render flags?
// Each "pass" has different render targets?
// _trans = 1,
// _ope = 0,1,7
// _zpre = 0
// _outline = 0
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, BinWrite, PartialEq, Eq, Clone, Copy, Hash)]
#[brw(repr(u16))]
pub enum RenderPassType {
    Unk0 = 0, // main opaque + some transparent?
    Unk1 = 1, // transparent pass with color output
    Unk6 = 6, // used for maps?
    Unk7 = 7, // transparent pass but writes to all outputs
    Unk9 = 9, // used for maps?
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct Texture {
    /// Index into the textures in [streaming](struct.Mxmd.html#structfield.streaming)
    /// or [packed_textures](struct.Mxmd.html#structfield.packed_textures).
    pub texture_index: u16,
    /// Index into the samplers in [samplers](struct.Materials.html#structfield.samplers).
    pub sampler_index: u16,
    /// Index into the samplers in [samplers](struct.Materials.html#structfield.samplers).
    // TODO: This sampler is the same as above but with a float value of 0.0?
    pub sampler_index2: u16,
    pub unk3: u16, // 0
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct MaterialUnk5 {
    #[br(parse_with = parse_opt_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub unk1: Option<MaterialUnk5Inner>,
}

#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct MaterialUnk5Inner {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    pub unk1: u32,

    // TODO: item type?
    #[br(parse_with = parse_count32_offset32, offset = base_offset)]
    #[xc3(count_offset(u32, u32))]
    pub unk2: Vec<[f32; 6]>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct MaterialUnk6 {
    #[br(parse_with = parse_count32_offset32)]
    #[br(args { offset: base_offset, inner: base_offset })]
    #[xc3(count_offset(u32, u32))]
    pub items: Vec<MaterialUnk6Item>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct MaterialUnk6Item {
    pub unk1: u32,
    pub material_index: u32,
    pub unk3: u32,

    #[br(parse_with = parse_count32_offset32)]
    #[br(args { offset: base_offset, inner: base_offset })]
    #[xc3(count_offset(u32, u32))]
    pub unk4: Vec<MaterialUnk6ItemUnk4>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct MaterialUnk6ItemUnk4 {
    pub unk1: u32,

    #[br(parse_with = parse_count32_offset32, offset = base_offset)]
    #[xc3(count_offset(u32, u32))]
    pub unk2: Vec<[f32; 3]>,
}

// xc1: 160, 164, 168 bytes
// xc2: 160 bytes
// xc3: 160, 164, 168, 200, 204 bytes
/// A collection of [Model] as well as skinning and animation information.
#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, PartialEq, Clone)]
#[br(stream = r)]
#[br(import_raw(version: u32))]
#[xc3(base_offset)]
pub struct Models {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    // TODO: Default value for version arg to make maps work properly?
    #[br(if(version != 10111))]
    pub models_flags: Option<ModelsFlags>,

    /// The maximum of all the [max_xyz](struct.Model.html#structfield.max_xyz) in [models](#structfield.models).
    pub max_xyz: [f32; 3],
    /// The minimum of all the [min_xyz](struct.Model.html#structfield.min_xyz) in [models](#structfield.models).
    pub min_xyz: [f32; 3],

    #[br(temp, restore_position)]
    models_offset: u32,

    #[br(parse_with = parse_offset32_count32, args { offset: base_offset, inner: base_offset })]
    #[xc3(offset_count(u32, u32))]
    pub models: Vec<Model>,

    pub unk2: u32,

    #[br(parse_with = parse_opt_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub skinning: Option<Skinning>,

    #[br(parse_with = parse_opt_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub model_unk11: Option<ModelUnk11>,

    pub unks3_1: [u32; 13],

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

    // TODO: Also morph related but for animations?
    #[br(parse_with = parse_opt_ptr32, offset = base_offset)]
    #[xc3(offset(u32), align(16))]
    pub model_unk1: Option<ModelUnk1>,

    #[br(parse_with = parse_opt_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub model_unk3: Option<ModelUnk3>,

    // TODO: not always aligned to 16?
    #[br(parse_with = parse_opt_ptr32, offset = base_offset)]
    #[xc3(offset(u32), align(8))]
    pub lod_data: Option<LodData>,

    // TODO: not always aligned to 16?
    // TODO: Only null for stage models?
    #[br(parse_with = parse_opt_ptr32, offset = base_offset)]
    #[xc3(offset(u32), align(4))]
    pub alpha_table: Option<AlphaTable>,

    pub unk_field2: u32,

    #[br(parse_with = parse_opt_ptr32)]
    #[br(args { offset: base_offset, inner: base_offset})]
    #[xc3(offset(u32))]
    pub model_unk9: Option<ModelUnk9>,

    // TODO: Completely different type for version 10111?
    #[br(parse_with = parse_opt_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub model_unk12: Option<ModelUnk12>,

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
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
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
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct ModelsExtraDataUnk2 {
    #[br(parse_with = parse_opt_ptr32)]
    #[br(args { offset: base_offset, inner: base_offset })]
    #[xc3(offset(u32))]
    pub model_unk10: Option<ModelUnk10>,
}

// 168 total bytes
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
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
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
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
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
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
    pub unks1: [u32; 3],  // always 0?
    pub unk2: (u16, u16), // TODO: rendering related?
    // TODO: padding?
    pub unks: [u32; 3],
}

// TODO: Figure out remaining indices.
/// Flags and resources associated with a single draw call.
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct Mesh {
    pub flags1: u32, // TODO: possible bits that are set, check outline, speff, etc
    pub flags2: MeshRenderFlags2,
    /// Index into [vertex_buffers](../vertex/struct.VertexData.html#structfield.vertex_buffers)
    /// for the associated [VertexData].
    pub vertex_buffer_index: u16,
    /// Index into [index_buffers](../vertex/struct.VertexData.html#structfield.index_buffers)
    /// for the associated [VertexData].
    pub index_buffer_index: u16,
    /// Index into [index_buffers](../vertex/struct.VertexData.html#structfield.index_buffers)
    /// for the associated [VertexData] for the depth only draw call used for shadow rendering.
    /// Custom models can use the same value as [index_buffer_index](#structfield.index_buffer_index).
    pub index_buffer_index2: u16,
    /// Index into [materials](struct.Materials.html#structfield.materials).
    pub material_index: u16,
    pub unk2: u32, // 0
    pub unk3: u16, // 0
    /// Index into [ext_meshes](struct.Models.html#structfield.ext_meshes).
    // TODO: enabled via a flag?
    pub ext_mesh_index: u16,
    pub unk4: u32, // 0
    pub unk5: u16, // TODO: used mostly for outline meshes?
    /// 1-based index into [items](struct.LodData.html#structfield.items).
    pub lod_item_index: u8,
    pub unk_mesh_index2: u8, // 1 to 20?
    /// Index into [items](struct.AlphaTable.html#structfield.items).
    pub alpha_table_index: u16,
    pub unk6: u16, // TODO: used mostly for outline meshes?
    // TODO: -1 for xc3 for "base" meshes and always 0 for xc1 and xc2
    // TODO: index for parent or base mesh for speff materials?
    pub base_mesh_index: i32,
    pub unk8: u32, // 0, 1
    pub unk9: u32, // 0
}

// TODO: remaining bits affect skinning?
/// Flags to determine how to draw a [Mesh].
#[bitsize(32)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(DebugBits, TryFromBits, BinRead, BinWrite, PartialEq, Clone, Copy)]
#[br(try_map = |x: u32| x.try_into().map_err(|e| format!("{e:?}")))]
#[bw(map = |&x| u32::from(x))]
pub struct MeshRenderFlags2 {
    /// The render pass for this draw call.
    pub render_pass: MeshRenderPass,
    pub unk5: u28,
}

// TODO: 16 also draws in the first pass but earlier?
// TODO: Also depends on technique type?
/// The render pass for this draw call.
#[bitsize(4)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, TryFromBits, PartialEq, Clone, Copy)]
pub enum MeshRenderPass {
    /// The first opaque pass with depth writes.
    Unk0 = 0,
    /// The first opaque pass with depth writes but earlier in the pass.
    Unk1 = 1,
    /// The alpha pass after the deferred pass without depth writes.
    Unk2 = 2,
    Unk4 = 4, // TODO: xc1 maps?
    /// The alpha pass immediately after [MeshRenderPass::Unk0] without depth writes.
    Unk8 = 8,
}

/// Flags to determine what data is present in [Models].
#[bitsize(32)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(DebugBits, FromBits, BinRead, BinWrite, PartialEq, Clone, Copy)]
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
    pub has_alpha_table: bool,
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
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
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
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(DebugBits, FromBits, BinRead, BinWrite, PartialEq, Clone, Copy)]
#[br(map = u16::into)]
#[bw(map = |&x| u16::from(x))]
pub struct ExtMeshFlags {
    pub unk1: bool, // true
    pub unk2: bool, // false
    pub unk3: bool, // false
    /// Whether to initially skip rendering assigned meshes.
    pub start_hidden: bool,
    pub unk5: bool,
    pub unk6: bool, // 0, 1 (xc3 only)
    pub unk: u10,   // 0
}

#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct MorphControllers {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    // TODO: same count as morph targets per descriptor in vertex data?
    #[br(parse_with = parse_offset32_count32, args { offset: base_offset, inner: base_offset })]
    #[xc3(offset_count(u32, u32))]
    pub controllers: Vec<MorphController>,

    pub unk1: u32,

    // TODO: padding?
    pub unk: [u32; 3],
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct MorphController {
    #[br(parse_with = parse_string_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub name1: String,

    // TODO: Is one of these names for the ModelUnk1Item1?
    #[br(parse_with = parse_string_opt_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub name2: Option<String>,

    pub unk1: u16, // 7?
    pub unk2: u16, // TODO: index into ModelUnk1Item1 used for animation tracks?
    pub unk3: u16, // 0?
    pub unk4: u16, // 3?

    // TODO: padding?
    pub unk: [u32; 3],
}

#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
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

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, PartialEq, Clone)]
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
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, PartialEq, Clone)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct Unk8 {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    pub unk1: u32,

    #[br(parse_with = parse_count32_offset32)]
    #[br(args { offset: base_offset, inner: base_offset })]
    #[xc3(count_offset(u32, u32))]
    pub unk2: Vec<Unk8Item>,

    #[br(parse_with = parse_ptr32)]
    #[br(args { offset: base_offset, inner: args! { count: unk2.len() } })]
    #[xc3(offset(u32), align(16))]
    pub unk3: Vec<[[f32; 4]; 4]>,

    // TODO: padding?
    pub unk: [u32; 4],
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct Unk8Item {
    #[br(parse_with = parse_string_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub name: String,
    pub unk1: u32,
    pub unk2: [[f32; 4]; 4],
    pub unk3: u32,
}

/// A table for mapping [ExtMesh] to [LodItem].
#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct AlphaTable {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    // TODO: used to assign ext mesh and lod alpha to a mesh?
    // TODO: assigned to meshes in order based on their ext mesh and lod?
    /// A mapping table for `(ext_mesh_index + 1, lod_item1_index + 1)`
    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub items: Vec<(u16, u16)>,

    // TODO: padding?
    pub unks: [u32; 4],
}

#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct ModelUnk5 {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    // TODO: DS_ names?
    #[br(parse_with = parse_count32_offset32)]
    #[br(args { offset: base_offset, inner: base_offset })]
    #[xc3(count_offset(u32, u32))]
    pub items: Vec<StringOffset32>,

    // TODO: padding?
    pub unks: [u32; 4],
}

#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
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
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
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

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct ModelUnk8 {
    // TODO: What type is this?
    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub unk1: Vec<[u32; 2]>,

    // TODO: What type is this?
    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub unk2: Vec<[f32; 4]>,

    // TODO: padding?
    pub unks: [u32; 2],
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct ModelUnk9 {
    // TODO: flags?
    // xc1: 1, 2, 3, 4, 5
    // xc3: 10000
    pub unk1: u32,

    #[br(args { unk1, base_offset})]
    pub inner: ModelUnk9Inner,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import { unk1: u32, base_offset: u64 })]
pub enum ModelUnk9Inner {
    // TODO: Safe to assume that this covers other cases?
    #[br(pre_assert(unk1 != 10000))]
    Unk0(ModelUnk9InnerUnk0),

    #[br(pre_assert(unk1 == 10000))]
    Unk1(#[br(args_raw(base_offset))] ModelUnk9InnerUnk1),
}

#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, PartialEq, Clone)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct ModelUnk9InnerUnk0 {
    // Subtract the unk1 size.
    #[br(temp, try_calc = r.stream_position().map(|p| p - 4))]
    base_offset: u64,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub items1: Vec<(u16, u16)>,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub items2: Vec<(u16, u16)>,

    // TODO: padding?
    pub unk: [u32; 4],
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct ModelUnk9InnerUnk1 {
    // TODO: These offsets are relative to the start of the struct for xc1?
    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub items: Vec<ModelUnk9Buffer>,

    #[br(parse_with = parse_ptr32)]
    #[br(args { offset: base_offset, inner: args! { count: model_unk9_buffer_length(&items) } })]
    #[xc3(offset(u32))]
    pub buffer: Vec<u8>,

    // TODO: Some sort of optional count?
    pub unk2: u32,

    // TODO: padding?
    pub unk: [u32; 3],
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct ModelUnk9Buffer {
    // TODO: items are 48 byte structs of f32 and i16 in buffer?
    pub offset: u32,
    pub count: u32,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct ModelUnk10 {
    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub unk1: Vec<u32>,
}

#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct ModelUnk11 {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    #[br(parse_with = parse_count32_offset32, offset = base_offset)]
    #[xc3(count_offset(u32, u32))]
    pub unk1: Vec<[u32; 6]>,

    #[br(parse_with = parse_count32_offset32, offset = base_offset)]
    #[xc3(count_offset(u32, u32))]
    pub unk2: Vec<[u32; 2]>,

    // TODO: padding?
    pub unks: [u32; 4],
}

#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct ModelUnk12 {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub items: Vec<ModelUnk12Item>,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub indices: Vec<u32>,

    pub unk2: u32,

    // TODO: array of 10 u16?
    pub unk: [u16; 22],
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct ModelUnk12Item {
    pub unk1: [f32; 4],
    pub unk2: [f32; 4],
    pub unk3: [f32; 4],
    pub unk4: [f32; 4],
}

// TODO: Some sort of float animation for eyes, morphs, etc?
#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, PartialEq, Clone)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct ModelUnk1 {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    // TODO: Related to ext meshes?
    // TODO: same count as track indices for xc2 extra animation for morph targets?
    #[br(parse_with = parse_offset32_count32)]
    #[br(args { offset: base_offset, inner: base_offset })]
    #[xc3(offset_count(u32, u32), align(4))]
    pub items1: Vec<ModelUnk1Item1>,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub items2: Vec<ModelUnk1Item2>,

    // TODO: Default values for items1?
    // TODO: same count as track indices for xc2 extra animation for morph targets?
    #[br(parse_with = parse_ptr32)]
    #[br(args { offset: base_offset, inner: args! { count: items1.len() }})]
    #[xc3(offset(u32))]
    pub items3: Vec<f32>,

    pub unk1: u32, // 0 or 1?

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub items4: Vec<[u16; 10]>,

    // flags?
    pub unk4: u32,
    pub unk5: u32,
    // TODO: not present for xc2?
    // TODO: Is this the correct check?
    #[br(if(unk4 != 0 || unk5 != 0))]
    #[br(args_raw(base_offset))]
    pub extra: Option<ModelUnk1Extra>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct ModelUnk1Extra {
    #[br(parse_with = parse_opt_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub unk_inner: Option<ModelUnk1Inner>,

    // TODO: only 12 bytes for chr/ch/ch01022012.wimdo?
    pub unk: [u32; 4],
}

// TODO: Another table like the alpha table?
#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct ModelUnk1Inner {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    // TODO: A mapping table for `(model_unk1_item1_index + 1, ???)`
    // TODO: What indexes into this table?
    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub items1: Vec<(u16, u16)>,

    // TODO: 0..N-1 arranged in a different order?
    #[br(parse_with = parse_ptr32)]
    #[br(args {
        offset: base_offset,
        inner: args! { count: items1.iter().map(|(i, j)| (*i + *j) as usize).max().unwrap_or_default() }
    })]
    #[xc3(offset(u32))]
    pub items2: Vec<u16>,

    // TODO: padding?
    pub unks: [u32; 5],
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct ModelUnk1Item1 {
    #[br(parse_with = parse_string_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub name: String,
    // TODO: padding?
    pub unk: [u32; 3],
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct ModelUnk1Item2 {
    pub unk1: u16,
    pub unk2: u16,
    pub unk3: u32,
    pub unk4: u32,
    pub unk5: u32,
    pub unk6: u32,
}

#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, PartialEq, Clone)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct LodData {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    pub unk1: u32, // 0?

    // TODO: Count related to number of mesh lod values?
    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32), align(16))]
    pub items: Vec<LodItem>,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub groups: Vec<LodGroup>,

    pub unks: [u32; 4],
}

// TODO: is lod: 0 in the mxmd special?
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct LodItem {
    pub unk1: [u32; 4], // [0, 0, 0, 0]
    pub unk2: f32,      // distance or radius?
    pub unk3: u8,       // 0
    pub index: u8,      // index within lod group?
    pub unk5: u8,       // 1, 2
    pub unk6: u8,       // 0
    pub unk7: [u32; 2], // [0, 0]
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct LodGroup {
    /// Index into [items](struct.LodData.html#structfield.items) for the highest level of detail.
    pub base_lod_index: u16,
    /// The number of LOD levels in this group.
    pub lod_count: u16,
}

/// A collection of [Mibl](crate::mibl::Mibl) textures embedded in the current file.
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

/// A single [Mibl](crate::mibl::Mibl) texture.
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
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
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, PartialEq, Clone)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct PackedExternalTextures<U>
where
    U: Xc3Write + 'static,
    for<'a> U: BinRead<Args<'a> = ()>,
    for<'a> U::Offsets<'a>: Xc3WriteOffsets<Args = ()>,
{
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    // TODO: Always identical to low textures in msrd?
    #[br(parse_with = parse_count32_offset32, args { offset: base_offset, inner: base_offset })]
    #[xc3(count_offset(u32, u32), align(2))]
    pub textures: Vec<PackedExternalTexture<U>>,

    pub unk2: u32, // 0

    #[xc3(shared_offset)]
    pub strings_offset: u32,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct PackedExternalTexture<U>
where
    U: Xc3Write + 'static,
    for<'a> U: BinRead<Args<'a> = ()>,
    for<'a> U::Offsets<'a>: Xc3WriteOffsets<Args = ()>,
{
    pub usage: U,

    /// The size of the texture file in bytes.
    pub length: u32,
    /// The offset of the texture file in bytes.
    pub offset: u32,

    #[br(parse_with = parse_string_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub name: String,
}

// TODO: These are big endian?
// TODO: Are these some sort of flags?
// TODO: Use these for default assignments without database?
// TODO: Possible to guess temp texture channels?
/// Hints on how the texture is used.
/// Actual usage is determined by the shader.
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
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
    Unk21 = 269615104,
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
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, PartialEq, Clone)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct Skinning {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    pub render_bone_count: u32,
    pub bone_count: u32,

    // Estimate the struct size based on its first offset.
    #[br(temp, restore_position)]
    bones_offset: u32,

    /// The bone list for the [BoneIndices](crate::vertex::DataType::BoneIndices) in the weights buffer.
    // TODO: Find a simpler way of writing this?
    // TODO: helper for separate count.
    #[br(parse_with = parse_ptr32)]
    #[br(args {
        offset: base_offset,
        inner: args! { count: bone_count as usize, inner: base_offset }
    })]
    #[xc3(offset(u32))]
    pub bones: Vec<Bone>,

    /// Column-major inverse of the world transform for each bone in [bones](#structfield.bones).
    #[br(parse_with = parse_ptr32)]
    #[br(args { offset: base_offset, inner: args! { count: bone_count as usize } })]
    #[xc3(offset(u32), align(16))]
    pub inverse_bind_transforms: Vec<[[f32; 4]; 4]>,

    // TODO: do these contain data for both types of constraints?
    #[br(parse_with = parse_opt_ptr32)]
    #[br(args { offset: base_offset, inner: args! { count: count_constraints(&bones) } })]
    #[xc3(offset(u32))]
    pub constraints: Option<Vec<BoneConstraint>>,

    #[br(parse_with = parse_opt_ptr32)]
    #[br(args { offset: base_offset, inner: args! { count: count_bounds(&bones) } })]
    #[xc3(offset(u32))]
    pub bounds: Option<Vec<BoneBounds>>,

    // TODO: 0..count-1?
    #[br(parse_with = parse_count32_offset32, offset = base_offset)]
    #[xc3(count_offset(u32, u32))]
    pub bone_indices: Vec<u16>,

    // offset 32
    // Use nested options to skip fields entirely if not present.
    #[br(if(constraints.is_some()))]
    #[br(args_raw(base_offset))]
    pub unk_offset4: Option<SkinningUnkBones>,

    #[br(if(bounds.is_some()))]
    #[br(args_raw(base_offset))]
    pub unk_offset5: Option<SkinningUnk5>,

    // TODO: not present in xc2?
    // TODO: procedural bones?
    #[br(if(!bone_indices.is_empty()))]
    #[br(args_raw(base_offset))]
    pub as_bone_data: Option<SkinningAsBoneData>,

    // TODO: Optional padding for xc3?
    // TODO: This doesn't always have correct padding?
    #[br(if(bones_offset == 52))]
    pub unk1: Option<[u32; 2]>,

    #[br(if(bones_offset == 60))]
    pub unk2: Option<[u32; 4]>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct SkinningUnkBones {
    #[br(parse_with = parse_opt_ptr32)]
    #[br(args { offset: base_offset, inner: base_offset })]
    #[xc3(offset(u32))]
    pub unk_offset4: Option<UnkBones>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct SkinningUnk5 {
    #[br(parse_with = parse_opt_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub unk_offset5: Option<SkeletonUnk5>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct SkinningAsBoneData {
    // TODO: procedural bones?
    #[br(parse_with = parse_opt_ptr32, args { offset: base_offset, inner: base_offset })]
    #[xc3(offset(u32))]
    pub as_bone_data: Option<AsBoneData>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct BoneConstraint {
    pub fixed_offset: [f32; 3],
    pub max_distance: f32,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct BoneBounds {
    pub center: [f32; 4],
    pub size: [f32; 4],
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct Bone {
    #[br(parse_with = parse_string_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub name: String,
    pub bounds_radius: f32,
    pub flags: BoneFlags,
    /// Index into [constraints](struct.Skinning.html#structfield.constraints)
    /// if [flags](#structfield.flags) enables any constraints and 0 otherwise.
    pub constraint_index: u8,
    /// Index into [bones](struct.Skinning.html#structfield.bones) of the parent bone
    /// if [flags](#structfield.flags) enables any constraints and 0 otherwise.
    pub parent_index: u8,
    /// Index into [bounds](struct.Skinning.html#structfield.bounds)
    /// if [flags](#structfield.flags) enables bounds and 0 otherwise.
    pub bounds_index: u32,
    // TODO: padding?
    pub unk: [u32; 2],
}

#[bitsize(16)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(DebugBits, FromBits, BinRead, BinWrite, PartialEq, Clone, Copy)]
#[br(map = u16::into)]
#[bw(map = |&x| u16::from(x))]
pub struct BoneFlags {
    pub fixed_offset_constraint: bool,
    pub bounds_offset: bool,
    pub distance_constraint: bool,
    pub no_camera_overlap: bool,
    pub unk: u12,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct UnkBones {
    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub bones: Vec<UnkBone>,

    #[br(parse_with = parse_ptr32)]
    #[br(args { offset: base_offset, inner: args! { count: bones.len() }})]
    #[xc3(offset(u32), align(16))]
    pub unk_offset: Vec<[[f32; 4]; 4]>,
    // TODO: no padding?
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct UnkBone {
    pub unk1: u32,
    /// Index in [bones](struct.Skeleton.html#structfield.bones).
    pub bone_index: u16,
    /// Index in [bones](struct.Skeleton.html#structfield.bones) of the parent bone.
    pub parent_index: u16,
    // TODO: padding?
    pub unks: [u32; 7],
}

#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, PartialEq, Clone)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct SkeletonUnk5 {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    #[br(parse_with = parse_count32_offset32)]
    #[br(args { offset: base_offset, inner: base_offset })]
    #[xc3(count_offset(u32, u32))]
    pub unk1: Vec<SkeletonUnk5Unk1>,

    // TODO: count?
    #[br(parse_with = parse_opt_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub unk_offset: Option<[f32; 12]>,

    // TODO: padding?
    pub unk: [u32; 5],
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct SkeletonUnk5Unk1 {
    pub unk1: [[f32; 4]; 4],
    pub unk2: u32,

    // TODO: all unk3 and then all unk4?
    #[br(parse_with = parse_count32_offset32, offset = base_offset)]
    #[xc3(count_offset(u32, u32))]
    pub unk3: Vec<SkeletonUnk5Unk1Unk3>,

    #[br(parse_with = parse_count32_offset32, offset = base_offset)]
    #[xc3(count_offset(u32, u32))]
    pub unk4: Vec<u32>,

    pub unk7: [f32; 15],
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct SkeletonUnk5Unk1Unk3 {
    pub unk1: f32,
    pub unk2: u16, // bone index?
    pub unk3: u16, // bone index?
}

// TODO: Data for AS_ bones?
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct AsBoneData {
    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub bones: Vec<AsBone>,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub values: Vec<AsBoneValue>,

    // TODO: Not bone count for ch01022012.wimdo?
    #[br(parse_with = parse_ptr32)]
    #[br(args { offset: base_offset, inner: args! { count: bones.len() }})]
    #[xc3(offset(u32), align(16))]
    pub transforms: Vec<AsBoneTransform>,

    pub unk3: u32,

    // TODO: padding?
    // TODO: only 4 bytes for ch01022012.wimdo?
    pub unk: [u32; 2],
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct AsBone {
    /// Index into [bones](struct.Skeleton.html#structfield.bones) for this bone.
    pub bone_index: u16,
    /// Index into [bones](struct.Skeleton.html#structfield.bones) of the parent bone.
    pub parent_index: u16,
    pub unk_end_index: u16,   // bones?
    pub unk_start_index: u16, // bones?
    pub unk1: u16,
    pub unk2: u16,
    pub value_count: u16,
    /// Index into [values](struct.AsBoneData.html#structfield.values).
    pub value_start_index: u16,
    /// The translation of this bone relative to its parent.
    pub translation: [f32; 3],
    pub unk5: [f32; 6], // ???
    /// The rotation of this bone relative to its parent.
    pub rotation_quaternion: [f32; 4],
    pub unk6: [f32; 3],
}

// TODO: Some of these aren't floats?
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct AsBoneValue {
    unk1: [f32; 4],
    unk2: [f32; 4],
    unk3: [f32; 4],
    unk4: [f32; 2],
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct AsBoneTransform {
    pub unk1: [[f32; 4]; 4],
    pub unk2: [[f32; 4]; 4],
    pub unk3: [[f32; 4]; 4],
}

// TODO: pointer to decl_gbl_cac in ch001011011.wimdo?
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

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct Unk1Unk1 {
    pub index: u16,
    pub unk2: u16, // 1
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct Unk1Unk2 {
    pub unk1: u16, // 0
    pub index: u16,
    pub unk3: u16,
    pub unk4: u16,
    pub unk5: u32, // 0
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct Unk1Unk3 {
    pub unk1: u16,
    pub unk2: u16,
    pub unk3: u32,
    pub unk4: u16,
    pub unk5: u16,
    pub unk6: u16,
    pub unk7: u16,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
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
    ExtMeshFlags,
    MeshRenderFlags2,
    MaterialFlags,
    MaterialRenderFlags,
    BoneFlags
);

impl Xc3WriteOffsets for SkinningOffsets<'_> {
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

        let bones = self.bones.write(writer, base_offset, data_ptr, endian)?;

        if !self.bone_indices.data.is_empty() {
            self.bone_indices
                .write_full(writer, base_offset, data_ptr, endian, ())?;
        }

        self.inverse_bind_transforms
            .write_full(writer, base_offset, data_ptr, endian, ())?;

        self.constraints
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.bounds
            .write_full(writer, base_offset, data_ptr, endian, ())?;

        self.unk_offset4
            .write_offsets(writer, base_offset, data_ptr, endian, ())?;
        self.as_bone_data
            .write_offsets(writer, base_offset, data_ptr, endian, ())?;
        self.unk_offset5
            .write_offsets(writer, base_offset, data_ptr, endian, ())?;

        for bone in bones.0 {
            bone.name
                .write_full(writer, base_offset, data_ptr, endian, ())?;
        }
        Ok(())
    }
}

impl Xc3WriteOffsets for ModelUnk1Offsets<'_> {
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

        let items1 = self.items1.write(writer, base_offset, data_ptr, endian)?;

        self.items3
            .write_full(writer, base_offset, data_ptr, endian, ())?;

        if !self.items2.data.is_empty() {
            self.items2
                .write_full(writer, base_offset, data_ptr, endian, ())?;
        }

        // TODO: Set alignment at type level for Xc3Write?
        if !self.items4.data.is_empty() {
            self.items4
                .write_full(writer, base_offset, data_ptr, endian, ())?;
        }

        for item in items1.0 {
            item.name
                .write_full(writer, base_offset, data_ptr, endian, ())?;
        }

        self.extra
            .write_offsets(writer, base_offset, data_ptr, endian, ())?;

        Ok(())
    }
}

impl Xc3WriteOffsets for LodDataOffsets<'_> {
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
        // Different order than field order.
        self.groups
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.items
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        Ok(())
    }
}

// TODO: Add derive attribute for skipping empty vecs?
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
        self.skinning
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        if !self.ext_meshes.data.is_empty() {
            self.ext_meshes
                .write_full(writer, base_offset, data_ptr, endian, ())?;
        }

        self.model_unk8
            .write_full(writer, base_offset, data_ptr, endian, ())?;

        // TODO: Padding before this?
        self.morph_controllers
            .write_full(writer, base_offset, data_ptr, endian, ())?;

        // Different order than field order.
        self.lod_data
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.model_unk7
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.model_unk11
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.model_unk1
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.model_unk12
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.alpha_table
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.model_unk3
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.model_unk9
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.extra
            .write_offsets(writer, base_offset, data_ptr, endian, ())?;

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
        if !self.textures.data.is_empty() {
            // TODO: Always skip offset for empty vec?
            self.textures
                .write_full(writer, base_offset, data_ptr, endian, ())?;
        }
        self.uniform_blocks
            .write_full(writer, base_offset, data_ptr, endian, ())?;

        self.parameters
            .write_full(writer, base_offset, data_ptr, endian, ())?;

        self.unk15
            .write_full(writer, base_offset, data_ptr, endian, ())?;

        // TODO: Why is there a variable amount of padding?
        // TODO: This isn't always accurate?
        *data_ptr += self.parameters.data.len() as u64 * 16;

        Ok(())
    }
}

// TODO: Add derive attribute for skipping empty vecs?
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

        // Material fields get split up and written in a different order.
        let materials = self
            .materials
            .write(writer, base_offset, data_ptr, endian)?;

        self.work_values
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.shader_vars
            .write_full(writer, base_offset, data_ptr, endian, ())?;

        for material in &materials.0 {
            material
                .techniques
                .write_full(writer, base_offset, data_ptr, endian, ())?;
        }

        for material in &materials.0 {
            material
                .textures
                .write_full(writer, base_offset, data_ptr, endian, ())?;
        }

        // Different order than field order.
        if !self.alpha_test_textures.data.is_empty() {
            self.alpha_test_textures
                .write_full(writer, base_offset, data_ptr, endian, ())?;
        }
        self.callbacks
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.material_unk2
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.fur_shells
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.samplers
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.unk6
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.unk5
            .write_offsets(writer, base_offset, data_ptr, endian, ())?;

        self.techniques
            .write_full(writer, base_offset, data_ptr, endian, ())?;

        // TODO: Offset not large enough?
        for material in &materials.0 {
            material
                .name
                .write_full(writer, base_offset, data_ptr, endian, ())?;
        }

        Ok(())
    }
}

impl Xc3WriteOffsets for MxmdOffsets<'_> {
    type Args = ();

    fn write_offsets<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
        endian: xc3_write::Endian,
        _args: Self::Args,
    ) -> xc3_write::Xc3Result<()> {
        self.models
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.unk8
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.materials
            .write_full(writer, base_offset, data_ptr, endian, ())?;

        // Different order than field order.
        self.streaming
            .write_full(writer, base_offset, data_ptr, endian, ())?;

        // Apply padding even if this is the end of the file.
        vec![0u8; (data_ptr.next_multiple_of(16) - *data_ptr) as usize]
            .xc3_write(writer, endian)?;
        *data_ptr = (*data_ptr).max(writer.stream_position()?);

        // TODO: Some files have 16 more bytes of padding?
        self.unk1
            .write_full(writer, base_offset, data_ptr, endian, ())?;

        self.vertex_data
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.spch
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.packed_textures
            .write_full(writer, base_offset, data_ptr, endian, ())?;

        // TODO: Align the file size itself for xc1?

        Ok(())
    }
}

// TODO: Add derive attribute for skipping empty vecs?
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
        if !self.unk4.data.is_empty() {
            self.unk4
                .write_full(writer, base_offset, data_ptr, endian, ())?;
        }
        Ok(())
    }
}

impl Xc3WriteOffsets for ModelUnk3ItemOffsets<'_> {
    type Args = ();

    fn write_offsets<W: std::io::prelude::Write + std::io::prelude::Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
        endian: xc3_write::Endian,
        _args: Self::Args,
    ) -> xc3_write::Xc3Result<()> {
        // Different order than field order.
        self.unk3
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.name
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        Ok(())
    }
}

impl Xc3WriteOffsets for FurShellsOffsets<'_> {
    type Args = ();

    fn write_offsets<W: std::io::prelude::Write + std::io::prelude::Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
        endian: xc3_write::Endian,
        _args: Self::Args,
    ) -> xc3_write::Xc3Result<()> {
        // Different order than field order.
        self.params
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.material_param_indices
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        Ok(())
    }
}

impl Xc3WriteOffsets for PackedTexturesOffsets<'_> {
    type Args = ();

    fn write_offsets<W: std::io::prelude::Write + std::io::prelude::Seek>(
        &self,
        writer: &mut W,
        _base_offset: u64,
        data_ptr: &mut u64,
        endian: xc3_write::Endian,
        _args: Self::Args,
    ) -> xc3_write::Xc3Result<()> {
        let base_offset = self.base_offset;

        // Names and data need to be written at the end.
        let textures = self.textures.write(writer, base_offset, data_ptr, endian)?;

        self.strings_offset
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        for texture in &textures.0 {
            texture
                .name
                .write_full(writer, base_offset, data_ptr, endian, ())?;
        }
        for texture in &textures.0 {
            texture
                .mibl_data
                .write_full(writer, base_offset, data_ptr, endian, ())?;
        }
        Ok(())
    }
}

impl<U> Xc3WriteOffsets for PackedExternalTexturesOffsets<'_, U>
where
    U: Xc3Write + 'static,
    for<'b> U: BinRead<Args<'b> = ()>,
    for<'b> U::Offsets<'b>: Xc3WriteOffsets<Args = ()>,
{
    type Args = ();

    fn write_offsets<W: std::io::prelude::Write + std::io::prelude::Seek>(
        &self,
        writer: &mut W,
        _base_offset: u64,
        data_ptr: &mut u64,
        endian: xc3_write::Endian,
        _args: Self::Args,
    ) -> xc3_write::Xc3Result<()> {
        let base_offset = self.base_offset;

        // Names need to be written at the end.
        let textures = self.textures.write(writer, base_offset, data_ptr, endian)?;

        self.strings_offset
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        for texture in &textures.0 {
            texture
                .name
                .write_full(writer, base_offset, data_ptr, endian, ())?;
        }
        Ok(())
    }
}

impl Xc3WriteOffsets for SkeletonUnk5Offsets<'_> {
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

        let unk1 = self.unk1.write(writer, base_offset, data_ptr, endian)?;
        for u in &unk1.0 {
            u.unk3
                .write_full(writer, base_offset, data_ptr, endian, ())?;
        }
        for u in &unk1.0 {
            u.unk4
                .write_full(writer, base_offset, data_ptr, endian, ())?;
        }
        self.unk_offset
            .write_full(writer, base_offset, data_ptr, endian, ())?;

        Ok(())
    }
}

impl Xc3WriteOffsets for Unk8Offsets<'_> {
    type Args = ();
    fn write_offsets<W: std::io::prelude::Write + std::io::prelude::Seek>(
        &self,
        writer: &mut W,
        _base_offset: u64,
        data_ptr: &mut u64,
        endian: xc3_write::Endian,
        _args: Self::Args,
    ) -> xc3_write::Xc3Result<()> {
        let base_offset = self.base_offset;
        let unk2 = self.unk2.write(writer, base_offset, data_ptr, endian)?;
        self.unk3
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        // Strings go at the end.
        for u in unk2.0 {
            u.name
                .write_full(writer, base_offset, data_ptr, endian, ())?;
        }
        Ok(())
    }
}

impl Xc3WriteOffsets for ModelUnk9InnerUnk0Offsets<'_> {
    type Args = ();

    fn write_offsets<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        _base_offset: u64,
        data_ptr: &mut u64,
        endian: xc3_write::Endian,
        args: Self::Args,
    ) -> xc3_write::Xc3Result<()> {
        // Subtract the unk1 size.
        let base_offset = self.base_offset.saturating_sub(4);
        if !self.items1.data.is_empty() {
            self.items1
                .write_full(writer, base_offset, data_ptr, endian, args)?;
        }
        if !self.items2.data.is_empty() {
            self.items2
                .write_full(writer, base_offset, data_ptr, endian, args)?;
        }
        Ok(())
    }
}

impl Xc3WriteOffsets for MaterialCallbacksOffsets<'_> {
    type Args = ();
    fn write_offsets<W: std::io::prelude::Write + std::io::prelude::Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
        endian: xc3_write::Endian,
        _args: Self::Args,
    ) -> xc3_write::Xc3Result<()> {
        // Different order than field order.
        self.work_callbacks
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.unk1
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.material_indices
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        Ok(())
    }
}

fn count_constraints(bones: &[Bone]) -> usize {
    // Assume all constraints are used.
    bones
        .iter()
        .map(|b| {
            if b.flags.distance_constraint() || b.flags.fixed_offset_constraint() {
                b.constraint_index as usize + 1
            } else {
                0
            }
        })
        .max()
        .unwrap_or_default()
}

fn count_bounds(bones: &[Bone]) -> usize {
    // Assume all bounds are used.
    bones
        .iter()
        .map(|b| {
            if b.flags.bounds_offset() {
                b.bounds_index as usize + 1
            } else {
                0
            }
        })
        .max()
        .unwrap_or_default()
}

fn model_unk9_buffer_length(buffers: &[ModelUnk9Buffer]) -> usize {
    // TODO: Is it safe to assume the item size?
    buffers
        .iter()
        .max_by_key(|b| b.offset)
        .map(|b| b.offset as usize + b.count as usize * 48)
        .unwrap_or_default()
}
