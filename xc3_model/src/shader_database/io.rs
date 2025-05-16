use std::{collections::BTreeMap, io::Cursor, path::Path};

use binrw::{binrw, BinRead, BinReaderExt, BinResult, BinWrite, BinWriterExt, NullString};
use ordered_float::OrderedFloat;
use smol_str::{SmolStr, ToSmolStr};
use varint_rs::{VarintReader, VarintWriter};

use super::{
    AttributeDependency, BufferDependency, Dependency, Operation, OutputExpr, ProgramHash,
    ShaderProgram, TextureDependency,
};

// Faster than the default hash implementation.
type IndexSet<T> = indexmap::IndexSet<T, ahash::RandomState>;

// Create a separate format optimized for storing on disk.
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
    // Use an ordered map for consistent ordering.
    #[br(parse_with = parse_map32)]
    #[bw(write_with = write_map32)]
    programs: BTreeMap<u32, ShaderProgramIndexed>,

    #[br(parse_with = parse_set)]
    #[bw(write_with = write_set)]
    dependencies: IndexSet<DependencyIndexed>,

    #[br(parse_with = parse_set)]
    #[bw(write_with = write_set)]
    buffer_dependencies: IndexSet<BufferDependencyIndexed>,

    #[br(parse_with = parse_set)]
    #[bw(write_with = write_set)]
    output_exprs: IndexSet<OutputExprIndexed>,

    // Storing multiple string lists enables 8-bit instead of 16-bit indices.
    #[br(parse_with = parse_strings)]
    #[bw(write_with = write_strings)]
    attribute_names: IndexSet<SmolStr>,

    #[br(parse_with = parse_strings)]
    #[bw(write_with = write_strings)]
    buffer_names: IndexSet<SmolStr>,

    #[br(parse_with = parse_strings)]
    #[bw(write_with = write_strings)]
    buffer_field_names: IndexSet<SmolStr>,

    #[br(parse_with = parse_strings)]
    #[bw(write_with = write_strings)]
    texture_names: IndexSet<SmolStr>,

    #[br(parse_with = parse_strings)]
    #[bw(write_with = write_strings)]
    outputs: IndexSet<SmolStr>,
}

#[binrw]
#[derive(Debug, PartialEq, Clone)]
struct MapIndexed {
    #[br(parse_with = parse_vec)]
    #[bw(write_with = write_vec)]
    map_models: Vec<ModelIndexed>,

    #[br(parse_with = parse_vec)]
    #[bw(write_with = write_vec)]
    prop_models: Vec<ModelIndexed>,

    #[br(parse_with = parse_vec)]
    #[bw(write_with = write_vec)]
    env_models: Vec<ModelIndexed>,
}

#[binrw]
#[derive(Debug, PartialEq, Clone)]
struct ModelIndexed {
    #[br(parse_with = parse_vec)]
    #[bw(write_with = write_vec)]
    programs: Vec<ShaderProgramIndexed>,
}

#[binrw]
#[derive(Debug, PartialEq, Clone)]
struct ShaderProgramIndexed {
    // There are very few unique dependencies across all shaders in a game dump.
    // Normalize the data to greatly reduce the size file size.
    #[br(parse_with = parse_vec)]
    #[bw(write_with = write_vec)]
    output_dependencies: Vec<(VarInt, VarInt)>,

    outline_width: OptVarInt,
    normal_intensity: OptVarInt,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, BinRead, BinWrite)]
enum OutputExprIndexed {
    #[brw(magic(0u8))]
    Value(VarInt),

    #[brw(magic(1u8))]
    Func {
        op: OperationIndexed,

        #[br(parse_with = parse_vec)]
        #[bw(write_with = write_vec)]
        args: Vec<VarInt>,
    },
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, BinRead, BinWrite)]
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
    Sqrt = 18,
    TexMatrix = 19,
    TexParallax = 20,
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
            Operation::Sqrt => Self::Sqrt,
            Operation::TexMatrix => Self::TexMatrix,
            Operation::TexParallax => Self::TexParallax,
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
            OperationIndexed::Sqrt => Self::Sqrt,
            OperationIndexed::TexMatrix => Self::TexMatrix,
            OperationIndexed::TexParallax => Self::TexParallax,
            OperationIndexed::Unk => Self::Unk,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, BinRead, BinWrite)]
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

#[derive(Debug, PartialEq, Eq, Hash, Clone, BinRead, BinWrite)]
enum DependencyIndexed {
    #[brw(magic(0u8))]
    Constant(
        #[br(map(|f: f32| f.into()))]
        #[bw(map(|f| f.0))]
        OrderedFloat<f32>,
    ),

    #[brw(magic(1u8))]
    Buffer(VarInt),

    #[brw(magic(2u8))]
    Texture(TextureDependencyIndexed),

    #[brw(magic(3u8))]
    Attribute(AttributeDependencyIndexed),
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, BinRead, BinWrite)]
struct BufferDependencyIndexed {
    name: VarInt,
    field: VarInt,
    index: OptVarInt,
    channel: Channel,
}

#[binrw]
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
struct TextureDependencyIndexed {
    name: VarInt,
    channel: Channel,

    #[br(parse_with = parse_vec)]
    #[bw(write_with = write_vec)]
    texcoords: Vec<VarInt>,
}

#[binrw]
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
struct TexCoordIndexed {
    name: VarInt,
    channel: Channel,
    params: TexCoordParamsIndexed,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, BinRead, BinWrite)]
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

#[derive(Debug, PartialEq, Eq, Hash, Clone, BinRead, BinWrite)]
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
        let mut database = Self::default();

        for (hash, p) in programs.into_iter() {
            let program = database.program_indexed(p);
            database.programs.insert(hash.0, program);
        }

        database
    }

    pub fn merge(self, others: impl Iterator<Item = Self>) -> Self {
        // Reuse existing indices when merging.
        let mut merged = self;

        // Reindex all programs.
        for other in others {
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
                    merged.add_dependency(d).0
                })
                .collect();

            // Remap indexed output exprs to reduce costly indexing and large allocations.
            let output_expr_indices: Vec<_> = other
                .output_exprs
                .iter()
                .map(|e| {
                    merged.add_output_expr_indexed(e, &other.output_exprs, &dependency_indices)
                })
                .collect();

            for (hash, program) in other.programs {
                let mut program = program;
                for (k, v) in &mut program.output_dependencies {
                    *k = output_indices[k.0];
                    *v = output_expr_indices[v.0];
                }
                merged.programs.insert(hash, program);
            }
        }

        merged
    }

    fn program_indexed(&mut self, p: ShaderProgram) -> ShaderProgramIndexed {
        ShaderProgramIndexed {
            output_dependencies: p
                .output_dependencies
                .into_iter()
                .map(|(output, value)| {
                    let output_index = add_string(&mut self.outputs, output);
                    (output_index, self.add_output_expr(&value))
                })
                .collect(),
            outline_width: OptVarInt(p.outline_width.map(|d| self.add_dependency(d).0)),
            normal_intensity: OptVarInt(p.normal_intensity.map(|i| self.add_output_expr(&i).0)),
        }
    }

    fn add_output_expr(&mut self, value: &OutputExpr) -> VarInt {
        // Insert values that this value depends on first.
        let v = match &value {
            OutputExpr::Value(d) => OutputExprIndexed::Value(self.add_dependency(d.clone())),
            OutputExpr::Func { op, args } => OutputExprIndexed::Func {
                op: (*op).into(),
                args: args.iter().map(|a| self.add_output_expr(a)).collect(),
            },
        };

        let (index, _) = self.output_exprs.insert_full(v);

        VarInt(index)
    }

    fn add_output_expr_indexed(
        &mut self,
        value: &OutputExprIndexed,
        values: &IndexSet<OutputExprIndexed>,
        dependency_indices: &[usize],
    ) -> VarInt {
        // Insert values that this value depends on first.
        let new_expr = match value {
            OutputExprIndexed::Value(d) => {
                OutputExprIndexed::Value(VarInt(dependency_indices[d.0]))
            }
            OutputExprIndexed::Func { op, args } => OutputExprIndexed::Func {
                op: *op,
                args: args
                    .iter()
                    .map(|a| self.add_output_expr_indexed(&values[a.0], values, dependency_indices))
                    .collect(),
            },
        };
        let (index, _) = self.output_exprs.insert_full(new_expr);
        VarInt(index)
    }

    fn add_dependency(&mut self, d: Dependency) -> VarInt {
        let dependency = self.dependency_indexed(d);
        let (index, _) = self.dependencies.insert_full(dependency);

        VarInt(index)
    }

    fn add_buffer_dependency(&mut self, b: BufferDependency) -> VarInt {
        let dependency = self.buffer_dependency_indexed(b);
        let (index, _) = self.buffer_dependencies.insert_full(dependency);

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
            DependencyIndexed::Constant(f) => Dependency::Constant(*f),
            DependencyIndexed::Buffer(b) => Dependency::Buffer(
                self.buffer_dependency_from_indexed(&self.buffer_dependencies[b.0]),
            ),
            DependencyIndexed::Texture(t) => Dependency::Texture(TextureDependency {
                name: self.texture_names[t.name.0].clone(),
                channel: t.channel.into(),
                texcoords: t
                    .texcoords
                    .iter()
                    .map(|coord| self.output_expr_from_indexed(&self.output_exprs[coord.0]))
                    .collect(),
            }),
            DependencyIndexed::Attribute(a) => Dependency::Attribute(AttributeDependency {
                name: self.attribute_names[a.name.0].clone(),
                channel: a.channel.into(),
            }),
        }
    }

    fn dependency_indexed(&mut self, d: Dependency) -> DependencyIndexed {
        match d {
            Dependency::Constant(c) => DependencyIndexed::Constant(c),
            Dependency::Buffer(b) => DependencyIndexed::Buffer(self.add_buffer_dependency(b)),
            Dependency::Texture(t) => DependencyIndexed::Texture(TextureDependencyIndexed {
                name: add_string(&mut self.texture_names, t.name),
                channel: t.channel.into(),
                texcoords: t
                    .texcoords
                    .iter()
                    .map(|t| self.add_output_expr(t))
                    .collect(),
            }),
            Dependency::Attribute(a) => DependencyIndexed::Attribute(AttributeDependencyIndexed {
                name: add_string(&mut self.attribute_names, a.name),
                channel: a.channel.into(),
            }),
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

fn add_string(strings: &mut IndexSet<SmolStr>, str: SmolStr) -> VarInt {
    VarInt(strings.insert_full(str).0)
}

// Variable length ints are slightly slower to parse but take up much less space.
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
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

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
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
fn parse_vec<T>() -> BinResult<Vec<T>>
where
    for<'a> T: BinRead<Args<'a> = ()> + 'static,
{
    let count = VarInt::read_options(reader, endian, ())?.0;
    <Vec<T>>::read_options(reader, endian, binrw::VecArgs { count, inner: () })
}

#[binrw::writer(writer, endian)]
fn write_vec<T>(value: &Vec<T>) -> BinResult<()>
where
    for<'a> T: BinWrite<Args<'a> = ()> + 'static,
{
    VarInt(value.len()).write_options(writer, endian, ())?;
    value.write_options(writer, endian, ())?;
    Ok(())
}

#[binrw::parser(reader, endian)]
fn parse_set<T>() -> BinResult<IndexSet<T>>
where
    T: std::hash::Hash + Eq,
    for<'a> T: BinRead<Args<'a> = ()> + 'static,
{
    let count = VarInt::read_options(reader, endian, ())?.0;
    let mut values = IndexSet::default();
    for _ in 0..count {
        let value = T::read_options(reader, endian, ())?;
        values.insert(value);
    }
    Ok(values)
}

#[binrw::writer(writer, endian)]
fn write_set<T>(values: &IndexSet<T>) -> BinResult<()>
where
    for<'a> T: BinWrite<Args<'a> = ()> + 'static,
{
    VarInt(values.len()).write_options(writer, endian, ())?;
    for v in values {
        v.write_options(writer, endian, ())?;
    }
    Ok(())
}

#[binrw::parser(reader, endian)]
fn parse_strings() -> BinResult<IndexSet<SmolStr>> {
    let count = VarInt::read_options(reader, endian, ())?.0;
    let mut values = IndexSet::default();
    for _ in 0..count {
        let s = NullString::read_options(reader, endian, ())?;
        values.insert(s.to_smolstr());
    }
    Ok(values)
}

#[binrw::writer(writer, endian)]
fn write_strings(value: &IndexSet<SmolStr>) -> BinResult<()> {
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
