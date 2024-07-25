#![no_main]

use libfuzzer_sys::fuzz_target;

#[derive(Debug, arbitrary::Arbitrary)]
struct Input {
    mxmd: xc3_lib::mxmd::legacy::MxmdLegacy,
    casmt: Option<Vec<u8>>,
    hkt: Option<xc3_lib::hkt::Hkt>,
    model_programs: Option<xc3_model::shader_database::ModelPrograms>,
}

fuzz_target!(|input: Input| {
    let _ = xc3_model::ModelRoot::from_mxmd_model_legacy(
        &input.mxmd,
        input.casmt,
        input.hkt.as_ref(),
        input.model_programs.as_ref(),
    );
});
