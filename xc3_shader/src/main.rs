use clap::{Parser, Subcommand};

use xc3_model::shader_database::ShaderDatabase;
use xc3_shader::dependencies::latte_dependencies;
use xc3_shader::extract::{extract_and_decompile_shaders, extract_and_disassemble_shaders};
use xc3_shader::graph::Graph;
use xc3_shader::shader_database::{create_shader_database, create_shader_database_legacy};

use xc3_shader::graph::glsl::glsl_dependencies;

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
        } => extract_and_disassemble_shaders(&input_folder, &output_folder, &gfd_tool),
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
            if let Some(merged) = input_files
                .iter()
                .map(|path| ShaderDatabase::from_file(path).unwrap())
                .reduce(|a, b| ShaderDatabase::merge(&a, &b))
            {
                merged.save(output_file).unwrap();
            }
        }
        Commands::LatteGlsl { input, output } => {
            let asm = std::fs::read_to_string(input).unwrap();
            let graph = Graph::from_latte_asm(&asm);
            std::fs::write(output, graph.to_glsl()).unwrap();
        }
    }

    println!("Finished in {:?}", start.elapsed());
}
