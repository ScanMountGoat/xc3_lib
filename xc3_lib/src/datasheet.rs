//! Structured data in .bin files or embedded in .beh files.
//!
//! # File Paths
//!
//! | Game | Versions | File Patterns |
//! | --- | --- | --- |
//! | Xenoblade Chronicles 1 DE | |  |
//! | Xenoblade Chronicles 2 |  | |
//! | Xenoblade Chronicles 3 |  | `datasheet/*.bin` |
use bilge::prelude::*;
use binrw::{BinRead, BinWrite, NullString};
use std::io::{Read, Seek};
use xc3_write::{Xc3Write, Xc3WriteOffsets};

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[brw(magic(b"\x56\x34\x12\x00\x00\x00\x00\x00\x0f"))]
#[xc3(magic(b"\x56\x34\x12\x00\x00\x00\x00\x00\x0f"))]
pub struct DataSheet {
    pub key_values: KeyValues,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct KeyValues {
    pub keys: Vec<String>,
    pub value: Value,
}

// TODO: Preserve data for writing?
// TODO: Separate type for signed vs unsigned integers?
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub enum Value {
    Integer(i64),
    Float(f64),
    List(Vec<ListItem>),
    Struct(Vec<Field>),
    String(String),
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct ListItem {
    pub size: Value,
    pub value: Value,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct Field {
    pub key: Value,
    pub size: Value,
    pub value: Value,
}

#[bitsize(8)]
#[derive(DebugBits, FromBits, BinRead, BinWrite, PartialEq, Clone, Copy)]
#[br(map = u8::into)]
#[bw(map = |&x| u8::from(x))]
struct ValueType {
    ty: DataType,
    value: u4,
}

#[bitsize(4)]
#[derive(Debug, FromBits, PartialEq, Clone, Copy)]
enum DataType {
    Unk0 = 0,
    Int = 1,
    F32 = 2,
    F64 = 3,
    CStr = 4,
    List1 = 5,
    Struct1 = 6,
    Bool = 7,
    U8 = 8,
    I8 = 9,
    Unk10 = 10,
    Unk11 = 11,
    Str = 12,
    List2 = 13,
    Struct2 = 14,
    Unk15 = 15,
}

// TODO: Derive for better errors
impl BinRead for KeyValues {
    type Args<'a> = ();

    fn read_options<R: std::io::prelude::Read + std::io::prelude::Seek>(
        reader: &mut R,
        endian: binrw::Endian,
        args: Self::Args<'_>,
    ) -> binrw::prelude::BinResult<Self> {
        // TODO: Avoid panics.
        let count = match Value::read_options(reader, endian, args)? {
            Value::Integer(count) => count,
            _ => todo!(),
        };

        let mut keys = Vec::new();
        for _ in 0..count as usize {
            let s = NullString::read_options(reader, endian, args)?;
            keys.push(s.to_string());
        }

        let value = Value::read_options(reader, endian, args)?;
        Ok(Self { keys, value })
    }
}

// TODO: Derive for better errors
impl BinRead for Value {
    type Args<'a> = ();

    fn read_options<R: std::io::prelude::Read + std::io::prelude::Seek>(
        reader: &mut R,
        endian: binrw::Endian,
        args: Self::Args<'_>,
    ) -> binrw::prelude::BinResult<Self> {
        let ty = ValueType::read_options(reader, endian, args)?;
        let ty_value = u8::from(ty.value());

        match ty.ty() {
            DataType::Unk0 => todo!(),
            DataType::Int => read_var_int(ty_value, reader, endian, args).map(Value::Integer),
            DataType::F32 => {
                let value = f32::read_options(reader, endian, args)?;
                Ok(Value::Float(value.into()))
            }
            DataType::F64 => {
                let value = f64::read_options(reader, endian, args)?;
                Ok(Value::Float(value))
            }
            DataType::CStr => {
                let string = NullString::read_options(reader, endian, args)?;
                Ok(Value::String(string.to_string()))
            }
            DataType::List1 => {
                let count = read_var_int(ty_value, reader, endian, args)?;
                let mut items = Vec::new();
                for _ in 0..count {
                    let item = ListItem::read_options(reader, endian, args)?;
                    items.push(item)
                }
                Ok(Value::List(items))
            }
            DataType::Struct1 => {
                let field_count = read_var_int(ty_value, reader, endian, args)?;
                let mut fields = Vec::new();
                for _ in 0..field_count {
                    let field = Field::read_options(reader, endian, args)?;
                    fields.push(field);
                }
                Ok(Value::Struct(fields))
            }
            DataType::Bool => {
                // TODO: Boolean?
                Ok(Value::Integer(ty_value.into()))
            }
            DataType::U8 => Ok(Value::Integer(ty_value.into())),
            DataType::I8 => Ok(Value::Integer(-i64::from(ty_value))),
            DataType::Unk10 => {
                // TODO: Float?
                Ok(Value::Integer(ty_value.into()))
            }
            DataType::Unk11 => {
                // TODO: Float?
                Ok(Value::Integer(-i64::from(ty_value)))
            }
            DataType::Str => {
                let mut buf = vec![0u8; ty_value as usize];
                reader.read_exact(&mut buf)?;
                let value = String::from_utf8(buf).unwrap();
                Ok(Value::String(value))
            }
            DataType::List2 => {
                let mut items = Vec::new();
                for _ in 0..ty_value {
                    let item = ListItem::read_options(reader, endian, args)?;
                    items.push(item)
                }
                Ok(Value::List(items))
            }
            DataType::Struct2 => {
                let mut fields = Vec::new();
                for _ in 0..ty_value {
                    let field = Field::read_options(reader, endian, args)?;
                    fields.push(field);
                }
                Ok(Value::Struct(fields))
            }
            DataType::Unk15 => todo!(),
        }
    }
}

fn read_var_int<R: Read + Seek>(
    type_low: u8,
    reader: &mut R,
    endian: binrw::Endian,
    args: (),
) -> Result<i64, binrw::Error> {
    // TODO: Avoid panics.
    match type_low {
        1 => {
            let value = u32::read_options(reader, endian, args)?;
            Ok(value.into())
        }
        2 => {
            let value = u16::read_options(reader, endian, args)?;
            Ok(value.into())
        }
        3 => {
            let value = u8::read_options(reader, endian, args)?;
            Ok(value.into())
        }
        4 => {
            let value = u8::read_options(reader, endian, args)?;
            Ok(-i64::from(value))
        }
        5 => {
            let value = u16::read_options(reader, endian, args)?;
            Ok(-i64::from(value))
        }
        6 => {
            let value = u32::read_options(reader, endian, args)?;
            Ok(-i64::from(value))
        }
        _ => todo!(),
    }
}
