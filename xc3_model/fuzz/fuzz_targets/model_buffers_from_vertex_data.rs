#![no_main]

use libfuzzer_sys::fuzz_target;

#[derive(Debug, arbitrary::Arbitrary)]
struct Input {
    vertex: xc3_lib::vertex::VertexData,
}

fuzz_target!(|input: Input| {
    let _ = xc3_model::vertex::ModelBuffers::from_vertex_data(&input.vertex);
});
