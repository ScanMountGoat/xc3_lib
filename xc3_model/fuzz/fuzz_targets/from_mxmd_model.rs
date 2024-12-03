#![no_main]

use libfuzzer_sys::fuzz_target;

#[derive(Debug, arbitrary::Arbitrary)]
struct Input {
    mxmd: xc3_lib::mxmd::Mxmd,
    chr: Option<xc3_lib::sar1::Sar1>,
    vertex: xc3_lib::vertex::VertexData,
    spch: xc3_lib::spch::Spch,
    textures: xc3_model::ExtractedTextures,
    texture_indices: Option<Vec<u16>>,
}

fuzz_target!(|input: Input| {
    let streaming_data = xc3_model::StreamingData {
        vertex: std::borrow::Cow::Owned(input.vertex),
        spch: std::borrow::Cow::Owned(input.spch),
        textures: input.textures,
        texture_indices: input.texture_indices,
    };
    // TODO: test database.
    let _ = xc3_model::ModelRoot::from_mxmd_model(&input.mxmd, input.chr, &streaming_data, None);
});
