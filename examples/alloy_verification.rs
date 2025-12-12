use upml_rs::{parse_plantuml, generators::alloy};
use std::fs::File;

fn main() -> upml_rs::Result<()> {
    println!("🏗️ Alloy Structural Verification Demo");
    println!("====================================\n");

    // Example state machine with potential structural issues
    let plantuml = r#"
        @startuml
        [*] --> Idle
        [*] --> Standby
        
        Idle --> Processing : Start
        Standby --> Processing : Activate
        
        Processing --> Completed : Success
        Processing --> Error : Failure
        Processing --> Processing : Continue
        
        Error --> Processing : Retry
        Error --> Idle : Reset
        
        Completed --> [*]
        
        // Orphaned states for testing
        OrphanState --> DeadEnd : SomeEvent
        @enduml
    "#;

    println!("📋 Parsing PlantUML state machine...");
    let state_machine = parse_plantuml(plantuml)?;
    
    println!("✅ Parsed successfully!");
    println!("   • States: {}", state_machine.states(false).len());
    println!("   • Initial states: {:?}", state_machine.initial_states());
    println!("   • Final states: {:?}", state_machine.final_states());

    println!("\n🔧 Generating Alloy structural model...");
    
    // Create configuration for structural analysis
    let config = alloy::AlloyConfig {
        max_states: 12,
        max_events: 10,
        max_transitions: 20,
        structural_only: false, // Include behavioral predicates
    };
    
    // Generate Alloy model
    let mut file = File::create("structural_verification.als")?;
    alloy::generate_model_with_config(&mut file, &state_machine, &config)?;
    
    println!("✅ Alloy model saved to: structural_verification.als");
    
    println!("\n📊 Generated Analysis Features:");
    println!("   • Structural constraints (states, transitions, events)");
    println!("   • Well-formedness predicates");
    println!("   • Determinism checking");
    println!("   • Deadlock-freedom analysis");
    println!("   • Reachability analysis");
    println!("   • Liveness and safety properties");
    println!("   • Automated assertions");

    println!("\n🚀 To analyze with Alloy, run:");
    println!("   alloy exec structural_verification.als");
    
    println!("\n💡 Expected analysis results:");
    println!("   • WellFormed: Should find issues with orphaned states");
    println!("   • Deterministic: May find non-deterministic transitions");
    println!("   • Reachability: Will identify unreachable states");
    println!("   • Assertions: Will check structural properties");

    println!("\n🎯 Alloy Benefits:");
    println!("   • Exhaustive structural analysis");
    println!("   • Counterexample generation");
    println!("   • Constraint satisfaction solving");
    println!("   • Model finding and verification");

    Ok(())
}