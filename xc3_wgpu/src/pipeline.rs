use xc3_model::{BlendState, CullMode, ShaderUnkType, StateFlags};

use crate::{DEPTH_FORMAT, GBUFFER_COLOR_FORMAT};

#[derive(Debug)]
pub struct ModelPipelineData {
    module: wgpu::ShaderModule,
    layout: wgpu::PipelineLayout,
}

impl ModelPipelineData {
    pub fn new(device: &wgpu::Device) -> Self {
        let module = crate::shader::model::create_shader_module(device);
        let layout = crate::shader::model::create_pipeline_layout(device);
        Self { module, layout }
    }
}

/// The non shared components of a pipeline for use with pipeline caching.
#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq)]
pub struct PipelineKey {
    pub unk_type: ShaderUnkType,
    pub flags: StateFlags,
    pub is_outline: bool,
}

impl PipelineKey {
    pub fn write_to_all_outputs(&self) -> bool {
        self.unk_type == ShaderUnkType::Unk0
    }
}

// TODO: Always set depth and stencil state?
pub fn model_pipeline(
    device: &wgpu::Device,
    data: &ModelPipelineData,
    key: &PipelineKey,
) -> wgpu::RenderPipeline {
    // Some shaders only write to the albedo output.
    // TODO: Is there a better of handling this than modifying the render pass?
    let targets = if key.write_to_all_outputs() {
        // TODO: alpha blending?
        // Create a target for each of the G-Buffer textures.
        // TODO: check outputs in wgsl_to_wgpu?
        // TODO: Constant in wgsl for output count?
        vec![
            Some(wgpu::ColorTargetState {
                format: GBUFFER_COLOR_FORMAT,
                blend: None,
                write_mask: wgpu::ColorWrites::all(),
            });
            6
        ]
    } else {
        vec![
            Some(wgpu::ColorTargetState {
                format: GBUFFER_COLOR_FORMAT,
                blend: blend_state(key.flags.blend_state),
                write_mask: wgpu::ColorWrites::all(),
            }),
            Some(wgpu::ColorTargetState {
                format: GBUFFER_COLOR_FORMAT,
                blend: None,
                write_mask: wgpu::ColorWrites::empty(),
            }),
            Some(wgpu::ColorTargetState {
                format: GBUFFER_COLOR_FORMAT,
                blend: None,
                write_mask: wgpu::ColorWrites::empty(),
            }),
            Some(wgpu::ColorTargetState {
                format: GBUFFER_COLOR_FORMAT,
                blend: None,
                write_mask: wgpu::ColorWrites::empty(),
            }),
            Some(wgpu::ColorTargetState {
                format: GBUFFER_COLOR_FORMAT,
                blend: None,
                write_mask: wgpu::ColorWrites::empty(),
            }),
            Some(wgpu::ColorTargetState {
                format: GBUFFER_COLOR_FORMAT,
                blend: None,
                write_mask: wgpu::ColorWrites::empty(),
            }),
        ]
    };

    let vertex_entry = if key.is_outline {
        crate::shader::model::vs_outline_main_entry(
            wgpu::VertexStepMode::Vertex,
            wgpu::VertexStepMode::Vertex,
            wgpu::VertexStepMode::Instance,
        )
    } else {
        crate::shader::model::vs_main_entry(
            wgpu::VertexStepMode::Vertex,
            wgpu::VertexStepMode::Vertex,
            wgpu::VertexStepMode::Instance,
        )
    };
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Model Pipeline"),
        layout: Some(&data.layout),
        vertex: crate::shader::model::vertex_state(&data.module, &vertex_entry),
        fragment: Some(wgpu::FragmentState {
            module: &data.module,
            entry_point: crate::shader::model::ENTRY_FS_MAIN,
            targets: &targets,
        }),
        primitive: wgpu::PrimitiveState {
            // TODO: Do all meshes using indexed triangle lists?
            topology: wgpu::PrimitiveTopology::TriangleList,
            polygon_mode: wgpu::PolygonMode::Fill,
            cull_mode: cull_mode(key.flags.cull_mode),
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

fn cull_mode(mode: CullMode) -> Option<wgpu::Face> {
    match mode {
        CullMode::Back => Some(wgpu::Face::Back),
        CullMode::Front => Some(wgpu::Face::Front),
        CullMode::Disabled => None,
        CullMode::Unk3 => Some(wgpu::Face::Front),
    }
}

fn blend_state(state: BlendState) -> Option<wgpu::BlendState> {
    match state {
        BlendState::Disabled => None,
        BlendState::AlphaBlend => Some(wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::SrcAlpha,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::SrcAlpha,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,
            },
        }),
        BlendState::Additive => Some(wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::SrcAlpha,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::SrcAlpha,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
        }),
        BlendState::Multiplicative => Some(wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::Zero,
                dst_factor: wgpu::BlendFactor::Src,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::Zero,
                dst_factor: wgpu::BlendFactor::Src,
                operation: wgpu::BlendOperation::Add,
            },
        }),
        BlendState::Unk6 => None,
    }
}
