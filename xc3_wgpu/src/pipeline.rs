use crate::{DEPTH_FORMAT, GBUFFER_COLOR_FORMAT};

pub fn model_pipeline(device: &wgpu::Device) -> wgpu::RenderPipeline {
    let module = crate::shader::model::create_shader_module(device);
    let render_pipeline_layout = crate::shader::model::create_pipeline_layout(device);

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Render Pipeline"),
        layout: Some(&render_pipeline_layout),
        vertex: crate::shader::model::vertex_state(
            &module,
            &crate::shader::model::vs_main_entry(wgpu::VertexStepMode::Vertex),
        ),
        fragment: Some(wgpu::FragmentState {
            module: &module,
            entry_point: crate::shader::model::ENTRY_FS_MAIN,
            // TODO: alpha blending?
            // Create a target for each of the G-Buffer textures.
            // TODO: check outputs in wgsl_to_wgpu?
            targets: &vec![
                Some(wgpu::ColorTargetState {
                    format: GBUFFER_COLOR_FORMAT,
                    blend: None,
                    write_mask: wgpu::ColorWrites::all(),
                });
                7
            ],
        }),
        primitive: wgpu::PrimitiveState {
            // TODO: Do all meshes using indexed triangle lists?
            topology: wgpu::PrimitiveTopology::TriangleList,
            polygon_mode: wgpu::PolygonMode::Fill,
            cull_mode: Some(wgpu::Face::Back),
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::LessEqual,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
    })
}
