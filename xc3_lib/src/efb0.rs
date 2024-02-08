use crate::{
    msrd::TextureResource, parse_count_offset, parse_offset_count, parse_opt_ptr32, parse_ptr32,
    parse_string_ptr32, spch::Spch, vertex::VertexData,
};
use bilge::prelude::*;
use binrw::{args, binread, BinRead};

/// .wiefb effect files for Xenoblade 2.
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, PartialEq, Clone)]
#[br(magic(b"efb0"))]
pub struct Efb0 {
    version: (u16, u16),
    // TODO: embedded mxmd, mibl, hcps?
}
