# xc3_lib
An experimental Rust library for reading rendering related file formats for Xenoblade Chronicles 3. Support for Xenoblade Chronicles 2 is still a WIP.

The initial focus is creating robust and readable parsing code for formats related to in game rendering. Writing support and higher level libraries may be added at a later date once the formats have been more thoroughly researched. Formats not directly related to rendering may be considered at a later date.

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
This library is still highly experimental. When adding this project to the Cargo.toml, specify a specific git revision or commit the Cargo.lock file to source control. This locks the version and avoids any breaking changes. The debug or JSON output has not stabilized and should not be assumed to be the same between commits.

`xc3_lib = { git = "https://github.com/ScanMountGoat/xc3_lib", rev = "commit_hash" }`  

## Building
After installing the [Rust toolchain](https://www.rust-lang.org/tools/install), run `cargo build --release` in the repository directory.
Running `cargo build` without the `--release` will result in faster compile times during development but dramatically worse runtime performance. The tools can be run using `cargo run --release -p <project> <args>`. xc3_tex uses [image_dds](https://github.com/ScanMountGoat/image_dds), which supports Windows x86, Linux x86, MacOS x86, and MacOS Apple Silicon due to using precompiled kernels for DDS encoding. Other projects should build on other platforms without issues.

## Documentation
The projects are not currently published to crates.io, so run `cargo doc -p xc3_lib --open` to generate and view the rustdoc output in the browser. Replace xc3_lib with the name of other packages to view the corresponding documentation. Contributors should see [Architecture](https://github.com/ScanMountGoat/xc3_lib/blob/main/ARCHITECTURE.md) and [Development](https://github.com/ScanMountGoat/xc3_lib/blob/main/DEVELOPMENT.md) for information.

## Tests
Unit tests and doc tests can be run using `cargo test`. 

Most of the file processing and conversion code is tested by running the xc3_test executable against a dump of the game extracted with [XbTool](https://github.com/AlexCSDev/XbTool/releases). Details for failed conversions will be printed to the console. File types can all be enabled at once or enabled individually.  
`cargo run -p xc3_test --release <path to xenoblade 2 or xenoblade 3 dump> --all`  
`cargo run -p xc3_test --release <path to xenoblade 2 or xenoblade 3 dump> --mxmd --mibl`

## Credits
This project makes use of a number of Rust crates that are useful for reverse engineering. For a full list of dependencies, see the Cargo.toml files.
* [binrw](https://github.com/jam1garner/binrw) - declarative binary parsing
* [tegra_swizzle](https://github.com/ScanMountGoat/tegra_swizzle) - efficient and robust Tegra X1 swizzling/deswizzling
* [image_dds](https://github.com/ScanMountGoat/image_dds) - encode/decode BCN image data

This project is based on previous reverse engineering work, including work done for Xenoblade 2.
Special thanks go to members of the World Tree Research discord (formerly the World of Alrest discord) for their assistance.
* [Xenoblade Data Hub](https://xenobladedata.github.io/)
* [XenoLib](https://github.com/PredatorCZ/XenoLib)
* [XB2AssetTool](https://github.com/BlockBuilder57/XB2AssetTool)
* [Xenoblade-Switch-Model-Importer-Noesis](https://github.com/Turk645/Xenoblade-Switch-Model-Importer-Noesis)
* [XbTool](https://github.com/AlexCSDev/XbTool)
