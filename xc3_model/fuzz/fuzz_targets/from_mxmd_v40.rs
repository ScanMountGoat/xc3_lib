#![no_main]

use libfuzzer_sys::fuzz_target;

#[derive(Debug, arbitrary::Arbitrary)]
struct Input {
    mxmd: xc3_lib::mxmd::MxmdV40,
    skel: Option<xc3_lib::bc::skel::Skel>,
    vertex: xc3_lib::mxmd::legacy::VertexData,
    spch: xc3_lib::spch::Spch,
    textures: xc3_model::ExtractedTextures,
}

fuzz_target!(|input: Input| {
    let files = xc3_model::model::import::ModelFilesV40 {
        models: &input.mxmd.models,
        materials: &input.mxmd.materials,
        vertex: std::borrow::Cow::Owned(input.vertex),
        spch: std::borrow::Cow::Owned(input.spch),
        textures: input.textures,
    };
    // TODO: test with database?
    let _ = xc3_model::ModelRoot::from_mxmd_v40(&files, input.skel, None);
});
