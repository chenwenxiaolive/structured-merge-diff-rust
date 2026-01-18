//! smd - Structured Merge Diff CLI tool
//!
//! A command line tool for performing structured operations on YAML/JSON files.

use clap::{Args, Parser, Subcommand};
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use structured_merge_diff::typed::Parser as SchemaParser;
use structured_merge_diff::value;

#[derive(Parser)]
#[command(name = "smd")]
#[command(about = "Structured Merge Diff - CLI tool for structured operations on YAML/JSON files")]
#[command(version)]
struct Cli {
    /// Path to the schema file
    #[arg(short, long)]
    schema: PathBuf,

    /// Name of type in the schema to use. If empty, the first type will be used.
    #[arg(short = 't', long)]
    type_name: Option<String>,

    /// Output location. Use '-' for stdout.
    #[arg(short, long, default_value = "-")]
    output: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List all types in the schema
    ListTypes,

    /// Validate a YAML/JSON file against the schema
    Validate {
        /// Path to the file to validate
        file: PathBuf,
    },

    /// Merge two YAML/JSON files
    Merge(TwoFileArgs),

    /// Compare two YAML/JSON files
    Compare(TwoFileArgs),

    /// Build a fieldset from a YAML/JSON file
    Fieldset {
        /// Path to the file
        file: PathBuf,
    },
}

#[derive(Args)]
struct TwoFileArgs {
    /// Path to the left-hand side file
    #[arg(long)]
    lhs: PathBuf,

    /// Path to the right-hand side file
    #[arg(long)]
    rhs: PathBuf,
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    if let Err(e) = run(cli) {
        eprintln!("Error: {}", e);
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    // Read and parse schema
    let schema_content = fs::read_to_string(&cli.schema)
        .map_err(|e| format!("Failed to read schema file {:?}: {}", cli.schema, e))?;

    let parser = SchemaParser::new(&schema_content)
        .map_err(|e| format!("Failed to parse schema: {}", e))?;

    // Determine type name
    let type_name = cli.type_name.unwrap_or_else(|| {
        parser.type_names().first().map(|s| s.to_string()).unwrap_or_default()
    });

    if type_name.is_empty() {
        return Err("No types found in schema".into());
    }

    // Open output
    let mut output: Box<dyn Write> = if cli.output == "-" {
        Box::new(io::stdout())
    } else {
        Box::new(fs::File::create(&cli.output)
            .map_err(|e| format!("Failed to create output file {:?}: {}", cli.output, e))?)
    };

    // Execute command
    match cli.command {
        Commands::ListTypes => {
            list_types(&parser, &mut output)?;
        }
        Commands::Validate { file } => {
            validate(&parser, &type_name, &file, &mut output)?;
        }
        Commands::Merge(args) => {
            merge(&parser, &type_name, &args.lhs, &args.rhs, &mut output)?;
        }
        Commands::Compare(args) => {
            compare(&parser, &type_name, &args.lhs, &args.rhs, &mut output)?;
        }
        Commands::Fieldset { file } => {
            fieldset(&parser, &type_name, &file, &mut output)?;
        }
    }

    Ok(())
}

fn list_types(parser: &SchemaParser, output: &mut dyn Write) -> Result<(), Box<dyn std::error::Error>> {
    writeln!(output, "Types in schema:")?;
    for name in parser.type_names() {
        writeln!(output, "  - {}", name)?;
    }
    Ok(())
}

fn validate(
    parser: &SchemaParser,
    type_name: &str,
    file: &PathBuf,
    output: &mut dyn Write,
) -> Result<(), Box<dyn std::error::Error>> {
    let content = fs::read_to_string(file)
        .map_err(|e| format!("Failed to read file {:?}: {}", file, e))?;

    let pt = parser.type_by_name(type_name);
    if !pt.is_valid() {
        return Err(format!("Type '{}' not found in schema", type_name).into());
    }

    let typed_value = pt.from_yaml(&content)
        .map_err(|e| format!("Failed to parse file: {}", e))?;

    match typed_value.validate(&[]) {
        Ok(()) => {
            writeln!(output, "Validation successful")?;
        }
        Err(errors) => {
            writeln!(output, "Validation errors:")?;
            for err in errors.iter() {
                writeln!(output, "  - {}", err)?;
            }
            return Err("Validation failed".into());
        }
    }

    Ok(())
}

fn merge(
    parser: &SchemaParser,
    type_name: &str,
    lhs_file: &PathBuf,
    rhs_file: &PathBuf,
    output: &mut dyn Write,
) -> Result<(), Box<dyn std::error::Error>> {
    let lhs_content = fs::read_to_string(lhs_file)
        .map_err(|e| format!("Failed to read LHS file {:?}: {}", lhs_file, e))?;
    let rhs_content = fs::read_to_string(rhs_file)
        .map_err(|e| format!("Failed to read RHS file {:?}: {}", rhs_file, e))?;

    let pt = parser.type_by_name(type_name);
    if !pt.is_valid() {
        return Err(format!("Type '{}' not found in schema", type_name).into());
    }

    let lhs = pt.from_yaml(&lhs_content)
        .map_err(|e| format!("Failed to parse LHS: {}", e))?;
    let rhs = pt.from_yaml(&rhs_content)
        .map_err(|e| format!("Failed to parse RHS: {}", e))?;

    let merged = lhs.merge(&rhs)
        .map_err(|e| format!("Merge failed: {}", e))?;

    let yaml = value::to_yaml(merged.value())
        .map_err(|e| format!("Failed to serialize result: {}", e))?;

    write!(output, "{}", yaml)?;

    Ok(())
}

fn compare(
    parser: &SchemaParser,
    type_name: &str,
    lhs_file: &PathBuf,
    rhs_file: &PathBuf,
    output: &mut dyn Write,
) -> Result<(), Box<dyn std::error::Error>> {
    let lhs_content = fs::read_to_string(lhs_file)
        .map_err(|e| format!("Failed to read LHS file {:?}: {}", lhs_file, e))?;
    let rhs_content = fs::read_to_string(rhs_file)
        .map_err(|e| format!("Failed to read RHS file {:?}: {}", rhs_file, e))?;

    let pt = parser.type_by_name(type_name);
    if !pt.is_valid() {
        return Err(format!("Type '{}' not found in schema", type_name).into());
    }

    let lhs = pt.from_yaml(&lhs_content)
        .map_err(|e| format!("Failed to parse LHS: {}", e))?;
    let rhs = pt.from_yaml(&rhs_content)
        .map_err(|e| format!("Failed to parse RHS: {}", e))?;

    let comparison = lhs.compare(&rhs)
        .map_err(|e| format!("Compare failed: {}", e))?;

    if comparison.is_same() {
        writeln!(output, "Objects are identical")?;
    } else {
        writeln!(output, "Objects differ:")?;
        if comparison.has_added() {
            writeln!(output, "\nAdded fields:")?;
            comparison.added.iterate(|path| {
                writeln!(output, "  + {}", path).ok();
            });
        }
        if comparison.has_removed() {
            writeln!(output, "\nRemoved fields:")?;
            comparison.removed.iterate(|path| {
                writeln!(output, "  - {}", path).ok();
            });
        }
        if comparison.has_modified() {
            writeln!(output, "\nModified fields:")?;
            comparison.modified.iterate(|path| {
                writeln!(output, "  ~ {}", path).ok();
            });
        }
    }

    Ok(())
}

fn fieldset(
    parser: &SchemaParser,
    type_name: &str,
    file: &PathBuf,
    output: &mut dyn Write,
) -> Result<(), Box<dyn std::error::Error>> {
    let content = fs::read_to_string(file)
        .map_err(|e| format!("Failed to read file {:?}: {}", file, e))?;

    let pt = parser.type_by_name(type_name);
    if !pt.is_valid() {
        return Err(format!("Type '{}' not found in schema", type_name).into());
    }

    let typed_value = pt.from_yaml(&content)
        .map_err(|e| format!("Failed to parse file: {}", e))?;

    let field_set = typed_value.to_field_set()
        .map_err(|e| format!("Failed to build fieldset: {}", e))?;

    writeln!(output, "Fields:")?;
    field_set.iterate(|path| {
        writeln!(output, "  {}", path).ok();
    });

    Ok(())
}
