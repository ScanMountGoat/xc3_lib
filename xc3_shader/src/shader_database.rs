use std::{ops::Deref, path::Path};

use bimap::BiBTreeMap;
use glsl_lang::{
    ast::{
        ExprData, LayoutQualifierSpecData, SingleDeclaration, StorageQualifierData,
        TranslationUnit, TypeQualifierSpecData,
    },
    parse::DefaultParse,
    visitor::{Host, Visit, Visitor},
};
use log::error;
use rayon::prelude::*;
use xc3_lib::mths::{FragmentShader, Mths};
use xc3_model::shader_database::{
    BufferDependency, Dependency, MapPrograms, ModelPrograms, ShaderDatabase, ShaderProgram,
};

use crate::{
    annotation::shader_source_no_extensions,
    dependencies::{
        attribute_dependencies, buffer_dependency, input_dependencies, texcoord_params,
    },
    graph::{
        query::{assign_x, clamp_x_zero_one, one_minus_x, one_plus_x, sqrt_x},
        Expr, Graph,
    },
};

fn shader_from_glsl(vertex: Option<&TranslationUnit>, fragment: &TranslationUnit) -> ShaderProgram {
    let frag = &Graph::from_glsl(fragment);
    let frag_attributes = &find_attribute_locations(fragment);

    let vertex = &vertex.map(|v| (Graph::from_glsl(v), find_attribute_locations(v)));

    let output_dependencies = (0..=5)
        .flat_map(|i| {
            "xyzw".chars().map(move |c| {
                let name = format!("out_attr{i}");
                let mut dependencies = input_dependencies(frag, frag_attributes, &name, Some(c));

                if let Some((vert, vert_attributes)) = vertex {
                    // Add texture parameters used for the corresponding vertex output.
                    // Most shaders apply UV transforms in the vertex shader.
                    apply_vertex_texcoord_params(
                        vert,
                        vert_attributes,
                        frag_attributes,
                        &mut dependencies,
                    );

                    apply_attribute_names(
                        vert,
                        vert_attributes,
                        frag_attributes,
                        &mut dependencies,
                    );
                }

                if i == 1 && c == 'y' {
                    if let Some(param) = geometric_specular_aa(frag) {
                        dependencies = vec![Dependency::Buffer(param)];
                    }
                }

                // Simplify the output name to save space.
                let output_name = format!("o{i}.{c}");
                (output_name.into(), dependencies)
            })
        })
        .filter(|(_, dependencies)| !dependencies.is_empty())
        .collect();

    ShaderProgram {
        // IndexMap gives consistent ordering for attribute names.
        output_dependencies,
    }
}

fn shader_from_latte_asm(
    vertex: &str,
    fragment: &str,
    fragment_shader: &FragmentShader,
) -> ShaderProgram {
    let frag = &Graph::from_latte_asm(fragment);
    let frag_attributes = &Attributes::default();

    // TODO: Fix vertex parsing errors.
    // let vert = &Graph::from_latte_asm(vertex);
    // let vert_attributes = &Attributes::default();

    // TODO: What is the largest number of outputs?
    let output_dependencies = (0..=5)
        .flat_map(|i| {
            "xyzw".chars().map(move |c| {
                let name = format!("PIX{i}");

                let mut dependencies = input_dependencies(frag, frag_attributes, &name, Some(c));

                // Add texture parameters used for the corresponding vertex output.
                // Most shaders apply UV transforms in the vertex shader.
                // apply_vertex_texcoord_params(
                //     vert,
                //     vert_attributes,
                //     frag_attributes,
                //     &mut dependencies,
                // );

                // apply_attribute_names(vert, vert_attributes, frag_attributes, &mut dependencies);

                // Apply annotations from the shader metadata.
                // We don't annotate the assembly itself to avoid parsing errors.
                for d in &mut dependencies {
                    match d {
                        Dependency::Constant(_) => (),
                        Dependency::Buffer(_) => (),
                        Dependency::Texture(t) => {
                            for sampler in &fragment_shader.samplers {
                                if t.name == format!("t{}", sampler.location) {
                                    t.name = (&sampler.name).into();
                                }
                            }
                        }
                        Dependency::Attribute(_) => todo!(),
                    }
                }

                // Simplify the output name to save space.
                let output_name = format!("o{i}.{c}");
                (output_name.into(), dependencies)
            })
        })
        .filter(|(_, dependencies)| !dependencies.is_empty())
        .collect();

    ShaderProgram {
        // IndexMap gives consistent ordering for attribute names.
        output_dependencies,
    }
}

fn geometric_specular_aa(frag: &Graph) -> Option<BufferDependency> {
    // TODO: is specular AA ever used with textures as input?
    // Extract the glossiness input from the following expression:
    // glossiness = 1.0 - sqrt(clamp((1.0 - glossiness)^2 + kernelRoughness2 0.0, 1.0))
    let node_index = frag
        .nodes
        .iter()
        .rposition(|n| n.output.name == "out_attr1" && n.output.channel == Some('y'))?;
    let last_node_index = *frag.node_dependencies_recursive(node_index, None).last()?;
    let last_node = frag.nodes.get(last_node_index)?;

    let node = assign_x(&frag.nodes, last_node)?;
    let node = one_minus_x(&frag.nodes, node)?;
    let node = sqrt_x(&frag.nodes, node)?;
    let node = clamp_x_zero_one(&frag.nodes, node)?;
    let node = match &node.input {
        Expr::Func { name, args, .. } => {
            if name == "fma" {
                match &args[..] {
                    [Expr::Node { node_index: i1, .. }, Expr::Node { node_index: i2, .. }, Expr::Node { .. }] => {
                        if i1 == i2 {
                            frag.nodes.get(*i1)
                        } else {
                            None
                        }
                    }
                    _ => None,
                }
            } else {
                None
            }
        }
        _ => None,
    }?;
    let node = one_plus_x(&frag.nodes, node)?;
    // TODO: Will this final node ever not be a parameter?
    // TODO: Add an option to get the expr itself?
    match &node.input {
        Expr::Sub(a, b) => match (a.deref(), b.deref()) {
            (Expr::Float(0.0), e) => buffer_dependency(e),
            _ => None,
        },
        _ => None,
    }
}

// TODO: Test 1, 2, and 3 layers.
fn _apply_normal_map_layering() {
    // want to find mix(n1, normalize(r), ratio)
    // compiled to (normalize(r) - n1) * ratio + n1
    // TODO: how specific does this need to be?
    // d = r (check for normal maps)
    // c = 0.0 - n1.x
    // a = fma(d, ???, c)
    // fma(a, ratio, n1.x)
}

fn apply_vertex_texcoord_params(
    vertex: &Graph,
    vertex_attributes: &Attributes,
    fragment_attributes: &Attributes,
    dependencies: &mut [Dependency],
) {
    for dependency in dependencies {
        if let Dependency::Texture(texture) = dependency {
            for texcoord in &mut texture.texcoords {
                // Convert a fragment input like "in_attr4" to its vertex output like "vTex0".
                if let Some(fragment_location) = fragment_attributes
                    .input_locations
                    .get_by_left(texcoord.name.as_str())
                {
                    if let Some(vertex_output_name) = vertex_attributes
                        .output_locations
                        .get_by_right(fragment_location)
                    {
                        // Preserve the channel ordering here.
                        // Find any additional scale parameters.
                        for c in texcoord.channels.chars() {
                            if let Some(node) = vertex.nodes.iter().rfind(|n| {
                                &n.output.name == vertex_output_name && n.output.channel == Some(c)
                            }) {
                                if let Expr::Node { node_index, .. } = &node.input {
                                    // Detect common cases for transforming UV coordinates.
                                    if let Some(params) = texcoord_params(vertex, *node_index) {
                                        texcoord.params = Some(params);
                                    }
                                }
                            }
                        }

                        // Also fix channels since the zw output may just be scaled vTex0.xy.
                        if let Some((actual_name, actual_channels)) =
                            find_texcoord_input_name_channels(
                                vertex,
                                texcoord,
                                vertex_output_name,
                                vertex_attributes,
                            )
                        {
                            texcoord.name = actual_name.into();
                            texcoord.channels = actual_channels.into();
                        }
                    }
                }
            }
        }
    }
}

// TODO: Share code with texcoord function.
fn apply_attribute_names(
    vertex: &Graph,
    vertex_attributes: &Attributes,
    fragment_attributes: &Attributes,
    dependencies: &mut [Dependency],
) {
    for dependency in dependencies {
        if let Dependency::Attribute(attribute) = dependency {
            // Convert a fragment input like "in_attr4" to its vertex output like "vTex0".
            if let Some(fragment_location) = fragment_attributes
                .input_locations
                .get_by_left(attribute.name.as_str())
            {
                if let Some(vertex_output_name) = vertex_attributes
                    .output_locations
                    .get_by_right(fragment_location)
                {
                    for c in attribute.channels.chars() {
                        if let Some(input_attribute) = attribute_dependencies(
                            vertex,
                            vertex_output_name,
                            Some(c),
                            vertex_attributes,
                            None,
                        )
                        .first()
                        {
                            attribute.name.clone_from(&input_attribute.name);
                        }
                    }
                }
            }
        }
    }
}

fn find_texcoord_input_name_channels(
    vertex: &Graph,
    texcoord: &xc3_model::shader_database::TexCoord,
    vertex_output_name: &str,
    vertex_attributes: &Attributes,
) -> Option<(String, String)> {
    // We only need to look up one output per texcoord.
    let c = texcoord.channels.chars().next();

    attribute_dependencies(vertex, vertex_output_name, c, vertex_attributes, None)
        .first()
        .map(|a| (a.name.to_string(), a.channels.to_string()))
}

pub fn create_shader_database(input: &str) -> ShaderDatabase {
    // Sort to make the output consistent.
    let mut folders: Vec<_> = std::fs::read_dir(input)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    folders.sort();

    let files = folders
        .par_iter()
        .filter_map(|folder| {
            // TODO: Find a better way to detect maps.
            if !folder.join("map").exists() {
                let programs = create_shader_programs(folder);

                let file = folder.file_name().unwrap().to_string_lossy().to_string();
                Some((file, ModelPrograms { programs }))
            } else {
                None
            }
        })
        .collect();

    let map_files = folders
        .par_iter()
        .filter_map(|folder| {
            // TODO: Find a better way to detect maps.
            if folder.join("map").exists() {
                let map_models = create_map_spchs(&folder.join("map"));
                let prop_models = create_map_spchs(&folder.join("prop"));
                let env_models = create_map_spchs(&folder.join("env"));

                let file = folder.file_name().unwrap().to_string_lossy().to_string();
                Some((
                    file,
                    MapPrograms {
                        map_models,
                        prop_models,
                        env_models,
                    },
                ))
            } else {
                None
            }
        })
        .collect();

    ShaderDatabase::from_models_maps(files, map_files)
}

pub fn create_shader_database_legacy(input: &str) -> ShaderDatabase {
    // Sort to make the output consistent.
    let mut folders: Vec<_> = std::fs::read_dir(input)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    folders.sort();

    // TODO: Should both the inner and outer loops use par_iter?
    let files = folders
        .par_iter()
        .map(|folder| {
            let programs = create_shader_programs_legacy(folder);
            let file = folder.file_name().unwrap().to_string_lossy().to_string();
            (file, ModelPrograms { programs })
        })
        .collect();

    ShaderDatabase::from_models_maps(files, Default::default())
}

fn create_map_spchs(folder: &Path) -> Vec<ModelPrograms> {
    // TODO: Not all maps have env or prop models?
    if let Ok(dir) = std::fs::read_dir(folder) {
        // Folders are generated like "ma01a/prop/4".
        // Sort by index to process files in the right order.
        let mut paths: Vec<_> = dir.into_iter().map(|e| e.unwrap().path()).collect();
        paths.sort_by_cached_key(|p| extract_folder_index(p));

        paths
            .into_iter()
            .map(|path| ModelPrograms {
                programs: create_shader_programs(&path),
            })
            .collect()
    } else {
        Vec::new()
    }
}

fn create_shader_programs(folder: &Path) -> Vec<ShaderProgram> {
    // Only check the first shader for now.
    // TODO: What do additional nvsd shader entries do?
    let mut paths: Vec<_> = globwalk::GlobWalkerBuilder::from_patterns(folder, &["*nvsd0*.frag"])
        .build()
        .unwrap()
        .filter_map(|e| e.map(|e| e.path().to_owned()).ok())
        .collect();

    // Shaders are generated as "slct{program_index}_nvsd{i}_{name}.glsl".
    // Sort by {program_index} to process files in the right order.
    paths.sort_by_cached_key(|p| extract_program_index(p));

    paths
        .par_iter()
        .filter_map(|path| {
            // TODO: Should the vertex shader be mandatory?
            let vertex_source = std::fs::read_to_string(path.with_extension("vert")).ok();
            let vertex = vertex_source.and_then(|s| {
                let source = shader_source_no_extensions(&s);
                match TranslationUnit::parse(source) {
                    Ok(vertex) => Some(vertex),
                    Err(e) => {
                        error!("Error parsing {path:?}: {e}");
                        None
                    }
                }
            });

            let frag_source = std::fs::read_to_string(path).unwrap();
            let frag_source = shader_source_no_extensions(&frag_source);
            match TranslationUnit::parse(frag_source) {
                Ok(fragment) => Some(shader_from_glsl(vertex.as_ref(), &fragment)),
                Err(e) => {
                    error!("Error parsing {path:?}: {e}");
                    None
                }
            }
        })
        .collect()
}

fn extract_program_index(p: &Path) -> usize {
    let name = p.file_name().unwrap().to_string_lossy();
    let start = "slct".len();
    let end = name.find('_').unwrap();
    name[start..end].parse::<usize>().unwrap()
}

fn extract_folder_index(p: &Path) -> usize {
    let name = p.file_name().unwrap().to_string_lossy();
    name.parse::<usize>().unwrap()
}

fn create_shader_programs_legacy(folder: &Path) -> Vec<ShaderProgram> {
    // Only check the first shader for now.
    // TODO: What do additional nvsd shader entries do?
    let mut paths: Vec<_> = globwalk::GlobWalkerBuilder::from_patterns(folder, &["*.frag.txt"])
        .build()
        .unwrap()
        .filter_map(|e| e.map(|e| e.path().to_owned()).ok())
        .collect();

    // Shaders are generated as "{program_index}.frag.txt".
    // Sort by {program_index} to process files in the right order.
    paths.sort_by_cached_key(|p| extract_program_index_legacy(p));

    paths
        .iter()
        .map(|path| {
            // f/i.frag.txt -> f/i
            let path = path.with_extension("").with_extension("");

            let mths = Mths::from_file(path.with_extension("cashd")).unwrap();

            // TODO: Should both shaders be mandatory?
            let vertex_source = std::fs::read_to_string(path.with_extension("vert.txt")).unwrap();
            let frag_source = std::fs::read_to_string(path.with_extension("frag.txt")).unwrap();
            let fragment_shader = mths.fragment_shader().unwrap();
            shader_from_latte_asm(&vertex_source, &frag_source, &fragment_shader)
        })
        .collect()
}

fn extract_program_index_legacy(p: &Path) -> usize {
    p.file_name()
        .unwrap()
        .to_string_lossy()
        .split_once('.')
        .unwrap()
        .0
        .parse::<usize>()
        .unwrap()
}

// TODO: module for this?
#[derive(Debug, Default)]
struct AttributeVisitor {
    attributes: Attributes,
}

#[derive(Debug, Default, PartialEq)]
pub struct Attributes {
    pub input_locations: BiBTreeMap<String, i32>,
    pub output_locations: BiBTreeMap<String, i32>,
}

impl Visitor for AttributeVisitor {
    fn visit_single_declaration(&mut self, declaration: &SingleDeclaration) -> Visit {
        if let Some(name) = &declaration.name {
            if let Some(qualifier) = &declaration.ty.content.qualifier {
                let mut is_input = None;
                let mut location = None;

                for q in &qualifier.qualifiers {
                    match &q.content {
                        TypeQualifierSpecData::Storage(storage) => match &storage.content {
                            StorageQualifierData::In => {
                                is_input = Some(true);
                            }
                            StorageQualifierData::Out => {
                                is_input = Some(false);
                            }
                            _ => (),
                        },
                        TypeQualifierSpecData::Layout(layout) => {
                            if let Some(id) = layout.content.ids.first() {
                                if let LayoutQualifierSpecData::Identifier(key, value) = &id.content
                                {
                                    if key.0 == "location" {
                                        if let Some(ExprData::IntConst(i)) =
                                            value.as_ref().map(|v| &v.content)
                                        {
                                            location = Some(*i);
                                        }
                                    }
                                }
                            }
                        }
                        _ => (),
                    }
                }

                if let (Some(is_input), Some(location)) = (is_input, location) {
                    if is_input {
                        self.attributes
                            .input_locations
                            .insert(name.0.to_string(), location);
                    } else {
                        self.attributes
                            .output_locations
                            .insert(name.0.to_string(), location);
                    }
                }
            }
        }

        Visit::Children
    }
}

pub fn find_attribute_locations(translation_unit: &TranslationUnit) -> Attributes {
    let mut visitor = AttributeVisitor::default();
    translation_unit.visit(&mut visitor);
    visitor.attributes
}

#[cfg(test)]
mod tests {
    use super::*;

    use indoc::indoc;
    use pretty_assertions::assert_eq;
    use xc3_model::shader_database::{
        BufferDependency, TexCoord, TexCoordParams, TextureDependency,
    };

    #[test]
    fn extract_program_index_multiple_digits() {
        assert_eq!(
            89,
            extract_program_index(Path::new(
                "xc3_shader_dump/ch01027000/slct89_nvsd0_shd0089.frag"
            ))
        );
        assert_eq!(
            89,
            extract_program_index(Path::new("xc3_shader_dump/ch01027000/slct89_nvsd1.frag"))
        );
    }

    #[test]
    fn find_attribute_locations_outputs() {
        let glsl = indoc! {"
            layout(location = 0) in vec4 in_attr0;
            layout(location = 4) in vec4 in_attr1;
            layout(location = 3) in vec4 in_attr2;

            layout(location = 3) out vec4 out_attr0;
            layout(location = 5) out vec4 out_attr1;
            layout(location = 7) out vec4 out_attr2;

            void main() {}
        "};

        let tu = TranslationUnit::parse(glsl).unwrap();
        assert_eq!(
            Attributes {
                input_locations: [
                    ("in_attr0".to_string(), 0),
                    ("in_attr1".to_string(), 4),
                    ("in_attr2".to_string(), 3)
                ]
                .into_iter()
                .collect(),
                output_locations: [
                    ("out_attr0".to_string(), 3),
                    ("out_attr1".to_string(), 5),
                    ("out_attr2".to_string(), 7)
                ]
                .into_iter()
                .collect(),
            },
            find_attribute_locations(&tu)
        );
    }

    #[test]
    fn shader_from_vertex_fragment_pyra_body() {
        // Test trimmed shaders from Pyra's metallic chest material.
        // xeno2/bl/bl000101, "ho_BL_TS2", shd0022.vert
        let glsl = indoc! {"
            layout(location = 0) in vec4 vPos;
            layout(location = 1) in vec4 nWgtIdx;
            layout(location = 2) in vec4 vTex0;
            layout(location = 3) in vec4 vColor;
            layout(location = 4) in vec4 vNormal;
            layout(location = 5) in vec4 vTan;
            layout(location = 0) out vec4 out_attr0;
            layout(location = 1) out vec4 out_attr1;
            layout(location = 2) out vec4 out_attr2;
            layout(location = 3) out vec4 out_attr3;
            layout(location = 4) out vec4 out_attr4;
            layout(location = 5) out vec4 out_attr5;
            layout(location = 6) out vec4 out_attr6;
            layout(location = 7) out vec4 out_attr7;
            layout(location = 8) out vec4 out_attr8;

            void main() 
            {
                temp_62 = vTex0.x;
                temp_64 = vTex0.y;
                temp_119 = temp_62 * U_Mate.gWrkFl4[0].x;
                temp_179 = temp_64 * U_Mate.gWrkFl4[0].y;
                out_attr4.x = temp_62;
                out_attr4.y = temp_64;
                out_attr4.z = temp_119;
                out_attr4.w = temp_179;
            }
        "};
        let vertex = TranslationUnit::parse(glsl).unwrap();

        // xeno2/bl/bl000101, "ho_BL_TS2", shd0022.frag
        let glsl = indoc! {"
            layout(location = 0) in vec4 in_attr0;
            layout(location = 1) in vec4 in_attr1;
            layout(location = 2) in vec4 in_attr2;
            layout(location = 3) in vec4 in_attr3;
            layout(location = 4) in vec4 in_attr4;
            layout(location = 5) in vec4 in_attr5;
            layout(location = 6) in vec4 in_attr6;
            layout(location = 7) in vec4 in_attr7;
            layout(location = 0) out vec4 out_attr0;
            layout(location = 1) out vec4 out_attr1;
            layout(location = 2) out vec4 out_attr2;
            layout(location = 3) out vec4 out_attr3;
            layout(location = 4) out vec4 out_attr4;
            layout(location = 5) out vec4 out_attr5;

            void main() 
            {
                temp_1 = in_attr4.x;
                temp_2 = in_attr4.y;
                temp_15 = in_attr4.z;
                temp_16 = in_attr4.w;
                temp_17 = texture(s5, vec2(temp_15, temp_16)).xyz;
                temp_18 = temp_17.x;
                temp_19 = temp_17.y;
                temp_20 = temp_17.z;
                temp_21 = texture(s4, vec2(temp_1, temp_2)).xy;
                temp_22 = temp_21.x;
                temp_23 = temp_21.y;
                out_attr1.x = temp_23;
                out_attr1.y = U_Mate.gWrkFl4[2].x;
                out_attr1.z = U_Mate.gWrkFl4[1].y;
                out_attr1.w = 0.07098039;
                out_attr5.x = temp_18;
                out_attr5.y = temp_19;
                out_attr5.z = temp_20;
                out_attr5.w = 0.0;
            }
        "};

        let fragment = TranslationUnit::parse(glsl).unwrap();
        let shader = shader_from_glsl(Some(&vertex), &fragment);
        assert_eq!(
            ShaderProgram {
                output_dependencies: [
                    (
                        "o1.x".into(),
                        vec![Dependency::Texture(TextureDependency {
                            name: "s4".into(),
                            channels: "y".into(),
                            texcoords: vec![
                                TexCoord {
                                    name: "vTex0".into(),
                                    channels: "x".into(),
                                    params: None,
                                },
                                TexCoord {
                                    name: "vTex0".into(),
                                    channels: "y".into(),
                                    params: None,
                                },
                            ]
                        })]
                    ),
                    (
                        "o1.y".into(),
                        vec![Dependency::Buffer(BufferDependency {
                            name: "U_Mate".into(),
                            field: "gWrkFl4".into(),
                            index: 2,
                            channels: "x".into(),
                        })]
                    ),
                    (
                        "o1.z".into(),
                        vec![Dependency::Buffer(BufferDependency {
                            name: "U_Mate".into(),
                            field: "gWrkFl4".into(),
                            index: 1,
                            channels: "y".into(),
                        })]
                    ),
                    ("o1.w".into(), vec![Dependency::Constant(0.07098039.into())]),
                    (
                        "o5.x".into(),
                        vec![Dependency::Texture(TextureDependency {
                            name: "s5".into(),
                            channels: "x".into(),
                            texcoords: vec![
                                TexCoord {
                                    name: "vTex0".into(),
                                    channels: "x".into(),
                                    params: Some(TexCoordParams::Scale(BufferDependency {
                                        name: "U_Mate".into(),
                                        field: "gWrkFl4".into(),
                                        index: 0,
                                        channels: "x".into(),
                                    }))
                                },
                                TexCoord {
                                    name: "vTex0".into(),
                                    channels: "y".into(),
                                    params: Some(TexCoordParams::Scale(BufferDependency {
                                        name: "U_Mate".into(),
                                        field: "gWrkFl4".into(),
                                        index: 0,
                                        channels: "y".into(),
                                    }))
                                },
                            ],
                        })]
                    ),
                    (
                        "o5.y".into(),
                        vec![Dependency::Texture(TextureDependency {
                            name: "s5".into(),
                            channels: "y".into(),
                            texcoords: vec![
                                TexCoord {
                                    name: "vTex0".into(),
                                    channels: "x".into(),
                                    params: Some(TexCoordParams::Scale(BufferDependency {
                                        name: "U_Mate".into(),
                                        field: "gWrkFl4".into(),
                                        index: 0,
                                        channels: "x".into(),
                                    }))
                                },
                                TexCoord {
                                    name: "vTex0".into(),
                                    channels: "y".into(),
                                    params: Some(TexCoordParams::Scale(BufferDependency {
                                        name: "U_Mate".into(),
                                        field: "gWrkFl4".into(),
                                        index: 0,
                                        channels: "y".into(),
                                    }))
                                },
                            ],
                        })]
                    ),
                    (
                        "o5.z".into(),
                        vec![Dependency::Texture(TextureDependency {
                            name: "s5".into(),
                            channels: "z".into(),
                            texcoords: vec![
                                TexCoord {
                                    name: "vTex0".into(),
                                    channels: "x".into(),
                                    params: Some(TexCoordParams::Scale(BufferDependency {
                                        name: "U_Mate".into(),
                                        field: "gWrkFl4".into(),
                                        index: 0,
                                        channels: "x".into(),
                                    }))
                                },
                                TexCoord {
                                    name: "vTex0".into(),
                                    channels: "y".into(),
                                    params: Some(TexCoordParams::Scale(BufferDependency {
                                        name: "U_Mate".into(),
                                        field: "gWrkFl4".into(),
                                        index: 0,
                                        channels: "y".into(),
                                    }))
                                },
                            ],
                        })]
                    ),
                    ("o5.w".into(), vec![Dependency::Constant(0.0.into())]),
                ]
                .into()
            },
            shader
        );
    }

    #[test]
    fn shader_from_vertex_fragment_mio_skirt() {
        // xeno3/chr/ch/ch11021013, "body_skert2", shd0028.frag
        let glsl = indoc! {"
            void main() {
                temp_0 = in_attr3.x;
                temp_1 = in_attr3.y;
                temp_2 = in_attr3.z;
                temp_3 = in_attr3.w;
                temp_8 = texture(s2, vec2(temp_0, temp_1)).xy;
                temp_8 = texture(s2, vec2(temp_0, temp_1)).xy;
                temp_9 = temp_8.x;
                temp_10 = temp_8.y;
                temp_11 = texture(gTResidentTex09, vec2(temp_2, temp_3)).xy;
                temp_12 = temp_11.x;
                temp_13 = temp_11.y;
                temp_26 = fma(temp_9, 2.0, 1.0039216);
                temp_27 = fma(temp_10, 2.0, 1.0039216);
                temp_28 = temp_26 * temp_26;
                temp_29 = fma(temp_12, 2.0, 1.0039216);
                temp_30 = fma(temp_13, 2.0, 1.0039216);
                temp_31 = 0.0 + temp_27;
                temp_32 = fma(temp_27, temp_27, temp_28);
                temp_33 = temp_29 * temp_29;
                temp_34 = 0.0 - temp_32;
                temp_35 = temp_34 + 1.0;
                temp_36 = fma(temp_30, temp_30, temp_33);
                temp_37 = sqrt(temp_35);
                temp_38 = 0.0 + temp_26;
                temp_40 = 0.0 - temp_36;
                temp_41 = temp_40 + 1.0;
                temp_42 = 0.0 - temp_38;
                temp_43 = temp_29 * temp_42;
                temp_44 = sqrt(temp_41);
                temp_47 = max(0.0, temp_37);
                temp_48 = 0.0 - temp_31;
                temp_49 = fma(temp_30, temp_48, temp_43);
                temp_51 = temp_47 + 1.0;
                temp_52 = max(0.0, temp_44);
                temp_53 = in_attr1.y;
                temp_54 = 0.0 - temp_51;
                temp_55 = temp_29 * temp_54;
                temp_56 = in_attr1.x;
                temp_57 = fma(temp_52, temp_51, temp_49);
                temp_58 = temp_52 * temp_51;
                temp_59 = 0.0 - temp_55;
                temp_60 = fma(temp_38, temp_57, temp_59);
                temp_61 = 0.0 - temp_51;
                temp_62 = temp_30 * temp_61;
                temp_63 = in_attr1.z;
                temp_64 = 0.0 - temp_58;
                temp_65 = fma(temp_51, temp_57, temp_64);
                temp_66 = 0.0 - temp_62;
                temp_67 = fma(temp_31, temp_57, temp_66);
                temp_68 = temp_56 * temp_56;
                temp_69 = fma(temp_53, temp_53, temp_68);
                temp_70 = temp_60 * temp_60;
                temp_71 = fma(temp_63, temp_63, temp_69);
                temp_72 = fma(temp_67, temp_67, temp_70);
                temp_73 = inversesqrt(temp_71);
                temp_74 = fma(temp_65, temp_65, temp_72);
                temp_75 = in_attr0.x;
                temp_76 = temp_56 * temp_73;
                temp_77 = inversesqrt(temp_74);
                temp_78 = temp_53 * temp_73;
                temp_79 = temp_63 * temp_73;
                temp_80 = 0.0 - temp_26;
                temp_81 = fma(temp_60, temp_77, temp_80);
                temp_82 = 0.0 - temp_27;
                temp_83 = fma(temp_67, temp_77, temp_82);
                temp_84 = in_attr0.y;
                temp_85 = 0.0 - temp_47;
                temp_86 = fma(temp_65, temp_77, temp_85);
                temp_87 = in_attr0.z;
                temp_88 = fma(temp_81, U_Mate.gWrkFl4[1].z, temp_26);
                temp_89 = fma(temp_83, U_Mate.gWrkFl4[1].z, temp_27);
                temp_90 = fma(temp_86, U_Mate.gWrkFl4[1].z, temp_47);
                temp_91 = temp_75 * temp_75;
                temp_92 = temp_88 * temp_88;
                temp_93 = fma(temp_89, temp_89, temp_92);
                temp_94 = fma(temp_84, temp_84, temp_91);
                temp_95 = fma(temp_87, temp_87, temp_94);
                temp_96 = fma(temp_90, temp_90, temp_93);
                temp_97 = inversesqrt(temp_95);
                temp_100 = inversesqrt(temp_96);
                temp_105 = temp_75 * temp_97;
                temp_106 = temp_84 * temp_97;
                temp_107 = temp_87 * temp_97;
                temp_108 = in_attr2.x;
                temp_109 = temp_88 * temp_100;
                temp_110 = temp_90 * temp_100;
                temp_111 = temp_89 * temp_100;
                temp_112 = in_attr2.y;
                temp_114 = temp_109 * temp_76;
                temp_115 = temp_109 * temp_78;
                temp_116 = in_attr2.z;
                temp_117 = temp_109 * temp_79;
                temp_118 = fma(temp_110, temp_105, temp_114);
                temp_120 = fma(temp_110, temp_106, temp_115);
                temp_121 = fma(temp_110, temp_107, temp_117);
                temp_122 = temp_108 * temp_108;
                temp_123 = fma(temp_112, temp_112, temp_122);
                temp_125 = fma(temp_116, temp_116, temp_123);
                temp_126 = inversesqrt(temp_125);
                temp_127 = temp_108 * temp_126;
                temp_128 = temp_112 * temp_126;
                temp_129 = temp_116 * temp_126;
                temp_130 = fma(temp_111, temp_127, temp_118);
                temp_132 = fma(temp_111, temp_128, temp_120);
                temp_133 = fma(temp_111, temp_129, temp_121);
                temp_134 = 0.0 - U_Mate.gWrkFl4[2].y;
                temp_135 = 1.0 + temp_134;
                temp_136 = temp_130 * temp_130;
                temp_137 = fma(temp_132, temp_132, temp_136);
                temp_138 = fma(temp_133, temp_133, temp_137);
                temp_140 = inversesqrt(temp_138);
                temp_143 = temp_130 * temp_140;
                temp_144 = temp_132 * temp_140;
                temp_145 = temp_133 * temp_140;
                temp_149 = 0.0 - temp_143;
                temp_150 = temp_149 + 0.0;
                temp_146 = temp_150;
                temp_151 = 0.0 - temp_144;
                temp_152 = temp_151 + 0.0;
                temp_147 = temp_152;
                temp_153 = 0.0 - temp_145;
                temp_154 = temp_153 + 0.0;
                temp_148 = temp_154;
                temp_155 = dFdy(temp_146);
                temp_156 = dFdy(temp_147);
                temp_157 = 1.0 * temp_155;
                temp_158 = dFdy(temp_148);
                temp_159 = 1.0 * temp_156;
                temp_160 = dFdx(temp_146);
                temp_161 = temp_157 * temp_157;
                temp_162 = 1.0 * temp_158;
                temp_164 = dFdx(temp_147);
                temp_165 = temp_160 * temp_160;
                temp_166 = fma(temp_159, temp_159, temp_161);
                temp_168 = fma(temp_164, temp_164, temp_165);
                temp_170 = fma(temp_162, temp_162, temp_166);
                temp_174 = dFdx(temp_148);
                temp_176 = fma(temp_174, temp_174, temp_168);
                temp_177 = temp_176 + temp_170;
                temp_182 = temp_177 * 0.5;
                temp_183 = min(temp_182, fp_c1.data[0].y);
                temp_184 = fma(temp_135, temp_135, temp_183);
                temp_185 = clamp(temp_184, 0.0, 1.0);
                temp_186 = sqrt(temp_185);
                temp_192 = 0.0 - temp_186;
                temp_193 = temp_192 + 1.0;
                out_attr1.y = temp_193;
            }
        "};

        // The pcmdo calcGeometricSpecularAA function compiles to the expression
        // glossiness = 1.0 - sqrt(clamp((1.0 - glossiness)^2 + kernelRoughness2 0.0, 1.0))
        // Consuming applications only care about the glossiness input.
        // This also avoids considering normal maps as a dependency.
        let fragment = TranslationUnit::parse(glsl).unwrap();
        let shader = shader_from_glsl(None, &fragment);
        assert_eq!(
            ShaderProgram {
                output_dependencies: [(
                    "o1.y".into(),
                    vec![Dependency::Buffer(BufferDependency {
                        name: "U_Mate".into(),
                        field: "gWrkFl4".into(),
                        index: 2,
                        channels: "y".into()
                    })]
                ),]
                .into()
            },
            shader
        );
    }

    #[test]
    fn shader_from_latte_asm_pc221115_frag_0() {
        // Elma's legs (visible on title screen).
        let asm = indoc! {"
            00 TEX: ADDR(208) CNT(4)

            0      SAMPLE          R2.xy__, R6.xy0x, t3, s3

            1      SAMPLE          R8.xyz_, R6.xy0x, t2, s2

            2      SAMPLE          R7.xyz_, R6.xy0x, t1, s1

            3      SAMPLE          R6.xyz_, R6.xy0x, t4, s4

            01 ALU: ADDR(32) CNT(127) KCACHE0(CB1:0-15)
            4   x: MULADD          R125.x, R2.x, (0x40000000, 2), -1.0f
                y: MULADD          R126.y, R2.y, (0x40000000, 2), -1.0f
                z: MOV             ____, 0.0f
                w: MUL             R124.w, R2.z, (0x41000000, 8)
                t: SQRT_IEEE       ____, R5.w SCL_210

            5   x: DOT4            ____, PV4.x, PV4.x
                y: DOT4            ____, PV4.y, PV4.y
                z: DOT4            ____, PV4.z, PV4.y
                w: DOT4            ____, (0x80000000, -0), 0.0f
                t: ADD             R0.w, -PS4, 1.0f CLAMP

            6   x: DOT4_IEEE       ____, R5.x, R5.x
                y: DOT4_IEEE       ____, R5.y, R5.y
                z: DOT4_IEEE       ____, R5.z, R5.z
                w: DOT4_IEEE       ____, (0x80000000, -0), 0.0f
                t: ADD             R127.w, -PV5.x, 1.0f

            7   x: DOT4_IEEE       ____, R3.x, R3.x
                y: DOT4_IEEE       R127.y, R3.y, R3.y
                z: DOT4_IEEE       ____, R3.z, R3.z
                w: DOT4_IEEE       ____, (0x80000000, -0), 0.0f
                t: RECIPSQRT_IEEE  ____, PV6.x SCL_210

            8   x: MUL             R127.x, R5.x, PS7
                y: FLOOR           R125.y, R124.w
                z: MUL             R126.z, R5.z, PS7
                w: MUL             R127.w, R5.y, PS7
                t: SQRT_IEEE       R127.z, R127.w SCL_210

            9   x: DOT4_IEEE       ____, R0.x, R0.x
                y: DOT4_IEEE       ____, R0.y, R0.y
                z: DOT4_IEEE       ____, R0.z, R0.z
                w: DOT4_IEEE       ____, (0x80000000, -0), 0.0f
                t: RECIPSQRT_IEEE  ____, R127.y SCL_210

            10  x: MUL             R126.x, R3.z, PS9
                y: MAX             ____, R127.z, 0.0f VEC_120
                z: MUL             R127.z, R3.y, PS9
                w: MUL             R126.w, R3.x, PS9
                t: RECIPSQRT_IEEE  R125.w, PV9.x SCL_210

            11  x: MUL             ____, R126.z, PV10.y
                y: MUL             ____, R127.w, PV10.y
                z: MUL             R126.z, R0.x, PS10
                w: MUL             ____, R127.x, PV10.y VEC_120
                t: MUL             R127.y, R0.y, PS10

            12  x: MUL             ____, R0.z, R125.w
                y: MULADD          R123.y, R126.x, R125.x, PV11.x
                z: MULADD          R123.z, R126.w, R125.x, PV11.w
                w: MULADD          R123.w, R127.z, R125.x, PV11.y VEC_120
                t: MUL             R124.y, R125.y, (0x3B808081, 0.003921569)

            13  x: MULADD          R126.x, R126.z, R126.y, PV12.z
                y: MULADD          R127.y, R127.y, R126.y, PV12.w
                z: MULADD          R126.z, PV12.x, R126.y, PV12.y
                w: FLOOR           R126.w, PS12
                t: MOV             R2.w, 0.0f

            14  x: DOT4_IEEE       ____, R1.x, R1.x
                y: DOT4_IEEE       ____, R1.y, R1.y
                z: DOT4_IEEE       ____, R1.z, R1.z
                w: DOT4_IEEE       ____, (0x80000000, -0), 0.0f
                t: MOV             R6.w, KC0[1].x

            15  x: DOT4_IEEE       ____, R126.x, R126.x
                y: DOT4_IEEE       ____, R127.y, R127.y
                z: DOT4_IEEE       ____, R126.z, R126.z
                w: DOT4_IEEE       ____, (0x80000000, -0), 0.0f
                t: RECIPSQRT_IEEE  ____, PV14.x SCL_210

            16  x: MUL             R125.x, R1.x, PS15
                y: MUL             R126.y, R1.y, PS15
                z: MUL             R127.z, R1.z, PS15
                w: MOV             R5.w, R8.z VEC_120
                t: RECIPSQRT_IEEE  ____, PV15.x SCL_210

            17  x: MUL             R126.x, R126.x, PS16
                y: MUL             R127.y, R127.y, PS16
                z: MUL             R126.z, R126.z, PS16
                w: MUL_IEEE        ____, R4.z, R4.z VEC_120
                t: ADD             R5.x, R124.w, -R125.y

            18  x: DOT4            ____, R125.x, PV17.x
                y: DOT4            ____, R126.y, PV17.y
                z: DOT4            ____, R127.z, PV17.z
                w: DOT4            ____, (0x80000000, -0), 0.0f
                t: MULADD_IEEE     R122.x, R4.y, R4.y, PV17.w

            19  x: MULADD_IEEE     R123.x, R4.x, R4.x, PS18
                y: MUL             ____, PV18.x, R0.w VEC_021
                z: MUL             R5.z, R126.w, (0x3B808081, 0.003921569)
                t: ADD             R5.y, R124.y, -R126.w

            20  x: MULADD          R126.x, -R125.x, PV19.y, R126.x
                y: MULADD          R127.y, -R126.y, PV19.y, R127.y
                z: MULADD          R127.z, -R127.z, PV19.y, R126.z
                t: RECIPSQRT_IEEE  R126.w, PV19.x SCL_210

            21  x: DOT4_IEEE       ____, PV20.x, PV20.x
                y: DOT4_IEEE       ____, PV20.y, PV20.y
                z: DOT4_IEEE       ____, PV20.z, PV20.z
                w: DOT4_IEEE       ____, (0x80000000, -0), 0.0f
                t: MUL             R1.x, R4.x, PS20

            22  y: MUL             R1.y, R4.y, R126.w
                z: MUL             R126.z, R4.z, R126.w
                t: RECIPSQRT_IEEE  ____, PV21.x SCL_210

            23  x: MUL             R4.x, R126.x, PS22
                y: MUL             R4.y, R127.y, PS22
                z: MUL             R127.z, R127.z, PS22

            24  x: MOV             R9.x, PV23.x
                y: ADD/2           ____, PV23.y, 1.0f
                z: ADD/2           ____, PV23.x, 1.0f
                w: MOV             R9.w, PV23.y
                t: MUL             ____, -R126.z, PV23.z

            25  x: ADD             ____, -PV24.y, 1.0f
                y: MUL             ____, R126.z, R127.z
                w: MAX             ____, PV24.z, 0.0f
                t: MULADD          R122.x, -R1.y, R4.y, PS24

            26  x: MULADD          R123.x, -R1.x, R4.x, PS25
                z: MAX             ____, PV25.x, 0.0f
                w: MIN             R3.w, PV25.w, 1.0f
                t: MULADD          R122.x, R1.y, R4.y, PV25.y

            27  y: MIN             R3.y, PV26.z, 1.0f
                z: ADD             R4.z, PV26.x, PV26.x
                w: MULADD          R0.w, R1.x, R4.x, PS26

            28  x: MULADD_D2       R123.x, -R4.z, R4.x, -R1.x
                y: MAX_DX10        ____, R0.w, -R0.w
                w: MULADD_D2       R123.w, -R4.z, R4.y, -R1.y

            29  x: ADD             R1.x, PV28.x, 0.5f
                y: ADD             R1.y, PV28.w, 0.5f
                z: ADD             R4.z, -PV28.y, 1.0f CLAMP

            02 TEX: ADDR(216) CNT(2) VALID_PIX

            30     SAMPLE          R3.xyzw, R3.wy0w, t0, s0

            31     SAMPLE          R1.xyz_, R1.xy0x, t5, s5

            03 ALU: ADDR(159) CNT(40) KCACHE0(CB1:0-15)
            32  x: MULADD          R126.x, KC0[0].z, R3.z, 0.0f
                y: MULADD          R127.y, KC0[0].y, R3.y, 0.0f
                z: MULADD          R123.z, KC0[0].w, R3.w, 0.0f
                w: MULADD          R126.w, KC0[0].x, R3.x, 0.0f
                t: LOG_CLAMPED     ____, R4.z SCL_210

            33  x: MULADD          R2.x, R8.x, R1.x, R7.x
                y: MULADD          R2.y, R8.x, R1.y, R7.y
                z: MULADD          R2.z, R8.x, R1.z, R7.z
                w: MUL             ____, KC0[2].w, PS32
                t: MOV/2           R1.w, PV32.z

            34  t: EXP_IEEE        ____, PV33.w SCL_210

            35  x: MULADD          R123.x, KC0[2].x, PS34, R126.w
                z: MULADD          R123.z, KC0[2].y, PS34, R127.y
                w: MULADD          R123.w, KC0[2].z, PS34, R126.x

            36  x: MUL             ____, R8.y, PV35.z
                y: MUL             ____, R8.y, PV35.x
                z: MUL             ____, R8.y, PV35.w

            37  x: MOV/2           R1.x, PV36.y
                y: MOV/2           R1.y, PV36.x
                z: MOV/2           R1.z, PV36.z

            38  x: MOV             R14.x, R5.x
                y: MOV             R14.y, R5.y
                z: MOV             R14.z, R5.z
                w: MOV             R14.w, R5.w

            39  x: MOV             R13.x, R6.x
                y: MOV             R13.y, R6.y
                z: MOV             R13.z, R6.z
                w: MOV             R13.w, R6.w

            40  x: MOV             R11.x, R2.x
                y: MOV             R11.y, R2.y
                z: MOV             R11.z, R2.z
                w: MOV             R11.w, R2.w

            41  x: MOV             R10.x, R1.x
                y: MOV             R10.y, R1.y
                z: MOV             R10.z, R1.z
                w: MOV             R10.w, R1.w

            42  x: MOV             R12.x, R9.x
                y: MOV             R12.y, R9.w
                z: MOV             R12.z, R9.z
                w: MOV             R12.w, R9.z

            04 EXP_DONE: PIX0, R10.xyzw BURSTCNT(4)

            END_OF_PROGRAM
        "};

        // TODO: Make this easier to test by taking metadata directly?
        let fragment_shader = xc3_lib::mths::FragmentShader {
            unk1: 0,
            unk2: 0,
            program_binary: Vec::new(),
            shader_mode: xc3_lib::mths::ShaderMode::UniformBlock,
            uniform_buffers: vec![xc3_lib::mths::UniformBuffer {
                name: "U_Mate".to_string(),
                offset: 1,
                size: 48,
            }],
            uniforms: vec![
                xc3_lib::mths::Uniform {
                    name: "Q".to_string(),
                    data_type: xc3_lib::mths::VarType::Vec4,
                    count: 1,
                    offset: 0,
                    uniform_buffer_index: 0,
                },
                xc3_lib::mths::Uniform {
                    name: "Q".to_string(),
                    data_type: xc3_lib::mths::VarType::Vec4,
                    count: 1,
                    offset: 8,
                    uniform_buffer_index: 0,
                },
                xc3_lib::mths::Uniform {
                    name: "Q".to_string(),
                    data_type: xc3_lib::mths::VarType::Vec4,
                    count: 1,
                    offset: 4,
                    uniform_buffer_index: 0,
                },
            ],
            unk9: [0, 0, 0, 0],
            samplers: vec![
                xc3_lib::mths::Sampler {
                    name: "gIBL".to_string(),
                    sampler_type: xc3_lib::mths::SamplerType::D2,
                    location: 0,
                },
                xc3_lib::mths::Sampler {
                    name: "s0".to_string(),
                    sampler_type: xc3_lib::mths::SamplerType::D2,
                    location: 1,
                },
                xc3_lib::mths::Sampler {
                    name: "s1".to_string(),
                    sampler_type: xc3_lib::mths::SamplerType::D2,
                    location: 2,
                },
                xc3_lib::mths::Sampler {
                    name: "s2".to_string(),
                    sampler_type: xc3_lib::mths::SamplerType::D2,
                    location: 3,
                },
                xc3_lib::mths::Sampler {
                    name: "s3".to_string(),
                    sampler_type: xc3_lib::mths::SamplerType::D2,
                    location: 4,
                },
                xc3_lib::mths::Sampler {
                    name: "texRef".to_string(),
                    sampler_type: xc3_lib::mths::SamplerType::D2,
                    location: 5,
                },
            ],
        };
        let shader = shader_from_latte_asm("".into(), &asm, &fragment_shader);
        assert_eq!(
            ShaderProgram {
                output_dependencies: [
                    (
                        "o0.x".into(),
                        vec![
                            Dependency::Texture(TextureDependency {
                                name: "s2".into(),
                                channels: "x".into(),
                                texcoords: Vec::new(),
                            }),
                            Dependency::Texture(TextureDependency {
                                name: "s2".into(),
                                channels: "y".into(),
                                texcoords: Vec::new(),
                            }),
                            Dependency::Texture(TextureDependency {
                                name: "s1".into(),
                                channels: "y".into(),
                                texcoords: Vec::new(),
                            }),
                            Dependency::Texture(TextureDependency {
                                name: "gIBL".into(),
                                channels: "x".into(),
                                texcoords: Vec::new(),
                            }),
                        ]
                    ),
                    (
                        "o0.y".into(),
                        vec![
                            Dependency::Texture(TextureDependency {
                                name: "s2".into(),
                                channels: "x".into(),
                                texcoords: Vec::new(),
                            }),
                            Dependency::Texture(TextureDependency {
                                name: "s2".into(),
                                channels: "y".into(),
                                texcoords: Vec::new(),
                            }),
                            Dependency::Texture(TextureDependency {
                                name: "s1".into(),
                                channels: "y".into(),
                                texcoords: Vec::new(),
                            }),
                            Dependency::Texture(TextureDependency {
                                name: "gIBL".into(),
                                channels: "y".into(),
                                texcoords: Vec::new(),
                            }),
                        ]
                    ),
                    (
                        "o0.z".into(),
                        vec![
                            Dependency::Texture(TextureDependency {
                                name: "s2".into(),
                                channels: "x".into(),
                                texcoords: Vec::new(),
                            }),
                            Dependency::Texture(TextureDependency {
                                name: "s2".into(),
                                channels: "y".into(),
                                texcoords: Vec::new(),
                            }),
                            Dependency::Texture(TextureDependency {
                                name: "s1".into(),
                                channels: "y".into(),
                                texcoords: Vec::new(),
                            }),
                            Dependency::Texture(TextureDependency {
                                name: "gIBL".into(),
                                channels: "z".into(),
                                texcoords: Vec::new(),
                            }),
                        ]
                    ),
                    (
                        "o0.w".into(),
                        vec![
                            Dependency::Texture(TextureDependency {
                                name: "s2".into(),
                                channels: "x".into(),
                                texcoords: Vec::new(),
                            }),
                            Dependency::Texture(TextureDependency {
                                name: "s2".into(),
                                channels: "y".into(),
                                texcoords: Vec::new(),
                            }),
                            Dependency::Texture(TextureDependency {
                                name: "gIBL".into(),
                                channels: "w".into(),
                                texcoords: Vec::new(),
                            }),
                        ]
                    ),
                    (
                        "o1.x".into(),
                        vec![
                            Dependency::Texture(TextureDependency {
                                name: "s2".into(),
                                channels: "x".into(),
                                texcoords: Vec::new(),
                            }),
                            Dependency::Texture(TextureDependency {
                                name: "s2".into(),
                                channels: "y".into(),
                                texcoords: Vec::new(),
                            }),
                            Dependency::Texture(TextureDependency {
                                name: "s1".into(),
                                channels: "x".into(),
                                texcoords: Vec::new(),
                            }),
                            Dependency::Texture(TextureDependency {
                                name: "s0".into(),
                                channels: "x".into(),
                                texcoords: Vec::new(),
                            }),
                            Dependency::Texture(TextureDependency {
                                name: "texRef".into(),
                                channels: "x".into(),
                                texcoords: Vec::new(),
                            }),
                        ]
                    ),
                    (
                        "o1.y".into(),
                        vec![
                            Dependency::Texture(TextureDependency {
                                name: "s2".into(),
                                channels: "x".into(),
                                texcoords: Vec::new(),
                            }),
                            Dependency::Texture(TextureDependency {
                                name: "s2".into(),
                                channels: "y".into(),
                                texcoords: Vec::new(),
                            }),
                            Dependency::Texture(TextureDependency {
                                name: "s1".into(),
                                channels: "x".into(),
                                texcoords: Vec::new(),
                            }),
                            Dependency::Texture(TextureDependency {
                                name: "s0".into(),
                                channels: "y".into(),
                                texcoords: Vec::new(),
                            }),
                            Dependency::Texture(TextureDependency {
                                name: "texRef".into(),
                                channels: "y".into(),
                                texcoords: Vec::new(),
                            }),
                        ]
                    ),
                    (
                        "o1.z".into(),
                        vec![
                            Dependency::Texture(TextureDependency {
                                name: "s2".into(),
                                channels: "x".into(),
                                texcoords: Vec::new(),
                            }),
                            Dependency::Texture(TextureDependency {
                                name: "s2".into(),
                                channels: "y".into(),
                                texcoords: Vec::new(),
                            }),
                            Dependency::Texture(TextureDependency {
                                name: "s1".into(),
                                channels: "x".into(),
                                texcoords: Vec::new(),
                            }),
                            Dependency::Texture(TextureDependency {
                                name: "s0".into(),
                                channels: "z".into(),
                                texcoords: Vec::new(),
                            }),
                            Dependency::Texture(TextureDependency {
                                name: "texRef".into(),
                                channels: "z".into(),
                                texcoords: Vec::new(),
                            }),
                        ]
                    ),
                    ("o1.w".into(), vec![Dependency::Constant(0.0.into())]),
                    (
                        "o2.x".into(),
                        vec![
                            Dependency::Texture(TextureDependency {
                                name: "s2".into(),
                                channels: "x".into(),
                                texcoords: Vec::new(),
                            }),
                            Dependency::Texture(TextureDependency {
                                name: "s2".into(),
                                channels: "y".into(),
                                texcoords: Vec::new(),
                            }),
                        ]
                    ),
                    (
                        "o2.y".into(),
                        vec![
                            Dependency::Texture(TextureDependency {
                                name: "s2".into(),
                                channels: "x".into(),
                                texcoords: Vec::new(),
                            }),
                            Dependency::Texture(TextureDependency {
                                name: "s2".into(),
                                channels: "y".into(),
                                texcoords: Vec::new(),
                            }),
                        ]
                    ),
                    (
                        "o3.x".into(),
                        vec![Dependency::Texture(TextureDependency {
                            name: "s3".into(),
                            channels: "x".into(),
                            texcoords: Vec::new(),
                        })]
                    ),
                    (
                        "o3.y".into(),
                        vec![Dependency::Texture(TextureDependency {
                            name: "s3".into(),
                            channels: "y".into(),
                            texcoords: Vec::new(),
                        })]
                    ),
                    (
                        "o3.z".into(),
                        vec![Dependency::Texture(TextureDependency {
                            name: "s3".into(),
                            channels: "z".into(),
                            texcoords: Vec::new(),
                        })],
                    ),
                    (
                        "o3.w".into(),
                        vec![Dependency::Buffer(BufferDependency {
                            name: "KC0".into(),
                            field: "".into(),
                            index: 1,
                            channels: "x".into(),
                        })],
                    ),
                    (
                        "o4.w".into(),
                        vec![Dependency::Texture(TextureDependency {
                            name: "s1".into(),
                            channels: "z".into(),
                            texcoords: Vec::new(),
                        })],
                    )
                ]
                .into()
            },
            shader
        );
    }
}
