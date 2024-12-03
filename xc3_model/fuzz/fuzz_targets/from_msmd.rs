#![no_main]

use libfuzzer_sys::fuzz_target;

#[derive(Debug, arbitrary::Arbitrary)]
struct Input {
    msmd: xc3_lib::msmd::Msmd,
    // TODO: test structured data instead of bytes?
    wismda: Vec<u8>,
}

fuzz_target!(|input: Input| {
    // TODO: test database.
    let _ = xc3_model::MapRoot::from_msmd(&input.msmd, &input.wismda, None);
});
