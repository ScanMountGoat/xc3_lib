# xc3_gltf
A command line tool for converting models and maps from Xenoblade X, Xenoblade 1 DE, Xenoblade 2, and Xenoblade 3 to glTF.

glTF is an open interchange format designed for efficiently transmitting and loading 3D models. The goal of xc3_gltf is to be able to quickly export models and maps into a format that can be understood by many 3D modeling applications and game engines. Use [xenoblade_blender](https://github.com/ScanMountGoat/xenoblade_blender) for better results when only using the models within Blender.

## Usage
Convert `.wimdo`, `.wismhd`, or `.camdo` files to `.gltf` or `.glb`. Exporting as `.gltf` will create a `.gltf`, `.bin`, and multiple `.png` files. File names will start with the name chosen for the output glTF file. Exporting as `.glb` will embed all data into a single `.glb` file. 

The shader database parameter is optional but highly recommended since the fallback texture assignments do not support channel packing of temp textures. The database parameter will default to `xc_combined.bin` in the executable directory if not specified.

The output will default to the first input file with the extension changed to `.glb`. This enables Windows users to simply drag and drop supported input files onto the executable to export as `.glb` with defaults for remaining arguments.

`xc3_gltf --help`  
`xc3_gltf "Xeno 2 Dump/map/ma02a.wismhd" --output map.gltf --database xc2.bin`  
`xc3_gltf "Xeno 2 Dump/model/np/np000301.wimdo" --output bana.gltf --database xc2.bin --anim "Xeno 2 Dump/model/np/np000301.mot"`    
`xc3_gltf "Xeno 3 Dump/chr/ch/ch01027000.wimdo" --output mio.gltf --database xc3.bin`  
`xc3_gltf "Xeno 3 Dump/chr/ch/ch01027000.wimdo" --output mio.glb --database xc3.bin --anim "Xeno 3 Dump/chr/ch/ch01027000_event.mot --anim "Xeno 3 Dump/chr/ch/ch01027000_field.mot"`  
`xc3_gltf "Xeno X Dump/chr_np/np/np009001.camdo" --output tatsu.gltf --database xcx.bin`    

## Features
* position, normal, tangent, texture coordinate, and vertex color attributes
* mesh instancing
* skeletons
* vertex skin weights
* morph targets (blend shapes)
* PNG textures
* material texture assignments
* channel packed textures based on shader data
* alpha textures for transparency
* texcoord scale via the `KHR_texture_transform` extension
* animations from `.mot` files

## Limitations
glTF is designed for compatibility, so the imported files will never perfectly recreate the Xenoblade specific data like materials and shaders. Dedicated importing plugins for a specific application or game engine will always be able to outperform formats like glTF in terms of speed and accuracy. Exported files have the following limitations:

* texcoord scale does not apply to normal and occlusion textures due to a limitation in the glTF crate
* materials do not include specular color
* materials do not contain all textures referenced in game (use xc3_tex to extract all unmodified textures)
* materials do not support blending between multiple color or normal maps
