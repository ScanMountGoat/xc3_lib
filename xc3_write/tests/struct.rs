use std::io::Cursor;

use hexlit::hex;
use xc3_write::{assert_hex_eq, Endian, WriteFull, Xc3Write, Xc3WriteOffsets};

#[derive(Xc3Write, Xc3WriteOffsets)]
#[xc3(align_after(20))]
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
    value
        .write_full(&mut writer, 0, &mut 0, Endian::Little, ())
        .unwrap();

    assert_hex_eq!(
        hex!(01000000 02ffff61 626300 0000803f 0000000000),
        writer.into_inner()
    );
}
