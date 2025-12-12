use upml_rs::{parse_plantuml, generators::nusmv};
use std::fs::File;
use std::io::Write;

fn main() -> upml_rs::Result<()> {
    // Example state machine with potential issues
    let plantuml = r#"
        @startuml
        [*] --> Idle
        
        Idle --> Processing : Start
        Processing --> Completed : Success
        Processing --> Error : Failure
        
        Error --> Processing : Retry
        Error --> Idle : Reset
        
        Completed --> [*]
        
        // Add a potential issue: unreachable state
        OrphanState --> DeadEnd : SomeEvent
        @enduml
    "#;

    println!("🔄 Parsing PlantUML state machine...");
    let state_machine = parse_plantuml(plantuml)?;
    
    println!("📊 State machine info:");
    println!("  • States: {}", state_machine.states(false).len());
    println!("  • Events: {}", state_machine.events().len());
    println!("  • Initial states: {:?}", state_machine.initial_states());
    println!("  • Final states: {:?}", state_machine.final_states());
    
    println!("\n🔧 Generating NuSMV model...");
    let mut output = Vec::new();
    nusmv::generate_model(&mut output, &state_machine)?;
    
    // Save to file
    let filename = "example_verification.smv";
    let mut file = File::create(filename)?;
    file.write_all(&output)?;
    
    println!("✅ NuSMV model saved to: {}", filename);
    
    // Display the generated model
    let model = String::from_utf8(output)?;
    println!("\n📄 Generated NuSMV model:");
    println!("{}", model);
    
    println!("\n🚀 To verify with NuSMV, run:");
    println!("   NuSMV {}", filename);
    
    println!("\n💡 Expected verification results:");
    println!("   • Some reachability properties may fail for OrphanState and DeadEnd");
    println!("   • Safety properties should pass (no deadlocks)");
    println!("   • Liveness properties should pass for the main flow");

    Ok(())
}