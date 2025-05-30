# Development
This document provides tips and guidelines for working on the various projects for xc3_lib.

## Editors
The first step is to [install Rust](https://www.rust-lang.org/tools/install). 
Commands for building, running, and testing code all use `cargo` terminal commands like `cargo build`, `cargo run`, and `cargo test`.
A way to edit text and run commands from terminal are all that are technically required to develop Rust code. 
It's recommended to use an editor like [Visual Studio Code](https://code.visualstudio.com/) with the [rust-analyzer](https://rust-analyzer.github.io/) language plugin.
`rust-analyzer` is also available for other editors for those that prefer to use something other than VSCode.

## Style and Formatting
Rust code should be formatted by running the cargo fmt command. This can also be done in VS Code using the Rust Analyzer extension and using the format document command (Alt+Shift+F). Running code lints with cargo clippy is also recommended. Running formatting and linting regularly during development helps keep the code easy to read for other Rust developers. WGSL shader files should be formatted using [wgsl_analyzer](https://github.com/wgsl-analyzer/wgsl-analyzer).

## Tests
Unit tests and doc tests can be run using `cargo test`.

Fuzz testing uses [cargo-fuzz](https://rust-fuzz.github.io/book/cargo-fuzz.html) and requires switching to the nightly toolchain on Linux or MacOS using `rustup default nightly`. Windows users can run the tests in Linux via Windows Subsystem for Linux (WSL). Fuzz tests should be run from within the specific project directory. Fuzz tests run forever, so stop the test manually using Ctrl+C.

```
cd xc3_model
cargo fuzz run from_mxmd_model
```

Most of the file processing and conversion code is tested by running the xc3_test executable against an extracted game dump. Details for failed conversions will be printed to the console. File types can all be enabled at once or enabled individually.  
`cargo run -p xc3_test --release <path to game dump> --all`  
`cargo run -p xc3_test --release <path to game dump> --mxmd --mibl`

The rendering can be tested by batch rendering files to PNG. This tests xc3_lib, xc3_wgpu, and xc3_model. Specifying the shader database from xc3_shader will allow xc3_wgpu to assign textures to the appropriate outputs.  
`cargo run -p xc3_wgpu_batch --release "root/model/bl" wimdo xc2.bin`  
`cargo run -p xc3_wgpu_batch --release "root/map" wismhd xc3.bin`  

Shader database tests use [insta](https://crates.io/crates/insta) and [cargo-insta](https://crates.io/crates/cargo-insta) for snapshot testing. Tests write shader types to a custom string format stored in `.snap` files. Running tests with `cargo test` will generate `.snap.new` files with any changes. Review changes with `cargo insta review`. Accept changes with `cargo insta accept`.

## CPU Profiling
For Linux and MacOS, an easy way to identify performance bottlenecks is by using [cargo-flamegraph](https://github.com/flamegraph-rs/flamegraph) or [samply](https://github.com/mstange/samply). Windows users can install the latest version of Visual Studio and use its builtin performance profiler. Visual Studio can profile the generated Rust executable and even view the data as a flamegraph. Make sure to profile in release mode with debug info enabled by temporarily adding the following lines to the `Cargo.toml` in the root directory.  

```toml
[profile.release]
debug = true
```

Some projects have [tracing](https://github.com/tokio-rs/tracing) support for profiling with `#[tracing::instrument]`. This can be used to generate spans, which are helpful for understanding the performance of multithreaded code. When compiling with `--features=tracing`, xc3_viewer can use `tracing-tracy` to connect to the profiling tool and trace viewer [tracy](https://github.com/wolfpld/tracy/releases/tag/v0.10).

## GPU Debugging
[RenderDoc](https://renderdoc.org/) is a powerful GPU debugging tool for Windows and Linux. 
RenderDoc can be used to debug rendering and graphics API usage issues not only in projects like xc3_wgpu and xc3_viewer but even the game itself running under an emulator like Ryujinx. This is especially helpful when identifying why the output of xc3_wgpu does not match up with in game by comparing the same model or scene in RenderDoc in the application and in game. Make sure to compile xc3_viewer in debug mode to enable labels for GPU resources like textures or render passes in RenderDoc.

Use the Vulkan API for best compatibility with RenderDoc with emulators. Using OpenGL will require compiling a custom build that disables unsupported OpenGL functionality. RenderDoc may not always connect properly to Ryujinx. Compiling Ryujinx.Gtk3 from source tends to be more reliable at least on Windows. For Cemu, use the Vulkan API and edit shaders by selecting "Decompile with SPIRV-Cross" in the edit drop down in RenderDoc.

MacOS users should use the GPU debugging capabilities built into XCode.

## GPU Profiling
GPU hardware manufacturers provide their own profiling tools that are more advanced than tools like RenderDoc or traditional FPS overlays. 
Examples include Nvidia Nsight Graphics, AMD Radeon GPU Profiler, or the Metal profiler and debugger in XCode.
These tools often assume advanced knowledge of modern graphics APIs and hardware capabilities. 
Consult the appropriate documentation for details and usage instructions. 

## Documentation
Documentation is generated by rustdoc. Simply add markdown comments to public types and fields following the existing examples. View the documentation locally in a browser by specifying the project name like `cargo doc -p xc3_lib --open`. After making changes, run the command without the `--open` flag and refresh the browser page.

## Dumping Game Files
Dump the base game, updates, and DLC from a modded Switch and a personal copy of the game. Install the files to nand and extract the romfs individually for the base game and all DLC. Ryujinx can also extract the romfs but does not support dumping DLC or update content. Extract all the arh and ard files to the same folder using [XbTool](https://github.com/AlexCSDev/XbTool/releases).

## In Game Testing
Files can be extracted from a dump of the game by dumping the romfs from Ryujinx and then dumping the ard and arh archive files using [XbTool](https://github.com/AlexCSDev/XbTool/releases).
 
### Xenoblade Chronicles 2
Files can be loaded without repacking the ard and arh files by using a romfs mod with the DLC. This also includes the free language pack DLC. The files should match the folder structure of the archive and use the title ID of the DLC. Example paths for the free Japanese voice DLC (0100E95004039063) for the US version of the game are listed below.  
`Ryujinx/mods/contents/0100E95004039063/romfs/monolib/shader/lib_nx.ini`  

### Xenoblade Chronicles 3
The easiest way to test files is using an emulator like Ryujinx and the [xc3-file-loader](https://github.com/RoccoDev/xc3-file-loader) plugin for loading modded files.

## Code Generation
For seeing the generated code from procedural macros, use [cargo expand](https://github.com/dtolnay/cargo-expand). For example, call `cargo expand -p xc3_lib mxmd > expanded.rs` to output the expanded contents of `mxmd.rs`.

## Shader Database
Multiple projects rely on a generated databases of shader metadata to properly assign textures and material parameters. This database is specific to a particular game version like Xenoblade 3 or Xenoblade 1 DE. The first step is to decompile and annotate the shaders. This requires `Ryujinx.ShaderTools`, which can be compiled from source from [Ryujinx](https://github.com/Ryujinx/Ryujinx). Note that this may took up to 30 minutes depending on your system and the number of shaders to decompile. The final step is to convert the decompiled shaders into a database. Example commands for Xenoblade 3 are listed below.  

`cargo run --release -p xc3_shader -- decompile-shaders "Xeno3 Dump" "xc3_shader_dump" Ryujinx.ShaderTools.exe`  
`cargo run --release -p xc3_shader -- shader-database "xc3_shader_dump" xc3.bin`

## Debugging File Parsing
The easiest way to test file parsing is by running xc3_test on an extracted game dump and noting any errors printed to the console. The `binrw` library used to generate parsing code also supports debugging the location and values of specific fields by adding the `#[br(dbg)]` attribute like in the example below. This makes it easy to identify the offset to use when hex editing for in game tests.

```rust
#[binread]
#[derive(Debug)]
pub struct Data {
    // Prints offset and value for field1 to stderr when parsing
    #[br(dbg)]
    field1: u32
}
```

Values can also be pretty printed using the appropriate debug format specifier. The output will look similar to Rust syntax.

```rust
fn main() {
    let value = xc3_lib::mxmd::Mxmd::from_file("ch01012013.wimdo").unwrap();;
    println!("{:#?}", value);
}
```

## Debugging File Writing
The easiest way to test errors when writing a file is to parse a file and then write it again without making changes. This should result in a binary identical output file. This can be checked using a hex editor like [HxD](https://mh-nexus.de/en/hxd/) or [ImHex](https://github.com/WerWolv/ImHex) for visual diff checking. See xc3_test for Rust code examples.

Another useful test is to write the file to binary and then read it again. The two file structs should compare as equal. Differences can indicate that data isn't being written properly. For more visual output, pretty print the debug representation before and after to text files using the `"{:#?}"` format specifier. The text can be diffed using an online diffing tool or directly in some editors like Visual Studio Code.

```rust
fn main() {
    let value = xc3_lib::mxmd::Mxmd::from_file("ch01012013.wimdo").unwrap();;
    std::fs::write!("mxmd.txt", format!("{:#?}", value)).unwrap();
    value.save("ch01012013.out.wimdo").unwrap();

    let new_value = xc3_lib::mxmd::Mxmd::from_file("ch01012013.out.wimdo").unwrap();
    std::fs::write!("mxmd.out.txt", format!("{:#?}", new_value)).unwrap();
}
```
