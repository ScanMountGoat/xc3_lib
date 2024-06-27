use crate::IndexMapExt;
use indexmap::IndexMap;
use ordered_float::OrderedFloat;
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;

use super::{
    AttributeDependency, BufferDependency, Dependency, MapPrograms, ModelPrograms, ShaderDatabase,
    ShaderProgram, TexCoord, TextureDependency,
};

// Create a separate smaller representation for on disk.
#[derive(Debug, Serialize, Deserialize)]
pub struct ShaderDatabaseIndexed {
    files: IndexMap<SmolStr, ModelIndexed>,
    map_files: IndexMap<SmolStr, MapIndexed>,
    dependencies: Vec<DependencyIndexed>,
    buffer_dependencies: Vec<BufferDependency>,
    outputs: Vec<SmolStr>,
}

#[derive(Debug, Serialize, Deserialize)]
struct MapIndexed {
    map_models: Vec<ModelIndexed>,
    prop_models: Vec<ModelIndexed>,
    env_models: Vec<ModelIndexed>,
}

// TODO: rename to ShaderPrograms.
#[derive(Debug, Serialize, Deserialize)]
#[serde(transparent)]
struct ModelIndexed {
    programs: Vec<ShaderProgramIndexed>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(transparent)]
struct ShaderProgramIndexed {
    // There are very few unique dependencies across all shaders in a game dump.
    // Normalize the data to greatly reduce the size of the JSON representation.
    output_dependencies: IndexMap<usize, Vec<usize>>,
}

// TODO: Also index texture and texcoord names?
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
        dbg!(value.dependencies.len(), value.buffer_dependencies.len());
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
                .map(|(n, s)| (n, model_from_indexed(s, &dependencies, &value.outputs)))
                .collect(),
            map_files: value
                .map_files
                .into_iter()
                .map(|(n, m)| {
                    (
                        n,
                        MapPrograms {
                            map_models: m
                                .map_models
                                .into_iter()
                                .map(|s| model_from_indexed(s, &dependencies, &value.outputs))
                                .collect(),
                            prop_models: m
                                .prop_models
                                .into_iter()
                                .map(|s| model_from_indexed(s, &dependencies, &value.outputs))
                                .collect(),
                            env_models: m
                                .env_models
                                .into_iter()
                                .map(|s| model_from_indexed(s, &dependencies, &value.outputs))
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
                        model_indexed(s, &mut dependency_to_index, &mut output_to_index),
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
                                    model_indexed(s, &mut dependency_to_index, &mut output_to_index)
                                })
                                .collect(),
                            prop_models: m
                                .prop_models
                                .iter()
                                .map(|s| {
                                    model_indexed(s, &mut dependency_to_index, &mut output_to_index)
                                })
                                .collect(),
                            env_models: m
                                .env_models
                                .iter()
                                .map(|s| {
                                    model_indexed(s, &mut dependency_to_index, &mut output_to_index)
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

fn model_indexed(
    model: &ModelPrograms,
    dependency_to_index: &mut IndexMap<Dependency, usize>,
    output_to_index: &mut IndexMap<SmolStr, usize>,
) -> ModelIndexed {
    ModelIndexed {
        programs: model
            .programs
            .iter()
            .map(|p| ShaderProgramIndexed {
                output_dependencies: p
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
    }
}

fn model_from_indexed(
    model: ModelIndexed,
    dependencies: &[Dependency],
    outputs: &[SmolStr],
) -> ModelPrograms {
    ModelPrograms {
        programs: model
            .programs
            .into_iter()
            .map(|p| ShaderProgram {
                output_dependencies: p
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
    }
}
