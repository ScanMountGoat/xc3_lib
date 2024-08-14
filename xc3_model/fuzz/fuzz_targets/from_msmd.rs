#![no_main]

use libfuzzer_sys::fuzz_target;

#[derive(Debug, arbitrary::Arbitrary)]
struct Input {
    msmd: xc3_lib::msmd::Msmd,
    wismda: Vec<u8>,
    map_programs: Option<xc3_model::shader_database::MapPrograms>,
}

fuzz_target!(|input: Input| {
    let _ = xc3_model::MapRoot::from_msmd(&input.msmd, &input.wismda, input.map_programs.as_ref());
});
