# xc3_shader
A command line tool for extracting and analyzing shaders for Xenoblade X, Xenoblade 1 DE, Xenoblade 2, and Xenoblade 3.

Decompiling shaders requires compiling `Ryujinx.ShaderTools` from the [Ryujinx](https://github.com/Ryujinx/Ryujinx) emulator. Disassembling Wii U shaders requires [gfd-tool](https://github.com/decaf-emu/decaf-emu/releases).

## Usage
Some example commands are provided below.

```
xc3_shader --help
xc3_shader decompile-shaders "Switch Game Dump" "Shader Dump" Ryujinx.ShaderTools.exe
xc3_shader disassemble-legacy-shaders "Wii U Dump" "Shader Dump" gfd-tool.exe
xc3_shader shader-database "Shader Dump" shader_database.json
xc3_shader glsl-dependencies shader.glsl out.glsl out_attr0.x
```
