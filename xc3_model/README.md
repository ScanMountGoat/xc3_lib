# xc3_model
xc3_model is a high level library for [xc3_lib](https://crates.io/crates/xc3_lib).

xc3_model provides a simple, standardized interface for working with models, textures, materials, and animations. All models and maps from all supported game versions are converted to the same set of types. This greatly reduces the amount of code needed for applications and libraries to work with in game models.

xc3_model has limited support for converting data back to xc3_lib types to enable saving changes to disk. The goal is for the resulting file structs to be functionally equivalent in game even if the data changes slightly due to simplifying assumptions or adjustments to improve cross game compatibility.

Python bindings for xc3_model are available with [xc3_model_py](https://github.com/ScanMountGoat/xc3_model_py). 
