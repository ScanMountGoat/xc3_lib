# xc3_lib
An experimental Rust library for reading various formats for Xenoblade Chronicles 3.

## Credits
This project is based on previous reverse engineering work, including work done for Xenoblade 2.
Special thanks go to members of the World Tree Research discord (formerly the World of Alrest discord) for their assistance.
* https://github.com/PredatorCZ/XenoLib
* https://github.com/BlockBuilder57/XB2AssetTool
* https://github.com/Turk645/Xenoblade-Switch-Model-Importer-Noesis
* https://github.com/AlexCSDev/XbTool

This project makes use of a number of Rust crates that are useful for reverse engineering. For a full list of dependencies, see the Cargo.toml files.
* https://github.com/jam1garner/binrw - declarative binary parsing
* https://github.com/ScanMountGoat/tegra_swizzle - efficient and robust Tegra X1 swizzling/deswizzling
* https://github.com/ScanMountGoat/image_dds - encode/decode BCN image data