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
type BufferDependencyIndex = Index<u16>;
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

    #[br(parse_with = parse_count16)]
    #[bw(write_with = write_count16)]
    buffer_dependencies: Vec<BufferDependencyIndexed>,

    // Storing multiple string tables enables 8-bit instead of 16-bit indices.
    #[br(parse_with = parse_count8)]
    #[bw(write_with = write_count8)]
    strings: Vec<NullString>,

    #[br(parse_with = parse_count8)]
    #[bw(write_with = write_count8)]
    texture_names: Vec<NullString>,

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

#[derive(Debug, PartialEq, Clone, Copy, BinRead, BinWrite)]
#[brw(repr(u8))]
pub enum Channel {
    None = 0,
    X = 1,
    Y = 2,
    Z = 3,
    W = 4,
}

impl From<Channel> for Option<char> {
    fn from(value: Channel) -> Self {
        match value {
            Channel::None => None,
            Channel::X => Some('x'),
            Channel::Y => Some('y'),
            Channel::Z => Some('z'),
            Channel::W => Some('w'),
        }
    }
}

impl From<Option<char>> for Channel {
    fn from(value: Option<char>) -> Self {
        match value {
            Some('x') => Self::X,
            Some('y') => Self::Y,
            Some('z') => Self::Z,
            Some('w') => Self::W,
            None => Self::None,
            _ => panic!("unable to convert {value:?} to channel"),
        }
    }
}

// TODO: How to handle recursion?
#[derive(Debug, PartialEq, Clone, BinRead, BinWrite)]
enum DependencyIndexed {
    #[brw(magic(0u8))]
    Constant(f32),

    #[brw(magic(1u8))]
    Buffer(BufferDependencyIndex),

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
    channel: Channel,
}

#[binrw]
#[derive(Debug, PartialEq, Clone)]
struct TextureDependencyIndexed {
    name: StringIndex,
    channel: Channel,

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
    channel: Channel,
    params: TexCoordParamsIndexed,
}

#[derive(Debug, PartialEq, Clone, BinRead, BinWrite)]
enum TexCoordParamsIndexed {
    #[brw(magic(0u8))]
    None,

    #[brw(magic(1u8))]
    Scale(BufferDependencyIndex),

    #[brw(magic(2u8))]
    Matrix([BufferDependencyIndex; 4]),

    #[brw(magic(3u8))]
    Parallax {
        mask: DependencyIndex,
        param: BufferDependencyIndex,
        param_ratio: BufferDependencyIndex,
    },
}

#[derive(Debug, PartialEq, Clone, BinRead, BinWrite)]
struct AttributeDependencyIndexed {
    name: StringIndex,
    channel: Channel,
}

impl Default for ShaderDatabaseIndexed {
    fn default() -> Self {
        Self {
            major_version: MAJOR_VERSION,
            minor_version: MINOR_VERSION,
            files: Default::default(),
            map_files: Default::default(),
            dependencies: Default::default(),
            buffer_dependencies: Default::default(),
            strings: Default::default(),
            texture_names: Default::default(),
            outputs: Default::default(),
        }
    }
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
        self.files.get(name).map(|f| self.model_from_indexed(f))
    }

    pub fn map(&self, name: &str) -> Option<MapPrograms> {
        self.map_files.get(name).map(|f| self.map_from_indexed(f))
    }

    pub fn from_models_maps(
        models: IndexMap<String, ModelPrograms>,
        maps: IndexMap<String, MapPrograms>,
    ) -> Self {
        let mut dependency_to_index = IndexMap::new();
        let mut buffer_dependency_to_index = IndexMap::new();

        let mut database = Self::default();

        for (name, model) in models {
            let model = database.model_indexed(
                model,
                &mut dependency_to_index,
                &mut buffer_dependency_to_index,
            );
            database.files.insert(name.into(), model);
        }

        for (name, map) in maps {
            let map = database.map_indexed(
                map,
                &mut dependency_to_index,
                &mut buffer_dependency_to_index,
            );
            database.map_files.insert(name.into(), map);
        }

        database
    }

    fn add_dependency(
        &mut self,
        d: Dependency,
        dependency_to_index: &mut IndexMap<Dependency, usize>,
        buffer_dependency_to_index: &mut IndexMap<BufferDependency, usize>,
    ) -> DependencyIndex {
        let index = match dependency_to_index.get(&d) {
            Some(index) => *index,
            None => {
                let dependency = self.dependency_indexed(
                    d.clone(),
                    dependency_to_index,
                    buffer_dependency_to_index,
                );

                let index = self.dependencies.len();

                self.dependencies.push(dependency);
                dependency_to_index.insert(d, index);

                index
            }
        };

        index.try_into().unwrap()
    }

    fn add_buffer_dependency(
        &mut self,
        b: BufferDependency,
        buffer_dependency_to_index: &mut IndexMap<BufferDependency, usize>,
    ) -> DependencyIndex {
        let index = match buffer_dependency_to_index.get(&b) {
            Some(index) => *index,
            None => {
                let dependency = self.buffer_dependency_indexed(b.clone());

                let index = self.buffer_dependencies.len();

                self.buffer_dependencies.push(dependency);
                buffer_dependency_to_index.insert(b, index);

                index
            }
        };

        index.try_into().unwrap()
    }

    fn add_output(&mut self, output: &str) -> StringIndex {
        add_string(&mut self.outputs, output)
    }

    fn add_string(&mut self, str: &str) -> StringIndex {
        add_string(&mut self.strings, str)
    }

    fn add_texture(&mut self, texture: &str) -> StringIndex {
        add_string(&mut self.texture_names, texture)
    }

    fn model_indexed(
        &mut self,
        model: ModelPrograms,
        dependency_to_index: &mut IndexMap<Dependency, usize>,
        buffer_dependency_to_index: &mut IndexMap<BufferDependency, usize>,
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
                            let output_index = self.add_output(&output);
                            (
                                output_index,
                                OutputDependenciesIndexed {
                                    dependencies: dependencies
                                        .dependencies
                                        .into_iter()
                                        .map(|d| {
                                            self.add_dependency(
                                                d,
                                                dependency_to_index,
                                                buffer_dependency_to_index,
                                            )
                                        })
                                        .collect(),
                                    layers: dependencies
                                        .layers
                                        .into_iter()
                                        .map(|l| TextureLayerIndexed {
                                            value: self.add_dependency(
                                                l.value,
                                                dependency_to_index,
                                                buffer_dependency_to_index,
                                            ),
                                            ratio: l
                                                .ratio
                                                .map(|r| {
                                                    self.add_dependency(
                                                        r,
                                                        dependency_to_index,
                                                        buffer_dependency_to_index,
                                                    )
                                                    .0
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
                })
                .collect(),
        }
    }

    fn map_indexed(
        &mut self,
        map: MapPrograms,
        dependency_to_index: &mut IndexMap<Dependency, usize>,
        buffer_dependency_to_index: &mut IndexMap<BufferDependency, usize>,
    ) -> MapIndexed {
        MapIndexed {
            map_models: map
                .map_models
                .into_iter()
                .map(|m| self.model_indexed(m, dependency_to_index, buffer_dependency_to_index))
                .collect(),
            prop_models: map
                .prop_models
                .into_iter()
                .map(|m| self.model_indexed(m, dependency_to_index, buffer_dependency_to_index))
                .collect(),
            env_models: map
                .env_models
                .into_iter()
                .map(|m| self.model_indexed(m, dependency_to_index, buffer_dependency_to_index))
                .collect(),
        }
    }

    fn map_from_indexed(&self, map: &MapIndexed) -> MapPrograms {
        MapPrograms {
            map_models: map
                .map_models
                .iter()
                .map(|s| self.model_from_indexed(s))
                .collect(),
            prop_models: map
                .prop_models
                .iter()
                .map(|s| self.model_from_indexed(s))
                .collect(),
            env_models: map
                .env_models
                .iter()
                .map(|s| self.model_from_indexed(s))
                .collect(),
        }
    }

    fn model_from_indexed(&self, model: &ModelIndexed) -> ModelPrograms {
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
                                self.outputs[output.0 as usize].to_smolstr(),
                                OutputDependencies {
                                    dependencies: output_dependencies
                                        .dependencies
                                        .iter()
                                        .map(|d| self.dependency_from_indexed(*d))
                                        .collect(),
                                    layers: output_dependencies
                                        .layers
                                        .iter()
                                        .map(|l| TextureLayer {
                                            value: self.dependency_from_indexed(l.value),
                                            ratio: usize::try_from(l.ratio).ok().map(|i| {
                                                self.dependency_from_indexed(i.try_into().unwrap())
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
                        .map(|i| self.dependency_from_indexed(i.try_into().unwrap())),
                })
                .collect(),
        }
    }

    fn dependency_from_indexed(&self, d: DependencyIndex) -> Dependency {
        match self.dependencies[d.0 as usize].clone() {
            DependencyIndexed::Constant(f) => Dependency::Constant(f.into()),
            DependencyIndexed::Buffer(b) => Dependency::Buffer(buffer_dependency(
                self.buffer_dependencies[b.0 as usize].clone(),
                &self.strings,
            )),
            DependencyIndexed::Texture(t) => Dependency::Texture(TextureDependency {
                name: self.texture_names[t.name.0 as usize].to_smolstr(),
                channel: t.channel.into(),
                texcoords: t
                    .texcoords
                    .into_iter()
                    .map(|coord| TexCoord {
                        name: self.strings[coord.name.0 as usize].to_smolstr(),
                        channel: coord.channel.into(),
                        params: match coord.params {
                            TexCoordParamsIndexed::None => None,
                            TexCoordParamsIndexed::Scale(s) => {
                                Some(TexCoordParams::Scale(buffer_dependency(
                                    self.buffer_dependencies[s.0 as usize].clone(),
                                    &self.strings,
                                )))
                            }
                            TexCoordParamsIndexed::Matrix(m) => {
                                Some(TexCoordParams::Matrix(m.map(|s| {
                                    buffer_dependency(
                                        self.buffer_dependencies[s.0 as usize].clone(),
                                        &self.strings,
                                    )
                                })))
                            }
                            TexCoordParamsIndexed::Parallax {
                                mask,
                                param,
                                param_ratio,
                            } => Some(TexCoordParams::Parallax {
                                mask: self.dependency_from_indexed(mask),
                                param: buffer_dependency(
                                    self.buffer_dependencies[param.0 as usize].clone(),
                                    &self.strings,
                                ),
                                param_ratio: buffer_dependency(
                                    self.buffer_dependencies[param_ratio.0 as usize].clone(),
                                    &self.strings,
                                ),
                            }),
                        },
                    })
                    .collect(),
            }),
            DependencyIndexed::Attribute(a) => Dependency::Attribute(AttributeDependency {
                name: self.strings[a.name.0 as usize].to_smolstr(),
                channel: a.channel.into(),
            }),
        }
    }

    fn dependency_indexed(
        &mut self,
        d: Dependency,
        dependency_to_index: &mut IndexMap<Dependency, usize>,
        buffer_dependency_to_index: &mut IndexMap<BufferDependency, usize>,
    ) -> DependencyIndexed {
        match d {
            Dependency::Constant(c) => DependencyIndexed::Constant(c.0),
            Dependency::Buffer(b) => {
                DependencyIndexed::Buffer(self.add_buffer_dependency(b, buffer_dependency_to_index))
            }
            Dependency::Texture(t) => DependencyIndexed::Texture(TextureDependencyIndexed {
                name: self.add_texture(&t.name),
                channel: t.channel.into(),
                texcoords: t
                    .texcoords
                    .into_iter()
                    .map(|t| TexCoordIndexed {
                        name: self.add_string(&t.name),
                        channel: t.channel.into(),
                        params: t
                            .params
                            .map(|params| match params {
                                TexCoordParams::Scale(s) => TexCoordParamsIndexed::Scale(
                                    self.add_buffer_dependency(s, buffer_dependency_to_index),
                                ),
                                TexCoordParams::Matrix(m) => {
                                    TexCoordParamsIndexed::Matrix(m.map(|s| {
                                        self.add_buffer_dependency(s, buffer_dependency_to_index)
                                    }))
                                }
                                TexCoordParams::Parallax {
                                    mask,
                                    param,
                                    param_ratio,
                                } => TexCoordParamsIndexed::Parallax {
                                    mask: self.add_dependency(
                                        mask,
                                        dependency_to_index,
                                        buffer_dependency_to_index,
                                    ),
                                    param: self
                                        .add_buffer_dependency(param, buffer_dependency_to_index),
                                    param_ratio: self.add_buffer_dependency(
                                        param_ratio,
                                        buffer_dependency_to_index,
                                    ),
                                },
                            })
                            .unwrap_or(TexCoordParamsIndexed::None),
                    })
                    .collect(),
            }),
            Dependency::Attribute(a) => DependencyIndexed::Attribute(AttributeDependencyIndexed {
                name: self.add_string(&a.name),
                channel: a.channel.into(),
            }),
        }
    }

    fn buffer_dependency_indexed(&mut self, b: BufferDependency) -> BufferDependencyIndexed {
        BufferDependencyIndexed {
            name: self.add_string(&b.name),
            field: self.add_string(&b.field),
            index: b.index.map(|i| i.try_into().unwrap()).unwrap_or(-1),
            channel: b.channel.into(),
        }
    }
}

fn add_string(strings: &mut Vec<NullString>, str: &str) -> StringIndex {
    // TODO: Store as regular strings.
    strings
        .iter()
        .position(|s| s.to_string() == str)
        .unwrap_or_else(|| {
            let index = strings.len();
            strings.push(str.into());
            index
        })
        .try_into()
        .unwrap()
}

fn buffer_dependency(b: BufferDependencyIndexed, strings: &[NullString]) -> BufferDependency {
    BufferDependency {
        name: strings[b.name.0 as usize].to_smolstr(),
        field: strings[b.field.0 as usize].to_smolstr(),
        index: usize::try_from(b.index).ok(),
        channel: b.channel.into(),
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
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
    (u16::try_from(value.len()).unwrap()).write_options(writer, endian, ())?;
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
    (u8::try_from(map.len()).unwrap()).write_options(writer, endian, ())?;
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
    (u32::try_from(map.len()).unwrap()).write_options(writer, endian, ())?;
    for (k, v) in map.iter() {
        (NullString::from(k.to_string())).write_options(writer, endian, ())?;
        v.write_options(writer, endian, ())?;
    }
    Ok(())
}
