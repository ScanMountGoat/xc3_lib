use glam::{vec4, Mat4, Vec4};
use wgpu::util::DeviceExt;
use xc3_model::MeshRenderPass;

use crate::{
    model::ModelGroup, pipeline::Output5Type, skeleton::BoneRenderer, DeviceBufferExt,
    MonolibShaderTextures, QueueBufferExt, COLOR_FORMAT, GBUFFER_COLOR_FORMAT,
    GBUFFER_NORMAL_FORMAT,
};

const DEPTH_STENCIL_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth24PlusStencil8;
const MAT_ID_DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth16Unorm;

// TODO: Add fallback textures for all the monolib shader textures?
pub struct Renderer {
    camera_buffer: wgpu::Buffer,
    camera: CameraData,

    model_bind_group0: crate::shader::model::bind_groups::BindGroup0,

    deferred_debug_pipeline: wgpu::RenderPipeline,
    deferred_bind_group0: crate::shader::deferred::bind_groups::BindGroup0,
    debug_settings_buffer: wgpu::Buffer,

    deferred_pipelines: [wgpu::RenderPipeline; 6],
    deferred_bind_group2: [crate::shader::deferred::bind_groups::BindGroup2; 6],

    render_mode: RenderMode,

    textures: Textures,

    morph_pipeline: wgpu::ComputePipeline,

    unbranch_to_depth_pipeline: wgpu::RenderPipeline,

    snn_filter_pipeline: wgpu::RenderPipeline,

    blit_pipeline: wgpu::RenderPipeline,

    blit_hair_pipeline: wgpu::RenderPipeline,

    solid_pipeline: wgpu::RenderPipeline,
    solid_bind_group0: crate::shader::solid::bind_groups::BindGroup0,
    solid_bind_group1: crate::shader::solid::bind_groups::BindGroup1,
    solid_culled_bind_group1: crate::shader::solid::bind_groups::BindGroup1,

    bone_renderer: BoneRenderer,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum RenderMode {
    /// Full lighting and shading based on in game rendering.
    /// Rendering is based on Xenoblade 3 but is compatible with all 3 games.
    Shaded = 0,
    /// Debug the first gbuffer texture "gtCol".
    GBuffer0 = 1,
    /// Debug the second gbuffer texture "gtEtc".
    GBuffer1 = 2,
    /// Debug the third gbuffer texture "gtNom".
    GBuffer2 = 3,
    /// Debug the fourth gbuffer texture "gtVelocity".
    GBuffer3 = 4,
    /// Debug the fifth gbuffer texture "gtDep".
    GBuffer4 = 5,
    /// Debug the sixth gbuffer texture "MrtLgtColor".
    GBuffer5 = 6,
    /// Debug the sixth gbuffer texture "gtSpecularCol".
    GBuffer6 = 7,
}

// Group resizable resources to avoid duplicating this logic.
pub struct Textures {
    depth_stencil: wgpu::TextureView,
    mat_id_depth: wgpu::TextureView,
    deferred_output: wgpu::TextureView,
    gbuffer: GBuffer,
    deferred_bind_group1: crate::shader::deferred::bind_groups::BindGroup1,
    unbranch_to_depth_bind_group0: crate::shader::unbranch_to_depth::bind_groups::BindGroup0,
    snn_filter_output: wgpu::TextureView,
    snn_filter_bind_group0: crate::shader::snn_filter::bind_groups::BindGroup0,
    blit_deferred_bind_group: crate::shader::blit::bind_groups::BindGroup0,
    blit_hair_bind_group: crate::shader::blit::bind_groups::BindGroup0,
}

impl Textures {
    fn new(device: &wgpu::Device, width: u32, height: u32) -> Self {
        let depth_stencil =
            create_texture(device, width, height, "depth_texture", DEPTH_STENCIL_FORMAT);
        let mat_id_depth_view = create_texture(
            device,
            width,
            height,
            "material ID depth texture",
            MAT_ID_DEPTH_FORMAT,
        );
        let gbuffer = create_gbuffer(device, width, height);
        let deferred_bind_group1 = create_deferred_bind_group1(device, &gbuffer);
        let unbranch_to_depth_bind_group0 = create_unbranch_to_depth_bindgroup(device, &gbuffer);

        // TODO: This uses a higher precision floating point format in game?
        // TODO: Does this need to support HDR for bloom?
        let deferred_output = create_texture(device, width, height, "GBuffer Output", COLOR_FORMAT);
        let snn_filter_output =
            create_texture(device, width, height, "SNN Filter Output", COLOR_FORMAT);

        let snn_filter_bind_group0 =
            create_snn_filter_bindgroup(device, &gbuffer, &deferred_output);

        let blit_hair_bind_group = create_blit_bindgroup(device, &snn_filter_output);
        let blit_deferred_bind_group = create_blit_bindgroup(device, &deferred_output);

        Self {
            depth_stencil,
            mat_id_depth: mat_id_depth_view,
            deferred_output,
            gbuffer,
            deferred_bind_group1,
            unbranch_to_depth_bind_group0,
            snn_filter_output,
            snn_filter_bind_group0,
            blit_hair_bind_group,
            blit_deferred_bind_group,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraData {
    pub view: Mat4,
    pub projection: Mat4,
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
    // TODO: What is this called in game?
    spec_color: wgpu::TextureView,
}

impl Renderer {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        width: u32,
        height: u32,
        surface_format: wgpu::TextureFormat,
        monolib_shader: &MonolibShaderTextures,
    ) -> Self {
        let camera = CameraData {
            view: Mat4::IDENTITY,
            projection: Mat4::IDENTITY,
            view_projection: Mat4::IDENTITY,
            position: Vec4::ZERO,
        };
        let camera_buffer = device.create_uniform_buffer(
            "camera buffer",
            &crate::shader::model::Camera {
                view: Mat4::IDENTITY,
                view_projection: Mat4::IDENTITY,
                position: Vec4::ZERO,
            },
        );

        let model_bind_group0 = crate::shader::model::bind_groups::BindGroup0::from_bindings(
            device,
            crate::shader::model::bind_groups::BindGroupLayout0 {
                camera: camera_buffer.as_entire_buffer_binding(),
            },
        );

        let render_mode = RenderMode::Shaded;
        let debug_settings_buffer = device.create_uniform_buffer(
            "Debug Settings",
            &crate::shader::deferred::DebugSettings {
                render_mode: render_mode as u32,
                channel: -1,
            },
        );

        let shared_sampler = device.create_sampler(&wgpu::SamplerDescriptor::default());

        // TODO: Why is the toon grad mip count not correct?
        let deferred_bind_group0 = crate::shader::deferred::bind_groups::BindGroup0::from_bindings(
            device,
            crate::shader::deferred::bind_groups::BindGroupLayout0 {
                debug_settings: debug_settings_buffer.as_entire_buffer_binding(),
                g_toon_grad: &monolib_shader
                    .toon_grad
                    .as_ref()
                    .map(|t| {
                        t.create_view(&wgpu::TextureViewDescriptor {
                            mip_level_count: Some(1),
                            ..Default::default()
                        })
                    })
                    .unwrap_or_else(|| {
                        default_toon_grad(device, queue).create_view(&wgpu::TextureViewDescriptor {
                            mip_level_count: Some(1),
                            ..Default::default()
                        })
                    }),
                shared_sampler: &shared_sampler,
            },
        );

        // TODO: Is is better to just create separate pipelines?
        let deferred_bind_group2 = [0, 1, 2, 3, 4, 5].map(|mat_id| {
            let buffer = device.create_uniform_buffer(
                "Render Settings",
                &crate::shader::deferred::RenderSettings { mat_id },
            );

            crate::shader::deferred::bind_groups::BindGroup2::from_bindings(
                device,
                crate::shader::deferred::bind_groups::BindGroupLayout2 {
                    render_settings: buffer.as_entire_buffer_binding(),
                },
            )
        });

        let morph_pipeline = crate::shader::morph::compute::create_main_pipeline(device);

        let unbranch_to_depth_pipeline = unbranch_to_depth_pipeline(device);

        // TODO: These should all be different entry points?
        let deferred_pipelines = [
            deferred_pipeline(device, crate::shader::deferred::ENTRY_FS_MAIN),
            deferred_pipeline(device, crate::shader::deferred::ENTRY_FS_MAIN),
            deferred_pipeline(device, crate::shader::deferred::ENTRY_FS_TOON),
            deferred_pipeline(device, crate::shader::deferred::ENTRY_FS_MAIN),
            deferred_pipeline(device, crate::shader::deferred::ENTRY_FS_MAIN),
            deferred_pipeline(device, crate::shader::deferred::ENTRY_FS_TOON),
        ];

        let deferred_debug_pipeline = deferred_debug_pipeline(device);

        let snn_filter_pipeline = snn_filter_pipeline(device);

        let blit_pipeline = blit_pipeline(device, surface_format);
        let blit_hair_pipeline = blit_hair_pipeline(device, surface_format);

        let textures = Textures::new(device, width, height);

        let solid_pipeline = solid_pipeline(device, surface_format);
        let solid_bind_group0 = crate::shader::solid::bind_groups::BindGroup0::from_bindings(
            device,
            crate::shader::solid::bind_groups::BindGroupLayout0 {
                camera: camera_buffer.as_entire_buffer_binding(),
            },
        );

        let uniforms_buffer = device.create_uniform_buffer(
            "bounds uniform buffer",
            &crate::shader::solid::Uniforms {
                color: vec4(1.0, 1.0, 1.0, 1.0),
            },
        );

        let solid_bind_group1 = crate::shader::solid::bind_groups::BindGroup1::from_bindings(
            device,
            crate::shader::solid::bind_groups::BindGroupLayout1 {
                uniforms: uniforms_buffer.as_entire_buffer_binding(),
            },
        );

        // Use a distinctive color to indicate the culled state.
        let culled_uniforms_buffer = device.create_uniform_buffer(
            "bounds culled uniform buffer",
            &crate::shader::solid::Uniforms {
                color: vec4(1.0, 0.0, 0.0, 1.0),
            },
        );

        let solid_culled_bind_group1 = crate::shader::solid::bind_groups::BindGroup1::from_bindings(
            device,
            crate::shader::solid::bind_groups::BindGroupLayout1 {
                uniforms: culled_uniforms_buffer.as_entire_buffer_binding(),
            },
        );

        let bone_renderer = BoneRenderer::new(device, &camera_buffer, surface_format);

        Self {
            camera_buffer,
            camera,
            model_bind_group0,
            deferred_pipelines,
            deferred_debug_pipeline,
            deferred_bind_group0,
            deferred_bind_group2,
            debug_settings_buffer,
            morph_pipeline,
            unbranch_to_depth_pipeline,
            textures,
            render_mode,
            snn_filter_pipeline,
            blit_pipeline,
            blit_hair_pipeline,
            solid_pipeline,
            solid_bind_group0,
            solid_bind_group1,
            solid_culled_bind_group1,
            bone_renderer,
        }
    }

    pub fn render_models(
        &self,
        output_view: &wgpu::TextureView,
        encoder: &mut wgpu::CommandEncoder,
        models: &[ModelGroup],
        draw_bounds: bool,
        draw_bones: bool,
    ) {
        // The passes and their ordering only loosely matches in game.
        // This enables better performance, portability, etc.
        self.compute_morphs(encoder, models);

        // TODO: changing the texture for output5 requires a new render pass?
        // TODO: does the in game rendering group these in any meaningful way?
        self.opaque_pass(encoder, models);
        self.alpha1_pass(encoder, models);
        self.alpha2_pass(encoder, models);
        self.unbranch_to_depth_pass(encoder);
        if self.render_mode == RenderMode::Shaded {
            self.deferred_pass(encoder);
            self.alpha3_pass(encoder, models, &self.textures.deferred_output);
            self.snn_filter_pass(encoder);
        } else {
            // Move forward passes earlier to show all meshes in debug modes.
            self.alpha3_pass(encoder, models, &self.textures.gbuffer.color);
            self.deferred_debug_pass(encoder);
        }
        self.final_pass(encoder, output_view, models, draw_bounds, draw_bones);
    }

    pub fn update_camera(&mut self, queue: &wgpu::Queue, camera_data: &CameraData) {
        queue.write_uniform_data(
            &self.camera_buffer,
            &crate::shader::model::Camera {
                view: camera_data.view,
                view_projection: camera_data.view_projection,
                position: camera_data.position,
            },
        );
        self.camera = *camera_data;
    }

    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        // Update each resource that depends on window size.
        self.textures = Textures::new(device, width, height);
    }

    pub fn update_debug_settings(
        &mut self,
        queue: &wgpu::Queue,
        render_mode: RenderMode,
        channel: i32,
    ) {
        self.render_mode = render_mode;
        queue.write_uniform_data(
            &self.debug_settings_buffer,
            &crate::shader::deferred::DebugSettings {
                render_mode: render_mode as u32,
                channel,
            },
        );
    }

    fn opaque_pass(&self, encoder: &mut wgpu::CommandEncoder, models: &[ModelGroup]) {
        // TODO: Interleave emissive and specular passes?
        let mut pass = self.begin_opaque_pass(encoder, Output5Type::Emission, false);
        self.model_bind_group0.set(&mut pass);

        for model in models {
            model.draw(
                &mut pass,
                true,
                MeshRenderPass::Unk1,
                &self.camera,
                Some(Output5Type::Emission),
            );
            model.draw(
                &mut pass,
                true,
                MeshRenderPass::Unk0,
                &self.camera,
                Some(Output5Type::Emission),
            );
            // TODO: Where is this supposed to go?
            model.draw(
                &mut pass,
                true,
                MeshRenderPass::Unk4,
                &self.camera,
                Some(Output5Type::Emission),
            );
        }
        drop(pass);

        let mut render_pass = self.begin_opaque_pass(encoder, Output5Type::Specular, true);
        self.model_bind_group0.set(&mut render_pass);

        for model in models {
            model.draw(
                &mut render_pass,
                true,
                MeshRenderPass::Unk1,
                &self.camera,
                Some(Output5Type::Specular),
            );
            model.draw(
                &mut render_pass,
                true,
                MeshRenderPass::Unk0,
                &self.camera,
                Some(Output5Type::Specular),
            );
            // TODO: Where is this supposed to go?
            model.draw(
                &mut render_pass,
                true,
                MeshRenderPass::Unk4,
                &self.camera,
                Some(Output5Type::Specular),
            );
        }
    }

    fn begin_opaque_pass<'a>(
        &'a self,
        encoder: &'a mut wgpu::CommandEncoder,
        output5_type: Output5Type,
        load: bool,
    ) -> wgpu::RenderPass<'a> {
        let attachment = |t, c| {
            if load {
                color_attachment_load(t)
            } else {
                color_attachment(t, c)
            }
        };

        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: match output5_type {
                Output5Type::Specular => Some("Model Pass Spec"),
                Output5Type::Emission => Some("Model Pass Emi"),
            },
            color_attachments: &[
                attachment(&self.textures.gbuffer.color, wgpu::Color::TRANSPARENT),
                attachment(&self.textures.gbuffer.etc_buffer, wgpu::Color::TRANSPARENT),
                attachment(
                    &self.textures.gbuffer.normal,
                    wgpu::Color {
                        r: 0.5,
                        g: 0.5,
                        b: 1.0,
                        a: 1.0,
                    },
                ),
                attachment(&self.textures.gbuffer.velocity, wgpu::Color::TRANSPARENT),
                attachment(
                    &self.textures.gbuffer.depth,
                    wgpu::Color {
                        r: 1.0,
                        g: 1.0,
                        b: 1.0,
                        a: 0.0,
                    },
                ),
                match output5_type {
                    Output5Type::Specular => {
                        // Always clear specular since it hasn't been rendered to yet.
                        color_attachment(
                            &self.textures.gbuffer.spec_color,
                            wgpu::Color::TRANSPARENT,
                        )
                    }
                    Output5Type::Emission => {
                        attachment(&self.textures.gbuffer.lgt_color, wgpu::Color::TRANSPARENT)
                    }
                },
            ],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.textures.depth_stencil,
                depth_ops: Some(wgpu::Operations {
                    load: if load {
                        wgpu::LoadOp::Load
                    } else {
                        wgpu::LoadOp::Clear(1.0)
                    },
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: Some(wgpu::Operations {
                    load: if load {
                        wgpu::LoadOp::Load
                    } else {
                        wgpu::LoadOp::Clear(0)
                    },
                    store: wgpu::StoreOp::Store,
                }),
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        })
    }

    fn alpha1_pass(&self, encoder: &mut wgpu::CommandEncoder, models: &[ModelGroup]) {
        // Deferred rendering requires a second forward pass for transparent meshes.
        // This pass only writes to the color output.
        // TODO: Research more about how this is implemented in game.
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Alpha Pass 1"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.textures.gbuffer.color,
                resolve_target: None,
                ops: wgpu::Operations {
                    // TODO: Does in game actually use load?
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.textures.depth_stencil,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    // TODO: Write to depth buffer?
                    store: wgpu::StoreOp::Store,
                }),
                // TODO: Does this pass ever write to the stencil buffer?
                stencil_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                }),
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        // TODO: organize into per frame, per model, etc?
        self.model_bind_group0.set(&mut render_pass);

        // TODO: Is this the correct unk type?
        for model in models {
            model.draw(
                &mut render_pass,
                false,
                MeshRenderPass::Unk8,
                &self.camera,
                None,
            );
        }
    }

    // TODO: Share code for drawing?
    fn alpha2_pass(&self, encoder: &mut wgpu::CommandEncoder, models: &[ModelGroup]) {
        // Deferred rendering requires a second forward pass for transparent meshes.
        // This pass writes to all outputs.
        // TODO: Research more about how this is implemented in game.
        let mut render_pass = self.begin_alpha2_pass(encoder, Output5Type::Emission);

        // TODO: organize into per frame, per model, etc?
        self.model_bind_group0.set(&mut render_pass);

        // TODO: Is this the correct unk type?
        for model in models {
            model.draw(
                &mut render_pass,
                true,
                MeshRenderPass::Unk8,
                &self.camera,
                Some(Output5Type::Emission),
            );
        }
        drop(render_pass);

        // TODO: Share code with above.
        let mut render_pass = self.begin_alpha2_pass(encoder, Output5Type::Specular);

        // TODO: organize into per frame, per model, etc?
        self.model_bind_group0.set(&mut render_pass);

        // TODO: Is this the correct unk type?
        for model in models {
            model.draw(
                &mut render_pass,
                true,
                MeshRenderPass::Unk8,
                &self.camera,
                Some(Output5Type::Specular),
            );
        }
        drop(render_pass);
    }

    // TODO: This can share code with the opaque pass?
    fn begin_alpha2_pass<'a>(
        &'a self,
        encoder: &'a mut wgpu::CommandEncoder,
        output5_type: Output5Type,
    ) -> wgpu::RenderPass<'a> {
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: match output5_type {
                Output5Type::Specular => Some("Alpha Pass 2 Spec"),
                Output5Type::Emission => Some("Alpha Pass 2 Emi"),
            },
            color_attachments: &[
                color_attachment_load(&self.textures.gbuffer.color),
                color_attachment_load(&self.textures.gbuffer.etc_buffer),
                color_attachment_load(&self.textures.gbuffer.normal),
                color_attachment_load(&self.textures.gbuffer.velocity),
                color_attachment_load(&self.textures.gbuffer.depth),
                match output5_type {
                    Output5Type::Specular => {
                        color_attachment_load(&self.textures.gbuffer.spec_color)
                    }
                    Output5Type::Emission => {
                        color_attachment_load(&self.textures.gbuffer.lgt_color)
                    }
                },
            ],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.textures.depth_stencil,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                }),
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        })
    }

    fn alpha3_pass(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        models: &[ModelGroup],
        output_view: &wgpu::TextureView,
    ) {
        // Deferred rendering requires a second forward pass for transparent meshes.
        // The transparent pass only writes to the color output.
        // TODO: Research more about how this is implemented in game.
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Alpha Pass 3"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: output_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    // TODO: Does in game actually use load?
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.textures.depth_stencil,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    // TODO: Write to depth buffer?
                    store: wgpu::StoreOp::Store,
                }),
                // TODO: Does this pass ever write to the stencil buffer?
                stencil_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                }),
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        // TODO: organize into per frame, per model, etc?
        self.model_bind_group0.set(&mut render_pass);

        // TODO: Is this the correct pass type?
        for model in models {
            model.draw(
                &mut render_pass,
                false,
                MeshRenderPass::Unk2,
                &self.camera,
                None,
            );
            // TODO: 0x21 is single output after deferred in xcx?
            // TODO: Test how this actually works in game.
            model.draw(
                &mut render_pass,
                false,
                MeshRenderPass::Unk1,
                &self.camera,
                None,
            );
        }
    }

    fn unbranch_to_depth_pass(&self, encoder: &mut wgpu::CommandEncoder) {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Unbranch to Depth Pass"),
            color_attachments: &[],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.textures.mat_id_depth,
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

        crate::shader::unbranch_to_depth::set_bind_groups(
            &mut render_pass,
            &self.textures.unbranch_to_depth_bind_group0,
        );

        render_pass.draw(0..3, 0..1);
    }

    fn deferred_pass(&self, encoder: &mut wgpu::CommandEncoder) {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Deferred Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.textures.deferred_output,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.textures.mat_id_depth,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        for (pipeline, bind_group2) in self
            .deferred_pipelines
            .iter()
            .zip(&self.deferred_bind_group2)
        {
            // Each material ID type renders with a separate pipeline in game.
            render_pass.set_pipeline(pipeline);

            crate::shader::deferred::set_bind_groups(
                &mut render_pass,
                &self.deferred_bind_group0,
                &self.textures.deferred_bind_group1,
                bind_group2,
            );

            render_pass.draw(0..3, 0..1);
        }
    }

    fn deferred_debug_pass(&self, encoder: &mut wgpu::CommandEncoder) {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Deferred Debug Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.textures.deferred_output,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.textures.mat_id_depth,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        render_pass.set_pipeline(&self.deferred_debug_pipeline);

        crate::shader::deferred::set_bind_groups(
            &mut render_pass,
            &self.deferred_bind_group0,
            &self.textures.deferred_bind_group1,
            &self.deferred_bind_group2[0],
        );

        render_pass.draw(0..3, 0..1);
    }

    fn snn_filter_pass(&self, encoder: &mut wgpu::CommandEncoder) {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Hair SNN Filter Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.textures.snn_filter_output,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.textures.depth_stencil,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                }),
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        render_pass.set_pipeline(&self.snn_filter_pipeline);

        render_pass.set_stencil_reference(0x40);

        crate::shader::snn_filter::set_bind_groups(
            &mut render_pass,
            &self.textures.snn_filter_bind_group0,
        );

        render_pass.draw(0..3, 0..1);
    }

    fn final_pass(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        output_view: &wgpu::TextureView,
        groups: &[ModelGroup],
        draw_bounds: bool,
        draw_bones: bool,
    ) {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Final Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: output_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.textures.depth_stencil,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                }),
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        self.blit_deferred(&mut render_pass);
        if self.render_mode == RenderMode::Shaded {
            self.blit_snn_filtered_hair(&mut render_pass);
        }

        // TODO: Some eye meshes draw in this pass?

        // TODO: Create a BoundsRenderer to store this data?
        if draw_bounds {
            render_pass.set_pipeline(&self.solid_pipeline);
            self.solid_bind_group0.set(&mut render_pass);

            for group in groups {
                group.draw_bounds(
                    &mut render_pass,
                    &self.solid_bind_group1,
                    &self.solid_culled_bind_group1,
                    &self.camera,
                );
            }
        }

        if draw_bones {
            for group in groups {
                self.bone_renderer.draw_bones(
                    &mut render_pass,
                    &group.bone_animated_transforms,
                    group.bone_count as u32,
                );
            }
        }
    }

    fn blit_deferred<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        render_pass.set_pipeline(&self.blit_pipeline);
        render_pass.set_stencil_reference(0x00);
        crate::shader::blit::set_bind_groups(render_pass, &self.textures.blit_deferred_bind_group);
        render_pass.draw(0..3, 0..1);
    }

    fn blit_snn_filtered_hair<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        render_pass.set_pipeline(&self.blit_hair_pipeline);
        render_pass.set_stencil_reference(0x40);
        crate::shader::blit::set_bind_groups(render_pass, &self.textures.blit_hair_bind_group);
        render_pass.draw(0..3, 0..1);
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
            g_depth: &gbuffer.depth,
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
            g_specular_color: &gbuffer.spec_color,
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

fn color_attachment_load(view: &wgpu::TextureView) -> Option<wgpu::RenderPassColorAttachment> {
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
        color: create_texture(device, width, height, "g_color", GBUFFER_COLOR_FORMAT),
        etc_buffer: create_texture(device, width, height, "g_etc_buffer", GBUFFER_COLOR_FORMAT),
        normal: create_texture(device, width, height, "g_normal", GBUFFER_NORMAL_FORMAT),
        velocity: create_texture(device, width, height, "g_velocity", GBUFFER_COLOR_FORMAT),
        depth: create_texture(device, width, height, "g_depth", GBUFFER_COLOR_FORMAT),
        lgt_color: create_texture(device, width, height, "g_lgt_color", GBUFFER_COLOR_FORMAT),
        spec_color: create_texture(
            device,
            width,
            height,
            "g_specular_color",
            GBUFFER_COLOR_FORMAT,
        ),
    }
}

fn create_texture(
    device: &wgpu::Device,
    width: u32,
    height: u32,
    label: &str,
    format: wgpu::TextureFormat,
) -> wgpu::TextureView {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });

    texture.create_view(&Default::default())
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
            compilation_options: Default::default(),
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
        cache: None,
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
        fragment: Some(crate::shader::deferred::fragment_state(
            &module,
            &crate::shader::deferred::fs_debug_entry([Some(wgpu::ColorTargetState {
                format: COLOR_FORMAT,
                blend: None,
                write_mask: wgpu::ColorWrites::all(),
            })]),
        )),
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
        cache: None,
    })
}

fn solid_pipeline(device: &wgpu::Device, format: wgpu::TextureFormat) -> wgpu::RenderPipeline {
    let module = crate::shader::solid::create_shader_module(device);
    let render_pipeline_layout = crate::shader::solid::create_pipeline_layout(device);

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Unbranch to Depth Pipeline"),
        layout: Some(&render_pipeline_layout),
        vertex: crate::shader::solid::vertex_state(
            &module,
            &crate::shader::solid::vs_main_entry(wgpu::VertexStepMode::Vertex),
        ),
        fragment: Some(crate::shader::solid::fragment_state(
            &module,
            &crate::shader::solid::fs_main_entry([Some(format.into())]),
        )),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::LineList,
            polygon_mode: wgpu::PolygonMode::Line,
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_STENCIL_FORMAT,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::LessEqual,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
        cache: None,
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
        fragment: Some(crate::shader::unbranch_to_depth::fragment_state(
            &module,
            &crate::shader::unbranch_to_depth::fs_main_entry([]),
        )),
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
        cache: None,
    })
}

// TODO: Create a function for simplifying stencil state creation.
fn snn_filter_pipeline(device: &wgpu::Device) -> wgpu::RenderPipeline {
    let module = crate::shader::snn_filter::create_shader_module(device);
    let render_pipeline_layout = crate::shader::snn_filter::create_pipeline_layout(device);

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("SNN Filter Pipeline"),
        layout: Some(&render_pipeline_layout),
        vertex: crate::shader::snn_filter::vertex_state(
            &module,
            &crate::shader::snn_filter::vs_main_entry(),
        ),
        fragment: Some(crate::shader::snn_filter::fragment_state(
            &module,
            &crate::shader::snn_filter::fs_main_entry([Some(COLOR_FORMAT.into())]),
        )),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_STENCIL_FORMAT,
            depth_write_enabled: false,
            depth_compare: wgpu::CompareFunction::Always,
            stencil: wgpu::StencilState {
                front: wgpu::StencilFaceState {
                    compare: wgpu::CompareFunction::Equal,
                    fail_op: wgpu::StencilOperation::Keep,
                    depth_fail_op: wgpu::StencilOperation::Keep,
                    pass_op: wgpu::StencilOperation::Keep,
                },
                back: wgpu::StencilFaceState {
                    compare: wgpu::CompareFunction::Equal,
                    fail_op: wgpu::StencilOperation::Keep,
                    depth_fail_op: wgpu::StencilOperation::Keep,
                    pass_op: wgpu::StencilOperation::Keep,
                },
                read_mask: 0x40,
                write_mask: 0x00,
            },
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
        cache: None,
    })
}

fn blit_hair_pipeline(device: &wgpu::Device, format: wgpu::TextureFormat) -> wgpu::RenderPipeline {
    let module = crate::shader::blit::create_shader_module(device);
    let render_pipeline_layout = crate::shader::blit::create_pipeline_layout(device);

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Blit Hair Pipeline"),
        layout: Some(&render_pipeline_layout),
        vertex: crate::shader::blit::vertex_state(&module, &crate::shader::blit::vs_main_entry()),
        fragment: Some(crate::shader::blit::fragment_state(
            &module,
            &crate::shader::blit::fs_main_entry([Some(format.into())]),
        )),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_STENCIL_FORMAT,
            depth_write_enabled: false,
            depth_compare: wgpu::CompareFunction::Always,
            stencil: wgpu::StencilState {
                front: wgpu::StencilFaceState {
                    compare: wgpu::CompareFunction::Equal,
                    fail_op: wgpu::StencilOperation::Keep,
                    depth_fail_op: wgpu::StencilOperation::Keep,
                    pass_op: wgpu::StencilOperation::Keep,
                },
                back: wgpu::StencilFaceState {
                    compare: wgpu::CompareFunction::Equal,
                    fail_op: wgpu::StencilOperation::Keep,
                    depth_fail_op: wgpu::StencilOperation::Keep,
                    pass_op: wgpu::StencilOperation::Keep,
                },
                read_mask: 0x40,
                write_mask: 0x00,
            },
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
        cache: None,
    })
}

fn blit_pipeline(device: &wgpu::Device, format: wgpu::TextureFormat) -> wgpu::RenderPipeline {
    let module = crate::shader::blit::create_shader_module(device);
    let render_pipeline_layout = crate::shader::blit::create_pipeline_layout(device);

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Blit Pipeline"),
        layout: Some(&render_pipeline_layout),
        vertex: crate::shader::blit::vertex_state(&module, &crate::shader::blit::vs_main_entry()),
        fragment: Some(crate::shader::blit::fragment_state(
            &module,
            &crate::shader::blit::fs_main_entry([Some(format.into())]),
        )),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_STENCIL_FORMAT,
            depth_write_enabled: false,
            depth_compare: wgpu::CompareFunction::Always,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
        cache: None,
    })
}

fn create_snn_filter_bindgroup(
    device: &wgpu::Device,
    gbuffer: &GBuffer,
    output: &wgpu::TextureView,
) -> crate::shader::snn_filter::bind_groups::BindGroup0 {
    let shared_sampler = device.create_sampler(&wgpu::SamplerDescriptor::default());

    // This uses the deferred pass output instead of the GBuffer color texture.
    crate::shader::snn_filter::bind_groups::BindGroup0::from_bindings(
        device,
        crate::shader::snn_filter::bind_groups::BindGroupLayout0 {
            g_color: output,
            g_depth: &gbuffer.depth,
            shared_sampler: &shared_sampler,
        },
    )
}

fn create_blit_bindgroup(
    device: &wgpu::Device,
    input: &wgpu::TextureView,
) -> crate::shader::blit::bind_groups::BindGroup0 {
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor::default());

    crate::shader::blit::bind_groups::BindGroup0::from_bindings(
        device,
        crate::shader::blit::bind_groups::BindGroupLayout0 {
            color: input,
            color_sampler: &sampler,
        },
    )
}

pub fn default_toon_grad(device: &wgpu::Device, queue: &wgpu::Queue) -> wgpu::Texture {
    device.create_texture_with_data(
        queue,
        &wgpu::TextureDescriptor {
            label: Some("toon grad default"),
            size: wgpu::Extent3d {
                width: 4,
                height: 4,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        },
        wgpu::util::TextureDataOrder::LayerMajor,
        &[255u8; 4 * 4 * 4],
    )
}
