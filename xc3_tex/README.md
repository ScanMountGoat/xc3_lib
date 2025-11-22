# xc3_tex
A command line tool for converting and replacing texture files for Xenoblade Chronicles X, Xenoblade Chronicles 1 Definitive Edition, Xenoblade Chronicles 2, Xenoblade Chronicles 3, and Xenoblade Chronicles X Definitive Edition.

xc3_tex supports converting proprietary in game texture formats to and from a variety of common formats like DDS or PNG. Using DDS is strongly recommended since it can preserve all the original texture array layers, depth slices, and mipmaps.

## Usage
See the help text for a full list of commands and supported formats.

`xc3_tex --help`  
`xc3_tex "Xeno 3 Dump/chr/tex/nx/m/00a57332.wismt" out.dds`  
`xc3_tex in.dds out.witex`  
`xc3_tex in.png out.witex --format BC7RgbaUnorm`

## Batch Conversion
xc3_tex can efficiently extract textures from supported files in a folder recursively using the batch-convert command. This can be used to convert all menu images and fonts to PNG.

`xc3_tex batch-convert "dump/menu" "*.{wilay, wifnt}" png`  
`xc3_tex batch-convert "dump/menu" "*.{bmn, catex, caavp, fnt}" png`

### DDS Conversion
xc3_tex also provides the ability to convert DDS files to and from uncompressed formats like PNG or TIFF. This is helpful on platforms like Linux and MacOS since many popular texture conversion tools are Windows only.

`xc3_tex in.dds out.png`  
`xc3_tex in.png out.dds --format BC7RgbaUnorm --quality Fast --no-mipmaps`  
`xc3_tex in.png out.dds --format BC3RgbaUnorm`

### Wilay Texture Replacement
Export the DDS and JPEG images by dragging and dropping the `.wilay` file onto the executable or by running the terminal command. After editing the images, use the edit-wilay command to replace the images.
The modified images do not need to match the resolution and DDS format of the originals.

`xc3_tex image.wilay image_folder`  
`xc3_tex edit-wilay image.wilay image_folder output.wilay`  

### Wimdo/Wismt Texture Replacement
Export the DDS images by dragging and dropping the `.wimdo` file onto the executable or by running the terminal command. The `.wismt` file should be in the same folder as the `.wimdo`. After editing the images, use the edit-wimdo command to replace the images. xc3_tex will output the modified `.wimdo` and `.wismt` files. The modified images do not need to match the resolution and DDS format of the originals.

`xc3_tex input.wimdo image_folder`  
`xc3_tex edit-wimdo input.wimdo image_folder output.wimdo`  

The exported DDS image files will have names formatted as "{file_name}.{index}.{name}.dds" like "ch01011013.0.1fbb6953.dds". The {file_name} should match the file being replaced. The {index} determines the texture index to replace. All indices in the range 0, ..., N-1 should be used for creating a file with N textures. Adding or removing textures is supported as long as the files have the appropriate names. The {name} is optional since textures in game are always referred to using their index.  

Most Xenoblade 3 models store higher resolution textures in the `chr/tex/nx` folder. Specifying the folder is optional if the input file is in a fully extracted game dump. The resulting `.wimdo` and `.wismt` files will be generated with embedded high resolution textures similar to Xenoblade 1 DE and Xenoblade 2 to avoid modifying `chr/tex/nx` textures that may be used by multiple models. This will likely result in larger file sizes than the originals.

`xc3_tex input/chr/ch/ch01011013.wimdo image_folder`  
`xc3_tex edit-wimdo ch01011013.wimdo image_folder output/chr/ch/ch01011013.wimdo input/chr/tex/nx`  

### Camdo/Casmt Textures
Export the DDS images by dragging and dropping the `.camdo` file onto the executable or by running the terminal command. Note that Xenoblade X textures will appear flipped vertically from the expected orientation. This is how texture data is stored, and models have a matching UV layout. Tools like xc3_gltf or xenoblade_blender can flip the textures since they don't attempt to preserve the original texture data. Replacing textures in `.camdo` models is not currently supported.

### Wifnt Texture Replacement
Export the DDS image by dragging and dropping the `.wifnt` file onto the executable or by running the terminal command. After editing the images, use the edit-wifnt command to replace the image.
The modified image does not need to match the resolution and format of the original.

`xc3_tex image.wifnt image.dds`  
`xc3_tex edit-wifnt image.wifnt image.dds output.wifnt`  
