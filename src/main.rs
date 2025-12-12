use clap::{Parser, ValueEnum};
use std::fs::File;
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::PathBuf;

use upml_rs::{parse_plantuml, generators, Result};

#[derive(Debug, Clone, ValueEnum)]
enum Backend {
    None,
    SpinFsm,
    SpinHsm,
    TlaFsm,
}

#[derive(Parser)]
#[command(name = "upml")]
#[command(about = "Plantuml state machine to spin/Promela or TLA+/PlusCal")]
#[command(version = upml_rs::VERSION)]
struct Args {
    /// Backend to use for code generation
    #[arg(short, long, value_enum, default_value = "none")]
    backend: Backend,

    /// Plantuml input file (default: stdin)
    #[arg(short, long)]
    input: Option<PathBuf>,

    /// Backend output file (default: stdout)
    #[arg(short, long)]
    output: Option<PathBuf>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Read input
    let input_content = match &args.input {
        Some(path) => {
            let file = File::open(path)?;
            let mut reader = BufReader::new(file);
            let mut content = String::new();
            reader.read_to_string(&mut content)?;
            content
        }
        None => {
            let mut content = String::new();
            io::stdin().read_to_string(&mut content)?;
            content
        }
    };

    // Parse PlantUML
    let mut state_machine = parse_plantuml(&input_content)?;
    
    // Set state machine ID from input filename if available
    if let Some(input_path) = &args.input {
        if let Some(stem) = input_path.file_stem() {
            state_machine.set_id(stem.to_string_lossy().to_string());
        }
    }

    // Prepare output writer
    let output: Box<dyn Write> = match &args.output {
        Some(path) => {
            let file = File::create(path)?;
            Box::new(BufWriter::new(file))
        }
        None => Box::new(io::stdout()),
    };

    // Generate output based on backend
    match args.backend {
        Backend::None => {
            // Just validate parsing, no output
            println!("Parsing successful. State machine has {} states.", 
                     state_machine.states(false).len());
        }
        Backend::SpinFsm => {
            generators::promela::generate_fsm(output, &state_machine)?;
        }
        Backend::SpinHsm => {
            generators::promela::generate_hsm(output, &state_machine)?;
        }
        Backend::TlaFsm => {
            generators::tla::generate_fsm(output, &state_machine)?;
        }
    }

    Ok(())
}