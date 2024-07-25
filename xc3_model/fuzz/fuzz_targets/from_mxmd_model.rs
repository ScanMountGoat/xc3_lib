#![no_main]

use libfuzzer_sys::fuzz_target;

#[derive(Debug, arbitrary::Arbitrary)]
struct Input {
    mxmd: xc3_lib::mxmd::Mxmd,
    chr: Option<xc3_lib::sar1::Sar1>,
    vertex: xc3_lib::vertex::VertexData,
    textures: xc3_model::ExtractedTextures,
    model_programs: Option<xc3_model::shader_database::ModelPrograms>,
}

fuzz_target!(|input: Input| {
    let streaming_data = xc3_model::StreamingData {
        vertex: std::borrow::Cow::Owned(input.vertex),
        textures: input.textures,
    };
    let _ = xc3_model::ModelRoot::from_mxmd_model(
        &input.mxmd,
        input.chr,
        &streaming_data,
        input.model_programs.as_ref(),
    );
});
