use smol_str::SmolStr;
use xc3_model::shader_database::{
    AssignmentValueXyz, Attribute, ChannelXyz, Operation, OperationXyz, OutputExpr, OutputExprXyz,
    Parameter, TextureAssignmentXyz, Value,
};

// Faster than the default hash implementation.
type IndexSet<T> = indexmap::IndexSet<T, ahash::RandomState>;
type IndexMap<K, V> = indexmap::IndexMap<K, V, ahash::RandomState>;

pub fn merge_xyz_assignments(
    x: usize,
    y: usize,
    z: usize,
    assignments: &[OutputExpr],
    assignments_xyz_index: &mut IndexMap<(usize, usize, usize), usize>,
    assignments_xyz: &mut IndexSet<OutputExprXyz>,
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
                        let args = merge_args(
                            op,
                            args_x,
                            args_y,
                            args_z,
                            assignments,
                            assignments_xyz_index,
                            assignments_xyz,
                        )?;
                        Some(OutputExprXyz::Func { op, args })
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
                                Some(OutputExprXyz::Value(AssignmentValueXyz::Texture(t_xyz)))
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
                        ) => Some(OutputExprXyz::Value(AssignmentValueXyz::Attribute {
                            name: name_xyz(n_x, n_y, n_z)?,
                            channel: channel_xyz(*c_x, *c_y, *c_z)?,
                        })),
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
                        ) => Some(OutputExprXyz::Value(AssignmentValueXyz::Parameter {
                            name: name_xyz(n_x, n_y, n_z)?,
                            field: name_xyz(f_x, f_y, f_z)?,
                            index: index_xyz(*i_x, *i_y, *i_z)?,
                            channel: channel_xyz(*c_x, *c_y, *c_z)?,
                        })),
                        (Value::Float(fx), Value::Float(fy), Value::Float(fz)) => {
                            Some(OutputExprXyz::Value(AssignmentValueXyz::Float([
                                *fx, *fy, *fz,
                            ])))
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

// TODO: make this reusable using some sort of trait?
fn merge_args(
    op: OperationXyz,
    args_x: &[usize],
    args_y: &[usize],
    args_z: &[usize],
    assignments: &[OutputExpr],
    assignments_xyz_index: &mut IndexMap<(usize, usize, usize), usize>,
    assignments_xyz: &mut IndexSet<OutputExprXyz>,
) -> Option<Vec<usize>> {
    let mut args = Vec::new();

    // TODO: Merge incompatible scalar args into vector args instead of returning None.
    match op {
        OperationXyz::Monochrome => {
            // TODO: Check that all args are the same?
            let rgb = merge_xyz_assignments(
                *args_x.get(0)?,
                *args_y.get(1)?,
                *args_z.get(2)?,
                assignments,
                assignments_xyz_index,
                assignments_xyz,
            )?;
            args.push(rgb);

            // TODO: This should be the same scalar for all channels?
            let ratio = merge_xyz_assignments(
                *args_x.get(3)?,
                *args_y.get(3)?,
                *args_z.get(3)?,
                assignments,
                assignments_xyz_index,
                assignments_xyz,
            )?;
            args.push(ratio);
        }
        _ => {
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
        }
    }

    Some(args)
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

#[cfg(test)]
mod tests {
    use super::*;

    use xc3_model::shader_database::Texture;

    fn merge_xyz(
        x: usize,
        y: usize,
        z: usize,
        assignments: &[OutputExpr],
    ) -> Option<(usize, Vec<OutputExprXyz>)> {
        let mut assignments_xyz_index = IndexMap::default();
        let mut assignments_xyz = IndexSet::default();
        let index = merge_xyz_assignments(
            x,
            y,
            z,
            assignments,
            &mut assignments_xyz_index,
            &mut assignments_xyz,
        )?;
        Some((index, assignments_xyz.into_iter().collect()))
    }

    #[test]
    fn merge_xyz_empty() {
        assert_eq!(None, merge_xyz(0, 0, 0, &[]));
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
        assert_eq!(None, merge_xyz(1, 2, 3, &assignments));
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
            Some((
                0,
                vec![OutputExprXyz::Value(AssignmentValueXyz::Texture(
                    TextureAssignmentXyz {
                        name: "s0".into(),
                        channel: Some(ChannelXyz::W),
                        texcoords: vec![0, 0]
                    }
                ))]
            )),
            merge_xyz(1, 1, 1, &assignments)
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
            Some((
                3,
                vec![
                    OutputExprXyz::Value(AssignmentValueXyz::Texture(TextureAssignmentXyz {
                        name: "s0".into(),
                        channel: Some(ChannelXyz::Xyz),
                        texcoords: vec![0, 0]
                    })),
                    OutputExprXyz::Value(AssignmentValueXyz::Float([
                        1.0.into(),
                        2.0.into(),
                        3.0.into()
                    ])),
                    OutputExprXyz::Value(AssignmentValueXyz::Attribute {
                        name: "vColor".into(),
                        channel: Some(ChannelXyz::Xyz)
                    }),
                    OutputExprXyz::Func {
                        op: OperationXyz::Fma,
                        args: vec![0, 1, 2]
                    }
                ]
            )),
            merge_xyz(10, 11, 12, &assignments)
        );
    }
}
