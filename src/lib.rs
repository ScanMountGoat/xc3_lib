// TODO: consistent naming for magics/extensions?
// mibl instead of lbim?
// TODO: Is the pointer placement algorithm similar enough to SSBH?
// TODO: naming for wismt vertex data?

use std::io::SeekFrom;

use binrw::{BinRead, BinResult, NullString, VecArgs};

pub mod dds;
pub mod drsm;
pub mod hpcs;
pub mod lbim;
pub mod model;
pub mod mxmd;
pub mod sar;

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
