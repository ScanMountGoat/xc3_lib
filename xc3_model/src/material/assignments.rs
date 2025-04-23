use ordered_float::OrderedFloat;
use smol_str::{SmolStr, ToSmolStr};
use xc3_lib::mxmd::TextureUsage;

use crate::{
    shader_database::{
        Dependency, Layer, LayerBlendMode, LayerValue, ShaderProgram, TexCoordParams,
        TextureDependency,
    },
    ImageTexture,
};

use super::{MaterialParameters, Texture};

/// Assignment information for the channels of each output.
/// This includes channels from textures, material parameters, or shader constants.
#[derive(Debug, Clone, PartialEq)]
pub struct OutputAssignments {
    pub assignments: [OutputAssignment; 6],

    /// The parameter multiplied by vertex alpha to determine outline width.
    pub outline_width: Option<ValueAssignment>,

    /// The intensity map for normal mapping.
    pub normal_intensity: Option<LayerAssignmentValue>,
}

impl OutputAssignments {
    /// Calculate the material ID from a hardcoded shader constant if present.
    pub fn mat_id(&self) -> Option<u32> {
        if let LayerAssignmentValue::Value(Some(ValueAssignment::Value(v))) = self.assignments[1].w
        {
            // TODO: Why is this sometimes 7?
            Some((v.0 * 255.0 + 0.1) as u32 & 0x7)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct OutputAssignment {
    /// The x values.
    pub x: LayerAssignmentValue,
    /// The y values.
    pub y: LayerAssignmentValue,
    /// The z values.
    pub z: LayerAssignmentValue,
    /// The w values.
    pub w: LayerAssignmentValue,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LayerAssignment {
    /// The layer value to blend with the previous layer.
    pub value: LayerAssignmentValue,
    /// The factor or blend weight for this layer.
    pub weight: LayerAssignmentValue,
    /// The blending operation for this layer.
    pub blend_mode: LayerBlendMode,
    pub is_fresnel: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum LayerAssignmentValue {
    Value(Option<ValueAssignment>),
    Layers(Vec<LayerAssignment>),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ValueAssignment {
    Texture(TextureAssignment),
    Attribute { name: SmolStr, channel_index: usize },
    Value(OrderedFloat<f32>),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TextureAssignment {
    // TODO: Include matrix transform or scale?
    // TODO: Always convert everything to a matrix?
    // TODO: how often is the matrix even used?
    pub name: SmolStr,
    pub channels: SmolStr,
    pub texcoord_name: Option<SmolStr>,
    pub texcoord_transforms: Option<([OrderedFloat<f32>; 4], [OrderedFloat<f32>; 4])>,
    pub parallax: Option<TexCoordParallax>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TexCoordParallax {
    pub mask_a: Box<ValueAssignment>,
    pub mask_b: Box<ValueAssignment>,
    pub ratio: OrderedFloat<f32>,
}

impl Default for LayerAssignmentValue {
    fn default() -> Self {
        Self::Value(None)
    }
}

impl ValueAssignment {
    pub fn from_dependency(
        d: &Dependency,
        parameters: &MaterialParameters,
        channel: char,
    ) -> Option<Self> {
        match d {
            Dependency::Constant(f) => Some(Self::Value(f.0.into())),
            Dependency::Buffer(b) => parameters.get_dependency(b).map(|f| Self::Value(f.into())),
            Dependency::Texture(texture) => {
                Some(Self::Texture(texture_assignment(texture, parameters)))
            }
            Dependency::Attribute(a) => {
                // Attributes may have multiple accessed channels.
                // First check if the current channel is used.
                // TODO: Does this always work as intended?
                let c = if a.channel == Some(channel) {
                    channel
                } else {
                    // TODO: avoid unwrap.
                    a.channel.unwrap()
                };

                Some(Self::Attribute {
                    name: a.name.clone(),
                    channel_index: "xyzw".find(c).unwrap(),
                })
            }
        }
    }
}

pub(crate) fn output_assignments(
    shader: &ShaderProgram,
    parameters: &MaterialParameters,
) -> OutputAssignments {
    OutputAssignments {
        assignments: [0, 1, 2, 3, 4, 5].map(|i| output_assignment(shader, parameters, i)),
        outline_width: shader
            .outline_width
            .as_ref()
            .and_then(|d| ValueAssignment::from_dependency(d, parameters, 'x')),
        normal_intensity: shader
            .normal_intensity
            .as_ref()
            .map(|l| layer_channel_assignment_value(parameters, 'x', l)),
    }
}

fn output_assignment(
    shader: &ShaderProgram,
    parameters: &MaterialParameters,
    output_index: usize,
) -> OutputAssignment {
    OutputAssignment {
        x: output_channel_assignment(shader, parameters, output_index, 0),
        y: output_channel_assignment(shader, parameters, output_index, 1),
        z: output_channel_assignment(shader, parameters, output_index, 2),
        w: output_channel_assignment(shader, parameters, output_index, 3),
    }
}

fn output_channel_assignment(
    shader: &ShaderProgram,
    parameters: &MaterialParameters,
    output_index: usize,
    channel_index: usize,
) -> LayerAssignmentValue {
    let layers = layer_assignments(shader, parameters, output_index, channel_index);
    if layers.is_empty() {
        LayerAssignmentValue::Value(None)
    } else {
        LayerAssignmentValue::Layers(layers)
    }
}

fn layer_assignments(
    shader: &ShaderProgram,
    parameters: &MaterialParameters,
    output_index: usize,
    channel_index: usize,
) -> Vec<LayerAssignment> {
    let channel = ['x', 'y', 'z', 'w'][channel_index];
    let output = format!("o{output_index}.{channel}");
    let layers = shader
        .output_dependencies
        .get(&SmolStr::from(output))
        .map(|d| d.layers.as_slice())
        .unwrap_or_default();

    layers
        .iter()
        .map(|l| layer_channel_assignment(parameters, channel, l))
        .collect()
}

fn layer_channel_assignment(
    parameters: &MaterialParameters,
    channel: char,
    l: &Layer,
) -> LayerAssignment {
    let value = layer_channel_assignment_value(parameters, channel, &l.value);
    let weight = layer_channel_assignment_value(parameters, channel, &l.ratio);

    LayerAssignment {
        value,
        weight,
        blend_mode: l.blend_mode,
        is_fresnel: l.is_fresnel,
    }
}

fn layer_channel_assignment_value(
    parameters: &MaterialParameters,
    channel: char,
    value: &LayerValue,
) -> LayerAssignmentValue {
    let value = match value {
        crate::shader_database::LayerValue::Value(d) => {
            LayerAssignmentValue::Value(ValueAssignment::from_dependency(d, parameters, channel))
        }
        crate::shader_database::LayerValue::Layers(layers) => LayerAssignmentValue::Layers(
            layers
                .iter()
                .map(|l| layer_channel_assignment(parameters, channel, l))
                .collect(),
        ),
    };
    value
}

fn texture_assignment(
    texture: &TextureDependency,
    parameters: &MaterialParameters,
) -> TextureAssignment {
    let texcoord_transforms = texcoord_transforms(texture, parameters);

    // TODO: different attribute for U and V?
    TextureAssignment {
        name: texture.name.clone(),
        channels: texture.channel.map(|c| c.to_smolstr()).unwrap_or_default(),
        texcoord_name: texture.texcoords.first().map(|t| t.name.clone()),
        texcoord_transforms,
        parallax: match texture.texcoords.first().and_then(|t| t.params.as_ref()) {
            Some(TexCoordParams::Parallax {
                mask_a,
                mask_b,
                ratio,
            }) => {
                let mask_a = ValueAssignment::from_dependency(mask_a, parameters, 'x');
                let mask_b = ValueAssignment::from_dependency(mask_b, parameters, 'x');
                // TODO: Why are these sometimes none for xcx de?
                match (mask_a, mask_b) {
                    (Some(mask_a), Some(mask_b)) => Some(TexCoordParallax {
                        mask_a: Box::new(mask_a),
                        mask_b: Box::new(mask_b),
                        ratio: parameters.get_dependency(ratio).unwrap_or_default().into(),
                    }),
                    _ => None,
                }
            }
            _ => None,
        },
    }
}

fn texcoord_transforms(
    texture: &TextureDependency,
    parameters: &MaterialParameters,
) -> Option<([OrderedFloat<f32>; 4], [OrderedFloat<f32>; 4])> {
    // Each texcoord component has its own params.
    // TODO: return a vector for everything.
    if let Some([u, v]) = texture.texcoords.get(..2) {
        let transform_u = texcoord_transform(u, parameters, 0)?;
        let transform_v = texcoord_transform(v, parameters, 1)?;
        Some((transform_u, transform_v))
    } else {
        None
    }
}

fn texcoord_transform(
    u: &crate::shader_database::TexCoord,
    parameters: &MaterialParameters,
    index: usize,
) -> Option<[OrderedFloat<f32>; 4]> {
    match u.params.as_ref()? {
        crate::shader_database::TexCoordParams::Scale(s) => {
            // Select and scale the appropriate component.
            let scale = parameters.get_dependency(s)?;
            let mut transform = [0.0.into(); 4];
            transform[index] = scale.into();
            Some(transform)
        }
        crate::shader_database::TexCoordParams::Matrix([x, y, z, w]) => Some([
            parameters.get_dependency(x)?.into(),
            parameters.get_dependency(y)?.into(),
            parameters.get_dependency(z)?.into(),
            parameters.get_dependency(w)?.into(),
        ]),
        crate::shader_database::TexCoordParams::Parallax { .. } => None,
    }
}

pub(crate) fn infer_assignment_from_textures(
    textures: &[Texture],
    image_textures: &[ImageTexture],
) -> OutputAssignments {
    // No assignment data is available.
    // Guess reasonable defaults based on the texture names or types.
    let assignment = |i: Option<usize>, c: usize| {
        LayerAssignmentValue::Value(i.map(|i| {
            ValueAssignment::Texture(TextureAssignment {
                name: format!("s{i}").into(),
                channels: ["x", "y", "z", "w"][c].into(),
                texcoord_name: None,
                texcoord_transforms: None,
                parallax: None,
            })
        }))
    };

    let color_index = textures.iter().position(|t| {
        matches!(
            // TODO: Why does this index out of range for xc2 legacy mxmd?
            image_textures
                .get(t.image_texture_index)
                .and_then(|t| t.usage),
            Some(TextureUsage::Col | TextureUsage::Col2 | TextureUsage::Col3 | TextureUsage::Col4)
        )
    });

    // This may only have two channels since BC5 is common.
    let normal_index = textures.iter().position(|t| {
        matches!(
            image_textures
                .get(t.image_texture_index)
                .and_then(|t| t.usage),
            Some(TextureUsage::Nrm | TextureUsage::Nrm2)
        )
    });

    let spm_index = textures.iter().position(|t| {
        matches!(
            image_textures.get(t.image_texture_index).and_then(|t| t.name.as_ref()),
            Some(name) if name.ends_with("_SPM")
        )
    });

    OutputAssignments {
        assignments: [
            OutputAssignment {
                x: assignment(color_index, 0),
                y: assignment(color_index, 1),
                z: assignment(color_index, 2),
                w: assignment(color_index, 3),
            },
            OutputAssignment::default(),
            OutputAssignment {
                x: assignment(normal_index, 0),
                y: assignment(normal_index, 1),
                ..Default::default()
            },
            OutputAssignment::default(),
            OutputAssignment::default(),
            OutputAssignment {
                x: assignment(spm_index, 0),
                y: assignment(spm_index, 1),
                z: assignment(spm_index, 2),
                ..Default::default()
            },
        ],
        outline_width: None,
        normal_intensity: None,
    }
}
