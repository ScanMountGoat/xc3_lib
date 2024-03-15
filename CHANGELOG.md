# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.7.0 - 2024-03-15
### Added
* Added support for anisotropic filtering to xc3_wgpu.
* Added support for `monolib/shader` textures to xc3_wgpu.
* Added options to set the mipmaps and quality when generating compressed files to xc3_tex.

### Changed
* Improved accuracy of file rebuilding.
* Improved the readability and quality of displayed error messages for xc3_viewer, xc3_gltf, and xc3_tex.
* Renamed `GBuffer*` types and methods to `Output*` since not all shaders write to the G-Buffer textures.
* Renamed `ShaderProgram ` to `MaterialTechnique` and `ShaderProgramInfo` to `Technique` to better match in game names.
* Reworked render pass assignments in xc3_wgpu to better match in game. This improves rendering accuracy of transparent meshes.
* Optimized rendering performance for xc3_wgpu using frustum culling.

### Fixed
* Fixed an issue where compressed wilay files failed to extract or convert with xc3_tex.
* Fixed an issue where some `.wilay` LAGP files would not properly rebuild all data when saving.

## 0.6.0 - 2024-02-16
### Added
* Added derives for `Clone` and `PartialEq` for xc3_lib types.
* Added derive for `Arbitrary` to xc3_lib and xc3_model types to facilitate fuzz testing.
* Added `ModelRoot::to_mxmd_model` for applying edits to the original `.wimdo` and `.wismt` files.
* Added `ModelBuffers::from_vertex_data` and `ModelBuffers::to_vertex_data` to xc3_model for converting to and from xc3_lib.
* Added rendering support for stencil flags to xc3_wgpu, improving sorting accuracy of eyelashes and eyebrows.
* Added support for DLC models for Xenoblade 2 and Xenoblade 3 for the provided shader JSON databases.
* Added support for texcoord scale to glTF export via the `KHR_texture_transform` extension. This does not yet support normal and AO due to limitations in the gltf crate.

### Fixed
* Fixed an issue where not all morph targets were being read.
* Fixed various issues related to loading DLC models and maps for Xenoblade 2 and Xenoblade 3.
* Fixed an issue where unused alpha channels in glTF diffuse textures would cause black renders in some applications.
* Fixed an issue where the final field in a uniform buffer struct was not annotated correctly for xc3_shader.

### Changed
* Improved rendering accuracy of toon shading pass.
* Improved accuracy of hair shading pass and added SNN blur kernel to Xenoblade 3 hair.
* Moved `Skeleton` field for xc3_model from `Models` to `ModelRoot` to better reflect in game data.
* Moved `update_bone_transforms` method for xc3_wgpu to `ModelGroup` to better reflect in game data.
* Adjusted `ModelBuffers` type for xc3_model to better reflect in game data.
* Adjusted `Xc3Renderer` constructor to take a parameter for the `monolib/shader` folder to load game specific global textures.
* Increased resolution from 512x512 to 1024x1024 for xc3_wgpu_batch PNG files.
* Adjusted state flags for mxmd materials.
* Appended program name to file names of extracted shaders if present for xc3_shader decompile-shaders.
* Optimized the JSON representation of `ShaderDatabase` to reduce size and enable more features in the future. The types for the JSON representation are not public and should be treated as an implementation detail. See the private structs in the source code for xc3_model for details.
* Improved accuracy of texture assignments for glTF export when not using a shader JSON database.
* Moved glTF export support to an optional "gltf" feature for xc3_model.
* Adjusted output file names for xc3_tex when extracting `.wimdo` textures to include the texture's name.

### Removed
* Removed `read_index_buffers`, `read_vertex_buffers`, `read_vertex_attributes`, and `read_index_buffers` from xc3_model. Use `ModelBuffers::from_vertex_data` instead.
* Removed serialize/deserialize support from the shader database types in xc3_model. Use `ShaderDatabase::from_file` and `ShaderDatabase::save` instead.

## 0.5.0 - 2024-01-27
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

## 0.4.1 - 2024-01-18
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
