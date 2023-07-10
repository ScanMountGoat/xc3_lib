use binrw::{BinResult, BinWrite};

pub(crate) trait Xc3Write {
    type Offsets;

    fn write<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        data_ptr: &mut u64,
    ) -> BinResult<Self::Offsets>;

    // TODO: Look at pointers to determine default alignment.
    const ALIGNMENT: u64 = 4;
}

// TODO: Macro for implementing for binwrite?
impl Xc3Write for String {
    type Offsets = ();

    fn write<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        data_ptr: &mut u64,
    ) -> BinResult<Self::Offsets> {
        self.as_bytes().write_le(writer)?;
        0u8.write_le(writer)?;
        *data_ptr = (*data_ptr).max(writer.stream_position()?);
        Ok(())
    }

    const ALIGNMENT: u64 = 1;
}

impl Xc3Write for u16 {
    type Offsets = ();

    fn write<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        data_ptr: &mut u64,
    ) -> BinResult<Self::Offsets> {
        let result = self.write_le(writer);
        *data_ptr = (*data_ptr).max(writer.stream_position()?);
        result
    }
}

impl<T: Xc3Write> Xc3Write for Vec<T> {
    type Offsets = Vec<T::Offsets>;

    fn write<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        data_ptr: &mut u64,
    ) -> BinResult<Self::Offsets> {
        let result = self.iter().map(|v| v.write(writer, data_ptr)).collect();
        *data_ptr = (*data_ptr).max(writer.stream_position()?);
        result
    }
}

const fn round_up(x: u64, n: u64) -> u64 {
    ((x + n - 1) / n) * n
}

// TODO: make the data ptr u32?
// TODO: Specify an alignment using another trait?
pub(crate) fn write_offset<W: std::io::Write + std::io::Seek, T: Xc3Write>(
    writer: &mut W,
    offset: u64,
    data_ptr_base_offset: u64,
    data_ptr: &mut u64,
    data: &T,
) -> BinResult<T::Offsets> {
    // Update the offset.
    writer.seek(std::io::SeekFrom::Start(offset))?;
    *data_ptr = round_up(*data_ptr, T::ALIGNMENT);
    ((*data_ptr - data_ptr_base_offset) as u32).write_le(writer)?;

    // Write the data.
    writer.seek(std::io::SeekFrom::Start(*data_ptr))?;
    let offsets = data.write(writer, data_ptr)?;

    Ok(offsets)
}
