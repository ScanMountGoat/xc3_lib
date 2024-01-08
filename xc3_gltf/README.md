# xc3_gltf
A command line tool for converting models and maps from Xenoblade 1 DE, Xenoblade 2, and Xenoblade 3 to glTF.

glTF export supports the following features:
- position, normal, tangent, texture coordinate, and vertex color attributes
- mesh instancing
- skeletons
- vertex skin weights
- morph targets (blend shapes)
- PNG textures
- material texture assignments
- channel packed textures based on shader data
- alpha textures for transparency

## Usage
Convert `.wimdo` or `.wismhd` files to `.gltf`. The shader database parameter is optional but highly recommended since the fallback texture assignments do not support channel packing of temp textures. Texture file names will start with the name chosen for the output glTF file.

`xc3_gltf --help`  
`xc3_gltf "Xeno 2 Dump\map\ma02a.wismhd" export\map.gltf xc2.json`  
`xc3_gltf "Xeno 3 Dump\chr\ch\ch01027000.wimdo" export\model.gltf xc3.json`    
