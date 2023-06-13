# Development
This document provides tips and guidelines for working on the various projects for xc3_lib.

## Editors
The first step is to [install Rust](https://www.rust-lang.org/tools/install). 
Commands for building, running, and testing code all use `cargo` terminal commands like `cargo build`, `cargo run`, and `cargo test`.
A way to edit text and run commands from terminal are all that are technically required to develop Rust code. 
It's recommended to use an editor like [Visual Studio Code](https://code.visualstudio.com/) with the [rust-analyzer](https://rust-analyzer.github.io/) language plugin.
`rust-analyzer` is also available for other editors for those that prefer to use something other than VSCode.

## Style and Formatting
Rust code should be formatted by running the cargo fmt command. 
This can also be done in VS Code using the Rust Analyzer extension and using the format document command (Alt+Shift+F). 
Running code lints with cargo clippy is also recommended. 
Running formatting and linting regularly during development helps keep the code easy to read for other Rust developers.

## CPU Profiling
For Linux and MacOS, an easy way to identify performance bottlenecks is by using [cargo-flamegraph](https://github.com/flamegraph-rs/flamegraph).
This tool is difficult to get working on Windows even with WSL. Windows users can install the latest version of Visual Studio and use its builtin performance profiler. 
Visual Studio can profile the generated Rust executable and even view the data as a flamegraph. 
Make sure to profile in release mode with debug info enabled by temporarily adding the following lines to the `Cargo.toml` in the root directory.
```toml
[profile.release]
debug = true
```

## GPU Debugging
[RenderDoc](https://renderdoc.org/) is a powerful GPU debugging tool. 
RenderDoc can be used to debug rendering and graphics API usage issues not only in projects like xc3_wgpu and xc3_viewer but even the game itself running under an emulator like Ryujinx. 
This is especially helpful when identifying why the output of xc3_wgpu does not match up with in game by comparing the same model or scene in RenderDoc in the application and in game.

## GPU Profiling
GPU hardware manufacturers provide their own profiling tools that are more advanced than tools like RenderDoc or traditional FPS overlays. 
Examples include Nvidia Nsight Graphics, AMD Radeon GPU Profiler, or the Metal profiler and debugger in XCode.
These tools often assume advanced knowledge of modern graphics APIs and hardware capabilities. 
Consult the appropriate documentation for details and usage instructions. 

## In Game Testing
The easiest way to test files is using an emulator like Ryujinx and the [xc3-file-loader](https://github.com/RoccoDev/xc3-file-loader) plugin for loading modded files.

## Debugging File Parsing - TODO
