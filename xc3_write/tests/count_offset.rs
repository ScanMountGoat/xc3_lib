use std::io::Cursor;

use hexlit::hex;
use xc3_write::{Endian, WriteFull, Xc3Write, Xc3WriteOffsets, assert_hex_eq};

#[test]
fn write_count_offset() {
    #[derive(Xc3Write, Xc3WriteOffsets)]
    struct Test {
        #[xc3(count_offset(u32, u32))]
        a: Vec<u8>,
    }

    let value = Test {
        a: vec![1, 2, 3, 4],
    };

    let mut writer = Cursor::new(Vec::new());
    value.xc3_write(&mut writer, Endian::Little).unwrap();

    assert_hex_eq!(hex!(04000000 00000000), writer.into_inner());
}

#[test]
fn write_count_offset_full() {
    #[derive(Xc3Write, Xc3WriteOffsets)]
    struct Test {
        #[xc3(count_offset(u32, u32))]
        a: Vec<u8>,
    }

    let value = Test {
        a: vec![1, 2, 3, 4],
    };

    let mut writer = Cursor::new(Vec::new());
    let mut data_ptr = 0;
    value
        .write_full(&mut writer, 0, &mut data_ptr, Endian::Little, ())
        .unwrap();

    assert_hex_eq!(hex!(04000000 08000000 01020304), writer.into_inner());
    assert_eq!(12, data_ptr);
}

#[test]
fn write_count_offset_full_align_0x0() {
    #[derive(Xc3Write, Xc3WriteOffsets)]
    struct Test {
        #[xc3(count_offset(u32, u32), align(16))]
        a: Vec<u8>,
    }

    let value = Test {
        a: vec![1, 2, 3, 4],
    };

    let mut writer = Cursor::new(Vec::new());
    let mut data_ptr = 0;
    value
        .write_full(&mut writer, 0, &mut data_ptr, Endian::Little, ())
        .unwrap();

    assert_hex_eq!(
        hex!(04000000 10000000 00000000 00000000 01020304),
        writer.into_inner()
    );
    assert_eq!(20, data_ptr);
}

#[test]
fn write_count_offset_full_align_0xff() {
    #[derive(Xc3Write, Xc3WriteOffsets)]
    struct Test {
        #[xc3(count_offset(u32, u32), align(16, 0xff))]
        a: Vec<u8>,
    }

    let value = Test {
        a: vec![1, 2, 3, 4],
    };

    let mut writer = Cursor::new(Vec::new());
    let mut data_ptr = 0;
    value
        .write_full(&mut writer, 0, &mut data_ptr, Endian::Little, ())
        .unwrap();

    assert_hex_eq!(
        hex!(04000000 10000000 ffffffff ffffffff 01020304),
        writer.into_inner()
    );
    assert_eq!(20, data_ptr);
}
