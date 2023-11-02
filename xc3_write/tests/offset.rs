use std::io::Cursor;

use hexlit::hex;
use xc3_write::{assert_hex_eq, write_full, Xc3Write, Xc3WriteOffsets};

#[derive(Xc3Write, Xc3WriteOffsets)]
struct A {
    #[xc3(offset(u32))]
    a: u32,
}

#[test]
fn write_offset() {
    let value = A { a: 1 };

    let mut writer = Cursor::new(Vec::new());
    let mut data_ptr = 0;
    value.xc3_write(&mut writer, &mut data_ptr).unwrap();

    assert_hex_eq!(hex!(00000000), writer.into_inner());
    assert_eq!(4, data_ptr);
}

#[test]
fn write_offset_full() {
    let value = A { a: 1 };

    let mut writer = Cursor::new(Vec::new());
    let mut data_ptr = 0;
    write_full(&value, &mut writer, 0, &mut data_ptr).unwrap();

    assert_hex_eq!(hex!(04000000 01000000), writer.into_inner());
    assert_eq!(8, data_ptr);
}
