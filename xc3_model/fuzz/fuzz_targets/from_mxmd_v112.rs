#![no_main]

use libfuzzer_sys::fuzz_target;

#[derive(Debug, arbitrary::Arbitrary)]
struct Input {
    mxmd: xc3_lib::mxmd::MxmdV112,
    skel: Option<xc3_lib::bc::skel::Skel>,
    vertex: xc3_lib::vertex::VertexData,
    spch: xc3_lib::spch::Spch,
    textures: xc3_model::ExtractedTextures,
    texture_indices: Option<Vec<u16>>,
}

fuzz_target!(|input: Input| {
    let files = xc3_model::model::import::ModelFilesV112 {
        models: &input.mxmd.models,
        materials: &input.mxmd.materials,
        vertex: std::borrow::Cow::Owned(input.vertex),
        spch: std::borrow::Cow::Owned(input.spch),
        textures: input.textures,
        texture_indices: input.texture_indices,
    };
    // TODO: test with database?
    let _ = xc3_model::ModelRoot::from_mxmd_v112(&files, input.skel, None);
});
