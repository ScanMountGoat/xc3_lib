//! Database for compiled shader metadata for more accurate rendering.
//!
//! In game shaders are precompiled and embedded in files like `.wismt`.
//! These types represent precomputed metadata like assignments to G-Buffer textures.
//! This is necessary for determining the usage of a texture like albedo or normal map
//! since the assignments are compiled into the shader code itself.
//!
//! Shader database files should be generated using the xc3_shader CLI tool.
//! Applications can parse the data with [ShaderDatabase::from_file]
//! to avoid needing to generate this data at runtime.

use std::{collections::BTreeMap, path::Path};

use indexmap::IndexMap;
use ordered_float::OrderedFloat;
use smol_str::SmolStr;

use crate::error::{LoadShaderDatabaseError, SaveShaderDatabaseError};

mod io;

/// Metadata for the assigned shaders for all models and maps in a game dump.
#[derive(Debug, PartialEq, Clone)]
pub struct ShaderDatabase(io::ShaderDatabaseIndexed);

impl ShaderDatabase {
    /// Load the database data from `path`.
    #[tracing::instrument(skip_all)]
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, LoadShaderDatabaseError> {
        // Avoid converting the indexed database to improve load times.
        // Most uses cases will only need data for a single model or map.
        let indexed = io::ShaderDatabaseIndexed::from_file(path)?;
        Ok(Self(indexed))
    }

    /// Serialize and save the database data to `path`.
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), SaveShaderDatabaseError> {
        self.0.save(path)?;
        Ok(())
    }

    /// The shader information for the specified shader program.
    pub fn shader_program(&self, hash: ProgramHash) -> Option<ShaderProgram> {
        self.0.shader_program(hash)
    }

    /// Create the internal database representation from non indexed data.
    pub fn from_programs(programs: BTreeMap<ProgramHash, ShaderProgram>) -> Self {
        Self(io::ShaderDatabaseIndexed::from_programs(programs))
    }

    /// Create a new database with combined entries from `other`.
    pub fn merge(&self, other: &Self) -> Self {
        Self(self.0.merge(&other.0))
    }
}

/// Unique identifier for compiled shader program data.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub struct ProgramHash(u32);

impl ProgramHash {
    /// Hash a legacy shader program.
    pub fn from_mths(mths: &xc3_lib::mths::Mths) -> Self {
        let mut hasher = crc32fast::Hasher::new();
        // TODO: Update metadata separately instead of entire buffer?
        hasher.update(&mths.data);
        Self(hasher.finalize())
    }

    /// Hash a shader program.
    pub fn from_spch_program(
        program: &xc3_lib::spch::ShaderProgram,
        vertex: &Option<xc3_lib::spch::ShaderBinary>,
        fragment: &Option<xc3_lib::spch::ShaderBinary>,
    ) -> Self {
        // Hash both code and metadata since programs with the same code
        // can have slightly different uniforms, buffers, etc.
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&program.program_data);

        if let Some(fragment) = fragment {
            hasher.update(&fragment.program_binary);
        }
        if let Some(vertex) = vertex {
            hasher.update(&vertex.program_binary);
        }

        Self(hasher.finalize())
    }
}

/// A single shader program with a vertex and fragment shader.
#[derive(Debug, PartialEq, Clone, Default)]
pub struct ShaderProgram {
    /// The input values used to initialize each fragment output.
    ///
    /// Each dependency can be thought of as a link
    /// between the dependency node and group output in a shader node graph.
    ///
    /// This assignment information is needed to accurately recreate the G-Buffer texture values.
    /// Renderers can generate unique shaders for each model
    /// or select inputs in a shared shader at render time like xc3_wgpu.
    /// Node based editors like Blender's shader editor should use these values
    /// to determine how to construct node groups.
    pub output_dependencies: IndexMap<SmolStr, OutputDependencies>,

    /// The parameter multiplied by vertex alpha to determine outline width.
    pub outline_width: Option<Dependency>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Clone)]
pub struct OutputDependencies {
    // TODO: This is redundant with layers.
    /// All of the possible dependencies that may affect the output.
    pub dependencies: Vec<Dependency>,
    /// Layering information if this output blends multiple texture dependencies.
    pub layers: Vec<Layer>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum Dependency {
    Constant(OrderedFloat<f32>),
    Buffer(BufferDependency),
    Texture(TextureDependency),
    Attribute(AttributeDependency),
}

/// A single buffer access like `UniformBuffer.field[0].y` or `UniformBuffer.field.y` in GLSL.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
pub struct BufferDependency {
    pub name: SmolStr,
    pub field: SmolStr,
    pub index: Option<usize>,
    pub channel: Option<char>,
}

/// A single texture access like `texture(s0, tex0.xy).rgb` in GLSL.
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct TextureDependency {
    pub name: SmolStr,
    pub channel: Option<char>,
    /// Texture coordinate values used for the texture function call.
    pub texcoords: Vec<TexCoord>,
}

/// A texture coordinate attribute with optional transform parameters.
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct TexCoord {
    /// The name of the attribute like "in_attr4".
    pub name: SmolStr,
    /// The accessed channels like "x" or "y".
    pub channel: Option<char>,
    pub params: Option<TexCoordParams>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum TexCoordParams {
    // A single scale parameter.
    Scale(BufferDependency),

    /// A float2x4 texture matrix.
    /// ```text
    /// u = dot(vec4(u, v, 0.0, 1.0), gTexMat[0].xyzw);
    /// v = dot(vec4(u, v, 0.0, 1.0), gTexMat[1].xyzw);
    /// ```
    Matrix([BufferDependency; 4]),

    /// Masked parallax mapping with `mix(mask_a, mask_b, ratio)` as the intensity.
    Parallax {
        mask_a: Dependency,
        mask_b: Dependency,
        ratio: BufferDependency,
    },
}

/// A single input attribute like `in_attr0.x` in GLSL.
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct AttributeDependency {
    pub name: SmolStr,
    pub channel: Option<char>,
}

// TODO: rename to operation with a, b, c?
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum LayerBlendMode {
    /// `mix(a, b, ratio)`
    Mix,
    /// `mix(a, a * b, ratio)`
    Mul,
    /// `a + b * ratio`
    Add,
    /// Normal blend mode similar to "Reoriented Normal Mapping" (RNM).
    AddNormal,
    /// `mix(a, overlay(a, b), ratio)`.
    Overlay2,
    /// `mix(a, overlay(a, b), ratio)`.
    Overlay,
    /// `pow(a, b)`
    Power,
    /// `min(a, b)`
    Min,
    /// `max(a, b)`
    Max,
    /// `clamp(a, b, ratio)`
    Clamp,
}

impl Default for LayerBlendMode {
    fn default() -> Self {
        Self::Mix
    }
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct Layer {
    pub value: LayerValue,
    pub ratio: LayerValue,
    pub blend_mode: LayerBlendMode,
    pub is_fresnel: bool,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum LayerValue {
    Value(Dependency),
    Layers(Vec<Layer>),
}

impl ShaderProgram {
    /// Returns the textures assigned to the output or `None` if the output does not use any texture.
    pub fn textures(&self, output_index: usize, channel: char) -> Vec<&TextureDependency> {
        let output = format!("o{output_index}.{channel}");

        self.output_dependencies
            .get(&SmolStr::from(output))
            .map(|d| d.dependencies.as_slice())
            .unwrap_or_default()
            .iter()
            .filter_map(|d| match d {
                Dependency::Texture(t) => Some(t),
                _ => None,
            })
            .collect()
    }

    /// Returns the float constant assigned directly to the output
    /// or `None` if the output does not use a constant.
    pub fn float_constant(&self, output_index: usize, channel: char) -> Option<f32> {
        let output = format!("o{output_index}.{channel}");

        // If a constant is assigned, it will be the only dependency.
        match self
            .output_dependencies
            .get(&SmolStr::from(output))?
            .dependencies
            .first()?
        {
            Dependency::Constant(f) => Some(f.0),
            _ => None,
        }
    }

    /// Returns the uniform buffer parameter assigned directly to the output
    /// or `None` if the output does not use a parameter.
    pub fn buffer_parameter(
        &self,
        output_index: usize,
        channel: char,
    ) -> Option<&BufferDependency> {
        let output = format!("o{output_index}.{channel}");

        // If a parameter is assigned, it will likely be the only dependency.
        match self
            .output_dependencies
            .get(&SmolStr::from(output))?
            .dependencies
            .first()?
        {
            Dependency::Buffer(b) => Some(b),
            _ => None,
        }
    }

    /// Returns the attribute assigned to the output
    /// or `None` if the output does not use an attribute.
    pub fn attribute(&self, output_index: usize, channel: char) -> Option<&AttributeDependency> {
        let output = format!("o{output_index}.{channel}");

        // If an attribute is assigned, it will likely be the only dependency.
        match self
            .output_dependencies
            .get(&SmolStr::from(output))?
            .dependencies
            .first()?
        {
            Dependency::Attribute(b) => Some(b),
            _ => None,
        }
    }
}

#[cfg(feature = "arbitrary")]
impl<'a> arbitrary::Arbitrary<'a> for AttributeDependency {
    fn arbitrary(u: &mut arbitrary::Unstructured) -> arbitrary::Result<Self> {
        Ok(Self {
            name: crate::arbitrary_smolstr(u)?,
            channel: u.arbitrary()?,
        })
    }
}

#[cfg(feature = "arbitrary")]
impl<'a> arbitrary::Arbitrary<'a> for BufferDependency {
    fn arbitrary(u: &mut arbitrary::Unstructured) -> arbitrary::Result<Self> {
        Ok(Self {
            name: crate::arbitrary_smolstr(u)?,
            field: crate::arbitrary_smolstr(u)?,
            index: u.arbitrary()?,
            channel: u.arbitrary()?,
        })
    }
}

#[cfg(feature = "arbitrary")]
impl<'a> arbitrary::Arbitrary<'a> for TextureDependency {
    fn arbitrary(u: &mut arbitrary::Unstructured) -> arbitrary::Result<Self> {
        Ok(Self {
            name: crate::arbitrary_smolstr(u)?,
            channel: u.arbitrary()?,
            texcoords: u.arbitrary()?,
        })
    }
}

#[cfg(feature = "arbitrary")]
impl<'a> arbitrary::Arbitrary<'a> for TexCoord {
    fn arbitrary(u: &mut arbitrary::Unstructured) -> arbitrary::Result<Self> {
        Ok(Self {
            name: crate::arbitrary_smolstr(u)?,
            channel: u.arbitrary()?,
            params: u.arbitrary()?,
        })
    }
}

#[cfg(feature = "arbitrary")]
impl<'a> arbitrary::Arbitrary<'a> for ShaderProgram {
    fn arbitrary(u: &mut arbitrary::Unstructured) -> arbitrary::Result<Self> {
        let output_dependencies: Vec<(String, OutputDependencies)> = u.arbitrary()?;
        Ok(Self {
            output_dependencies: output_dependencies
                .into_iter()
                .map(|(k, v)| (k.into(), v))
                .collect(),
            outline_width: u.arbitrary()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn material_channel_assignment_empty() {
        let shader = ShaderProgram {
            output_dependencies: IndexMap::new(),
            outline_width: None,
        };
        assert!(shader.textures(0, 'x').is_empty());
    }

    #[test]
    fn material_channel_assignment_single_output_no_assignment() {
        let shader = ShaderProgram {
            output_dependencies: [(
                "o0.x".into(),
                OutputDependencies {
                    dependencies: Vec::new(),
                    layers: Vec::new(),
                },
            )]
            .into(),
            outline_width: None,
        };
        assert!(shader.textures(0, 'x').is_empty());
    }

    #[test]
    fn material_channel_assignment_multiple_output_assignment() {
        let shader = ShaderProgram {
            output_dependencies: [
                (
                    "o0.x".into(),
                    OutputDependencies {
                        dependencies: vec![Dependency::Texture(TextureDependency {
                            name: "s0".into(),
                            channel: Some('y'),
                            texcoords: Vec::new(),
                        })],
                        layers: Vec::new(),
                    },
                ),
                (
                    "o0.y".into(),
                    OutputDependencies {
                        dependencies: vec![
                            Dependency::Texture(TextureDependency {
                                name: "tex".into(),
                                channel: Some('y'),
                                texcoords: Vec::new(),
                            }),
                            Dependency::Texture(TextureDependency {
                                name: "s2".into(),
                                channel: Some('z'),
                                texcoords: Vec::new(),
                            }),
                        ],
                        layers: Vec::new(),
                    },
                ),
                (
                    "o1.x".into(),
                    OutputDependencies {
                        dependencies: vec![Dependency::Texture(TextureDependency {
                            name: "s3".into(),
                            channel: Some('y'),
                            texcoords: Vec::new(),
                        })],
                        layers: Vec::new(),
                    },
                ),
            ]
            .into(),
            outline_width: None,
        };
        assert_eq!(
            vec![
                &TextureDependency {
                    name: "tex".into(),
                    channel: Some('y'),
                    texcoords: Vec::new()
                },
                &TextureDependency {
                    name: "s2".into(),
                    channel: Some('z'),
                    texcoords: Vec::new()
                },
            ],
            shader.textures(0, 'y')
        );
    }

    #[test]
    fn float_constant_multiple_assigments() {
        let shader = ShaderProgram {
            output_dependencies: [
                (
                    "o0.x".into(),
                    OutputDependencies {
                        dependencies: vec![Dependency::Texture(TextureDependency {
                            name: "s0".into(),
                            channel: Some('y'),
                            texcoords: Vec::new(),
                        })],
                        layers: Vec::new(),
                    },
                ),
                (
                    "o0.y".into(),
                    OutputDependencies {
                        dependencies: vec![
                            Dependency::Texture(TextureDependency {
                                name: "tex".into(),
                                channel: Some('y'),
                                texcoords: Vec::new(),
                            }),
                            Dependency::Texture(TextureDependency {
                                name: "s2".into(),
                                channel: Some('z'),
                                texcoords: Vec::new(),
                            }),
                        ],
                        layers: Vec::new(),
                    },
                ),
                (
                    "o1.z".into(),
                    OutputDependencies {
                        dependencies: vec![Dependency::Constant(0.5.into())],
                        layers: Vec::new(),
                    },
                ),
            ]
            .into(),
            outline_width: None,
        };
        assert_eq!(None, shader.float_constant(0, 'x'));
        assert_eq!(Some(0.5), shader.float_constant(1, 'z'));
    }

    #[test]
    fn buffer_parameter_multiple_assigments() {
        let shader = ShaderProgram {
            output_dependencies: [
                (
                    "o0.x".into(),
                    OutputDependencies {
                        dependencies: vec![Dependency::Texture(TextureDependency {
                            name: "s0".into(),
                            channel: Some('y'),
                            texcoords: Vec::new(),
                        })],
                        layers: Vec::new(),
                    },
                ),
                (
                    "o0.y".into(),
                    OutputDependencies {
                        dependencies: vec![
                            Dependency::Texture(TextureDependency {
                                name: "tex".into(),
                                channel: Some('y'),
                                texcoords: Vec::new(),
                            }),
                            Dependency::Texture(TextureDependency {
                                name: "s2".into(),
                                channel: Some('z'),
                                texcoords: Vec::new(),
                            }),
                        ],
                        layers: Vec::new(),
                    },
                ),
                (
                    "o1.z".into(),
                    OutputDependencies {
                        dependencies: vec![Dependency::Buffer(BufferDependency {
                            name: "U_Mate".into(),
                            field: "param".into(),
                            index: Some(31),
                            channel: Some('w'),
                        })],
                        layers: Vec::new(),
                    },
                ),
            ]
            .into(),
            outline_width: None,
        };
        assert_eq!(None, shader.buffer_parameter(0, 'x'));
        assert_eq!(
            Some(&BufferDependency {
                name: "U_Mate".into(),
                field: "param".into(),
                index: Some(31),
                channel: Some('w')
            }),
            shader.buffer_parameter(1, 'z')
        );
    }
}
