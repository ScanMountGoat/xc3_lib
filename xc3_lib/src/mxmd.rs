use std::io::SeekFrom;

use crate::{parse_count_offset, parse_offset_count, parse_ptr32, parse_string_ptr32};
use bilge::prelude::*;
use binrw::{args, binread, BinRead, FilePtr32, NamedArgs};
use serde::Serialize;

/// .wimdo files
#[binread]
#[derive(Debug, Serialize)]
#[br(magic(b"DMXM"))]
pub struct Mxmd {
    version: u32,

    #[br(parse_with = FilePtr32::parse)]
    pub mesh: Mesh,

    #[br(parse_with = FilePtr32::parse)]
    pub materials: Materials,

    #[br(parse_with = parse_ptr32)]
    unk1: Option<Unk1>,
    unk2: u32,
    unk3: u32,
    unk4: u32,
    unk5: u32,

    // uncached textures?
    #[br(parse_with = FilePtr32::parse)]
    pub textures: Textures,
}

#[binread]
#[derive(Debug, Serialize)]
#[br(stream = r)]
pub struct Materials {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    #[br(args { base_offset, inner: base_offset })]
    pub materials: List<Material>,

    // offset?
    unk1: u32,
    unk2: u32,

    // TODO: Materials have offsets into these arrays for parameter values?
    #[br(parse_with = parse_offset_count, args_raw(base_offset))]
    floats: Vec<f32>,

    #[br(parse_with = parse_offset_count, args_raw(base_offset))]
    ints: Vec<u32>,

    #[br(parse_with = FilePtr32::parse)]
    #[br(args { offset: base_offset, inner: base_offset })]
    unk_offset1: MaterialUnk1,

    // is this ever not 0?
    unk4: u32,

    // TODO: How large is each element?
    #[br(parse_with = parse_offset_count, args_raw(base_offset))]
    unks: Vec<MaterialUnk>,

    unks1: [u32; 2],

    #[br(parse_with = parse_count_offset, args_raw(base_offset))]
    unks2: Vec<(u32, u32)>,

    unks3: [u32; 7],

    #[br(parse_with = FilePtr32::parse, offset = base_offset)]
    pub samplers: Samplers,

    // padding?
    unks4: [u32; 4],
}

#[binread]
#[derive(Debug, Serialize)]
pub struct MaterialUnk {
    unk1: [u16; 8],
}

#[binread]
#[derive(Debug, Serialize)]
#[br(import_raw(base_offset: u64))]
pub struct MaterialUnk1 {
    #[br(parse_with = parse_offset_count, args_raw(base_offset))]
    unk1: Vec<u32>,
    #[br(parse_with = parse_offset_count, args_raw(base_offset))]
    unk2: Vec<u16>,
}

#[binread]
#[derive(Debug, Serialize)]
pub struct Samplers {
    unk1: u32, // count?
    unk2: u32, // offset?
    unk3: u32, // pad?
    unk4: u32, // pad?

    // pointed to by above?
    #[br(count = unk1)]
    pub samplers: Vec<Sampler>,
}

#[binread]
#[derive(Debug, Serialize)]
pub struct Sampler {
    // TODO: Serialize bitfields like structs?
    #[br(map(|x: u32| x.into()))]
    #[serde(skip_serializing)]
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
    unk1: bool,
    unk3: bool,
    unk: u23,
}

#[binread]
#[derive(Debug, Serialize)]
#[br(import_raw(base_offset: u64))]
pub struct Material {
    #[br(parse_with = parse_string_ptr32, args(base_offset))]
    pub name: String,

    unk1: u16,
    unk2: u16,
    unk3: u16,
    unk4: u16,

    /// Color multiplier value assigned to the `gMatCol` shader uniform.
    pub color: [f32; 4],

    unk_float: f32,

    // TODO: materials with zero textures?
    /// Defines the shader's sampler bindings in order for s0, s1, s2, ...
    #[br(parse_with = parse_offset_count, args_raw(base_offset))]
    pub textures: Vec<Texture>,

    pub flags: MaterialFlags,

    m_unks1: [u32; 6],
    m_unk5: u32,

    // always count 1?
    #[br(parse_with = parse_offset_count, args_raw(base_offset))]
    pub shader_programs: Vec<ShaderProgram>,

    m_unks2: [u16; 16],
}

#[binread]
#[derive(Debug, Serialize)]
pub struct MaterialFlags {
    pub flag0: u8,
    pub blend_state: BlendState,
    pub cull_mode: CullMode,
    pub flag3: u8,
    pub stencil_state1: StencilState1,
    pub stencil_state2: StencilState2,
    pub depth_func: DepthFunc,
    pub flag7: u8,
}

// TODO: Convert these to equations for RGB and alpha for docs.
// TODO: Is it worth documenting this outside of xc3_wgpu?
// flag, col src, col dst, col op, alpha src, alpha dst, alpha op
// 0 = disabled
// 1, Src Alpha, 1 - Src Alpha, Add, Src Alpha, 1 - Src Alpha, Add
// 2, Src Alpha, One, Add, Src Alpha, One, Add
// 3, Zero, Src Col, Add, Zero, Src Col, Add
// 6, disabled + ???
#[binread]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[br(repr(u8))]
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
#[binread]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[br(repr(u8))]
pub enum StencilState1 {
    Always = 0,
    Unk1 = 1,
    Always2 = 4,
    Unk8 = 8,
    UnkHair = 16,
    Unk20 = 20,
}

// TODO: Does this flag actually disable stencil?
#[binread]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[br(repr(u8))]
pub enum StencilState2 {
    Disabled = 0,
    Enabled = 1,
    Unk6 = 6,
    Unk7 = 7,
    Unk8 = 8,
}

#[binread]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[br(repr(u8))]
pub enum DepthFunc {
    Disabled = 0,
    LessEqual = 1,
    Equal = 3,
}

#[binread]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[br(repr(u8))]
pub enum CullMode {
    Back = 0,
    Front = 1,
    Disabled = 2,
    Unk3 = 3, // front + ???
}

#[binread]
#[derive(Debug, Serialize)]
pub struct ShaderProgram {
    pub program_index: u32, // index into programs in wismt?
    pub unk_type: ShaderUnkType,
    pub parent_material_index: u16, // index of the parent material?
    pub unk4: u32,                  // always 1?
}

// Affects what pass the object renders in?
// Each "pass" has different render targets?
// _trans = 1,
// _ope = 0,1,7
// _zpre = 0
// _outline = 0
#[binread]
#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize)]
#[br(repr(u16))]
pub enum ShaderUnkType {
    Unk0 = 0, // main opaque + some transparent?
    Unk1 = 1, // second layer transparent?
    Unk7 = 7, // additional eye effect layer?
}

#[binread]
#[derive(Debug, Serialize)]
pub struct Texture {
    pub texture_index: u16,
    pub sampler_index: u16,
    pub unk2: u16,
    pub unk3: u16,
}

#[binread]
#[derive(Debug, Serialize)]
#[br(stream = r)]
pub struct Mesh {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    unk1: u32,

    max_xyz: [f32; 3],
    min_xyz: [f32; 3],

    #[br(args { base_offset, inner: base_offset })]
    pub items: List<DataItem>,

    unk2: u32,

    #[br(parse_with = parse_ptr32, args_raw(base_offset))]
    skeleton: Option<Skeleton>,

    unks3: [u32; 22],

    #[br(parse_with = parse_ptr32, args_raw(base_offset))]
    pub unk_offset1: Option<MeshUnk1>,

    unk_offset2: u32,

    #[br(parse_with = parse_ptr32, args_raw(base_offset))]
    lod_data: Option<LodData>,
}

// TODO: Better names for these types
#[binread]
#[derive(Debug, Serialize)]
#[br(import_raw(base_offset: u64))]
pub struct DataItem {
    #[br(parse_with = parse_offset_count, args_raw(base_offset))]
    pub sub_items: Vec<SubDataItem>,

    unk1: u32,
    max_xyz: [f32; 3],
    min_xyz: [f32; 3],
    bounding_radius: f32,
    unks: [u32; 7],
}

// TODO: Better names for these types
#[binread]
#[derive(Debug, Serialize)]
pub struct SubDataItem {
    flags1: u32,
    flags2: u32,
    pub vertex_buffer_index: u16,
    pub index_buffer_index: u16,
    unk_index: u16,
    pub material_index: u16,
    unk2: u32,
    unk3: u32,
    unk4: u32,
    unk5: u16,
    pub lod: u16,
    // TODO: groups?
    unks6: [i32; 4],
}

#[binread]
#[derive(Debug, Serialize)]
#[br(stream = r)]
pub struct MeshUnk1 {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    #[br(parse_with = FilePtr32::parse)]
    #[br(args { offset: base_offset, inner: base_offset })]
    pub inner: MeshUnk1Inner,
    unk1: [u32; 14],
}

#[binread]
#[derive(Debug, Serialize)]
#[br(import_raw(base_offset: u64))]
pub struct MeshUnk1Inner {
    #[br(parse_with = parse_string_ptr32, args(base_offset))]
    pub unk1: String,

    unk2: [f32; 9],
}

#[binread]
#[derive(Debug, Serialize)]
#[br(stream = r)]
pub struct LodData {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    unk1: u32,

    // another list?
    unk2: u32,
    unk3: u32,

    #[br(parse_with = parse_offset_count, args_raw(base_offset))]
    items: Vec<(u16, u16)>,
}

#[binread]
#[derive(Debug, Serialize)]
#[br(stream = r)]
pub struct Textures {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    unks: [u32; 5],

    unk_offset: u32, // 292 bytes?

    unks2: [u32; 8],

    #[br(parse_with = FilePtr32::parse, offset = base_offset)]
    unk2: [u32; 7],

    #[br(parse_with = parse_ptr32, args_raw(base_offset))]
    pub items: Option<TextureItems>,
}

#[binread]
#[derive(Debug, Serialize)]
#[br(stream = r)]
pub struct TextureItems {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    count: u32,
    offset: u32,
    unk2: u32,
    strings_offset: u32,

    #[br(args { count: count as usize, inner: args! { base_offset } })]
    pub textures: Vec<TextureItem>,
}

#[binread]
#[derive(Debug, Serialize)]
#[br(import { base_offset: u64 })]
pub struct TextureItem {
    unk1: u16,
    unk2: u16,
    unk3: u16, // size?
    unk4: u16,
    unk5: u16, // some sort of offset (sum of previous unk3)?
    unk6: u16,

    #[br(parse_with = parse_string_ptr32, args(base_offset))]
    pub name: String,
}

#[binread]
#[derive(Debug, Serialize)]
#[br(stream = r)]
pub struct Skeleton {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    count1: u32,
    count2: u32,

    // TODO: Find a simpler way of writing this?
    #[br(parse_with = FilePtr32::parse)]
    #[br(args {
        offset: base_offset,
        inner: args! {
            count: count1 as usize,
            inner: base_offset
        }
    })]
    bones: Vec<Bone>,

    // TODO: Create a matrix type?
    #[br(parse_with = FilePtr32::parse)]
    #[br(args { offset: base_offset, inner: args! { count: count1 as usize } })]
    transforms: Vec<[[f32; 4]; 4]>,

    unk_offset1: u32,
    unk_offset2: u32,
    count3: u32,
    unk_offset3: u32,
    unk_offset4: u32,
}

#[binread]
#[derive(Debug, Serialize)]
#[br(import_raw(base_offset: u64))]
pub struct Bone {
    #[br(parse_with = parse_string_ptr32, args(base_offset))]
    name: String,
    unk1: f32,
    unk_type: u32,
    #[br(pad_after = 8)]
    unk_index: u32,
}

#[binread]
#[derive(Debug, Serialize)]
#[br(stream = r)]
pub struct Unk1 {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    #[br(parse_with = parse_count_offset, args_raw(base_offset))]
    unk1: Vec<Unk1Unk1>,

    #[br(parse_with = parse_count_offset, args_raw(base_offset))]
    unk2: Vec<Unk1Unk2>,

    #[br(parse_with = parse_count_offset, args_raw(base_offset))]
    unk3: Vec<Unk1Unk3>,

    // angle values?
    #[br(parse_with = parse_count_offset, args_raw(base_offset))]
    unk4: Vec<Unk1Unk4>,
}

#[binread]
#[derive(Debug, Serialize)]
pub struct Unk1Unk1 {
    index: u16,
    unk2: u16, // 1
}

#[binread]
#[derive(Debug, Serialize)]
pub struct Unk1Unk2 {
    unk1: u16, // 0
    index: u16,
    unk3: u16,
    unk4: u16,
    unk5: u32, // 0
}

#[binread]
#[derive(Debug, Serialize)]
pub struct Unk1Unk3 {
    unk1: u16,
    unk2: u16,
    unk3: u32,
    unk4: u16,
    unk5: u16,
    unk6: u16,
    unk7: u16,
}

#[binread]
#[derive(Debug, Serialize)]
pub struct Unk1Unk4 {
    unk1: f32,
    unk2: f32,
    unk3: f32,
    unk4: u32,
}

/// A [u32] offset and [u32] count with an optional base offset.
#[derive(Clone, NamedArgs)]
pub struct ListArgs<Inner: Default> {
    #[named_args(default = 0)]
    base_offset: u64,
    #[named_args(default = Inner::default())]
    inner: Inner,
}

#[binread]
#[derive(Debug, Serialize)]
#[br(import_raw(args: ListArgs<T::Args<'_>>))]
#[serde(transparent)]
pub struct List<T>
where
    T: BinRead + 'static,
    for<'a> <T as BinRead>::Args<'a>: Clone + Default,
{
    #[br(temp)]
    offset: u32,
    #[br(temp)]
    count: u32,

    #[br(args { count: count as usize, inner: args.inner })]
    #[br(seek_before = SeekFrom::Start(args.base_offset + offset as u64))]
    #[br(restore_position)]
    pub elements: Vec<T>,
}
