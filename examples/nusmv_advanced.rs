use upml_rs::{parse_plantuml, generators::nusmv};
use std::fs::File;

fn main() -> upml_rs::Result<()> {
    let plantuml = r#"
        @startuml
        [*] --> Idle
        Idle --> Processing : Start
        Processing --> Completed : Success
        Processing --> Error : Failure
        Error --> Processing : Retry
        Error --> Idle : Reset
        Completed --> [*]
        @enduml
    "#;

    println!("🔧 Generating advanced NuSMV model with fairness constraints...");
    let state_machine = parse_plantuml(plantuml)?;
    
    // Create configuration with fairness and custom properties
    let config = nusmv::NuSMVConfig {
        enable_fairness: true,
        custom_properties: vec![
            "AG (state = Error -> EF (state != Error))".to_string(),
            "AG EF (state = Idle)".to_string(),
            "EF (state = Completed)".to_string(),
        ],
        basic_properties_only: false,
    };
    
    // Generate model with advanced features
    let mut file = File::create("advanced_model.smv")?;
    nusmv::generate_model_with_config(&mut file, &state_machine, &config)?;
    
    println!("✅ Advanced NuSMV model saved to: advanced_model.smv");
    println!("\n📋 Features included:");
    println!("   • Fairness constraints for events and progress");
    println!("   • Custom temporal logic properties");
    println!("   • Advanced determinism checks");
    println!("   • Extended reachability analysis");
    
    println!("\n🚀 To verify with fairness, run:");
    println!("   NuSMV advanced_model.smv");

    Ok(())
}