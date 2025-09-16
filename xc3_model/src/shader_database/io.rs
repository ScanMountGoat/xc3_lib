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
type IndexMap<K, V> = indexmap::IndexMap<K, V, ahash::RandomState>;

// Create a separate format optimized for storing on disk.
#[binrw]
#[derive(Debug, PartialEq, Clone, Default)]
#[brw(magic(b"SHDB"))]
pub struct ShaderDatabaseIndexed {
    // File version numbers should be updated with each release.
    // This improves the error when parsing an incompatible version.
    #[br(assert(version == 4))]
    #[bw(calc = 4)]
    version: u32,

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
        // TODO: Avoid unwrap
        #[br(map(|x: u8| Operation::from_repr(x as usize).unwrap()))]
        #[bw(map(|x| *x as u8))]
        op: Operation,

        #[br(parse_with = parse_vec)]
        #[bw(write_with = write_vec)]
        args: Vec<VarInt>,
    },
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
    Float(
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

    #[brw(magic(4u8))]
    Int(i32),
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
            for (hash, program) in &other.programs {
                let program = other.program_from_indexed(program);
                let indexed = merged.program_indexed(program);
                merged.programs.insert(*hash, indexed);
            }
        }

        merged
    }

    fn program_indexed(&mut self, p: ShaderProgram) -> ShaderProgramIndexed {
        // Remap exprs indexed for this program to exprs indexed for all programs.
        let mut expr_indices = IndexMap::default();

        ShaderProgramIndexed {
            output_dependencies: p
                .output_dependencies
                .into_iter()
                .map(|(output, value)| {
                    let output_index = add_string(&mut self.outputs, output);
                    (
                        output_index,
                        self.add_output_expr(&p.exprs[value], &p.exprs, &mut expr_indices),
                    )
                })
                .collect(),
            outline_width: OptVarInt(
                p.outline_width
                    .map(|d| self.add_dependency(d, &p.exprs, &mut expr_indices).0),
            ),
            normal_intensity: OptVarInt(p.normal_intensity.map(|i| {
                self.add_output_expr(&p.exprs[i], &p.exprs, &mut expr_indices)
                    .0
            })),
        }
    }

    fn add_output_expr<'a>(
        &mut self,
        value: &'a OutputExpr,
        exprs: &'a [OutputExpr],
        expr_indices: &mut IndexMap<&'a OutputExpr, VarInt>,
    ) -> VarInt {
        match expr_indices.get(value) {
            Some(i) => *i,
            None => {
                // Insert values that this value depends on first.
                let v = match &value {
                    OutputExpr::Value(d) => OutputExprIndexed::Value(self.add_dependency(
                        d.clone(),
                        exprs,
                        expr_indices,
                    )),
                    OutputExpr::Func { op, args } => OutputExprIndexed::Func {
                        op: *op,
                        args: args
                            .iter()
                            .map(|a| self.add_output_expr(&exprs[*a], exprs, expr_indices))
                            .collect(),
                    },
                };

                let (index, _) = self.output_exprs.insert_full(v);
                expr_indices.insert(value, VarInt(index));

                VarInt(index)
            }
        }
    }

    fn add_dependency<'a>(
        &mut self,
        d: Dependency,
        exprs: &'a [OutputExpr],
        expr_indices: &mut IndexMap<&'a OutputExpr, VarInt>,
    ) -> VarInt {
        let dependency = self.dependency_indexed(d, exprs, expr_indices);
        let (index, _) = self.dependencies.insert_full(dependency);

        VarInt(index)
    }

    fn add_buffer_dependency(&mut self, b: BufferDependency) -> VarInt {
        let dependency = self.buffer_dependency_indexed(b);
        let (index, _) = self.buffer_dependencies.insert_full(dependency);

        VarInt(index)
    }

    fn program_from_indexed(&self, p: &ShaderProgramIndexed) -> ShaderProgram {
        // Remap exprs indexed for all programs to exprs indexed for this program.
        let mut exprs = IndexSet::default();
        let mut expr_to_index = IndexMap::default();

        ShaderProgram {
            output_dependencies: p
                .output_dependencies
                .iter()
                .map(|(output, value)| {
                    (
                        self.outputs[output.0].clone(),
                        self.output_expr_from_indexed(value.0, &mut exprs, &mut expr_to_index),
                    )
                })
                .collect(),
            outline_width: p.outline_width.0.map(|i| {
                self.dependency_from_indexed(&self.dependencies[i], &mut exprs, &mut expr_to_index)
            }),
            normal_intensity: p
                .normal_intensity
                .0
                .map(|i| self.output_expr_from_indexed(i, &mut exprs, &mut expr_to_index)),
            exprs: exprs.into_iter().collect(),
        }
    }

    fn output_expr_from_indexed(
        &self,
        value: usize,
        exprs: &mut IndexSet<OutputExpr>,
        expr_to_index: &mut IndexMap<usize, usize>,
    ) -> usize {
        match expr_to_index.get(&value) {
            Some(i) => *i,
            None => {
                let expr = match &self.output_exprs[value] {
                    OutputExprIndexed::Value(d) => OutputExpr::Value(self.dependency_from_indexed(
                        &self.dependencies[d.0],
                        exprs,
                        expr_to_index,
                    )),
                    OutputExprIndexed::Func { op, args } => OutputExpr::Func {
                        op: *op,
                        args: args
                            .iter()
                            .map(|a| self.output_expr_from_indexed(a.0, exprs, expr_to_index))
                            .collect(),
                    },
                };
                let index = exprs.insert_full(expr).0;
                expr_to_index.insert(value, index);
                index
            }
        }
    }

    fn dependency_from_indexed(
        &self,
        d: &DependencyIndexed,
        exprs: &mut IndexSet<OutputExpr>,
        expr_to_index: &mut IndexMap<usize, usize>,
    ) -> Dependency {
        match d {
            DependencyIndexed::Int(i) => Dependency::Int(*i),
            DependencyIndexed::Float(f) => Dependency::Float(*f),
            DependencyIndexed::Buffer(b) => Dependency::Buffer(
                self.buffer_dependency_from_indexed(&self.buffer_dependencies[b.0]),
            ),
            DependencyIndexed::Texture(t) => Dependency::Texture(TextureDependency {
                name: self.texture_names[t.name.0].clone(),
                channel: t.channel.into(),
                texcoords: t
                    .texcoords
                    .iter()
                    .map(|coord| self.output_expr_from_indexed(coord.0, exprs, expr_to_index))
                    .collect(),
            }),
            DependencyIndexed::Attribute(a) => Dependency::Attribute(AttributeDependency {
                name: self.attribute_names[a.name.0].clone(),
                channel: a.channel.into(),
            }),
        }
    }

    fn dependency_indexed<'a>(
        &mut self,
        d: Dependency,
        exprs: &'a [OutputExpr],
        expr_indices: &mut IndexMap<&'a OutputExpr, VarInt>,
    ) -> DependencyIndexed {
        match d {
            Dependency::Int(i) => DependencyIndexed::Int(i),
            Dependency::Float(c) => DependencyIndexed::Float(c),
            Dependency::Buffer(b) => DependencyIndexed::Buffer(self.add_buffer_dependency(b)),
            Dependency::Texture(t) => DependencyIndexed::Texture(TextureDependencyIndexed {
                name: add_string(&mut self.texture_names, t.name),
                channel: t.channel.into(),
                texcoords: t
                    .texcoords
                    .iter()
                    .map(|t| self.add_output_expr(&exprs[*t], exprs, expr_indices))
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
