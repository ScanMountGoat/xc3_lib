use std::io::Cursor;

use hexlit::hex;
use xc3_write::{assert_hex_eq, Xc3Write, Xc3WriteOffsets};

#[derive(Xc3Write, Xc3WriteOffsets)]
struct A {
    a: u32,
    b: u8,
    c: Vec<i8>,
    d: String,
}

#[test]
fn write_struct_no_offsets() {
    let value = A {
        a: 1,
        b: 2,
        c: vec![-1, -1],
        d: "abc".to_string(),
    };

    let mut writer = Cursor::new(Vec::new());
    let mut data_ptr = 0;
    value.xc3_write(&mut writer, &mut data_ptr).unwrap();

    assert_hex_eq!(hex!(01000000 02ffff61 626300), writer.into_inner());
    assert_eq!(11, data_ptr);
}
