use std::io::Cursor;

use hexlit::hex;
use xc3_write::{assert_hex_eq, Xc3Write, Xc3WriteOffsets};

#[derive(Xc3Write, Xc3WriteOffsets)]
struct A {
    a: u32,
    b: u8,
    c: Vec<i8>,
    d: String,
    #[xc3(saved_position(false))]
    _e: f32,
}

#[test]
fn write_struct_no_offsets() {
    let value = A {
        a: 1,
        b: 2,
        c: vec![-1, -1],
        d: "abc".to_string(),
        _e: 1.0,
    };

    let mut writer = Cursor::new(Vec::new());
    value.xc3_write(&mut writer).unwrap();

    assert_hex_eq!(hex!(01000000 02ffff61 626300 0000803f), writer.into_inner());
}
