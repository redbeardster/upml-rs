use upml_rs::{parse_plantuml, analysis::ReachabilityAnalyzer};

fn main() -> upml_rs::Result<()> {
    let plantuml = r#"
        @startuml
        [*] --> Idle
        Idle --> Processing : Start
        Processing --> Completed : Finish
        Processing --> Error : Failure
        Error --> Idle : Retry
        Completed --> [*]
        @enduml
    "#;

    let state_machine = parse_plantuml(plantuml)?;
    let analyzer = ReachabilityAnalyzer::new();
    
    // Test path from Idle to Completed
    let path_analysis = analyzer.find_path(&state_machine, "Idle", "Completed")?;
    
    println!("Path from Idle to Completed:");
    println!("  Reachable: {}", path_analysis.is_reachable);
    println!("  Path length: {}", path_analysis.path_length);
    
    if let Some(path) = &path_analysis.shortest_path {
        println!("  Shortest path: {:?}", path);
    }
    
    println!("  All paths ({} found):", path_analysis.all_paths.len());
    for (i, path) in path_analysis.all_paths.iter().enumerate() {
        println!("    Path {}: {:?}", i + 1, path);
    }
    
    // Test path from Error to Completed
    let path_analysis2 = analyzer.find_path(&state_machine, "Error", "Completed")?;
    
    println!("\nPath from Error to Completed:");
    println!("  Reachable: {}", path_analysis2.is_reachable);
    println!("  Path length: {}", path_analysis2.path_length);
    
    if let Some(path) = &path_analysis2.shortest_path {
        println!("  Shortest path: {:?}", path);
    }

    Ok(())
}