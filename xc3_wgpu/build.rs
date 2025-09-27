use wesl::{Mangler, Wesl};
use wgsl_to_wgpu::{MatrixVectorTypes, Module, ModulePath, WriteOptions};

fn main() {
    let out_dir = std::env::var("OUT_DIR").unwrap();

    let mut wesl = Wesl::new("src/shader");
    wesl.set_mangler(wesl::ManglerKind::Escape);
    wesl.set_options(wesl::CompileOptions {
        mangle_root: false,
        strip: false,
        ..Default::default()
    });

    let options = WriteOptions {
        derive_bytemuck_vertex: true,
        derive_encase_host_shareable: true,
        matrix_vector_types: MatrixVectorTypes::Glam,
        ..Default::default()
    };
    let mut module = Module::default();

    for name in [
        "blit",
        "bone",
        "collision",
        "deferred",
        "model",
        "morph",
        "snn_filter",
        "solid",
        "unbranch_to_depth",
    ] {
        println!("cargo:rerun-if-changed=src/shader/{name}.wgsl");

        let wgsl = wesl
            .compile(&wesl::ModulePath::from_path(format!("/{name}.wgsl")))
            .unwrap()
            .to_string();
        module
            .add_shader_module(
                &wgsl,
                None,
                options,
                ModulePath {
                    components: vec![name.to_string()],
                },
                demangle_wesl,
            )
            .unwrap();
    }

    let text = module.to_generated_bindings(options);
    std::fs::write(format!("{out_dir}/shader.rs"), &text).unwrap();
}

fn demangle_wesl(name: &str) -> wgsl_to_wgpu::TypePath {
    // Assume all paths are absolute paths.
    // TODO: Detect if mangling is necessary without relying on implementation details?
    if name.starts_with("package_") {
        let mangler = wesl::EscapeMangler;
        let (path, name) = mangler
            .unmangle(name)
            .unwrap_or((wesl::ModulePath::new_root(), name.to_string()));

        // Assume all wesl paths are absolute paths.
        wgsl_to_wgpu::TypePath {
            parent: wgsl_to_wgpu::ModulePath {
                components: path.components,
            },
            name,
        }
    } else {
        wgsl_to_wgpu::TypePath {
            parent: wgsl_to_wgpu::ModulePath::default(),
            name: name.to_string(),
        }
    }
}
