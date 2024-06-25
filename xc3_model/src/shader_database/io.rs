use crate::IndexMapExt;
use indexmap::IndexMap;
use ordered_float::OrderedFloat;
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;

use super::{
    AttributeDependency, BufferDependency, Dependency, Map, Shader, ShaderDatabase, ShaderProgram,
    Spch, TexCoord, TextureDependency,
};

// Create a separate smaller representation for on disk.
#[derive(Debug, Serialize, Deserialize)]
pub struct ShaderDatabaseIndexed {
    files: IndexMap<SmolStr, SpchIndexed>,
    map_files: IndexMap<SmolStr, MapIndexed>,
    dependencies: Vec<DependencyIndexed>,
    buffer_dependencies: Vec<BufferDependency>,
    outputs: Vec<SmolStr>,
}

#[derive(Debug, Serialize, Deserialize)]
struct MapIndexed {
    map_models: Vec<SpchIndexed>,
    prop_models: Vec<SpchIndexed>,
    env_models: Vec<SpchIndexed>,
}

// TODO: rename to ShaderPrograms.
#[derive(Debug, Serialize, Deserialize)]
#[serde(transparent)]
struct SpchIndexed {
    programs: Vec<ShaderProgramIndexed>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(transparent)]
struct ShaderProgramIndexed {
    shaders: Vec<ShaderIndexed>,
}

// TODO: How to reduce size of buffer parameters for texture coordinates?
#[derive(Debug, Serialize, Deserialize)]
#[serde(transparent)]
struct ShaderIndexed {
    // There are very few unique dependencies across all shaders in a game dump.
    // Normalize the data to greatly reduce the size of the JSON representation.
    output_dependencies: IndexMap<usize, Vec<usize>>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
enum DependencyIndexed {
    Constant(OrderedFloat<f32>),
    Buffer(usize),
    Texture(SmolStr, SmolStr, Vec<TexCoordIndexed>),
    Attribute(SmolStr, SmolStr),
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
struct TexCoordIndexed(SmolStr, SmolStr, Vec<usize>);

// Take the disk representation by value to reduce clones.
impl From<ShaderDatabaseIndexed> for ShaderDatabase {
    fn from(value: ShaderDatabaseIndexed) -> Self {
        let dependencies: Vec<_> = value
            .dependencies
            .into_iter()
            .map(|d| match d {
                DependencyIndexed::Constant(f) => Dependency::Constant(f),
                DependencyIndexed::Buffer(i) => {
                    Dependency::Buffer(value.buffer_dependencies[i].clone())
                }
                DependencyIndexed::Texture(name, channels, texcoords) => {
                    Dependency::Texture(TextureDependency {
                        name,
                        channels,
                        texcoords: texcoords
                            .into_iter()
                            .map(|TexCoordIndexed(name, channels, params)| TexCoord {
                                name,
                                channels,
                                params: params
                                    .into_iter()
                                    .map(|i| value.buffer_dependencies[i].clone())
                                    .collect(),
                            })
                            .collect(),
                    })
                }
                DependencyIndexed::Attribute(name, channels) => {
                    Dependency::Attribute(AttributeDependency { name, channels })
                }
            })
            .collect();

        Self {
            files: value
                .files
                .into_iter()
                .map(|(n, s)| (n, spch_from_indexed(s, &dependencies, &value.outputs)))
                .collect(),
            map_files: value
                .map_files
                .into_iter()
                .map(|(n, m)| {
                    (
                        n,
                        Map {
                            map_models: m
                                .map_models
                                .into_iter()
                                .map(|s| spch_from_indexed(s, &dependencies, &value.outputs))
                                .collect(),
                            prop_models: m
                                .prop_models
                                .into_iter()
                                .map(|s| spch_from_indexed(s, &dependencies, &value.outputs))
                                .collect(),
                            env_models: m
                                .env_models
                                .into_iter()
                                .map(|s| spch_from_indexed(s, &dependencies, &value.outputs))
                                .collect(),
                        },
                    )
                })
                .collect(),
        }
    }
}

impl From<&ShaderDatabase> for ShaderDatabaseIndexed {
    fn from(value: &ShaderDatabase) -> Self {
        let mut dependency_to_index = IndexMap::new();
        let mut output_to_index = IndexMap::new();
        let mut buffer_dependency_to_index = IndexMap::new();

        Self {
            files: value
                .files
                .iter()
                .map(|(n, s)| {
                    (
                        n.clone(),
                        spch_indexed(s, &mut dependency_to_index, &mut output_to_index),
                    )
                })
                .collect(),
            map_files: value
                .map_files
                .iter()
                .map(|(n, m)| {
                    (
                        n.clone(),
                        MapIndexed {
                            map_models: m
                                .map_models
                                .iter()
                                .map(|s| {
                                    spch_indexed(s, &mut dependency_to_index, &mut output_to_index)
                                })
                                .collect(),
                            prop_models: m
                                .prop_models
                                .iter()
                                .map(|s| {
                                    spch_indexed(s, &mut dependency_to_index, &mut output_to_index)
                                })
                                .collect(),
                            env_models: m
                                .env_models
                                .iter()
                                .map(|s| {
                                    spch_indexed(s, &mut dependency_to_index, &mut output_to_index)
                                })
                                .collect(),
                        },
                    )
                })
                .collect(),
            dependencies: dependency_to_index
                .into_keys()
                .map(|d| match d {
                    Dependency::Constant(c) => DependencyIndexed::Constant(c),
                    Dependency::Buffer(b) => {
                        DependencyIndexed::Buffer(buffer_dependency_to_index.entry_index(b))
                    }
                    Dependency::Texture(t) => DependencyIndexed::Texture(
                        t.name,
                        t.channels,
                        t.texcoords
                            .into_iter()
                            .map(|t| {
                                TexCoordIndexed(
                                    t.name,
                                    t.channels,
                                    t.params
                                        .into_iter()
                                        .map(|p| buffer_dependency_to_index.entry_index(p))
                                        .collect(),
                                )
                            })
                            .collect(),
                    ),
                    Dependency::Attribute(a) => DependencyIndexed::Attribute(a.name, a.channels),
                })
                .collect(),
            buffer_dependencies: buffer_dependency_to_index.into_keys().collect(),
            outputs: output_to_index.into_keys().collect(),
        }
    }
}

fn spch_indexed(
    spch: &Spch,
    dependency_to_index: &mut IndexMap<Dependency, usize>,
    output_to_index: &mut IndexMap<SmolStr, usize>,
) -> SpchIndexed {
    SpchIndexed {
        programs: spch
            .programs
            .iter()
            .map(|p| ShaderProgramIndexed {
                shaders: p
                    .shaders
                    .iter()
                    .map(|s| ShaderIndexed {
                        output_dependencies: s
                            .output_dependencies
                            .iter()
                            .map(|(output, dependencies)| {
                                // This works since the map preserves insertion order.
                                let output_index = output_to_index.entry_index(output.clone());
                                (
                                    output_index,
                                    dependencies
                                        .iter()
                                        .map(|d| dependency_to_index.entry_index(d.clone()))
                                        .collect(),
                                )
                            })
                            .collect(),
                    })
                    .collect(),
            })
            .collect(),
    }
}

fn spch_from_indexed(spch: SpchIndexed, dependencies: &[Dependency], outputs: &[SmolStr]) -> Spch {
    Spch {
        programs: spch
            .programs
            .into_iter()
            .map(|p| ShaderProgram {
                shaders: p
                    .shaders
                    .into_iter()
                    .map(|s| Shader {
                        output_dependencies: s
                            .output_dependencies
                            .into_iter()
                            .map(|(output, output_dependencies)| {
                                (
                                    outputs[output].clone(),
                                    output_dependencies
                                        .into_iter()
                                        .map(|d| dependencies[d].clone())
                                        .collect(),
                                )
                            })
                            .collect(),
                    })
                    .collect(),
            })
            .collect(),
    }
}
