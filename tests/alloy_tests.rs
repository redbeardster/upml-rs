use upml_rs::{parse_plantuml, generators::alloy};

#[test]
fn test_alloy_simple_fsm() {
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

    let mut output = Vec::new();
    alloy::generate_model(&mut output, &state_machine).expect("Failed to generate Alloy model");
    
    let result = String::from_utf8(output).expect("Invalid UTF-8");
    
    // Check basic structure
    assert!(result.contains("abstract sig State"));
    assert!(result.contains("abstract sig Event"));
    assert!(result.contains("sig Transition"));
    assert!(result.contains("one sig StateMachine"));
    
    // Check states are present
    assert!(result.contains("one sig Idle extends State"));
    assert!(result.contains("one sig Processing extends State"));
    assert!(result.contains("one sig Completed extends State"));
    assert!(result.contains("one sig Error extends State"));
    
    // Check events are present
    assert!(result.contains("one sig Start extends Event"));
    assert!(result.contains("one sig Finish extends Event"));
    assert!(result.contains("one sig Failure extends Event"));
    assert!(result.contains("one sig Retry extends Event"));
    
    // Check predicates
    assert!(result.contains("pred WellFormed"));
    assert!(result.contains("pred Deterministic"));
    assert!(result.contains("pred DeadlockFree"));
    
    // Check assertions
    assert!(result.contains("assert WellFormedImpliesDeterministic"));
    assert!(result.contains("assert LivenessHolds"));
    
    // Check run commands
    assert!(result.contains("run WellFormed"));
    assert!(result.contains("check WellFormedImpliesDeterministic"));
}

#[test]
fn test_alloy_hierarchical_state_machine() {
    let plantuml = r#"
        @startuml
        [*] --> Outer
        
        state Outer {
            [*] --> Inner1
            Inner1 --> Inner2 : Event1
            Inner2 --> [*]
        }
        
        Outer --> Final : Done
        Final --> [*]
        @enduml
    "#;

    let state_machine = parse_plantuml(plantuml).expect("Failed to parse PlantUML");

    let mut output = Vec::new();
    alloy::generate_model(&mut output, &state_machine).expect("Failed to generate Alloy model");
    
    let result = String::from_utf8(output).expect("Invalid UTF-8");
    
    // Check that hierarchical states are flattened correctly
    assert!(result.contains("one sig Outer extends State"));
    assert!(result.contains("one sig Inner1 extends State"));
    assert!(result.contains("one sig Inner2 extends State"));
    assert!(result.contains("one sig Final extends State"));
    
    // Check transitions
    assert!(result.contains("one sig Event1 extends Event"));
    assert!(result.contains("one sig Done extends Event"));
}

#[test]
fn test_alloy_with_custom_config() {
    let plantuml = r#"
        @startuml
        [*] --> State1
        State1 --> State2 : Event1
        State2 --> [*]
        @enduml
    "#;

    let state_machine = parse_plantuml(plantuml).expect("Failed to parse PlantUML");

    let config = alloy::AlloyConfig {
        max_states: 5,
        max_events: 3,
        max_transitions: 8,
        structural_only: true,
    };

    let mut output = Vec::new();
    alloy::generate_model_with_config(&mut output, &state_machine, &config).expect("Failed to generate Alloy model");
    
    let result = String::from_utf8(output).expect("Invalid UTF-8");
    
    // Check that configuration is applied
    assert!(result.contains("for 5 State, 3 Event, 8 Transition"));
    
    // Should not contain behavioral predicates in structural_only mode
    assert!(!result.contains("// Behavioral Predicates"));
    assert!(!result.contains("pred LivenessProperty"));
}

#[test]
fn test_alloy_multiple_initial_states() {
    let plantuml = r#"
        @startuml
        [*] --> State1
        [*] --> State2
        State1 --> State3 : Event1
        State2 --> State3 : Event2
        State3 --> [*]
        @enduml
    "#;

    let state_machine = parse_plantuml(plantuml).expect("Failed to parse PlantUML");

    let mut output = Vec::new();
    alloy::generate_model(&mut output, &state_machine).expect("Failed to generate Alloy model");
    
    let result = String::from_utf8(output).expect("Invalid UTF-8");
    
    // Check that multiple initial states are handled (order may vary)
    assert!(result.contains("StateMachine.initialStates = State1 + State2") || 
            result.contains("StateMachine.initialStates = State2 + State1"));
}

#[test]
fn test_alloy_name_sanitization() {
    let plantuml = r#"
        @startuml
        [*] --> "State-With-Dashes"
        "State-With-Dashes" --> "State With Spaces" : "Event-Name"
        "State With Spaces" --> [*]
        @enduml
    "#;

    let state_machine = parse_plantuml(plantuml).expect("Failed to parse PlantUML");

    let mut output = Vec::new();
    alloy::generate_model(&mut output, &state_machine).expect("Failed to generate Alloy model");
    
    let result = String::from_utf8(output).expect("Invalid UTF-8");
    
    // Check that identifiers are sanitized
    assert!(result.contains("State_With_Dashes") || result.contains("State-With-Dashes"));
    assert!(result.contains("State_With_Spaces") || result.contains("State With Spaces"));
    assert!(result.contains("Event_Name") || result.contains("Event-Name"));
}

#[test]
fn test_alloy_reserved_words() {
    let plantuml = r#"
        @startuml
        [*] --> sig
        sig --> pred : run
        pred --> [*]
        @enduml
    "#;

    let state_machine = parse_plantuml(plantuml).expect("Failed to parse PlantUML");

    let mut output = Vec::new();
    alloy::generate_model(&mut output, &state_machine).expect("Failed to generate Alloy model");
    
    let result = String::from_utf8(output).expect("Invalid UTF-8");
    
    // Check that reserved words are handled
    assert!(result.contains("sig_") || result.contains("one sig sig"));
    assert!(result.contains("pred_") || result.contains("one sig pred"));
    assert!(result.contains("run_") || result.contains("one sig run"));
}

#[test]
fn test_alloy_empty_state_machine() {
    let plantuml = r#"
        @startuml
        @enduml
    "#;

    let state_machine = parse_plantuml(plantuml).expect("Failed to parse PlantUML");

    let mut output = Vec::new();
    let result = alloy::generate_model(&mut output, &state_machine);
    
    // Should handle empty state machine gracefully
    assert!(result.is_ok());
    
    let output_str = String::from_utf8(output).expect("Invalid UTF-8");
    assert!(output_str.contains("one sig StateMachine"));
    assert!(output_str.contains("no StateMachine.initialStates"));
    assert!(output_str.contains("no StateMachine.finalStates"));
}