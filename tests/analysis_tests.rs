use upml_rs::{parse_plantuml, analysis::{StateMachineValidator, MetricsAnalyzer, ReachabilityAnalyzer}};

#[test]
fn test_analysis_simple_state_machine() {
    let plantuml = r#"
        @startuml
        [*] --> State1
        State1 --> State2 : Event1
        State2 --> [*]
        @enduml
    "#;

    let state_machine = parse_plantuml(plantuml).expect("Failed to parse PlantUML");

    // Test validation
    let validator = StateMachineValidator::new();
    let validation_report = validator.validate(&state_machine).expect("Validation failed");
    
    assert_eq!(validation_report.summary.errors, 0);
    assert_eq!(validation_report.summary.total_states, 2);
    assert_eq!(validation_report.summary.total_transitions, 2);

    // Test metrics
    let metrics_analyzer = MetricsAnalyzer::new();
    let complexity = metrics_analyzer.analyze_complexity(&state_machine).expect("Metrics analysis failed");
    
    assert_eq!(complexity.state_complexity, 2);
    assert_eq!(complexity.transition_complexity, 2);
    assert_eq!(complexity.cyclomatic_complexity, 2);

    // Test reachability
    let reachability_analyzer = ReachabilityAnalyzer::new();
    let reachability = reachability_analyzer.analyze(&state_machine).expect("Reachability analysis failed");
    
    assert_eq!(reachability.unreachable_states.len(), 0);
    assert!(reachability.reachable_states.len() >= 2);
}

#[test]
fn test_analysis_complex_state_machine() {
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

    let state_machine = parse_plantuml(plantuml).expect("Failed to parse PlantUML");

    // Test validation
    let validator = StateMachineValidator::new();
    let validation_report = validator.validate(&state_machine).expect("Validation failed");
    
    assert_eq!(validation_report.summary.errors, 0);
    assert_eq!(validation_report.summary.total_states, 4);

    // Test state metrics
    let metrics_analyzer = MetricsAnalyzer::new();
    let state_metrics = metrics_analyzer.analyze_states(&state_machine).expect("State metrics failed");
    
    assert_eq!(state_metrics.len(), 4);
    
    // Find Processing state which should have higher fan-out
    let processing_state = state_metrics.iter().find(|s| s.id == "Processing").unwrap();
    assert_eq!(processing_state.outgoing_transitions, 2);

    // Test path finding
    let reachability_analyzer = ReachabilityAnalyzer::new();
    let path_analysis = reachability_analyzer.find_path(&state_machine, "Idle", "Completed").expect("Path analysis failed");
    
    assert!(path_analysis.is_reachable);
    assert!(path_analysis.shortest_path.is_some());
    assert!(path_analysis.path_length >= 3); // Idle -> Processing -> Completed
}

#[test]
fn test_analysis_unreachable_state() {
    let plantuml = r#"
        @startuml
        [*] --> State1
        State1 --> State2 : Event1
        State2 --> [*]
        
        State3 --> State4 : Event2
        @enduml
    "#;

    let state_machine = parse_plantuml(plantuml).expect("Failed to parse PlantUML");

    // Test validation should find unreachable states
    let validator = StateMachineValidator::new();
    let validation_report = validator.validate(&state_machine).expect("Validation failed");
    
    // Should have warnings about unreachable states
    assert!(validation_report.summary.warnings > 0);
    
    // Check that State3 and State4 are identified as unreachable
    let unreachable_issues: Vec<_> = validation_report.issues.iter()
        .filter(|issue| matches!(issue.issue_type, upml_rs::analysis::IssueType::UnreachableState))
        .collect();
    
    assert!(unreachable_issues.len() >= 2);

    // Test reachability analysis
    let reachability_analyzer = ReachabilityAnalyzer::new();
    let reachability = reachability_analyzer.analyze(&state_machine).expect("Reachability analysis failed");
    
    assert!(reachability.unreachable_states.len() >= 2);
    assert!(reachability.unreachable_states.contains("State3"));
    assert!(reachability.unreachable_states.contains("State4"));
}

#[test]
fn test_analysis_missing_initial_state() {
    let plantuml = r#"
        @startuml
        State1 --> State2 : Event1
        State2 --> State1 : Event2
        @enduml
    "#;

    let state_machine = parse_plantuml(plantuml).expect("Failed to parse PlantUML");

    // Test validation should find missing initial state
    let validator = StateMachineValidator::new();
    let validation_report = validator.validate(&state_machine).expect("Validation failed");
    
    // Should have error about missing initial state
    assert!(validation_report.summary.errors > 0);
    
    let missing_initial_issues: Vec<_> = validation_report.issues.iter()
        .filter(|issue| matches!(issue.issue_type, upml_rs::analysis::IssueType::MissingInitialState))
        .collect();
    
    assert_eq!(missing_initial_issues.len(), 1);
}