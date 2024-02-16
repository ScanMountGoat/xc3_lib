# xc3_gltf
A command line tool for converting models and maps from Xenoblade 1 DE, Xenoblade 2, and Xenoblade 3 to glTF.

glTF is an open interchange format designed for efficiently transmitting and loading 3D models. The goal of xc3_gltf is to be able to quickly export models and maps into a format that can be understood by many 3D modeling applications and game engines.

## Usage
Convert `.wimdo` or `.wismhd` files to `.gltf`. The shader database parameter is optional but highly recommended since the fallback texture assignments do not support channel packing of temp textures. Texture file names will start with the name chosen for the output glTF file.

`xc3_gltf --help`  
`xc3_gltf "Xeno 2 Dump\map\ma02a.wismhd" export\map.gltf xc2.json`  
`xc3_gltf "Xeno 3 Dump\chr\ch\ch01027000.wimdo" export\model.gltf xc3.json`    

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

## Limitations
glTF is designed for compatibility, so the imported files will never perfectly recreate the Xenoblade specific data like materials and shaders. Dedicated importing plugins for a specific application or game engine will always be able to outperform formats like glTF in terms of speed and accuracy. Exported files have the following limitations:

* texcoord scale does not apply to normal and occlusion textures due to a limitation in the glTF crate
* materials do not include specular color or emissive maps
* materials do not contain all textures referenced in game like additional normal maps
* materials do not support global textures like detail normal maps or Xenoblade 3's eyepatch textures
* materials do not support blending between multiple color or normal maps

## Blender Tips
There are some limitations with using glTF compared to a dedicated importing addon. The following tips may help with minor inconsistencies after importing.

* change Alpha to "Channel Packed" for textures in the Shader Editor if alpha is black in renders
* set Bone Dir to "Temperance" or "Fortune" in the import settings for cleaner looking armatures