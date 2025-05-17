# xc3_shader
A command line tool for extracting and analyzing shaders for Xenoblade Chronicles X, Xenoblade Chronicles 1 Definitive Edition, Xenoblade Chronicles 2, Xenoblade Chronicles 3, and Xenoblade Chronicles X Definitive Edition.

## Usage
Some example commands are provided below.

```
xc3_shader --help
xc3_shader decompile-shaders "Switch Game Dump" "Shader Dump" Ryujinx.ShaderTools.exe
xc3_shader disassemble-legacy-shaders "Wii U Dump" "Shader Dump" gfd-tool.exe
xc3_shader shader-database "Shader Dump" shader_database.bin
xc3_shader glsl-dependencies shader.glsl out.glsl out_attr0.x
xc3_shader latte-dependencies shader.txt out.glsl PIX0.x
xc3_shader latte-glsl shader.txt out.glsl
xc3_shader merge-databases combined.bin xc1.bin xc3.bin xc3.bin xcx.bin
```

## Shader Database
The shader database contains a simplified graph representation defined in xc3_model for each compiled shader program. This representation is converted into WGSL code for xc3_wgpu, material assignments for xc3_gltf, and shader nodes in xenoblade_blender. Pregenerated database files are available in [releases](https://github.com/ScanMountGoat/xc3_lib/releases).

Shader analysis converts GLSL and Wii U shader assembly code to a directed graph similar to shader node graphs in game engines or content creation applications. This enables the detection of subgraphs for common operations like compiled functions or matrix multiplications while handling differences in variable names or algebraically equivalent expressions. For example, dozens of directed shader graph nodes can be converted to a single overlay blend layer for the shader database and a single call to a user defined overlay blend function in WGSL.

`Wii U Binary -> assembly (gfd-tool) -> Graph (xc3_shader) -> ShaderProgram (xc3_shader) -> WGSL (xc3_wgpu)`  
`Switch Binary -> GLSL (Ryujinx.ShaderTools) -> Graph (xc3_shader) -> ShaderProgram (xc3_shader) -> WGSL (xc3_wgpu)`  

Decompiling shaders requires compiling `Ryujinx.ShaderTools` from the [Ryujinx](https://github.com/Ryujinx/Ryujinx) emulator. Disassembling Wii U shaders requires [gfd-tool](https://github.com/decaf-emu/decaf-emu/releases).

### Analyzing Shaders
1. Identify the shader in the decompiled shader dump using the model name and the slct index. The slct index is displayed as a custom property on the material in [xenoblade_blender](https://github.com/ScanMountGoat/xenoblade_blender) for convenience.
2. Print out the shader database representation using `xc3_shader glsl-output-dependencies` command.
3. Extract the code for the desired output for manual analysis using the `xc3_shader glsl-dependencies` command.
4. Analyze the relevant lines of shader code. There are many ways of doing this. Applying basic substitution until recognizable patterns emerge is one approach. Common GLSL functions like `mix(a, b, ratio)` may compile into multiple instructions and may not always be compiled the same way. Keep in mind relevant hardware details like the Switch using scalar operations and the Wii U using mostly vector operations.