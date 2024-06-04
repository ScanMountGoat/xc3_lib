use std::io::Cursor;

use hexlit::hex;
use xc3_write::{assert_hex_eq, write_full, Xc3Write, Xc3WriteOffsets};

#[test]
fn write_offset() {
    #[derive(Xc3Write, Xc3WriteOffsets)]
    struct Test {
        #[xc3(offset(u32))]
        a: u32,
    }

    let value = Test { a: 1 };

    let mut writer = Cursor::new(Vec::new());
    value.xc3_write(&mut writer).unwrap();

    assert_hex_eq!(hex!(00000000), writer.into_inner());
}

#[test]
fn write_offset_full() {
    #[derive(Xc3Write, Xc3WriteOffsets)]
    struct Test {
        #[xc3(offset(u32))]
        inner: Inner,
    }

    #[derive(Xc3Write, Xc3WriteOffsets)]
    #[xc3(align(8))]
    struct Inner {
        a: u32,
    }

    let value = Test {
        inner: Inner { a: 1 },
    };

    let mut writer = Cursor::new(Vec::new());
    let mut data_ptr = 0;
    write_full(&value, &mut writer, 0, &mut data_ptr).unwrap();

    assert_hex_eq!(hex!(08000000 00000000 01000000), writer.into_inner());
    assert_eq!(12, data_ptr);
}

#[test]
fn write_offset_full_align_0x0() {
    #[derive(Xc3Write, Xc3WriteOffsets)]
    struct Test {
        #[xc3(offset(u32), align(8))]
        a: u32,
    }

    let value = Test { a: 1 };

    let mut writer = Cursor::new(Vec::new());
    let mut data_ptr = 0;
    write_full(&value, &mut writer, 0, &mut data_ptr).unwrap();

    assert_hex_eq!(hex!(08000000 00000000 01000000), writer.into_inner());
    assert_eq!(12, data_ptr);
}

#[test]
fn write_offset_full_align_0xff() {
    #[derive(Xc3Write, Xc3WriteOffsets)]
    struct Test {
        #[xc3(offset(u32), align(8, 0xff))]
        a: u32,
    }

    let value = Test { a: 1 };

    let mut writer = Cursor::new(Vec::new());
    let mut data_ptr = 0;
    write_full(&value, &mut writer, 0, &mut data_ptr).unwrap();

    assert_hex_eq!(hex!(08000000 ffffffff 01000000), writer.into_inner());
    assert_eq!(12, data_ptr);
}

#[test]
fn write_offset_full_optional_offset_some() {
    #[derive(Xc3Write, Xc3WriteOffsets)]
    struct Test {
        #[xc3(offset(u32))]
        a: Option<u32>,
    }

    let value = Test { a: Some(1) };

    let mut writer = Cursor::new(Vec::new());
    let mut data_ptr = 0;
    write_full(&value, &mut writer, 0, &mut data_ptr).unwrap();

    assert_hex_eq!(hex!(04000000 01000000), writer.into_inner());
    assert_eq!(8, data_ptr);
}

#[test]
fn write_offset_full_optional_offset_none() {
    #[derive(Xc3Write, Xc3WriteOffsets)]
    struct Test {
        #[xc3(offset(u32))]
        a: Option<u32>,
    }

    // This should still write a null offset.
    let value = Test { a: None };

    let mut writer = Cursor::new(Vec::new());
    let mut data_ptr = 0;
    write_full(&value, &mut writer, 0, &mut data_ptr).unwrap();

    assert_hex_eq!(hex!(00000000), writer.into_inner());
    assert_eq!(4, data_ptr);
}
