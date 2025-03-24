//! A binary writing and layout implementation using separate write and layout passes.
use std::io::Seek;
use std::io::SeekFrom;
use std::io::Write;
use std::marker::PhantomData;
use std::ops::Deref;

// io::Error supports custom error variants if needed.
// Writing will typically only fail from io errors on the writer anyway.
pub type Xc3Result<T> = Result<T, std::io::Error>;

pub mod strings;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Endian {
    Little,
    Big,
}

/// The write pass that writes fields and placeholder offsets.
pub trait Xc3Write {
    /// The type storing offset data to be used in [Xc3WriteOffsets].
    type Offsets<'a>
    where
        Self: 'a;

    /// Write all fields and placeholder offsets.
    /// This should almost always be derived for non primitive types.
    ///
    /// An object's size is defined as the difference between the writer position
    /// before and after the first pass and does not need to be user defined.
    /// Custom implementations of [Xc3Write] should ensure the write head points after the data
    /// when the function returns to ensure correct size calculations.
    fn xc3_write<W: Write + Seek>(
        &self,
        writer: &mut W,
        endian: Endian,
    ) -> Xc3Result<Self::Offsets<'_>>;

    /// Return `Some(_)` if the offset should be updated and
    /// `Some(true)` if the data should also be written.
    /// Defaults to `Some(true)`.
    fn should_write(&self) -> Option<bool> {
        Some(true)
    }

    /// The alignment of absolute offsets for this type in bytes.
    const ALIGNMENT: u64 = 4;
}

/// The layout pass that updates and writes data for all fields in [Xc3Write::Offsets] recursively.
pub trait Xc3WriteOffsets {
    type Args;

    /// Update and write pointed to data for all fields in [Xc3Write::Offsets].
    ///
    /// The goal is to call [Offset::write] or [Xc3WriteOffsets::write_offsets]
    /// in increasing order by absolute offset stored in `data_ptr`.
    /// For writing in order by field recursively, simply derive [Xc3WriteOffsets].
    /// Manually implementing this trait allows flexibility for cases like placing strings
    /// for all types at the end of the file.
    fn write_offsets<W: Write + Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
        endian: Endian,
        args: Self::Args,
    ) -> Xc3Result<()>;
}

/// A complete writing combining [Xc3Write] and [Xc3WriteOffsets].
///
/// Most types should rely on the blanket impl.
/// For types without offsets, simply set [Xc3WriteOffsets::Args] to the unit type `()`.
pub trait WriteFull {
    /// The type for [Xc3WriteOffsets::Args].
    type Args;

    /// A complete write uses a two pass approach to handle offsets.
    ///
    /// We can fully write any type that can fully write its offset values.
    /// This includes types with an offset type of () like primitive types.
    fn write_full<W: Write + Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
        endian: Endian,
        offset_args: Self::Args,
    ) -> Xc3Result<()>;
}

impl<T, A> WriteFull for T
where
    T: Xc3Write,
    for<'a> T::Offsets<'a>: Xc3WriteOffsets<Args = A>,
{
    type Args = A;

    fn write_full<W: Write + Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
        endian: Endian,
        offset_args: Self::Args,
    ) -> Xc3Result<()> {
        // Ensure all items are written before their pointed to data.
        let offsets = self.xc3_write(writer, endian)?;
        *data_ptr = (*data_ptr).max(writer.stream_position()?);

        offsets.write_offsets(writer, base_offset, data_ptr, endian, offset_args)?;
        // Account for padding or alignment added after writing.
        *data_ptr = (*data_ptr).max(writer.stream_position()?);
        Ok(())
    }
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

impl<P, T: Xc3Write> std::fmt::Debug for Offset<'_, P, T> {
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

    pub fn set_offset<W>(&self, writer: &mut W, offset: u64, endian: Endian) -> Xc3Result<()>
    where
        W: Write + Seek,
        // TODO: Create a trait for this?
        P: TryFrom<u64> + Xc3Write,
        <P as TryFrom<u64>>::Error: std::fmt::Debug,
    {
        writer.seek(SeekFrom::Start(self.position))?;
        let offset = P::try_from(offset).unwrap();
        offset.xc3_write(writer, endian)?;
        Ok(())
    }

    fn set_offset_seek<W>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
        type_alignment: u64,
        should_write: bool,
        endian: Endian,
    ) -> Xc3Result<()>
    where
        W: Write + Seek,
        // TODO: Create a trait for this?
        P: TryFrom<u64> + Xc3Write,
        <P as TryFrom<u64>>::Error: std::fmt::Debug,
    {
        // Assume the data pointer hasn't been modified since the first pass.
        *data_ptr = (*data_ptr).max(writer.stream_position()?);

        // Account for the type or field alignment.
        let alignment = self.field_alignment.unwrap_or(type_alignment);
        let aligned_data_pr = data_ptr.next_multiple_of(alignment);

        // Update the offset value.
        self.set_offset(writer, aligned_data_pr - base_offset, endian)?;

        if should_write {
            // Seek to the data position.
            // Handle any padding up the desired alignment.
            writer.seek(SeekFrom::Start(*data_ptr))?;
            vec![self.padding_byte; (aligned_data_pr - *data_ptr) as usize]
                .xc3_write(writer, endian)?;
            // Point the data pointer past this data.
            *data_ptr = (*data_ptr).max(writer.stream_position()?);
        }

        Ok(())
    }
}

impl<P, T> Offset<'_, P, T>
where
    T: Xc3Write,
    P: TryFrom<u64> + Xc3Write,
    <P as TryFrom<u64>>::Error: std::fmt::Debug,
{
    pub fn write<W: Write + Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
        endian: Endian,
    ) -> Xc3Result<T::Offsets<'_>> {
        if let Some(should_write) = self.data.should_write() {
            self.set_offset_seek(
                writer,
                base_offset,
                data_ptr,
                T::ALIGNMENT,
                should_write,
                endian,
            )?;
        }
        let offsets = self.data.xc3_write(writer, endian)?;
        *data_ptr = (*data_ptr).max(writer.stream_position()?);
        Ok(offsets)
    }
}

impl<P, T> Offset<'_, P, T>
where
    T: Xc3Write + WriteFull,
    P: TryFrom<u64> + Xc3Write,
    <P as TryFrom<u64>>::Error: std::fmt::Debug,
{
    pub fn write_full<W: Write + Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
        endian: Endian,
        args: T::Args,
    ) -> Xc3Result<()> {
        // Always skip null offsets but not always empty vecs.
        if let Some(should_write) = self.data.should_write() {
            self.set_offset_seek(
                writer,
                base_offset,
                data_ptr,
                T::ALIGNMENT,
                should_write,
                endian,
            )?;
            self.data
                .write_full(writer, base_offset, data_ptr, endian, args)?;
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
                    endian: Endian,
                ) -> Xc3Result<Self::Offsets<'_>> {
                    match endian {
                        Endian::Little => writer.write_all(&self.to_le_bytes())?,
                        Endian::Big => writer.write_all(&self.to_be_bytes())?,
                    }
                    Ok(())
                }

                // TODO: Should this be specified manually?
                const ALIGNMENT: u64 = std::mem::align_of::<$ty>() as u64;
            }
        )*

    };
}

xc3_write_impl!(i8, i16, i32, i64, u8, u16, u32, u64, f32, f64);

// TODO: macro for handling larger tuples?
impl<A: Xc3Write, B: Xc3Write> Xc3Write for (A, B) {
    type Offsets<'a>
        = (A::Offsets<'a>, B::Offsets<'a>)
    where
        A: 'a,
        B: 'a;

    fn xc3_write<W: Write + Seek>(
        &self,
        writer: &mut W,
        endian: Endian,
    ) -> Xc3Result<Self::Offsets<'_>> {
        Ok((
            self.0.xc3_write(writer, endian)?,
            self.1.xc3_write(writer, endian)?,
        ))
    }
}

impl<A: Xc3WriteOffsets, B: Xc3WriteOffsets> Xc3WriteOffsets for (A, B) {
    type Args = ();

    fn write_offsets<W: Write + Seek>(
        &self,
        _: &mut W,
        _: u64,
        _: &mut u64,
        _: Endian,
        _: (),
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
        endian: Endian,
    ) -> Xc3Result<Self::Offsets<'_>> {
        for value in self {
            value.xc3_write(writer, endian)?;
        }
        Ok(())
    }
}

impl<T: Xc3Write> Xc3Write for Box<T> {
    type Offsets<'a>
        = T::Offsets<'a>
    where
        Self: 'a;

    fn xc3_write<W: Write + Seek>(
        &self,
        writer: &mut W,
        endian: Endian,
    ) -> Xc3Result<Self::Offsets<'_>> {
        self.deref().xc3_write(writer, endian)
    }
}

impl Xc3Write for String {
    type Offsets<'a> = ();

    fn xc3_write<W: Write + Seek>(
        &self,
        writer: &mut W,
        _: Endian,
    ) -> Xc3Result<Self::Offsets<'_>> {
        writer.write_all(self.as_bytes())?;
        writer.write_all(&[0u8])?;
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
        endian: Endian,
    ) -> Xc3Result<Self::Offsets<'_>> {
        // TODO: Find a less hacky way to specialize Vec<u8>.
        let offsets = if let Some(bytes) = <dyn core::any::Any>::downcast_ref::<Vec<u8>>(self) {
            // Avoiding writing buffers byte by byte to drastically improve performance.
            writer.write_all(bytes)?;
            Vec::new()
        } else {
            self.iter()
                .map(|v| v.xc3_write(writer, endian))
                .collect::<Result<Vec<_>, _>>()?
        };
        Ok(VecOffsets(offsets))
    }

    fn should_write(&self) -> Option<bool> {
        Some(!self.is_empty())
    }
}

impl<T, A> Xc3WriteOffsets for VecOffsets<T>
where
    T: Xc3WriteOffsets<Args = A>,
    A: Clone,
{
    type Args = A;

    fn write_offsets<W: Write + Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
        endian: Endian,
        args: Self::Args,
    ) -> Xc3Result<()> {
        // TODO: How to support non clone args?
        for item in &self.0 {
            item.write_offsets(writer, base_offset, data_ptr, endian, args.clone())?;
        }
        Ok(())
    }
}

impl Xc3Write for () {
    type Offsets<'a> = ();

    fn xc3_write<W: Write + Seek>(&self, _: &mut W, _: Endian) -> Xc3Result<Self::Offsets<'_>> {
        Ok(())
    }

    const ALIGNMENT: u64 = 1;
}

impl Xc3WriteOffsets for () {
    type Args = ();
    fn write_offsets<W: Write + Seek>(
        &self,
        _: &mut W,
        _: u64,
        _: &mut u64,
        _: Endian,
        _: Self::Args,
    ) -> Xc3Result<()> {
        Ok(())
    }
}

impl<T> Xc3Write for Option<T>
where
    T: Xc3Write,
{
    type Offsets<'a>
        = Option<T::Offsets<'a>>
    where
        Self: 'a;

    fn xc3_write<W: Write + Seek>(
        &self,
        writer: &mut W,
        endian: Endian,
    ) -> Xc3Result<Self::Offsets<'_>> {
        self.as_ref()
            .map(|v| v.xc3_write(writer, endian))
            .transpose()
    }

    fn should_write(&self) -> Option<bool> {
        self.as_ref().map(|_| true)
    }

    const ALIGNMENT: u64 = T::ALIGNMENT;
}

impl<T, A> Xc3WriteOffsets for Option<T>
where
    T: Xc3WriteOffsets<Args = A>,
{
    type Args = A;

    fn write_offsets<W: Write + Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
        endian: Endian,
        args: Self::Args,
    ) -> Xc3Result<()> {
        if let Some(value) = self {
            value.write_offsets(writer, base_offset, data_ptr, endian, args)?;
        }
        Ok(())
    }
}

#[doc(hidden)]
#[macro_export]
macro_rules! assert_hex_eq {
    ($a:expr, $b:expr) => {
        pretty_assertions::assert_str_eq!(hex::encode($a), hex::encode($b))
    };
}
