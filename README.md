# xc3_lib
Rust libraries and tools for working with rendering related file formats for Xenoblade Chronicles 2 and Xenoblade Chronicles 3. Files from Xenoblade Chronicles 1 DE should work but are not well tested. 

See [Architecture](https://github.com/ScanMountGoat/xc3_lib/blob/main/ARCHITECTURE.md) for an overview of the various projects. Report any bugs or request new features in [issues](https://github.com/ScanMountGoat/xc3_lib/issues). Python bindings for xc3_model are available with [xc3_model_py](https://github.com/ScanMountGoat/xc3_model_py).

## Formats
| Format | File Paths | Description |
| --- | --- | --- |
| Mibl | `chr/tex/nx/*/*.wismt`, `monolib/shader/*.{witex,witx}` | textures |
| Msmd | `map/*.wismhd` | maps |
| Msrd | `chr/{ch,en,oj,wp}/*.wismt` | models, textures, shaders |
| Mxmd | `chr/{ch,en,oj,wp}/*.wimdo`, `monolib/shader/*.wimdo` | models, materials |
| Sar1 | `chr/{ch,en,oj,wp}/*.{chr,mot}` | skeletons, animations |
| Spch | `monolib/shader/*.wishp` | shaders |
| Xbc1 | *embedded in files* | zlib compressed data |

File formats and where to find them in a game dump are outlined above. Note that the same extension can be used for multiple formats. Some formats like `Msrd` contain files from other formats like `Spch` embedded as compressed `Xbc1` archives.

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
