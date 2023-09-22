use std::io::Seek;
use std::io::SeekFrom;
use std::io::Write;

use binrw::{BinResult, BinWrite};

/// The initial write and dummy offset pass.
pub(crate) trait Xc3Write {
    type Offsets<'a>
    where
        Self: 'a;

    fn xc3_write<W: Write + Seek>(
        &self,
        writer: &mut W,
        data_ptr: &mut u64,
    ) -> BinResult<Self::Offsets<'_>>;

    const ALIGNMENT: u64 = 4;
}

// TODO: Come up with a better name.
/// The full write operation that updates all offsets.
/// This should write recursively, so it doesn't return anything.
pub(crate) trait Xc3WriteFull {
    fn write_full<W: Write + Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
    ) -> BinResult<()>;
}

// Support importing both the trait and derive macro at once.
pub(crate) use xc3_lib_derive::Xc3Write;
pub(crate) use xc3_lib_derive::Xc3WriteFull;

pub(crate) struct Offset<'a, T> {
    /// The position in the file for the offset field.
    pub position: u64,
    /// The data pointed to by the offset.
    pub data: &'a T,
    /// Alignment override applied at the field level.
    pub field_alignment: Option<u64>,
}

impl<'a, T: Xc3Write> std::fmt::Debug for Offset<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Don't print the actual data to avoid excessive output.
        f.debug_struct("Offset")
            .field("position", &self.position)
            .field("data", &std::any::type_name::<T>())
            .finish()
    }
}

impl<'a, T> Offset<'a, T> {
    pub fn new(position: u64, data: &'a T, field_alignment: Option<u64>) -> Self {
        Self {
            position,
            data,
            field_alignment,
        }
    }

    fn set_offset_seek<W: Write + Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
        type_alignment: u64,
    ) -> Result<(), binrw::Error> {
        // Account for the type or field alignment.
        let alignment = self.field_alignment.unwrap_or(type_alignment);
        *data_ptr = round_up(*data_ptr, alignment);

        // Update the offset value.
        writer.seek(SeekFrom::Start(self.position))?;
        ((*data_ptr - base_offset) as u32).write_le(writer)?;

        // Seek to the data position.
        writer.seek(SeekFrom::Start(*data_ptr))?;
        Ok(())
    }
}

impl<'a, T: Xc3Write> Offset<'a, T> {
    // TODO: make the data ptr u32?
    // TODO: Specify an alignment using another trait?
    pub(crate) fn write_offset<W: Write + Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
    ) -> BinResult<T::Offsets<'_>> {
        self.set_offset_seek(writer, base_offset, data_ptr, T::ALIGNMENT)?;
        let offsets = self.data.xc3_write(writer, data_ptr)?;
        Ok(offsets)
    }
}

// This doesn't need specialization because Option does not impl Xc3Write.
impl<'a, T: Xc3Write> Offset<'a, Option<T>> {
    pub(crate) fn write_offset<W: Write + Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
    ) -> BinResult<Option<T::Offsets<'_>>> {
        // Only update the offset if there is data.
        if let Some(data) = self.data {
            self.set_offset_seek(writer, base_offset, data_ptr, T::ALIGNMENT)?;
            let offsets = data.xc3_write(writer, data_ptr)?;
            Ok(Some(offsets))
        } else {
            Ok(None)
        }
    }
}

impl<'a, T> Xc3WriteFull for Offset<'a, T>
where
    T: Xc3Write + Xc3WriteFull,
{
    fn write_full<W: Write + Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
    ) -> BinResult<()> {
        self.set_offset_seek(writer, base_offset, data_ptr, T::ALIGNMENT)?;
        self.data.write_full(writer, base_offset, data_ptr)?;
        Ok(())
    }
}

// This doesn't need specialization because Option does not impl Xc3WriteFull.
impl<'a, T> Xc3WriteFull for Offset<'a, Option<T>>
where
    T: Xc3Write + Xc3WriteFull,
{
    fn write_full<W: Write + Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
    ) -> BinResult<()> {
        // Only update the offset if there is data.
        if let Some(data) = self.data {
            self.set_offset_seek(writer, base_offset, data_ptr, T::ALIGNMENT)?;
            data.write_full(writer, base_offset, data_ptr)?;
        }
        Ok(())
    }
}

// TODO: This won't work as a blanket impl because of Vec?
macro_rules! xc3_write_binwrite_impl {
    ($($ty:ty),*) => {
        $(
            impl Xc3Write for $ty {
                // This also implements Xc3WriteFull since () is Xc3WriteFull.
                type Offsets<'a> = ();

                fn xc3_write<W: std::io::Write + std::io::Seek>(
                    &self,
                    writer: &mut W,
                    data_ptr: &mut u64,
                ) -> binrw::BinResult<Self::Offsets<'_>> {
                    self.write_le(writer)?;
                    *data_ptr = (*data_ptr).max(writer.stream_position()?);
                    Ok(())
                }

                const ALIGNMENT: u64 = std::mem::size_of::<$ty>() as u64;
            }
        )*

    };
}

pub(crate) use xc3_write_binwrite_impl;

// TODO: Add alignment as a parameter.
xc3_write_binwrite_impl!(u8, u16, u32, u64, f32, (u16, u16), (u8, u8, u16));

// TODO: Support types with offsets?
impl<const N: usize, T> Xc3Write for [T; N]
where
    T: Xc3Write + 'static,
{
    type Offsets<'a> = ();

    fn xc3_write<W: Write + Seek>(
        &self,
        writer: &mut W,
        data_ptr: &mut u64,
    ) -> BinResult<Self::Offsets<'_>> {
        for value in self {
            value.xc3_write(writer, data_ptr)?;
        }
        Ok(())
    }
}

impl Xc3Write for String {
    type Offsets<'a> = ();

    fn xc3_write<W: Write + Seek>(
        &self,
        writer: &mut W,
        data_ptr: &mut u64,
    ) -> BinResult<Self::Offsets<'_>> {
        self.as_bytes().write_le(writer)?;
        0u8.write_le(writer)?;
        *data_ptr = (*data_ptr).max(writer.stream_position()?);
        Ok(())
    }

    const ALIGNMENT: u64 = 1;
}

// Create a new type to differentiate vec and a vec of offsets.
// This allows using a blanket implementation for write full.
pub struct VecOffsets<T>(pub Vec<T>);

impl<T> Xc3Write for Vec<T>
where
    T: Xc3Write + 'static,
{
    type Offsets<'a> = VecOffsets<T::Offsets<'a>>;

    fn xc3_write<W: Write + Seek>(
        &self,
        writer: &mut W,
        data_ptr: &mut u64,
    ) -> BinResult<Self::Offsets<'_>> {
        // TODO: Find a less hacky way to specialize Vec<u8>.
        let offsets = if let Some(bytes) = <dyn core::any::Any>::downcast_ref::<Vec<u8>>(self) {
            // Avoiding writing buffers byte by byte to drastically improve performance.
            writer.write_all(&bytes)?;
            Vec::new()
        } else {
            self.iter()
                .map(|v| v.xc3_write(writer, data_ptr))
                .collect::<Result<Vec<_>, _>>()?
        };
        *data_ptr = (*data_ptr).max(writer.stream_position()?);
        Ok(VecOffsets(offsets))
    }
}

// TODO: Incorporate the base offset from offsets using option?
// We can fully write any type that can fully write its offset values.
// This includes types with an offset type of () like primitive types.
impl<T> Xc3WriteFull for T
where
    T: Xc3Write + 'static,
    for<'a> T::Offsets<'a>: Xc3WriteFull,
{
    fn write_full<W: Write + Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
    ) -> BinResult<()> {
        // Ensure all items are written before their pointed to data.
        let offsets = self.xc3_write(writer, data_ptr)?;
        offsets.write_full(writer, base_offset, data_ptr)?;
        Ok(())
    }
}

impl<T> Xc3WriteFull for VecOffsets<T>
where
    T: Xc3WriteFull,
{
    fn write_full<W: Write + Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
    ) -> BinResult<()> {
        for item in &self.0 {
            item.write_full(writer, base_offset, data_ptr)?;
        }
        Ok(())
    }
}

impl Xc3WriteFull for () {
    fn write_full<W: Write + Seek>(
        &self,
        _writer: &mut W,
        _base_offset: u64,
        _data_ptr: &mut u64,
    ) -> BinResult<()> {
        Ok(())
    }
}

pub(crate) const fn round_up(x: u64, n: u64) -> u64 {
    ((x + n - 1) / n) * n
}
