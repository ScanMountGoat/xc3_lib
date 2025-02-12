use std::io::Cursor;

use hexlit::hex;
use xc3_write::{assert_hex_eq, strings::StringSectionUnique, Endian, Offset};

#[test]
fn write_string_section_unique() {
    let mut section = StringSectionUnique::default();

    section.insert_offset(&Offset::new(0, &String::from("abc"), None, 0u8));
    section.insert_offset(&Offset::new(4, &String::from("def"), None, 0u8));

    let mut writer = Cursor::new(Vec::new());
    section
        .write(&mut writer, 0, &mut 9, 4, Endian::Little)
        .unwrap();

    assert_hex_eq!(
        hex!(0c000000 10000000 00ffffff 61626300 64656600),
        writer.into_inner()
    );
}
