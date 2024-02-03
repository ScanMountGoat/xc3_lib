use std::path::Path;

use glam::{Mat4, Vec4};
use wgpu::util::DeviceExt;
use xc3_lib::mibl::Mibl;
use xc3_model::ImageTexture;

use crate::{model::ModelGroup, texture::create_texture, COLOR_FORMAT, GBUFFER_COLOR_FORMAT};

const MAT_ID_DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth16Unorm;

// TODO: Rename this since it supports all 3 games?
// TODO: Add fallback textures for all the monolib shader textures?
pub struct Xc3Renderer {
    camera_buffer: wgpu::Buffer,

    model_bind_group0: crate::shader::model::bind_groups::BindGroup0,

    deferred_debug_pipeline: wgpu::RenderPipeline,
    deferred_bind_group0: crate::shader::deferred::bind_groups::BindGroup0,
    deferred_bind_group1: crate::shader::deferred::bind_groups::BindGroup1,
    debug_settings_buffer: wgpu::Buffer,

    deferred_pipelines: [wgpu::RenderPipeline; 6],
    deferred_bind_group2: [crate::shader::deferred::bind_groups::BindGroup2; 6],

    render_mode: u32,

    gbuffer: GBuffer,

    morph_pipeline: wgpu::ComputePipeline,

    unbranch_to_depth_pipeline: wgpu::RenderPipeline,
    unbranch_to_depth_bind_group0: crate::shader::unbranch_to_depth::bind_groups::BindGroup0,
    mat_id_depth_view: wgpu::TextureView,

    depth_view: wgpu::TextureView,
}

pub struct CameraData {
    pub view: Mat4,
    pub view_projection: Mat4,
    pub position: Vec4,
}

// Fragment outputs for all 3 games to use in the deferred pass.
// Names adapted from output functions from pcsmt fragment GLSL shaders.
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
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        width: u32,
        height: u32,
        monolib_shader: &Path,
    ) -> Self {
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

        let depth_view = create_depth_texture(device, width, height);

        let gbuffer = create_gbuffer(device, width, height);
        let deferred_bind_group1 = create_deferred_bind_group1(device, &gbuffer);

        let render_mode = 0;
        let debug_settings_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Debug Settings"),
            contents: bytemuck::cast_slice(&[crate::shader::deferred::DebugSettings {
                render_mode,
            }]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // TODO: Create a MonolibShaderTextures type that documents global textures?
        // TODO: Are the mappings the same for all 3 games?
        // TODO: Add an option to load defaults if no path is provided?
        // TODO: Why is this mip count not correct in the mibl?
        let mibl = Mibl::from_file(monolib_shader.join("toon_grad.witex")).unwrap();
        let grad = ImageTexture::from_mibl(&mibl, None, None).unwrap();
        let xc3_toon_grad =
            create_texture(device, queue, &grad).create_view(&wgpu::TextureViewDescriptor {
                mip_level_count: Some(1),
                ..Default::default()
            });

        let shared_sampler = device.create_sampler(&wgpu::SamplerDescriptor::default());

        let deferred_bind_group0 = crate::shader::deferred::bind_groups::BindGroup0::from_bindings(
            device,
            crate::shader::deferred::bind_groups::BindGroupLayout0 {
                debug_settings: debug_settings_buffer.as_entire_buffer_binding(),
                g_toon_grad: &xc3_toon_grad,
                shared_sampler: &shared_sampler,
            },
        );

        // TODO: Is is better to just create separate pipelines?
        let deferred_bind_group2 = [0, 1, 2, 3, 4, 5].map(|mat_id| {
            let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Render Settings"),
                contents: bytemuck::cast_slice(&[crate::shader::deferred::RenderSettings {
                    mat_id,
                }]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

            crate::shader::deferred::bind_groups::BindGroup2::from_bindings(
                device,
                crate::shader::deferred::bind_groups::BindGroupLayout2 {
                    render_settings: buffer.as_entire_buffer_binding(),
                },
            )
        });

        let morph_pipeline = crate::shader::morph::compute::create_main_pipeline(device);

        let unbranch_to_depth_pipeline = unbranch_to_depth_pipeline(device);
        let unbranch_to_depth_bind_group0 = create_unbranch_to_depth_bindgroup(device, &gbuffer);

        let mat_id_depth_view = create_mat_id_depth_texture(device, width, height);

        // TODO: These should all be different entry points?
        let deferred_pipelines = [
            deferred_pipeline(device, crate::shader::deferred::ENTRY_FS_MAIN),
            deferred_pipeline(device, crate::shader::deferred::ENTRY_FS_MAIN),
            deferred_pipeline(device, crate::shader::deferred::ENTRY_FS_TOON),
            deferred_pipeline(device, crate::shader::deferred::ENTRY_FS_MAIN),
            deferred_pipeline(device, crate::shader::deferred::ENTRY_FS_MAIN),
            deferred_pipeline(device, crate::shader::deferred::ENTRY_FS_MAIN),
        ];

        let deferred_debug_pipeline = deferred_debug_pipeline(device);

        Self {
            camera_buffer,
            model_bind_group0,
            deferred_pipelines,
            deferred_debug_pipeline,
            depth_view,
            deferred_bind_group0,
            deferred_bind_group1,
            deferred_bind_group2,
            gbuffer,
            debug_settings_buffer,
            morph_pipeline,
            unbranch_to_depth_pipeline,
            unbranch_to_depth_bind_group0,
            mat_id_depth_view,
            render_mode,
        }
    }

    pub fn render_models(
        &self,
        output_view: &wgpu::TextureView,
        encoder: &mut wgpu::CommandEncoder,
        models: &[ModelGroup],
    ) {
        // The passes and their ordering only loosely matches in game.
        // This enables better performance, portability, etc.
        self.compute_morphs(encoder, models);

        // Deferred rendering requires a second forward pass for transparent meshes.
        // TODO: Research more about how this is implemented in game.
        self.model_pass(encoder, models);
        self.transparent_pass(encoder, models);
        self.unbranch_to_depth_pass(encoder);
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
        self.mat_id_depth_view = create_mat_id_depth_texture(device, width, height);
        self.gbuffer = create_gbuffer(device, width, height);
        self.deferred_bind_group1 = create_deferred_bind_group1(device, &self.gbuffer);
        self.unbranch_to_depth_bind_group0 =
            create_unbranch_to_depth_bindgroup(device, &self.gbuffer);
    }

    pub fn update_debug_settings(&mut self, queue: &wgpu::Queue, render_mode: u32) {
        // TODO: enum for render mode?
        self.render_mode = render_mode;
        queue.write_buffer(
            &self.debug_settings_buffer,
            0,
            bytemuck::cast_slice(&[crate::shader::deferred::DebugSettings { render_mode }]),
        );
    }

    fn model_pass(&self, encoder: &mut wgpu::CommandEncoder, models: &[ModelGroup]) {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Model Pass"),
            color_attachments: &[
                color_attachment(&self.gbuffer.color, wgpu::Color::TRANSPARENT),
                color_attachment(&self.gbuffer.etc_buffer, wgpu::Color::TRANSPARENT),
                color_attachment(
                    &self.gbuffer.normal,
                    wgpu::Color {
                        r: 0.5,
                        g: 0.5,
                        b: 1.0,
                        a: 1.0,
                    },
                ),
                color_attachment(&self.gbuffer.velocity, wgpu::Color::TRANSPARENT),
                color_attachment(
                    &self.gbuffer.depth,
                    wgpu::Color {
                        r: 1.0,
                        g: 1.0,
                        b: 1.0,
                        a: 0.0,
                    },
                ),
                color_attachment(&self.gbuffer.lgt_color, wgpu::Color::TRANSPARENT),
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
        // The transparent pass only writes to the color output.
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

    fn unbranch_to_depth_pass(&self, encoder: &mut wgpu::CommandEncoder) {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Unbranch to Depth Pass"),
            color_attachments: &[],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.mat_id_depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(0.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        render_pass.set_pipeline(&self.unbranch_to_depth_pipeline);

        crate::shader::unbranch_to_depth::bind_groups::set_bind_groups(
            &mut render_pass,
            crate::shader::unbranch_to_depth::bind_groups::BindGroups {
                bind_group0: &self.unbranch_to_depth_bind_group0,
            },
        );

        render_pass.draw(0..3, 0..1);
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
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.mat_id_depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Discard,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        if self.render_mode == 0 {
            for (pipeline, bind_group2) in self
                .deferred_pipelines
                .iter()
                .zip(&self.deferred_bind_group2)
            {
                // Each material ID type renders with a separate pipeline in game.
                render_pass.set_pipeline(pipeline);

                crate::shader::deferred::bind_groups::set_bind_groups(
                    &mut render_pass,
                    crate::shader::deferred::bind_groups::BindGroups {
                        bind_group0: &self.deferred_bind_group0,
                        bind_group1: &self.deferred_bind_group1,
                        bind_group2,
                    },
                );

                render_pass.draw(0..3, 0..1);
            }
        } else {
            render_pass.set_pipeline(&self.deferred_debug_pipeline);

            crate::shader::deferred::bind_groups::set_bind_groups(
                &mut render_pass,
                crate::shader::deferred::bind_groups::BindGroups {
                    bind_group0: &self.deferred_bind_group0,
                    bind_group1: &self.deferred_bind_group1,
                    bind_group2: &self.deferred_bind_group2[0],
                },
            );

            render_pass.draw(0..3, 0..1);
        }
    }

    fn compute_morphs(&self, encoder: &mut wgpu::CommandEncoder, models: &[ModelGroup]) {
        // Reset the buffers each frame before updating them.
        for model in models {
            model.reset_morphs(encoder);
        }

        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Compute Morphs"),
            timestamp_writes: None,
        });
        compute_pass.set_pipeline(&self.morph_pipeline);

        for model in models {
            model.compute_morphs(&mut compute_pass);
        }
    }
}

fn create_unbranch_to_depth_bindgroup(
    device: &wgpu::Device,
    gbuffer: &GBuffer,
) -> crate::shader::unbranch_to_depth::bind_groups::BindGroup0 {
    let shared_sampler = device.create_sampler(&wgpu::SamplerDescriptor::default());

    crate::shader::unbranch_to_depth::bind_groups::BindGroup0::from_bindings(
        device,
        crate::shader::unbranch_to_depth::bind_groups::BindGroupLayout0 {
            g_etc_buffer: &gbuffer.etc_buffer,
            shared_sampler: &shared_sampler,
        },
    )
}

fn create_deferred_bind_group1(
    device: &wgpu::Device,
    gbuffer: &GBuffer,
) -> crate::shader::deferred::bind_groups::BindGroup1 {
    crate::shader::deferred::bind_groups::BindGroup1::from_bindings(
        device,
        crate::shader::deferred::bind_groups::BindGroupLayout1 {
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

fn create_mat_id_depth_texture(
    device: &wgpu::Device,
    width: u32,
    height: u32,
) -> wgpu::TextureView {
    let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("material ID depth texture"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: MAT_ID_DEPTH_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });

    depth_texture.create_view(&Default::default())
}

// TODO: Create 5-6 pipelines for each material type.
// TODO: Just change the fragment entry point?
fn deferred_pipeline(device: &wgpu::Device, entry_point: &str) -> wgpu::RenderPipeline {
    let module = crate::shader::deferred::create_shader_module(device);
    let render_pipeline_layout = crate::shader::deferred::create_pipeline_layout(device);

    // TODO: Debug pipeline should use func always?
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Deferred Pipeline"),
        layout: Some(&render_pipeline_layout),
        vertex: crate::shader::deferred::vertex_state(
            &module,
            &crate::shader::deferred::vs_main_entry(),
        ),
        fragment: Some(wgpu::FragmentState {
            module: &module,
            entry_point,
            targets: &[Some(wgpu::ColorTargetState {
                format: COLOR_FORMAT,
                blend: None,
                write_mask: wgpu::ColorWrites::all(),
            })],
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: Some(wgpu::DepthStencilState {
            format: MAT_ID_DEPTH_FORMAT,
            depth_write_enabled: false,
            depth_compare: wgpu::CompareFunction::Equal,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
    })
}

fn deferred_debug_pipeline(device: &wgpu::Device) -> wgpu::RenderPipeline {
    let module = crate::shader::deferred::create_shader_module(device);
    let render_pipeline_layout = crate::shader::deferred::create_pipeline_layout(device);

    // Make sure the depth test always passes to avoid needing multiple draws.
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Deferred Debug Pipeline"),
        layout: Some(&render_pipeline_layout),
        vertex: crate::shader::deferred::vertex_state(
            &module,
            &crate::shader::deferred::vs_main_entry(),
        ),
        fragment: Some(wgpu::FragmentState {
            module: &module,
            entry_point: crate::shader::deferred::ENTRY_FS_DEBUG,
            targets: &[Some(wgpu::ColorTargetState {
                format: COLOR_FORMAT,
                blend: None,
                write_mask: wgpu::ColorWrites::all(),
            })],
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: Some(wgpu::DepthStencilState {
            format: MAT_ID_DEPTH_FORMAT,
            depth_write_enabled: false,
            depth_compare: wgpu::CompareFunction::Always,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
    })
}

fn unbranch_to_depth_pipeline(device: &wgpu::Device) -> wgpu::RenderPipeline {
    let module = crate::shader::unbranch_to_depth::create_shader_module(device);
    let render_pipeline_layout = crate::shader::unbranch_to_depth::create_pipeline_layout(device);

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Unbranch to Depth Pipeline"),
        layout: Some(&render_pipeline_layout),
        vertex: crate::shader::unbranch_to_depth::vertex_state(
            &module,
            &crate::shader::unbranch_to_depth::vs_main_entry(),
        ),
        fragment: Some(wgpu::FragmentState {
            module: &module,
            entry_point: crate::shader::unbranch_to_depth::ENTRY_FS_MAIN,
            targets: &[],
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: Some(wgpu::DepthStencilState {
            format: MAT_ID_DEPTH_FORMAT,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Always,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
    })
}
