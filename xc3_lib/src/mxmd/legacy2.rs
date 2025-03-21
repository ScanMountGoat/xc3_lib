use crate::{
    msrd::{Streaming, StreamingDataLegacyInner},
    parse_count32_offset32, parse_count32_offset32_unchecked, parse_offset, parse_offset32_count32,
    parse_opt_ptr32, parse_ptr32, parse_string_ptr32,
    spco::Spco,
    vertex::VertexAttribute,
    xc3_write_binwrite_impl, StringOffset32,
};
use binrw::{args, binread, BinRead, BinWrite};
use xc3_write::{Xc3Write, Xc3WriteOffsets};

use super::{
    legacy::{Materials, Models, VertexData},
    PackedTextures,
};

// TODO: How much code can be shared with modern switch formats?
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, PartialEq, Clone)]
#[br(magic(b"DMXM"))]
#[xc3(magic(b"DMXM"))]
pub struct MxmdLegacy2 {
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

    // #[br(parse_with = parse_opt_ptr32)]
    // #[xc3(offset(u32))]
    pub unk1: u32,

    #[br(parse_with = parse_opt_ptr32)]
    #[xc3(offset(u32))]
    pub vertex_data: Option<VertexData>,

    #[br(parse_with = parse_opt_ptr32)]
    #[xc3(offset(u32))]
    pub shaders: Option<Spco>,

    #[br(parse_with = parse_opt_ptr32)]
    #[xc3(offset(u32))]
    pub packed_textures: Option<PackedTextures>,

    pub unk3: u32,

    /// Streaming information for the .wismt file or [None] if no .wismt file.
    #[br(parse_with = parse_opt_ptr32)]
    #[xc3(offset(u32))]
    pub streaming: Option<Streaming>,

    // TODO: padding?
    pub unk: [u32; 7],
}
