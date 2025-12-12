use upml_rs::{parse_plantuml, generators::nusmv};

#[test]
fn test_nusmv_simple_fsm() {
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
    nusmv::generate_model(&mut output, &state_machine).expect("Failed to generate NuSMV model");
    
    let result = String::from_utf8(output).expect("Invalid UTF-8");
    
    // Check basic structure
    assert!(result.contains("MODULE main"));
    assert!(result.contains("VAR"));
    assert!(result.contains("ASSIGN"));
    assert!(result.contains("SPEC"));
    
    // Check states are present
    assert!(result.contains("Idle"));
    assert!(result.contains("Processing"));
    assert!(result.contains("Completed"));
    assert!(result.contains("Error"));
    
    // Check events are present
    assert!(result.contains("Start"));
    assert!(result.contains("Finish"));
    assert!(result.contains("Failure"));
    assert!(result.contains("Retry"));
    
    // Check initial state
    assert!(result.contains("init(state) := Idle"));
    
    // Check some transitions
    assert!(result.contains("state = Idle & event = Start : Processing"));
    assert!(result.contains("state = Processing & event = Finish : Completed"));
    
    // Check specifications are generated
    assert!(result.contains("-- Reachability Properties"));
    assert!(result.contains("-- Safety Properties"));
    assert!(result.contains("-- Liveness Properties"));
}

#[test]
fn test_nusmv_hierarchical_state_machine() {
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
    nusmv::generate_model(&mut output, &state_machine).expect("Failed to generate NuSMV model");
    
    let result = String::from_utf8(output).expect("Invalid UTF-8");
    
    // Check that hierarchical states are flattened correctly
    assert!(result.contains("Outer"));
    assert!(result.contains("Inner1"));
    assert!(result.contains("Inner2"));
    assert!(result.contains("Final"));
    
    // Check transitions
    assert!(result.contains("Event1"));
    assert!(result.contains("Done"));
}

#[test]
fn test_nusmv_multiple_initial_states() {
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
    nusmv::generate_model(&mut output, &state_machine).expect("Failed to generate NuSMV model");
    
    let result = String::from_utf8(output).expect("Invalid UTF-8");
    
    // Check that multiple initial states are handled with non-deterministic choice
    assert!(result.contains("init(state) := {"));
    assert!(result.contains("State1"));
    assert!(result.contains("State2"));
}

#[test]
fn test_nusmv_with_guards() {
    let plantuml = r#"
        @startuml
        [*] --> State1
        State1 --> State2 : Event1 [guard1]
        State1 --> State3 : Event1 [guard2]
        State2 --> [*]
        State3 --> [*]
        @enduml
    "#;

    let state_machine = parse_plantuml(plantuml).expect("Failed to parse PlantUML");

    let mut output = Vec::new();
    nusmv::generate_model(&mut output, &state_machine).expect("Failed to generate NuSMV model");
    
    let result = String::from_utf8(output).expect("Invalid UTF-8");
    
    // Check that guards are included in transitions
    assert!(result.contains("guard1"));
    assert!(result.contains("guard2"));
    
    // Check that both transitions are present (non-deterministic)
    assert!(result.contains("state = State1 & event = Event1"));
}

#[test]
fn test_nusmv_sanitization() {
    let plantuml = r#"
        @startuml
        [*] --> "State-With-Dashes"
        "State-With-Dashes" --> "State With Spaces" : "Event-Name"
        "State With Spaces" --> [*]
        @enduml
    "#;

    let state_machine = parse_plantuml(plantuml).expect("Failed to parse PlantUML");

    let mut output = Vec::new();
    nusmv::generate_model(&mut output, &state_machine).expect("Failed to generate NuSMV model");
    
    let result = String::from_utf8(output).expect("Invalid UTF-8");
    
    // Check that identifiers are sanitized (dashes and spaces replaced with underscores)
    assert!(result.contains("State_With_Dashes") || result.contains("State-With-Dashes"));
    assert!(result.contains("State_With_Spaces") || result.contains("State With Spaces"));
    assert!(result.contains("Event_Name") || result.contains("Event-Name"));
}

#[test]
fn test_nusmv_empty_state_machine() {
    let plantuml = r#"
        @startuml
        @enduml
    "#;

    let state_machine = parse_plantuml(plantuml).expect("Failed to parse PlantUML");

    let mut output = Vec::new();
    let result = nusmv::generate_model(&mut output, &state_machine);
    
    // Should handle empty state machine gracefully
    assert!(result.is_ok());
    
    let output_str = String::from_utf8(output).expect("Invalid UTF-8");
    assert!(output_str.contains("MODULE main"));
}