use std::io::Cursor;

use hexlit::hex;
use xc3_write::{assert_hex_eq, write_full, Xc3Write, Xc3WriteOffsets};

#[test]
fn write_offset_count() {
    #[derive(Xc3Write, Xc3WriteOffsets)]
    struct Test {
        #[xc3(offset_count(u32, u32))]
        a: Vec<u8>,
    }

    let value = Test {
        a: vec![1, 2, 3, 4],
    };

    let mut writer = Cursor::new(Vec::new());
    value.xc3_write(&mut writer).unwrap();

    assert_hex_eq!(hex!(00000000 04000000), writer.into_inner());
}

#[test]
fn write_offset_count_full() {
    #[derive(Xc3Write, Xc3WriteOffsets)]
    struct Test {
        #[xc3(offset_count(u32, u32))]
        a: Vec<u8>,
    }

    let value = Test {
        a: vec![1, 2, 3, 4],
    };

    let mut writer = Cursor::new(Vec::new());
    let mut data_ptr = 0;
    write_full(&value, &mut writer, 0, &mut data_ptr).unwrap();

    assert_hex_eq!(hex!(08000000 04000000 01020304), writer.into_inner());
    assert_eq!(12, data_ptr);
}

#[test]
fn write_offset_count_full_align_0x0() {
    #[derive(Xc3Write, Xc3WriteOffsets)]
    struct Test {
        #[xc3(offset_count(u32, u32), align(16))]
        a: Vec<u8>,
    }

    let value = Test {
        a: vec![1, 2, 3, 4],
    };

    let mut writer = Cursor::new(Vec::new());
    let mut data_ptr = 0;
    write_full(&value, &mut writer, 0, &mut data_ptr).unwrap();

    assert_hex_eq!(
        hex!(10000000 04000000 00000000 00000000 01020304),
        writer.into_inner()
    );
    assert_eq!(20, data_ptr);
}

#[test]
fn write_offset_count_full_align_0xff() {
    #[derive(Xc3Write, Xc3WriteOffsets)]
    struct Test {
        #[xc3(offset_count(u32, u32), align(16, 0xff))]
        a: Vec<u8>,
    }

    let value = Test {
        a: vec![1, 2, 3, 4],
    };

    let mut writer = Cursor::new(Vec::new());
    let mut data_ptr = 0;
    write_full(&value, &mut writer, 0, &mut data_ptr).unwrap();

    assert_hex_eq!(
        hex!(10000000 04000000 ffffffff ffffffff 01020304),
        writer.into_inner()
    );
    assert_eq!(20, data_ptr);
}
