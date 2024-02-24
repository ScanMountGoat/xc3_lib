//! # xc3_wgpu
//! A Xenoblade Chronicles model rendering library.
//!
//! Xenoblade 1 DE, Xenoblade 2, and Xenoblade 3 are all supported
//! with Xenoblade 1 DE receiving the least testing.
//!
//! # Getting Started
//! The first step is to initialize an [Xc3Renderer].
//! This only needs to be done once since the renderer can be updated using methods
//! as screen size and parameters change.
//! The initial size should match the current window dimensions.
//!
//! Models and maps are all loaded from the same [xc3_model] types.
//! The shader database is optional but will improve rendering accuracy.
//!
//! In each frame, render the [ModelGroup] using [Xc3Renderer::render_models].
//!
//! ```rust no_run
//! use xc3_wgpu::Xc3Renderer;
//! use xc3_model::shader_database::ShaderDatabase;
//!
//! # fn test() -> (wgpu::Device, wgpu::Queue) { todo!() }
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! # let (device, queue) = test();
//! let renderer = Xc3Renderer::new(&device, &queue, 1920, 1080, "monolib/shader");
//!
//! let database = ShaderDatabase::from_file("xc3.json")?;
//!
//! let root = xc3_model::load_model("ch01011013.wimdo", Some(&database))?;
//! let groups = xc3_wgpu::load_model(&device, &queue, &[root]);
//!
//! let roots = xc3_model::load_map("ma59a.wismhd", Some(&database))?;
//! let groups = xc3_wgpu::load_model(&device, &queue, &roots);
//! # Ok(())
//! # }
//! ```
//!
//! # Animation
//! Skeletal animations should use [Models::update_bone_transforms] and
//! the [Animation](xc3_model::animation::Animation) type from [xc3_model].

mod animation;
mod culling;
mod material;
mod model;
mod monolib;
mod pipeline;
mod renderer;
mod sampler;
mod shader;
mod texture;

pub use material::Material;
pub use model::{load_model, Mesh, Model, ModelBuffers, ModelGroup, Models};
pub use monolib::MonolibShaderTextures;
pub use renderer::{CameraData, RenderMode, Xc3Renderer};

// TODO: How is sRGB gamma handled in game?

/// The format used for the final RGBA render pass.
/// Applications should use this format when integrating the renderer.
pub const COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8Unorm;

/// The format used to store each of the G-Buffer textures for deferred rendering.
pub const GBUFFER_COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8Unorm;

/// The format used for depth textures for depth testing.
pub const DEPTH_STENCIL_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth24PlusStencil8;

/// The features required by the renderer.
pub const FEATURES: wgpu::Features =
    wgpu::Features::TEXTURE_COMPRESSION_BC.union(wgpu::Features::POLYGON_MODE_LINE);
