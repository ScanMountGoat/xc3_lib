# xc3_tex
A command line tool for converting and replacing texture files for Xenoblade 1 DE, Xenoblade 2, and Xenoblade 3.

xc3_tex supports converting proprietary in game texture formats to and from a variety of common formats like DDS or PNG. All the original texture array layers, depth slices, and mipmaps will be preserved when using DDS. Using DDS is strongly recommended for most conversions.

## Usage
See the help text for a full list of commands and supported formats.

`xc3_tex --help`  
`xc3_tex "Xeno 3 Dump\chr\tex\nx\m\00a57332.wismt" out.dds`  
`xc3_tex in.dds out.wismt`  
`xc3_tex in.dds out.witex`  
`xc3_tex in.png out.witex BC7Unorm`

### DDS Conversion
xc3_tex also provides the ability to convert DDS files to and from uncompressed formats like PNG or TIFF. This is helpful on platforms like Linux and MacOS since many popular texture conversion tools are Windows only.

`xc3_tex in.dds out.png`  
`xc3_tex in.png out.dds BC7Unorm`

### Wilay Texture Replacement
Export the DDS and JPEG images by dragging and dropping the `.wilay` file onto the executable or by running the terminal command. After editing the images, use the edit-wilay command to replace the images.
The modified images do not need to match the resolution and format of the originals.

`xc3_tex image.wilay image_folder`  
`xc3_tex edit-wilay image.wilay image_folder output.wilay`  