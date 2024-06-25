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

/// Metadata for the assigned [Shader] for all models and maps in a game dump.
#[derive(Debug, PartialEq, Clone)]
pub struct ShaderDatabase {
    /// The `.wimdo` file name without the extension and shader data for each file.
    pub files: IndexMap<SmolStr, Spch>,
    /// The `.wismhd` file name without the extension and shader data for each map.
    pub map_files: IndexMap<SmolStr, Map>,
}

impl ShaderDatabase {
    /// Loads and deserializes the JSON data from `path`.
    ///
    /// This uses a modified JSON representation internally to reduce file size.
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, LoadShaderDatabaseError> {
        let json = std::fs::read_to_string(path)?;
        let indexed: io::ShaderDatabaseIndexed = serde_json::from_str(&json)?;
        Ok(indexed.into())
    }

    /// Serialize and save the JSON data from `path`.
    ///
    /// This uses a modified JSON representation internally to reduce file size.
    pub fn save<P: AsRef<Path>>(
        &self,
        path: P,
        pretty_print: bool,
    ) -> Result<(), SaveShaderDatabaseError> {
        let indexed = io::ShaderDatabaseIndexed::from(self);
        let json = if pretty_print {
            serde_json::to_string_pretty(&indexed)?
        } else {
            serde_json::to_string(&indexed)?
        };
        std::fs::write(path, json)?;
        Ok(())
    }
}

/// Shaders for the different map model types.
#[derive(Debug, PartialEq, Clone)]
pub struct Map {
    pub map_models: Vec<Spch>,
    pub prop_models: Vec<Spch>,
    pub env_models: Vec<Spch>,
}

/// The decompiled shader data for a single shader container file.
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Clone)]
pub struct Spch {
    pub programs: Vec<ShaderProgram>,
}

/// A collection of shaders.
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Clone)]
pub struct ShaderProgram {
    /// Some shaders have multiple NVSD sections, so the length may be greater than 1.
    pub shaders: Vec<Shader>,
}

// TODO: Document how to try sampler, constant, parameter in order.
/// The buffer elements, textures, and constants used to initialize each fragment output.
///
/// This assumes inputs are assigned directly to outputs without any modifications.
/// Fragment shaders typically only perform basic input and channel selection in practice.
///
/// This assignment information is needed to accurately recreate the G-Buffer texture values.
/// Renderers can generate unique shaders for each model
/// or select inputs in a shared shader at render time like xc3_wgpu.
/// Node based editors like Blender's shader editor should use these values
/// to determine how to construct node groups.
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Clone)]
pub struct Shader {
    /// A list of input dependencies like "s0.xyz" assigned to each output like "out_attr0.x".
    ///
    /// Each dependency can be thought of as a link
    /// between the dependency node and group output in a shader node graph.
    pub output_dependencies: IndexMap<SmolStr, Vec<Dependency>>,
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
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Serialize, Deserialize)]
pub struct BufferDependency {
    pub name: SmolStr,
    pub field: SmolStr,
    pub index: usize,
    pub channels: SmolStr,
}

/// A single texture access like `texture(s0, tex0.xy).rgb` in GLSL.
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct TextureDependency {
    pub name: SmolStr,
    pub channels: SmolStr,
    /// Texture coordinate values used for the texture function call.
    pub texcoords: Vec<TexCoord>,
}

/// A texture coordinate attribute with optional transform parameters.
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct TexCoord {
    /// The name of the attribute like "in_attr4".
    pub name: SmolStr,
    /// The accessed channels like "x" or "y".
    pub channels: SmolStr,
    /// These can generally be assumed to be scale or matrix transforms.
    pub params: Vec<BufferDependency>,
}

/// A single input attribute like `in_attr0.x` in GLSL.
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct AttributeDependency {
    pub name: SmolStr,
    pub channels: SmolStr,
}

impl Shader {
    /// Returns the textures assigned to the output or `None` if the output does not use any texture.
    ///
    /// This currently uses a heuristic where textures like "s0" are returned before "s4" or "gTResidentTex05"
    /// to resolve some assignment issues.
    pub fn textures(&self, output_index: usize, channel: char) -> Vec<&TextureDependency> {
        let output = format!("o{output_index}.{channel}");

        // Find the first material referenced samplers like "s0" or "s1".
        let mut textures: Vec<_> = self
            .output_dependencies
            .get(&SmolStr::from(output))
            .map(|d| d.as_slice())
            .unwrap_or_default()
            .iter()
            .filter_map(|d| match d {
                Dependency::Texture(t) => Some(t),
                _ => None,
            })
            .collect();

        // TODO: Is there a better heuristic than always picking the lowest sampler index?
        textures
            .sort_by(|a, b| material_sampler_index(&a.name).cmp(&material_sampler_index(&b.name)));
        textures
    }

    /// Returns the float constant assigned directly to the output
    /// or `None` if the output does not use a constant.
    pub fn float_constant(&self, output_index: usize, channel: char) -> Option<f32> {
        let output = format!("o{output_index}.{channel}");

        // If a constant is assigned, it will be the only dependency.
        match self
            .output_dependencies
            .get(&SmolStr::from(output))?
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
            .first()?
        {
            Dependency::Attribute(b) => Some(b),
            _ => None,
        }
    }
}

fn material_sampler_index(sampler: &str) -> usize {
    // TODO: Just parse int?
    match sampler {
        "s0" => 0,
        "s1" => 1,
        "s2" => 2,
        "s3" => 3,
        "s4" => 4,
        "s5" => 5,
        "s6" => 6,
        "s7" => 7,
        "s8" => 8,
        "s9" => 9,
        // TODO: How to handle this case?
        _ => usize::MAX,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn material_channel_assignment_empty() {
        let shader = Shader {
            output_dependencies: IndexMap::new(),
        };
        assert!(shader.textures(0, 'x').is_empty());
    }

    #[test]
    fn material_channel_assignment_single_output_no_assignment() {
        let shader = Shader {
            output_dependencies: [("o0.x".into(), Vec::new())].into(),
        };
        assert!(shader.textures(0, 'x').is_empty());
    }

    #[test]
    fn material_channel_assignment_multiple_output_assignment() {
        let shader = Shader {
            output_dependencies: [
                (
                    "o0.x".into(),
                    vec![Dependency::Texture(TextureDependency {
                        name: "s0".into(),
                        channels: "y".into(),
                        texcoords: Vec::new(),
                    })],
                ),
                (
                    "o0.y".into(),
                    vec![
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
                ),
                (
                    "o1.x".into(),
                    vec![Dependency::Texture(TextureDependency {
                        name: "s3".into(),
                        channels: "xyz".into(),
                        texcoords: Vec::new(),
                    })],
                ),
            ]
            .into(),
        };
        assert_eq!(
            vec![
                &TextureDependency {
                    name: "s2".into(),
                    channels: "z".into(),
                    texcoords: Vec::new()
                },
                &TextureDependency {
                    name: "tex".into(),
                    channels: "xyz".into(),
                    texcoords: Vec::new()
                }
            ],
            shader.textures(0, 'y')
        );
    }

    #[test]
    fn float_constant_multiple_assigments() {
        let shader = Shader {
            output_dependencies: [
                (
                    "o0.x".into(),
                    vec![Dependency::Texture(TextureDependency {
                        name: "s0".into(),
                        channels: "y".into(),
                        texcoords: Vec::new(),
                    })],
                ),
                (
                    "o0.y".into(),
                    vec![
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
                ),
                ("o1.z".into(), vec![Dependency::Constant(0.5.into())]),
            ]
            .into(),
        };
        assert_eq!(None, shader.float_constant(0, 'x'));
        assert_eq!(Some(0.5), shader.float_constant(1, 'z'));
    }

    #[test]
    fn buffer_parameter_multiple_assigments() {
        let shader = Shader {
            output_dependencies: [
                (
                    "o0.x".into(),
                    vec![Dependency::Texture(TextureDependency {
                        name: "s0".into(),
                        channels: "y".into(),
                        texcoords: Vec::new(),
                    })],
                ),
                (
                    "o0.y".into(),
                    vec![
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
                ),
                (
                    "o1.z".into(),
                    vec![Dependency::Buffer(BufferDependency {
                        name: "U_Mate".into(),
                        field: "param".into(),
                        index: 31,
                        channels: "w".into(),
                    })],
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
