//! Database for compiled shader metadata for more accurate rendering.
//!
//! In game shaders are precompiled and embedded in files like `.wismt`.
//! These types represent precomputed metadata like assignments to G-Buffer textures.
//! This is necessary for determining the usage of a texture like albedo or normal map
//! since the assignments are compiled into the shader code itself.
//!
//! Shader database files should be generated using the xc3_shader CLI tool.
//! Applications can parse the data with [ShaderDatabase::from_file]
//! to avoid needing to generate this data at runtime.

use std::{collections::BTreeMap, path::Path};

use indexmap::IndexMap;
use ordered_float::OrderedFloat;
use smol_str::SmolStr;

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
    pub fn merge(&self, other: &Self) -> Self {
        Self(self.0.merge(&other.0))
    }
}

/// Unique identifier for compiled shader program data.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub struct ProgramHash(u32);

impl ProgramHash {
    /// Hash a legacy shader program.
    pub fn from_mths(mths: &xc3_lib::mths::Mths) -> Self {
        let mut hasher = crc32fast::Hasher::new();
        // TODO: Update metadata separately instead of entire buffer?
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
    /// A tree of values assigned to a fragment output.
    ///
    /// This assignment information is needed to accurately recreate the G-Buffer texture values.
    /// Renderers can generate unique shaders for each model like xc3_wgpu.
    /// Node based editors like Blender's shader editor should use these values
    /// to determine how to construct node groups.
    pub output_dependencies: IndexMap<SmolStr, OutputExpr>,

    /// The parameter multiplied by vertex alpha to determine outline width.
    pub outline_width: Option<Dependency>,

    /// The intensity map for normal mapping.
    pub normal_intensity: Option<OutputExpr>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum Dependency {
    Constant(OrderedFloat<f32>),
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
    /// Texture coordinate values used for the texture function call.
    pub texcoords: Vec<TexCoord>,
}

/// A texture coordinate attribute with optional transform parameters.
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct TexCoord {
    /// The name of the attribute like "in_attr4".
    pub name: SmolStr,
    /// The accessed channels like "x" or "y".
    pub channel: Option<char>,
    pub params: Option<TexCoordParams>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum TexCoordParams {
    // A single scale parameter.
    Scale(BufferDependency),

    /// A float2x4 texture matrix.
    /// ```text
    /// u = dot(vec4(u, v, 0.0, 1.0), gTexMat[0].xyzw);
    /// v = dot(vec4(u, v, 0.0, 1.0), gTexMat[1].xyzw);
    /// ```
    Matrix([BufferDependency; 4]),

    /// Masked parallax mapping with `mix(mask_a, mask_b, ratio)` as the intensity.
    Parallax {
        mask_a: Dependency,
        mask_b: Dependency,
        ratio: BufferDependency,
    },
}

/// A single input attribute like `in_attr0.x` in GLSL.
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct AttributeDependency {
    pub name: SmolStr,
    pub channel: Option<char>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum Operation {
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
    /// Normal blend mode similar to "Reoriented Normal Mapping" (RNM).
    AddNormal,
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
    Unk,
}

impl Default for Operation {
    fn default() -> Self {
        Self::Mix
    }
}

// TODO: replace layer and layervalue with this
/// A tree of computations with [Dependency] for the leaf values.
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum OutputExpr {
    Value(Dependency),
    // TODO: is it worth having separate unary, binary, etc ops
    Func {
        op: Operation,
        args: Vec<OutputExpr>,
    },
}

impl Default for OutputExpr {
    fn default() -> Self {
        // TODO: Create a special value for unsupported values?
        Self::Func {
            op: Operation::Unk,
            args: Vec::new(),
        }
    }
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
impl<'a> arbitrary::Arbitrary<'a> for TexCoord {
    fn arbitrary(u: &mut arbitrary::Unstructured) -> arbitrary::Result<Self> {
        Ok(Self {
            name: crate::arbitrary_smolstr(u)?,
            channel: u.arbitrary()?,
            params: u.arbitrary()?,
        })
    }
}

#[cfg(feature = "arbitrary")]
impl<'a> arbitrary::Arbitrary<'a> for ShaderProgram {
    fn arbitrary(u: &mut arbitrary::Unstructured) -> arbitrary::Result<Self> {
        let output_dependencies: Vec<(String, OutputDependencies)> = u.arbitrary()?;
        Ok(Self {
            output_dependencies: output_dependencies
                .into_iter()
                .map(|(k, v)| (k.into(), v))
                .collect(),
            outline_width: u.arbitrary()?,
        })
    }
}
