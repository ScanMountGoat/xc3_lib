// TODO: Rework public API
pub mod material;
pub mod model;
pub mod pipeline;
pub mod renderer;
pub mod sampler;
pub mod shader;
pub mod texture;

// TODO: How is sRGB gamma handled in game?
pub const COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8Unorm;
pub const GBUFFER_COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

/// The features required by the renderer.
pub const FEATURES: wgpu::Features = wgpu::Features::TEXTURE_COMPRESSION_BC;