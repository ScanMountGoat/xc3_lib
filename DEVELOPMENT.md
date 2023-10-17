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

## CPU Profiling
For Linux and MacOS, an easy way to identify performance bottlenecks is by using [cargo-flamegraph](https://github.com/flamegraph-rs/flamegraph).
This tool is difficult to get working on Windows even with WSL. Windows users can install the latest version of Visual Studio and use its builtin performance profiler. 
Visual Studio can profile the generated Rust executable and even view the data as a flamegraph. 
Make sure to profile in release mode with debug info enabled by temporarily adding the following lines to the `Cargo.toml` in the root directory.
```toml
[profile.release]
debug = true
```

Some projects have [tracing](https://github.com/tokio-rs/tracing) support for profiling with `#[tracing::instrument]`. This can be used to generate spans, which are helpful for understanding the performance of multithreaded code. When compiling with `--features=tracing`, xc3_viewer can use `tracing-chrome` to generate JSON reports that are viewable using https://ui.perfetto.dev/.

## GPU Debugging
[RenderDoc](https://renderdoc.org/) is a powerful GPU debugging tool. 
RenderDoc can be used to debug rendering and graphics API usage issues not only in projects like xc3_wgpu and xc3_viewer but even the game itself running under an emulator like Ryujinx. 
This is especially helpful when identifying why the output of xc3_wgpu does not match up with in game by comparing the same model or scene in RenderDoc in the application and in game.

## GPU Profiling
GPU hardware manufacturers provide their own profiling tools that are more advanced than tools like RenderDoc or traditional FPS overlays. 
Examples include Nvidia Nsight Graphics, AMD Radeon GPU Profiler, or the Metal profiler and debugger in XCode.
These tools often assume advanced knowledge of modern graphics APIs and hardware capabilities. 
Consult the appropriate documentation for details and usage instructions. 

## Tests
Unit tests and doc tests can be run using `cargo test`. 

Most of the file processing and conversion code is tested by running the xc3_test executable against a dump of the game extracted with [XbTool](https://github.com/AlexCSDev/XbTool/releases). Details for failed conversions will be printed to the console. File types can all be enabled at once or enabled individually.  
`cargo run -p xc3_test --release <path to xenoblade 2 or xenoblade 3 dump> --all`  
`cargo run -p xc3_test --release <path to xenoblade 2 or xenoblade 3 dump> --mxmd --mibl`

The rendering can be tested by batch rendering files to PNG. This tests xc3_lib, xc3_wgpu, and xc3_model. Specifying the GBuffer JSON database from xc3_shader will allow xc3_wgpu to assign textures to the appropriate outputs.  
`cargo run -p xc3_wgpu_batch --release "xenoblade 2 dump/model/bl" wimdo`  
`cargo run -p xc3_wgpu_batch --release "xenoblade 3 dump/map" wismhd gbuffer.json`  

## In Game Testing
 Files can be extracted from a dump of the game by dumping the romfs from Yuzu or Ryujinx and then dumping the ard and arh archive files using [XbTool](https://github.com/AlexCSDev/XbTool/releases).
 
### Xenoblade Chronicles 2
Files can be loaded without repacking the ard and arh files by using a romfs mod with the DLC. This also includes the free language pack DLC. 
The files should match the folder structure of the archive and use the title ID of the DLC like `yuzu/load/modded files/model/bl/bl000101.arc`.

### Xenoblade Chronicles 3
The easiest way to test files is using an emulator like Ryujinx and the [xc3-file-loader](https://github.com/RoccoDev/xc3-file-loader) plugin for loading modded files.

## Code Generation
For seeing the generated code from procedural macros, use [cargo expand](https://github.com/dtolnay/cargo-expand). For example, call `cargo expand -p xc3_lib mxmd > expanded.rs` to output the expanded contents of `mxmd.rs`.

## Shader JSON Database
Multiple projects rely on a generated JSON databases of shader metadata to properly assign textures and material parameters. This database is specific to a particular game version like Xenoblade 3 or Xenoblade 1 DE. The first step is to decompile and annotate the shaders. This requires `Ryujinx.ShaderTools`, which can be compiled from source from [Ryujinx](https://github.com/Ryujinx/Ryujinx). Note that this may took up to 30 minutes depending on your system and the number of shaders to decompile. The final step is to convert the decompiled shaders into a JSON database. Example commands for Xenoblade 3 are listed below.  

`cargo run --release -p xc3_shader -- decompile-shaders "Xeno3 Dump" "xc3_shader_dump" Ryujinx.ShaderTools.exe`  
`cargo run --release -p xc3_shader -- g-buffer-database "xc3_shader_dump" xc3.json`

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
   let value = xc3_lib::mxmd::Mxmd::from_file("ch01012013.wimdo");
   println!("{:#?}", value);
}
```

## Debugging File Writing
The easiest way to test errors when writing a file is to parse a file and then write it again without making changes. This should result in a binary identical output file. This can be checked using a hex editor like [HxD](https://mh-nexus.de/en/hxd/) or [ImHex](https://github.com/WerWolv/ImHex) for visual diff checking. See xc3_test for Rust code examples.

Another useful test is to write the file to binary and then read it again. The two data structures like `Mxmd` or `Msrd` should compare as equal. Differences can indicate that data isn't being written properly. For more visual output, pretty print the debug representation before and after to text files using the `"{:#?}"` format specifier. The text can be diffed using an online diffing tool or directly in some editors like VsCode.
