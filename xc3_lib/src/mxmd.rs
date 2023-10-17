//! Model data in `.wimdo` files.
//!
//! XC3: `chr/{ch,en,oj,wp}/*.wimdo`, `monolib/shader/*.wimdo`
use crate::{
    msrd::TextureResource, parse_count32_offset32, parse_offset32_count32, parse_opt_ptr32,
    parse_ptr32, parse_string_opt_ptr32, parse_string_ptr32, spch::Spch, vertex::VertexData,
};
use bilge::prelude::*;
use binrw::{args, binread, BinRead, BinWrite};
use xc3_write::{round_up, xc3_write_binwrite_impl, Xc3Write, Xc3WriteOffsets};

/// .wimdo files
#[derive(Debug, BinRead, Xc3Write)]
#[br(magic(b"DMXM"))]
#[xc3(magic(b"DMXM"))]
pub struct Mxmd {
    // TODO: Version differences?
    #[br(assert(version == 10111 || version == 10112))]
    pub version: u32,

    #[br(parse_with = parse_ptr32)]
    #[xc3(offset32)]
    pub models: Models,

    #[br(parse_with = parse_ptr32)]
    #[xc3(offset32, align(16))]
    pub materials: Materials,

    #[br(parse_with = parse_opt_ptr32)]
    #[xc3(offset32)]
    pub unk1: Option<Unk1>,

    /// Embedded vertex data for .wimdo only models with no .wismt.
    #[br(parse_with = parse_opt_ptr32)]
    #[xc3(offset32)]
    pub vertex_data: Option<VertexData>,

    /// Embedded shader data for .wimdo only models with no .wismt.
    #[br(parse_with = parse_opt_ptr32)]
    #[xc3(offset32)]
    pub spch: Option<Spch>,

    #[br(parse_with = parse_opt_ptr32)]
    #[xc3(offset32)]
    pub packed_textures: Option<PackedTextures>,

    pub unk5: u32,

    // unpacked textures?
    #[br(parse_with = parse_opt_ptr32)]
    #[xc3(offset32)]
    pub textures: Option<Textures>,

    // TODO: padding?
    pub unk: [u32; 10],
}

#[binread]
#[derive(Debug, Xc3Write)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct Materials {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    #[br(parse_with = parse_offset32_count32, args { offset: base_offset, inner: base_offset })]
    #[xc3(offset32_count32)]
    pub materials: Vec<Material>,

    // offset?
    pub unk1: u32,
    pub unk2: u32,

    // TODO: Materials have offsets into these arrays for parameter values?
    // material body has a uniform at shader offset 64 but offset 48 in this floats buffer
    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset32_count32, align(16))]
    pub floats: Vec<f32>, // work values?

    // TODO: final number counts up from 0?
    // TODO: Some sort of index or offset?
    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset32_count32)]
    pub ints: Vec<(u8, u8, u16)>, // shader vars?

    #[br(parse_with = parse_opt_ptr32)]
    #[br(args { offset: base_offset, inner: base_offset })]
    #[xc3(offset32)]
    pub unk_offset1: Option<MaterialUnk1>, // callbacks?

    // TODO: is this ever not 0?
    pub unk4: u32,

    /// Info for each of the shaders in the associated [Spch](crate::spch::Spch).
    #[br(parse_with = parse_offset32_count32, args { offset: base_offset, inner: base_offset })]
    #[xc3(offset32_count32)]
    pub shader_programs: Vec<ShaderProgramInfo>,

    pub unks1: [u32; 2],

    #[br(parse_with = parse_count32_offset32, offset = base_offset)]
    #[xc3(count32_offset32)]
    pub alpha_test_textures: Vec<AlphaTestTexture>,

    pub unks3: [u32; 7],

    #[br(parse_with = parse_opt_ptr32, offset = base_offset)]
    #[xc3(offset32)]
    pub samplers: Option<Samplers>,

    // TODO: padding?
    pub unks4: [u32; 4],
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct AlphaTestTexture {
    // TODO: (_, 0, 1) has alpha testing?
    // TODO: Test different param values?
    pub texture_index: u16,
    pub unk1: u16,
    pub unk2: u32,
}

/// `ml::MdsMatTechnique` in the Xenoblade 2 binary.
#[derive(Debug, BinRead, Xc3Write)]
#[br(import_raw(base_offset: u64))]
pub struct ShaderProgramInfo {
    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset32_count32)]
    pub unk1: Vec<u64>, // vertex attributes?

    pub unk3: u32, // 0
    pub unk4: u32, // 0

    // work values?
    // TODO: matches up with uniform parameters for U_Mate?
    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset32_count32)]
    pub parameters: Vec<MaterialParameter>, // var table?

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset32_count32)]
    pub textures: Vec<u16>, // textures?

    // ssbos and then uniform buffers ordered by handle?
    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset32_count32)]
    pub uniform_blocks: Vec<(u16, u16)>, // uniform blocks?

    pub unk11: u32, // material texture count?

    pub unk12: u16, // counts up from 0?
    pub unk13: u16, // unk11 + unk12?

    // TODO: padding?
    pub padding: [u32; 5],
}

/// `ml::MdsMatVariableTbl` in the Xenoblade 2 binary.
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
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
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
#[br(import_raw(base_offset: u64))]
pub struct MaterialUnk1 {
    // count matches up with Material.unk_start_index?
    // TODO: affects material parameter assignment?
    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset32_count32)]
    pub unk1: Vec<(u16, u16)>,

    // 0 1 2 ... material_count - 1
    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset32_count32)]
    pub unk2: Vec<u16>,

    // TODO: padding?
    pub unk: [u32; 8],
}

#[binread]
#[derive(Debug, Xc3Write, Xc3WriteOffsets)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct Samplers {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    #[br(parse_with = parse_count32_offset32, offset = base_offset)]
    #[xc3(count32_offset32)]
    pub samplers: Vec<Sampler>,

    // TODO: padding?
    pub unk: [u32; 2],
}

#[derive(Debug, BinRead, BinWrite)]
pub struct Sampler {
    #[br(map(|x: u32| x.into()))]
    #[bw(map(|x| u32::from(*x)))]
    pub flags: SamplerFlags,

    // Is this actually a float?
    pub unk2: f32,
}

/// Texture sampler settings for addressing and filtering.
#[bitsize(32)]
#[derive(DebugBits, FromBits, Clone, Copy)]
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
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
#[br(import_raw(base_offset: u64))]
pub struct Material {
    #[br(parse_with = parse_string_ptr32, offset = base_offset)]
    #[xc3(offset32)]
    pub name: String,

    #[br(map(|x: u32| x.into()))]
    pub flags: MaterialFlags,

    pub render_flags: u32,

    /// Color multiplier value assigned to the `gMatCol` shader uniform.
    pub color: [f32; 4],

    // TODO: final byte controls reference?
    pub alpha_test_ref: [u8; 4],

    // TODO: materials with zero textures?
    /// Defines the shader's sampler bindings in order for s0, s1, s2, ...
    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset32_count32)]
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
    #[xc3(offset32_count32)]
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
#[derive(DebugBits, FromBits, Clone, Copy)]
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
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct ShaderProgram {
    /// Index into [shader_programs](struct.Materials.html#structfield.shader_programs).
    pub program_index: u32,
    pub unk_type: ShaderUnkType,
    pub parent_material_index: u16, // buffer index?
    pub flags: u32,                 // always 1?
}

// Affects what pass the object renders in?
// Each "pass" has different render targets?
// _trans = 1,
// _ope = 0,1,7
// _zpre = 0
// _outline = 0
#[derive(Debug, BinRead, BinWrite, PartialEq, Eq, Clone, Copy)]
#[brw(repr(u16))]
pub enum ShaderUnkType {
    Unk0 = 0, // main opaque + some transparent?
    Unk1 = 1, // second layer transparent?
    Unk6 = 6, // used for maps?
    Unk7 = 7, // additional eye effect layer?
    Unk9 = 9, // used for maps?
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct Texture {
    pub texture_index: u16,
    pub sampler_index: u16,
    pub unk2: u16,
    pub unk3: u16,
}

#[binread]
#[derive(Debug, Xc3Write)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct Models {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    pub unk1: u32,

    pub max_xyz: [f32; 3],
    pub min_xyz: [f32; 3],

    #[br(parse_with = parse_offset32_count32, args { offset: base_offset, inner: base_offset })]
    #[xc3(offset32_count32)]
    pub models: Vec<Model>,

    pub unk2: u32,

    #[br(parse_with = parse_opt_ptr32, offset = base_offset)]
    #[xc3(offset32)]
    pub skinning: Option<Skinning>,

    pub unks3_1: [u32; 14],

    // TODO: previous string section size aligned to 16?
    #[br(parse_with = parse_offset32_count32, args { offset: base_offset, inner: base_offset })]
    #[xc3(offset32_count32, align(16))]
    pub model_unks: Vec<ModelUnk>,

    pub unks3_2: [u32; 5],

    #[br(parse_with = parse_opt_ptr32, offset = base_offset)]
    #[xc3(offset32)]
    morph_controllers: Option<MorphControllers>,

    // TODO: eye animations?
    #[br(parse_with = parse_opt_ptr32, offset = base_offset)]
    #[xc3(offset32, align(16))]
    pub unk_offset1: Option<MeshUnk1>,

    #[br(parse_with = parse_opt_ptr32, offset = base_offset)]
    #[xc3(offset32)]
    pub model_unk3: Option<ModelUnk3>,

    #[br(parse_with = parse_opt_ptr32, offset = base_offset)]
    #[xc3(offset32)]
    pub lod_data: Option<LodData>,

    #[br(parse_with = parse_opt_ptr32, offset = base_offset)]
    #[xc3(offset32)]
    pub model_unk4: Option<ModelUnk4>,

    pub unk_field2: u32,
    pub unk_fields: [u32; 4],

    #[br(parse_with = parse_opt_ptr32, offset = base_offset)]
    #[xc3(offset32)]
    pub model_unk5: Option<ModelUnk5>,

    // TODO: padding?
    pub unk: [u32; 8],
}

/// A collection of meshes where each [Mesh] represents one draw call.
///
/// Each [Model] has an associated [VertexData](crate::vertex::VertexData) containing vertex and index buffers.
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
#[br(import_raw(base_offset: u64))]
pub struct Model {
    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset32_count32)]
    pub meshes: Vec<Mesh>,

    pub unk1: u32,
    pub max_xyz: [f32; 3],
    pub min_xyz: [f32; 3],
    pub bounding_radius: f32,
    pub unks: [u32; 7],
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct Mesh {
    pub render_flags: u32,
    pub skin_flags: u32,
    pub vertex_buffer_index: u16,
    pub index_buffer_index: u16,
    pub unk_index: u16,
    pub material_index: u16,
    pub unk2: u32,
    pub unk3: u32,
    pub unk4: u32,
    pub unk5: u16,
    pub lod: u16, // TODO: flags?
    // TODO: groups?
    pub unks6: [i32; 4],
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
#[br(import_raw(base_offset: u64))]
pub struct ModelUnk {
    #[br(parse_with = parse_string_ptr32, offset = base_offset)]
    #[xc3(offset32)]
    name1: String,

    // TODO: Always an empty string?
    #[br(parse_with = parse_string_ptr32, offset = base_offset)]
    #[xc3(offset32)]
    name2: String,

    unk1: u16,
    unk2: u16,
    unk3: u32,
}

#[binread]
#[derive(Debug, Xc3Write, Xc3WriteOffsets)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct MorphControllers {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    // TODO: same count as morph targets per descriptor in vertex data?
    #[br(parse_with = parse_offset32_count32, args { offset: base_offset, inner: base_offset })]
    #[xc3(offset32_count32)]
    controllers: Vec<MorphController>,

    unk1: u32,
    unk2: u32,
    unk3: u32,
    unk4: u32,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
#[br(import_raw(base_offset: u64))]
pub struct MorphController {
    #[br(parse_with = parse_string_ptr32, offset = base_offset)]
    #[xc3(offset32)]
    name1: String,

    #[br(parse_with = parse_string_opt_ptr32, offset = base_offset)]
    #[xc3(offset32)]
    name2: Option<String>,

    unk1: u32,
    unk2: u32,
    unk3: u32,
    unk4: u32,
    unk5: u32,
}

#[binread]
#[derive(Debug, Xc3Write, Xc3WriteOffsets)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct ModelUnk3 {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    #[br(parse_with = parse_count32_offset32, args { offset: base_offset, inner: base_offset })]
    #[xc3(count32_offset32)]
    pub items: Vec<ModelUnk3Inner>,

    // TODO: padding?
    pub unk: [u32; 4],
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
#[br(import_raw(base_offset: u64))]
pub struct ModelUnk3Inner {
    // DECL_GBL_CALC
    #[br(parse_with = parse_string_ptr32, offset = base_offset)]
    #[xc3(offset32)]
    name: String,

    unk1: u32,
    unk2: u32,
    unk3: u32,
    unk4: u32,
    unk5: u32,
    unk6: u32,
    unk7: u32,
}

#[binread]
#[derive(Debug, Xc3Write, Xc3WriteOffsets)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct ModelUnk4 {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    // 0 ... N-1
    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset32_count32)]
    items: Vec<u32>,

    unk1: u32,
    unk2: u32,
    unk3: u32,
    unk4: u32,
}

#[binread]
#[derive(Debug, Xc3Write, Xc3WriteOffsets)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct ModelUnk5 {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    // TODO: What type is this?
    #[br(parse_with = parse_count32_offset32, offset = base_offset)]
    #[xc3(count32_offset32)]
    pub items: Vec<[u32; 2]>,

    unk1: u32,
    unk2: u32,
    unk3: u32,
    unk4: u32,
}

// TODO: Some sort of animation?
#[binread]
#[derive(Debug, Xc3Write)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct MeshUnk1 {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    #[br(parse_with = parse_offset32_count32)]
    #[br(args { offset: base_offset, inner: base_offset })]
    #[xc3(offset32_count32)]
    pub items1: Vec<MeshUnk1Item1>,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset32_count32)]
    pub items2: Vec<MeshUnk1Item2>,

    #[br(parse_with = parse_ptr32)]
    #[br(args { offset: base_offset, inner: args! { count: items1.len() }})]
    #[xc3(offset32)]
    pub items3: Vec<f32>,
    pub unk1: u32, // 0 or 1?

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset32_count32)]
    pub items4: Vec<[u32; 5]>,

    // flags?
    pub unk4: u32,
    pub unk5: u32,

    // TODO: Is this the correct check?
    #[br(parse_with = parse_opt_ptr32, offset = base_offset)]
    #[xc3(offset32)]
    #[br(if(unk4 != 0 || unk5 != 0))]
    pub unk_inner: Option<MeshUnk1Inner>,
    // TODO: padding if unk_inner?
    // TODO: only 12 bytes for chr/ch/ch01022012.wimdo?
    // pub unk: [u32; 4],
}

#[binread]
#[derive(Debug, Xc3Write, Xc3WriteOffsets)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct MeshUnk1Inner {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset32_count32)]
    pub items1: Vec<(u16, u16)>,

    // 0..N-1 arranged in a different order?
    #[br(parse_with = parse_ptr32)]
    #[br(args {
        offset: base_offset,
        inner: args! {
            count: items1.iter().map(|(a,_)| *a).max().unwrap_or_default() as usize * 2
        }
    })]
    #[xc3(offset32)]
    pub unk_offset: Vec<u16>,

    // TODO: padding?
    pub unks: [u32; 5],
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
#[br(import_raw(base_offset: u64))]
pub struct MeshUnk1Item1 {
    #[br(parse_with = parse_string_ptr32, offset = base_offset)]
    #[xc3(offset32)]
    pub name: String,
    // TODO: padding?
    pub unk: [u32; 3],
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct MeshUnk1Item2 {
    pub unk1: u32,
    pub unk2: u32,
    pub unk3: u32,
    pub unk4: u32,
    pub unk5: u32,
}

#[binread]
#[derive(Debug, Xc3Write)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct LodData {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    pub unk1: u32,

    // TODO: Count related to number of mesh lod values?
    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset32_count32)]
    pub items1: Vec<LodItem1>,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset32_count32)]
    pub items2: Vec<LodItem2>,

    pub unks: [u32; 4],
}

// TODO: is lod: 0 in the mxmd special?
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct LodItem1 {
    pub unk1: [u32; 4],
    pub unk2: f32,
    // second element is index related to count in LodItem2?
    // [0,0,1,0], [0,1,1,0], [0,2,1,0], ...
    pub unk3: [u8; 4],
    pub unk4: [u32; 2],
}

// TODO: lod group?
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct LodItem2 {
    pub base_lod_index: u16,
    pub lod_count: u16,
    // TODO: padding?
    pub unk1: u32,
    pub unk2: u32,
}

// TODO: Derive Xc3Write?
#[binread]
#[derive(Debug)]
#[br(stream = r)]
pub struct Textures {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    pub tag: u32, // 4097 or sometimes 0?

    // TODO: How to derive for non offset fields that have offsets?
    #[br(args { base_offset, tag })]
    pub inner: TexturesInner,
}

#[derive(Debug, BinRead)]
#[br(import { base_offset: u64, tag: u32 })]
pub enum TexturesInner {
    #[br(pre_assert(tag == 0))]
    Unk0(#[br(args_raw(base_offset))] Textures1),

    #[br(pre_assert(tag == 4097))]
    Unk1(#[br(args_raw(base_offset))] Textures2),
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
#[br(import_raw(base_offset: u64))]
pub struct Textures1 {
    pub unk1: u32, // TODO: count for multiple packed textures?
    // low textures?
    #[br(parse_with = parse_ptr32, offset = base_offset)]
    #[xc3(offset32)]
    pub textures1: PackedExternalTextures,

    // high textures?
    #[br(parse_with = parse_opt_ptr32, offset = base_offset)]
    #[xc3(offset32)]
    pub textures2: Option<PackedExternalTextures>,

    pub unk4: u32,
    pub unk5: u32,
    // TODO: more fields?
}

#[derive(Debug, BinRead, Xc3Write)]
#[br(import_raw(base_offset: u64))]
pub struct Textures2 {
    pub unk1: u32, // 103

    #[br(parse_with = parse_count32_offset32, offset = base_offset)]
    #[xc3(count32_offset32)]
    pub unk2: Vec<[u32; 5]>,

    #[br(parse_with = parse_count32_offset32, offset = base_offset)]
    #[xc3(count32_offset32)]
    pub unk3: Vec<TexturesUnk>,

    pub unk4: [u32; 7],

    #[br(parse_with = parse_count32_offset32, offset = base_offset)]
    #[xc3(count32_offset32)]
    pub indices: Vec<u16>,

    #[br(parse_with = parse_opt_ptr32, offset = base_offset)]
    #[xc3(offset32)]
    pub textures: Option<PackedExternalTextures>,

    pub unk5: u32,

    // TODO: same as the type in msrd?
    #[br(parse_with = parse_count32_offset32, offset = base_offset)]
    #[xc3(count32_offset32)]
    pub resources: Vec<TextureResource>,

    // TODO: padding?
    pub unk: [u32; 4],
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct TexturesUnk {
    pub unk1: u32,
    pub unk2: u32,
    pub unk3: u32,
}

#[binread]
#[derive(Debug, Xc3Write, Xc3WriteOffsets)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct PackedTextures {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    #[br(parse_with = parse_count32_offset32, args { offset: base_offset, inner: base_offset })]
    #[xc3(count32_offset32)]
    pub textures: Vec<PackedTexture>,

    pub unk2: u32,
    pub strings_offset: u32,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
#[br(import_raw(base_offset: u64))]
pub struct PackedTexture {
    pub unk1: u32,

    // TODO: Optimized function for reading bytes?
    #[br(parse_with = parse_count32_offset32, offset = base_offset)]
    #[xc3(count32_offset32)]
    pub mibl_data: Vec<u8>,

    #[br(parse_with = parse_string_ptr32, offset = base_offset)]
    #[xc3(offset32)]
    pub name: String,
}

#[binread]
#[derive(Debug, Xc3Write, Xc3WriteOffsets)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct PackedExternalTextures {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    #[br(parse_with = parse_count32_offset32, args { offset: base_offset, inner: base_offset })]
    #[xc3(count32_offset32, align(2))]
    pub textures: Vec<PackedExternalTexture>,

    pub unk2: u32,
    pub strings_offset: u32,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
#[br(import_raw(base_offset: u64))]
pub struct PackedExternalTexture {
    pub unk1: u32,

    // TODO: These offsets are for different places for maps and characters?
    pub mibl_length: u32,
    pub mibl_offset: u32,

    #[br(parse_with = parse_string_ptr32, offset = base_offset)]
    #[xc3(offset32)]
    pub name: String,
}

// TODO: Fix offset writing.
#[binread]
#[derive(Debug, Xc3Write)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct Skinning {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    pub count1: u32,
    pub count2: u32,

    // TODO: Find a simpler way of writing this?
    // TODO: helper for separate count.
    #[br(parse_with = parse_ptr32)]
    #[br(args {
        offset: base_offset,
        inner: args! { count: count1 as usize, inner: base_offset }
    })]
    #[xc3(offset32)]
    pub bones: Vec<Bone>,

    // TODO: inverse bind matrix?
    /// Column-major transformation matrices for each of the bones in [bones](#structfield.bones).
    #[br(parse_with = parse_ptr32)]
    #[br(args { offset: base_offset, inner: args! { count: count1 as usize } })]
    #[xc3(offset32, align(16))]
    pub transforms1: Vec<[[f32; 4]; 4]>,

    // TODO: Count related to bone unk_type?
    #[br(parse_with = parse_opt_ptr32)]
    #[br(args { offset: base_offset, inner: args! { count: 4 } })]
    #[xc3(offset32)]
    pub transforms2: Option<Vec<[f32; 4]>>,

    // TODO: related to max unk index on bone?
    #[br(parse_with = parse_opt_ptr32)]
    #[br(args {
        offset: base_offset,
        inner: args! { count: bones.iter().map(|b| b.unk_index as usize + 1).max().unwrap_or_default() }
    })]
    #[xc3(offset32)]
    pub transforms3: Option<Vec<[[f32; 4]; 2]>>,

    // TODO: 0..count-1?
    #[br(parse_with = parse_count32_offset32, offset = base_offset)]
    #[xc3(count32_offset32)]
    pub bone_indices: Vec<u16>,

    #[br(parse_with = parse_opt_ptr32)]
    #[br(args { offset: base_offset, inner: base_offset })]
    #[br(if(transforms2.is_some()))]
    #[xc3(offset32)]
    pub unk_offset4: Option<SkeletonUnk4>,

    #[br(parse_with = parse_opt_ptr32, offset = base_offset)]
    #[br(if(transforms3.is_some()))]
    #[xc3(offset32)]
    pub unk_offset5: Option<SkeletonUnk5>,

    // TODO: procedural bones?
    #[br(parse_with = parse_opt_ptr32, args { offset: base_offset, inner: base_offset })]
    #[br(if(!bone_indices.is_empty()))]
    #[xc3(offset32)]
    pub as_bone_data: Option<AsBoneData>,
    // TODO: padding if as bone data is present??
    // pub unk: [u32; 4],
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
#[br(import_raw(base_offset: u64))]
pub struct Bone {
    #[br(parse_with = parse_string_ptr32, offset = base_offset)]
    #[xc3(offset32)]
    pub name: String,
    pub unk1: f32,
    pub unk_type: (u16, u16),
    /// Index into [transforms3](struct.Skinning.html#structfield.transforms3).
    pub unk_index: u32,
    // TODO: padding?
    pub unk: [u32; 2],
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
#[br(import_raw(base_offset: u64))]
pub struct SkeletonUnk4 {
    // TODO: u16 indices?
    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset32_count32)]
    pub unk1: Vec<[u16; 21]>,

    #[br(parse_with = parse_ptr32)]
    #[br(args { offset: base_offset, inner: args! { count: unk1.len() }})]
    #[xc3(offset32)]
    pub unk_offset: Vec<[[f32; 4]; 4]>,
    // TODO: no padding?
}

#[binread]
#[derive(Debug, Xc3Write, Xc3WriteOffsets)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct SkeletonUnk5 {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    // TODO: element size?
    #[br(parse_with = parse_count32_offset32, offset = base_offset)]
    #[xc3(count32_offset32)]
    pub unk1: Vec<[u16; 105]>,

    // TODO: count?
    #[br(parse_with = parse_opt_ptr32, offset = base_offset)]
    #[xc3(offset32)]
    pub unk_offset: Option<[f32; 12]>,

    // TODO: padding?
    pub unk: [u32; 5],
}

// TODO: Data for AS_ bones?
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
#[br(import_raw(base_offset: u64))]
pub struct AsBoneData {
    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset32_count32)]
    pub bones: Vec<AsBone>,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset32_count32)]
    pub unk1: Vec<AsBoneValue>,

    #[br(parse_with = parse_ptr32)]
    #[br(args { offset: base_offset, inner: args! { count: bones.len() * 3 }})]
    #[xc3(offset32)]
    pub unk2: Vec<[[f32; 4]; 4]>,

    pub unk3: u32,

    // TODO: padding?
    pub unk: [u32; 2],
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct AsBone {
    /// The index in [bones](struct.Skeleton.html#structfield.bones).
    pub bone_index: u16,
    /// The index in [bones](struct.Skeleton.html#structfield.bones) of the parent bone.
    pub parent_index: u16,
    pub unk: [u32; 19],
}

// TODO: Some of these aren't floats?
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct AsBoneValue {
    unk1: [f32; 4],
    unk2: [f32; 4],
    unk3: [f32; 4],
    unk4: [f32; 2],
}

// TODO: pointer to decl_gbl_cac in ch001011011.wimdo?
#[binread]
#[derive(Debug, Xc3Write, Xc3WriteOffsets)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct Unk1 {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    #[br(parse_with = parse_count32_offset32, offset = base_offset)]
    #[xc3(count32_offset32)]
    pub unk1: Vec<Unk1Unk1>,

    #[br(parse_with = parse_count32_offset32, offset = base_offset)]
    #[xc3(count32_offset32)]
    pub unk2: Vec<Unk1Unk2>,

    #[br(parse_with = parse_count32_offset32, offset = base_offset)]
    #[xc3(count32_offset32)]
    pub unk3: Vec<Unk1Unk3>,

    // TODO: Don't write offset if zero count.
    // angle values?
    #[br(parse_with = parse_count32_offset32, offset = base_offset)]
    #[xc3(count32_offset32)]
    pub unk4: Vec<Unk1Unk4>,

    // TODO: padding?
    pub unk: [u32; 4],
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct Unk1Unk1 {
    pub index: u16,
    pub unk2: u16, // 1
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct Unk1Unk2 {
    pub unk1: u16, // 0
    pub index: u16,
    pub unk3: u16,
    pub unk4: u16,
    pub unk5: u32, // 0
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct Unk1Unk3 {
    pub unk1: u16,
    pub unk2: u16,
    pub unk3: u32,
    pub unk4: u16,
    pub unk5: u16,
    pub unk6: u16,
    pub unk7: u16,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct Unk1Unk4 {
    pub unk1: f32,
    pub unk2: f32,
    pub unk3: f32,
    pub unk4: u32,
}

xc3_write_binwrite_impl!(ParamType, Sampler, ShaderUnkType, StateFlags);

impl Xc3Write for MaterialFlags {
    type Offsets<'a> = ();

    fn xc3_write<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        data_ptr: &mut u64,
    ) -> binrw::BinResult<Self::Offsets<'_>> {
        u32::from(*self).write_le(writer)?;
        *data_ptr = (*data_ptr).max(writer.stream_position()?);
        Ok(())
    }
}

// TODO: Derive this?
#[doc(hidden)]
pub struct TexturesOffsets<'a> {
    base_offset: u64,
    inner: TexturesOffsetsInner<'a>,
}

#[doc(hidden)]
pub enum TexturesOffsetsInner<'a> {
    Unk0(Textures1Offsets<'a>),
    Unk1(Textures2Offsets<'a>),
}

// TODO: Derive this?
impl Xc3Write for Textures {
    type Offsets<'a> = TexturesOffsets<'a>;

    fn xc3_write<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        data_ptr: &mut u64,
    ) -> binrw::BinResult<Self::Offsets<'_>> {
        let base_offset = writer.stream_position()?;
        self.tag.write_le(writer)?;
        let inner = match &self.inner {
            TexturesInner::Unk0(t) => TexturesOffsetsInner::Unk0(t.xc3_write(writer, data_ptr)?),
            TexturesInner::Unk1(t) => TexturesOffsetsInner::Unk1(t.xc3_write(writer, data_ptr)?),
        };
        Ok(TexturesOffsets { base_offset, inner })
    }
}

impl<'a> Xc3WriteOffsets for TexturesOffsets<'a> {
    fn write_offsets<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        _base_offset: u64,
        data_ptr: &mut u64,
    ) -> binrw::BinResult<()> {
        let base_offset = self.base_offset;
        match &self.inner {
            TexturesOffsetsInner::Unk0(offsets) => {
                offsets.write_offsets(writer, base_offset, data_ptr)
            }
            TexturesOffsetsInner::Unk1(offsets) => {
                offsets.write_offsets(writer, base_offset, data_ptr)
            }
        }
    }
}

impl<'a> Xc3WriteOffsets for SkinningOffsets<'a> {
    fn write_offsets<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        _base_offset: u64,
        data_ptr: &mut u64,
    ) -> binrw::BinResult<()> {
        let base_offset = self.base_offset;

        let bones = self.bones.write_offset(writer, base_offset, data_ptr)?;

        self.bone_indices
            .write_full(writer, base_offset, data_ptr)?;

        self.transforms1.write_full(writer, base_offset, data_ptr)?;

        self.transforms2.write_full(writer, base_offset, data_ptr)?;
        self.transforms3.write_full(writer, base_offset, data_ptr)?;

        self.unk_offset4.write_full(writer, base_offset, data_ptr)?;

        self.as_bone_data
            .write_full(writer, base_offset, data_ptr)?;

        self.unk_offset5.write_full(writer, base_offset, data_ptr)?;

        for bone in bones.0 {
            bone.name.write_full(writer, base_offset, data_ptr)?;
        }

        Ok(())
    }
}

impl<'a> Xc3WriteOffsets for MeshUnk1Offsets<'a> {
    fn write_offsets<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        _base_offset: u64,
        data_ptr: &mut u64,
    ) -> binrw::BinResult<()> {
        let base_offset = self.base_offset;

        let items1 = self.items1.write_offset(writer, base_offset, data_ptr)?;

        self.items3.write_full(writer, base_offset, data_ptr)?;

        self.items2.write_full(writer, base_offset, data_ptr)?;

        // TODO: Set alignment at type level for Xc3Write?
        *data_ptr = round_up(*data_ptr, 16);
        self.items4.write_full(writer, base_offset, data_ptr)?;

        for item in items1.0 {
            item.name.write_full(writer, base_offset, data_ptr)?;
        }

        self.unk_inner.write_full(writer, base_offset, data_ptr)?;

        Ok(())
    }
}

impl<'a> Xc3WriteOffsets for LodDataOffsets<'a> {
    fn write_offsets<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        _base_offset: u64,
        data_ptr: &mut u64,
    ) -> binrw::BinResult<()> {
        let base_offset = self.base_offset;
        // Different order than field order.
        self.items2.write_full(writer, base_offset, data_ptr)?;
        self.items1.write_full(writer, base_offset, data_ptr)?;
        Ok(())
    }
}

impl<'a> Xc3WriteOffsets for ModelsOffsets<'a> {
    fn write_offsets<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        _base_offset: u64,
        data_ptr: &mut u64,
    ) -> binrw::BinResult<()> {
        let base_offset = self.base_offset;

        self.models.write_full(writer, base_offset, data_ptr)?;
        self.skinning.write_full(writer, base_offset, data_ptr)?;
        self.model_unks.write_full(writer, base_offset, data_ptr)?;
        self.morph_controllers
            .write_full(writer, base_offset, data_ptr)?;

        // Different order than field order.
        self.lod_data.write_full(writer, base_offset, data_ptr)?;
        self.unk_offset1.write_full(writer, base_offset, data_ptr)?;
        self.model_unk4.write_full(writer, base_offset, data_ptr)?;
        self.model_unk3.write_full(writer, base_offset, data_ptr)?;
        self.model_unk5.write_full(writer, base_offset, data_ptr)?;

        Ok(())
    }
}

impl<'a> Xc3WriteOffsets for ShaderProgramInfoOffsets<'a> {
    fn write_offsets<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
    ) -> binrw::BinResult<()> {
        // Different order than field order.
        self.unk1.write_full(writer, base_offset, data_ptr)?;
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

impl<'a> Xc3WriteOffsets for MaterialsOffsets<'a> {
    fn write_offsets<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        _base_offset: u64,
        data_ptr: &mut u64,
    ) -> binrw::BinResult<()> {
        let base_offset = self.base_offset;

        // Material fields get split up and written in a different order.
        let materials = self.materials.write_offset(writer, base_offset, data_ptr)?;

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
        self.alpha_test_textures
            .write_full(writer, base_offset, data_ptr)?;
        self.unk_offset1.write_full(writer, base_offset, data_ptr)?;
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
    ) -> binrw::BinResult<()> {
        self.models.write_full(writer, base_offset, data_ptr)?;
        self.materials.write_full(writer, base_offset, data_ptr)?;

        // Different order than field order.
        self.textures.write_full(writer, base_offset, data_ptr)?;

        // TODO: 16 bytes of padding before this?
        *data_ptr += 16;
        self.unk1.write_full(writer, base_offset, data_ptr)?;

        self.vertex_data.write_full(writer, base_offset, data_ptr)?;
        self.spch.write_full(writer, base_offset, data_ptr)?;
        self.packed_textures
            .write_full(writer, base_offset, data_ptr)?;

        Ok(())
    }
}

impl<'a> Xc3WriteOffsets for Textures2Offsets<'a> {
    fn write_offsets<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
    ) -> binrw::BinResult<()> {
        self.unk2.write_full(writer, base_offset, data_ptr)?;
        self.unk3.write_full(writer, base_offset, data_ptr)?;

        // Different order than field order.
        self.resources.write_full(writer, base_offset, data_ptr)?;
        self.indices.write_full(writer, base_offset, data_ptr)?;
        self.textures.write_full(writer, base_offset, data_ptr)?;

        Ok(())
    }
}
