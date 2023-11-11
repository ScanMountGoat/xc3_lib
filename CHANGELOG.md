# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## unreleased
### Added
* Added animation support to xc3_model.
* Added `xc3_lib::hash` module with useful in game hash and checksum implementations.
* Added support for `R4G4B4A4` textures.
* Added support for `BC6UFloat` textures.
* Added helper functions for extracting from archives and creating archives from data.
* Added support for additional vertex data types used in Xenoblade 1 DE.
* Added support for morph targets to glTF export.
* Added support for texture samplers to glTF export.

### Fixed
* Fixed some cubic (type 1) animations not using the correct bone list during playback.
* Fixed reading of morph target data for targets after the base target.
* Fixed an issue where some anims failed to load due to incorrectly reading game specific extra data.

### Changed
* Improved accuracy for file rebuilding.
* Reduced dependencies for various projects.
* Changed animation playback functions to take time in seconds to properly handle animation speed.
* Adjusted how MIBl alignment is handled to ensure the MIBL <-> DDS conversion is always 1:1.
* Adjusted glTF texture assignment to assume first texture is albedo by default.

## 0.1.0 - 2023-10-29
First release! 