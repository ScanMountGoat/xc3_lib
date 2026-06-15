use ordered_float::OrderedFloat;
use smol_str::SmolStr;
use std::hash::Hash;

use crate::expr::{Attribute, OutputExpr, Parameter, Value};

// Faster than the default hash implementation.
type IndexSet<T> = indexmap::IndexSet<T, ahash::RandomState>;
type IndexMap<K, V> = indexmap::IndexMap<K, V, ahash::RandomState>;

pub trait OperationXyzChannel {
    type OperationXyz;

    fn operation_xyz_channel(&self) -> Option<(Self::OperationXyz, Option<char>)>;
}

pub trait MergeXyzArgs<Op>: Sized {
    fn merge_xyz_args(
        &self,
        args_x: &[usize],
        args_y: &[usize],
        args_z: &[usize],
        exprs: &[OutputExpr<Op>],
        exprs_xyz: &mut ExprCacheXyz<Self>,
    ) -> Option<Vec<usize>>;
}

// TODO: make this generic over the vector length?
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum OutputExprXyz<Op> {
    Value(ValueXyz),
    Func {
        op: Op,
        /// Indices for the [OutputExprXyz] for the function argument list `[arg0, arg1, ...]`.
        args: Vec<usize>,
        channel: Option<ChannelXyz>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ValueXyz {
    Texture(TextureXyz),
    Attribute(AttributeXyz),
    Parameter(ParameterXyz),
    Float([OrderedFloat<f32>; 3]),
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct TextureXyz {
    /// The name of the texture like `s0` or `gTResidentTex09`.
    pub name: SmolStr,
    /// Indices into scalar [OutputExpr] for the texture coordinates.
    pub texcoords: Vec<usize>,
    pub channel: Option<ChannelXyz>,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct AttributeXyz {
    pub name: SmolStr,
    pub channel: Option<ChannelXyz>,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct ParameterXyz {
    pub name: SmolStr,
    pub field: SmolStr,
    pub index: Option<usize>,
    pub channel: Option<ChannelXyz>,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChannelXyz {
    Xyz,
    X,
    Y,
    Z,
    W,
}

// Cache graph expr -> output expr index to visit nodes only once.
#[derive(Debug, Default)]
pub struct ExprCacheXyz<Op> {
    exprs: IndexSet<OutputExprXyz<Op>>,
    expr_xyz_index: IndexMap<(usize, usize, usize), usize>,
}

impl<Op> ExprCacheXyz<Op> {
    /// Get the collection of unique [OutputExprXyz].
    pub fn into_exprs(self) -> Vec<OutputExprXyz<Op>> {
        self.exprs.into_iter().collect()
    }
}

pub fn merge_xyz_exprs<Op>(
    x: usize,
    y: usize,
    z: usize,
    exprs: &[OutputExpr<Op>],
    exprs_xyz: &mut ExprCacheXyz<Op::OperationXyz>,
) -> Option<usize>
where
    Op: OperationXyzChannel + Copy,
    <Op as OperationXyzChannel>::OperationXyz: MergeXyzArgs<Op> + PartialEq + Eq + Hash,
{
    // Avoid processing the same set of assignments more than once.
    match exprs_xyz.expr_xyz_index.get(&(x, y, z)) {
        Some(index) => Some(*index),
        None => {
            let x_assignment = exprs.get(x)?;
            let y_assignment = exprs.get(y)?;
            let z_assignment = exprs.get(z)?;

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
                    let (op, channel) = op_xyz(*op_x, *op_y, *op_z)?;
                    if args_x.len() == args_y.len() && args_y.len() == args_z.len() {
                        let args = op.merge_xyz_args(args_x, args_y, args_z, exprs, exprs_xyz)?;
                        Some(OutputExprXyz::Func { op, args, channel })
                    } else {
                        None
                    }
                }
                (OutputExpr::Value(vx), OutputExpr::Value(vy), OutputExpr::Value(vz)) => {
                    // TODO: Check that channels are one of the supported channels.
                    match (vx, vy, vz) {
                        (Value::Texture(tx), Value::Texture(ty), Value::Texture(tz)) => {
                            if tx.texcoords == ty.texcoords && ty.texcoords == tz.texcoords {
                                Some(OutputExprXyz::Value(ValueXyz::Texture(TextureXyz {
                                    name: name_xyz(&tx.name, &ty.name, &tz.name)?,
                                    channel: channel_xyz(tx.channel, ty.channel, tz.channel)?,
                                    texcoords: tx.texcoords.clone(), // TODO: These should refer to the scalar assignments?
                                })))
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
                        ) => Some(OutputExprXyz::Value(ValueXyz::Attribute(AttributeXyz {
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
                        ) => Some(OutputExprXyz::Value(ValueXyz::Parameter(ParameterXyz {
                            name: name_xyz(n_x, n_y, n_z)?,
                            field: name_xyz(f_x, f_y, f_z)?,
                            index: index_xyz(*i_x, *i_y, *i_z)?,
                            channel: channel_xyz(*c_x, *c_y, *c_z)?,
                        }))),
                        (Value::Float(fx), Value::Float(fy), Value::Float(fz)) => {
                            Some(OutputExprXyz::Value(ValueXyz::Float([*fx, *fy, *fz])))
                        }
                        _ => None,
                    }
                }
                _ => None,
            }?;

            let index = exprs_xyz.exprs.insert_full(assignment_xyz).0;
            exprs_xyz.expr_xyz_index.insert((x, y, z), index);
            Some(index)
        }
    }
}

fn op_xyz<Op: OperationXyzChannel>(
    x: Op,
    y: Op,
    z: Op,
) -> Option<(Op::OperationXyz, Option<ChannelXyz>)>
where
    <Op as OperationXyzChannel>::OperationXyz: PartialEq,
{
    // Channel merging requires all operations to be the same.
    // Single channel vector operations like ReflectY need to output xyz or a scalar like xxx to merge.
    // This simplifies support in applications without convenient channel swizzling like shader node editors.
    let (op_x, c_x) = x.operation_xyz_channel()?;
    let (op_y, c_y) = y.operation_xyz_channel()?;
    let (op_z, c_z) = z.operation_xyz_channel()?;

    if op_x == op_y && op_y == op_z {
        match [c_x, c_y, c_z] {
            [Some('x'), Some('y'), Some('z')] => Some((op_x, Some(ChannelXyz::Xyz))),
            [Some('x'), Some('x'), Some('x')] => Some((op_x, Some(ChannelXyz::X))),
            [Some('y'), Some('y'), Some('y')] => Some((op_x, Some(ChannelXyz::Y))),
            [Some('z'), Some('z'), Some('z')] => Some((op_x, Some(ChannelXyz::Z))),
            [Some('w'), Some('w'), Some('w')] => Some((op_x, Some(ChannelXyz::W))),
            [None, None, None] => Some((op_x, None)),
            _ => None,
        }
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

impl<Op> std::fmt::Display for OutputExprXyz<Op>
where
    Op: std::fmt::Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputExprXyz::Value(d) => write!(f, "{d}"),
            OutputExprXyz::Func { op, args, channel } => {
                let args: Vec<_> = args.iter().map(|a| format!("var{a}")).collect();
                write!(f, "{op}({}){}", args.join(", "), channels_xyz(*channel))
            }
        }
    }
}

impl std::fmt::Display for ValueXyz {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValueXyz::Float(c) => write!(f, "{c:?}"),
            ValueXyz::Parameter(p) => write!(f, "{p}"),
            ValueXyz::Texture(t) => write!(f, "{t}"),
            ValueXyz::Attribute(a) => write!(f, "{a}"),
        }
    }
}

impl std::fmt::Display for ParameterXyz {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}{}{}{}",
            self.name,
            if !self.field.is_empty() {
                format!(".{}", self.field)
            } else {
                String::new()
            },
            self.index.map(|i| format!("[{i}]")).unwrap_or_default(),
            channels_xyz(self.channel)
        )
    }
}

impl std::fmt::Display for TextureXyz {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let args: Vec<_> = self.texcoords.iter().map(|t| format!("var{t}")).collect();
        write!(
            f,
            "Texture({}, {}){}",
            self.name,
            args.join(", "),
            channels_xyz(self.channel)
        )
    }
}

impl std::fmt::Display for AttributeXyz {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", self.name, channels_xyz(self.channel))
    }
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

fn channels_xyz(c: Option<ChannelXyz>) -> String {
    c.map(|c| format!(".{c}")).unwrap_or_default()
}
