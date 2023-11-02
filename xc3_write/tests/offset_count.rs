use std::io::Cursor;

use hexlit::hex;
use xc3_write::{assert_hex_eq, write_full, Xc3Write, Xc3WriteOffsets};

#[derive(Xc3Write, Xc3WriteOffsets)]
struct A {
    #[xc3(offset_count(u32, u32))]
    a: Vec<u8>,
}

#[test]
fn write_offset_count() {
    let value = A {
        a: vec![1, 2, 3, 4],
    };

    let mut writer = Cursor::new(Vec::new());
    let mut data_ptr = 0;
    value.xc3_write(&mut writer, &mut data_ptr).unwrap();

    assert_hex_eq!(hex!(00000000 04000000), writer.into_inner());
    assert_eq!(8, data_ptr);
}

#[test]
fn write_offset_count_full() {
    let value = A {
        a: vec![1, 2, 3, 4],
    };

    let mut writer = Cursor::new(Vec::new());
    let mut data_ptr = 0;
    write_full(&value, &mut writer, 0, &mut data_ptr).unwrap();

    assert_hex_eq!(hex!(08000000 04000000 01020304), writer.into_inner());
    assert_eq!(12, data_ptr);
}
