//! Database for compiled shader metadata for more accurate rendering.
//!
//! In game shaders are precompiled and embedded in files like `.wismt`.
//! These types represent precomputed metadata like assignments to G-Buffer textures.
//! This is necessary for determining the usage of a texture like albedo or normal map
//! since the assignments are compiled into the shader code itself.
//!
//! Shader database JSON files should be generated using the xc3_shader CLI tool.
//! Applications can deserialize the JSON with [ShaderDatabase::from_file]
//! to avoid needing to generate this data at runtime.

use std::path::Path;

use indexmap::IndexMap;
use ordered_float::OrderedFloat;
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;
use thiserror::Error;

mod io;

#[derive(Debug, Error)]
pub enum LoadShaderDatabaseError {
    #[error("error reading shader JSON file")]
    Io(#[from] std::io::Error),

    #[error("error deserializing shader JSON")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Error)]
pub enum SaveShaderDatabaseError {
    #[error("error writing shader JSON file")]
    Io(#[from] std::io::Error),

    #[error("error serializing shader JSON")]
    Json(#[from] serde_json::Error),
}

/// Metadata for the assigned shaders for all models and maps in a game dump.
#[derive(Debug, PartialEq, Clone)]
pub struct ShaderDatabase(io::ShaderDatabaseIndexed);

impl ShaderDatabase {
    /// Loads and deserializes the JSON data from `path`.
    ///
    /// This uses a modified JSON representation internally to reduce file size.
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, LoadShaderDatabaseError> {
        // Avoid converting the indexed database to improve load times.
        // Most uses cases will only need data for a single model or map.
        let json = std::fs::read_to_string(path)?;
        let indexed = serde_json::from_str(&json)?;
        Ok(Self(indexed))
    }

    /// Serialize and save the JSON data from `path`.
    ///
    /// This uses a modified JSON representation internally to reduce file size.
    pub fn save<P: AsRef<Path>>(
        &self,
        path: P,
        pretty_print: bool,
    ) -> Result<(), SaveShaderDatabaseError> {
        let json = if pretty_print {
            serde_json::to_string_pretty(&self.0)?
        } else {
            serde_json::to_string(&self.0)?
        };
        std::fs::write(path, json)?;
        Ok(())
    }

    /// The shader information for the `.wimdo` file name without the extension.
    pub fn model(&self, name: &str) -> Option<ModelPrograms> {
        self.0.model(name)
    }

    /// The shader information for the `.wismhd` file name without the extension.
    pub fn map(&self, name: &str) -> Option<MapPrograms> {
        self.0.map(name)
    }

    /// Create the internal database representation from non indexed data.
    pub fn from_models_maps(
        models: IndexMap<String, ModelPrograms>,
        maps: IndexMap<String, MapPrograms>,
    ) -> Self {
        Self(io::ShaderDatabaseIndexed::from_models_maps(models, maps))
    }
}

/// Shaders for the different map model types.
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Clone)]
pub struct MapPrograms {
    pub map_models: Vec<ModelPrograms>,
    pub prop_models: Vec<ModelPrograms>,
    pub env_models: Vec<ModelPrograms>,
}

/// The decompiled shader data for a single shader container file.
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Clone)]
pub struct ModelPrograms {
    pub programs: Vec<ShaderProgram>,
}

// TODO: Document how to try sampler, constant, parameter in order.
/// A single shader program with a vertex and fragment shader.
#[derive(Debug, PartialEq, Clone)]
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
}

#[derive(Debug, PartialEq, Clone)]
pub struct OutputDependencies {
    /// All of the possible dependencies that may affect the output.
    pub dependencies: Vec<Dependency>,
    /// Layering information if this output blends multiple texture dependencies.
    pub layers: Vec<TextureLayer>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum Dependency {
    Constant(OrderedFloat<f32>),
    Buffer(BufferDependency),
    Texture(TextureDependency),
    Attribute(AttributeDependency),
}

/// A single buffer access like `UniformBuffer.field[0].y` in GLSL.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Serialize, Deserialize)]
pub struct BufferDependency {
    pub name: SmolStr,
    pub field: SmolStr,
    pub index: usize,
    pub channels: SmolStr,
}

/// A single texture access like `texture(s0, tex0.xy).rgb` in GLSL.
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct TextureDependency {
    pub name: SmolStr,
    pub channels: SmolStr,
    /// Texture coordinate values used for the texture function call.
    pub texcoords: Vec<TexCoord>,
}

/// A texture coordinate attribute with optional transform parameters.
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct TexCoord {
    /// The name of the attribute like "in_attr4".
    pub name: SmolStr,
    /// The accessed channels like "x" or "y".
    pub channels: SmolStr,
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
}

/// A single input attribute like `in_attr0.x` in GLSL.
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct AttributeDependency {
    pub name: SmolStr,
    pub channels: SmolStr,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
pub enum LayerBlendMode {
    /// `mix(a, b, ratio)`
    Mix,
    /// `mix(a, a * b, ratio)`
    MixRatio,
    /// `a + b * ratio`
    Add,
    /// Normal blend mode similar to "Reoriented Normal Mapping" (RNM).
    AddNormal,
}

impl Default for LayerBlendMode {
    fn default() -> Self {
        Self::Mix
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct TextureLayer {
    pub value: Dependency,
    pub ratio: Option<Dependency>,
    pub blend_mode: LayerBlendMode,
    pub is_fresnel: bool,
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
        let output_dependencies: Vec<(String, Vec<Dependency>)> = u.arbitrary()?;
        Ok(Self {
            name: crate::arbitrary_smolstr(u)?,
            channels: crate::arbitrary_smolstr(u)?,
        })
    }
}

#[cfg(feature = "arbitrary")]
impl<'a> arbitrary::Arbitrary<'a> for BufferDependency {
    fn arbitrary(u: &mut arbitrary::Unstructured) -> arbitrary::Result<Self> {
        let output_dependencies: Vec<(String, Vec<Dependency>)> = u.arbitrary()?;
        Ok(Self {
            name: crate::arbitrary_smolstr(u)?,
            field: crate::arbitrary_smolstr(u)?,
            index: u.arbitrary()?,
            channels: crate::arbitrary_smolstr(u)?,
        })
    }
}

#[cfg(feature = "arbitrary")]
impl<'a> arbitrary::Arbitrary<'a> for TextureDependency {
    fn arbitrary(u: &mut arbitrary::Unstructured) -> arbitrary::Result<Self> {
        let output_dependencies: Vec<(String, Vec<Dependency>)> = u.arbitrary()?;
        Ok(Self {
            name: crate::arbitrary_smolstr(u)?,
            channels: crate::arbitrary_smolstr(u)?,
            texcoords: u.arbitrary()?,
        })
    }
}

#[cfg(feature = "arbitrary")]
impl<'a> arbitrary::Arbitrary<'a> for TexCoord {
    fn arbitrary(u: &mut arbitrary::Unstructured) -> arbitrary::Result<Self> {
        let output_dependencies: Vec<(String, Vec<Dependency>)> = u.arbitrary()?;
        Ok(Self {
            name: crate::arbitrary_smolstr(u)?,
            channels: crate::arbitrary_smolstr(u)?,
            params: u.arbitrary()?,
        })
    }
}

#[cfg(feature = "arbitrary")]
impl<'a> arbitrary::Arbitrary<'a> for ShaderProgram {
    fn arbitrary(u: &mut arbitrary::Unstructured) -> arbitrary::Result<Self> {
        let output_dependencies: Vec<(String, Vec<Dependency>)> = u.arbitrary()?;
        Ok(Self {
            output_dependencies: output_dependencies
                .into_iter()
                .map(|(k, v)| (k.into(), v))
                .collect(),
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
                            channels: "y".into(),
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
                                channels: "xyz".into(),
                                texcoords: Vec::new(),
                            }),
                            Dependency::Texture(TextureDependency {
                                name: "s2".into(),
                                channels: "z".into(),
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
                            channels: "xyz".into(),
                            texcoords: Vec::new(),
                        })],
                        layers: Vec::new(),
                    },
                ),
            ]
            .into(),
        };
        assert_eq!(
            vec![
                &TextureDependency {
                    name: "tex".into(),
                    channels: "xyz".into(),
                    texcoords: Vec::new()
                },
                &TextureDependency {
                    name: "s2".into(),
                    channels: "z".into(),
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
                            channels: "y".into(),
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
                                channels: "xyz".into(),
                                texcoords: Vec::new(),
                            }),
                            Dependency::Texture(TextureDependency {
                                name: "s2".into(),
                                channels: "z".into(),
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
                            channels: "y".into(),
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
                                channels: "xyz".into(),
                                texcoords: Vec::new(),
                            }),
                            Dependency::Texture(TextureDependency {
                                name: "s2".into(),
                                channels: "z".into(),
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
                            index: 31,
                            channels: "w".into(),
                        })],
                        layers: Vec::new(),
                    },
                ),
            ]
            .into(),
        };
        assert_eq!(None, shader.buffer_parameter(0, 'x'));
        assert_eq!(
            Some(&BufferDependency {
                name: "U_Mate".into(),
                field: "param".into(),
                index: 31,
                channels: "w".into()
            }),
            shader.buffer_parameter(1, 'z')
        );
    }
}
