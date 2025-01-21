# Architecture
This document is intended to help get familiar with the layout of the codebase as well as the function of its various projects.

## Overview
File processing logic is split up into a number of projects to better serve the needs of consuming libraries, plugins, and applications. The commandline tool xc3_test doesn't need 3D graphics capabilities from xc3_wgpu, for example. 

File data starts as an unstructured array of bytes in one or more binary files. Each project applies some amount of processing and converts the data to a new form. The basic process is outlined below for a few example workflows.

**Model Rendering**
1. Parse files (xc3_lib).
2. Decompress, decode, and convert models and textures to a standardized format (xc3_model).
3. Convert the xc3_model data to renderable buffers, textures, pipelines etc (xc3_wgpu).
4. Initialize an `Xc3Renderer`, load the models, and render them on screen (xc3_viewer).

**Batch Rendering**
1. Parse files (xc3_lib).
2. Decompress, decode, and convert models and textures to a standardized format (xc3_model).
3. Convert the xc3_model data to renderable buffers, textures, pipelines etc (xc3_wgpu).
4. Initialize an `Xc3Renderer`, load the models, and render directly to a texture (xc3_wgpu_batch).

**gltf Export**
1. Parse files (xc3_lib).
2. Decompress, decode, and convert models and textures to a standardized format (xc3_model).
3. Convert the xc3_model data to the gltf format and textures to PNG (xc3_model).

### Errors and Invalid Input
xc3_lib and xc3_model are as strict as possible and use a "parse, don't validate" approach. An overly strict implementation that rejects in game files will be easily detected using xc3_test. Allowing invalid or unrecognized input may still load in game but creates additional edge cases for tooling to support. Rejected input believed to be valid needs to be reviewed manually to determine if any code changes are necessary.

Rendering and conversion operations don't need to be as strict since most major errors are caught in xc3_lib and xc3_model. Rendering skips or applies defaults for invalid data to allow rendering to continue. A partially rendered model is easier to debug than a blank viewport. Conversion utilities skip files that do not convert properly. Non fatal errors or warnings are reported to the user with print or log statements.

## Projects
### xc3_gltf
A command line tool for converting models and maps from Xenoblade 1 DE, Xenoblade 2, and Xenoblade 3 to glTF. This project is a thin wrapper over the conversion capabilities provided by xc3_model. Sharing the xc3_model format between glTF and xc3_wgpu reduces duplicate code code and ensures the conversion code receives more testing. The project provides an alternative to dedicated importer addons and also handles using the xc3_shader database to automatically repack image texture channels. glTF is designed as an interchange format, so there are some limitations in terms of what the output file can support. Using xc3_model directly or xc3_model_py provides consumers with more control at the cost of increased complexity.

### xc3_lib
The goal is to create the simplest possible API that can fully represent the data on disk. This means many buffers will not be decompressed, decoded, deswizzled, etc since this would make it harder to export a file binary identical with the original. Fully representing the binary files makes the public API more complex but makes it easy to test that reading and writing in game files results in identical data with a simple assert. Many xc3_lib types contain unresearched fields and padding fields to ensure that all data is preserved.

The struct and enum definitions also serve as file format documentation for the names and types of fields. Unlike a separate format wiki or binary template files, the code and documentation will always be in sync and can be tested with automated tests. The types also document the ordering of data items in a file via the write implementations.

Operations like deswizzling and decompression are implemented as functions that users must explicitly call to return a type representing the new data rather than modifying the original object. This helps ensure encoding or decoding operations are only performed once and avoids ambiguity about whether an object is in the encoded or decoded state. The `to_dds` operation for `Mibl` textures deswizzles and returns a new `DdsFile` file, for example. More advanced decoding operations are implemented in xc3_model.

Unlike xc3_model, xc3_lib does not make any attempt to be easy to integrate with other languages. Taking advantage of Rust's type system and code generation enables more idiomatic and robust code.

### xc3_model
xc3_model provides an abstraction over xc3_lib that is easier to read, edit, and convert. The representations in xc3_model also allows consuming code to use the same code for different model types and format versions. For example, xc3_model uses a collection of `ImageData` stored at the root level of a model hierarchy to encompass all the many ways that `Mibl` data can be packed and stored across different files.

xc3_model types attempts to fully represent the data in the corresponding xc3_lib types. This enables simple roundtrip tests between xc3_lib and xc3_model data. In practice, some files may not be exactly identical after the conversion due to simplifying assumptions or to enable better cross game compatibility. The goal is for the resulting xc3_lib types to be functionally equivalent in game even if the underlying file data changes slightly. 

Most applications and libraries should depend on xc3_model instead of xc3_lib. The simpler API should also experience fewer breaking changes due to its high level nature compared to xc3_lib. xc3_model is also designed to be easier to make bindings to other languages with a focus on simple types like structs with named fields, lists, and C-style enums. This allows projects like [xc3_model_py](https://github.com/ScanMountGoat/xc3_model_py) to have a nearly identical API. The simpler API of xc3_model compared to xc3_lib means that the Python bindings can have minimal overhead and similar performance to the original Rust code.

### xc3_shader
A library and command line tool for working with the game's compiled shader programs. Parameter names are applied to decompiled GLSL files in `annotation.rs`. Shaders are extracted and decompiled in `extract.rs`. `shader_database.rs` creates a precomputed database of assignments from shader inputs to G-Buffer textures for determining input usage like albedo vs normal. This analysis is facilitated by parsing and converting the code to a directed graph representation in `dependencies.rs`. Decompiling is handled by `Ryujinx.ShaderTools` from [Ryujinx](https://github.com/Ryujinx/Ryujinx). 

Other projects like xc3_model, xc3_wgpu, and xc3_viewer use the generated shader database to determine how to assign textures in a material. This includes which textures to assign to a particular output like normals or albedo as well as how to unpack and pack the texture color channels. The shader database is usually an optional argument since not all applications require assigned textures. The actual database types are stored in xc3_model to separate the CLI tool and its dependencies. The database format is not stable, so consuming code should use xc3_model as a library to parse the database files. 

### xc3_test
A command line tool for testing parsing and conversion code for all files in an extracted dump. This allows files of a given type to be checked efficiently in parallel and avoids needing to commit game files to source control. The main goal is to ensure that all files in the game dump of a given format load and convert without any errors. More specific tests are usually better suited as unit tests in the associated projects.

### xc3_tex
A command line tool for converting texture files to and from DDS or image formats like PNG, TIFF, or JPEG. DDS works well as an intermediate format, so the code just calls the appropriate conversion functions and handles command line parameters.

### xc3_viewer
A simple winit desktop application for rendering model files using xc3_wgpu. This is intended as a development aid rather than for end users. xc3_viewer utilizes a number of projects, so checking models for rendering errors can be an effective way to find bugs in other projects.

### xc3_wgpu
A wgpu based renderer for model files with an emphasis on portability and readability over perfect in game accuracy. The most important user accessible type is `Xc3Renderer` since this renders models and implements a series of render passes based on the in game renderer.

wgpu initializes GPU resources using immutable descriptor objects similar to Vulkan. This makes xc3_wgpu a good way to document how parameters in game files affect rendering in game since all the rendering state is explicitly initialized in Rust functions. Code in xc3_wgpu is organized based on key wgpu objects like pipelines or samplers to make this information easier to find.

Shaders are written in WGSL for best compatibility with wgpu/WebGPU. Most of the boilerplate code for working with the WGSL shaders is generated in the `build.rs` script using [wgsl_to_wgpu](https://github.com/ScanMountGoat/wgsl_to_wgpu).

### xc3_wgpu_batch
A CLI program for testing the entire loading and rendering code from xc3_lib, xc3_model, and xc3_wgpu. xc3_wgpu_batch renders directly to textures to create PNG files, so  no window is ever constructed. This makes it easy to identify major rendering errors or models that fail to load properly. Changes to the file formats themselves should use xc3_test since xc3_test runs faster and gives more detailed feedback on errors compared to xc3_wgpu_batch.

### xc3_write
The two pass writing system for handling writing of binary files and offset calculation. See [Offsets](https://github.com/ScanMountGoat/xc3_lib/blob/main/Offsets.md) for a high level overview and pseudocode.

### xc3_write_derive
A procedural macro for generating code for xc3_write at compile time.
