//! Legacy types for Xenoblade Chronicles X DE.
use binrw::BinRead;
use xc3_write::{Xc3Write, Xc3WriteOffsets};

use crate::{msrd::Streaming, parse_opt_ptr32, parse_ptr32, spco::Spco};

use super::{
    PackedTextures,
    legacy::{Materials, Models, Unk1, VertexData},
};

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, PartialEq, Clone)]
pub struct MxmdV40 {
    /// A collection of [Model](super::legacy::Model) and associated data.
    #[br(parse_with = parse_ptr32)]
    #[xc3(offset(u32), align(16))]
    pub models: Models,

    #[br(parse_with = parse_ptr32)]
    #[xc3(offset(u32), align(16))]
    pub materials: Materials,

    #[br(parse_with = parse_opt_ptr32)]
    #[xc3(offset(u32), align(16))]
    pub unk1: Option<Unk1>,

    #[br(parse_with = parse_opt_ptr32)]
    #[xc3(offset(u32), align(4096))]
    pub vertex_data: Option<VertexData>,

    #[br(parse_with = parse_opt_ptr32)]
    #[xc3(offset(u32), align(4096))]
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

impl Xc3WriteOffsets for MxmdV40Offsets<'_> {
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
        self.models
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.materials
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.vertex_data
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.shaders
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.packed_textures
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.streaming
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.unk1
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        // TODO: sometimes aligned to 16 like with msrd?
        Ok(())
    }
}
