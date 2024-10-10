use xc3_model::material::{BlendMode, ColorWriteMode, CullMode, RenderPassType, StateFlags};

use crate::{DEPTH_STENCIL_FORMAT, GBUFFER_COLOR_FORMAT, GBUFFER_NORMAL_FORMAT};

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
    pub output5_type: Output5Type,
}

#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq)]
pub enum Output5Type {
    Specular,
    Emission,
}

impl PipelineKey {
    pub fn write_to_all_outputs(&self) -> bool {
        matches!(
            self.flags.color_write_mode,
            ColorWriteMode::Unk0 | ColorWriteMode::Unk10
        )
    }

    pub fn stencil_reference(&self) -> u32 {
        // TODO: move this to xc3_lib?
        // TODO: Test remaining values.
        match self.flags.stencil_value {
            xc3_model::material::StencilValue::Unk0 => 10,
            xc3_model::material::StencilValue::Unk1 => 0,
            xc3_model::material::StencilValue::Unk4 => 14,
            xc3_model::material::StencilValue::Unk5 => 0,
            xc3_model::material::StencilValue::Unk8 => 0,
            xc3_model::material::StencilValue::Unk9 => 0,
            xc3_model::material::StencilValue::Unk12 => 0,
            xc3_model::material::StencilValue::Unk16 => 74,
            xc3_model::material::StencilValue::Unk20 => 0,
            xc3_model::material::StencilValue::Unk33 => 0,
            xc3_model::material::StencilValue::Unk37 => 0,
            xc3_model::material::StencilValue::Unk41 => 0,
            xc3_model::material::StencilValue::Unk49 => 0,
            xc3_model::material::StencilValue::Unk97 => 0,
            xc3_model::material::StencilValue::Unk105 => 0,
        }
    }
}

// TODO: Always set depth and stencil state?
pub fn model_pipeline(
    device: &wgpu::Device,
    data: &ModelPipelineData,
    key: &PipelineKey,
) -> wgpu::RenderPipeline {
    let vertex_entry = if key.is_outline {
        crate::shader::model::vs_outline_main_entry(
            wgpu::VertexStepMode::Vertex,
            wgpu::VertexStepMode::Vertex,
        )
    } else {
        crate::shader::model::vs_main_entry(
            wgpu::VertexStepMode::Vertex,
            wgpu::VertexStepMode::Vertex,
        )
    };

    // Some shaders only write to the albedo output.
    // TODO: Is there a better way of handling this than modifying the render pass?
    if key.is_outline {
        let entry = crate::shader::model::fs_outline_entry([
            Some(GBUFFER_COLOR_FORMAT.into()),
            Some(GBUFFER_COLOR_FORMAT.into()),
            Some(GBUFFER_NORMAL_FORMAT.into()),
            Some(GBUFFER_COLOR_FORMAT.into()),
            Some(GBUFFER_COLOR_FORMAT.into()),
            Some(GBUFFER_COLOR_FORMAT.into()),
        ]);
        model_pipeline_inner(device, data, vertex_entry, entry, key)
    } else if key.write_to_all_outputs() {
        // TODO: Do outputs other than color ever use blending?
        // Create a target for each of the G-Buffer textures.
        let entry = crate::shader::model::fs_main_entry([
            Some(wgpu::ColorTargetState {
                format: GBUFFER_COLOR_FORMAT,
                blend: blend_state(key.flags.blend_mode),
                write_mask: wgpu::ColorWrites::all(),
            }),
            Some(GBUFFER_COLOR_FORMAT.into()),
            Some(GBUFFER_NORMAL_FORMAT.into()),
            Some(GBUFFER_COLOR_FORMAT.into()),
            Some(GBUFFER_COLOR_FORMAT.into()),
            Some(GBUFFER_COLOR_FORMAT.into()),
        ]);
        model_pipeline_inner(device, data, vertex_entry, entry, key)
    } else {
        let entry = crate::shader::model::fs_alpha_entry([Some(wgpu::ColorTargetState {
            format: GBUFFER_COLOR_FORMAT,
            blend: blend_state(key.flags.blend_mode),
            write_mask: wgpu::ColorWrites::all(),
        })]);
        model_pipeline_inner(device, data, vertex_entry, entry, key)
    }
}

fn model_pipeline_inner<const N: usize>(
    device: &wgpu::Device,
    data: &ModelPipelineData,
    vertex_entry: crate::shader::model::VertexEntry<2>,
    fragment_entry: crate::shader::model::FragmentEntry<N>,
    key: &PipelineKey,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Model Pipeline"),
        layout: Some(&data.layout),
        vertex: crate::shader::model::vertex_state(&data.module, &vertex_entry),
        fragment: Some(crate::shader::model::fragment_state(
            &data.module,
            &fragment_entry,
        )),
        primitive: wgpu::PrimitiveState {
            // TODO: Do all meshes using indexed triangle lists?
            topology: wgpu::PrimitiveTopology::TriangleList,
            polygon_mode: wgpu::PolygonMode::Fill,
            cull_mode: cull_mode(key.flags.cull_mode),
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_STENCIL_FORMAT,
            // TODO: affected by alpha test depth prepass?
            depth_write_enabled: key.flags.depth_write_mode != 1,
            depth_compare: match key.flags.depth_func {
                xc3_model::material::DepthFunc::Disabled => wgpu::CompareFunction::Always,
                xc3_model::material::DepthFunc::LessEqual => wgpu::CompareFunction::LessEqual,
                xc3_model::material::DepthFunc::Equal => wgpu::CompareFunction::Equal,
            },
            stencil: stencil_state(key.flags.stencil_mode),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
        cache: None,
    })
}

fn stencil_state(mode: xc3_model::material::StencilMode) -> wgpu::StencilState {
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
            xc3_model::material::StencilMode::Unk0 => 0xff,
            xc3_model::material::StencilMode::Unk1 => 0xff,
            xc3_model::material::StencilMode::Unk2 => 0xff,
            xc3_model::material::StencilMode::Unk6 => 0x4,
            xc3_model::material::StencilMode::Unk7 => 0xff,
            xc3_model::material::StencilMode::Unk8 => 0xff,
        },
        write_mask: match mode {
            xc3_model::material::StencilMode::Unk0 => 0xff,
            xc3_model::material::StencilMode::Unk1 => 0xff,
            xc3_model::material::StencilMode::Unk2 => 0xff,
            xc3_model::material::StencilMode::Unk6 => 0x4b,
            xc3_model::material::StencilMode::Unk7 => 0xff,
            xc3_model::material::StencilMode::Unk8 => 0xff,
        },
    }
}

fn stencil_compare(mode: xc3_model::material::StencilMode) -> wgpu::CompareFunction {
    match mode {
        xc3_model::material::StencilMode::Unk0 => wgpu::CompareFunction::Always,
        xc3_model::material::StencilMode::Unk1 => wgpu::CompareFunction::Always,
        xc3_model::material::StencilMode::Unk2 => wgpu::CompareFunction::Always,
        xc3_model::material::StencilMode::Unk6 => wgpu::CompareFunction::Equal,
        xc3_model::material::StencilMode::Unk7 => wgpu::CompareFunction::Always,
        xc3_model::material::StencilMode::Unk8 => wgpu::CompareFunction::Always,
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
        BlendMode::Blend => Some(wgpu::BlendState {
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
        BlendMode::Unk2 => Some(wgpu::BlendState {
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
        BlendMode::Multiply => Some(wgpu::BlendState {
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
        BlendMode::MultiplyInverted => Some(wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::OneMinusDst,
                dst_factor: wgpu::BlendFactor::Zero,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::OneMinusDst,
                dst_factor: wgpu::BlendFactor::Zero,
                operation: wgpu::BlendOperation::Add,
            },
        }),
        BlendMode::Add => Some(wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
        }),
        // Values not in range [1,5] disable blending in setupMrtAlphaBlend in xc3 binary.
        _ => None,
    }
}
