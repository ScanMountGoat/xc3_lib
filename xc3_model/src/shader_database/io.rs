use std::{collections::BTreeMap, io::Cursor, path::Path};

use binrw::{binrw, BinRead, BinReaderExt, BinResult, BinWrite, BinWriterExt, NullString};
use smol_str::{SmolStr, ToSmolStr};
use varint_rs::{VarintReader, VarintWriter};

use super::{
    AttributeDependency, BufferDependency, Dependency, Operation, OutputExpr, ProgramHash,
    ShaderProgram, TexCoord, TexCoordParams, TextureDependency,
};

// Faster than the default hash implementation.
type IndexSet<T> = indexmap::IndexSet<T, ahash::RandomState>;

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
    output_exprs: Vec<OutputExprIndexed>,

    #[br(parse_with = parse_count)]
    #[bw(write_with = write_count)]
    tex_coords: Vec<TexCoordIndexed>,

    // Storing multiple string lists enables 8-bit instead of 16-bit indices.
    #[br(parse_with = parse_strings)]
    #[bw(write_with = write_strings)]
    attribute_names: Vec<SmolStr>,

    #[br(parse_with = parse_strings)]
    #[bw(write_with = write_strings)]
    buffer_names: Vec<SmolStr>,

    #[br(parse_with = parse_strings)]
    #[bw(write_with = write_strings)]
    buffer_field_names: Vec<SmolStr>,

    #[br(parse_with = parse_strings)]
    #[bw(write_with = write_strings)]
    texture_names: Vec<SmolStr>,

    #[br(parse_with = parse_strings)]
    #[bw(write_with = write_strings)]
    outputs: Vec<SmolStr>,
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
    output_dependencies: Vec<(VarInt, VarInt)>,

    outline_width: OptVarInt,
    normal_intensity: OptVarInt,
}

#[derive(Debug, PartialEq, Clone, BinRead, BinWrite)]
enum OutputExprIndexed {
    #[brw(magic(0u8))]
    Value(VarInt),

    #[brw(magic(1u8))]
    Func {
        op: OperationIndexed,

        #[br(parse_with = parse_count)]
        #[bw(write_with = write_count)]
        args: Vec<VarInt>,
    },
}

#[derive(Debug, PartialEq, Clone, Copy, BinRead, BinWrite)]
#[brw(repr(u8))]
pub enum OperationIndexed {
    Unk = 0,
    Mix = 1,
    Mul = 2,
    Div = 3,
    Add = 4,
    Sub = 5,
    Fma = 6,
    MulRatio = 7,
    AddNormal = 8,
    Overlay = 9,
    Overlay2 = 10,
    OverlayRatio = 11,
    Power = 12,
    Min = 13,
    Max = 14,
    Clamp = 15,
    Abs = 16,
    Fresnel = 17,
}

impl From<Operation> for OperationIndexed {
    fn from(value: Operation) -> Self {
        match value {
            Operation::Mix => Self::Mix,
            Operation::Mul => Self::Mul,
            Operation::Div => Self::Div,
            Operation::Add => Self::Add,
            Operation::Sub => Self::Sub,
            Operation::Fma => Self::Fma,
            Operation::MulRatio => Self::MulRatio,
            Operation::AddNormal => Self::AddNormal,
            Operation::Overlay => Self::Overlay,
            Operation::Overlay2 => Self::Overlay2,
            Operation::OverlayRatio => Self::OverlayRatio,
            Operation::Power => Self::Power,
            Operation::Min => Self::Min,
            Operation::Max => Self::Max,
            Operation::Clamp => Self::Clamp,
            Operation::Abs => Self::Abs,
            Operation::Fresnel => Self::Fresnel,
            Operation::Unk => Self::Unk,
        }
    }
}

impl From<OperationIndexed> for Operation {
    fn from(value: OperationIndexed) -> Self {
        match value {
            OperationIndexed::Mix => Self::Mix,
            OperationIndexed::Mul => Self::Mul,
            OperationIndexed::Div => Self::Div,
            OperationIndexed::Add => Self::Add,
            OperationIndexed::Sub => Self::Sub,
            OperationIndexed::Fma => Self::Fma,
            OperationIndexed::MulRatio => Self::MulRatio,
            OperationIndexed::AddNormal => Self::AddNormal,
            OperationIndexed::Overlay => Self::Overlay,
            OperationIndexed::Overlay2 => Self::Overlay2,
            OperationIndexed::OverlayRatio => Self::OverlayRatio,
            OperationIndexed::Power => Self::Power,
            OperationIndexed::Min => Self::Min,
            OperationIndexed::Max => Self::Max,
            OperationIndexed::Clamp => Self::Clamp,
            OperationIndexed::Abs => Self::Abs,
            OperationIndexed::Fresnel => Self::Fresnel,
            OperationIndexed::Unk => Self::Unk,
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
        let mut dependency_to_index = IndexSet::default();
        let mut buffer_dependency_to_index = IndexSet::default();
        let mut output_expr_to_index = IndexSet::default();
        let mut tex_coord_to_index = IndexSet::default();

        let mut database = Self::default();

        // Use an ordered map for consistent ordering.
        for (hash, p) in programs.into_iter() {
            let program = database.program_indexed(
                p,
                &mut dependency_to_index,
                &mut buffer_dependency_to_index,
                &mut output_expr_to_index,
                &mut tex_coord_to_index,
            );
            database.programs.insert(hash.0, program);
        }

        database
    }

    pub fn merge(self, others: impl Iterator<Item = Self>) -> Self {
        // Reuse existing indices when merging.
        let mut dependency_to_index = self
            .dependencies
            .iter()
            .map(|d| self.dependency_from_indexed(d))
            .collect();
        let mut buffer_dependency_to_index = self
            .buffer_dependencies
            .iter()
            .map(|b| self.buffer_dependency_from_indexed(b))
            .collect();
        let mut tex_coord_to_index = self
            .tex_coords
            .iter()
            .map(|t| self.tex_coord_from_indexed(t))
            .collect();

        let mut merged = self;

        // Reindex all programs.
        for mut other in others {
            // Remap indices to process unique items only once.
            let output_indices: Vec<_> = other
                .outputs
                .iter()
                .map(|o| add_string(&mut merged.outputs, o.clone()))
                .collect();

            let dependency_indices: Vec<_> = other
                .dependencies
                .iter()
                .map(|d| {
                    let d = other.dependency_from_indexed(d);
                    merged
                        .add_dependency(
                            d,
                            &mut dependency_to_index,
                            &mut buffer_dependency_to_index,
                            &mut tex_coord_to_index,
                        )
                        .0
                })
                .collect();

            // Remap output exprs in place to avoid costly indexing and large allocations.
            // TODO: Collect only unique exprs.
            let base_index = merged.output_exprs.len();
            for expr in &mut other.output_exprs {
                match expr {
                    OutputExprIndexed::Value(d) => {
                        *d = VarInt(dependency_indices[d.0]);
                    }
                    OutputExprIndexed::Func { args, .. } => {
                        for arg in args {
                            arg.0 += base_index;
                        }
                    }
                }
            }
            merged.output_exprs.extend_from_slice(&other.output_exprs);

            for (hash, program) in &other.programs {
                let mut program = program.clone();
                for (k, v) in &mut program.output_dependencies {
                    *k = output_indices[k.0];
                    v.0 += base_index;
                }
                merged.programs.insert(*hash, program);
            }
        }

        merged
    }

    fn program_indexed(
        &mut self,
        p: ShaderProgram,
        dependency_to_index: &mut IndexSet<Dependency>,
        buffer_dependency_to_index: &mut IndexSet<BufferDependency>,
        output_expr_to_index: &mut IndexSet<OutputExpr>,
        tex_coord_to_index: &mut IndexSet<TexCoord>,
    ) -> ShaderProgramIndexed {
        ShaderProgramIndexed {
            output_dependencies: p
                .output_dependencies
                .into_iter()
                .map(|(output, value)| {
                    let output_index = add_string(&mut self.outputs, output);
                    (
                        output_index,
                        self.add_output_expr(
                            dependency_to_index,
                            buffer_dependency_to_index,
                            output_expr_to_index,
                            tex_coord_to_index,
                            &value,
                        ),
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
            normal_intensity: OptVarInt(p.normal_intensity.map(|i| {
                self.add_output_expr(
                    dependency_to_index,
                    buffer_dependency_to_index,
                    output_expr_to_index,
                    tex_coord_to_index,
                    &i,
                )
                .0
            })),
        }
    }

    fn add_output_expr(
        &mut self,
        dependency_to_index: &mut IndexSet<Dependency>,
        buffer_dependency_to_index: &mut IndexSet<BufferDependency>,
        output_expr_to_index: &mut IndexSet<OutputExpr>,
        tex_coord_to_index: &mut IndexSet<TexCoord>,
        value: &OutputExpr,
    ) -> VarInt {
        let index = match output_expr_to_index.get_index_of(value) {
            Some(index) => index,
            None => {
                let v = match &value {
                    OutputExpr::Value(d) => OutputExprIndexed::Value(self.add_dependency(
                        d.clone(),
                        dependency_to_index,
                        buffer_dependency_to_index,
                        tex_coord_to_index,
                    )),
                    OutputExpr::Func { op, args } => OutputExprIndexed::Func {
                        op: (*op).into(),
                        args: args
                            .iter()
                            .map(|a| {
                                self.add_output_expr(
                                    dependency_to_index,
                                    buffer_dependency_to_index,
                                    output_expr_to_index,
                                    tex_coord_to_index,
                                    a,
                                )
                            })
                            .collect(),
                    },
                };

                let index = self.output_exprs.len();

                self.output_exprs.push(v);
                output_expr_to_index.insert(value.clone());

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
                .map(|(output, value)| {
                    (
                        self.outputs[output.0].clone(),
                        self.output_expr_from_indexed(&self.output_exprs[value.0]),
                    )
                })
                .collect(),
            outline_width: p
                .outline_width
                .0
                .map(|i| self.dependency_from_indexed(&self.dependencies[i])),
            normal_intensity: p
                .normal_intensity
                .0
                .map(|i| self.output_expr_from_indexed(&self.output_exprs[i])),
        }
    }

    fn output_expr_from_indexed(&self, value: &OutputExprIndexed) -> OutputExpr {
        match value {
            OutputExprIndexed::Value(d) => {
                OutputExpr::Value(self.dependency_from_indexed(&self.dependencies[d.0]))
            }
            OutputExprIndexed::Func { op, args } => OutputExpr::Func {
                op: (*op).into(),
                args: args
                    .iter()
                    .map(|a| self.output_expr_from_indexed(&self.output_exprs[a.0]))
                    .collect(),
            },
        }
    }

    fn dependency_from_indexed(&self, d: &DependencyIndexed) -> Dependency {
        match d {
            DependencyIndexed::Constant(f) => Dependency::Constant((*f).into()),
            DependencyIndexed::Buffer(b) => Dependency::Buffer(
                self.buffer_dependency_from_indexed(&self.buffer_dependencies[b.0]),
            ),
            DependencyIndexed::Texture(t) => Dependency::Texture(TextureDependency {
                name: self.texture_names[t.name.0].clone(),
                channel: t.channel.into(),
                texcoords: t
                    .texcoords
                    .iter()
                    .map(|coord| self.tex_coord_from_indexed(&self.tex_coords[coord.0]))
                    .collect(),
            }),
            DependencyIndexed::Attribute(a) => Dependency::Attribute(AttributeDependency {
                name: self.attribute_names[a.name.0].clone(),
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
                name: add_string(&mut self.texture_names, t.name),
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
                name: add_string(&mut self.attribute_names, a.name),
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

    fn tex_coord_from_indexed(&self, coord: &TexCoordIndexed) -> TexCoord {
        TexCoord {
            name: self.attribute_names[coord.name.0].clone(),
            channel: coord.channel.into(),
            params: match coord.params {
                TexCoordParamsIndexed::None => None,
                TexCoordParamsIndexed::Scale(s) => Some(TexCoordParams::Scale(
                    self.buffer_dependency_from_indexed(&self.buffer_dependencies[s.0]),
                )),
                TexCoordParamsIndexed::Matrix(m) => {
                    Some(TexCoordParams::Matrix(m.map(|s| {
                        self.buffer_dependency_from_indexed(&self.buffer_dependencies[s.0])
                    })))
                }
                TexCoordParamsIndexed::Parallax {
                    mask_a,
                    mask_b,
                    ratio,
                } => Some(TexCoordParams::Parallax {
                    mask_a: self.dependency_from_indexed(&self.dependencies[mask_a.0]),
                    mask_b: self.dependency_from_indexed(&self.dependencies[mask_b.0]),
                    ratio: self.buffer_dependency_from_indexed(&self.buffer_dependencies[ratio.0]),
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
            name: add_string(&mut self.attribute_names, t.name),
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

    fn buffer_dependency_from_indexed(&self, b: &BufferDependencyIndexed) -> BufferDependency {
        BufferDependency {
            name: self.buffer_names[b.name.0].clone(),
            field: self.buffer_field_names[b.field.0].clone(),
            index: b.index.0,
            channel: b.channel.into(),
        }
    }

    fn buffer_dependency_indexed(&mut self, b: BufferDependency) -> BufferDependencyIndexed {
        BufferDependencyIndexed {
            name: add_string(&mut self.buffer_names, b.name),
            field: add_string(&mut self.buffer_field_names, b.field),
            index: OptVarInt(b.index),
            channel: b.channel.into(),
        }
    }
}

fn add_string(strings: &mut Vec<SmolStr>, str: SmolStr) -> VarInt {
    VarInt(strings.iter().position(|s| s == &str).unwrap_or_else(|| {
        let index = strings.len();
        strings.push(str);
        index
    }))
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

#[binrw::parser(reader, endian)]
fn parse_strings() -> BinResult<Vec<SmolStr>> {
    let count = VarInt::read_options(reader, endian, ())?.0;
    let strings =
        <Vec<NullString>>::read_options(reader, endian, binrw::VecArgs { count, inner: () })?;
    Ok(strings.into_iter().map(|s| s.to_smolstr()).collect())
}

#[binrw::writer(writer, endian)]
fn write_strings(value: &Vec<SmolStr>) -> BinResult<()> {
    VarInt(value.len()).write_options(writer, endian, ())?;
    for v in value {
        NullString::from(v.as_str()).write_options(writer, endian, ())?;
    }
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
