//! In game shaders are precompiled and embedded in files like `.wismt`.
//! xc3_shader can extract and also decompile them if provided a path to `Ryujinx.ShaderTools.exe`.
//! xc3_shader can also analyze the decompiled GLSL code to determine
//! which inputs are assigned to G-Buffer outputs.
//! This step is necessary for determining the usage of a texture like albedo or normal map
//! since the assignments are compiled into the shader code itself.
//! Applications can use the generated G-Buffer database to avoid needing to generate this data at runtime.

// TODO: Do these all need to be public?
pub mod annotation;
pub mod dependencies;
pub mod extract;
pub mod gbuffer_database;
