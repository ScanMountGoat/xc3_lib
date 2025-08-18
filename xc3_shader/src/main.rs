use std::path::Path;

use clap::{Parser, Subcommand};

use glsl_lang::ast::TranslationUnit;
use glsl_lang::parse::DefaultParse;
use xc3_model::shader_database::ShaderDatabase;
use xc3_shader::dependencies::latte_dependencies;
use xc3_shader::extract::{
    annotate_all_legacy_shaders, extract_all_legacy_shaders, extract_and_decompile_shaders,
};
use xc3_shader::graph::Graph;
use xc3_shader::shader_database::{
    create_shader_database, create_shader_database_legacy, shader_from_glsl, shader_graphviz,
    shader_str,
};

use xc3_shader::graph::glsl::{glsl_dependencies, shader_source_no_extensions};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Extract and decompile shaders into a folder for each .wimdo or .wismhd file.
    /// JSON metadata for each program will also be saved in the output folder.
    DecompileShaders {
        /// The root folder for Xenoblade 1 DE, Xenoblade 2, or Xenoblade 3.
        input_folder: String,
        /// The output folder for the decompiled shaders.
        output_folder: String,
        /// The path to the Ryujinx.ShaderTools executable
        shader_tools: Option<String>,
    },
    /// Extract and disassemble shaders into a folder for each .camdo file.
    DisassembleLegacyShaders {
        /// The root folder for Xenoblade X.
        input_folder: String,
        /// The output folder for the disassembled shaders.
        output_folder: String,
        /// The path to the gfd-tool executable
        gfd_tool: String,
    },
    /// Create annotated GLSL shaders for each .camdo file.
    AnnotateLegacyShaders {
        /// The root folder for Xenoblade X.
        input_folder: String,
        /// The output folder for the shaders.
        output_folder: String,
    },
    /// Create a database of decompiled shader data.
    ShaderDatabase {
        /// The output folder from decompiling shaders.
        input_folder: String,
        /// The output database file.
        output_file: String,
    },
    /// Create a database of decompiled shader data for Xenoblade X.
    ShaderDatabaseLegacy {
        /// The output folder from decompiling shaders.
        input_folder: String,
        /// The output database file.
        output_file: String,
    },
    /// Create a combined database of decompiled shader data.
    MergeDatabases {
        /// The output database file.
        output_file: String,
        /// The input database files.
        input_files: Vec<String>,
    },
    /// Find all lines of GLSL code influencing the final assignment of a variable.
    GlslDependencies {
        /// The input GLSL file.
        input: String,
        /// The output GLSL file.
        output: String,
        /// The name of the variable to analyze.
        var: String,
    },
    /// Find all lines of GLSL code influencing the final assignment of a variable.
    LatteDependencies {
        /// The input Latte ASM file.
        input: String,
        /// The output GLSL file.
        output: String,
        /// The name of the variable to analyze.
        var: String,
    },
    /// Convert Wii U Latte shader assembly to GLSL.
    LatteGlsl {
        /// The input Latte ASM file.
        input: String,
        /// The output GLSL file.
        output: String,
    },
    /// Find output dependencies for the given GLSL shader program.
    GlslOutputDependencies {
        /// The input fragment GLSL file.
        frag: String,
        /// The output txt or Graphviz dot file.
        output: String,
    },
}

fn main() {
    simple_logger::SimpleLogger::new()
        .with_level(log::LevelFilter::Warn)
        .init()
        .unwrap();

    let cli = Cli::parse();

    let start = std::time::Instant::now();
    // TODO: make annotation optional
    match cli.command {
        Commands::DecompileShaders {
            input_folder,
            output_folder,
            shader_tools,
        } => extract_and_decompile_shaders(&input_folder, &output_folder, shader_tools.as_deref()),
        Commands::DisassembleLegacyShaders {
            input_folder,
            output_folder,
            gfd_tool,
        } => extract_all_legacy_shaders(&input_folder, &output_folder, &gfd_tool),
        Commands::AnnotateLegacyShaders {
            input_folder,
            output_folder,
        } => annotate_all_legacy_shaders(&input_folder, &output_folder),
        Commands::ShaderDatabase {
            input_folder,
            output_file,
        } => {
            let database = create_shader_database(&input_folder);
            database.save(output_file).unwrap();
        }
        Commands::ShaderDatabaseLegacy {
            input_folder,
            output_file,
        } => {
            let database = create_shader_database_legacy(&input_folder);
            database.save(output_file).unwrap();
        }
        Commands::GlslDependencies { input, output, var } => {
            let source = std::fs::read_to_string(input).unwrap();
            let (var, channels) = var.split_once('.').unwrap_or((&var, ""));
            let source_out = glsl_dependencies(&source, var, channels.chars().next());
            std::fs::write(output, source_out).unwrap();
        }
        Commands::LatteDependencies { input, output, var } => {
            let source = std::fs::read_to_string(input).unwrap();
            let (var, channels) = var.split_once('.').unwrap_or((&var, ""));
            let source_out = latte_dependencies(&source, var, channels.chars().next());
            std::fs::write(output, source_out).unwrap();
        }
        Commands::MergeDatabases {
            input_files,
            output_file,
        } => {
            if let Some((merged, others)) = input_files.split_first() {
                let base = ShaderDatabase::from_file(merged).unwrap();
                let others: Vec<_> = others
                    .iter()
                    .map(|o| ShaderDatabase::from_file(o).unwrap())
                    .collect();
                let merged = base.merge(others.into_iter());
                merged.save(output_file).unwrap();
            }
        }
        Commands::LatteGlsl { input, output } => {
            let asm = std::fs::read_to_string(input).unwrap();
            let graph = Graph::from_latte_asm(&asm);
            std::fs::write(output, graph.to_glsl()).unwrap();
        }
        Commands::GlslOutputDependencies { frag, output } => {
            let frag_glsl = std::fs::read_to_string(&frag).unwrap();
            let frag_glsl = shader_source_no_extensions(&frag_glsl);
            let fragment = TranslationUnit::parse(frag_glsl).unwrap();

            // TODO: make an argument for this?
            let vert = std::fs::read_to_string(Path::new(&frag).with_extension("vert"))
                .ok()
                .map(|v| TranslationUnit::parse(&v).unwrap());

            let shader = shader_from_glsl(vert.as_ref(), &fragment);
            if output.ends_with(".dot") {
                std::fs::write(output, shader_graphviz(&shader)).unwrap();
            } else {
                std::fs::write(output, shader_str(&shader)).unwrap();
            }
        }
    }
    println!("Finished in {:?}", start.elapsed());
}
