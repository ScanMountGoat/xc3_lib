# xc3_lib
Rust libraries and tools for working with rendering related file formats for Xenoblade Chronicles 2 and Xenoblade Chronicles 3. Files from Xenoblade Chronicles 1 DE should work but are not well tested. 

See [Architecture](https://github.com/ScanMountGoat/xc3_lib/blob/main/ARCHITECTURE.md) for an overview of the various projects. Report any bugs or request new features in [issues](https://github.com/ScanMountGoat/xc3_lib/issues). Python bindings for xc3_model are available with [xc3_model_py](https://github.com/ScanMountGoat/xc3_model_py).

## Formats
xc3_lib supports a number of in game formats. All formats support reading. Write support is still a WIP for some formats. Click on the links to open the corresponding Rust module in xc3_lib.

| Format | Magic | Extensions | Read | Write |
| --- | --- | --- | --- | --- |
| [Apmd](https://github.com/ScanMountGoat/xc3_lib/blob/main/xc3_lib/src/apmd.rs) | DMPA | `wimdo` | ✔️ | ✔️ | 
| [Dhal](https://github.com/ScanMountGoat/xc3_lib/blob/main/xc3_lib/src/dhal.rs) | LAHD | `wilay` | ✔️ | ❌ | 
| [Ltpc](https://github.com/ScanMountGoat/xc3_lib/blob/main/xc3_lib/src/ltpc.rs) | LTPC |  | ✔️ | ✔️ | 
| [Mibl](https://github.com/ScanMountGoat/xc3_lib/blob/main/xc3_lib/src/mibl.rs) | LBIM | `wismt`, `witex`, `witx` | ✔️ | ✔️ | 
| [Msmd](https://github.com/ScanMountGoat/xc3_lib/blob/main/xc3_lib/src/msmd.rs) | DMSM | `wismhd` | ✔️ | ❌ | 
| [Msrd](https://github.com/ScanMountGoat/xc3_lib/blob/main/xc3_lib/src/msrd.rs) | DRSM |  `wismt` | ✔️ | ✔️* |
| [Mxmd](https://github.com/ScanMountGoat/xc3_lib/blob/main/xc3_lib/src/mxmd.rs) | DMXM | `wimdo` | ✔️ | ✔️* | 
| [Sar1](https://github.com/ScanMountGoat/xc3_lib/blob/main/xc3_lib/src/sar1.rs) | 1RAS | `chr`, `mot` | ✔️ | ✔️ | 
| [Spch](https://github.com/ScanMountGoat/xc3_lib/blob/main/xc3_lib/src/spch.rs) | HCPS | `wishp` | ✔️ | ✔️ | 
| [Xbc1](https://github.com/ScanMountGoat/xc3_lib/blob/main/xc3_lib/src/xbc1.rs) | xbc1 | `wismt` | ✔️ | ✔️ | 

\* *Some files are not binary identical with the originals after saving.*

## Usage
These projects are still highly unstable. When adding any of these projects to the Cargo.toml, specify a specific git revision or commit the Cargo.lock file to source control. This locks the version and avoids any breaking changes. The debug or JSON output has not stabilized and should not be assumed to be the same between commits.

```toml
xc3_model = { git = "https://github.com/ScanMountGoat/xc3_lib", rev = "commit_hash" }
xc3_wgpu = { git = "https://github.com/ScanMountGoat/xc3_lib", rev = "commit_hash" }
xc3_lib = { git = "https://github.com/ScanMountGoat/xc3_lib", rev = "commit_hash" }
```

## Building
After installing the [Rust toolchain](https://www.rust-lang.org/tools/install), run `cargo build --release` in the repository directory.
Running `cargo build` without the `--release` will result in faster compile times during development but dramatically worse runtime performance. The tools can be run using `cargo run --release -p <project> <args>`. xc3_tex uses [image_dds](https://github.com/ScanMountGoat/image_dds), which supports Windows x86, Linux x86, MacOS x86, and MacOS Apple Silicon due to using precompiled kernels for DDS encoding. Other projects should build on other platforms without issues.

## Documentation
The projects are not currently published to crates.io, so run `cargo doc -p xc3_lib --open` to generate and view the rustdoc output in the browser. Replace xc3_lib with the name of other packages to view the corresponding documentation. Contributors should see [Architecture](https://github.com/ScanMountGoat/xc3_lib/blob/main/ARCHITECTURE.md) and [Development](https://github.com/ScanMountGoat/xc3_lib/blob/main/DEVELOPMENT.md) for information.

## Credits
This project is based on previous reverse engineering work, including work done for Xenoblade 2.
Special thanks go to members of the World Tree Research discord (formerly the World of Alrest discord) for their assistance.
* [Xenoblade Data Hub](https://xenobladedata.github.io/)
* [xc2f wiki](https://github.com/atnavon/xc2f/wiki)
* [Xenoblade-Switch-Model-Importer-Noesis](https://github.com/Turk645/Xenoblade-Switch-Model-Importer-Noesis)
* [fmt_xc3](https://github.com/Joschuka/fmt_xc3)
* [XbTool](https://github.com/AlexCSDev/XbTool)
* [XenoLib](https://github.com/PredatorCZ/XenoLib)
