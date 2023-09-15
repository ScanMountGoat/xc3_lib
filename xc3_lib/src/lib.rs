//! A library for reading and writing rendering related file formats.
//!
//! # Game Versions
//! Xenoblade 1 DE, Xenoblade 2, and Xenoblade 3 are supported
//! with Xenoblade 3 receiving the most testing.
//! Struct documentation contains the corresponding
//! type from Xenoblade 2 binary symbols where appropriate.
use std::{
    error::Error,
    io::{BufWriter, Cursor, Read, Seek, SeekFrom, Write},
    path::Path,
};

use crate::write::Xc3WriteFull;
use binrw::{
    file_ptr::FilePtrArgs, BinRead, BinReaderExt, BinResult, BinWrite, FilePtr64, NullString,
    VecArgs,
};
use log::trace;

pub mod apmd;
pub mod bc;
pub mod dds;
pub mod dhal;
pub mod ltpc;
pub mod map;
pub mod mibl;
pub mod msmd;
pub mod msrd;
pub mod mxmd;
pub mod sar1;
pub mod spch;
pub mod vertex;
mod write;
pub mod xbc1;

const PAGE_SIZE: u64 = 4096;

fn parse_offset_count<T, R, Args>(
    reader: &mut R,
    endian: binrw::Endian,
    args: FilePtrArgs<Args>,
) -> BinResult<Vec<T>>
where
    for<'a> T: BinRead<Args<'a> = Args> + 'static,
    R: std::io::Read + std::io::Seek,
    Args: Clone,
{
    let offset = u32::read_options(reader, endian, ())?;
    let count = u32::read_options(reader, endian, ())?;
    parse_vec(reader, endian, args, offset as u64, count as usize)
}

fn parse_count_offset<T, R, Args>(
    reader: &mut R,
    endian: binrw::Endian,
    args: FilePtrArgs<Args>,
) -> BinResult<Vec<T>>
where
    for<'a> T: BinRead<Args<'a> = Args> + 'static,
    R: std::io::Read + std::io::Seek,
    Args: Clone,
{
    let count = u32::read_options(reader, endian, ())?;
    let offset = u32::read_options(reader, endian, ())?;
    parse_vec(reader, endian, args, offset as u64, count as usize)
}

fn parse_vec<T, R, Args>(
    reader: &mut R,
    endian: binrw::Endian,
    args: FilePtrArgs<Args>,
    offset: u64,
    count: usize,
) -> BinResult<Vec<T>>
where
    for<'a> T: BinRead<Args<'a> = Args> + 'static,
    R: std::io::Read + std::io::Seek,
    Args: Clone,
{
    let saved_pos = reader.stream_position()?;

    reader.seek(SeekFrom::Start(offset + args.offset))?;
    trace!(
        "{:?}: {:?}",
        std::any::type_name::<Vec<T>>(),
        reader.stream_position().unwrap()
    );

    let values = Vec::<T>::read_options(
        reader,
        endian,
        VecArgs {
            count,
            inner: args.inner,
        },
    )?;

    reader.seek(SeekFrom::Start(saved_pos))?;

    Ok(values)
}

fn parse_string_ptr32<R: Read + Seek>(
    reader: &mut R,
    endian: binrw::Endian,
    args: FilePtrArgs<()>,
) -> BinResult<String> {
    let value: NullString = parse_ptr32(reader, endian, args)?;
    Ok(value.to_string())
}

fn parse_string_ptr64<R: Read + Seek>(
    reader: &mut R,
    endian: binrw::Endian,
    args: FilePtrArgs<()>,
) -> BinResult<String> {
    // TODO: Create parse_ptr64 for offset logging.
    let value: NullString = FilePtr64::parse(reader, endian, args)?;
    Ok(value.to_string())
}

fn parse_ptr32<T, R, Args>(
    reader: &mut R,
    endian: binrw::Endian,
    args: FilePtrArgs<Args>,
) -> BinResult<T>
where
    for<'a> T: BinRead<Args<'a> = Args> + 'static,
    R: std::io::Read + std::io::Seek,
    Args: Clone,
{
    // Read a value pointed to by a relative offset.
    let offset = u32::read_options(reader, endian, ())?;
    let saved_pos = reader.stream_position()?;

    reader.seek(SeekFrom::Start(offset as u64 + args.offset))?;
    trace!(
        "{}: {:?}",
        std::any::type_name::<T>(),
        reader.stream_position().unwrap()
    );
    let value = T::read_options(reader, endian, args.inner)?;
    reader.seek(SeekFrom::Start(saved_pos))?;

    Ok(value)
}

fn parse_opt_ptr32<T, R, Args>(
    reader: &mut R,
    endian: binrw::Endian,
    args: FilePtrArgs<Args>,
) -> BinResult<Option<T>>
where
    for<'a> T: BinRead<Args<'a> = Args> + 'static,
    R: std::io::Read + std::io::Seek,
    Args: Clone,
{
    // Read a value pointed to by a nullable relative offset.
    let offset = u32::read_options(reader, endian, ())?;
    if offset > 0 {
        let saved_pos = reader.stream_position()?;
        reader.seek(SeekFrom::Start(offset as u64 + args.offset))?;
        trace!(
            "{:?}: {:?}",
            std::any::type_name::<T>(),
            reader.stream_position().unwrap()
        );
        let value = T::read_options(reader, endian, args.inner)?;
        reader.seek(SeekFrom::Start(saved_pos))?;

        Ok(Some(value))
    } else {
        Ok(None)
    }
}

// TODO: Dedicated error types?
macro_rules! file_write_impl {
    ($($type_name:path),*) => {
        $(
            impl $type_name {
                pub fn write<W: Write + Seek>(&self, writer: &mut W) -> Result<(), Box<dyn Error>> {
                    self.write_le(writer).map_err(Into::into)
                }

                pub fn write_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn Error>> {
                    let mut writer = BufWriter::new(std::fs::File::create(path)?);
                    self.write_le(&mut writer).map_err(Into::into)
                }
            }
        )*
    };
}

file_write_impl!(mibl::Mibl, xbc1::Xbc1);

macro_rules! file_write_full_impl {
    ($($type_name:path),*) => {
        $(
            impl $type_name {
                pub fn write<W: Write + Seek>(&self, writer: &mut W) -> Result<(), Box<dyn Error>> {
                    self.write_full(writer, 0, &mut 0).map_err(Into::into)
                }

                pub fn write_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn Error>> {
                    let mut writer = BufWriter::new(std::fs::File::create(path)?);
                    self.write(&mut writer)
                }
            }
        )*
    };
}

file_write_full_impl!(
    vertex::VertexData,
    ltpc::Ltpc,
    msrd::Msrd,
    apmd::Apmd,
    spch::Spch
);

// TODO: Dedicated error types?
macro_rules! file_read_impl {
    ($($type_name:path),*) => {
        $(
            impl $type_name {
                pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, Box<dyn Error>> {
                    reader.read_le().map_err(Into::into)
                }

                pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn Error>> {
                    let mut reader = Cursor::new(std::fs::read(path)?);
                    reader.read_le().map_err(Into::into)
                }

                pub fn from_bytes(bytes: &[u8]) -> Result<Self, Box<dyn Error>> {
                    Self::read(&mut Cursor::new(bytes))
                }
            }
        )*
    };
}

file_read_impl!(
    mibl::Mibl,
    xbc1::Xbc1,
    msmd::Msmd,
    msrd::Msrd,
    mxmd::Mxmd,
    sar1::Sar1,
    spch::Spch,
    vertex::VertexData,
    dhal::Dhal,
    ltpc::Ltpc,
    apmd::Apmd
);
