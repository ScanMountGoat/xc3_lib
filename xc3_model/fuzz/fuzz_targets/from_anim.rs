#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|anim: xc3_lib::bc::anim::Anim| {
    let _ = xc3_model::animation::Animation::from_anim(&anim);
});
