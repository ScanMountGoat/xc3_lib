//! A binary writing and layout implementation using separate write and layout passes.
use std::error::Error;
use std::io::Seek;
use std::io::SeekFrom;
use std::io::Write;
use std::marker::PhantomData;

// TODO: Create a dedicated error type?
pub type Xc3Result<T> = Result<T, Box<dyn Error>>;

/// The write pass that writes fields and placeholder offsets.
pub trait Xc3Write {
    /// The type storing offset data to be used in [Xc3WriteOffsets].
    type Offsets<'a>
    where
        Self: 'a;

    /// Write all fields and placeholder offsets
    /// and set `data_ptr` to the position after writing.
    /// This should almost always be derived for non primitive types.
    fn xc3_write<W: Write + Seek>(
        &self,
        writer: &mut W,
        data_ptr: &mut u64,
    ) -> Xc3Result<Self::Offsets<'_>>;

    /// The alignment of absolute offsets for this type in bytes.
    const ALIGNMENT: u64 = 4;
}

/// The layout pass that updates and writes data for all fields in [Xc3Write::Offsets] recursively.
pub trait Xc3WriteOffsets {
    /// Update and write pointed to data for all fields in [Xc3Write::Offsets].
    ///
    /// The goal is to call [Offset::write_offset] or [Xc3WriteOffsets::write_offsets]
    /// in increasing order by absolute offset stored in `data_ptr`.
    /// For writing in order by field recursively, simply derive [Xc3WriteOffsets].
    /// Manually implementing this trait allows flexibility for cases like placing strings
    /// for all types at the end of the file.
    fn write_offsets<W: Write + Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
    ) -> Xc3Result<()>;
}

/// A complete write uses a two pass approach to handle offsets.
///
/// We can fully write any type that can fully write its offset values.
/// This includes types with an offset type of () like primitive types.
pub fn write_full<'a, T, W>(
    value: &'a T,
    writer: &mut W,
    base_offset: u64,
    data_ptr: &mut u64,
) -> Xc3Result<()>
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
    /// The byte used for padding or alignment.
    /// This is usually `0u8`.
    pub padding_byte: u8,
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
    pub fn new(position: u64, data: &'a T, field_alignment: Option<u64>, padding_byte: u8) -> Self {
        Self {
            position,
            data,
            field_alignment,
            padding_byte,
            phantom: PhantomData,
        }
    }

    fn set_offset_seek<W>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
        type_alignment: u64,
    ) -> Xc3Result<()>
    where
        W: Write + Seek,
        // TODO: Create a trait for this?
        P: TryFrom<u64> + Xc3Write,
        <P as TryFrom<u64>>::Error: std::fmt::Debug,
    {
        // Account for the type or field alignment.
        let alignment = self.field_alignment.unwrap_or(type_alignment);
        let aligned_data_pr = round_up(*data_ptr, alignment);

        // Update the offset value.
        writer.seek(SeekFrom::Start(self.position))?;
        let offset = P::try_from(aligned_data_pr - base_offset).unwrap();
        offset.xc3_write(writer, data_ptr)?;

        // Seek to the data position.
        // Handle any padding up the desired alignment.
        writer.seek(SeekFrom::Start(*data_ptr))?;
        vec![self.padding_byte; (aligned_data_pr - *data_ptr) as usize]
            .xc3_write(writer, data_ptr)?;

        Ok(())
    }
}

impl<'a, P, T> Offset<'a, P, T>
where
    T: Xc3Write,
    P: TryFrom<u64> + Xc3Write,
    <P as TryFrom<u64>>::Error: std::fmt::Debug,
{
    pub fn write_offset<W: Write + Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
    ) -> Xc3Result<T::Offsets<'_>> {
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
    P: TryFrom<u64> + Xc3Write,
    <P as TryFrom<u64>>::Error: std::fmt::Debug,
{
    pub fn write_offset<W: Write + Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
    ) -> Xc3Result<Option<T::Offsets<'_>>> {
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
    P: TryFrom<u64> + Xc3Write,
    <P as TryFrom<u64>>::Error: std::fmt::Debug,
{
    pub fn write_full<W: Write + Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
    ) -> Xc3Result<()> {
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
    P: TryFrom<u64> + Xc3Write,
    <P as TryFrom<u64>>::Error: std::fmt::Debug,
{
    pub fn write_full<W: Write + Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
    ) -> Xc3Result<()> {
        // Only update the offset if there is data.
        if let Some(data) = self.data {
            self.set_offset_seek(writer, base_offset, data_ptr, T::ALIGNMENT)?;
            write_full(data, writer, base_offset, data_ptr)?;
        }
        Ok(())
    }
}

macro_rules! xc3_write_impl {
    ($($ty:ty),*) => {
        $(
            impl Xc3Write for $ty {
                // This also enables write_full since () implements Xc3WriteOffsets.
                type Offsets<'a> = ();

                fn xc3_write<W: std::io::Write + std::io::Seek>(
                    &self,
                    writer: &mut W,
                    data_ptr: &mut u64,
                ) -> Xc3Result<Self::Offsets<'_>> {
                    writer.write_all(&self.to_le_bytes())?;
                    *data_ptr = (*data_ptr).max(writer.stream_position()?);
                    Ok(())
                }

                // TODO: Should this be specified manually?
                const ALIGNMENT: u64 = std::mem::align_of::<$ty>() as u64;
            }
        )*

    };
}

xc3_write_impl!(i8, i16, i32, i64, u8, u16, u32, u64, f32);

// TODO: macro for handling larger tuples?
impl<A: Xc3Write, B: Xc3Write> Xc3Write for (A, B) {
    type Offsets<'a> = (A::Offsets<'a>, B::Offsets<'a>) where A: 'a, B: 'a;

    fn xc3_write<W: Write + Seek>(
        &self,
        writer: &mut W,
        data_ptr: &mut u64,
    ) -> Xc3Result<Self::Offsets<'_>> {
        Ok((
            self.0.xc3_write(writer, data_ptr)?,
            self.1.xc3_write(writer, data_ptr)?,
        ))
    }
}

impl<A: Xc3WriteOffsets, B: Xc3WriteOffsets> Xc3WriteOffsets for (A, B) {
    fn write_offsets<W: Write + Seek>(
        &self,
        _writer: &mut W,
        _base_offset: u64,
        _data_ptr: &mut u64,
    ) -> Xc3Result<()> {
        Ok(())
    }
}

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
    ) -> Xc3Result<Self::Offsets<'_>> {
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
    ) -> Xc3Result<Self::Offsets<'_>> {
        writer.write_all(self.as_bytes())?;
        writer.write_all(&[0u8])?;
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
    ) -> Xc3Result<Self::Offsets<'_>> {
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
    ) -> Xc3Result<()> {
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
    ) -> Xc3Result<Self::Offsets<'_>> {
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
    ) -> Xc3Result<()> {
        Ok(())
    }
}

/// A small helper function for manually aligning the `data_ptr`.
pub const fn round_up(x: u64, n: u64) -> u64 {
    ((x + n - 1) / n) * n
}

#[doc(hidden)]
#[macro_export]
macro_rules! assert_hex_eq {
    ($a:expr, $b:expr) => {
        pretty_assertions::assert_str_eq!(hex::encode($a), hex::encode($b))
    };
}
