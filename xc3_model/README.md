# xc3_model
xc3_model provides an abstraction over [xc3_lib](https://crates.io/crates/xc3_lib) that is easier to read, edit, and convert. 

xc3_model provides a simple, standardized interface for working with models, textures, materials, and animations. These abstractions may encompass multiply binary format types in xc3_lib. All models and maps from all supported file revisions and game versions are converted to the same types. This greatly reduces the amount of code needed for applications and libraries to work with in game models.

xc3_model types attempts to fully represent the data in the corresponding xc3_lib types. This enables simple roundtrip tests between xc3_lib and xc3_model data. In practice, some files may not be exactly identical after the conversion due to simplifying assumptions or to enable better cross game compatibility. The goal is for the resulting xc3_lib types to be functionally equivalent in game even if the underlying file data changes slightly. 

Most applications and libraries should depend on xc3_model instead of xc3_lib. The simpler API experience fewer breaking changes than xc3_lib and is easier to wrap for use in other languages. This allows projects like [xc3_model_py](https://github.com/ScanMountGoat/xc3_model_py) to have a nearly identical API. The simpler API of xc3_model compared to xc3_lib means that the Python bindings can have minimal overhead and similar performance to the original Rust code.

xc3_model has limited support for converting data back to xc3_lib types to enable saving changes to disk. The goal is for the resulting file structs to be functionally equivalent in game even if the data changes slightly due to simplifying assumptions or adjustments to improve cross game compatibility.