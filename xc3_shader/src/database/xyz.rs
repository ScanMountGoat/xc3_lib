use crate::expr::{
    OutputExpr,
    xyz::{ExprCacheXyz, MergeXyzArgs, OperationXyzChannel, merge_xyz_exprs},
};
use xc3_model::shader_database::{Operation, OperationXyz};

impl OperationXyzChannel for Operation {
    type OperationXyz = OperationXyz;

    fn operation_xyz_channel(&self) -> Option<(Self::OperationXyz, Option<char>)> {
        // TODO: Support more operations as vector operations?
        match *self {
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
}

impl MergeXyzArgs<Operation> for OperationXyz {
    fn merge_xyz_args(
        &self,
        args_x: &[usize],
        args_y: &[usize],
        args_z: &[usize],
        exprs: &[OutputExpr<Operation>],
        exprs_xyz: &mut ExprCacheXyz<Self>,
    ) -> Option<Vec<usize>> {
        let mut args = Vec::new();

        // TODO: Merge incompatible scalar args into vector args instead of returning None.
        match *self {
            OperationXyz::Monochrome => {
                // TODO: Check that all args are the same?
                let rgb = merge_xyz_exprs(
                    *args_x.first()?,
                    *args_y.get(1)?,
                    *args_z.get(2)?,
                    exprs,
                    exprs_xyz,
                )?;
                args.push(rgb);

                // TODO: This should be the same scalar for all channels?
                let ratio = merge_xyz_exprs(
                    *args_x.get(3)?,
                    *args_y.get(3)?,
                    *args_z.get(3)?,
                    exprs,
                    exprs_xyz,
                )?;
                args.push(ratio);
            }
            OperationXyz::Reflect => {
                // TODO: Check that all args are the same.
                let eye = merge_xyz_exprs(args_x[0], args_x[1], args_x[2], exprs, exprs_xyz)?;
                args.push(eye);

                let normal = merge_xyz_exprs(args_x[3], args_x[4], args_x[5], exprs, exprs_xyz)?;
                args.push(normal);
            }
            _ => {
                for ((x, y), z) in args_x.iter().zip(args_y.iter()).zip(args_z.iter()) {
                    let arg = merge_xyz_exprs(*x, *y, *z, exprs, exprs_xyz)?;
                    args.push(arg);
                }
            }
        }

        Some(args)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::expr::{
        Attribute, Texture, Value,
        xyz::{AttributeXyz, ChannelXyz, OutputExprXyz, TextureXyz, ValueXyz},
    };

    fn merge_xyz(
        x: usize,
        y: usize,
        z: usize,
        exprs: &[OutputExpr<Operation>],
    ) -> Option<(usize, Vec<OutputExprXyz<OperationXyz>>)> {
        let mut exprs_xyz = ExprCacheXyz::default();
        let index = merge_xyz_exprs(x, y, z, exprs, &mut exprs_xyz)?;
        Some((index, exprs_xyz.into_exprs()))
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
                vec![OutputExprXyz::Value(ValueXyz::Texture(TextureXyz {
                    name: "s0".into(),
                    channel: Some(ChannelXyz::W),
                    texcoords: vec![0, 0]
                }))]
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
                    OutputExprXyz::Value(ValueXyz::Texture(TextureXyz {
                        name: "s0".into(),
                        channel: Some(ChannelXyz::Xyz),
                        texcoords: vec![0, 0]
                    })),
                    OutputExprXyz::Value(ValueXyz::Float([1.0.into(), 2.0.into(), 3.0.into()])),
                    OutputExprXyz::Value(ValueXyz::Attribute(AttributeXyz {
                        name: "vColor".into(),
                        channel: Some(ChannelXyz::Xyz)
                    })),
                    OutputExprXyz::Func {
                        op: OperationXyz::Fma,
                        args: vec![0, 1, 2],
                        channel: None
                    }
                ]
            )),
            merge_xyz(10, 11, 12, &assignments)
        );
    }
}
