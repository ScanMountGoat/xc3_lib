# xc3_lib [![GitHub release (latest by date including pre-releases)](https://img.shields.io/github/v/release/ScanMountGoat/xc3_lib?include_prereleases)](https://github.com/ScanMountGoat/xc3_lib/releases/latest)
Rust libraries and tools for working with rendering related file formats for Xenoblade Chronicles X, Xenoblade Chronicles 1 Definitive Edition, Xenoblade Chronicles 2, Xenoblade Chronicles 3, and Xenoblade Chronicles X Definitive Edition.

Report any bugs or request new features in [issues](https://github.com/ScanMountGoat/xc3_lib/issues). Download precompiled binaries for the tools in [releases](https://github.com/ScanMountGoat/xc3_lib/releases). Python bindings for xc3_model are available with [xc3_model_py](https://github.com/ScanMountGoat/xc3_model_py). See the [xenoblade rendering research website](https://scanmountgoat.github.io/xenoblade-rendering-research/) for information on topics related to in game rendering.

## Formats
xc3_lib supports a number of in game formats. All formats support reading. Write support is still a WIP for some formats. Click on the links to open the corresponding Rust module in xc3_lib. Extensions starting with `wi` are for the Switch like `wimdo` or `wismt`. Extensions starting with `pc` are for PC builds like `pcmdo` or `pcsmt`. Extensions starting with `ca` are for the Wii U like `camdo` or `casmt`.

| Format | Magic | Extensions | Write |
| --- | --- | --- | --- |
| [Apmd](https://github.com/ScanMountGoat/xc3_lib/blob/main/xc3_lib/src/apmd.rs) | DMPA | `wimdo` | ✔️ | 
| [Bc](https://github.com/ScanMountGoat/xc3_lib/blob/main/xc3_lib/src/bc.rs) | BC | `anm`, `motstm_data` |  ✔️* |
| [Beb](https://github.com/ScanMountGoat/xc3_lib/blob/main/xc3_lib/src/beb.rs) | | `beb` |  ✔️ | 
| [Beh](https://github.com/ScanMountGoat/xc3_lib/blob/main/xc3_lib/src/beh.rs) | hdev | `beh` |  ❌ | 
| [Bmn](https://github.com/ScanMountGoat/xc3_lib/blob/main/xc3_lib/src/bmn.rs) | BMN | `bmn` | ❌ | 
| [Dhal](https://github.com/ScanMountGoat/xc3_lib/blob/main/xc3_lib/src/dhal.rs) | LAHD | `wilay` | ✔️* | 
| [Eva](https://github.com/ScanMountGoat/xc3_lib/blob/main/xc3_lib/src/eva.rs) | eva | `eva` | ✔️* | 
| [Fnt](https://github.com/ScanMountGoat/xc3_lib/blob/main/xc3_lib/src/fnt.rs) | | `fnt` | ✔️ | 
| [Idcm](https://github.com/ScanMountGoat/xc3_lib/blob/main/xc3_lib/src/idcm.rs) | IDCM | `wiidcm`, `idcm` | ✔️* | 
| [Laft](https://github.com/ScanMountGoat/xc3_lib/blob/main/xc3_lib/src/laft.rs) | LAFT | `wifnt` | ✔️ | 
| [Lagp](https://github.com/ScanMountGoat/xc3_lib/blob/main/xc3_lib/src/lagp.rs) | LAGP | `wilay` | ✔️* | 
| [Laps](https://github.com/ScanMountGoat/xc3_lib/blob/main/xc3_lib/src/laps.rs) | LAPS | `wilay` | ✔️* | 
| [Last](https://github.com/ScanMountGoat/xc3_lib/blob/main/xc3_lib/src/last.rs) | LAST | `wisty` | ✔️ | 
| [Ltpc](https://github.com/ScanMountGoat/xc3_lib/blob/main/xc3_lib/src/ltpc.rs) | LTPC | | ✔️ | 
| [Mibl](https://github.com/ScanMountGoat/xc3_lib/blob/main/xc3_lib/src/mibl.rs) | LBIM | `witex`, `witx` | ✔️ | 
| [Msmd](https://github.com/ScanMountGoat/xc3_lib/blob/main/xc3_lib/src/msmd.rs) | DMSM | `wismhd` | ❌ | 
| [Msrd](https://github.com/ScanMountGoat/xc3_lib/blob/main/xc3_lib/src/msrd.rs) | DRSM |  `wismt` | ✔️* |
| [Mtxt](https://github.com/ScanMountGoat/xc3_lib/blob/main/xc3_lib/src/mtxt.rs) | MTXT | `catex`, `calut`, `caavp` | ✔️ | 
| [Mxmd](https://github.com/ScanMountGoat/xc3_lib/blob/main/xc3_lib/src/mxmd.rs) | DMXM | `wimdo` | ✔️* | 
| [MxmdLegacy](https://github.com/ScanMountGoat/xc3_lib/blob/main/xc3_lib/src/mxmd/legacy.rs) | MXMD | `camdo` | ❌ | 
| [Sar1](https://github.com/ScanMountGoat/xc3_lib/blob/main/xc3_lib/src/sar1.rs) | 1RAS | `arc`, `chr`, `mot` | ✔️ | 
| [Spch](https://github.com/ScanMountGoat/xc3_lib/blob/main/xc3_lib/src/spch.rs) | HCPS | `wishp` | ✔️ | 
| [Xbc1](https://github.com/ScanMountGoat/xc3_lib/blob/main/xc3_lib/src/xbc1.rs) | xbc1 | `wismt` | ✔️ | 

\* *Some files are not binary identical with the originals after saving.*

## Projects
See [Architecture](https://github.com/ScanMountGoat/xc3_lib/blob/main/ARCHITECTURE.md) for a design overview of the various projects. 
Click on the docs.rs links below to see the generated rustdoc documentation.

### Libraries
- [![Crates.io](https://img.shields.io/crates/v/xc3_lib.svg?label=xc3_lib)](https://crates.io/crates/xc3_lib) [![docs.rs](https://docs.rs/xc3_lib/badge.svg)](https://docs.rs/xc3_lib/) - file format library
- [![Crates.io](https://img.shields.io/crates/v/xc3_model.svg?label=xc3_model)](https://crates.io/crates/xc3_model) [![docs.rs](https://docs.rs/xc3_model/badge.svg)](https://docs.rs/xc3_model/) - higher level API for xc3_lib
- [![Crates.io](https://img.shields.io/crates/v/xc3_wgpu.svg?label=xc3_wgpu)](https://crates.io/crates/xc3_wgpu) [![docs.rs](https://docs.rs/xc3_wgpu/badge.svg)](https://docs.rs/xc3_wgpu/) - model and map renderer
- [![Crates.io](https://img.shields.io/crates/v/xc3_write.svg?label=xc3_write)](https://crates.io/crates/xc3_write) [![docs.rs](https://docs.rs/xc3_write/badge.svg)](https://docs.rs/xc3_write/) - binary writing and layout

### Binaries
- [xc3_gltf](https://github.com/ScanMountGoat/xc3_lib/tree/main/xc3_gltf) - convert models and maps to glTF
- [xc3_test](https://github.com/ScanMountGoat/xc3_lib/tree/main/xc3_test) - test against files in an extracted dump
- [xc3_tex](https://github.com/ScanMountGoat/xc3_lib/tree/main/xc3_tex) - convert textures to and from common formats and replace textures in `wilay` and `wimdo` files
- [xc3_viewer](https://github.com/ScanMountGoat/xc3_lib/tree/main/xc3_viewer) - simple model viewer for testing `xc3_wgpu`
- [xc3_wgpu_batch](https://github.com/ScanMountGoat/xc3_lib/tree/main/xc3_wgpu_batch) - batch render models and maps to PNG

## Building
After installing the [Rust toolchain](https://www.rust-lang.org/tools/install), run `cargo build --release` in the repository directory to build the tools to `target/release`.
Running `cargo build` without the `--release` will result in faster compile times during development but dramatically worse runtime performance. The tools can also be run using `cargo run --release -p <project> <args>`. xc3_tex uses [image_dds](https://github.com/ScanMountGoat/image_dds), which supports Windows x86, Linux x86, MacOS x86, and MacOS Apple Silicon due to using precompiled kernels for DDS encoding. Other projects should build on other platforms without issues.

## Credits
This project is based on previous reverse engineering work, including work done for Xenoblade X and Xenoblade 2.
Special thanks go to members of the World Tree Research discord (formerly the World of Alrest discord) for their assistance.
* [xc2f wiki](https://github.com/atnavon/xc2f/wiki)
* [Xenoblade-Switch-Model-Importer-Noesis](https://github.com/Turk645/Xenoblade-Switch-Model-Importer-Noesis)
* [fmt_xc3](https://github.com/Joschuka/fmt_xc3)
* [XbTool](https://github.com/AlexCSDev/XbTool)
* [SimpleDimple](https://github.com/modeco80/SimpleDimple)
* [xc3_data_sheet.py](https://gist.github.com/RoccoDev/935c43c98384115f56dd3af57667b030)
* [XenoLib](https://github.com/PredatorCZ/XenoLib)
* [HavokLib](https://github.com/PredatorCZ/HavokLib)
* [Nintendo-File-Formats](https://github.com/kinnay/Nintendo-File-Formats)
* [wut](https://github.com/devkitPro/wut)
* [Xenoblade Data Hub](https://xenobladedata.github.io/)
