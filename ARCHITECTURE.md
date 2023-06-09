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

## Projects
### xc3_lib
The file format library and utilities. The goal is to create the simplest possible API that can fully represent the data on disk. This means many buffers will not be decompressed, decoded, deswizzled, etc since this would make it harder to export a file binary identical with the original. 

Operations like deswizzling and decompression are implemented as functions that users must explicitly call to return new data rather than modifying the types representing the data on disk. The deswizzing operation for `Mibl` textures returns a new .dds file, for example. More advanced decoding operations are implemented in higher level projects like xc3_wgpu or xc3_model.

### xc3_lib_derive
A procedural macro for generating code for xc3_lib at compile time.

### xc3_model
Higher level data access for xc3_lib. xc3_model converts the game specific data structures and buffers into a standardized format that is easier to edit, render, and convert. Examples including reading attributes from vertex buffers and constructing a scene hierarchy from model files. This makes it easier to implement features like gltf export since the gltf exporter doesn't need to work directly with game files. xc3_wgpu also utilizes xc3_model for converting the model data into a renderable form. The end goal is for the vast majority of applications and libraries to only need to depend on xc3_model for working with in game model data. The simpler API should also experience fewer breaking changes since it's an abstraction over xc3_lib.

### xc3_shader
A library and command line tool for working with the game's compiled shader programs. Parameter names are applied to decompiled GLSL files in `annotation.rs`. Shaders are extracted and decompiled in `extract.rs`. `gbuffer_database.rs` creates a precomputed database of assignments from shader inputs to G-Buffer textures for determining input usage like albedo vs normal. This analysis is facilitated by parsing and converting the code to a directed graph representation in `dependencies.rs`.

### xc3_test
A command line tool for testing parsing and conversion code for all files in an extracted dump. This allows files of a given type to be checked efficiently in parallel and avoids needing to commit game files to source control. The main goal is to ensure that all files in the game dump of a given format parse without any errors. More specific tests are usually better suited as unit tests in the associated projects.

### xc3_tex
A command line tool for converting texture files to and from DDS or image formats like PNG, TIFF, or JPEG. DDS works well as an intermediate format, so the code just calls the appropriate conversion functions and handles command line parameters.

### xc3_viewer
A simple winit desktop application for rendering model files using xc3_wgpu. This is intended as a development aid rather than for end users. xc3_viewer utilizes a number of projects, so checking models for rendering errors can be an effective way to find bugs in other projects.

### xc3_wgpu
A wgpu based renderer for model files with an emphasis on portability and readability over perfect in game accuracy. The most important user accessible type is `Xc3Renderer` since this renders models and implements a series of render passes based on the in game renderer.

wgpu initializes GPU resources using immutable descriptor objects similar to Vulkan. This makes xc3_wgpu a good way to document how parameters in game files affect rendering in game since all the rendering state is explicitly initialized in Rust functions. Code in xc3_wgpu is organized based on key wgpu objects like pipelines or samplers to make this information easier to find.

### xc3_wgpu_batch
A CLI program for testing the entire loading and rendering code from xc3_lib, xc3_model, and xc3_wgpu. xc3_wgpu_batch renders directly to textures to create PNG files, so  no window is ever constructed. This makes it easy to identify major rendering errors or models that fail to load properly. Changes to the file formats themselves should use xc3_test since xc3_test runs faster and gives more detailed feedback on errors compared to xc3_wgpu_batch.
