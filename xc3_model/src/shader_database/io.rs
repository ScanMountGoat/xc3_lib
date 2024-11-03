use std::{io::Cursor, path::Path};

use crate::IndexMapExt;
use binrw::{binrw, BinRead, BinReaderExt, BinResult, BinWrite, BinWriterExt, NullString};
use indexmap::IndexMap;
use smol_str::{SmolStr, ToSmolStr};

use super::{
    AttributeDependency, BufferDependency, Dependency, LayerBlendMode, MapPrograms, ModelPrograms,
    OutputDependencies, ShaderProgram, TexCoord, TexCoordParams, TextureDependency, TextureLayer,
};

// TODO: Shared generic type?
#[derive(Debug, PartialEq, Clone, BinRead, BinWrite)]
struct StringIndex(u8);

impl TryFrom<usize> for StringIndex {
    type Error = <u8 as TryFrom<usize>>::Error;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        u8::try_from(value).map(Self)
    }
}

#[derive(Debug, PartialEq, Clone, BinRead, BinWrite)]
struct DependencyIndex(u16);

impl TryFrom<usize> for DependencyIndex {
    type Error = <u16 as TryFrom<usize>>::Error;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        u16::try_from(value).map(Self)
    }
}

#[derive(Debug, PartialEq, Clone)]
struct Index<T>(T);

impl<T> BinRead for Index<T>
where
    for<'a> T: BinRead<Args<'a> = ()>,
{
    type Args<'a> = ();

    fn read_options<R: std::io::Read + std::io::Seek>(
        reader: &mut R,
        endian: binrw::Endian,
        args: Self::Args<'_>,
    ) -> BinResult<Self> {
        T::read_options(reader, endian, args).map(Self)
    }
}

impl<T> BinWrite for Index<T>
where
    T: BinWrite,
{
    type Args<'a> = T::Args<'a>;

    fn write_options<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        endian: binrw::Endian,
        args: Self::Args<'_>,
    ) -> BinResult<()> {
        self.0.write_options(writer, endian, args)
    }
}

impl<T> TryFrom<usize> for Index<T>
where
    T: TryFrom<usize>,
{
    type Error = <T as TryFrom<usize>>::Error;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        T::try_from(value).map(Self)
    }
}

// Create a separate smaller representation for on disk.
#[binrw]
#[derive(Debug, PartialEq, Clone)]
#[brw(magic(b"SHDB"))]
pub struct ShaderDatabaseIndexed {
    // TODO: store and check version based on crate version

    // TODO: store file names in a separate list

    // TODO: parse/write with to clean this up
    #[br(temp)]
    #[bw(try_calc = u32::try_from(files.len()))]
    file_count: u32,
    #[br(count = file_count)]
    files: Vec<(NullString, ModelIndexed)>,

    #[br(temp)]
    #[bw(try_calc = u32::try_from(map_files.len()))]
    map_file_count: u32,
    #[br(count = map_file_count)]
    map_files: Vec<(NullString, MapIndexed)>,

    #[br(temp)]
    #[bw(try_calc = u16::try_from(dependencies.len()))]
    dependency_count: u16,
    #[br(count = dependency_count)]
    dependencies: Vec<DependencyIndexed>,

    // TODO: struct to make accessing values easier?
    #[br(temp)]
    #[bw(try_calc = u8::try_from(strings.len()))]
    string_count: u8,
    #[br(count = string_count)]
    strings: Vec<NullString>,

    #[br(temp)]
    #[bw(try_calc = u8::try_from(outputs.len()))]
    output_count: u8,
    #[br(count = output_count)]
    outputs: Vec<NullString>,
}

#[binrw]
#[derive(Debug, PartialEq, Clone)]
struct MapIndexed {
    #[br(temp)]
    #[bw(try_calc = u16::try_from(map_models.len()))]
    map_model_count: u16,
    #[br(count = map_model_count)]
    map_models: Vec<ModelIndexed>,

    #[br(temp)]
    #[bw(try_calc = u16::try_from(prop_models.len()))]
    prop_model_count: u16,
    #[br(count = prop_model_count)]
    prop_models: Vec<ModelIndexed>,

    #[br(temp)]
    #[bw(try_calc = u16::try_from(env_models.len()))]
    env_model_count: u16,
    #[br(count = env_model_count)]
    env_models: Vec<ModelIndexed>,
}

#[binrw]
#[derive(Debug, PartialEq, Clone)]
struct ModelIndexed {
    #[br(temp)]
    #[bw(try_calc = u16::try_from(programs.len()))]
    program_count: u16,
    #[br(count = program_count)]
    programs: Vec<ShaderProgramIndexed>,
}

#[binrw]
#[derive(Debug, PartialEq, Clone)]
struct ShaderProgramIndexed {
    // There are very few unique dependencies across all shaders in a game dump.
    // Normalize the data to greatly reduce the size file size.
    #[br(temp)]
    #[bw(try_calc = u8::try_from(output_dependencies.len()))]
    output_dependency_count: u8,
    #[br(count = output_dependency_count)]
    output_dependencies: Vec<(StringIndex, OutputDependenciesIndexed)>,
}

#[binrw]
#[derive(Debug, PartialEq, Clone)]
struct OutputDependenciesIndexed {
    #[br(temp)]
    #[bw(try_calc = u16::try_from(dependencies.len()))]
    dependency_count: u16,
    #[br(count = dependency_count)]
    dependencies: Vec<DependencyIndex>,

    #[br(temp)]
    #[bw(try_calc = u8::try_from(layers.len()))]
    layer_count: u8,
    #[br(count = layer_count)]
    layers: Vec<TextureLayerIndexed>,
}

#[derive(Debug, PartialEq, Clone, BinRead, BinWrite)]
struct TextureLayerIndexed {
    value: DependencyIndex,
    ratio: i16, // TODO: make dependency indexed optional?
    blend_mode: LayerBlendModeIndexed,
    is_fresnel: u8,
}

#[derive(Debug, PartialEq, Clone, Copy, BinRead, BinWrite)]
#[brw(repr(u8))]
pub enum LayerBlendModeIndexed {
    Mix = 0,
    MixRatio = 1,
    Add = 2,
    AddNormal = 3,
    Overlay = 4,
}

impl From<LayerBlendMode> for LayerBlendModeIndexed {
    fn from(value: LayerBlendMode) -> Self {
        match value {
            LayerBlendMode::Mix => Self::Mix,
            LayerBlendMode::MixRatio => Self::MixRatio,
            LayerBlendMode::Add => Self::Add,
            LayerBlendMode::AddNormal => Self::AddNormal,
            LayerBlendMode::Overlay => Self::Overlay,
        }
    }
}

impl From<LayerBlendModeIndexed> for LayerBlendMode {
    fn from(value: LayerBlendModeIndexed) -> Self {
        match value {
            LayerBlendModeIndexed::Mix => Self::Mix,
            LayerBlendModeIndexed::MixRatio => Self::MixRatio,
            LayerBlendModeIndexed::Add => Self::Add,
            LayerBlendModeIndexed::AddNormal => Self::AddNormal,
            LayerBlendModeIndexed::Overlay => Self::Overlay,
        }
    }
}

#[derive(Debug, PartialEq, Clone, BinRead, BinWrite)]
enum DependencyIndexed {
    #[brw(magic(0u8))]
    Constant(f32),

    #[brw(magic(1u8))]
    Buffer(BufferDependencyIndexed),

    #[brw(magic(2u8))]
    Texture(TextureDependencyIndexed),

    #[brw(magic(3u8))]
    Attribute(AttributeDependencyIndexed),
}

#[derive(Debug, PartialEq, Clone, BinRead, BinWrite)]
struct BufferDependencyIndexed {
    name: StringIndex,
    field: StringIndex,
    index: u8,
    channels: StringIndex,
}

#[binrw]
#[derive(Debug, PartialEq, Clone)]
struct TextureDependencyIndexed {
    name: StringIndex,
    channels: StringIndex,

    #[br(temp)]
    #[bw(try_calc = u8::try_from(texcoords.len()))]
    texcoord_count: u8,
    #[br(count = texcoord_count)]
    texcoords: Vec<TexCoordIndexed>,
}

#[binrw]
#[derive(Debug, PartialEq, Clone)]
struct TexCoordIndexed {
    name: StringIndex,
    channels: StringIndex,
    params: TexCoordParamsIndexed,
}

#[derive(Debug, PartialEq, Clone, BinRead, BinWrite)]
enum TexCoordParamsIndexed {
    #[brw(magic(0u8))]
    None,

    #[brw(magic(1u8))]
    Scale(BufferDependencyIndexed),

    #[brw(magic(2u8))]
    Matrix([BufferDependencyIndexed; 4]),
}

#[derive(Debug, PartialEq, Clone, BinRead, BinWrite)]
struct AttributeDependencyIndexed {
    name: StringIndex,
    channels: StringIndex,
}

impl ShaderDatabaseIndexed {
    pub fn from_file<P: AsRef<Path>>(path: P) -> BinResult<Self> {
        let mut reader = Cursor::new(std::fs::read(path)?);
        reader.read_le()
    }

    pub fn save<P: AsRef<Path>>(&self, path: P) -> BinResult<()> {
        let mut writer = Cursor::new(Vec::new());
        writer.write_le(self)?;
        std::fs::write(path, writer.into_inner())?;
        Ok(())
    }

    pub fn model(&self, name: &str) -> Option<ModelPrograms> {
        // TODO: Faster searches.
        self.files.iter().find_map(|(n, f)| {
            if n.to_string() == name {
                Some(model_from_indexed(
                    f,
                    &self.dependencies,
                    &self.strings,
                    &self.outputs,
                ))
            } else {
                None
            }
        })
    }

    pub fn map(&self, name: &str) -> Option<MapPrograms> {
        self.map_files.iter().find_map(|(n, f)| {
            if n.to_string() == name {
                Some(map_from_indexed(
                    f,
                    &self.dependencies,
                    &self.strings,
                    &self.outputs,
                ))
            } else {
                None
            }
        })
    }

    pub fn from_models_maps(
        models: IndexMap<String, ModelPrograms>,
        maps: IndexMap<String, MapPrograms>,
    ) -> Self {
        let mut dependency_to_index = IndexMap::new();
        let mut string_to_index = IndexMap::new();
        let mut output_to_index = IndexMap::new();

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
                    Dependency::Constant(c) => DependencyIndexed::Constant(c.0),
                    Dependency::Buffer(b) => DependencyIndexed::Buffer(buffer_dependency_indexed(
                        b,
                        &mut string_to_index,
                    )),
                    Dependency::Texture(t) => {
                        DependencyIndexed::Texture(TextureDependencyIndexed {
                            name: string_to_index.entry_index(t.name).try_into().unwrap(),
                            channels: string_to_index.entry_index(t.channels).try_into().unwrap(),
                            texcoords: t
                                .texcoords
                                .into_iter()
                                .map(|t| TexCoordIndexed {
                                    name: string_to_index.entry_index(t.name).try_into().unwrap(),
                                    channels: string_to_index
                                        .entry_index(t.channels)
                                        .try_into()
                                        .unwrap(),
                                    params: t
                                        .params
                                        .map(|params| match params {
                                            TexCoordParams::Scale(s) => {
                                                TexCoordParamsIndexed::Scale(
                                                    buffer_dependency_indexed(
                                                        s,
                                                        &mut string_to_index,
                                                    ),
                                                )
                                            }
                                            TexCoordParams::Matrix(m) => {
                                                TexCoordParamsIndexed::Matrix(m.map(|s| {
                                                    buffer_dependency_indexed(
                                                        s,
                                                        &mut string_to_index,
                                                    )
                                                }))
                                            }
                                        })
                                        .unwrap_or(TexCoordParamsIndexed::None),
                                })
                                .collect(),
                        })
                    }
                    Dependency::Attribute(a) => {
                        DependencyIndexed::Attribute(AttributeDependencyIndexed {
                            name: string_to_index.entry_index(a.name).try_into().unwrap(),
                            channels: string_to_index.entry_index(a.channels).try_into().unwrap(),
                        })
                    }
                })
                .collect(),
            strings: string_to_index
                .into_keys()
                .map(|k| k.to_string().into())
                .collect(),
            outputs: output_to_index
                .into_keys()
                .map(|k| k.to_string().into())
                .collect(),
        }
    }
}

fn buffer_dependency_indexed(
    b: BufferDependency,
    string_to_index: &mut IndexMap<SmolStr, usize>,
) -> BufferDependencyIndexed {
    BufferDependencyIndexed {
        name: string_to_index.entry_index(b.name).try_into().unwrap(),
        field: string_to_index.entry_index(b.field).try_into().unwrap(),
        index: b.index.try_into().unwrap(),
        channels: string_to_index.entry_index(b.channels).try_into().unwrap(),
    }
}

fn dependency_from_indexed(d: DependencyIndexed, strings: &[NullString]) -> Dependency {
    match d {
        DependencyIndexed::Constant(f) => Dependency::Constant(f.into()),
        DependencyIndexed::Buffer(b) => Dependency::Buffer(buffer_dependency(b, strings)),
        DependencyIndexed::Texture(t) => Dependency::Texture(TextureDependency {
            name: strings[t.name.0 as usize].to_smolstr(),
            channels: strings[t.channels.0 as usize].to_smolstr(),
            texcoords: t
                .texcoords
                .into_iter()
                .map(|coord| TexCoord {
                    name: strings[coord.name.0 as usize].to_smolstr(),
                    channels: strings[coord.channels.0 as usize].to_smolstr(),
                    params: match coord.params {
                        TexCoordParamsIndexed::None => None,
                        TexCoordParamsIndexed::Scale(s) => {
                            Some(TexCoordParams::Scale(buffer_dependency(s, strings)))
                        }
                        TexCoordParamsIndexed::Matrix(m) => Some(TexCoordParams::Matrix(
                            m.map(|s| buffer_dependency(s, strings)),
                        )),
                    },
                })
                .collect(),
        }),
        DependencyIndexed::Attribute(a) => Dependency::Attribute(AttributeDependency {
            name: strings[a.name.0 as usize].to_smolstr(),
            channels: strings[a.channels.0 as usize].to_smolstr(),
        }),
    }
}

fn buffer_dependency(b: BufferDependencyIndexed, strings: &[NullString]) -> BufferDependency {
    BufferDependency {
        name: strings[b.name.0 as usize].to_smolstr(),
        field: strings[b.field.0 as usize].to_smolstr(),
        index: b.index as usize,
        channels: strings[b.channels.0 as usize].to_smolstr(),
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
            .map(|p| {
                ShaderProgramIndexed {
                    output_dependencies: p
                        .output_dependencies
                        .into_iter()
                        .map(|(output, dependencies)| {
                            // This works since the map preserves insertion order.
                            let output_index = output_to_index.entry_index(output);
                            (
                                output_index.try_into().unwrap(),
                                OutputDependenciesIndexed {
                                    dependencies: dependencies
                                        .dependencies
                                        .into_iter()
                                        .map(|d| {
                                            dependency_to_index.entry_index(d).try_into().unwrap()
                                        })
                                        .collect(),
                                    layers: dependencies
                                        .layers
                                        .into_iter()
                                        .map(|l| TextureLayerIndexed {
                                            value: dependency_to_index
                                                .entry_index(l.value)
                                                .try_into()
                                                .unwrap(),
                                            ratio: l
                                                .ratio
                                                .map(|r| dependency_to_index.entry_index(r) as i16)
                                                .unwrap_or(-1),
                                            blend_mode: l.blend_mode.into(),
                                            is_fresnel: l.is_fresnel.into(),
                                        })
                                        .collect(),
                                },
                            )
                        })
                        .collect(),
                }
            })
            .collect(),
    }
}

fn model_from_indexed(
    model: &ModelIndexed,
    dependencies: &[DependencyIndexed],
    strings: &[NullString],
    outputs: &[NullString],
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
                            outputs[output.0 as usize].to_smolstr(),
                            OutputDependencies {
                                dependencies: output_dependencies
                                    .dependencies
                                    .iter()
                                    .map(|d| {
                                        dependency_from_indexed(
                                            dependencies[d.0 as usize].clone(),
                                            strings,
                                        )
                                    })
                                    .collect(),
                                layers: output_dependencies
                                    .layers
                                    .iter()
                                    .map(|l| TextureLayer {
                                        value: dependency_from_indexed(
                                            dependencies[l.value.0 as usize].clone(),
                                            strings,
                                        ),
                                        ratio: usize::try_from(l.ratio).ok().map(|i| {
                                            dependency_from_indexed(
                                                dependencies[i].clone(),
                                                strings,
                                            )
                                        }),
                                        blend_mode: l.blend_mode.into(),
                                        is_fresnel: l.is_fresnel != 0,
                                    })
                                    .collect(),
                            },
                        )
                    })
                    .collect(),
            })
            .collect(),
    }
}

fn map_from_indexed(
    map: &MapIndexed,
    dependencies: &[DependencyIndexed],
    strings: &[NullString],
    outputs: &[NullString],
) -> MapPrograms {
    MapPrograms {
        map_models: map
            .map_models
            .iter()
            .map(|s| model_from_indexed(s, dependencies, strings, outputs))
            .collect(),
        prop_models: map
            .prop_models
            .iter()
            .map(|s| model_from_indexed(s, dependencies, strings, outputs))
            .collect(),
        env_models: map
            .env_models
            .iter()
            .map(|s| model_from_indexed(s, dependencies, strings, outputs))
            .collect(),
    }
}
