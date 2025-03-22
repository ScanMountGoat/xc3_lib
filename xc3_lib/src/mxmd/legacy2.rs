//! Legacy types for Xenoblade Chronicles X DE.
use binrw::BinRead;
use xc3_write::{Xc3Write, Xc3WriteOffsets};

use crate::{msrd::Streaming, parse_opt_ptr32, parse_ptr32, spco::Spco};

use super::{
    legacy::{Materials, Models, VertexData},
    PackedTextures,
};

// TODO: How much code can be shared with modern switch formats?
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct MxmdV40 {
    // TODO: This type is different for legacy.
    /// A collection of [Model](super::legacy::Model) and associated data.
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
