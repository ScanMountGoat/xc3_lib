use std::io::Cursor;

use hexlit::hex;
use xc3_write::{Endian, Xc3Write, Xc3WriteOffsets, assert_hex_eq};

#[test]
fn write_enum_variant_magic() {
    #[derive(Xc3Write, Xc3WriteOffsets)]
    #[xc3(magic(1u32))]
    enum A {
        #[xc3(magic(2u32))]
        B(u32),
    }

    let value = A::B(3);

    let mut writer = Cursor::new(Vec::new());
    value.xc3_write(&mut writer, Endian::Little).unwrap();

    assert_hex_eq!(hex!(01000000 02000000 03000000), writer.into_inner());
}
