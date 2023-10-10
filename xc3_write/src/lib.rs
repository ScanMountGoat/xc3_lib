use std::io::Seek;
use std::io::SeekFrom;
use std::io::Write;
use std::marker::PhantomData;

use binrw::{BinResult, BinWrite};

/// The write pass that writes fields and placeholder offsets.
pub trait Xc3Write {
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

/// The layout pass that updates and writes data for all fields in [Xc3Write::Offsets] recursively.
pub trait Xc3WriteOffsets {
    fn write_offsets<W: Write + Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
    ) -> BinResult<()>;
}

// A complete write uses a two pass approach to handle offsets.
// We can fully write any type that can fully write its offset values.
// This includes types with an offset type of () like primitive types.
pub fn write_full<'a, T, W>(
    value: &'a T,
    writer: &mut W,
    base_offset: u64,
    data_ptr: &mut u64,
) -> BinResult<()>
where
    W: Write + Seek,
    T: Xc3Write + 'static,
    T::Offsets<'a>: Xc3WriteOffsets,
{
    // TODO: Incorporate the base offset from offsets using option?
    // Ensure all items are written before their pointed to data.
    let offsets = value.xc3_write(writer, data_ptr)?;
    offsets.write_offsets(writer, base_offset, data_ptr)?;
    Ok(())
}

// Support importing both the trait and derive macro at once.
pub use xc3_write_derive::Xc3Write;
pub use xc3_write_derive::Xc3WriteOffsets;

pub struct FieldPosition<'a, T> {
    /// The position in the file for the field.
    pub position: u64,
    /// The field value.
    pub data: &'a T,
}

impl<'a, T> FieldPosition<'a, T> {
    pub fn new(position: u64, data: &'a T) -> Self {
        Self { position, data }
    }
}

pub struct Offset<'a, P, T> {
    /// The position in the file for the offset field.
    pub position: u64,
    /// The data pointed to by the offset.
    pub data: &'a T,
    /// Alignment override applied at the field level.
    pub field_alignment: Option<u64>,
    phantom: PhantomData<P>,
}

impl<'a, P, T: Xc3Write> std::fmt::Debug for Offset<'a, P, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Don't print the actual data to avoid excessive output.
        f.debug_struct("Offset")
            .field("position", &self.position)
            .field("data", &std::any::type_name::<T>())
            .finish()
    }
}

impl<'a, P, T> Offset<'a, P, T> {
    pub fn new(position: u64, data: &'a T, field_alignment: Option<u64>) -> Self {
        Self {
            position,
            data,
            field_alignment,
            phantom: PhantomData,
        }
    }

    fn set_offset_seek<W>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
        type_alignment: u64,
    ) -> Result<(), binrw::Error>
    where
        W: Write + Seek,
        // TODO: Create a trait for this?
        P: TryFrom<u64>,
        <P as TryFrom<u64>>::Error: std::fmt::Debug,
        for<'b> P: BinWrite<Args<'b> = ()>,
    {
        // Account for the type or field alignment.
        let alignment = self.field_alignment.unwrap_or(type_alignment);
        *data_ptr = round_up(*data_ptr, alignment);

        // Update the offset value.
        writer.seek(SeekFrom::Start(self.position))?;
        let offset = P::try_from(*data_ptr - base_offset).unwrap();
        offset.write_le(writer)?;

        // Seek to the data position.
        writer.seek(SeekFrom::Start(*data_ptr))?;
        Ok(())
    }
}

impl<'a, P, T> Offset<'a, P, T>
where
    T: Xc3Write,
    P: TryFrom<u64>,
    <P as TryFrom<u64>>::Error: std::fmt::Debug,
    for<'b> P: BinWrite<Args<'b> = ()>,
{
    pub fn write_offset<W: Write + Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
    ) -> BinResult<T::Offsets<'_>> {
        // TODO: How to avoid setting this for empty vecs?
        self.set_offset_seek(writer, base_offset, data_ptr, T::ALIGNMENT)?;
        let offsets = self.data.xc3_write(writer, data_ptr)?;
        Ok(offsets)
    }
}

// This doesn't need specialization because Option does not impl Xc3Write.
impl<'a, P, T> Offset<'a, P, Option<T>>
where
    T: Xc3Write,
    P: TryFrom<u64>,
    <P as TryFrom<u64>>::Error: std::fmt::Debug,
    for<'b> P: BinWrite<Args<'b> = ()>,
{
    pub fn write_offset<W: Write + Seek>(
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

impl<'a, P, T> Offset<'a, P, T>
where
    T: Xc3Write + 'static,
    T::Offsets<'a>: Xc3WriteOffsets,
    P: TryFrom<u64>,
    <P as TryFrom<u64>>::Error: std::fmt::Debug,
    for<'b> P: BinWrite<Args<'b> = ()>,
{
    pub fn write_full<W: Write + Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
    ) -> BinResult<()> {
        // TODO: How to avoid setting this for empty vecs?
        self.set_offset_seek(writer, base_offset, data_ptr, T::ALIGNMENT)?;
        write_full(self.data, writer, base_offset, data_ptr)?;
        Ok(())
    }
}

// This doesn't need specialization because Option does not impl Xc3WriteOffsets.
impl<'a, P, T> Offset<'a, P, Option<T>>
where
    T: Xc3Write + 'static,
    T::Offsets<'a>: Xc3WriteOffsets,
    P: TryFrom<u64>,
    <P as TryFrom<u64>>::Error: std::fmt::Debug,
    for<'b> P: BinWrite<Args<'b> = ()>,
{
    pub fn write_full<W: Write + Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
    ) -> BinResult<()> {
        // Only update the offset if there is data.
        if let Some(data) = self.data {
            self.set_offset_seek(writer, base_offset, data_ptr, T::ALIGNMENT)?;
            write_full(data, writer, base_offset, data_ptr)?;
        }
        Ok(())
    }
}

// TODO: This won't work as a blanket impl because of Vec?
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
                ) -> binrw::BinResult<Self::Offsets<'_>> {
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

xc3_write_binwrite_impl!(
    i8,
    i16,
    i32,
    i64,
    u8,
    u16,
    u32,
    u64,
    f32,
    (u16, u16),
    (u8, u8, u16)
);

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
            writer.write_all(bytes)?;
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

impl<T> Xc3WriteOffsets for VecOffsets<T>
where
    T: Xc3WriteOffsets,
{
    fn write_offsets<W: Write + Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
    ) -> BinResult<()> {
        for item in &self.0 {
            item.write_offsets(writer, base_offset, data_ptr)?;
        }
        Ok(())
    }
}

impl Xc3Write for () {
    type Offsets<'a> = ();

    fn xc3_write<W: Write + Seek>(
        &self,
        _writer: &mut W,
        _data_ptr: &mut u64,
    ) -> BinResult<Self::Offsets<'_>> {
        Ok(())
    }

    const ALIGNMENT: u64 = 1;
}

impl Xc3WriteOffsets for () {
    fn write_offsets<W: Write + Seek>(
        &self,
        _writer: &mut W,
        _base_offset: u64,
        _data_ptr: &mut u64,
    ) -> BinResult<()> {
        Ok(())
    }
}

pub const fn round_up(x: u64, n: u64) -> u64 {
    ((x + n - 1) / n) * n
}
