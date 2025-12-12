use clap::{Parser, ValueEnum};
use std::fs::File;
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::PathBuf;

use upml_rs::{parse_plantuml, generators, analysis, verification, Result};

#[derive(Debug, Clone, ValueEnum)]
enum Backend {
    None,
    SpinFsm,
    SpinHsm,
    TlaFsm,
    NuSmv,
    Alloy,
    Analyze,
    Verify,
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
        Backend::NuSmv => {
            // For now, use default configuration
            // TODO: Add CLI options for custom properties and fairness
            generators::nusmv::generate_model(output, &state_machine)?;
        }
        Backend::Alloy => {
            generators::alloy::generate_model(output, &state_machine)?;
        }
        Backend::Analyze => {
            analyze_state_machine(&state_machine)?;
        }
        Backend::Verify => {
            run_verification(&state_machine, &args)?;
        }
    }

    Ok(())
}

fn analyze_state_machine(state_machine: &upml_rs::StateMachine) -> Result<()> {
    use analysis::{StateMachineValidator, MetricsAnalyzer, ReachabilityAnalyzer};

    println!("🔍 State Machine Analysis Report");
    println!("================================");
    println!();

    // Basic information
    println!("📊 Basic Information:");
    println!("  • State Machine ID: {}", state_machine.id);
    println!("  • Total States: {}", state_machine.states(false).len());
    println!("  • Total Transitions: {}", state_machine.transitions().len());
    println!("  • Total Events: {}", state_machine.events().len());
    println!("  • Hierarchy Depth: {}", state_machine.depth());
    println!();

    // Validation
    println!("✅ Validation Results:");
    let validator = StateMachineValidator::new();
    let validation_report = validator.validate(state_machine)?;
    
    println!("  • Errors: {}", validation_report.summary.errors);
    println!("  • Warnings: {}", validation_report.summary.warnings);
    println!("  • Info: {}", validation_report.summary.infos);
    
    if !validation_report.issues.is_empty() {
        println!();
        println!("🚨 Issues Found:");
        for issue in &validation_report.issues {
            let icon = match issue.severity {
                analysis::Severity::Error => "❌",
                analysis::Severity::Warning => "⚠️",
                analysis::Severity::Info => "ℹ️",
            };
            println!("  {} {}: {}", icon, issue.issue_type, issue.message);
            if let Some(location) = &issue.location {
                println!("     Location: {}", location);
            }
            if !issue.suggestions.is_empty() {
                println!("     Suggestions:");
                for suggestion in &issue.suggestions {
                    println!("       • {}", suggestion);
                }
            }
            println!();
        }
    }

    // Complexity metrics
    println!("📈 Complexity Metrics:");
    let metrics_analyzer = MetricsAnalyzer::new();
    let complexity = metrics_analyzer.analyze_complexity(state_machine)?;
    
    println!("  • Cyclomatic Complexity: {}", complexity.cyclomatic_complexity);
    println!("  • State Complexity: {}", complexity.state_complexity);
    println!("  • Transition Complexity: {}", complexity.transition_complexity);
    println!("  • Depth Complexity: {}", complexity.depth_complexity);
    println!("  • Average Fan-out: {:.2}", complexity.fan_out_complexity);
    println!("  • Average Fan-in: {:.2}", complexity.fan_in_complexity);
    println!("  • Coupling Factor: {:.2}", complexity.coupling_factor);
    println!("  • Cohesion Factor: {:.2}", complexity.cohesion_factor);
    println!();

    // Reachability analysis
    println!("🎯 Reachability Analysis:");
    let reachability_analyzer = ReachabilityAnalyzer::new();
    let reachability = reachability_analyzer.analyze(state_machine)?;
    
    println!("  • Reachable States: {}", reachability.reachable_states.len());
    println!("  • Unreachable States: {}", reachability.unreachable_states.len());
    
    if !reachability.unreachable_states.is_empty() {
        println!("    Unreachable: {:?}", reachability.unreachable_states);
    }
    
    println!("  • Strongly Connected Components: {}", reachability.strongly_connected_components.len());
    
    if reachability.strongly_connected_components.len() > 1 {
        println!("    Components:");
        for (i, component) in reachability.strongly_connected_components.iter().enumerate() {
            if component.len() > 1 {
                println!("      {}: {:?}", i + 1, component);
            }
        }
    }
    println!();

    // State-specific metrics
    println!("🏛️ State Analysis:");
    let state_metrics = metrics_analyzer.analyze_states(state_machine)?;
    
    for metrics in &state_metrics {
        if metrics.is_critical || metrics.fan_in > 2 || metrics.fan_out > 2 {
            let critical_marker = if metrics.is_critical { " (Critical)" } else { "" };
            println!("  • {}{}: in={}, out={}, depth={}", 
                     metrics.id, critical_marker, metrics.fan_in, metrics.fan_out, metrics.depth_level);
        }
    }
    
    if state_metrics.iter().all(|m| !m.is_critical && m.fan_in <= 2 && m.fan_out <= 2) {
        println!("  • All states have normal complexity");
    }
    println!();

    // Summary
    let quality_score = calculate_quality_score(&validation_report, &complexity);
    println!("🎖️ Overall Quality Score: {:.1}/10", quality_score);
    
    let quality_level = match quality_score as u8 {
        9..=10 => "Excellent",
        7..=8 => "Good", 
        5..=6 => "Fair",
        3..=4 => "Poor",
        _ => "Needs Improvement",
    };
    println!("   Quality Level: {}", quality_level);

    Ok(())
}

fn calculate_quality_score(validation_report: &analysis::ValidationReport, complexity: &analysis::ComplexityMetrics) -> f64 {
    let mut score = 10.0;
    
    // Deduct for validation issues
    score -= validation_report.summary.errors as f64 * 2.0;
    score -= validation_report.summary.warnings as f64 * 0.5;
    
    // Deduct for high complexity
    if complexity.cyclomatic_complexity > 10 {
        score -= 1.0;
    }
    if complexity.coupling_factor > 0.5 {
        score -= 1.0;
    }
    if complexity.fan_out_complexity > 3.0 {
        score -= 0.5;
    }
    
    // Bonus for good cohesion
    if complexity.cohesion_factor > 0.7 {
        score += 0.5;
    }
    
    score.max(0.0).min(10.0)
}

fn run_verification(state_machine: &upml_rs::StateMachine, args: &Args) -> Result<()> {
    use verification::{VerificationRunner, VerificationTool, RunConfig};
    use std::collections::HashMap;
    use std::path::PathBuf;

    println!("🚀 Automated Verification");
    println!("=========================\n");

    // Detect available tools
    let available_tools = VerificationRunner::detect_available_tools();
    println!("🔍 Detected verification tools:");
    for tool in &available_tools {
        println!("  ✅ {}", tool.display_name());
    }
    
    if available_tools.is_empty() {
        println!("❌ No verification tools found!");
        println!("   Please install one or more of: SPIN, TLA+, NuSMV, Alloy");
        return Ok(());
    }
    println!();

    // Generate models for all available tools
    println!("📄 Generating verification models...");
    let mut model_files = HashMap::new();
    let base_name = args.input
        .as_ref()
        .and_then(|p| p.file_stem())
        .and_then(|s| s.to_str())
        .unwrap_or("model");

    for tool in &available_tools {
        let filename = format!("{}_{}.{}", base_name, tool.executable(), tool.file_extension());
        let path = PathBuf::from(&filename);
        
        let mut file = std::fs::File::create(&path)?;
        
        match tool {
            VerificationTool::Spin => {
                generators::promela::generate_fsm(&mut file, state_machine)?;
            }
            VerificationTool::TlaPlus => {
                generators::tla::generate_fsm(&mut file, state_machine)?;
            }
            VerificationTool::NuSMV => {
                generators::nusmv::generate_model(&mut file, state_machine)?;
            }
            VerificationTool::Alloy => {
                generators::alloy::generate_model(&mut file, state_machine)?;
            }
        }
        
        model_files.insert(tool.clone(), path.clone());
        println!("  ✅ Generated {}", filename);
    }
    println!();

    // Configure verification runner
    let config = RunConfig {
        timeout: Some(300), // 5 minutes per tool
        work_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        parallel: true,
        keep_files: true, // Keep generated files for inspection
        tool_args: HashMap::new(),
    };

    // Run verification
    println!("🔧 Running verification tools...");
    let runner = VerificationRunner::new(config);
    let results = runner.run_all(&model_files)?;

    // Display results
    results.print_report();

    // Save results to JSON
    let json_results = results.to_json()?;
    let results_file = format!("{}_verification_results.json", base_name);
    std::fs::write(&results_file, json_results)?;
    println!("\n💾 Detailed results saved to: {}", results_file);

    Ok(())
}