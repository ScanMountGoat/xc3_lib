use indexmap::IndexSet;
use smol_str::format_smolstr;
use xc3_lib::mxmd::TextureUsage;

use crate::{
    ImageTexture,
    shader_database::{Attribute, OutputExpr, OutputExprXyz, ShaderProgram, Texture, Value},
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
        exprs_xyz: shader.exprs_xyz.clone(),
    }
}

fn output_assignment(shader: &ShaderProgram, output_index: usize) -> OutputAssignment {
    OutputAssignment {
        x: output_channel_assignment(shader, output_index, 'x'),
        y: output_channel_assignment(shader, output_index, 'y'),
        z: output_channel_assignment(shader, output_index, 'z'),
        w: output_channel_assignment(shader, output_index, 'w'),
        // TODO: Get output_index .xyz from map
        xyz: None,
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
                    name: format_smolstr!("s{}", i?),
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
                xyz: None, // TODO: this can be some
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
        exprs_xyz: Vec::new(),
    }
}
