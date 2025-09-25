use std::io::Cursor;

use hexlit::hex;
use xc3_write::{
    Endian, Offset, assert_hex_eq,
    strings::{StringSectionUniqueSorted, WriteOptions},
};

#[test]
fn write_string_section_unique_sorted() {
    let mut section = StringSectionUniqueSorted::default();

    section.insert_offset64(&Offset::new(0, &"def".to_string(), None, 0u8));
    section.insert_offset64(&Offset::new(8, &"abc".to_string(), None, 0u8));
    section.insert_offset64(&Offset::new(16, &"def".to_string(), None, 0u8));

    let mut writer = Cursor::new(Vec::new());
    section
        .write(
            &mut writer,
            &mut 25,
            &WriteOptions {
                start_alignment: 4,
                start_padding_byte: 0xff,
                string_alignment: 1,
                string_padding_byte: 0,
            },
            Endian::Little,
        )
        .unwrap();

    assert_hex_eq!(
        hex!(20000000 00000000 1c000000 00000000 20000000 00000000 00ffffff 61626300 64656600),
        writer.into_inner()
    );
}
