#![no_main]

use libfuzzer_sys::fuzz_target;

#[derive(Debug, arbitrary::Arbitrary)]
struct Input {
    root: xc3_model::ModelRoot,
    mxmd: xc3_lib::mxmd::Mxmd,
    msrd: xc3_lib::msrd::Msrd,
}

fuzz_target!(|input: Input| {
    let _ = input.root.to_mxmd_model(&input.mxmd, &input.msrd);
});
