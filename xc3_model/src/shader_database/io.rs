use std::{collections::BTreeMap, io::Cursor, path::Path};

use binrw::{binrw, BinRead, BinReaderExt, BinResult, BinWrite, BinWriterExt, NullString};
use indexmap::IndexSet;
use smol_str::ToSmolStr;
use varint_rs::{VarintReader, VarintWriter};

use super::{
    AttributeDependency, BufferDependency, Dependency, Layer, LayerBlendMode, LayerValue,
    OutputDependencies, ProgramHash, ShaderProgram, TexCoord, TexCoordParams, TextureDependency,
};

// Create a separate optimized representation for on disk.
#[binrw]
#[derive(Debug, PartialEq, Clone, Default)]
#[brw(magic(b"SHDB"))]
pub struct ShaderDatabaseIndexed {
    // File version numbers should be updated with each release.
    // This improves the error when parsing an incompatible version.
    #[br(assert(major_version == 3))]
    #[bw(calc = 3)]
    major_version: u16,
    #[bw(calc = 0)]
    _minor_version: u16,

    // Store unique shader programs across all models and maps.
    // This results in significantly fewer unique entries,
    // supports moving entries between files,
    // and allows for combining databases from different games.
    #[br(parse_with = parse_map32)]
    #[bw(write_with = write_map32)]
    programs: BTreeMap<u32, ShaderProgramIndexed>,

    #[br(parse_with = parse_count)]
    #[bw(write_with = write_count)]
    dependencies: Vec<DependencyIndexed>,

    #[br(parse_with = parse_count)]
    #[bw(write_with = write_count)]
    buffer_dependencies: Vec<BufferDependencyIndexed>,

    #[br(parse_with = parse_count)]
    #[bw(write_with = write_count)]
    layer_values: Vec<LayerValueIndexed>,

    #[br(parse_with = parse_count)]
    #[bw(write_with = write_count)]
    tex_coords: Vec<TexCoordIndexed>,

    // Storing multiple string lists enables 8-bit instead of 16-bit indices.
    #[br(parse_with = parse_count)]
    #[bw(write_with = write_count)]
    attribute_names: Vec<NullString>,

    #[br(parse_with = parse_count)]
    #[bw(write_with = write_count)]
    buffer_names: Vec<NullString>,

    #[br(parse_with = parse_count)]
    #[bw(write_with = write_count)]
    buffer_field_names: Vec<NullString>,

    #[br(parse_with = parse_count)]
    #[bw(write_with = write_count)]
    texture_names: Vec<NullString>,

    #[br(parse_with = parse_count)]
    #[bw(write_with = write_count)]
    outputs: Vec<NullString>,
}

#[binrw]
#[derive(Debug, PartialEq, Clone)]
struct MapIndexed {
    #[br(parse_with = parse_count)]
    #[bw(write_with = write_count)]
    map_models: Vec<ModelIndexed>,

    #[br(parse_with = parse_count)]
    #[bw(write_with = write_count)]
    prop_models: Vec<ModelIndexed>,

    #[br(parse_with = parse_count)]
    #[bw(write_with = write_count)]
    env_models: Vec<ModelIndexed>,
}

#[binrw]
#[derive(Debug, PartialEq, Clone)]
struct ModelIndexed {
    #[br(parse_with = parse_count)]
    #[bw(write_with = write_count)]
    programs: Vec<ShaderProgramIndexed>,
}

#[binrw]
#[derive(Debug, PartialEq, Clone)]
struct ShaderProgramIndexed {
    // There are very few unique dependencies across all shaders in a game dump.
    // Normalize the data to greatly reduce the size file size.
    #[br(parse_with = parse_count)]
    #[bw(write_with = write_count)]
    output_dependencies: Vec<(VarInt, OutputDependenciesIndexed)>,

    outline_width: OptVarInt,
}

#[binrw]
#[derive(Debug, PartialEq, Clone)]
struct OutputDependenciesIndexed {
    #[br(parse_with = parse_count)]
    #[bw(write_with = write_count)]
    dependencies: Vec<VarInt>,

    #[br(parse_with = parse_count)]
    #[bw(write_with = write_count)]
    layers: Vec<LayerIndexed>,
}

#[derive(Debug, PartialEq, Clone, BinRead, BinWrite)]
struct LayerIndexed {
    value: VarInt,
    ratio: VarInt,
    blend_mode: LayerBlendModeIndexed,
    is_fresnel: u8,
}

#[derive(Debug, PartialEq, Clone, BinRead, BinWrite)]
enum LayerValueIndexed {
    #[brw(magic(0u8))]
    Value(VarInt),

    #[brw(magic(1u8))]
    Layers(
        #[br(parse_with = parse_count)]
        #[bw(write_with = write_count)]
        Vec<LayerIndexed>,
    ),
}

#[derive(Debug, PartialEq, Clone, Copy, BinRead, BinWrite)]
#[brw(repr(u8))]
pub enum LayerBlendModeIndexed {
    Mix = 0,
    Mul = 1,
    Add = 2,
    AddNormal = 3,
    Overlay2 = 4,
    Overlay = 5,
    Power = 6,
    Min = 7,
    Max = 8,
    Clamp = 9,
}

impl From<LayerBlendMode> for LayerBlendModeIndexed {
    fn from(value: LayerBlendMode) -> Self {
        match value {
            LayerBlendMode::Mix => Self::Mix,
            LayerBlendMode::Mul => Self::Mul,
            LayerBlendMode::Add => Self::Add,
            LayerBlendMode::AddNormal => Self::AddNormal,
            LayerBlendMode::Overlay2 => Self::Overlay2,
            LayerBlendMode::Overlay => Self::Overlay,
            LayerBlendMode::Power => Self::Power,
            LayerBlendMode::Min => Self::Min,
            LayerBlendMode::Max => Self::Max,
            LayerBlendMode::Clamp => Self::Clamp,
        }
    }
}

impl From<LayerBlendModeIndexed> for LayerBlendMode {
    fn from(value: LayerBlendModeIndexed) -> Self {
        match value {
            LayerBlendModeIndexed::Mix => Self::Mix,
            LayerBlendModeIndexed::Mul => Self::Mul,
            LayerBlendModeIndexed::Add => Self::Add,
            LayerBlendModeIndexed::AddNormal => Self::AddNormal,
            LayerBlendModeIndexed::Overlay2 => Self::Overlay2,
            LayerBlendModeIndexed::Overlay => Self::Overlay,
            LayerBlendModeIndexed::Power => Self::Power,
            LayerBlendModeIndexed::Min => Self::Min,
            LayerBlendModeIndexed::Max => Self::Max,
            LayerBlendModeIndexed::Clamp => Self::Clamp,
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
            _ => {
                // TODO: Why does this happen for xcx de?
                println!("unable to convert {value:?} to channel");
                Self::None
            }
        }
    }
}

#[derive(Debug, PartialEq, Clone, BinRead, BinWrite)]
enum DependencyIndexed {
    #[brw(magic(0u8))]
    Constant(f32),

    #[brw(magic(1u8))]
    Buffer(VarInt),

    #[brw(magic(2u8))]
    Texture(TextureDependencyIndexed),

    #[brw(magic(3u8))]
    Attribute(AttributeDependencyIndexed),
}

#[derive(Debug, PartialEq, Clone, BinRead, BinWrite)]
struct BufferDependencyIndexed {
    name: VarInt,
    field: VarInt,
    index: OptVarInt,
    channel: Channel,
}

#[binrw]
#[derive(Debug, PartialEq, Clone)]
struct TextureDependencyIndexed {
    name: VarInt,
    channel: Channel,

    #[br(parse_with = parse_count)]
    #[bw(write_with = write_count)]
    texcoords: Vec<VarInt>,
}

#[binrw]
#[derive(Debug, PartialEq, Clone)]
struct TexCoordIndexed {
    name: VarInt,
    channel: Channel,
    params: TexCoordParamsIndexed,
}

#[derive(Debug, PartialEq, Clone, BinRead, BinWrite)]
enum TexCoordParamsIndexed {
    #[brw(magic(0u8))]
    None,

    #[brw(magic(1u8))]
    Scale(VarInt),

    #[brw(magic(2u8))]
    Matrix([VarInt; 4]),

    #[brw(magic(3u8))]
    Parallax {
        mask_a: VarInt,
        mask_b: VarInt,
        ratio: VarInt,
    },
}

#[derive(Debug, PartialEq, Clone, BinRead, BinWrite)]
struct AttributeDependencyIndexed {
    name: VarInt,
    channel: Channel,
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

    pub fn shader_program(&self, hash: ProgramHash) -> Option<ShaderProgram> {
        self.programs
            .get(&hash.0)
            .map(|p| self.program_from_indexed(p))
    }

    pub fn from_programs(programs: BTreeMap<ProgramHash, ShaderProgram>) -> Self {
        let mut dependency_to_index = IndexSet::new();
        let mut buffer_dependency_to_index = IndexSet::new();
        let mut layer_value_to_index = IndexSet::new();
        let mut tex_coord_to_index = IndexSet::new();

        let mut database = Self::default();

        // Use an ordered map for consistent ordering.
        for (hash, p) in programs.into_iter() {
            let program = database.program_indexed(
                p,
                &mut dependency_to_index,
                &mut buffer_dependency_to_index,
                &mut layer_value_to_index,
                &mut tex_coord_to_index,
            );
            database.programs.insert(hash.0, program);
        }

        database
    }

    pub fn merge(&self, other: &Self) -> Self {
        // TODO: reuse existing indices when merging?
        let mut dependency_to_index = IndexSet::new();
        let mut buffer_dependency_to_index = IndexSet::new();
        let mut layer_value_to_index = IndexSet::new();
        let mut tex_coord_to_index = IndexSet::new();

        let mut merged = Self::default();

        // Reindex all programs.
        for (hash, program) in &self.programs {
            let program = self.program_from_indexed(program);
            let indexed = merged.program_indexed(
                program,
                &mut dependency_to_index,
                &mut buffer_dependency_to_index,
                &mut layer_value_to_index,
                &mut tex_coord_to_index,
            );
            merged.programs.insert(*hash, indexed);
        }

        for (hash, program) in &other.programs {
            let program = other.program_from_indexed(program);
            let indexed = merged.program_indexed(
                program,
                &mut dependency_to_index,
                &mut buffer_dependency_to_index,
                &mut layer_value_to_index,
                &mut tex_coord_to_index,
            );
            merged.programs.insert(*hash, indexed);
        }

        merged
    }

    fn program_indexed(
        &mut self,
        p: ShaderProgram,
        dependency_to_index: &mut IndexSet<Dependency>,
        buffer_dependency_to_index: &mut IndexSet<BufferDependency>,
        layer_value_to_index: &mut IndexSet<LayerValue>,
        tex_coord_to_index: &mut IndexSet<TexCoord>,
    ) -> ShaderProgramIndexed {
        ShaderProgramIndexed {
            output_dependencies: p
                .output_dependencies
                .into_iter()
                .map(|(output, dependencies)| {
                    let output_index = add_string(&mut self.outputs, &output);
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
                                        tex_coord_to_index,
                                    )
                                })
                                .collect(),
                            layers: dependencies
                                .layers
                                .iter()
                                .map(|l| {
                                    self.add_output_layer_indexed(
                                        l,
                                        dependency_to_index,
                                        buffer_dependency_to_index,
                                        layer_value_to_index,
                                        tex_coord_to_index,
                                    )
                                })
                                .collect(),
                        },
                    )
                })
                .collect(),
            outline_width: OptVarInt(p.outline_width.map(|d| {
                self.add_dependency(
                    d,
                    dependency_to_index,
                    buffer_dependency_to_index,
                    tex_coord_to_index,
                )
                .0
            })),
        }
    }

    fn add_output_layer_indexed(
        &mut self,
        l: &Layer,
        dependency_to_index: &mut IndexSet<Dependency>,
        buffer_dependency_to_index: &mut IndexSet<BufferDependency>,
        layer_value_to_index: &mut IndexSet<LayerValue>,
        tex_coord_to_index: &mut IndexSet<TexCoord>,
    ) -> LayerIndexed {
        LayerIndexed {
            value: self.add_layer_value(
                dependency_to_index,
                buffer_dependency_to_index,
                layer_value_to_index,
                tex_coord_to_index,
                &l.value,
            ),
            ratio: self.add_layer_value(
                dependency_to_index,
                buffer_dependency_to_index,
                layer_value_to_index,
                tex_coord_to_index,
                &l.ratio,
            ),
            blend_mode: l.blend_mode.into(),
            is_fresnel: l.is_fresnel.into(),
        }
    }

    fn add_layer_value(
        &mut self,
        dependency_to_index: &mut IndexSet<Dependency>,
        buffer_dependency_to_index: &mut IndexSet<BufferDependency>,
        layer_value_to_index: &mut IndexSet<LayerValue>,
        tex_coord_to_index: &mut IndexSet<TexCoord>,
        value: &LayerValue,
    ) -> VarInt {
        let index = match layer_value_to_index.get_index_of(value) {
            Some(index) => index,
            None => {
                let v = match &value {
                    LayerValue::Value(d) => LayerValueIndexed::Value(self.add_dependency(
                        d.clone(),
                        dependency_to_index,
                        buffer_dependency_to_index,
                        tex_coord_to_index,
                    )),
                    LayerValue::Layers(layers) => LayerValueIndexed::Layers(
                        layers
                            .iter()
                            .map(|l| {
                                self.add_output_layer_indexed(
                                    l,
                                    dependency_to_index,
                                    buffer_dependency_to_index,
                                    layer_value_to_index,
                                    tex_coord_to_index,
                                )
                            })
                            .collect(),
                    ),
                };

                let index = self.layer_values.len();

                self.layer_values.push(v);
                layer_value_to_index.insert(value.clone());

                index
            }
        };

        VarInt(index)
    }

    fn add_dependency(
        &mut self,
        d: Dependency,
        dependency_to_index: &mut IndexSet<Dependency>,
        buffer_dependency_to_index: &mut IndexSet<BufferDependency>,
        tex_coord_to_index: &mut IndexSet<TexCoord>,
    ) -> VarInt {
        let index = match dependency_to_index.get_index_of(&d) {
            Some(index) => index,
            None => {
                let dependency = self.dependency_indexed(
                    d.clone(),
                    dependency_to_index,
                    buffer_dependency_to_index,
                    tex_coord_to_index,
                );

                let index = self.dependencies.len();

                self.dependencies.push(dependency);
                dependency_to_index.insert(d);

                index
            }
        };

        VarInt(index)
    }

    fn add_buffer_dependency(
        &mut self,
        b: BufferDependency,
        buffer_dependency_to_index: &mut IndexSet<BufferDependency>,
    ) -> VarInt {
        let index = match buffer_dependency_to_index.get_index_of(&b) {
            Some(index) => index,
            None => {
                let dependency = self.buffer_dependency_indexed(b.clone());

                let index = self.buffer_dependencies.len();

                self.buffer_dependencies.push(dependency);
                buffer_dependency_to_index.insert(b);

                index
            }
        };

        VarInt(index)
    }

    fn program_from_indexed(&self, p: &ShaderProgramIndexed) -> ShaderProgram {
        ShaderProgram {
            output_dependencies: p
                .output_dependencies
                .iter()
                .map(|(output, output_dependencies)| {
                    (
                        self.outputs[output.0].to_smolstr(),
                        OutputDependencies {
                            dependencies: output_dependencies
                                .dependencies
                                .iter()
                                .map(|d| self.dependency_from_indexed(*d))
                                .collect(),
                            layers: output_dependencies
                                .layers
                                .iter()
                                .map(|l| self.output_layer_from_indexed(l))
                                .collect(),
                        },
                    )
                })
                .collect(),
            outline_width: p
                .outline_width
                .0
                .map(|i| self.dependency_from_indexed(VarInt(i))),
        }
    }

    fn output_layer_from_indexed(&self, l: &LayerIndexed) -> Layer {
        Layer {
            value: self.output_layer_value_from_indexed(&self.layer_values[l.value.0]),
            ratio: self.output_layer_value_from_indexed(&self.layer_values[l.ratio.0]),
            blend_mode: l.blend_mode.into(),
            is_fresnel: l.is_fresnel != 0,
        }
    }

    fn output_layer_value_from_indexed(&self, value: &LayerValueIndexed) -> LayerValue {
        match value {
            LayerValueIndexed::Value(d) => LayerValue::Value(self.dependency_from_indexed(*d)),
            LayerValueIndexed::Layers(layers) => LayerValue::Layers(
                layers
                    .iter()
                    .map(|l| self.output_layer_from_indexed(l))
                    .collect(),
            ),
        }
    }

    fn dependency_from_indexed(&self, d: VarInt) -> Dependency {
        match &self.dependencies[d.0] {
            DependencyIndexed::Constant(f) => Dependency::Constant((*f).into()),
            DependencyIndexed::Buffer(b) => {
                Dependency::Buffer(self.buffer_dependency_from_indexed(*b))
            }
            DependencyIndexed::Texture(t) => Dependency::Texture(TextureDependency {
                name: self.texture_names[t.name.0].to_smolstr(),
                channel: t.channel.into(),
                texcoords: t
                    .texcoords
                    .iter()
                    .map(|coord| self.tex_coord_from_indexed(*coord))
                    .collect(),
            }),
            DependencyIndexed::Attribute(a) => Dependency::Attribute(AttributeDependency {
                name: self.attribute_names[a.name.0].to_smolstr(),
                channel: a.channel.into(),
            }),
        }
    }

    fn dependency_indexed(
        &mut self,
        d: Dependency,
        dependency_to_index: &mut IndexSet<Dependency>,
        buffer_dependency_to_index: &mut IndexSet<BufferDependency>,
        tex_coord_to_index: &mut IndexSet<TexCoord>,
    ) -> DependencyIndexed {
        match d {
            Dependency::Constant(c) => DependencyIndexed::Constant(c.0),
            Dependency::Buffer(b) => {
                DependencyIndexed::Buffer(self.add_buffer_dependency(b, buffer_dependency_to_index))
            }
            Dependency::Texture(t) => DependencyIndexed::Texture(TextureDependencyIndexed {
                name: add_string(&mut self.texture_names, &t.name),
                channel: t.channel.into(),
                texcoords: t
                    .texcoords
                    .into_iter()
                    .map(|t| {
                        self.add_tex_coord(
                            t,
                            dependency_to_index,
                            buffer_dependency_to_index,
                            tex_coord_to_index,
                        )
                    })
                    .collect(),
            }),
            Dependency::Attribute(a) => DependencyIndexed::Attribute(AttributeDependencyIndexed {
                name: add_string(&mut self.attribute_names, &a.name),
                channel: a.channel.into(),
            }),
        }
    }

    fn add_tex_coord(
        &mut self,
        t: TexCoord,
        dependency_to_index: &mut IndexSet<Dependency>,
        buffer_dependency_to_index: &mut IndexSet<BufferDependency>,
        tex_coord_to_index: &mut IndexSet<TexCoord>,
    ) -> VarInt {
        let index = match tex_coord_to_index.get_index_of(&t) {
            Some(index) => index,
            None => {
                let tex_coord = self.tex_coord_indexed(
                    t.clone(),
                    dependency_to_index,
                    buffer_dependency_to_index,
                    tex_coord_to_index,
                );

                let index = self.tex_coords.len();

                self.tex_coords.push(tex_coord);
                tex_coord_to_index.insert(t);

                index
            }
        };

        VarInt(index)
    }

    fn tex_coord_from_indexed(&self, tex_coord: VarInt) -> TexCoord {
        let coord = &self.tex_coords[tex_coord.0];
        TexCoord {
            name: self.attribute_names[coord.name.0].to_smolstr(),
            channel: coord.channel.into(),
            params: match coord.params {
                TexCoordParamsIndexed::None => None,
                TexCoordParamsIndexed::Scale(s) => Some(TexCoordParams::Scale(
                    self.buffer_dependency_from_indexed(s),
                )),
                TexCoordParamsIndexed::Matrix(m) => Some(TexCoordParams::Matrix(
                    m.map(|s| self.buffer_dependency_from_indexed(s)),
                )),
                TexCoordParamsIndexed::Parallax {
                    mask_a,
                    mask_b,
                    ratio,
                } => Some(TexCoordParams::Parallax {
                    mask_a: self.dependency_from_indexed(mask_a),
                    mask_b: self.dependency_from_indexed(mask_b),
                    ratio: self.buffer_dependency_from_indexed(ratio),
                }),
            },
        }
    }

    fn tex_coord_indexed(
        &mut self,
        t: TexCoord,
        dependency_to_index: &mut IndexSet<Dependency>,
        buffer_dependency_to_index: &mut IndexSet<BufferDependency>,
        tex_coord_to_index: &mut IndexSet<TexCoord>,
    ) -> TexCoordIndexed {
        TexCoordIndexed {
            name: add_string(&mut self.attribute_names, &t.name),
            channel: t.channel.into(),
            params: t
                .params
                .map(|params| match params {
                    TexCoordParams::Scale(s) => TexCoordParamsIndexed::Scale(
                        self.add_buffer_dependency(s, buffer_dependency_to_index),
                    ),
                    TexCoordParams::Matrix(m) => TexCoordParamsIndexed::Matrix(
                        m.map(|s| self.add_buffer_dependency(s, buffer_dependency_to_index)),
                    ),
                    TexCoordParams::Parallax {
                        mask_a,
                        mask_b,
                        ratio,
                    } => TexCoordParamsIndexed::Parallax {
                        mask_a: self.add_dependency(
                            mask_a,
                            dependency_to_index,
                            buffer_dependency_to_index,
                            tex_coord_to_index,
                        ),
                        mask_b: self.add_dependency(
                            mask_b,
                            dependency_to_index,
                            buffer_dependency_to_index,
                            tex_coord_to_index,
                        ),
                        ratio: self.add_buffer_dependency(ratio, buffer_dependency_to_index),
                    },
                })
                .unwrap_or(TexCoordParamsIndexed::None),
        }
    }

    fn buffer_dependency_from_indexed(&self, b: VarInt) -> BufferDependency {
        let b = &self.buffer_dependencies[b.0];
        BufferDependency {
            name: self.buffer_names[b.name.0].to_smolstr(),
            field: self.buffer_field_names[b.field.0].to_smolstr(),
            index: b.index.0,
            channel: b.channel.into(),
        }
    }

    fn buffer_dependency_indexed(&mut self, b: BufferDependency) -> BufferDependencyIndexed {
        BufferDependencyIndexed {
            name: add_string(&mut self.buffer_names, &b.name),
            field: add_string(&mut self.buffer_field_names, &b.field),
            index: OptVarInt(b.index),
            channel: b.channel.into(),
        }
    }
}

fn add_string(strings: &mut Vec<NullString>, str: &str) -> VarInt {
    VarInt(
        strings
            .iter()
            .position(|s| s.to_string() == str)
            .unwrap_or_else(|| {
                let index = strings.len();
                strings.push(str.into());
                index
            }),
    )
}

// Variable length ints are slightly slower to parse but take up much less space.
#[derive(Debug, PartialEq, Clone, Copy)]
struct VarInt(usize);

impl BinRead for VarInt {
    type Args<'a> = ();

    fn read_options<R: std::io::Read + std::io::Seek>(
        reader: &mut R,
        _endian: binrw::Endian,
        _args: Self::Args<'_>,
    ) -> BinResult<Self> {
        reader.read_usize_varint().map(Self).map_err(Into::into)
    }
}

impl BinWrite for VarInt {
    type Args<'a> = ();

    fn write_options<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        _endian: binrw::Endian,
        _args: Self::Args<'_>,
    ) -> BinResult<()> {
        writer.write_usize_varint(self.0).map_err(Into::into)
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
struct OptVarInt(Option<usize>);

impl BinRead for OptVarInt {
    type Args<'a> = ();

    fn read_options<R: std::io::Read + std::io::Seek>(
        reader: &mut R,
        _endian: binrw::Endian,
        _args: Self::Args<'_>,
    ) -> BinResult<Self> {
        let value = reader.read_usize_varint()?;
        let index = value.checked_sub(1);
        Ok(Self(index))
    }
}

impl BinWrite for OptVarInt {
    type Args<'a> = ();

    fn write_options<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        _endian: binrw::Endian,
        _args: Self::Args<'_>,
    ) -> BinResult<()> {
        match self.0 {
            Some(index) => writer.write_usize_varint(index + 1)?,
            None => writer.write_usize_varint(0)?,
        }
        Ok(())
    }
}

#[binrw::parser(reader, endian)]
fn parse_count<T>() -> BinResult<Vec<T>>
where
    for<'a> T: BinRead<Args<'a> = ()> + 'static,
{
    let count = VarInt::read_options(reader, endian, ())?.0;
    <Vec<T>>::read_options(reader, endian, binrw::VecArgs { count, inner: () })
}

#[binrw::writer(writer, endian)]
fn write_count<T>(value: &Vec<T>) -> BinResult<()>
where
    for<'a> T: BinWrite<Args<'a> = ()> + 'static,
{
    VarInt(value.len()).write_options(writer, endian, ())?;
    value.write_options(writer, endian, ())?;
    Ok(())
}

fn parse_map32<T, R>(
    reader: &mut R,
    endian: binrw::Endian,
    _args: (),
) -> BinResult<BTreeMap<u32, T>>
where
    for<'a> T: BinRead<Args<'a> = ()> + 'static,
    R: std::io::Read + std::io::Seek,
{
    let count = u32::read_options(reader, endian, ())?;

    let mut map = BTreeMap::new();
    for _ in 0..count {
        let (key, value) = <(u32, T)>::read_options(reader, endian, ())?;
        map.insert(key, value);
    }
    Ok(map)
}

#[binrw::writer(writer, endian)]
fn write_map32<T>(map: &BTreeMap<u32, T>) -> BinResult<()>
where
    for<'a> T: BinWrite<Args<'a> = ()> + 'static,
{
    (u32::try_from(map.len()).unwrap()).write_options(writer, endian, ())?;
    for (k, v) in map.iter() {
        k.write_options(writer, endian, ())?;
        v.write_options(writer, endian, ())?;
    }
    Ok(())
}
