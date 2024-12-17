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
//! use xc3_wgpu::{MonolibShaderTextures, Renderer};
//! use xc3_model::shader_database::ShaderDatabase;
//!
//! # fn test() -> (wgpu::Device, wgpu::Queue) { todo!() }
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! # let (device, queue) = test();
//! let monolib_shader = MonolibShaderTextures::from_file(&device, &queue, "monolib/shader");
//! let renderer = Renderer::new(&device, &queue, 1920, 1080, wgpu::TextureFormat::Bgra8Unorm, &monolib_shader);
//!
//! let database = ShaderDatabase::from_file("xc3.bin")?;
//!
//! let root = xc3_model::load_model("ch01011013.wimdo", Some(&database))?;
//! let groups = xc3_wgpu::load_model(&device, &queue, &[root], &monolib_shader);
//!
//! let roots = xc3_model::load_map("ma59a.wismhd", Some(&database))?;
//! let groups = xc3_wgpu::load_map(&device, &queue, &roots, &monolib_shader);
//! # Ok(())
//! # }
//! ```
//!
//! # Animation
//! Skeletal animations should use [Models::update_bone_transforms] and
//! the [Animation](xc3_model::animation::Animation) type from [xc3_model].

mod animation;
mod collision;
mod culling;
mod material;
mod model;
mod monolib;
mod pipeline;
mod renderer;
mod sampler;
mod shader;
mod skeleton;
mod texture;

pub use collision::{load_collisions, Collision};
pub use material::Material;
pub use model::{load_map, load_model, Mesh, Model, ModelBuffers, ModelGroup, Models};
pub use monolib::MonolibShaderTextures;
pub use renderer::{CameraData, RenderMode, Renderer};

use encase::{internal::WriteInto, ShaderSize, ShaderType, StorageBuffer, UniformBuffer};
use indexmap::IndexMap;
use std::hash::Hash;
use wgpu::util::DeviceExt;

// TODO: How is sRGB gamma handled in game?

const COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const GBUFFER_COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const GBUFFER_NORMAL_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgb10a2Unorm;
const DEPTH_STENCIL_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth24PlusStencil8;

/// The features required by the renderer.
pub const FEATURES: wgpu::Features =
    wgpu::Features::TEXTURE_COMPRESSION_BC.union(wgpu::Features::POLYGON_MODE_LINE);

trait DeviceBufferExt {
    fn create_uniform_buffer<T: ShaderType + WriteInto + ShaderSize>(
        &self,
        label: &str,
        contents: &T,
    ) -> wgpu::Buffer;

    fn create_storage_buffer<T: ShaderType + WriteInto + ShaderSize>(
        &self,
        label: &str,
        contents: &[T],
    ) -> wgpu::Buffer;
}

impl DeviceBufferExt for wgpu::Device {
    fn create_uniform_buffer<T: ShaderType + WriteInto + ShaderSize>(
        &self,
        label: &str,
        data: &T,
    ) -> wgpu::Buffer {
        let mut buffer = UniformBuffer::new(Vec::new());
        buffer.write(&data).unwrap();

        // TODO: is it worth not adding COPY_DST to all buffers?
        self.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(label),
            contents: &buffer.into_inner(),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        })
    }

    fn create_storage_buffer<T: ShaderType + WriteInto + ShaderSize>(
        &self,
        label: &str,
        data: &[T],
    ) -> wgpu::Buffer {
        let mut buffer = StorageBuffer::new(Vec::new());
        buffer.write(&data).unwrap();

        // TODO: is it worth not adding COPY_DST to all buffers?
        self.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(label),
            contents: &buffer.into_inner(),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        })
    }
}

trait QueueBufferExt {
    fn write_uniform_data<T: ShaderType + WriteInto + ShaderSize>(
        &self,
        buffer: &wgpu::Buffer,
        data: &T,
    );

    fn write_storage_data<T: ShaderType + WriteInto + ShaderSize>(
        &self,
        buffer: &wgpu::Buffer,
        data: &[T],
    );
}

impl QueueBufferExt for wgpu::Queue {
    fn write_uniform_data<T: ShaderType + WriteInto + ShaderSize>(
        &self,
        buffer: &wgpu::Buffer,
        data: &T,
    ) {
        let mut bytes = UniformBuffer::new(Vec::new());
        bytes.write(&data).unwrap();

        self.write_buffer(buffer, 0, &bytes.into_inner());
    }

    fn write_storage_data<T: ShaderType + WriteInto + ShaderSize>(
        &self,
        buffer: &wgpu::Buffer,
        data: &[T],
    ) {
        let mut bytes = StorageBuffer::new(Vec::new());
        bytes.write(&data).unwrap();

        self.write_buffer(buffer, 0, &bytes.into_inner());
    }
}

/// A trait for mapping unique items to an index.
pub trait IndexMapExt<T> {
    fn entry_index(&mut self, key: T) -> usize;
}

impl<T> IndexMapExt<T> for IndexMap<T, usize>
where
    T: Hash + Eq,
{
    fn entry_index(&mut self, key: T) -> usize {
        let new_value = self.len();
        *self.entry(key).or_insert(new_value)
    }
}
