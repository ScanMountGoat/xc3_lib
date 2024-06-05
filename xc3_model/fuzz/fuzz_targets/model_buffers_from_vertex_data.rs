#![no_main]

use libfuzzer_sys::fuzz_target;

#[derive(Debug, arbitrary::Arbitrary)]
struct Input {
    vertex: xc3_lib::vertex::VertexData,
    skinning: Option<xc3_lib::mxmd::Skinning>,
}

fuzz_target!(|input: Input| {
    let _ =
        xc3_model::vertex::ModelBuffers::from_vertex_data(&input.vertex, input.skinning.as_ref());
});
