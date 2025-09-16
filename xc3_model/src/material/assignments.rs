use indexmap::IndexSet;
use ordered_float::OrderedFloat;
use smol_str::SmolStr;
use xc3_lib::mxmd::TextureUsage;

use crate::{
    shader_database::{Dependency, Operation, OutputExpr, ShaderProgram, TextureDependency},
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

    /// Index into [assignments](#structfield.assignments) for the intensity map for normal mapping.
    pub normal_intensity: Option<usize>,

    /// Unique values shared between all outputs.
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
    Int(i32),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TextureAssignment {
    /// The name of the texture like `s0` or `gTResidentTex09`.
    pub name: SmolStr,
    pub channel: Option<char>,
    /// Indices into [assigmments](struct.OutputAssignments.html#structfield.assignments)
    /// for the texture coordinates.
    pub texcoords: Vec<usize>,
}

/// Assignment information for the channels of each output.
/// This includes channels from textures, material parameters, or shader constants.
#[derive(Debug, Clone, PartialEq)]
pub struct OutputAssignmentXyz {
    /// Index into [assignments](#structfield.assignments) for the most recent assignment.
    pub assignment: usize, // TODO: option int?

    /// Unique shared values.
    pub assignments: Vec<AssignmentXyz>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AssignmentXyz {
    Value(Option<AssignmentValueXyz>),
    Func {
        op: Operation,
        /// Index into XYZ [assignments](struct.OutputAssignmentXyz.html#structfield.assignments)
        /// for the function argument list `[arg0, arg1, ...]`.
        args: Vec<usize>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AssignmentValueXyz {
    Texture(TextureAssignmentXyz),
    Attribute {
        name: SmolStr,
        channel: Option<ChannelXyz>,
    },
    Float([OrderedFloat<f32>; 3]),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TextureAssignmentXyz {
    /// The name of the texture like `s0` or `gTResidentTex09`.
    pub name: SmolStr,
    pub channel: Option<ChannelXyz>,
    /// Indices into scalar [assigmments](struct.OutputAssignments.html#structfield.assignments)
    /// for the texture coordinates.
    pub texcoords: Vec<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChannelXyz {
    Xyz,
    X,
    Y,
    Z,
    W,
}

impl Default for Assignment {
    fn default() -> Self {
        Self::Value(None)
    }
}

impl AssignmentValue {
    pub fn from_dependency(
        d: &Dependency,
        shader: &ShaderProgram,
        parameters: &MaterialParameters,
        assignments: &mut IndexSet<Assignment>,
    ) -> Option<Self> {
        match d {
            Dependency::Int(i) => Some(Self::Int(*i)),
            Dependency::Float(f) => Some(Self::Float(f.0.into())),
            Dependency::Buffer(b) => parameters.get_dependency(b).map(|f| Self::Float(f.into())),
            Dependency::Texture(texture) => Some(Self::Texture(texture_assignment(
                texture,
                shader,
                parameters,
                assignments,
            ))),
            Dependency::Attribute(a) => Some(Self::Attribute {
                name: a.name.clone(),
                channel: a.channel,
            }),
        }
    }
}

impl OutputAssignment {
    pub fn merge_xyz(&self, assignments: &[Assignment]) -> Option<OutputAssignmentXyz> {
        let mut assignments_xyz = IndexSet::new();
        let i =
            merge_xyz_assignments(self.x?, self.y?, self.z?, assignments, &mut assignments_xyz)?;

        Some(OutputAssignmentXyz {
            assignment: i,
            assignments: assignments_xyz.into_iter().collect(),
        })
    }
}

fn merge_xyz_assignments(
    x: usize,
    y: usize,
    z: usize,
    assignments: &[Assignment],
    assignments_xyz: &mut IndexSet<AssignmentXyz>,
) -> Option<usize> {
    let x = assignments.get(x)?;
    let y = assignments.get(y)?;
    let z = assignments.get(z)?;

    let assignment_xyz = match (x, y, z) {
        (
            Assignment::Func {
                op: op_x,
                args: args_x,
            },
            Assignment::Func {
                op: op_y,
                args: args_y,
            },
            Assignment::Func {
                op: op_z,
                args: args_z,
            },
        ) => {
            let op = op_xyz(*op_x, *op_y, *op_z)?;
            if args_x.len() == args_y.len() && args_y.len() == args_z.len() {
                let mut args = Vec::new();
                for ((x, y), z) in args_x.iter().zip(args_y.iter()).zip(args_z.iter()) {
                    let arg = merge_xyz_assignments(*x, *y, *z, assignments, assignments_xyz)?;
                    args.push(arg);
                }
                Some(AssignmentXyz::Func { op, args })
            } else {
                None
            }
        }
        (Assignment::Value(vx), Assignment::Value(vy), Assignment::Value(vz)) => {
            // TODO: Check that channels are one of the supported channels.
            match (vx, vy, vz) {
                (
                    Some(AssignmentValue::Texture(tx)),
                    Some(AssignmentValue::Texture(ty)),
                    Some(AssignmentValue::Texture(tz)),
                ) => {
                    if tx.texcoords == ty.texcoords && ty.texcoords == tz.texcoords {
                        let t_xyz = TextureAssignmentXyz {
                            name: name_xyz(&tx.name, &ty.name, &tz.name)?,
                            channel: channel_xyz(tx.channel, ty.channel, tz.channel)?,
                            texcoords: tx.texcoords.clone(), // TODO: These should refer to the scalar assignments?
                        };
                        Some(AssignmentXyz::Value(Some(AssignmentValueXyz::Texture(
                            t_xyz,
                        ))))
                    } else {
                        None
                    }
                }
                (
                    Some(AssignmentValue::Attribute {
                        name: n_x,
                        channel: c_x,
                    }),
                    Some(AssignmentValue::Attribute {
                        name: n_y,
                        channel: c_y,
                    }),
                    Some(AssignmentValue::Attribute {
                        name: n_z,
                        channel: c_z,
                    }),
                ) => Some(AssignmentXyz::Value(Some(AssignmentValueXyz::Attribute {
                    name: name_xyz(n_x, n_y, n_z)?,
                    channel: channel_xyz(*c_x, *c_y, *c_z)?,
                }))),
                (
                    Some(AssignmentValue::Float(fx)),
                    Some(AssignmentValue::Float(fy)),
                    Some(AssignmentValue::Float(fz)),
                ) => Some(AssignmentXyz::Value(Some(AssignmentValueXyz::Float([
                    *fx, *fy, *fz,
                ])))),
                (None, None, None) => Some(AssignmentXyz::Value(None)),
                _ => None,
            }
        }
        _ => None,
    }?;

    let index = assignments_xyz.insert_full(assignment_xyz).0;
    Some(index)
}

fn op_xyz(x: Operation, y: Operation, z: Operation) -> Option<Operation> {
    // Skip any operations that involve vector arguments.
    // TODO: Should all operations be supported?
    // TODO: Combine xyz and scalar operations?
    if x == y
        && y == z
        && !matches!(
            x,
            Operation::AddNormalX
                | Operation::AddNormalY
                | Operation::ReflectX
                | Operation::ReflectY
                | Operation::ReflectZ
                | Operation::Dot4
                | Operation::NormalMapX
                | Operation::NormalMapY
                | Operation::NormalMapZ,
        )
    {
        Some(x)
    } else {
        None
    }
}

fn name_xyz(x: &SmolStr, y: &SmolStr, z: &SmolStr) -> Option<SmolStr> {
    if x == y && y == z {
        Some(x.clone())
    } else {
        None
    }
}

fn channel_xyz(x: Option<char>, y: Option<char>, z: Option<char>) -> Option<Option<ChannelXyz>> {
    match (x, y, z) {
        (Some('x'), Some('y'), Some('z')) => Some(Some(ChannelXyz::Xyz)),
        (Some('x'), Some('x'), Some('x')) => Some(Some(ChannelXyz::X)),
        (Some('y'), Some('y'), Some('y')) => Some(Some(ChannelXyz::Y)),
        (Some('z'), Some('z'), Some('z')) => Some(Some(ChannelXyz::Z)),
        (Some('w'), Some('w'), Some('w')) => Some(Some(ChannelXyz::W)),
        (None, None, None) => Some(None),
        _ => None,
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
        outline_width: shader.outline_width.as_ref().and_then(|d| {
            AssignmentValue::from_dependency(d, shader, parameters, &mut assignments)
        }),
        normal_intensity: shader
            .normal_intensity
            .as_ref()
            .map(|i| assignment_value(shader, parameters, &shader.exprs[*i], &mut assignments)),
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
        .map(|v| assignment_value(shader, parameters, &shader.exprs[*v], assignments))
}

fn assignment_value(
    shader: &ShaderProgram,
    parameters: &MaterialParameters,
    value: &OutputExpr,
    assignments: &mut IndexSet<Assignment>,
) -> usize {
    let value = match value {
        crate::shader_database::OutputExpr::Value(d) => Assignment::Value(
            AssignmentValue::from_dependency(d, shader, parameters, assignments),
        ),
        crate::shader_database::OutputExpr::Func { op, args } => Assignment::Func {
            op: *op,
            args: args
                .iter()
                .map(|a| assignment_value(shader, parameters, &shader.exprs[*a], assignments))
                .collect(),
        },
    };
    let (index, _) = assignments.insert_full(value);
    index
}

fn texture_assignment(
    texture: &TextureDependency,
    shader: &ShaderProgram,
    parameters: &MaterialParameters,
    assignments: &mut IndexSet<Assignment>,
) -> TextureAssignment {
    TextureAssignment {
        name: texture.name.clone(),
        channel: texture.channel,
        texcoords: texture
            .texcoords
            .iter()
            .map(|c| assignment_value(shader, parameters, &shader.exprs[*c], assignments))
            .collect(),
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
        let u = assignments
            .insert_full(Assignment::Value(Some(AssignmentValue::Attribute {
                name: "vTex0".into(),
                channel: Some('x'),
            })))
            .0;
        let v = assignments
            .insert_full(Assignment::Value(Some(AssignmentValue::Attribute {
                name: "vTex0".into(),
                channel: Some('y'),
            })))
            .0;
        Some(
            assignments
                .insert_full(Assignment::Value(i.map(|i| {
                    AssignmentValue::Texture(TextureAssignment {
                        name: format!("s{i}").into(),
                        channel: Some(['x', 'y', 'z', 'w'][c]),
                        texcoords: vec![u, v],
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_xyz_empty() {
        assert_eq!(
            None,
            OutputAssignment {
                x: Some(0),
                y: Some(0),
                z: Some(0),
                w: None,
            }
            .merge_xyz(&[])
        );
    }

    #[test]
    fn merge_xyz_invalid_channels() {
        let assignments = [
            Assignment::Value(Some(AssignmentValue::Float(0.0.into()))),
            Assignment::Value(Some(AssignmentValue::Texture(TextureAssignment {
                name: "s0".into(),
                channel: Some('z'),
                texcoords: vec![0, 0],
            }))),
            Assignment::Value(Some(AssignmentValue::Texture(TextureAssignment {
                name: "s0".into(),
                channel: Some('y'),
                texcoords: vec![0, 0],
            }))),
            Assignment::Value(Some(AssignmentValue::Texture(TextureAssignment {
                name: "s0".into(),
                channel: Some('x'),
                texcoords: vec![0, 0],
            }))),
        ];
        assert_eq!(
            None,
            OutputAssignment {
                x: Some(1),
                y: Some(2),
                z: Some(3),
                w: None,
            }
            .merge_xyz(&assignments)
        );
    }

    #[test]
    fn merge_xyz_single_channel() {
        let assignments = [
            Assignment::Value(Some(AssignmentValue::Float(0.0.into()))),
            Assignment::Value(Some(AssignmentValue::Texture(TextureAssignment {
                name: "s0".into(),
                channel: Some('w'),
                texcoords: vec![0, 0],
            }))),
        ];
        assert_eq!(
            Some(OutputAssignmentXyz {
                assignment: 0,
                assignments: vec![AssignmentXyz::Value(Some(AssignmentValueXyz::Texture(
                    TextureAssignmentXyz {
                        name: "s0".into(),
                        channel: Some(ChannelXyz::W),
                        texcoords: vec![0, 0]
                    }
                )))]
            }),
            OutputAssignment {
                x: Some(1),
                y: Some(1),
                z: Some(1),
                w: None,
            }
            .merge_xyz(&assignments)
        );
    }

    #[test]
    fn merge_xyz_multiple_channels() {
        let assignments = [
            Assignment::Value(Some(AssignmentValue::Float(0.0.into()))),
            Assignment::Value(Some(AssignmentValue::Texture(TextureAssignment {
                name: "s0".into(),
                channel: Some('x'),
                texcoords: vec![0, 0],
            }))),
            Assignment::Value(Some(AssignmentValue::Float(1.0.into()))),
            Assignment::Value(Some(AssignmentValue::Attribute {
                name: "vColor".into(),
                channel: Some('x'),
            })),
            Assignment::Value(Some(AssignmentValue::Texture(TextureAssignment {
                name: "s0".into(),
                channel: Some('y'),
                texcoords: vec![0, 0],
            }))),
            Assignment::Value(Some(AssignmentValue::Float(2.0.into()))),
            Assignment::Value(Some(AssignmentValue::Attribute {
                name: "vColor".into(),
                channel: Some('y'),
            })),
            Assignment::Value(Some(AssignmentValue::Texture(TextureAssignment {
                name: "s0".into(),
                channel: Some('z'),
                texcoords: vec![0, 0],
            }))),
            Assignment::Value(Some(AssignmentValue::Float(3.0.into()))),
            Assignment::Value(Some(AssignmentValue::Attribute {
                name: "vColor".into(),
                channel: Some('z'),
            })),
            Assignment::Func {
                op: Operation::Fma,
                args: vec![1, 2, 3],
            },
            Assignment::Func {
                op: Operation::Fma,
                args: vec![4, 5, 6],
            },
            Assignment::Func {
                op: Operation::Fma,
                args: vec![7, 8, 9],
            },
        ];
        assert_eq!(
            Some(OutputAssignmentXyz {
                assignment: 3,
                assignments: vec![
                    AssignmentXyz::Value(Some(AssignmentValueXyz::Texture(TextureAssignmentXyz {
                        name: "s0".into(),
                        channel: Some(ChannelXyz::Xyz),
                        texcoords: vec![0, 0]
                    }))),
                    AssignmentXyz::Value(Some(AssignmentValueXyz::Float([
                        1.0.into(),
                        2.0.into(),
                        3.0.into()
                    ]))),
                    AssignmentXyz::Value(Some(AssignmentValueXyz::Attribute {
                        name: "vColor".into(),
                        channel: Some(ChannelXyz::Xyz)
                    })),
                    AssignmentXyz::Func {
                        op: Operation::Fma,
                        args: vec![0, 1, 2]
                    }
                ]
            }),
            OutputAssignment {
                x: Some(10),
                y: Some(11),
                z: Some(12),
                w: None,
            }
            .merge_xyz(&assignments)
        );
    }
}
