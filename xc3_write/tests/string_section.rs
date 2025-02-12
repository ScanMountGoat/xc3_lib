use std::io::Cursor;

use hexlit::hex;
use xc3_write::{
    assert_hex_eq,
    strings::{StringSection, WriteOptions},
    Endian, Offset,
};

#[test]
fn write_string_section() {
    let mut section = StringSection::default();

    section.insert_offset32(&Offset::new(0, &"abc".to_string(), None, 0u8));
    section.insert_offset32(&Offset::new(4, &"def".to_string(), None, 0u8));
    section.insert_offset32(&Offset::new(8, &"abc".to_string(), None, 0u8));

    let mut writer = Cursor::new(Vec::new());
    section
        .write(
            &mut writer,
            0,
            &mut 13,
            &WriteOptions {
                start_alignment: 4,
                start_padding_byte: 0xff,
                string_alignment: 8,
                string_padding_byte: 0x12,
            },
            Endian::Little,
        )
        .unwrap();

    assert_hex_eq!(
        hex!(10000000 18000000 20000000 00ffffff 61626300 12121212 64656600 12121212 61626300 12121212),
        writer.into_inner()
    );
}
