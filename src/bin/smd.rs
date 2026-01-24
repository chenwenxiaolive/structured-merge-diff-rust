//! smd - Structured Merge Diff CLI tool
//!
//! A command line tool for performing structured operations on YAML/JSON files.

use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use structured_merge_diff::typed::Parser as SchemaParser;
use structured_merge_diff::value;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn print_help() {
    eprintln!(
        r#"smd {} - Structured Merge Diff CLI tool

USAGE:
    smd [OPTIONS] <COMMAND>

OPTIONS:
    -s, --schema <FILE>      Path to the schema file (required)
    -t, --type-name <NAME>   Name of type in the schema to use
    -o, --output <FILE>      Output location. Use '-' for stdout (default: -)
    -h, --help               Print help information
    -V, --version            Print version information

COMMANDS:
    list-types               List all types in the schema
    validate <FILE>          Validate a YAML/JSON file against the schema
    merge --lhs <FILE> --rhs <FILE>
                             Merge two YAML/JSON files
    compare --lhs <FILE> --rhs <FILE>
                             Compare two YAML/JSON files
    fieldset <FILE>          Build a fieldset from a YAML/JSON file
"#,
        VERSION
    );
}

fn print_version() {
    println!("smd {}", VERSION);
}

#[derive(Debug)]
struct Cli {
    schema: PathBuf,
    type_name: Option<String>,
    output: String,
    command: Command,
}

#[derive(Debug)]
enum Command {
    ListTypes,
    Validate { file: PathBuf },
    Merge { lhs: PathBuf, rhs: PathBuf },
    Compare { lhs: PathBuf, rhs: PathBuf },
    Fieldset { file: PathBuf },
}

fn parse_args() -> Result<Cli, String> {
    let args: Vec<String> = env::args().collect();
    let mut i = 1;

    let mut schema: Option<PathBuf> = None;
    let mut type_name: Option<String> = None;
    let mut output = "-".to_string();
    let mut command: Option<Command> = None;

    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => {
                print_help();
                std::process::exit(0);
            }
            "-V" | "--version" => {
                print_version();
                std::process::exit(0);
            }
            "-s" | "--schema" => {
                i += 1;
                if i >= args.len() {
                    return Err("Missing value for --schema".to_string());
                }
                schema = Some(PathBuf::from(&args[i]));
            }
            "-t" | "--type-name" => {
                i += 1;
                if i >= args.len() {
                    return Err("Missing value for --type-name".to_string());
                }
                type_name = Some(args[i].clone());
            }
            "-o" | "--output" => {
                i += 1;
                if i >= args.len() {
                    return Err("Missing value for --output".to_string());
                }
                output = args[i].clone();
            }
            "list-types" => {
                command = Some(Command::ListTypes);
            }
            "validate" => {
                i += 1;
                if i >= args.len() {
                    return Err("Missing file argument for validate".to_string());
                }
                command = Some(Command::Validate {
                    file: PathBuf::from(&args[i]),
                });
            }
            "merge" => {
                let mut lhs: Option<PathBuf> = None;
                let mut rhs: Option<PathBuf> = None;
                i += 1;
                while i < args.len() {
                    match args[i].as_str() {
                        "--lhs" => {
                            i += 1;
                            if i >= args.len() {
                                return Err("Missing value for --lhs".to_string());
                            }
                            lhs = Some(PathBuf::from(&args[i]));
                        }
                        "--rhs" => {
                            i += 1;
                            if i >= args.len() {
                                return Err("Missing value for --rhs".to_string());
                            }
                            rhs = Some(PathBuf::from(&args[i]));
                        }
                        _ => {
                            i -= 1;
                            break;
                        }
                    }
                    i += 1;
                }
                match (lhs, rhs) {
                    (Some(l), Some(r)) => {
                        command = Some(Command::Merge { lhs: l, rhs: r });
                    }
                    _ => {
                        return Err("merge requires --lhs and --rhs arguments".to_string());
                    }
                }
            }
            "compare" => {
                let mut lhs: Option<PathBuf> = None;
                let mut rhs: Option<PathBuf> = None;
                i += 1;
                while i < args.len() {
                    match args[i].as_str() {
                        "--lhs" => {
                            i += 1;
                            if i >= args.len() {
                                return Err("Missing value for --lhs".to_string());
                            }
                            lhs = Some(PathBuf::from(&args[i]));
                        }
                        "--rhs" => {
                            i += 1;
                            if i >= args.len() {
                                return Err("Missing value for --rhs".to_string());
                            }
                            rhs = Some(PathBuf::from(&args[i]));
                        }
                        _ => {
                            i -= 1;
                            break;
                        }
                    }
                    i += 1;
                }
                match (lhs, rhs) {
                    (Some(l), Some(r)) => {
                        command = Some(Command::Compare { lhs: l, rhs: r });
                    }
                    _ => {
                        return Err("compare requires --lhs and --rhs arguments".to_string());
                    }
                }
            }
            "fieldset" => {
                i += 1;
                if i >= args.len() {
                    return Err("Missing file argument for fieldset".to_string());
                }
                command = Some(Command::Fieldset {
                    file: PathBuf::from(&args[i]),
                });
            }
            arg => {
                return Err(format!("Unknown argument: {}", arg));
            }
        }
        i += 1;
    }

    let schema = schema.ok_or_else(|| "Missing required --schema argument".to_string())?;
    let command = command.ok_or_else(|| "Missing command".to_string())?;

    Ok(Cli {
        schema,
        type_name,
        output,
        command,
    })
}

fn main() -> ExitCode {
    let cli = match parse_args() {
        Ok(cli) => cli,
        Err(e) => {
            eprintln!("Error: {}", e);
            eprintln!();
            print_help();
            return ExitCode::FAILURE;
        }
    };

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
        Command::ListTypes => {
            list_types(&parser, &mut output)?;
        }
        Command::Validate { file } => {
            validate(&parser, &type_name, &file, &mut output)?;
        }
        Command::Merge { lhs, rhs } => {
            merge(&parser, &type_name, &lhs, &rhs, &mut output)?;
        }
        Command::Compare { lhs, rhs } => {
            compare(&parser, &type_name, &lhs, &rhs, &mut output)?;
        }
        Command::Fieldset { file } => {
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
