//! Database for compiled shader metadata for more accurate rendering.
//!
//! In game shaders are precompiled and embedded in files like `.wismt`.
//! These types represent compiled shader instructions as a graph for
//! to make it easier to generate shader code or material nodes in applications.
//!
//! Shader database files should be generated using the xc3_shader CLI tool.
//! Applications can parse the data with [ShaderDatabase::from_file]
//! to avoid needing to generate this data at runtime.

use std::{collections::BTreeMap, path::Path};

use indexmap::IndexMap;
use ordered_float::OrderedFloat;
use smol_str::SmolStr;
use strum::{Display, FromRepr};

use crate::error::{LoadShaderDatabaseError, SaveShaderDatabaseError};

mod io;

/// Metadata for the assigned shaders for all models and maps in a game dump.
#[derive(Debug, PartialEq, Clone)]
pub struct ShaderDatabase(io::ShaderDatabaseIndexed);

impl ShaderDatabase {
    /// Load the database data from `path`.
    #[tracing::instrument(skip_all)]
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, LoadShaderDatabaseError> {
        // Avoid converting the indexed database to improve load times.
        // Most uses cases will only need data for a single model or map.
        let indexed = io::ShaderDatabaseIndexed::from_file(path)?;
        Ok(Self(indexed))
    }

    /// Serialize and save the database data to `path`.
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), SaveShaderDatabaseError> {
        self.0.save(path)?;
        Ok(())
    }

    /// The shader information for the specified shader program.
    pub fn shader_program(&self, hash: ProgramHash) -> Option<ShaderProgram> {
        self.0.shader_program(hash)
    }

    /// Create the internal database representation from non indexed data.
    pub fn from_programs(programs: BTreeMap<ProgramHash, ShaderProgram>) -> Self {
        Self(io::ShaderDatabaseIndexed::from_programs(programs))
    }

    /// Create a new database with combined entries from `other`.
    pub fn merge(self, others: impl Iterator<Item = Self>) -> Self {
        Self(self.0.merge(others.into_iter().map(|o| o.0)))
    }
}

/// Unique identifier for compiled shader program data.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub struct ProgramHash(u32);

impl ProgramHash {
    /// Hash a legacy shader program.
    pub fn from_mths(mths: &xc3_lib::mths::Mths) -> Self {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&mths.data);
        Self(hasher.finalize())
    }

    /// Hash a shader program.
    pub fn from_spch_program(
        program: &xc3_lib::spch::ShaderProgram,
        vertex: &Option<xc3_lib::spch::ShaderBinary>,
        fragment: &Option<xc3_lib::spch::ShaderBinary>,
    ) -> Self {
        // Hash both code and metadata since programs with the same code
        // can have slightly different uniforms, buffers, etc.
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&program.program_data);

        if let Some(fragment) = fragment {
            hasher.update(&fragment.program_binary);
        }
        if let Some(vertex) = vertex {
            hasher.update(&vertex.program_binary);
        }

        Self(hasher.finalize())
    }
}

/// A single shader program with a vertex and fragment shader.
#[derive(Debug, PartialEq, Clone, Default)]
pub struct ShaderProgram {
    /// Indices into [exprs](#structfield.exprs) for values assigned to a fragment output.
    ///
    /// This assignment information is needed to accurately recreate the G-Buffer texture values.
    /// Renderers can generate unique shaders for each model like xc3_wgpu.
    /// Node based editors like Blender's shader editor should use these values
    /// to determine how to construct node groups.
    pub output_dependencies: IndexMap<SmolStr, usize>,

    // TODO: Index into exprs as well
    /// The parameter multiplied by vertex alpha to determine outline width.
    pub outline_width: Option<Dependency>,

    /// Index into [exprs](#structfield.exprs) for the normal map intensity.
    pub normal_intensity: Option<usize>,

    /// Unique exprs used for this program.
    pub exprs: Vec<OutputExpr>,
}

/// A single access to a constant or global resource like a texture.
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum Dependency {
    Int(i32),
    Float(OrderedFloat<f32>),
    Buffer(BufferDependency),
    Texture(TextureDependency),
    Attribute(AttributeDependency),
}

/// A single buffer access like `UniformBuffer.field[0].y` or `UniformBuffer.field.y` in GLSL.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
pub struct BufferDependency {
    pub name: SmolStr,
    pub field: SmolStr,
    pub index: Option<usize>,
    pub channel: Option<char>,
}

/// A single texture access like `texture(s0, tex0.xy).rgb` in GLSL.
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct TextureDependency {
    pub name: SmolStr,
    pub channel: Option<char>,
    /// Indices into [exprs](struct.ShaderProgram.html#structfield.exprs)
    /// for texture coordinate values used for the texture function call.
    pub texcoords: Vec<usize>,
}

/// A single input attribute like `in_attr0.x` in GLSL.
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct AttributeDependency {
    pub name: SmolStr,
    pub channel: Option<char>,
}

/// A function or operation applied to one or more [OutputExpr].
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, Display, FromRepr)]
pub enum Operation {
    /// An unsupported operation or function call.
    Unk,
    /// `mix(arg0, arg1, arg2)`
    Mix,
    /// `arg0 * arg1`
    Mul,
    /// `arg0 / arg1`
    Div,
    /// `arg0 + arg1`
    Add,
    /// `arg0 - arg1`
    Sub,
    /// `fma(arg0, arg1, arg2)` or `arg0 * arg1 + arg2`
    Fma,
    /// `mix(arg0, arg0 * arg1, arg2)`
    MulRatio,
    /// Blend mode `add_normal(n1_x, n1_y, n2_x, n2_y, ratio).x` similar to "Reoriented Normal Mapping" (RNM).
    AddNormalX,
    /// Blend mode `add_normal(n1_x, n1_y, n2_x, n2_y, ratio).y` similar to "Reoriented Normal Mapping" (RNM).
    AddNormalY,
    /// `overlay(arg0, arg1)`.
    Overlay,
    /// `overlay2(arg0, arg1)`.
    Overlay2,
    /// `mix(arg0, overlay(arg0, arg1), arg2)`.
    OverlayRatio,
    /// `pow(arg0, arg1)`
    Power,
    /// `min(arg0, arg1)`
    Min,
    /// `max(arg0, arg1)`
    Max,
    /// `clamp(arg0, arg1, arg2)`
    Clamp,
    /// `abs(arg0)`
    Abs,
    /// `pow(1.0 - n_dot_v, arg0 * 5.0)`
    Fresnel,
    /// `sqrt(arg0)`
    Sqrt,
    /// `dot(vec4(arg0, arg1, 0.0, 1.0), (arg2, arg3, arg4, arg5))`
    TexMatrix,
    /// `arg0 + arg1 * 0.7 * (normal.x * tangent.x - normal.x * bitangent.x)`
    TexParallaxX,
    /// `arg0 + arg1 * 0.7 * (normal.x * tangent.y - normal.x * bitangent.y)`
    TexParallaxY,
    /// `reflect(vec3(arg0, arg1, arg2), vec3(arg3, arg4, arg5)).x`
    ReflectX,
    /// `reflect(vec3(arg0, arg1, arg2), vec3(arg3, arg4, arg5)).y`
    ReflectY,
    /// `reflect(vec3(arg0, arg1, arg2), vec3(arg3, arg4, arg5)).z`
    ReflectZ,
    /// `floor(arg0)`
    Floor,
    /// `if arg0 { arg1 } else { arg2 }` or `mix(arg2, arg1, arg0)`
    Select,
    /// `arg0 == arg1`
    Equal,
    /// `arg0 != arg1`
    NotEqual,
    /// `arg0 < arg1`
    Less,
    /// `arg0 > arg1`
    Greater,
    /// `arg0 <= arg1`
    LessEqual,
    /// `arg0 >= arg1`
    GreaterEqual,
    /// `dot(vec4(arg0, arg1, arg2, arg3), vec4(arg4, arg5, arg6, arg7))`
    Dot4,
    /// `apply_normal_map(create_normal_map(arg0, arg1), tangent.xyz, bitangent.xyz, normal.xyz).x`
    NormalMapX,
    /// `apply_normal_map(create_normal_map(arg0, arg1), tangent.xyz, bitangent.xyz, normal.xyz).y`
    NormalMapY,
    /// `apply_normal_map(create_normal_map(arg0, arg1), tangent.xyz, bitangent.xyz, normal.xyz).z`
    NormalMapZ,
    /// `monochrome(arg0, arg1, arg2, arg3).x`
    MonochromeX,
    /// `monochrome(arg0, arg1, arg2, arg3).y`
    MonochromeY,
    /// `monochrome(arg0, arg1, arg2, arg3).z`
    MonochromeZ,
    /// `-arg0`
    Negate,
}

impl Default for Operation {
    fn default() -> Self {
        Self::Unk
    }
}

/// A tree of computations with [Dependency] for the leaf values.
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum OutputExpr {
    /// A single value.
    Value(Dependency),
    /// An operation applied to one or more [OutputExpr].
    Func {
        /// The operation this function performs.
        op: Operation,
        /// Indices into [exprs](struct.ShaderProgram.html#structfield.exprs) for the function argument list `[arg0, arg1, ...]`.
        args: Vec<usize>,
    },
}

impl Default for OutputExpr {
    fn default() -> Self {
        Self::Func {
            op: Operation::Unk,
            args: Vec::new(),
        }
    }
}

impl std::fmt::Display for OutputExpr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputExpr::Value(d) => write!(f, "{d}"),
            OutputExpr::Func { op, args } => {
                let args: Vec<_> = args.iter().map(|a| format!("var{a}")).collect();
                write!(f, "{op}({})", args.join(", "))
            }
        }
    }
}

impl std::fmt::Display for AttributeDependency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", &self.name, channels(self.channel))
    }
}

impl std::fmt::Display for BufferDependency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}{}{}{}",
            &self.name,
            if self.field.is_empty() {
                String::new()
            } else {
                format!(".{}", &self.field)
            },
            self.index.map(|i| format!("[{i}]")).unwrap_or_default(),
            channels(self.channel)
        )
    }
}

impl std::fmt::Display for TextureDependency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let args: Vec<_> = self.texcoords.iter().map(|t| format!("var{t}")).collect();
        write!(
            f,
            "Texture({}, {}){}",
            &self.name,
            args.join(", "),
            channels(self.channel)
        )
    }
}

impl std::fmt::Display for Dependency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Dependency::Int(i) => write!(f, "{i:?}"),
            Dependency::Float(c) => write!(f, "{c:?}"),
            Dependency::Buffer(b) => write!(f, "{b}"),
            Dependency::Texture(t) => write!(f, "{t}"),
            Dependency::Attribute(a) => write!(f, "{a}"),
        }
    }
}

fn channels(c: Option<char>) -> String {
    c.map(|c| format!(".{c}")).unwrap_or_default()
}

#[cfg(feature = "arbitrary")]
impl<'a> arbitrary::Arbitrary<'a> for AttributeDependency {
    fn arbitrary(u: &mut arbitrary::Unstructured) -> arbitrary::Result<Self> {
        Ok(Self {
            name: crate::arbitrary_smolstr(u)?,
            channel: u.arbitrary()?,
        })
    }
}

#[cfg(feature = "arbitrary")]
impl<'a> arbitrary::Arbitrary<'a> for BufferDependency {
    fn arbitrary(u: &mut arbitrary::Unstructured) -> arbitrary::Result<Self> {
        Ok(Self {
            name: crate::arbitrary_smolstr(u)?,
            field: crate::arbitrary_smolstr(u)?,
            index: u.arbitrary()?,
            channel: u.arbitrary()?,
        })
    }
}

#[cfg(feature = "arbitrary")]
impl<'a> arbitrary::Arbitrary<'a> for TextureDependency {
    fn arbitrary(u: &mut arbitrary::Unstructured) -> arbitrary::Result<Self> {
        Ok(Self {
            name: crate::arbitrary_smolstr(u)?,
            channel: u.arbitrary()?,
            texcoords: u.arbitrary()?,
        })
    }
}

#[cfg(feature = "arbitrary")]
impl<'a> arbitrary::Arbitrary<'a> for ShaderProgram {
    fn arbitrary(u: &mut arbitrary::Unstructured) -> arbitrary::Result<Self> {
        let output_dependencies: Vec<(String, usize)> = u.arbitrary()?;
        Ok(Self {
            output_dependencies: output_dependencies
                .into_iter()
                .map(|(k, v)| (k.into(), v))
                .collect(),
            outline_width: u.arbitrary()?,
            normal_intensity: u.arbitrary()?,
            exprs: u.arbitrary()?,
        })
    }
}
