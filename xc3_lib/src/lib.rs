//! A library for reading and writing rendering related file formats.
//!
//! Xenoblade 1 DE, Xenoblade 2, and Xenoblade 3 are supported
//! with Xenoblade 3 receiving the most testing.
//! Struct documentation contains the corresponding
//! type from Xenoblade 2 binary symbols where appropriate.
//!
//! # Getting Started
//! Each format has its own module based on the name of the type representing the root of the file.
//! Only these top level types support reading and writing from files.
//!
//! ```rust no_run
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Read from disk.
//! let mxmd = xc3_lib::mxmd::Mxmd::from_file("ch01011013.wimdo")?;
//! println!("{mxmd:#?}");
//!
//! // Save to disk after making any changes.
//! mxmd.write_to_file("out.wimdo")?;
//! # Ok(())
//! # }
//! ```
//!
//! # Design
//! xc3_lib provides safe, efficient, and robust reading and writing code for binary file formats.
//! Each file format consists of a set of Rust types representing the structures in the binary file.
//! xc3_lib uses derive macros to generate reading and writing code from the type and its attribute annotations.
//! This avoids the need to separately document the format and reading and writing logic.
//!
//! Each type is intended to be as specific as possible while still being able to generate a binary identical output.
//! Enums are used instead of raw integers to reject unknown variants, for example.
//! Each file is fully parsed and invalid input is not sanitized in any way.
//! xc3_lib can validate the contents of a binary file by parsing it but cannot validate
//! higher level constraints like entry indices being in range.
//! These checks are performed by higher level libraries like xc3_model or xc3_wgpu.
//!
//! Operations that would be impossible to reverse accurately like compression or byte buffers must be decoded and encoded in
//! a separate step. This allows identical outputs when no modifications are needed to binary buffers.
use std::{
    error::Error,
    io::{BufWriter, Cursor, Read, Seek, SeekFrom, Write},
    marker::PhantomData,
    path::Path,
};

use binrw::{
    file_ptr::FilePtrArgs, BinRead, BinReaderExt, BinResult, BinWrite, NullString, VecArgs,
};
use log::trace;
use xc3_write::write_full;

pub mod apmd;
pub mod bc;
pub mod dds;
pub mod dhal;
pub mod error;
pub mod eva;
pub mod hash;
pub mod ltpc;
pub mod map;
pub mod mibl;
pub mod msmd;
pub mod msrd;
pub mod mxmd;
pub mod sar1;
pub mod spch;
pub mod vertex;
pub mod xbc1;

struct Ptr<P> {
    phantom: PhantomData<P>,
}

impl<P> Ptr<P>
where
    P: Into<u64>,
    for<'a> P: BinRead<Args<'a> = ()>,
{
    fn parse<T, R, Args>(
        reader: &mut R,
        endian: binrw::Endian,
        args: FilePtrArgs<Args>,
    ) -> BinResult<T>
    where
        for<'a> T: BinRead<Args<'a> = Args> + 'static,
        R: std::io::Read + std::io::Seek,
        Args: Clone,
    {
        // Reading data at the current position produces confusing errors.
        // Fail early since an offset of 0 always seems to indicate no value.
        let pos = reader.stream_position()?;
        Self::parse_opt(reader, endian, args).and_then(|value| {
            value.ok_or(binrw::Error::AssertFail {
                pos,
                message: "unexpected null offset".to_string(),
            })
        })
    }

    fn parse_opt<T, R, Args>(
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
        let offset = P::read_options(reader, endian, ())?.into();
        if offset > 0 {
            let value = parse_ptr(offset, reader, endian, args)?;
            Ok(Some(value))
        } else {
            Ok(None)
        }
    }
}

fn parse_offset32_count32<T, R, Args>(
    reader: &mut R,
    endian: binrw::Endian,
    args: FilePtrArgs<Args>,
) -> BinResult<Vec<T>>
where
    for<'a> T: BinRead<Args<'a> = Args> + 'static,
    R: std::io::Read + std::io::Seek,
    Args: Clone,
{
    let pos = reader.stream_position()?;
    let offset = u32::read_options(reader, endian, ())?;
    let count = u32::read_options(reader, endian, ())?;

    if offset == 0 && count != 0 {
        return Err(binrw::Error::AssertFail {
            pos,
            message: format!("unexpected null offset for count {count}"),
        });
    }

    parse_vec(reader, endian, args, offset as u64, count as usize)
}

fn parse_count16_offset32<T, R, Args>(
    reader: &mut R,
    endian: binrw::Endian,
    args: FilePtrArgs<Args>,
) -> BinResult<Vec<T>>
where
    for<'a> T: BinRead<Args<'a> = Args> + 'static,
    R: std::io::Read + std::io::Seek,
    Args: Clone,
{
    let count = u16::read_options(reader, endian, ())?;
    let pos = reader.stream_position()?;
    let offset = u32::read_options(reader, endian, ())?;

    if offset == 0 && count != 0 {
        return Err(binrw::Error::AssertFail {
            pos,
            message: format!("unexpected null offset for count {count}"),
        });
    }

    parse_vec(reader, endian, args, offset as u64, count as usize)
}

fn parse_count32_offset32<T, R, Args>(
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
    let pos = reader.stream_position()?;
    let offset = u32::read_options(reader, endian, ())?;

    if offset == 0 && count != 0 {
        return Err(binrw::Error::AssertFail {
            pos,
            message: format!("unexpected null offset for count {count}"),
        });
    }

    parse_vec(reader, endian, args, offset as u64, count as usize)
}

fn parse_offset64_count32<T, R, Args>(
    reader: &mut R,
    endian: binrw::Endian,
    args: FilePtrArgs<Args>,
) -> BinResult<Vec<T>>
where
    for<'a> T: BinRead<Args<'a> = Args> + 'static,
    R: std::io::Read + std::io::Seek,
    Args: Clone,
{
    let pos = reader.stream_position()?;
    let offset = u64::read_options(reader, endian, ())?;
    let count = u32::read_options(reader, endian, ())?;

    if offset == 0 && count != 0 {
        return Err(binrw::Error::AssertFail {
            pos,
            message: format!("unexpected null offset for count {count}"),
        });
    }

    parse_vec(reader, endian, args, offset, count as usize)
}

fn parse_ptr<T, R, Args>(
    offset: u64,
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
    let saved_pos = reader.stream_position()?;

    reader.seek(SeekFrom::Start(offset + args.offset))?;
    trace!(
        "{}: {:?}",
        std::any::type_name::<T>(),
        reader.stream_position()?
    );
    let value = T::read_options(reader, endian, args.inner)?;
    reader.seek(SeekFrom::Start(saved_pos))?;

    Ok(value)
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
        reader.stream_position()?
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

fn parse_string_opt_ptr32<R: Read + Seek>(
    reader: &mut R,
    endian: binrw::Endian,
    args: FilePtrArgs<()>,
) -> BinResult<Option<String>> {
    let value: Option<NullString> = parse_opt_ptr32(reader, endian, args)?;
    Ok(value.map(|value| value.to_string()))
}

fn parse_string_ptr64<R: Read + Seek>(
    reader: &mut R,
    endian: binrw::Endian,
    args: FilePtrArgs<()>,
) -> BinResult<String> {
    let value: NullString = parse_ptr64(reader, endian, args)?;
    Ok(value.to_string())
}

fn parse_string_opt_ptr64<R: Read + Seek>(
    reader: &mut R,
    endian: binrw::Endian,
    args: FilePtrArgs<()>,
) -> BinResult<Option<String>> {
    let value: Option<NullString> = parse_opt_ptr64(reader, endian, args)?;
    Ok(value.map(|value| value.to_string()))
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
    Ptr::<u32>::parse(reader, endian, args)
}

fn parse_ptr64<T, R, Args>(
    reader: &mut R,
    endian: binrw::Endian,
    args: FilePtrArgs<Args>,
) -> BinResult<T>
where
    for<'a> T: BinRead<Args<'a> = Args> + 'static,
    R: std::io::Read + std::io::Seek,
    Args: Clone,
{
    Ptr::<u64>::parse(reader, endian, args)
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
    Ptr::<u32>::parse_opt(reader, endian, args)
}

fn parse_opt_ptr64<T, R, Args>(
    reader: &mut R,
    endian: binrw::Endian,
    args: FilePtrArgs<Args>,
) -> BinResult<Option<T>>
where
    for<'a> T: BinRead<Args<'a> = Args> + 'static,
    R: std::io::Read + std::io::Seek,
    Args: Clone,
{
    Ptr::<u64>::parse_opt(reader, endian, args)
}

// TODO: Dedicated error types?
macro_rules! file_write_impl {
    ($($type_name:path),*) => {
        $(
            impl $type_name {
                pub fn write<W: Write + Seek>(&self, writer: &mut W) -> Result<(), Box<dyn Error>> {
                    self.write_le(writer).map_err(Into::into)
                }

                /// Write to `path` using a buffered writer for better performance.
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
                    write_full(self, writer, 0, &mut 0).map_err(Into::into)
                }

                /// Write to `path` using a buffered writer for better performance.
                pub fn write_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn Error>> {
                    let mut writer = BufWriter::new(std::fs::File::create(path)?);
                    self.write(&mut writer)
                }
            }
        )*
    };
}

file_write_full_impl!(
    apmd::Apmd,
    ltpc::Ltpc,
    msrd::Msrd,
    mxmd::Mxmd,
    spch::Spch,
    vertex::VertexData,
    sar1::Sar1,
    msmd::Msmd,
    dhal::Dhal,
    bc::Bc,
    eva::Eva
);

// TODO: Dedicated error types?
macro_rules! file_read_impl {
    ($($type_name:path),*) => {
        $(
            impl $type_name {
                pub fn read<R: Read + Seek>(reader: &mut R) -> binrw::BinResult<Self> {
                    reader.read_le().map_err(Into::into)
                }

                /// Read from `path` using a fully buffered reader for performance.
                pub fn from_file<P: AsRef<Path>>(path: P) -> binrw::BinResult<Self> {
                    let mut reader = Cursor::new(std::fs::read(path)?);
                    reader.read_le().map_err(Into::into)
                }

                /// Read from `bytes` using a fully buffered reader for performance.
                pub fn from_bytes<T: AsRef<[u8]>>(bytes: T) -> binrw::BinResult<Self> {
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
    apmd::Apmd,
    bc::Bc,
    eva::Eva
);

#[macro_export]
macro_rules! xc3_write_binwrite_impl {
    ($($ty:ty),*) => {
        $(
            impl Xc3Write for $ty {
                // This also enables write_full since () implements Xc3WriteOffsets.
                type Offsets<'a> = ();

                fn xc3_write<W: std::io::Write + std::io::Seek>(
                    &self,
                    writer: &mut W,
                    data_ptr: &mut u64,
                ) -> xc3_write::Xc3Result<Self::Offsets<'_>> {
                    self.write_le(writer)?;
                    *data_ptr = (*data_ptr).max(writer.stream_position()?);
                    Ok(())
                }

                // TODO: Should this be specified manually?
                const ALIGNMENT: u64 = std::mem::align_of::<$ty>() as u64;
            }
        )*

    };
}
