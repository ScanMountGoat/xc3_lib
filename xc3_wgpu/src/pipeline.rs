use xc3_lib::mxmd::StencilMode;
use xc3_model::{BlendMode, CullMode, RenderPassType, StateFlags};

use crate::{DEPTH_STENCIL_FORMAT, GBUFFER_COLOR_FORMAT};

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

// TODO: This also needs to take into account mesh flags?
/// The non shared components of a pipeline for use with pipeline caching.
#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq)]
pub struct PipelineKey {
    pub pass_type: RenderPassType,
    pub flags: StateFlags,
    pub is_outline: bool,
}

impl PipelineKey {
    pub fn write_to_all_outputs(&self) -> bool {
        self.pass_type == RenderPassType::Unk0
    }

    pub fn stencil_reference(&self) -> u32 {
        // TODO: move this to xc3_lib?
        // TODO: Test remaining values.
        match self.flags.stencil_value {
            xc3_lib::mxmd::StencilValue::Unk0 => 10,
            xc3_lib::mxmd::StencilValue::Unk1 => 0,
            xc3_lib::mxmd::StencilValue::Unk4 => 14,
            xc3_lib::mxmd::StencilValue::Unk5 => 0,
            xc3_lib::mxmd::StencilValue::Unk8 => 0,
            xc3_lib::mxmd::StencilValue::Unk9 => 0,
            xc3_lib::mxmd::StencilValue::Unk12 => 0,
            xc3_lib::mxmd::StencilValue::Unk16 => 74,
            xc3_lib::mxmd::StencilValue::Unk20 => 0,
            xc3_model::StencilValue::Unk33 => 0,
            xc3_model::StencilValue::Unk37 => 0,
            xc3_model::StencilValue::Unk41 => 0,
            xc3_model::StencilValue::Unk49 => 0,
            xc3_model::StencilValue::Unk97 => 0,
            xc3_model::StencilValue::Unk105 => 0,
        }
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
                blend: blend_state(key.flags.blend_mode),
                write_mask: wgpu::ColorWrites::all(),
            }),
            None,
            None,
            None,
            None,
            None,
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
            format: DEPTH_STENCIL_FORMAT,
            // TODO: this depends on the mesh render pass?
            depth_write_enabled: true,
            depth_compare: match key.flags.depth_func {
                xc3_lib::mxmd::DepthFunc::Disabled => wgpu::CompareFunction::Always,
                xc3_lib::mxmd::DepthFunc::LessEqual => wgpu::CompareFunction::LessEqual,
                xc3_lib::mxmd::DepthFunc::Equal => wgpu::CompareFunction::Equal,
            },
            stencil: stencil_state(key.flags.stencil_mode),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
    })
}

fn stencil_state(mode: StencilMode) -> wgpu::StencilState {
    wgpu::StencilState {
        front: wgpu::StencilFaceState {
            compare: stencil_compare(mode),
            fail_op: wgpu::StencilOperation::Keep,
            depth_fail_op: wgpu::StencilOperation::Keep,
            pass_op: wgpu::StencilOperation::Replace,
        },
        back: wgpu::StencilFaceState {
            compare: stencil_compare(mode),
            fail_op: wgpu::StencilOperation::Keep,
            depth_fail_op: wgpu::StencilOperation::Keep,
            pass_op: wgpu::StencilOperation::Replace,
        },
        // TODO: Should these depend on stencil value?
        read_mask: match mode {
            StencilMode::Unk0 => 0xff,
            StencilMode::Unk1 => 0xff,
            StencilMode::Unk2 => 0xff,
            StencilMode::Unk6 => 0x4,
            StencilMode::Unk7 => 0xff,
            StencilMode::Unk8 => 0xff,
        },
        write_mask: match mode {
            StencilMode::Unk0 => 0xff,
            StencilMode::Unk1 => 0xff,
            StencilMode::Unk2 => 0xff,
            StencilMode::Unk6 => 0x4b,
            StencilMode::Unk7 => 0xff,
            StencilMode::Unk8 => 0xff,
        },
    }
}

fn stencil_compare(mode: StencilMode) -> wgpu::CompareFunction {
    match mode {
        StencilMode::Unk0 => wgpu::CompareFunction::Always,
        StencilMode::Unk1 => wgpu::CompareFunction::Always,
        StencilMode::Unk2 => wgpu::CompareFunction::Always,
        StencilMode::Unk6 => wgpu::CompareFunction::Equal,
        StencilMode::Unk7 => wgpu::CompareFunction::Always,
        StencilMode::Unk8 => wgpu::CompareFunction::Always,
    }
}

fn cull_mode(mode: CullMode) -> Option<wgpu::Face> {
    match mode {
        CullMode::Back => Some(wgpu::Face::Back),
        CullMode::Front => Some(wgpu::Face::Front),
        CullMode::Disabled => None,
        CullMode::Unk3 => Some(wgpu::Face::Front),
    }
}

fn blend_state(state: BlendMode) -> Option<wgpu::BlendState> {
    match state {
        BlendMode::Disabled => None,
        BlendMode::AlphaBlend => Some(wgpu::BlendState {
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
        BlendMode::Additive => Some(wgpu::BlendState {
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
        BlendMode::Multiplicative => Some(wgpu::BlendState {
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
        // TODO: How to handle unk5?
        BlendMode::Unk5 | BlendMode::Unk6 => None,
    }
}
