use indexmap::{IndexMap, IndexSet};
use ordered_float::OrderedFloat;
use smol_str::SmolStr;
use strum::{Display, FromRepr};
use xc3_lib::mxmd::TextureUsage;

use crate::{
    ImageTexture,
    shader_database::{Attribute, Operation, OutputExpr, Parameter, ShaderProgram, Texture, Value},
};

use super::MaterialParameters;

/// Assignment information for the channels of each output.
/// This includes channels from textures, material parameters, or shader constants.
#[derive(Debug, Clone, PartialEq)]
pub struct OutputAssignments {
    pub output_assignments: [OutputAssignment; 6],

    // TODO: make this the same type as normal intensity.
    /// The parameter multiplied by vertex alpha to determine outline width.
    pub outline_width: Option<Value>,

    /// Index into [assignments](#structfield.assignments) for the intensity map for normal mapping.
    pub normal_intensity: Option<usize>,

    /// Index into [assignments](#structfield.assignments) for the intensity for vValInf normal mapping.
    pub val_inf_intensity: Option<usize>,

    /// Unique values shared between all outputs.
    pub exprs: Vec<OutputExpr>,
}

impl OutputAssignments {
    /// Calculate the material ID from a hardcoded shader constant if present.
    pub fn mat_id(&self) -> Option<u32> {
        if let OutputExpr::Value(Value::Float(v)) = self.exprs.get(self.output_assignments[1].w?)? {
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
    /// Index into [exprs](struct.OutputAssignments.html#structfield.exprs) for the x value.
    pub x: Option<usize>,
    /// Index into [exprs](struct.OutputAssignments.html#structfield.exprs) for the y value.
    pub y: Option<usize>,
    /// Index into [exprs](struct.OutputAssignments.html#structfield.exprs) for the z value.
    pub z: Option<usize>,
    /// Index into [exprs](struct.OutputAssignments.html#structfield.exprs) for the w value.
    pub w: Option<usize>,
}

/// Assignment information for the channels of each output.
/// This includes channels from textures, material parameters, or shader constants.
#[derive(Debug, Clone, PartialEq)]
pub struct OutputAssignmentXyz {
    /// Index into [exprs](#structfield.exprs) for the most recent assignment.
    pub expr: usize, // TODO: option int?

    /// Unique shared values.
    pub exprs: Vec<AssignmentXyz>,
}

// TODO: OutputExprXyz
// TODO: index into scalar or vector exprs for arguments
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AssignmentXyz {
    Value(Option<AssignmentValueXyz>),
    Func {
        op: OperationXyz,
        /// Index into XYZ [exprs](struct.OutputAssignmentXyz.html#structfield.exprs)
        /// for the function argument list `[arg0, arg1, ...]`.
        args: Vec<usize>,
    },
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, Display, FromRepr, Default)]
pub enum OperationXyz {
    /// An unsupported operation or function call.
    #[default]
    Unk,
    /// `mix(arg0.xyz, arg1.xyz, arg2.xyz)`
    Mix,
    /// `arg0.xyz * arg1.xyz`
    Mul,
    /// `arg0.xyz / arg1.xyz`
    Div,
    /// `arg0.xyz + arg1.xyz`
    Add,
    /// `arg0.xyz - arg1.xyz`
    Sub,
    /// `fma(arg0.xyz, arg1.xyz, arg2.xyz)` or `arg0.xyz * arg1.xyz + arg2.xyz`
    Fma,
    /// `mix(arg0.xyz, arg0.xyz * arg1.xyz, arg2.xyz)`
    MulRatio,
    /// `overlay(arg0.xyz, arg1.xyz)`.
    Overlay,
    /// `overlay2(arg0.xyz, arg1.xyz)`.
    Overlay2,
    /// `mix(arg0.xyz, overlay(arg0.xyz, arg1.xyz), arg2.xyz)`.
    OverlayRatio,
    /// `pow(arg0.xyz, arg1.xyz)`
    Power,
    /// `min(arg0.xyz, arg1.xyz)`
    Min,
    /// `max(arg0.xyz, arg1.xyz)`
    Max,
    /// `clamp(arg0.xyz, arg1.xyz, arg2.xyz)`
    Clamp,
    /// `abs(arg0.xyz)`
    Abs,
    /// `pow(vec3(1.0 - n_dot_v), arg0.xyz * 5.0)`
    Fresnel,
    /// `sqrt(arg0.xyz)`
    Sqrt,
    /// `reflect(arg0.xyz, arg1.xyz)`
    Reflect,
    /// `floor(arg0.xyz)`
    Floor,
    /// `if arg0.xyz { arg1.xyz } else { arg2.xyz }` or `mix(arg2.xyz, arg1.xyz, arg0.xyz)`
    Select,
    /// `arg0.xyz == arg1.xyz`
    Equal,
    /// `arg0.xyz != arg1.xyz`
    NotEqual,
    /// `arg0.xyz < arg1.xyz`
    Less,
    /// `arg0.xyz > arg1.xyz`
    Greater,
    /// `arg0.xyz <= arg1.xyz`
    LessEqual,
    /// `arg0.xyz >= arg1.xyz`
    GreaterEqual,
    /// `monochrome(arg0.xyz, arg1.xyz)`
    Monochrome,
    /// `-arg0.xyz`
    Negate,
    /// `float(arg0.xyz)`
    Float,
    /// `int(arg0.xyz)`
    Int,
    /// `uint(arg0.xyz)`
    Uint,
    /// `trunc(arg0.xyz)`
    Truncate,
    /// `floatBitsToInt(arg0.xyz)`
    FloatBitsToInt,
    /// `intBitsToFloat(arg0.xyz)`
    IntBitsToFloat,
    /// `uintBitsToFloat(arg0.xyz)`
    UintBitsToFloat,
    /// `inversesqrt(arg0.xyz)`
    InverseSqrt,
    /// `!arg0.xyz`
    Not,
    /// `arg0.xyz << arg1.xyz`
    LeftShift,
    /// `arg0.xyz >> arg1.xyz`
    RightShift,
    /// `exp2(arg0.xyz)`
    Exp2,
    /// `log2(arg0.xyz)`
    Log2,
    /// `sin(arg0.xyz)`
    Sin,
    /// `cos(arg0.xyz)`
    Cos,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AssignmentValueXyz {
    Texture(TextureAssignmentXyz),
    Attribute {
        name: SmolStr,
        channel: Option<ChannelXyz>,
    },
    Parameter {
        name: SmolStr,
        field: SmolStr,
        index: Option<usize>,
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

impl std::fmt::Display for ChannelXyz {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChannelXyz::Xyz => write!(f, "xyz"),
            ChannelXyz::X => write!(f, "xxx"),
            ChannelXyz::Y => write!(f, "yyy"),
            ChannelXyz::Z => write!(f, "zzz"),
            ChannelXyz::W => write!(f, "www"),
        }
    }
}

fn assignment_value(d: &Value, parameters: &MaterialParameters) -> Value {
    match d {
        Value::Int(i) => Value::Int(*i),
        Value::Float(f) => Value::Float(f.0.into()),
        Value::Parameter(b) => {
            if b.name != "U_Mate" {
                parameters
                    .get_parameter(b)
                    .map(|f| Value::Float(f.into()))
                    .unwrap_or_else(|| Value::Parameter(b.clone()))
            } else {
                Value::Parameter(b.clone())
            }
        }
        Value::Texture(t) => Value::Texture(t.clone()),
        Value::Attribute(a) => Value::Attribute(a.clone()),
    }
}

impl OutputAssignment {
    pub fn merge_xyz(&self, assignments: &[OutputExpr]) -> Option<OutputAssignmentXyz> {
        let mut assignments_xyz_index = IndexMap::new();
        let mut assignments_xyz = IndexSet::new();
        let i = merge_xyz_assignments(
            self.x?,
            self.y?,
            self.z?,
            assignments,
            &mut assignments_xyz_index,
            &mut assignments_xyz,
        )?;

        Some(OutputAssignmentXyz {
            expr: i,
            exprs: assignments_xyz.into_iter().collect(),
        })
    }
}

fn merge_xyz_assignments(
    x: usize,
    y: usize,
    z: usize,
    assignments: &[OutputExpr],
    assignments_xyz_index: &mut IndexMap<(usize, usize, usize), usize>,
    assignments_xyz: &mut IndexSet<AssignmentXyz>,
) -> Option<usize> {
    // Avoid processing the same set of assignments more than once.
    match assignments_xyz_index.get(&(x, y, z)) {
        Some(index) => Some(*index),
        None => {
            let x_assignment = assignments.get(x)?;
            let y_assignment = assignments.get(y)?;
            let z_assignment = assignments.get(z)?;

            let assignment_xyz = match (x_assignment, y_assignment, z_assignment) {
                (
                    OutputExpr::Func {
                        op: op_x,
                        args: args_x,
                    },
                    OutputExpr::Func {
                        op: op_y,
                        args: args_y,
                    },
                    OutputExpr::Func {
                        op: op_z,
                        args: args_z,
                    },
                ) => {
                    let op = op_xyz(*op_x, *op_y, *op_z)?;
                    if args_x.len() == args_y.len() && args_y.len() == args_z.len() {
                        // TODO: Convert args into vector args instead of keeping scalar args?
                        // TODO: This should more intelligently merge args based on the operation.
                        let mut args = Vec::new();
                        for ((x, y), z) in args_x.iter().zip(args_y.iter()).zip(args_z.iter()) {
                            let arg = merge_xyz_assignments(
                                *x,
                                *y,
                                *z,
                                assignments,
                                assignments_xyz_index,
                                assignments_xyz,
                            )?;
                            args.push(arg);
                        }
                        Some(AssignmentXyz::Func { op, args })
                    } else {
                        None
                    }
                }
                (OutputExpr::Value(vx), OutputExpr::Value(vy), OutputExpr::Value(vz)) => {
                    // TODO: Check that channels are one of the supported channels.
                    match (vx, vy, vz) {
                        (Value::Texture(tx), Value::Texture(ty), Value::Texture(tz)) => {
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
                            Value::Attribute(Attribute {
                                name: n_x,
                                channel: c_x,
                            }),
                            Value::Attribute(Attribute {
                                name: n_y,
                                channel: c_y,
                            }),
                            Value::Attribute(Attribute {
                                name: n_z,
                                channel: c_z,
                            }),
                        ) => Some(AssignmentXyz::Value(Some(AssignmentValueXyz::Attribute {
                            name: name_xyz(n_x, n_y, n_z)?,
                            channel: channel_xyz(*c_x, *c_y, *c_z)?,
                        }))),
                        (
                            Value::Parameter(Parameter {
                                name: n_x,
                                field: f_x,
                                index: i_x,
                                channel: c_x,
                            }),
                            Value::Parameter(Parameter {
                                name: n_y,
                                field: f_y,
                                index: i_y,
                                channel: c_y,
                            }),
                            Value::Parameter(Parameter {
                                name: n_z,
                                field: f_z,
                                index: i_z,
                                channel: c_z,
                            }),
                        ) => Some(AssignmentXyz::Value(Some(AssignmentValueXyz::Parameter {
                            name: name_xyz(n_x, n_y, n_z)?,
                            field: name_xyz(f_x, f_y, f_z)?,
                            index: index_xyz(*i_x, *i_y, *i_z)?,
                            channel: channel_xyz(*c_x, *c_y, *c_z)?,
                        }))),
                        (Value::Float(fx), Value::Float(fy), Value::Float(fz)) => {
                            Some(AssignmentXyz::Value(Some(AssignmentValueXyz::Float([
                                *fx, *fy, *fz,
                            ]))))
                        }
                        _ => None,
                    }
                }
                _ => None,
            }?;

            let index = assignments_xyz.insert_full(assignment_xyz).0;
            assignments_xyz_index.insert((x, y, z), index);
            Some(index)
        }
    }
}

fn op_xyz(x: Operation, y: Operation, z: Operation) -> Option<OperationXyz> {
    // Channel merging requires all operations to be the same.
    // Single channel vector operations like ReflectY need to output xyz to merge.
    // This simplifies support in applications without convenient channel swizzling like shader node editors.
    let (op_x, c_x) = operation_xyz_channel(x)?;
    let (op_y, c_y) = operation_xyz_channel(y)?;
    let (op_z, c_z) = operation_xyz_channel(z)?;

    // TODO: is it worth supporting cases like reflect(a.xyz, b.xyz).zzz?
    if op_x == op_y
        && op_y == op_z
        && matches!(
            [c_x, c_y, c_z],
            [Some('x'), Some('y'), Some('z')] | [None, None, None]
        )
    {
        Some(op_x)
    } else {
        None
    }
}

fn operation_xyz_channel(op: Operation) -> Option<(OperationXyz, Option<char>)> {
    // TODO: Support more operations as vector operations?
    match op {
        Operation::Unk => Some((OperationXyz::Unk, None)),
        Operation::Mix => Some((OperationXyz::Mix, None)),
        Operation::Mul => Some((OperationXyz::Mul, None)),
        Operation::Div => Some((OperationXyz::Div, None)),
        Operation::Add => Some((OperationXyz::Add, None)),
        Operation::Sub => Some((OperationXyz::Sub, None)),
        Operation::Fma => Some((OperationXyz::Fma, None)),
        Operation::MulRatio => Some((OperationXyz::MulRatio, None)),
        Operation::AddNormalX => None,
        Operation::AddNormalY => None,
        Operation::Overlay => Some((OperationXyz::Overlay, None)),
        Operation::Overlay2 => Some((OperationXyz::Overlay2, None)),
        Operation::OverlayRatio => Some((OperationXyz::OverlayRatio, None)),
        Operation::Power => Some((OperationXyz::Power, None)),
        Operation::Min => Some((OperationXyz::Min, None)),
        Operation::Max => Some((OperationXyz::Max, None)),
        Operation::Clamp => Some((OperationXyz::Clamp, None)),
        Operation::Abs => Some((OperationXyz::Abs, None)),
        Operation::Fresnel => Some((OperationXyz::Fresnel, None)),
        Operation::Sqrt => Some((OperationXyz::Sqrt, None)),
        Operation::TexMatrix => None,
        Operation::TexParallaxX => None,
        Operation::TexParallaxY => None,
        Operation::ReflectX => Some((OperationXyz::Reflect, Some('x'))),
        Operation::ReflectY => Some((OperationXyz::Reflect, Some('x'))),
        Operation::ReflectZ => Some((OperationXyz::Reflect, Some('x'))),
        Operation::Floor => Some((OperationXyz::Floor, None)),
        Operation::Select => Some((OperationXyz::Select, None)),
        Operation::Equal => Some((OperationXyz::Equal, None)),
        Operation::NotEqual => Some((OperationXyz::NotEqual, None)),
        Operation::Less => Some((OperationXyz::Less, None)),
        Operation::Greater => Some((OperationXyz::Greater, None)),
        Operation::LessEqual => Some((OperationXyz::LessEqual, None)),
        Operation::GreaterEqual => Some((OperationXyz::GreaterEqual, None)),
        Operation::Dot4 => None,
        Operation::NormalMapX => None,
        Operation::NormalMapY => None,
        Operation::NormalMapZ => None,
        Operation::MonochromeX => Some((OperationXyz::Monochrome, Some('x'))),
        Operation::MonochromeY => Some((OperationXyz::Monochrome, Some('y'))),
        Operation::MonochromeZ => Some((OperationXyz::Monochrome, Some('z'))),
        Operation::Negate => Some((OperationXyz::Negate, None)),
        Operation::FurInstanceAlpha => None,
        Operation::Float => Some((OperationXyz::Float, None)),
        Operation::Int => Some((OperationXyz::Int, None)),
        Operation::Uint => Some((OperationXyz::Uint, None)),
        Operation::Truncate => Some((OperationXyz::Truncate, None)),
        Operation::FloatBitsToInt => Some((OperationXyz::FloatBitsToInt, None)),
        Operation::IntBitsToFloat => Some((OperationXyz::IntBitsToFloat, None)),
        Operation::UintBitsToFloat => Some((OperationXyz::UintBitsToFloat, None)),
        Operation::InverseSqrt => Some((OperationXyz::InverseSqrt, None)),
        Operation::Not => Some((OperationXyz::Not, None)),
        Operation::LeftShift => Some((OperationXyz::LeftShift, None)),
        Operation::RightShift => Some((OperationXyz::RightShift, None)),
        Operation::PartialDerivativeX => None,
        Operation::PartialDerivativeY => None,
        Operation::Exp2 => Some((OperationXyz::Exp2, None)),
        Operation::Log2 => Some((OperationXyz::Log2, None)),
        Operation::Sin => Some((OperationXyz::Sin, None)),
        Operation::Cos => Some((OperationXyz::Cos, None)),
    }
}

fn name_xyz(x: &SmolStr, y: &SmolStr, z: &SmolStr) -> Option<SmolStr> {
    if x == y && y == z {
        Some(x.clone())
    } else {
        None
    }
}

fn index_xyz(x: Option<usize>, y: Option<usize>, z: Option<usize>) -> Option<Option<usize>> {
    if x == y && y == z { Some(x) } else { None }
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
    // Use the existing indices to avoid costly caching or recursion.
    let assignments = shader
        .exprs
        .iter()
        .map(|e| expr_with_parameter_values(parameters, e))
        .collect();

    OutputAssignments {
        output_assignments: [0, 1, 2, 3, 4, 5].map(|i| output_assignment(shader, i)),
        outline_width: shader
            .outline_width
            .as_ref()
            .map(|d| assignment_value(d, parameters)),
        normal_intensity: shader.normal_intensity,
        val_inf_intensity: shader.val_inf_intensity,
        exprs: assignments,
    }
}

fn output_assignment(shader: &ShaderProgram, output_index: usize) -> OutputAssignment {
    OutputAssignment {
        x: output_channel_assignment(shader, output_index, 'x'),
        y: output_channel_assignment(shader, output_index, 'y'),
        z: output_channel_assignment(shader, output_index, 'z'),
        w: output_channel_assignment(shader, output_index, 'w'),
    }
}

fn output_channel_assignment(
    shader: &ShaderProgram,
    output_index: usize,
    channel: char,
) -> Option<usize> {
    let output = format!("o{output_index}.{channel}");
    shader
        .output_dependencies
        .get(&SmolStr::from(output))
        .copied()
}

fn expr_with_parameter_values(parameters: &MaterialParameters, expr: &OutputExpr) -> OutputExpr {
    match expr {
        OutputExpr::Value(d) => OutputExpr::Value(assignment_value(d, parameters)),
        OutputExpr::Func { op, args } => OutputExpr::Func {
            op: *op,
            args: args.clone(),
        },
    }
}

pub(crate) fn infer_assignment_from_textures(
    textures: &[super::Texture],
    image_textures: &[ImageTexture],
) -> OutputAssignments {
    // No assignment data is available.
    // Guess reasonable defaults based on the texture names or types.
    let mut assignments = IndexSet::new();

    let mut assignment = |i: Option<usize>, c: usize| {
        let u = assignments
            .insert_full(OutputExpr::Value(Value::Attribute(Attribute {
                name: "vTex0".into(),
                channel: Some('x'),
            })))
            .0;
        let v = assignments
            .insert_full(OutputExpr::Value(Value::Attribute(Attribute {
                name: "vTex0".into(),
                channel: Some('y'),
            })))
            .0;
        Some(
            assignments
                .insert_full(OutputExpr::Value(Value::Texture(Texture {
                    name: format!("s{}", i?).into(),
                    channel: Some(['x', 'y', 'z', 'w'][c]),
                    texcoords: vec![u, v],
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
        val_inf_intensity: None,
        exprs: assignments.into_iter().collect(),
    }
}

#[cfg(test)]
mod tests {
    use crate::shader_database::Texture;

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
            OutputExpr::Value(Value::Float(0.0.into())),
            OutputExpr::Value(Value::Texture(Texture {
                name: "s0".into(),
                channel: Some('z'),
                texcoords: vec![0, 0],
            })),
            OutputExpr::Value(Value::Texture(Texture {
                name: "s0".into(),
                channel: Some('y'),
                texcoords: vec![0, 0],
            })),
            OutputExpr::Value(Value::Texture(Texture {
                name: "s0".into(),
                channel: Some('x'),
                texcoords: vec![0, 0],
            })),
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
            OutputExpr::Value(Value::Float(0.0.into())),
            OutputExpr::Value(Value::Texture(Texture {
                name: "s0".into(),
                channel: Some('w'),
                texcoords: vec![0, 0],
            })),
        ];
        assert_eq!(
            Some(OutputAssignmentXyz {
                expr: 0,
                exprs: vec![AssignmentXyz::Value(Some(AssignmentValueXyz::Texture(
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
            OutputExpr::Value(Value::Float(0.0.into())),
            OutputExpr::Value(Value::Texture(Texture {
                name: "s0".into(),
                channel: Some('x'),
                texcoords: vec![0, 0],
            })),
            OutputExpr::Value(Value::Float(1.0.into())),
            OutputExpr::Value(Value::Attribute(Attribute {
                name: "vColor".into(),
                channel: Some('x'),
            })),
            OutputExpr::Value(Value::Texture(Texture {
                name: "s0".into(),
                channel: Some('y'),
                texcoords: vec![0, 0],
            })),
            OutputExpr::Value(Value::Float(2.0.into())),
            OutputExpr::Value(Value::Attribute(Attribute {
                name: "vColor".into(),
                channel: Some('y'),
            })),
            OutputExpr::Value(Value::Texture(Texture {
                name: "s0".into(),
                channel: Some('z'),
                texcoords: vec![0, 0],
            })),
            OutputExpr::Value(Value::Float(3.0.into())),
            OutputExpr::Value(Value::Attribute(Attribute {
                name: "vColor".into(),
                channel: Some('z'),
            })),
            OutputExpr::Func {
                op: Operation::Fma,
                args: vec![1, 2, 3],
            },
            OutputExpr::Func {
                op: Operation::Fma,
                args: vec![4, 5, 6],
            },
            OutputExpr::Func {
                op: Operation::Fma,
                args: vec![7, 8, 9],
            },
        ];
        assert_eq!(
            Some(OutputAssignmentXyz {
                expr: 3,
                exprs: vec![
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
                        op: OperationXyz::Fma,
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
