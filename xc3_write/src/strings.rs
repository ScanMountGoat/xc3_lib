use std::{
    collections::BTreeMap,
    io::{Seek, SeekFrom, Write},
};

use indexmap::IndexMap;

use crate::{Endian, Offset, Xc3Result, Xc3Write};

// TODO: support 32 and 64 bit offsets.

/// Offsets to unique strings in alphabetical order.
#[derive(Debug, Clone, Default)]
pub struct StringSectionUniqueSorted {
    // Unique strings are stored in alphabetical order.
    name_to_offsets: BTreeMap<String, Vec<u64>>,
}

impl StringSectionUniqueSorted {
    /// Insert a 64-bit offset to update later.
    pub fn insert_offset(&mut self, offset: &Offset<'_, u64, String>) {
        self.name_to_offsets
            .entry(offset.data.clone())
            .or_default()
            .push(offset.position);
    }

    /// Write the strings at `data_ptr` and update all stored offsets.
    pub fn write<W: Write + Seek>(
        &self,
        writer: &mut W,
        data_ptr: &mut u64,
        alignment: u64,
        endian: Endian,
    ) -> Xc3Result<()> {
        let name_to_position =
            write_strings(self.name_to_offsets.keys(), writer, data_ptr, alignment)?;

        for (name, offsets) in &self.name_to_offsets {
            update_offsets(&name_to_position, writer, 0, endian, name, offsets)?;
        }

        Ok(())
    }
}

/// Offsets to unique strings in insertion order.
#[derive(Default)]
pub struct StringSectionUnique {
    name_to_offsets: IndexMap<String, Vec<u64>>,
}

impl StringSectionUnique {
    /// Insert a 32-bit offset to update later.
    pub fn insert_offset(&mut self, offset: &Offset<'_, u32, String>) {
        self.name_to_offsets
            .entry(offset.data.clone())
            .or_default()
            .push(offset.position);
    }

    /// Write the strings at `data_ptr` and update all stored offsets.
    pub fn write<W: Write + Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
        alignment: u64,
        endian: Endian,
    ) -> Xc3Result<()> {
        let name_to_position =
            write_strings(self.name_to_offsets.keys(), writer, data_ptr, alignment)?;

        for (name, offsets) in &self.name_to_offsets {
            update_offsets(
                &name_to_position,
                writer,
                base_offset,
                endian,
                name,
                offsets,
            )?;
        }

        Ok(())
    }
}

fn update_offsets<W: Write + Seek>(
    name_to_position: &BTreeMap<&String, u64>,
    writer: &mut W,
    base_offset: u64,
    endian: Endian,
    name: &String,
    offsets: &[u64],
) -> Result<(), std::io::Error> {
    Ok(for offset in offsets {
        let position = name_to_position[name];
        // Assume all string pointers are 4 bytes.
        writer.seek(SeekFrom::Start(*offset))?;
        let final_offset = position - base_offset;
        (final_offset as u32).xc3_write(writer, endian)?;
    })
}

fn write_strings<'a, W: Write + Seek>(
    names: impl Iterator<Item = &'a String>,
    writer: &mut W,
    data_ptr: &mut u64,
    alignment: u64,
) -> Xc3Result<BTreeMap<&'a String, u64>> {
    // TODO: Cleaner way to handle alignment?
    let mut name_to_position = BTreeMap::new();
    writer.seek(SeekFrom::Start(*data_ptr))?;
    align(writer, *data_ptr, alignment, 0xff)?;

    for name in names {
        let offset = writer.stream_position()?;
        writer.write_all(name.as_bytes())?;
        writer.write_all(&[0u8])?;
        name_to_position.insert(name, offset);
    }
    *data_ptr = (*data_ptr).max(writer.stream_position()?);

    Ok(name_to_position)
}

fn align<W: Write>(writer: &mut W, size: u64, align: u64, pad: u8) -> std::io::Result<()> {
    let aligned_size = size.next_multiple_of(align);
    let padding = aligned_size - size;
    writer.write_all(&vec![pad; padding as usize])?;
    Ok(())
}
