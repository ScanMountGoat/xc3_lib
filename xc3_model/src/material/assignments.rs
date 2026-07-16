use indexmap::IndexSet;
use smol_str::format_smolstr;
use xc3_lib::mxmd::TextureUsage;

use crate::{
    ImageTexture,
    shader_database::{
        Attribute, ChannelXyz, OutputExpr, OutputExprXyz, ShaderProgram, Texture, TextureXyz,
        Value, ValueXyz,
    },
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

    /// Index into [exprs](#structfield.exprs) for the intensity map for normal mapping.
    pub normal_intensity: Option<usize>,

    /// Index into [exprs](#structfield.exprs) for the intensity for vValInf normal mapping.
    pub val_inf_intensity: Option<usize>,

    /// Index into [exprs](#structfield.exprs) for the fragment discard condition.
    pub discard_condition: Option<usize>,

    /// Unique values shared between all outputs.
    pub exprs: Vec<OutputExpr>,

    /// Unique merged XYZ values shared between all outputs.
    pub exprs_xyz: Vec<OutputExprXyz>,
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

    /// Index into [exprs_xyz](struct.OutputAssignments.html#structfield.exprs_xyz) for the xyz value
    /// if merging the XYZ channels is possible.
    pub xyz: Option<usize>,
}

fn assignment_value(v: &Value, parameters: &MaterialParameters) -> Value {
    match v {
        Value::Int(i) => Value::Int(*i),
        Value::Float(f) => Value::Float(*f),
        Value::Parameter(p) => {
            if p.name != "U_Mate" {
                parameters
                    .get_parameter(p)
                    .map(|f| Value::Float(f.into()))
                    .unwrap_or_else(|| Value::Parameter(p.clone()))
            } else {
                Value::Parameter(p.clone())
            }
        }
        Value::Texture(t) => Value::Texture(t.clone()),
        Value::Attribute(a) => Value::Attribute(a.clone()),
    }
}

fn assignment_value_xyz(v: &ValueXyz, parameters: &MaterialParameters) -> ValueXyz {
    match v {
        ValueXyz::Float(f) => ValueXyz::Float(*f),
        ValueXyz::Parameter(p) => {
            if p.name != "U_Mate" {
                parameters
                    .get_parameter_xyz(p)
                    .map(|f| ValueXyz::Float(f.map(Into::into)))
                    .unwrap_or_else(|| ValueXyz::Parameter(p.clone()))
            } else {
                ValueXyz::Parameter(p.clone())
            }
        }
        ValueXyz::Texture(t) => ValueXyz::Texture(t.clone()),
        ValueXyz::Attribute(a) => ValueXyz::Attribute(a.clone()),
    }
}

pub(crate) fn output_assignments(
    shader: &ShaderProgram,
    parameters: &MaterialParameters,
) -> OutputAssignments {
    // Use the existing indices to avoid costly caching or recursion.
    let exprs = shader
        .exprs
        .iter()
        .map(|e| expr_with_parameter_values(parameters, e))
        .collect();

    let exprs_xyz = shader
        .exprs_xyz
        .iter()
        .map(|e| expr_xyz_with_parameter_values(parameters, e))
        .collect();

    OutputAssignments {
        output_assignments: [0, 1, 2, 3, 4, 5].map(|i| output_assignment(shader, i)),
        outline_width: shader
            .outline_width
            .as_ref()
            .map(|d| assignment_value(d, parameters)),
        normal_intensity: shader.normal_intensity,
        val_inf_intensity: shader.val_inf_intensity,
        discard_condition: shader.discard_condition,
        exprs,
        exprs_xyz,
    }
}

fn output_assignment(shader: &ShaderProgram, output_index: usize) -> OutputAssignment {
    OutputAssignment {
        x: output_channel_assignment(shader, output_index, 'x'),
        y: output_channel_assignment(shader, output_index, 'y'),
        z: output_channel_assignment(shader, output_index, 'z'),
        w: output_channel_assignment(shader, output_index, 'w'),
        xyz: shader
            .output_dependencies_xyz
            .get(&format_smolstr!("o{output_index}.xyz"))
            .copied(),
    }
}

fn output_channel_assignment(
    shader: &ShaderProgram,
    output_index: usize,
    channel: char,
) -> Option<usize> {
    shader
        .output_dependencies
        .get(&format_smolstr!("o{output_index}.{channel}"))
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

fn expr_xyz_with_parameter_values(
    parameters: &MaterialParameters,
    expr: &OutputExprXyz,
) -> OutputExprXyz {
    match expr {
        OutputExprXyz::Value(d) => OutputExprXyz::Value(assignment_value_xyz(d, parameters)),
        OutputExprXyz::Func { op, args, channel } => OutputExprXyz::Func {
            op: *op,
            args: args.clone(),
            channel: *channel,
        },
    }
}

pub(crate) fn infer_assignment_from_textures(
    textures: &[super::Texture],
    image_textures: &[ImageTexture],
) -> OutputAssignments {
    // No assignment data is available.
    // Guess reasonable defaults based on the texture names or types.
    let mut exprs = IndexSet::new();
    let mut exprs_xyz = IndexSet::new();

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
                x: texture_expr(&mut exprs, color_index, 'x'),
                y: texture_expr(&mut exprs, color_index, 'y'),
                z: texture_expr(&mut exprs, color_index, 'z'),
                w: texture_expr(&mut exprs, color_index, 'w'),
                xyz: texture_expr_xyz(&mut exprs, &mut exprs_xyz, color_index),
            },
            OutputAssignment::default(),
            OutputAssignment {
                x: texture_expr(&mut exprs, normal_index, 'x'),
                y: texture_expr(&mut exprs, normal_index, 'y'),
                ..Default::default()
            },
            OutputAssignment::default(),
            OutputAssignment::default(),
            OutputAssignment {
                x: texture_expr(&mut exprs, spm_index, 'x'),
                y: texture_expr(&mut exprs, spm_index, 'y'),
                z: texture_expr(&mut exprs, spm_index, 'z'),
                xyz: texture_expr_xyz(&mut exprs, &mut exprs_xyz, spm_index),
                ..Default::default()
            },
        ],
        outline_width: None,
        normal_intensity: None,
        val_inf_intensity: None,
        discard_condition: None,
        exprs: exprs.into_iter().collect(),
        exprs_xyz: exprs_xyz.into_iter().collect(),
    }
}

fn texture_expr(exprs: &mut IndexSet<OutputExpr>, i: Option<usize>, c: char) -> Option<usize> {
    let u = exprs
        .insert_full(OutputExpr::Value(Value::Attribute(Attribute {
            name: "vTex0".into(),
            channel: Some('x'),
        })))
        .0;
    let v = exprs
        .insert_full(OutputExpr::Value(Value::Attribute(Attribute {
            name: "vTex0".into(),
            channel: Some('y'),
        })))
        .0;
    Some(
        exprs
            .insert_full(OutputExpr::Value(Value::Texture(Texture {
                name: format_smolstr!("s{}", i?),
                channel: Some(c),
                texcoords: vec![u, v],
            })))
            .0,
    )
}

fn texture_expr_xyz(
    exprs: &mut IndexSet<OutputExpr>,
    exprs_xyz: &mut IndexSet<OutputExprXyz>,
    i: Option<usize>,
) -> Option<usize> {
    let u = exprs
        .insert_full(OutputExpr::Value(Value::Attribute(Attribute {
            name: "vTex0".into(),
            channel: Some('x'),
        })))
        .0;
    let v = exprs
        .insert_full(OutputExpr::Value(Value::Attribute(Attribute {
            name: "vTex0".into(),
            channel: Some('y'),
        })))
        .0;
    Some(
        exprs_xyz
            .insert_full(OutputExprXyz::Value(ValueXyz::Texture(TextureXyz {
                name: format_smolstr!("s{}", i?),
                channel: Some(ChannelXyz::Xyz),
                texcoords: vec![u, v],
            })))
            .0,
    )
}
