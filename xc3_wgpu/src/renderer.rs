use glam::{uvec4, Mat4, Vec4};
use wgpu::util::DeviceExt;

use crate::{model::ModelGroup, COLOR_FORMAT, GBUFFER_COLOR_FORMAT};

pub struct Xc3Renderer {
    camera_buffer: wgpu::Buffer,

    model_bind_group0: crate::shader::model::bind_groups::BindGroup0,

    deferred_pipeline: wgpu::RenderPipeline,
    deferred_bind_group0: crate::shader::deferred::bind_groups::BindGroup0,
    deferred_bind_group1: crate::shader::deferred::bind_groups::BindGroup1,
    gbuffer: GBuffer,
    debug_settings_buffer: wgpu::Buffer,

    depth_view: wgpu::TextureView,
}

pub struct CameraData {
    pub view: Mat4,
    pub view_projection: Mat4,
    pub position: Vec4,
}

// Fragment outputs for all 3 games to use in the deferred pass.
// TODO: Are there ever more than 6 outputs?
pub struct GBuffer {
    color: wgpu::TextureView,
    etc_buffer: wgpu::TextureView,
    normal: wgpu::TextureView,
    velocity: wgpu::TextureView,
    depth: wgpu::TextureView,
    lgt_color: wgpu::TextureView,
}

impl Xc3Renderer {
    pub fn new(device: &wgpu::Device, width: u32, height: u32) -> Self {
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("camera buffer"),
            contents: bytemuck::cast_slice(&[crate::shader::model::Camera {
                view: Mat4::IDENTITY,
                view_projection: Mat4::IDENTITY,
                position: Vec4::ZERO,
            }]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let model_bind_group0 = crate::shader::model::bind_groups::BindGroup0::from_bindings(
            device,
            crate::shader::model::bind_groups::BindGroupLayout0 {
                camera: camera_buffer.as_entire_buffer_binding(),
            },
        );

        let deferred_pipeline = deferred_pipeline(device);

        let depth_view = create_depth_texture(device, width, height);

        let gbuffer = create_gbuffer(device, width, height);
        let deferred_bind_group0 = create_deferred_bind_group0(device, &gbuffer);

        // The resolution should match the screen resolution, so a default sampler is fine.
        let shared_sampler = device.create_sampler(&wgpu::SamplerDescriptor::default());

        let debug_settings_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Debug Settings"),
            contents: bytemuck::cast_slice(&[crate::shader::deferred::DebugSettings {
                index: uvec4(0, 0, 0, 0),
            }]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let deferred_bind_group1 = crate::shader::deferred::bind_groups::BindGroup1::from_bindings(
            device,
            crate::shader::deferred::bind_groups::BindGroupLayout1 {
                shared_sampler: &shared_sampler,
                camera: camera_buffer.as_entire_buffer_binding(),
                debug_settings: debug_settings_buffer.as_entire_buffer_binding(),
            },
        );

        Self {
            camera_buffer,
            model_bind_group0,
            deferred_pipeline,
            depth_view,
            deferred_bind_group0,
            deferred_bind_group1,
            gbuffer,
            debug_settings_buffer,
        }
    }

    pub fn render_models(
        &self,
        output_view: &wgpu::TextureView,
        encoder: &mut wgpu::CommandEncoder,
        models: &[ModelGroup],
    ) {
        // Deferred rendering requires a second forward pass for transparent meshes.
        // TODO: Research more about how this is implemented in game.
        self.model_pass(encoder, models);
        self.transparent_pass(encoder, models);
        self.deferred_pass(encoder, output_view);
    }

    pub fn update_camera(&self, queue: &wgpu::Queue, camera_data: &CameraData) {
        queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[crate::shader::model::Camera {
                view: camera_data.view,
                view_projection: camera_data.view_projection,
                position: camera_data.position,
            }]),
        );
    }

    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        // Update each resource that depends on window size.
        self.depth_view = create_depth_texture(device, width, height);

        self.gbuffer = create_gbuffer(device, width, height);
        self.deferred_bind_group0 = create_deferred_bind_group0(device, &self.gbuffer);
    }

    pub fn update_debug_settings(&self, queue: &wgpu::Queue, index: u32) {
        queue.write_buffer(
            &self.debug_settings_buffer,
            0,
            bytemuck::cast_slice(&[crate::shader::deferred::DebugSettings {
                index: uvec4(index, 0, 0, 0),
            }]),
        );
    }

    fn model_pass(&self, encoder: &mut wgpu::CommandEncoder, models: &[ModelGroup]) {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Model Pass"),
            color_attachments: &[
                color_attachment(&self.gbuffer.color, wgpu::Color::BLACK),
                color_attachment(&self.gbuffer.etc_buffer, wgpu::Color::BLACK),
                color_attachment(
                    &self.gbuffer.normal,
                    wgpu::Color {
                        r: 0.5,
                        g: 0.5,
                        b: 1.0,
                        a: 1.0,
                    },
                ),
                color_attachment(&self.gbuffer.velocity, wgpu::Color::BLACK),
                color_attachment(
                    &self.gbuffer.depth,
                    wgpu::Color {
                        r: 1.0,
                        g: 1.0,
                        b: 1.0,
                        a: 0.0,
                    },
                ),
                color_attachment(&self.gbuffer.lgt_color, wgpu::Color::BLACK),
            ],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        // TODO: organize into per frame, per model, etc?
        self.model_bind_group0.set(&mut render_pass);

        for model in models {
            model.draw(&mut render_pass, false);
        }
    }

    fn transparent_pass(&self, encoder: &mut wgpu::CommandEncoder, models: &[ModelGroup]) {
        // The transparent pass only writes to the albedo output.
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Transparent Pass"),
            color_attachments: &[
                Some(wgpu::RenderPassColorAttachment {
                    view: &self.gbuffer.color,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        // TODO: Does in game actually use load?
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                }),
                color_attachment_disabled(&self.gbuffer.etc_buffer),
                color_attachment_disabled(&self.gbuffer.normal),
                color_attachment_disabled(&self.gbuffer.velocity),
                color_attachment_disabled(&self.gbuffer.depth),
                color_attachment_disabled(&self.gbuffer.lgt_color),
            ],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    // TODO: Write to depth buffer?
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        // TODO: organize into per frame, per model, etc?
        self.model_bind_group0.set(&mut render_pass);

        // TODO: Is this the correct unk type?
        for model in models {
            model.draw(&mut render_pass, true);
        }
    }

    fn deferred_pass(&self, encoder: &mut wgpu::CommandEncoder, output_view: &wgpu::TextureView) {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Deferred Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: output_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        render_pass.set_pipeline(&self.deferred_pipeline);

        crate::shader::deferred::bind_groups::set_bind_groups(
            &mut render_pass,
            crate::shader::deferred::bind_groups::BindGroups {
                bind_group0: &self.deferred_bind_group0,
                bind_group1: &self.deferred_bind_group1,
            },
        );

        render_pass.draw(0..3, 0..1);
    }
}

fn create_deferred_bind_group0(
    device: &wgpu::Device,
    gbuffer: &GBuffer,
) -> crate::shader::deferred::bind_groups::BindGroup0 {
    crate::shader::deferred::bind_groups::BindGroup0::from_bindings(
        device,
        crate::shader::deferred::bind_groups::BindGroupLayout0 {
            g_color: &gbuffer.color,
            g_etc_buffer: &gbuffer.etc_buffer,
            g_normal: &gbuffer.normal,
            g_velocity: &gbuffer.velocity,
            g_depth: &gbuffer.depth,
            g_lgt_color: &gbuffer.lgt_color,
        },
    )
}

fn color_attachment(
    view: &wgpu::TextureView,
    color: wgpu::Color,
) -> Option<wgpu::RenderPassColorAttachment> {
    Some(wgpu::RenderPassColorAttachment {
        view,
        resolve_target: None,
        ops: wgpu::Operations {
            load: wgpu::LoadOp::Clear(color),
            store: wgpu::StoreOp::Store,
        },
    })
}

fn color_attachment_disabled(view: &wgpu::TextureView) -> Option<wgpu::RenderPassColorAttachment> {
    // Necessary to fix a validation error about writing to missing attachments.
    // This could also be fixed by modifying the shader code.
    Some(wgpu::RenderPassColorAttachment {
        view,
        resolve_target: None,
        ops: wgpu::Operations {
            load: wgpu::LoadOp::Load,
            store: wgpu::StoreOp::Store,
        },
    })
}

fn create_gbuffer(device: &wgpu::Device, width: u32, height: u32) -> GBuffer {
    GBuffer {
        color: create_gbuffer_texture(device, width, height, "g_color"),
        etc_buffer: create_gbuffer_texture(device, width, height, "g_etc_buffer"),
        normal: create_gbuffer_texture(device, width, height, "g_normal"),
        velocity: create_gbuffer_texture(device, width, height, "g_velocity"),
        depth: create_gbuffer_texture(device, width, height, "g_depth"),
        lgt_color: create_gbuffer_texture(device, width, height, "g_lgt_color"),
    }
}

fn create_gbuffer_texture(
    device: &wgpu::Device,
    width: u32,
    height: u32,
    name: &str,
) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some(name),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: GBUFFER_COLOR_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        })
        .create_view(&wgpu::TextureViewDescriptor::default())
}

fn create_depth_texture(device: &wgpu::Device, width: u32, height: u32) -> wgpu::TextureView {
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

    depth_texture.create_view(&Default::default())
}

fn deferred_pipeline(device: &wgpu::Device) -> wgpu::RenderPipeline {
    let module = crate::shader::deferred::create_shader_module(device);
    let render_pipeline_layout = crate::shader::deferred::create_pipeline_layout(device);

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Deferred Pipeline"),
        layout: Some(&render_pipeline_layout),
        vertex: crate::shader::deferred::vertex_state(
            &module,
            &crate::shader::deferred::vs_main_entry(),
        ),
        fragment: Some(wgpu::FragmentState {
            module: &module,
            entry_point: crate::shader::deferred::ENTRY_FS_MAIN,
            // TODO: alpha blending?
            targets: &[Some(wgpu::ColorTargetState {
                format: COLOR_FORMAT,
                blend: None,
                write_mask: wgpu::ColorWrites::all(),
            })],
        }),
        primitive: wgpu::PrimitiveState {
            // TODO: Do all meshes using indexed triangle lists?
            topology: wgpu::PrimitiveTopology::TriangleList,
            polygon_mode: wgpu::PolygonMode::Fill,
            cull_mode: Some(wgpu::Face::Back),
            ..Default::default()
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
    })
}
