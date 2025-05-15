use indexmap::IndexSet;
use ordered_float::OrderedFloat;
use smol_str::SmolStr;
use xc3_lib::mxmd::TextureUsage;

use crate::{
    shader_database::{
        Dependency, Operation, OutputExpr, ShaderProgram, TexCoordParams, TextureDependency,
    },
    ImageTexture,
};

use super::{MaterialParameters, Texture};

/// Assignment information for the channels of each output.
/// This includes channels from textures, material parameters, or shader constants.
#[derive(Debug, Clone, PartialEq)]
pub struct OutputAssignments {
    pub output_assignments: [OutputAssignment; 6],

    // TODO: make this the same type as normal intensity.
    /// The parameter multiplied by vertex alpha to determine outline width.
    pub outline_width: Option<AssignmentValue>,

    /// Index into [assignments](#structfield.assignments] for the intensity map for normal mapping.
    pub normal_intensity: Option<usize>,

    pub assignments: Vec<Assignment>,
}

impl OutputAssignments {
    /// Calculate the material ID from a hardcoded shader constant if present.
    pub fn mat_id(&self) -> Option<u32> {
        if let Assignment::Value(Some(AssignmentValue::Float(v))) =
            self.assignments.get(self.output_assignments[1].w?)?
        {
            // TODO: Why is this sometimes 7?
            Some((v.0 * 255.0 + 0.1) as u32 & 0x7)
        } else {
            None
        }
    }
}

// TODO: Come up with better names.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct OutputAssignment {
    /// Index into [assignments](struct.OutputAssignments.html#structfield.assignments) for the x value.
    pub x: Option<usize>,
    /// Index into [assignments](struct.OutputAssignments.html#structfield.assignments) for the y value.
    pub y: Option<usize>,
    /// Index into [assignments](struct.OutputAssignments.html#structfield.assignments) for the z value.
    pub z: Option<usize>,
    /// Index into [assignments](struct.OutputAssignments.html#structfield.assignments) for the w value.
    pub w: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Assignment {
    Value(Option<AssignmentValue>),
    Func {
        op: Operation,
        /// Index into [assignments](struct.OutputAssignments.html#structfield.assignments)
        /// for the function argument list `[arg0, arg1, ...]`.
        args: Vec<usize>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AssignmentValue {
    Texture(TextureAssignment),
    Attribute {
        name: SmolStr,
        channel: Option<char>,
    },
    Float(OrderedFloat<f32>),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TextureAssignment {
    pub name: SmolStr,
    pub channel: Option<char>,
    pub texcoord_name: Option<SmolStr>,
    pub texcoord_transforms: Option<([OrderedFloat<f32>; 4], [OrderedFloat<f32>; 4])>,
    pub parallax: Option<TexCoordParallax>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TexCoordParallax {
    pub mask_a: Box<AssignmentValue>,
    pub mask_b: Box<AssignmentValue>,
    pub ratio: Box<AssignmentValue>,
}

impl Default for Assignment {
    fn default() -> Self {
        Self::Value(None)
    }
}

impl AssignmentValue {
    pub fn from_dependency(d: &Dependency, parameters: &MaterialParameters) -> Option<Self> {
        match d {
            Dependency::Constant(f) => Some(Self::Float(f.0.into())),
            Dependency::Buffer(b) => parameters.get_dependency(b).map(|f| Self::Float(f.into())),
            Dependency::Texture(texture) => {
                Some(Self::Texture(texture_assignment(texture, parameters)))
            }
            Dependency::Attribute(a) => Some(Self::Attribute {
                name: a.name.clone(),
                channel: a.channel,
            }),
        }
    }
}

pub(crate) fn output_assignments(
    shader: &ShaderProgram,
    parameters: &MaterialParameters,
) -> OutputAssignments {
    let mut assignments = IndexSet::new();

    OutputAssignments {
        output_assignments: [0, 1, 2, 3, 4, 5]
            .map(|i| output_assignment(shader, parameters, &mut assignments, i)),
        outline_width: shader
            .outline_width
            .as_ref()
            .and_then(|d| AssignmentValue::from_dependency(d, parameters)),
        normal_intensity: shader
            .normal_intensity
            .as_ref()
            .map(|l| assignment_value(parameters, l, &mut assignments)),
        assignments: assignments.into_iter().collect(),
    }
}

fn output_assignment(
    shader: &ShaderProgram,
    parameters: &MaterialParameters,
    assignments: &mut IndexSet<Assignment>,
    output_index: usize,
) -> OutputAssignment {
    OutputAssignment {
        x: output_channel_assignment(shader, parameters, assignments, output_index, 'x'),
        y: output_channel_assignment(shader, parameters, assignments, output_index, 'y'),
        z: output_channel_assignment(shader, parameters, assignments, output_index, 'z'),
        w: output_channel_assignment(shader, parameters, assignments, output_index, 'w'),
    }
}

fn output_channel_assignment(
    shader: &ShaderProgram,
    parameters: &MaterialParameters,
    assignments: &mut IndexSet<Assignment>,
    output_index: usize,
    channel: char,
) -> Option<usize> {
    let output = format!("o{output_index}.{channel}");
    shader
        .output_dependencies
        .get(&SmolStr::from(output))
        .map(|v| assignment_value(parameters, v, assignments))
}

fn assignment_value(
    parameters: &MaterialParameters,
    value: &OutputExpr,
    assignments: &mut IndexSet<Assignment>,
) -> usize {
    let value = match value {
        crate::shader_database::OutputExpr::Value(d) => {
            Assignment::Value(AssignmentValue::from_dependency(d, parameters))
        }
        crate::shader_database::OutputExpr::Func { op, args } => Assignment::Func {
            op: *op,
            args: args
                .iter()
                .map(|a| assignment_value(parameters, a, assignments))
                .collect(),
        },
    };
    let (index, _) = assignments.insert_full(value);
    index
}

fn texture_assignment(
    texture: &TextureDependency,
    parameters: &MaterialParameters,
) -> TextureAssignment {
    let texcoord_transforms = texcoord_transforms(texture, parameters);

    // TODO: different attribute for U and V?
    TextureAssignment {
        name: texture.name.clone(),
        channel: texture.channel,
        texcoord_name: texture.texcoords.first().map(|t| t.name.clone()),
        texcoord_transforms,
        parallax: match texture.texcoords.first().and_then(|t| t.params.as_ref()) {
            Some(TexCoordParams::Parallax {
                mask_a,
                mask_b,
                ratio,
            }) => {
                let mask_a = AssignmentValue::from_dependency(mask_a, parameters);
                let mask_b = AssignmentValue::from_dependency(mask_b, parameters);
                let ratio = AssignmentValue::from_dependency(ratio, parameters);
                // TODO: Why are these sometimes none for xcx de?
                match (mask_a, mask_b, ratio) {
                    (Some(mask_a), Some(mask_b), Some(ratio)) => Some(TexCoordParallax {
                        mask_a: Box::new(mask_a),
                        mask_b: Box::new(mask_b),
                        ratio: Box::new(ratio),
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
    let mut assignments = IndexSet::new();

    let mut assignment = |i: Option<usize>, c: usize| {
        Some(
            assignments
                .insert_full(Assignment::Value(i.map(|i| {
                    AssignmentValue::Texture(TextureAssignment {
                        name: format!("s{i}").into(),
                        channel: Some(['x', 'y', 'z', 'w'][c]),
                        texcoord_name: None,
                        texcoord_transforms: None,
                        parallax: None,
                    })
                })))
                .0,
        )
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
        output_assignments: [
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
        assignments: assignments.into_iter().collect(),
    }
}
