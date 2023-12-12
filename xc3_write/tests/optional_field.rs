use std::io::Cursor;

use hexlit::hex;
use xc3_write::{assert_hex_eq, write_full, Xc3Write, Xc3WriteOffsets};

#[test]
fn write_offset_full_some() {
    #[derive(Xc3Write, Xc3WriteOffsets)]
    struct Test {
        #[xc3(offset(u32))]
        a: u32,
        b: Option<u32>,
    }

    let value = Test { a: 1, b: Some(2) };

    let mut writer = Cursor::new(Vec::new());
    let mut data_ptr = 0;
    write_full(&value, &mut writer, 0, &mut data_ptr).unwrap();

    assert_hex_eq!(hex!(08000000 02000000 01000000), writer.into_inner());
    assert_eq!(12, data_ptr);
}

#[test]
fn write_offset_full_none() {
    // A null field has 0 size.
    #[derive(Xc3Write, Xc3WriteOffsets)]
    struct Test {
        #[xc3(offset(u32))]
        a: u32,
        b: Option<u32>,
    }

    let value = Test { a: 1, b: None };

    let mut writer = Cursor::new(Vec::new());
    let mut data_ptr = 0;
    write_full(&value, &mut writer, 0, &mut data_ptr).unwrap();

    assert_hex_eq!(hex!(04000000 01000000), writer.into_inner());
    assert_eq!(8, data_ptr);
}
