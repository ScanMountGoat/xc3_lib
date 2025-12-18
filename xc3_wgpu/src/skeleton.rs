use glam::vec4;
use wgpu::util::DeviceExt;

use crate::DEPTH_STENCIL_FORMAT;

pub struct BoneRenderer {
    vertex_buffer: wgpu::Buffer,
    pipeline: wgpu::RenderPipeline,
    bind_group0: crate::shader::bone::bind_groups::BindGroup0,
}

impl BoneRenderer {
    pub fn new(
        device: &wgpu::Device,
        camera_buffer: &wgpu::Buffer,
        format: wgpu::TextureFormat,
    ) -> Self {
        let vertex_buffer = axes_vertex_buffer(device);

        let module = crate::shader::bone::create_shader_module(device);
        let render_pipeline_layout = crate::shader::bone::create_pipeline_layout(device);

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Bone Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: crate::shader::bone::vertex_state(
                &module,
                &crate::shader::bone::vs_main_entry(
                    wgpu::VertexStepMode::Vertex,
                    wgpu::VertexStepMode::Instance,
                ),
            ),
            fragment: Some(crate::shader::bone::fragment_state(
                &module,
                &crate::shader::bone::fs_main_entry([Some(format.into())]),
            )),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                polygon_mode: wgpu::PolygonMode::Line,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_STENCIL_FORMAT,
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::Always,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let bind_group0 = crate::shader::bone::bind_groups::BindGroup0::from_bindings(
            device,
            crate::shader::bone::bind_groups::BindGroupLayout0 {
                camera: camera_buffer.as_entire_buffer_binding(),
            },
        );

        Self {
            vertex_buffer,
            pipeline,
            bind_group0,
        }
    }

    pub fn draw_bones<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        bone_transforms: &'a wgpu::Buffer,
        bone_count: u32,
    ) {
        if bone_count > 0 {
            render_pass.set_pipeline(&self.pipeline);

            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_vertex_buffer(1, bone_transforms.slice(..));

            crate::shader::bone::set_bind_groups(render_pass, &self.bind_group0);

            render_pass.draw(0..6, 0..bone_count);
        }
    }
}

pub fn axes_vertex_buffer(device: &wgpu::Device) -> wgpu::Buffer {
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Bone Axes Vertex Buffer"),
        contents: bytemuck::cast_slice(&[
            // X+
            crate::shader::bone::VertexInput {
                position: vec4(0.0, 0.0, 0.0, 1.0),
                normal: vec4(1.0, 0.0, 0.0, 1.0),
            },
            crate::shader::bone::VertexInput {
                position: vec4(1.0, 0.0, 0.0, 1.0),
                normal: vec4(1.0, 0.0, 0.0, 1.0),
            },
            // Y+
            crate::shader::bone::VertexInput {
                position: vec4(0.0, 0.0, 0.0, 1.0),
                normal: vec4(0.0, 1.0, 0.0, 1.0),
            },
            crate::shader::bone::VertexInput {
                position: vec4(0.0, 1.0, 0.0, 1.0),
                normal: vec4(0.0, 1.0, 0.0, 1.0),
            },
            // Z+
            crate::shader::bone::VertexInput {
                position: vec4(0.0, 0.0, 0.0, 1.0),
                normal: vec4(0.0, 0.0, 1.0, 1.0),
            },
            crate::shader::bone::VertexInput {
                position: vec4(0.0, 0.0, 1.0, 1.0),
                normal: vec4(0.0, 0.0, 1.0, 1.0),
            },
        ]),
        usage: wgpu::BufferUsages::VERTEX,
    })
}
