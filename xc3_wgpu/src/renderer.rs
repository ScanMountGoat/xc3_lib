use glam::{Mat4, Vec4};
use wgpu::util::DeviceExt;

use crate::{model::Model, pipeline::model_pipeline};

pub struct Xc3Renderer {
    camera_buffer: wgpu::Buffer,

    bind_group0: crate::shader::model::bind_groups::BindGroup0,
    model_pipeline: wgpu::RenderPipeline,

    depth_texture: wgpu::Texture,
    depth_view: wgpu::TextureView,
}

pub struct CameraData {
    pub view_projection: Mat4,
    pub position: Vec4,
}

impl Xc3Renderer {
    pub fn new(device: &wgpu::Device, width: u32, height: u32) -> Self {
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("camera buffer"),
            contents: bytemuck::cast_slice(&[crate::shader::model::Camera {
                view_projection: Mat4::IDENTITY,
                position: Vec4::ZERO,
            }]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group0 = crate::shader::model::bind_groups::BindGroup0::from_bindings(
            device,
            crate::shader::model::bind_groups::BindGroupLayout0 {
                camera: camera_buffer.as_entire_buffer_binding(),
            },
        );

        let model_pipeline = model_pipeline(device);

        let (depth_texture, depth_view) = create_depth_texture(device, width, height);

        Self {
            camera_buffer,
            bind_group0,
            model_pipeline,
            depth_texture,
            depth_view,
        }
    }

    pub fn render_model(
        &self,
        output_view: &wgpu::TextureView,
        encoder: &mut wgpu::CommandEncoder,
        model: &Model,
    ) {
        // TODO: deferred rendering like in game?
        self.model_pass(encoder, output_view, model);
    }

    pub fn update_camera(&self, queue: &wgpu::Queue, camera_data: &CameraData) {
        queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[crate::shader::model::Camera {
                view_projection: camera_data.view_projection,
                position: camera_data.position,
            }]),
        );
    }

    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        // Update each resource that depends on window size.
        let (depth_texture, depth_view) = create_depth_texture(device, width, height);
        self.depth_texture = depth_texture;
        self.depth_view = depth_view;
    }

    fn model_pass(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        output_view: &wgpu::TextureView,
        model: &Model,
    ) {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Model Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: output_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: true,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: true,
                }),
                stencil_ops: None,
            }),
        });

        render_pass.set_pipeline(&self.model_pipeline);

        // TODO: organize into per frame, per model, etc?
        self.bind_group0.set(&mut render_pass);

        model.draw(&mut render_pass);
    }
}

fn create_depth_texture(
    device: &wgpu::Device,
    width: u32,
    height: u32,
) -> (wgpu::Texture, wgpu::TextureView) {
    let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("depth texture"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: crate::DEPTH_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });

    let depth_view = depth_texture.create_view(&Default::default());

    (depth_texture, depth_view)
}
