// TODO: Rework public API
pub mod material;
pub mod model;
pub mod pipeline;
pub mod renderer;
pub mod shader;
pub mod texture;

pub const COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8UnormSrgb;
pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
