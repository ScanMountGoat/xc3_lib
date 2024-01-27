# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

### 0.5.0 - 2024-01-27
### Fixed
* Fixed an issue where `Msrd::from_extracted_files` would sometimes incorrectly calculate streaming data offsets.
* Fixed an issue where some `.wilay` files would not properly rebuild all data when saving.

### Changed
* Improved accuracy of file rebuilding.
* Optimized glTF file sizes by only including referenced vertex buffers.
* Reduced memory usage and improved export times for glTF conversion.
* Changed loading functions to return an error instead of panic.
* `Msrd::from_extracted_files` now always packs `chr/tex/nx` textures into the model's `.wismt` streams. This avoids conflicts for shared `.wismt` texture files.
* Adjusted handling of xc3_tex `chr/tex/nx` parameter to match repacking changes.

### 0.4.1 - 2024-01-18
### Fixed
* Fixed an issue where texture dimensions were reported incorrectly for xc3_wgpu.

## 0.4.0 - 2024-01-17
### Added
* Added support for Xenoblade 3 `chr/tex/nx` textures for unpacking and packing Msrd files.
* Added rendering support for culling to xc3_wgpu.
* Added rendering support for object outlines to xc3_wgpu.
* Added support to xc3_tex for extracting images from `.wimdo` files to a folder.
* Added support to xc3_tex for replacing images in `.wimdo` files from a folder using the `edit-wimdo` command.

### Fixed
* Fixed an issue where high resolution textures weren't read properly from legacy wismt files.
* Fixed an issue where map textures did not always correctly load the base mip level.
* Fixed an issue where generated JSON shader database entries had incorrect ordering for maps.
* Fixed an issue where meshes past the base level of detail (LOD) would not use correct skin weights.

### Changed
* Renamed `write_to_file` methods to `save` for all relevant types.
* Improved accuracy of file rebuilding.
* Adjusted output of xc3_tex commands to display elapsed time and converted file count.
* Adjusted wilay saving in xc3_tex to use xbc1 compression if present in the original file.

## 0.3.0 - 2023-12-23
### Added
* Added `glsl-dependencies` command to xc3_shader for printing lines affecting a particular variable.
* Added support for legacy streaming data used for some Xenoblade 2 models.
* Added support for PC files like `.pcmdo` and `.pcsmt`.
* Added support for `LAGP` in `.wilay` files.
* Added `TextureUsage` enum, enabling more accurate texture assignments when missing shader database information.
* Added support to xc3_tex for extracting images from `.wilay` files to a folder.
* Added support to xc3_tex for replacing images in `.wilay` files from a folder using the `edit-wilay` command.

### Fixed
* Fixed an issue where material parameters were not annotated correctly in decompiled shaders.
* Fixed an issue where material parameters were not handled correctly in the shader JSON.
* Fixed an issue where some Xenoblade 3 models used incorrect vertex skinning weights.
* Fixed an issue where Xenoblade 1 DE and Xenoblade 2 models did not load the high resolution base mip level.
* Fixed an issue where map textures did not load the high resolution base mip level.
* Fixed an issue where some Xenoblade 3 DLC maps failed to load due to prop instance indexing issues.
* Fixed an issue where gltf export would fail if the destination folder did not exist.

### Changed
* Improved accuracy for file rebuilding.
* Combined Msrd extract methods into a single `Msrd::extract_files` method for better performance.

## 0.2.0 - 2023-11-22
### Added
* Added animation support to xc3_model.
* Added `xc3_lib::hash` module with useful in game hash and checksum implementations.
* Added support for `R4G4B4A4` textures.
* Added support for `BC6UFloat` textures.
* Added helper functions for extracting from archives and creating archives from data.
* Added support for additional vertex data types used in Xenoblade 1 DE.
* Added support for morph targets to glTF export.
* Added support for texture samplers to glTF export.
* Added support for exporting GLSL shader code from Nvsp in Spch to xc3_lib.

### Fixed
* Fixed some cubic (type 1) animations not using the correct bone list during playback.
* Fixed reading of morph target data for targets after the base target.
* Fixed an issue where some anims failed to load due to incorrectly reading game specific extra data.

### Changed
* Improved accuracy for file rebuilding.
* Reduced dependencies for various projects.
* Changed animation playback functions to take time in seconds to properly handle animation speed.
* Adjusted how Mibl alignment is handled to ensure the Mibl <-> DDS conversion is always 1:1.
* Adjusted glTF texture assignment to assume first texture is albedo by default.
* Switched to tracy for viewing traces.
* Adjusted decompiled shader annotation to include uniform buffers fields when possible.

## 0.1.0 - 2023-10-29
First release! 
