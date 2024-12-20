use wgsl_to_wgpu::{create_shader_module_embedded, MatrixVectorTypes, WriteOptions};

fn main() {
    let out_dir = std::env::var("OUT_DIR").unwrap();
    write_shader(
        include_str!("src/shader/blit.wgsl"),
        "src/shader/blit.wgsl",
        format!("{out_dir}/blit.rs"),
    );
    write_shader(
        include_str!("src/shader/bone.wgsl"),
        "src/shader/bone.wgsl",
        format!("{out_dir}/bone.rs"),
    );
    write_shader(
        include_str!("src/shader/collision.wgsl"),
        "src/shader/collision.wgsl",
        format!("{out_dir}/collision.rs"),
    );
    write_shader(
        include_str!("src/shader/deferred.wgsl"),
        "src/shader/deferred.wgsl",
        format!("{out_dir}/deferred.rs"),
    );
    write_shader(
        include_str!("src/shader/model.wgsl"),
        "src/shader/model.wgsl",
        format!("{out_dir}/model.rs"),
    );
    write_shader(
        include_str!("src/shader/morph.wgsl"),
        "src/shader/morph.wgsl",
        format!("{out_dir}/morph.rs"),
    );
    write_shader(
        include_str!("src/shader/snn_filter.wgsl"),
        "src/shader/snn_filter.wgsl",
        format!("{out_dir}/snn_filter.rs"),
    );
    write_shader(
        include_str!("src/shader/solid.wgsl"),
        "src/shader/solid.wgsl",
        format!("{out_dir}/solid.rs"),
    );
    write_shader(
        include_str!("src/shader/unbranch_to_depth.wgsl"),
        "src/shader/unbranch_to_depth.wgsl",
        format!("{out_dir}/unbranch_to_depth.rs"),
    );
}

fn write_shader(wgsl_source: &str, wgsl_path: &str, output_path: String) {
    println!("cargo:rerun-if-changed={wgsl_path}");

    // Generate the Rust bindings and write to a file.
    let text = create_shader_module_embedded(
        wgsl_source,
        WriteOptions {
            derive_bytemuck_vertex: true,
            derive_encase_host_shareable: true,
            matrix_vector_types: MatrixVectorTypes::Glam,
            ..Default::default()
        },
    )
    .unwrap();

    std::fs::write(output_path, text.as_bytes()).unwrap();
}
