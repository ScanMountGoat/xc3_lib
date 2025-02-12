use std::{
    collections::BTreeMap,
    io::{Seek, SeekFrom, Write},
};

use indexmap::IndexMap;

use crate::{Endian, Offset, Xc3Result, Xc3Write};

// TODO: support 32 and 64 bit offsets.

#[derive(Debug)]
pub struct WriteOptions {
    /// Alignment of the start of the string section in bytes.
    pub start_alignment: u64,

    /// The padding byte for aligning the start of the string section.
    pub start_padding_byte: u8,

    /// Alignment in bytes applied after writing each string.
    pub string_alignment: u64,

    /// The padding byte used for aligning strings.
    pub string_padding_byte: u8,
}

impl Default for WriteOptions {
    fn default() -> Self {
        Self {
            start_alignment: 1,
            start_padding_byte: 0,
            string_alignment: 1,
            string_padding_byte: 0,
        }
    }
}

/// Offsets to unique strings in alphabetical order.
#[derive(Debug, Clone, Default)]
pub struct StringSectionUniqueSorted {
    // Unique strings are stored in alphabetical order.
    name_to_offsets: BTreeMap<String, Vec<u64>>,
}

impl StringSectionUniqueSorted {
    /// Insert a 64-bit offset to update later.
    pub fn insert_offset64(&mut self, offset: &Offset<'_, u64, String>) {
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
        options: &WriteOptions,
        endian: Endian,
    ) -> Xc3Result<()> {
        let name_positions = write_strings(self.name_to_offsets.keys(), writer, data_ptr, options)?;

        for (offsets, position) in self.name_to_offsets.values().zip(name_positions) {
            update_offsets(writer, 0, endian, position as u64, offsets)?;
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
    pub fn insert_offset32(&mut self, offset: &Offset<'_, u32, String>) {
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
        options: &WriteOptions,
        endian: Endian,
    ) -> Xc3Result<()> {
        let name_positions = write_strings(self.name_to_offsets.keys(), writer, data_ptr, options)?;

        for (offsets, position) in self.name_to_offsets.values().zip(name_positions) {
            update_offsets(writer, base_offset, endian, position as u64, offsets)?;
        }

        Ok(())
    }
}

/// Offsets to strings in insertion order.
#[derive(Default)]
pub struct StringSection {
    name_to_offset: Vec<(String, u64)>,
}

impl StringSection {
    /// Insert a 32-bit offset to update later.
    pub fn insert_offset32(&mut self, offset: &Offset<'_, u32, String>) {
        self.name_to_offset
            .push((offset.data.clone(), offset.position));
    }

    /// Write the strings at `data_ptr` and update all stored offsets.
    pub fn write<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
        options: &WriteOptions,
        endian: Endian,
    ) -> Xc3Result<()> {
        let name_positions = write_strings(
            self.name_to_offset.iter().map(|(n, _)| n),
            writer,
            data_ptr,
            options,
        )?;

        // TODO: make base offset an argument or force it to first string?
        for ((_, offset), position) in self.name_to_offset.iter().zip(name_positions) {
            update_offsets(writer, base_offset, endian, position as u64, &[*offset])?;
        }

        Ok(())
    }
}

fn update_offsets<W: Write + Seek>(
    writer: &mut W,
    base_offset: u64,
    endian: Endian,
    position: u64,
    offsets: &[u64],
) -> Result<(), std::io::Error> {
    for offset in offsets {
        // Assume all string pointers are 4 bytes.
        writer.seek(SeekFrom::Start(*offset))?;
        let final_offset = position - base_offset;
        (final_offset as u32).xc3_write(writer, endian)?;
    }
    Ok(())
}

fn write_strings<'a, W: Write + Seek>(
    names: impl Iterator<Item = &'a String>,
    writer: &mut W,
    data_ptr: &mut u64,
    options: &WriteOptions,
) -> Xc3Result<Vec<u32>> {
    let mut name_positions = Vec::new();
    writer.seek(std::io::SeekFrom::Start(*data_ptr))?;
    align(
        writer,
        *data_ptr,
        options.start_alignment,
        options.start_padding_byte,
    )?;

    for name in names {
        // Assume all string pointers are 4 bytes.
        let position = writer.stream_position()? as u32;

        writer.write_all(name.as_bytes())?;
        writer.write_all(&[0u8])?;

        // Apply alignment to each string.
        let position_after_write = writer.stream_position()?;
        align(
            writer,
            position_after_write,
            options.string_alignment,
            options.string_padding_byte,
        )?;

        name_positions.push(position);
    }
    *data_ptr = (*data_ptr).max(writer.stream_position()?);

    Ok(name_positions)
}

fn align<W: Write>(writer: &mut W, size: u64, align: u64, pad: u8) -> std::io::Result<()> {
    let aligned_size = size.next_multiple_of(align);
    let padding = aligned_size - size;
    writer.write_all(&vec![pad; padding as usize])?;
    Ok(())
}
