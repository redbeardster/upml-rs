use std::collections::HashMap;
use std::path::PathBuf;
use upml_rs::{parse_plantuml, generators, verification, Result};
use upml_rs::verification::{VerificationRunner, VerificationTool, RunConfig};

fn main() -> Result<()> {
    println!("🚀 UPML-RS Automated Verification Example");
    println!("==========================================\n");

    // Example PlantUML state machine
    let plantuml_content = r#"
        @startuml ATM_System
        [*] --> Idle
        
        state Idle {
            Idle --> CardInserted : insert_card
        }
        
        state CardInserted {
            CardInserted --> PinEntry : card_valid
            CardInserted --> Idle : card_invalid / eject_card()
        }
        
        state PinEntry {
            PinEntry --> Authenticated : pin_correct [attempts < 3]
            PinEntry --> PinEntry : pin_incorrect [attempts < 3] / increment_attempts()
            PinEntry --> CardBlocked : pin_incorrect [attempts >= 3] / block_card()
        }
        
        state Authenticated {
            Authenticated --> SelectTransaction : authenticated
        }
        
        state SelectTransaction {
            SelectTransaction --> WithdrawCash : select_withdraw
            SelectTransaction --> CheckBalance : select_balance
            SelectTransaction --> Idle : cancel / eject_card()
        }
        
        state WithdrawCash {
            WithdrawCash --> DispensingCash : amount_valid [balance >= amount]
            WithdrawCash --> SelectTransaction : amount_invalid
        }
        
        state DispensingCash {
            DispensingCash --> Idle : cash_dispensed / eject_card()
        }
        
        state CheckBalance {
            CheckBalance --> SelectTransaction : balance_shown
        }
        
        state CardBlocked {
            CardBlocked --> Idle : timeout / eject_card()
        }
        
        @enduml
    "#;

    println!("📋 Parsing PlantUML state machine...");
    let state_machine = parse_plantuml(plantuml_content)?;
    
    println!("✅ Parsed successfully!");
    println!("   • States: {}", state_machine.states(false).len());
    println!("   • Transitions: {}", state_machine.transitions().len());
    println!("   • Events: {}", state_machine.events().len());
    println!("   • Hierarchy Depth: {}", state_machine.depth());
    println!();

    // Detect available verification tools
    println!("🔍 Detecting available verification tools...");
    let available_tools = VerificationRunner::detect_available_tools();
    
    if available_tools.is_empty() {
        println!("❌ No verification tools found!");
        println!("   Please install one or more of: SPIN, TLA+, NuSMV, Alloy");
        return Ok(());
    }
    
    println!("✅ Found {} verification tool(s):", available_tools.len());
    for tool in &available_tools {
        println!("   • {}", tool.display_name());
    }
    println!();

    // Generate models for all available tools
    println!("📄 Generating verification models...");
    let mut model_files = HashMap::new();
    
    for tool in &available_tools {
        let filename = format!("atm_system.{}", tool.file_extension());
        let path = PathBuf::from(&filename);
        
        let mut file = std::fs::File::create(&path)?;
        
        match tool {
            VerificationTool::Spin => {
                generators::promela::generate_fsm(&mut file, &state_machine)?;
                println!("   ✅ Generated Promela model: {}", filename);
            }
            VerificationTool::TlaPlus => {
                generators::tla::generate_fsm(&mut file, &state_machine)?;
                println!("   ✅ Generated TLA+ model: {}", filename);
            }
            VerificationTool::NuSMV => {
                generators::nusmv::generate_model(&mut file, &state_machine)?;
                println!("   ✅ Generated NuSMV model: {}", filename);
            }
            VerificationTool::Alloy => {
                generators::alloy::generate_model(&mut file, &state_machine)?;
                println!("   ✅ Generated Alloy model: {}", filename);
            }
        }
        
        model_files.insert(tool.clone(), path);
    }
    println!();

    // Configure verification runner
    println!("⚙️  Configuring verification runner...");
    let config = RunConfig {
        timeout: Some(300), // 5 minutes per tool
        work_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        parallel: true, // Run tools in parallel for speed
        keep_files: true, // Keep generated files for inspection
        tool_args: HashMap::new(),
    };
    
    println!("   • Timeout: {} seconds per tool", config.timeout.unwrap_or(0));
    println!("   • Parallel execution: {}", config.parallel);
    println!("   • Keep files: {}", config.keep_files);
    println!();

    // Run automated verification
    println!("🔧 Running automated verification...");
    let runner = VerificationRunner::new(config);
    let results = runner.run_all(&model_files)?;

    // Display comprehensive results
    println!();
    results.print_report();

    // Demonstrate programmatic access to results
    println!("\n🔍 Detailed Analysis:");
    println!("=====================");
    
    for (tool, result) in &results.tool_results {
        println!("\n{} Results:", tool.display_name());
        println!("  • Status: {:?}", result.status);
        println!("  • Execution Time: {:.2}s", result.execution_time.as_secs_f64());
        println!("  • Exit Code: {:?}", result.exit_code);
        
        if !result.stdout.is_empty() {
            println!("  • Output Preview: {}", 
                     result.stdout.lines().take(3).collect::<Vec<_>>().join(" | "));
        }
        
        if !result.stderr.is_empty() {
            println!("  • Errors: {}", 
                     result.stderr.lines().take(2).collect::<Vec<_>>().join(" | "));
        }
    }

    // Save results for later analysis
    let json_results = results.to_json()?;
    let results_file = "atm_system_verification_results.json";
    std::fs::write(results_file, json_results)?;
    println!("\n💾 Detailed results saved to: {}", results_file);

    // Provide recommendations based on results
    println!("\n💡 Recommendations:");
    match results.overall_status {
        verification::VerificationStatus::Success => {
            println!("   ✅ All verification tools passed successfully!");
            println!("   ✅ Your state machine model appears to be correct.");
            println!("   ✅ Consider adding more complex properties for deeper verification.");
        }
        verification::VerificationStatus::Failed => {
            println!("   ⚠️  Some verification tools found issues:");
            println!("   • Review the counterexamples in the detailed output");
            println!("   • Check for unreachable states or deadlocks");
            println!("   • Verify that your model matches intended behavior");
        }
        verification::VerificationStatus::Error => {
            println!("   ❌ Verification encountered errors:");
            println!("   • Check that all verification tools are properly installed");
            println!("   • Verify that generated models are syntactically correct");
            println!("   • Consider simplifying the model if it's too complex");
        }
    }

    println!("\n🎯 Next Steps:");
    println!("   1. Review generated model files for accuracy");
    println!("   2. Run individual tools with custom properties if needed");
    println!("   3. Use the analysis backend for detailed complexity metrics");
    println!("   4. Integrate verification into your CI/CD pipeline");

    Ok(())
}