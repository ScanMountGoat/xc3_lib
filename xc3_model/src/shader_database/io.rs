use crate::IndexMapExt;
use indexmap::IndexMap;
use ordered_float::OrderedFloat;
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;

use super::{
    AttributeDependency, BufferDependency, Dependency, MapPrograms, ModelPrograms, ShaderProgram,
    TexCoord, TexCoordParams, TextureDependency, TextureLayer,
};

// Create a separate smaller representation for on disk.
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct ShaderDatabaseIndexed {
    files: IndexMap<SmolStr, ModelIndexed>,
    map_files: IndexMap<SmolStr, MapIndexed>,
    dependencies: Vec<DependencyIndexed>,
    buffer_dependencies: Vec<BufferDependency>,
    outputs: Vec<SmolStr>,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
struct MapIndexed {
    map_models: Vec<ModelIndexed>,
    prop_models: Vec<ModelIndexed>,
    env_models: Vec<ModelIndexed>,
}

// TODO: rename to ShaderPrograms.
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
#[serde(transparent)]
struct ModelIndexed {
    programs: Vec<ShaderProgramIndexed>,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
struct ShaderProgramIndexed {
    // There are very few unique dependencies across all shaders in a game dump.
    // Normalize the data to greatly reduce the size of the JSON representation.
    output_dependencies: IndexMap<usize, Vec<usize>>,

    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default)]
    normal_layers: Vec<TextureLayerIndexed>,
}

// TODO: Also index texture and texcoord names?
#[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
enum DependencyIndexed {
    Constant(OrderedFloat<f32>),
    Buffer(usize),
    Texture(SmolStr, SmolStr, Vec<TexCoordIndexed>),
    Attribute(SmolStr, SmolStr),
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
struct TexCoordIndexed(SmolStr, SmolStr, Option<TexCoordParamsIndexed>);

#[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
pub enum TexCoordParamsIndexed {
    Scale(usize),
    Matrix([usize; 4]),
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
struct TextureLayerIndexed {
    name: String,
    channel: Option<char>,
    ratio: Option<usize>,
}

impl ShaderDatabaseIndexed {
    pub fn model(&self, name: &str) -> Option<ModelPrograms> {
        self.files.get(&SmolStr::from(name)).map(|model| {
            model_from_indexed(
                model,
                &self.dependencies,
                &self.buffer_dependencies,
                &self.outputs,
            )
        })
    }

    pub fn map(&self, name: &str) -> Option<MapPrograms> {
        self.map_files
            .get(&SmolStr::from(name))
            .map(|map| MapPrograms {
                map_models: map
                    .map_models
                    .iter()
                    .map(|s| {
                        model_from_indexed(
                            s,
                            &self.dependencies,
                            &self.buffer_dependencies,
                            &self.outputs,
                        )
                    })
                    .collect(),
                prop_models: map
                    .prop_models
                    .iter()
                    .map(|s| {
                        model_from_indexed(
                            s,
                            &self.dependencies,
                            &self.buffer_dependencies,
                            &self.outputs,
                        )
                    })
                    .collect(),
                env_models: map
                    .env_models
                    .iter()
                    .map(|s| {
                        model_from_indexed(
                            s,
                            &self.dependencies,
                            &self.buffer_dependencies,
                            &self.outputs,
                        )
                    })
                    .collect(),
            })
    }

    pub fn from_models_maps(
        models: IndexMap<String, ModelPrograms>,
        maps: IndexMap<String, MapPrograms>,
    ) -> Self {
        let mut dependency_to_index = IndexMap::new();
        let mut output_to_index = IndexMap::new();
        let mut buffer_dependency_to_index = IndexMap::new();

        Self {
            files: models
                .into_iter()
                .map(|(n, s)| {
                    (
                        n.into(),
                        model_indexed(s, &mut dependency_to_index, &mut output_to_index),
                    )
                })
                .collect(),
            map_files: maps
                .into_iter()
                .map(|(n, m)| {
                    (
                        n.into(),
                        MapIndexed {
                            map_models: m
                                .map_models
                                .into_iter()
                                .map(|s| {
                                    model_indexed(s, &mut dependency_to_index, &mut output_to_index)
                                })
                                .collect(),
                            prop_models: m
                                .prop_models
                                .into_iter()
                                .map(|s| {
                                    model_indexed(s, &mut dependency_to_index, &mut output_to_index)
                                })
                                .collect(),
                            env_models: m
                                .env_models
                                .into_iter()
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
                                    t.params.map(|params| match params {
                                        TexCoordParams::Scale(s) => TexCoordParamsIndexed::Scale(
                                            buffer_dependency_to_index.entry_index(s),
                                        ),
                                        TexCoordParams::Matrix(m) => TexCoordParamsIndexed::Matrix(
                                            m.map(|v| buffer_dependency_to_index.entry_index(v)),
                                        ),
                                    }),
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

fn dependency_from_indexed(
    d: DependencyIndexed,
    buffer_dependencies: &[BufferDependency],
) -> Dependency {
    match d {
        DependencyIndexed::Constant(f) => Dependency::Constant(f),
        DependencyIndexed::Buffer(i) => Dependency::Buffer(buffer_dependencies[i].clone()),
        DependencyIndexed::Texture(name, channels, texcoords) => {
            Dependency::Texture(TextureDependency {
                name,
                channels,
                texcoords: texcoords
                    .into_iter()
                    .map(|TexCoordIndexed(name, channels, params)| TexCoord {
                        name,
                        channels,
                        params: params.map(|params| match params {
                            TexCoordParamsIndexed::Scale(s) => {
                                TexCoordParams::Scale(buffer_dependencies[s].clone())
                            }
                            TexCoordParamsIndexed::Matrix(m) => {
                                TexCoordParams::Matrix(m.map(|v| buffer_dependencies[v].clone()))
                            }
                        }),
                    })
                    .collect(),
            })
        }
        DependencyIndexed::Attribute(name, channels) => {
            Dependency::Attribute(AttributeDependency { name, channels })
        }
    }
}

fn model_indexed(
    model: ModelPrograms,
    dependency_to_index: &mut IndexMap<Dependency, usize>,
    output_to_index: &mut IndexMap<SmolStr, usize>,
) -> ModelIndexed {
    ModelIndexed {
        programs: model
            .programs
            .into_iter()
            .map(|p| ShaderProgramIndexed {
                output_dependencies: p
                    .output_dependencies
                    .into_iter()
                    .map(|(output, dependencies)| {
                        // This works since the map preserves insertion order.
                        let output_index = output_to_index.entry_index(output);
                        (
                            output_index,
                            dependencies
                                .into_iter()
                                .map(|d| dependency_to_index.entry_index(d))
                                .collect(),
                        )
                    })
                    .collect(),
                normal_layers: p
                    .normal_layers
                    .into_iter()
                    .map(|l| TextureLayerIndexed {
                        name: l.name,
                        channel: l.channel,
                        ratio: l.ratio.map(|r| dependency_to_index.entry_index(r)),
                    })
                    .collect(),
            })
            .collect(),
    }
}

fn model_from_indexed(
    model: &ModelIndexed,
    dependencies: &[DependencyIndexed],
    buffer_dependencies: &[BufferDependency],
    outputs: &[SmolStr],
) -> ModelPrograms {
    ModelPrograms {
        programs: model
            .programs
            .iter()
            .map(|p| ShaderProgram {
                output_dependencies: p
                    .output_dependencies
                    .iter()
                    .map(|(output, output_dependencies)| {
                        (
                            outputs[*output].clone(),
                            output_dependencies
                                .iter()
                                .map(|d| {
                                    dependency_from_indexed(
                                        dependencies[*d].clone(),
                                        buffer_dependencies,
                                    )
                                })
                                .collect(),
                        )
                    })
                    .collect(),
                normal_layers: p
                    .normal_layers
                    .iter()
                    .map(|l| TextureLayer {
                        name: l.name.clone(),
                        channel: l.channel,
                        ratio: l.ratio.map(|i| {
                            dependency_from_indexed(dependencies[i].clone(), buffer_dependencies)
                        }),
                    })
                    .collect(),
            })
            .collect(),
    }
}
