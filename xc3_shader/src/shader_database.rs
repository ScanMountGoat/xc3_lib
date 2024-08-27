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
use indexmap::IndexMap;
use log::error;
use rayon::prelude::*;
use xc3_lib::mths::{FragmentShader, Mths};
use xc3_model::shader_database::{
    BufferDependency, Dependency, MapPrograms, ModelPrograms, ShaderDatabase, ShaderProgram,
    TextureLayer,
};

use crate::{
    annotation::shader_source_no_extensions,
    dependencies::{
        attribute_dependencies, buffer_dependency, input_dependencies, texcoord_params,
    },
    graph::{
        query::{
            assign_x, assign_x_recursive, clamp_x_zero_one, dot3_a_b, fma_half_half, mix_a_b_ratio,
            normalize, one_minus_x, one_plus_x, sqrt_x, zero_minus_x,
        },
        Expr, Graph, Node,
    },
};

fn shader_from_glsl(vertex: Option<&TranslationUnit>, fragment: &TranslationUnit) -> ShaderProgram {
    let frag = &Graph::from_glsl(fragment);
    let frag_attributes = &find_attribute_locations(fragment);

    let vertex = &vertex.map(|v| (Graph::from_glsl(v), find_attribute_locations(v)));

    let mut color_layers = Vec::new();
    let mut normal_layers = Vec::new();

    let mut output_dependencies = IndexMap::new();
    for i in 0..=5 {
        for c in "xyzw".chars() {
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

                apply_attribute_names(vert, vert_attributes, frag_attributes, &mut dependencies);
            }

            if i == 0 && c == 'x' {
                // TODO: This will be different for each channel.
                color_layers = find_color_layers(frag, "out_attr0", Some('x'), &dependencies)
                    .unwrap_or_default();
            } else if i == 1 && c == 'y' {
                if let Some(param) = geometric_specular_aa(frag) {
                    dependencies = vec![Dependency::Buffer(param)];
                }
            } else if i == 2 && c == 'x' {
                normal_layers = find_normal_layers(frag, "out_attr2", Some('x'), &dependencies)
                    .unwrap_or_default();
            }

            if !dependencies.is_empty() {
                // Simplify the output name to save space.
                let output_name = format!("o{i}.{c}");
                output_dependencies.insert(output_name.into(), dependencies);
            }
        }
    }

    ShaderProgram {
        // IndexMap gives consistent ordering for attribute names.
        output_dependencies,
        color_layers,
        normal_layers,
    }
}

fn shader_from_latte_asm(
    _vertex: &str,
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
        color_layers: Vec::new(),
        normal_layers: Vec::new(),
    }
}

fn find_color_layers(
    frag: &Graph,
    output: &str,
    channel: Option<char>,
    dependencies: &[Dependency],
) -> Option<Vec<TextureLayer>> {
    let node_index = frag
        .nodes
        .iter()
        .rposition(|n| n.output.name == output && n.output.channel == channel)?;
    let last_node_index = *frag.node_dependencies_recursive(node_index, None).last()?;
    let last_node = frag.nodes.get(last_node_index)?;

    // matCol.xyz in pcmdo shaders.
    let mut current_col = assign_x(&frag.nodes, last_node)?;

    // This isn't always present for all materials in all games.
    // Xenoblade 1 DE and Xenoblade 3 both seem to do this for non map materials.
    if let Some((mat_cols, _monochrome_ratio)) = calc_monochrome(&frag.nodes, current_col) {
        // TODO: Select the appropriate channel.
        current_col = mat_cols[0];
    }

    let mut layers = Vec::new();

    // Shaders can blend multiple layers with getPixelCalcOver.
    // TODO: Store layering information.
    while let Some((mat_col, layer, ratio)) = mix_a_b_ratio(&frag.nodes, current_col) {
        let mut layer = layer;
        if let Some(n) = node_expr(&frag.nodes, layer) {
            layer = &assign_x_recursive(&frag.nodes, n).input;
        }

        // TODO: Some texture layers use parameters instead of texture color.
        if let Some((name, channel)) = texture_name_channel(layer) {
            // TODO: Should this ever be not none?
            let ratio = ratio_dependency(ratio, &frag.nodes, dependencies);
            layers.push(TextureLayer {
                name,
                channel,
                ratio,
            });
        }

        current_col = mat_col;
    }

    let base = assign_x_recursive(&frag.nodes, current_col);
    if let Some((name, channel)) = texture_name_channel(&base.input) {
        layers.push(TextureLayer {
            name,
            channel,
            ratio: None,
        });
    }

    // We start from the output, so these are in reverse order.
    layers.reverse();

    Some(layers)
}

fn calc_monochrome<'a>(nodes: &'a [Node], node: &'a Node) -> Option<([&'a Node; 3], &'a Expr)> {
    // calcMonochrome in pcmdo fragment shaders fro XC1 and XC3.
    // TODO: Check weight values for XC1 (0.3, 0.59, 0.11) or XC3 (0.01, 0.01, 0.01)?
    let (_mat_col, monochrome, monochrome_ratio) = mix_a_b_ratio(nodes, node)?;
    let monochrome = node_expr(nodes, monochrome)?;
    let (mat_col, _monochrome_weights) = dot3_a_b(nodes, monochrome)?;
    Some((mat_col, monochrome_ratio))
}

fn find_normal_layers(
    frag: &Graph,
    output: &str,
    channel: Option<char>,
    dependencies: &[Dependency],
) -> Option<Vec<TextureLayer>> {
    let node_index = frag
        .nodes
        .iter()
        .rposition(|n| n.output.name == output && n.output.channel == channel)?;
    let last_node_index = *frag.node_dependencies_recursive(node_index, None).last()?;
    let last_node = frag.nodes.get(last_node_index)?;

    let node = assign_x(&frag.nodes, last_node)?;

    // TODO: function for detecting fma?
    // setMrtNormal in pcmdo shaders.
    let view_normal = fma_half_half(&frag.nodes, node)?;
    let view_normal = assign_x_recursive(&frag.nodes, view_normal);
    let view_normal = normalize(&frag.nodes, view_normal)?;

    // TODO: front facing in calcNormalZAbs in pcmdo?

    // nomWork input for getCalcNormalMap in pcmdo shaders.
    let nom_work = calc_normal_map(frag, view_normal)?;
    let mut nom_work = nom_work[0];

    let mut layers = Vec::new();

    // Some shaders layer more than one additional normal map.
    while let Some((n2, layer_nom_work, ratio)) = pixel_calc_add_normal(&frag.nodes, nom_work) {
        if let Some((name, channel)) = texture_name_channel(&n2.input) {
            let ratio = ratio_dependency(ratio, &frag.nodes, dependencies);
            layers.push(TextureLayer {
                name,
                channel,
                ratio,
            });
        }
        if let Some(n1) = pixel_calc_add_normal_n1(&frag.nodes, layer_nom_work) {
            if let Some((name, channel)) = texture_name_channel(&n1.input) {
                layers.push(TextureLayer {
                    name,
                    channel,
                    ratio: None,
                });
            }
        }
        nom_work = layer_nom_work;
    }

    // We start from the output, so these are in reverse order.
    layers.reverse();

    Some(layers)
}

fn ratio_dependency(
    ratio: &Expr,
    nodes: &[Node],
    dependencies: &[Dependency],
) -> Option<Dependency> {
    // Reduce any assignment chains for what's likely a parameter or texture assignment.
    // TODO: Convert ratio to a dependency.
    let mut ratio = ratio;
    if let Some(n) = node_expr(nodes, ratio) {
        let n = assign_x_recursive(nodes, n);
        ratio = &n.input;
    }

    buffer_dependency(ratio)
        .map(Dependency::Buffer)
        .or_else(|| match ratio {
            Expr::Func {
                name,
                args,
                channel,
            } => {
                if name == "texture" {
                    if let Some(Expr::Global { name, .. }) = args.first() {
                        dependencies
                            .iter()
                            .find(|d| {
                                if let Dependency::Texture(t) = d {
                                    t.name == name && t.channels.contains(channel.unwrap())
                                } else {
                                    false
                                }
                            })
                            .cloned()
                    } else {
                        // TODO: How to handle this case?
                        None
                    }
                } else {
                    None
                }
            }
            // TODO: Find dependencies recursively?
            _ => None,
        })
}

fn texture_name_channel(input: &Expr) -> Option<(String, Option<char>)> {
    match input {
        Expr::Func {
            name,
            args,
            channel,
        } => {
            if name == "texture" {
                if let Some(Expr::Global { name, .. }) = args.first() {
                    Some((name.clone(), *channel))
                } else {
                    None
                }
            } else {
                None
            }
        }
        _ => None,
    }
}

fn pixel_calc_add_normal<'a>(
    nodes: &'a [Node],
    nom_work: &'a Node,
) -> Option<(&'a Node, &'a Node, &'a Expr)> {
    // getPixelCalcAddNormal in pcmdo shaders.
    // normalize(mix(nomWork, normalize(r), ratio))
    // XC2: ratio * (normalize(r) - nomWork) + nomWork
    // XC3: (normalize(r) - nomWork) * ratio + nomWork
    // TODO: Is it worth detecting the textures used for r?
    let result = normalize(nodes, nom_work)?;
    let (a, b, nom_work) = match &result.input {
        Expr::Func { name, args, .. } => {
            if name == "fma" {
                match &args[..] {
                    [a, b, Expr::Node { node_index, .. }] => Some((a, b, nodes.get(*node_index)?)),
                    _ => None,
                }
            } else {
                None
            }
        }
        _ => None,
    }?;
    // Detect ratio * r or r * ratio to handle both XC2 and XC3.
    let (n2, ratio) = node_expr(nodes, a)
        .and_then(|r| Some((pixel_calc_add_normal_n2(nodes, r)?, b)))
        .or_else(|| {
            node_expr(nodes, b).and_then(|r| Some((pixel_calc_add_normal_n2(nodes, r)?, a)))
        })?;

    Some((n2, nom_work, ratio))
}

fn node_expr<'a>(nodes: &'a [Node], e: &Expr) -> Option<&'a Node> {
    if let Expr::Node { node_index, .. } = e {
        nodes.get(*node_index)
    } else {
        None
    }
}

fn pixel_calc_add_normal_n1<'a>(nodes: &'a [Node], nom_work: &'a Node) -> Option<&'a Node> {
    // Extract the texture for n1 if present.
    // This will only work for the base layer.
    let node = match &nom_work.input {
        Expr::Func { name, args, .. } => {
            if name == "fma" {
                match &args[..] {
                    [Expr::Node { node_index, .. }, _, _] => nodes.get(*node_index),
                    _ => None,
                }
            } else {
                None
            }
        }
        _ => None,
    }?;
    let node = assign_x(nodes, node)?;
    Some(node)
}

fn pixel_calc_add_normal_n2<'a>(nodes: &'a [Node], r: &'a Node) -> Option<&'a Node> {
    // Extract n2 from the expression for r.
    // n2 = nor.x
    // n = fma(n2, 2.0, -1.0)
    // n = n * ???
    // n = 0.0 - n
    // n = ??? * ??? - n
    // n * inversesqrt(???) - n1.x
    let node = match &r.input {
        Expr::Func { name, args, .. } => {
            if name == "fma" {
                match &args[..] {
                    [Expr::Node { node_index, .. }, _, _] => nodes.get(*node_index),
                    _ => None,
                }
            } else {
                None
            }
        }
        _ => None,
    }?;
    let node = match &node.input {
        Expr::Func { name, args, .. } => {
            if name == "fma" {
                match &args[..] {
                    [_, _, Expr::Node { node_index, .. }] => nodes.get(*node_index),
                    _ => None,
                }
            } else {
                None
            }
        }
        _ => None,
    }?;
    let node = zero_minus_x(nodes, node)?;
    let node = match &node.input {
        Expr::Mul(x, y) => match (x.deref(), y.deref()) {
            (Expr::Node { node_index: x, .. }, _) => nodes.get(*x),
            _ => None,
        },
        _ => None,
    }?;
    let n2 = match &node.input {
        Expr::Func { name, args, .. } => {
            if name == "fma" {
                match &args[..] {
                    [Expr::Node { node_index, .. }, _, _] => nodes.get(*node_index),
                    _ => None,
                }
            } else {
                None
            }
        }
        _ => None,
    }?;
    let n2 = assign_x(nodes, n2)?;
    Some(n2)
}

fn calc_normal_map<'a>(frag: &'a Graph, view_normal: &'a Node) -> Option<[&'a Node; 3]> {
    // getCalcNormalMap in pcmdo shaders.
    // result = normalize(nomWork).x, normalize(tangent).x
    // result = fma(normalize(nomWork).y, normalize(bitangent).x, result)
    // result = fma(normalize(nomWork).z, normalize(normal).x, result)
    let (nrm, _tangent_normal_bitangent) = dot3_a_b(&frag.nodes, view_normal)?;
    Some(nrm)
}

fn geometric_specular_aa(frag: &Graph) -> Option<BufferDependency> {
    // TODO: is specular AA ever used with textures as input?
    // calcGeometricSpecularAA in pcmdo shaders.
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
    use smol_str::SmolStr;
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
        // Test shaders from Pyra's metallic chest material.
        // xeno2/bl/bl000101, "ho_BL_TS2", shd0022.vert
        let glsl = include_str!("data/bl000101.22.vert");
        let vertex = TranslationUnit::parse(glsl).unwrap();

        // xeno2/bl/bl000101, "ho_BL_TS2", shd0022.frag
        let glsl = include_str!("data/bl000101.22.frag");
        let fragment = TranslationUnit::parse(glsl).unwrap();

        let shader = shader_from_glsl(Some(&vertex), &fragment);

        assert!(shader.color_layers.is_empty());
        assert!(shader.normal_layers.is_empty());

        assert_eq!(
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
            })],
            shader.output_dependencies[&SmolStr::from("o1.x")]
        );
        assert_eq!(
            vec![Dependency::Buffer(BufferDependency {
                name: "U_Mate".into(),
                field: "gWrkFl4".into(),
                index: 2,
                channels: "x".into(),
            })],
            shader.output_dependencies[&SmolStr::from("o1.y")]
        );
        assert_eq!(
            vec![Dependency::Buffer(BufferDependency {
                name: "U_Mate".into(),
                field: "gWrkFl4".into(),
                index: 1,
                channels: "y".into(),
            })],
            shader.output_dependencies[&SmolStr::from("o1.z")]
        );
        assert_eq!(
            vec![Dependency::Constant(0.07098039.into())],
            shader.output_dependencies[&SmolStr::from("o1.w")]
        );
        assert_eq!(
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
            })],
            shader.output_dependencies[&SmolStr::from("o5.x")]
        );
        assert_eq!(
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
            })],
            shader.output_dependencies[&SmolStr::from("o5.y")]
        );
        assert_eq!(
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
            })],
            shader.output_dependencies[&SmolStr::from("o5.z")]
        );
        assert_eq!(
            vec![Dependency::Constant(0.0.into())],
            shader.output_dependencies[&SmolStr::from("o5.w")]
        );
    }

    #[test]
    fn shader_from_fragment_mio_skirt() {
        // xeno3/chr/ch/ch11021013, "body_skert2", shd0028.frag
        let glsl = include_str!("data/ch11021013.28.frag");

        // The pcmdo calcGeometricSpecularAA function compiles to the expression
        // glossiness = 1.0 - sqrt(clamp((1.0 - glossiness)^2 + kernelRoughness2 0.0, 1.0))
        // Consuming applications only care about the glossiness input.
        // This also avoids considering normal maps as a dependency.
        let fragment = TranslationUnit::parse(glsl).unwrap();
        let shader = shader_from_glsl(None, &fragment);

        assert_eq!(
            vec![
                TextureLayer {
                    name: "s0".to_string(),
                    channel: Some('x'),
                    ratio: None
                },
                TextureLayer {
                    name: "gTResidentTex04".to_string(),
                    channel: Some('x'),
                    ratio: None
                }
            ],
            shader.color_layers
        );
        assert_eq!(
            vec![
                TextureLayer {
                    name: "s2".to_string(),
                    channel: Some('x'),
                    ratio: None
                },
                TextureLayer {
                    name: "gTResidentTex09".to_string(),
                    channel: Some('x'),
                    ratio: Some(Dependency::Buffer(BufferDependency {
                        name: "U_Mate".into(),
                        field: "gWrkFl4".into(),
                        index: 1,
                        channels: "z".into()
                    }))
                }
            ],
            shader.normal_layers
        );

        assert_eq!(
            vec![Dependency::Buffer(BufferDependency {
                name: "U_Mate".into(),
                field: "gWrkFl4".into(),
                index: 2,
                channels: "y".into()
            })],
            shader.output_dependencies[&SmolStr::from("o1.y")]
        );
    }

    #[test]
    fn shader_from_fragment_mio_metal() {
        // xeno3/chr/ch/ch11021013, "tlent_mio_metal1", shd0031.frag
        let glsl = include_str!("data/ch11021013.31.frag");

        // Test multiple calls to getPixelCalcAddNormal.
        let fragment = TranslationUnit::parse(glsl).unwrap();
        let shader = shader_from_glsl(None, &fragment);
        // TODO: This is missing a non texture layer.
        assert_eq!(
            vec![
                TextureLayer {
                    name: "s0".to_string(),
                    channel: Some('x'),
                    ratio: None
                },
                TextureLayer {
                    name: "gTResidentTex04".to_string(),
                    channel: Some('x'),
                    ratio: None
                }
            ],
            shader.color_layers
        );
        assert_eq!(
            vec![
                TextureLayer {
                    name: "s2".to_string(),
                    channel: Some('x'),
                    ratio: None
                },
                TextureLayer {
                    name: "gTResidentTex09".to_string(),
                    channel: Some('x'),
                    ratio: Some(Dependency::Buffer(BufferDependency {
                        name: "U_Mate".into(),
                        field: "gWrkFl4".into(),
                        index: 2,
                        channels: "y".into()
                    }))
                },
                TextureLayer {
                    name: "s3".to_string(),
                    channel: Some('x'),
                    ratio: Some(Dependency::Buffer(BufferDependency {
                        name: "U_Mate".into(),
                        field: "gWrkFl4".into(),
                        index: 2,
                        channels: "z".into()
                    }))
                },
            ],
            shader.normal_layers
        );

        assert_eq!(
            vec![Dependency::Buffer(BufferDependency {
                name: "U_Mate".into(),
                field: "gWrkFl4".into(),
                index: 3,
                channels: "y".into()
            })],
            shader.output_dependencies[&SmolStr::from("o1.y")]
        );
        assert_eq!(
            vec![
                Dependency::Texture(TextureDependency {
                    name: "gTResidentTex09".into(),
                    channels: "x".into(),
                    texcoords: vec![
                        TexCoord {
                            name: "in_attr4".into(),
                            channels: "z".into(),
                            params: None,
                        },
                        TexCoord {
                            name: "in_attr4".into(),
                            channels: "w".into(),
                            params: None,
                        },
                    ],
                }),
                Dependency::Texture(TextureDependency {
                    name: "gTResidentTex09".into(),
                    channels: "y".into(),
                    texcoords: vec![
                        TexCoord {
                            name: "in_attr4".into(),
                            channels: "z".into(),
                            params: None,
                        },
                        TexCoord {
                            name: "in_attr4".into(),
                            channels: "w".into(),
                            params: None,
                        },
                    ],
                }),
                Dependency::Texture(TextureDependency {
                    name: "s2".into(),
                    channels: "x".into(),
                    texcoords: vec![
                        TexCoord {
                            name: "in_attr4".into(),
                            channels: "x".into(),
                            params: None,
                        },
                        TexCoord {
                            name: "in_attr4".into(),
                            channels: "y".into(),
                            params: None,
                        },
                    ],
                }),
                Dependency::Texture(TextureDependency {
                    name: "s2".into(),
                    channels: "y".into(),
                    texcoords: vec![
                        TexCoord {
                            name: "in_attr4".into(),
                            channels: "x".into(),
                            params: None,
                        },
                        TexCoord {
                            name: "in_attr4".into(),
                            channels: "y".into(),
                            params: None,
                        },
                    ],
                }),
                Dependency::Texture(TextureDependency {
                    name: "s3".into(),
                    channels: "x".into(),
                    texcoords: vec![
                        TexCoord {
                            name: "in_attr5".into(),
                            channels: "x".into(),
                            params: None,
                        },
                        TexCoord {
                            name: "in_attr5".into(),
                            channels: "y".into(),
                            params: None,
                        },
                    ],
                }),
                Dependency::Texture(TextureDependency {
                    name: "s3".into(),
                    channels: "y".into(),
                    texcoords: vec![
                        TexCoord {
                            name: "in_attr5".into(),
                            channels: "x".into(),
                            params: None,
                        },
                        TexCoord {
                            name: "in_attr5".into(),
                            channels: "y".into(),
                            params: None,
                        },
                    ],
                }),
            ],
            shader.output_dependencies[&SmolStr::from("o2.x")]
        );
    }

    #[test]
    fn shader_from_fragment_haze_body() {
        // xeno2/model/np/np001101, "body", shd0013.frag
        let glsl = include_str!("data/np001101.13.frag");

        // Test multiple normal layers with texture masks.
        let fragment = TranslationUnit::parse(glsl).unwrap();
        let shader = shader_from_glsl(None, &fragment);
        assert_eq!(
            vec![
                Dependency::Texture(TextureDependency {
                    name: "s2".into(),
                    channels: "x".into(),
                    texcoords: vec![
                        TexCoord {
                            name: "in_attr4".into(),
                            channels: "x".into(),
                            params: None,
                        },
                        TexCoord {
                            name: "in_attr4".into(),
                            channels: "y".into(),
                            params: None,
                        },
                    ],
                }),
                Dependency::Texture(TextureDependency {
                    name: "s2".into(),
                    channels: "y".into(),
                    texcoords: vec![
                        TexCoord {
                            name: "in_attr4".into(),
                            channels: "x".into(),
                            params: None,
                        },
                        TexCoord {
                            name: "in_attr4".into(),
                            channels: "y".into(),
                            params: None,
                        },
                    ],
                }),
                Dependency::Texture(TextureDependency {
                    name: "s3".into(),
                    channels: "x".into(),
                    texcoords: vec![
                        TexCoord {
                            name: "in_attr4".into(),
                            channels: "z".into(),
                            params: None,
                        },
                        TexCoord {
                            name: "in_attr4".into(),
                            channels: "w".into(),
                            params: None,
                        },
                    ],
                }),
                Dependency::Texture(TextureDependency {
                    name: "s3".into(),
                    channels: "y".into(),
                    texcoords: vec![
                        TexCoord {
                            name: "in_attr4".into(),
                            channels: "z".into(),
                            params: None,
                        },
                        TexCoord {
                            name: "in_attr4".into(),
                            channels: "w".into(),
                            params: None,
                        },
                    ],
                }),
                Dependency::Texture(TextureDependency {
                    name: "s4".into(),
                    channels: "x".into(),
                    texcoords: vec![
                        TexCoord {
                            name: "in_attr4".into(),
                            channels: "x".into(),
                            params: None,
                        },
                        TexCoord {
                            name: "in_attr4".into(),
                            channels: "y".into(),
                            params: None,
                        },
                    ],
                }),
                Dependency::Texture(TextureDependency {
                    name: "s5".into(),
                    channels: "x".into(),
                    texcoords: vec![
                        TexCoord {
                            name: "in_attr5".into(),
                            channels: "x".into(),
                            params: None,
                        },
                        TexCoord {
                            name: "in_attr5".into(),
                            channels: "y".into(),
                            params: None,
                        },
                    ],
                }),
                Dependency::Texture(TextureDependency {
                    name: "s5".into(),
                    channels: "y".into(),
                    texcoords: vec![
                        TexCoord {
                            name: "in_attr5".into(),
                            channels: "x".into(),
                            params: None,
                        },
                        TexCoord {
                            name: "in_attr5".into(),
                            channels: "y".into(),
                            params: None,
                        },
                    ],
                }),
                Dependency::Texture(TextureDependency {
                    name: "s6".into(),
                    channels: "x".into(),
                    texcoords: vec![
                        TexCoord {
                            name: "in_attr4".into(),
                            channels: "x".into(),
                            params: None,
                        },
                        TexCoord {
                            name: "in_attr4".into(),
                            channels: "y".into(),
                            params: None,
                        },
                    ],
                }),
            ],
            shader.output_dependencies[&SmolStr::from("o2.x")]
        );
    }

    #[test]
    fn shader_from_latte_asm_pc221115_frag_0() {
        // Elma's legs (visible on title screen).
        let asm = include_str!("data/pc221115.0.frag.txt");

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
                .into(),
                color_layers: Vec::new(),
                normal_layers: Vec::new()
            },
            shader
        );
    }
}
