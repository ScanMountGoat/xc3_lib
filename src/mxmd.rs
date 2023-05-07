use std::io::SeekFrom;

use crate::parse_array;
use binrw::{args, binread, BinRead, BinResult, FilePtr32, NullString, VecArgs};

/// .wimdo files
#[binread]
#[derive(Debug)]
#[br(magic(b"DMXM"))]
pub struct Mxmd {
    version: u32,
    mesh_offset: u32,
    #[br(parse_with = FilePtr32::parse)]
    materials: Materials,
    unk1: u32, // points after the texture names?
    unk2: u32,
    unk3: u32,
    unk4: u32,
    unk5: u32,
    unk6: u32, // points after the material names
}

// TODO: find a way to derive binread.
#[derive(Debug)]
pub struct Materials {
    materials: Vec<Material>,
}

// TODO: make this generic?
impl BinRead for Materials {
    type Args<'a> = ();

    fn read_options<R: std::io::Read + std::io::Seek>(
        reader: &mut R,
        endian: binrw::Endian,
        args: Self::Args<'_>,
    ) -> BinResult<Self> {
        let base_offset = reader.stream_position()?;

        let offset = u32::read_options(reader, endian, ())?;
        let count = u32::read_options(reader, endian, ())?;
        let saved_pos = reader.stream_position()?;

        reader.seek(SeekFrom::Start(base_offset + offset as u64))?;
        let materials = <Vec<Material>>::read_options(
            reader,
            endian,
            VecArgs {
                count: count as usize,
                inner: args! { base_offset },
            },
        )?;
        reader.seek(SeekFrom::Start(saved_pos))?;

        Ok(Self { materials })
    }
}

/// 116 bytes?
#[binread]
#[derive(Debug)]
#[br(import { base_offset: u64 })]
pub struct Material {
    #[br(parse_with = parse_string_ptr, args(base_offset))]
    name: String,

    unk1: u16,
    unk2: u16,
    unk3: u16,
    unk4: u16,

    unks1: [f32; 5],

    #[br(parse_with = parse_relative_array, args(base_offset))]
    textures: Vec<Texture>,

    unks: [u32; 19],
}

#[binread]
#[derive(Debug)]
pub struct Texture {
    texture_index: u16,
    unk1: u16,
    unk2: u16,
    unk3: u16,
}

// TODO: type for this shared with hpcs?
fn parse_string_ptr<R: std::io::Read + std::io::Seek>(
    reader: &mut R,
    endian: binrw::Endian,
    args: (u64,),
) -> BinResult<String> {
    let offset = u32::read_options(reader, endian, ())?;
    let saved_pos = reader.stream_position()?;

    reader.seek(SeekFrom::Start(args.0 + offset as u64))?;
    let value = NullString::read_options(reader, endian, ())?;
    reader.seek(SeekFrom::Start(saved_pos))?;

    Ok(value.to_string())
}

// TODO: Make a type for this?
// TODO: Add inner args?
fn parse_relative_array<R, T>(
    reader: &mut R,
    endian: binrw::Endian,
    args: (u64,),
) -> BinResult<Vec<T>>
where
    R: std::io::Read + std::io::Seek,
    for<'a> T: BinRead<Args<'a> = ()> + 'static,
{
    let base_offset = args.0;

    let relative_offset = u32::read_options(reader, endian, ())?;
    let count = u32::read_options(reader, endian, ())?;
    let saved_pos = reader.stream_position()?;

    reader.seek(SeekFrom::Start(base_offset + relative_offset as u64))?;
    let values = <Vec<T>>::read_options(
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
