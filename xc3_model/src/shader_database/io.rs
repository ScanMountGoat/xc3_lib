use std::{io::Cursor, path::Path};

use crate::IndexMapExt;
use binrw::{binrw, BinRead, BinReaderExt, BinResult, BinWrite, BinWriterExt, NullString};
use indexmap::IndexMap;
use smol_str::{SmolStr, ToSmolStr};

use super::{
    AttributeDependency, BufferDependency, Dependency, LayerBlendMode, MapPrograms, ModelPrograms,
    OutputDependencies, ShaderProgram, TexCoord, TexCoordParams, TextureDependency, TextureLayer,
};

const MAJOR_VERSION: u16 = 1;
const MINOR_VERSION: u16 = 0;

type StringIndex = Index<u8>;
type DependencyIndex = Index<u16>;

// Create a separate optimized representation for on disk.
#[binrw]
#[derive(Debug, PartialEq, Clone)]
#[brw(magic(b"SHDB"))]
pub struct ShaderDatabaseIndexed {
    #[br(assert(major_version == MAJOR_VERSION))]
    major_version: u16,
    minor_version: u16,

    #[br(parse_with = parse_map32)]
    #[bw(write_with = write_map32)]
    files: IndexMap<SmolStr, ModelIndexed>,

    #[br(parse_with = parse_map32)]
    #[bw(write_with = write_map32)]
    map_files: IndexMap<SmolStr, MapIndexed>,

    #[br(parse_with = parse_count16)]
    #[bw(write_with = write_count16)]
    dependencies: Vec<DependencyIndexed>,

    #[br(parse_with = parse_count8)]
    #[bw(write_with = write_count8)]
    strings: Vec<NullString>,

    // Storing outputs separately enables 8-bit instead of 16-bit indices.
    #[br(parse_with = parse_count8)]
    #[bw(write_with = write_count8)]
    outputs: Vec<NullString>,
}

#[binrw]
#[derive(Debug, PartialEq, Clone)]
struct MapIndexed {
    #[br(parse_with = parse_count16)]
    #[bw(write_with = write_count16)]
    map_models: Vec<ModelIndexed>,

    #[br(parse_with = parse_count16)]
    #[bw(write_with = write_count16)]
    prop_models: Vec<ModelIndexed>,

    #[br(parse_with = parse_count16)]
    #[bw(write_with = write_count16)]
    env_models: Vec<ModelIndexed>,
}

#[binrw]
#[derive(Debug, PartialEq, Clone)]
struct ModelIndexed {
    #[br(parse_with = parse_count16)]
    #[bw(write_with = write_count16)]
    programs: Vec<ShaderProgramIndexed>,
}

#[binrw]
#[derive(Debug, PartialEq, Clone)]
struct ShaderProgramIndexed {
    // There are very few unique dependencies across all shaders in a game dump.
    // Normalize the data to greatly reduce the size file size.
    #[br(parse_with = parse_count8)]
    #[bw(write_with = write_count8)]
    output_dependencies: Vec<(StringIndex, OutputDependenciesIndexed)>,

    // TODO: Add optional dependency type.
    outline_width: i16,
}

#[binrw]
#[derive(Debug, PartialEq, Clone)]
struct OutputDependenciesIndexed {
    #[br(parse_with = parse_count16)]
    #[bw(write_with = write_count16)]
    dependencies: Vec<DependencyIndex>,

    #[br(parse_with = parse_count8)]
    #[bw(write_with = write_count8)]
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
    index: i8, // TODO: optional index type
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
        self.files
            .get(name)
            .map(|f| model_from_indexed(f, &self.dependencies, &self.strings, &self.outputs))
    }

    pub fn map(&self, name: &str) -> Option<MapPrograms> {
        self.map_files
            .get(name)
            .map(|f| map_from_indexed(f, &self.dependencies, &self.strings, &self.outputs))
    }

    pub fn from_models_maps(
        models: IndexMap<String, ModelPrograms>,
        maps: IndexMap<String, MapPrograms>,
    ) -> Self {
        let mut dependency_to_index = IndexMap::new();
        let mut string_to_index = IndexMap::new();
        let mut output_to_index = IndexMap::new();

        Self {
            major_version: MAJOR_VERSION,
            minor_version: MINOR_VERSION,
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
        index: b.index.map(|i| i.try_into().unwrap()).unwrap_or(-1),
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
        index: usize::try_from(b.index).ok(),
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
                                                .map(|r| {
                                                    dependency_to_index
                                                        .entry_index(r)
                                                        .try_into()
                                                        .unwrap()
                                                })
                                                .unwrap_or(-1),
                                            blend_mode: l.blend_mode.into(),
                                            is_fresnel: l.is_fresnel.into(),
                                        })
                                        .collect(),
                                },
                            )
                        })
                        .collect(),
                    outline_width: p
                        .outline_width
                        .map(|d| dependency_to_index.entry_index(d).try_into().unwrap())
                        .unwrap_or(-1),
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
                outline_width: usize::try_from(p.outline_width)
                    .ok()
                    .map(|i| dependency_from_indexed(dependencies[i].clone(), strings)),
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

fn parse_count<T, R, N>(reader: &mut R, endian: binrw::Endian) -> BinResult<Vec<T>>
where
    for<'a> T: BinRead<Args<'a> = ()> + 'static,
    for<'a> N: BinRead<Args<'a> = ()> + TryInto<usize>,
    <N as TryInto<usize>>::Error: std::fmt::Debug,
    R: std::io::Read + std::io::Seek,
{
    let count = N::read_options(reader, endian, ())?;

    <Vec<T>>::read_options(
        reader,
        endian,
        binrw::VecArgs {
            count: count.try_into().unwrap(),
            inner: (),
        },
    )
}

fn parse_count16<T, R>(reader: &mut R, endian: binrw::Endian, _args: ()) -> BinResult<Vec<T>>
where
    for<'a> T: BinRead<Args<'a> = ()> + 'static,
    R: std::io::Read + std::io::Seek,
{
    parse_count::<T, R, u16>(reader, endian)
}

#[binrw::writer(writer, endian)]
fn write_count16<T>(value: &Vec<T>) -> BinResult<()>
where
    for<'a> T: BinWrite<Args<'a> = ()> + 'static,
{
    (value.len() as u16).write_options(writer, endian, ())?;
    value.write_options(writer, endian, ())?;
    Ok(())
}

fn parse_count8<T, R>(reader: &mut R, endian: binrw::Endian, _args: ()) -> BinResult<Vec<T>>
where
    for<'a> T: BinRead<Args<'a> = ()> + 'static,
    R: std::io::Read + std::io::Seek,
{
    parse_count::<T, R, u8>(reader, endian)
}

#[binrw::writer(writer, endian)]
fn write_count8<T>(map: &Vec<T>) -> BinResult<()>
where
    for<'a> T: BinWrite<Args<'a> = ()> + 'static,
{
    (map.len() as u8).write_options(writer, endian, ())?;
    map.write_options(writer, endian, ())?;
    Ok(())
}

fn parse_map32<T, R>(
    reader: &mut R,
    endian: binrw::Endian,
    _args: (),
) -> BinResult<IndexMap<SmolStr, T>>
where
    for<'a> T: BinRead<Args<'a> = ()> + 'static,
    R: std::io::Read + std::io::Seek,
{
    let count = u32::read_options(reader, endian, ())?;

    let mut map = IndexMap::new();
    for _ in 0..count {
        let (key, value) = <(NullString, T)>::read_options(reader, endian, ())?;
        map.insert(key.to_smolstr(), value);
    }
    Ok(map)
}

#[binrw::writer(writer, endian)]
fn write_map32<T>(map: &IndexMap<SmolStr, T>) -> BinResult<()>
where
    for<'a> T: BinWrite<Args<'a> = ()> + 'static,
{
    (map.len() as u32).write_options(writer, endian, ())?;
    for (k, v) in map.iter() {
        (NullString::from(k.to_string())).write_options(writer, endian, ())?;
        v.write_options(writer, endian, ())?;
    }
    Ok(())
}
