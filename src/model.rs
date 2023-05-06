use std::io::SeekFrom;

use binrw::{binread, BinRead, BinResult, VecArgs};

// wismt model data
// are these buffers referenced by wimdo?
// TODO: what to call this?
#[binread]
#[derive(Debug)]
pub struct ModelData {
    #[br(parse_with = parse_array)]
    vertex_buffers: Vec<VertexBuffer>,

    #[br(parse_with = parse_array)]
    index_buffers: Vec<IndexBuffer>,

    // padding?
    unk0: u32,
    unk1: u32,
    unk2: u32,

    // 144 bytes of data?
    unk_offset0: u32,

    // 332 bytes of data?
    unk_offset1: u32,
    unk4: u32,

    morph_offset: u32,

    unk5: u32,

    data_base_offset: u32,

    unk6: u32,

    weight_offset: u32,

    unk7: u32,
    // padding?
}

// vertex buffer?
#[binread]
#[derive(Debug)]
pub struct VertexBuffer {
    data_offset: u32,
    vertex_count: u32,
    vertex_size: u32,

    #[br(parse_with = parse_array)]
    attributes: Vec<VertexAttribute>,

    // padding?
    unk1: u32,
    unk2: u32,
    unk3: u32,
}

#[binread]
#[derive(Debug)]
pub struct VertexAttribute {
    data_type: DataType,
    data_size: u16,
}

#[binread]
#[derive(Debug)]
#[br(repr(u16))]
pub enum DataType {
    Position = 0,    // f32x3
    WeightIndex = 3, // u32
    Uv1 = 5,         // f32x2
    Uv2 = 6,
    Uv3 = 7,
    VertexColor = 17, // u8x4
    Normal = 28,      // i8x4?
    Unk29 = 29,       // i8x4? tangent?
    WeightShort = 41,
    BoneId2 = 42,
}

#[binread]
#[derive(Debug)]
pub struct IndexBuffer {
    // is this data u16?
    data_offset: u32,
    data_length: u32,
    // padding?
    unk1: u32,
    unk2: u32,
    unk3: u32,
}

// TODO: Make a type for this and just use temp to derive it?
fn parse_array<T, R>(reader: &mut R, endian: binrw::Endian, _args: ()) -> BinResult<Vec<T>>
where
    for<'a> T: BinRead<Args<'a> = ()> + 'static,
    R: std::io::Read + std::io::Seek,
{
    let offset = u32::read_options(reader, endian, ())?;
    let count = u32::read_options(reader, endian, ())?;

    let saved_pos = reader.stream_position()?;

    reader.seek(SeekFrom::Start(offset as u64))?;

    let values = Vec::<T>::read_options(
        reader,
        endian,
        VecArgs {
            count: count as usize,
            inner: (),
        },
    )?;

    reader.seek(SeekFrom::Start(saved_pos))?;

    Ok(values)
}

// TODO: functions for accessing data.
