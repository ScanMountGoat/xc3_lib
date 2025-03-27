# xc3_gltf
A command line tool for converting models and maps from Xenoblade Chronicles X, Xenoblade Chronicles 1 Definitive Editino, Xenoblade Chronicles 2, Xenoblade Chronicles 3, and Xenoblade Chronicles X Definitive Edition to glTF.

glTF is an open interchange format designed for efficiently transmitting and loading 3D models. The goal of xc3_gltf is to be able to quickly export models and maps into a format that can be understood by many 3D modeling applications and game engines. Use [xenoblade_blender](https://github.com/ScanMountGoat/xenoblade_blender) for better results when only using the models within Blender.

## Usage
Convert `.wimdo`, `.wismhd`, or `.camdo` files to `.gltf` or `.glb`. Exporting as `.gltf` will create a `.gltf`, `.bin`, and multiple `.png` files. File names will start with the name chosen for the output glTF file. Exporting as `.glb` will embed all data into a single `.glb` file. 

The shader database parameter is optional but highly recommended since the fallback texture assignments only support basic color and normal maps. The database parameter will default to `xc_combined.bin` in the executable directory if not specified.

The output will default to the first input file with the extension changed to `.glb`. This enables Windows users to simply drag and drop supported input files onto the executable to export as `.glb` with defaults for remaining arguments.

`xc3_gltf --help`  
`xc3_gltf map/ma02a.wismhd --output map.gltf --database xc2.bin`  
`xc3_gltf model/np/np000301.wimdo --output bana.gltf --database xc2.bin --anim model/np/np000301.mot`    
`xc3_gltf chr/ch/ch01027000.wimdo --output mio.gltf --database xc3.bin`  
`xc3_gltf chr_np/np/np009001.camdo --output tatsu.gltf --database xcx.bin`  

Load multiple animations by specifying `--anim` for each animation file.  
`xc3_gltf chr/ch/ch01027000.wimdo --output mio.glb --database xc3.bin --anim chr/ch/ch01027000_event.mot --anim chr/ch/ch01027000_field.mot`  

Loading multiple model files will create a combined skeleton with all bones. Animations will apply to the combined skeleton.  
`xc3_gltf chr/pc/pc070109.wimdo chr/pc/pc070205.wimdo chr/pc/pc070204.wimdo chr/pc/pc070203.wimdo chr/pc/pc070202.wimdo chr/pc/pc070201.wimdo --output melia_de.glb --database xc1.bin --anim chr/pc/mp070000.mot`

`xc3_gltf chr_pc/pc221115.camdo chr_pc/pc221111.camdo chr_pc/pc221112.camdo chr_pc/pc221113.camdo chr_pc/pc221114.camdo chr_fc/fc282010.camdo chr_fc/fc281010.camdo --output elma.glb --database xcx.bin`

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
