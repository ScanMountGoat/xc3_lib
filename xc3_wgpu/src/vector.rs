use crate::DEPTH_STENCIL_FORMAT;

// TODO: come up with a better name than "vector"
pub struct VectorRenderer {
    pipeline: wgpu::RenderPipeline,
    bind_group0: crate::shader::vector::bind_groups::BindGroup0,
}

impl VectorRenderer {
    pub fn new(
        device: &wgpu::Device,
        camera_buffer: &wgpu::Buffer,
        format: wgpu::TextureFormat,
    ) -> Self {
        let module = crate::shader::vector::create_shader_module(device);
        let render_pipeline_layout = crate::shader::vector::create_pipeline_layout(device);

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Vector Debug Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: crate::shader::vertex_state(
                &module,
                &crate::shader::vector::vs_main_entry(wgpu::VertexStepMode::Vertex),
            ),
            fragment: Some(crate::shader::fragment_state(
                &module,
                &crate::shader::vector::fs_main_entry([Some(format.into())]),
            )),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                polygon_mode: wgpu::PolygonMode::Line,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_STENCIL_FORMAT,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Always),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let bind_group0 = crate::shader::vector::bind_groups::BindGroup0::from_bindings(
            device,
            crate::shader::vector::bind_groups::BindGroupLayout0 {
                camera: camera_buffer.as_entire_buffer_binding(),
            },
        );

        Self {
            pipeline,
            bind_group0,
        }
    }

    pub fn draw_vectors<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        vertex_buffer: &'a wgpu::Buffer,
        vertex_count: u32,
    ) {
        if vertex_count > 0 {
            render_pass.set_pipeline(&self.pipeline);

            render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));

            crate::shader::vector::set_bind_groups(render_pass, &self.bind_group0);

            render_pass.draw(0..vertex_count, 0..1);
        }
    }
}
