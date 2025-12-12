use upml_rs::generators;
use upml_rs::state_machine::{StateMachineBuilder, Transition, Activity, Location};
use std::io::Cursor;


#[test]
fn test_complex_state_machine_builder() {
    let sm = StateMachineBuilder::new("ComplexMachine".to_string())
        .add_region("main_region".to_string())
            .add_state("Idle".to_string())
                .initial()
                .add_transition(
                    Transition::new(
                        "t1".to_string(),
                        "Idle".to_string(),
                        "Processing".to_string(),
                        "Start".to_string(),
                    )
                    .with_guard(vec!["resource_available".to_string(), "!busy".to_string()])
                    .with_effect(vec!["allocate_resource()".to_string(), "set_busy_flag()".to_string()])
                )
                .add_activity(Activity::new(
                    "a1".to_string(),
                    "Idle".to_string(),
                    "entry".to_string(),
                    vec!["initialize_system()".to_string()],
                ))
                .finish_state()
            .add_state("Processing".to_string())
                .add_transition(
                    Transition::new(
                        "t2".to_string(),
                        "Processing".to_string(),
                        "Idle".to_string(),
                        "Complete".to_string(),
                    )
                    .with_effect(vec!["release_resource()".to_string(), "clear_busy_flag()".to_string()])
                )
                .add_transition(
                    Transition::new(
                        "t3".to_string(),
                        "Processing".to_string(),
                        "Error".to_string(),
                        "Failure".to_string(),
                    )
                    .with_guard(vec!["error_count < 3".to_string()])
                    .with_effect(vec!["increment_error_count()".to_string(), "log_error()".to_string()])
                )
                .finish_state()
            .add_state("Error".to_string())
                .add_transition(
                    Transition::new(
                        "t4".to_string(),
                        "Error".to_string(),
                        "Processing".to_string(),
                        "Retry".to_string(),
                    )
                    .with_guard(vec!["error_count < 3".to_string()])
                    .with_effect(vec!["reset_state()".to_string()])
                )
                .add_transition(
                    Transition::new(
                        "t5".to_string(),
                        "Error".to_string(),
                        "Failed".to_string(),
                        "Retry".to_string(),
                    )
                    .with_guard(vec!["error_count >= 3".to_string()])
                    .with_effect(vec!["cleanup()".to_string(), "notify_admin()".to_string()])
                )
                .finish_state()
            .add_state("Failed".to_string())
                .final_state()
                .finish_state()
            .finish_region()
        .build();

    // Проверяем структуру
    assert_eq!(sm.id, "ComplexMachine");
    assert_eq!(sm.regions.len(), 1);
    
    let states = sm.states(false);
    assert_eq!(states.len(), 4);
    assert!(states.contains("Idle"));
    assert!(states.contains("Processing"));
    assert!(states.contains("Error"));
    assert!(states.contains("Failed"));
    
    let events = sm.events();
    assert!(events.contains("Start"));
    assert!(events.contains("Complete"));
    assert!(events.contains("Failure"));
    assert!(events.contains("Retry"));
    
    let initial_states = sm.initial_states();
    assert!(initial_states.contains("Idle"));
    
    // Проверяем глубину иерархии (может быть больше 1 из-за внутренней структуры)
    assert!(sm.depth() >= 1);
}

#[test]
fn test_hierarchical_state_machine() {
    let sm = StateMachineBuilder::new("HierarchicalMachine".to_string())
        .add_region("main_region".to_string())
            .add_state("Operation".to_string())
                .initial()
                .add_subregion("operation_region".to_string())
                    .add_state("Launch".to_string())
                        .initial()
                        .add_transition(
                            Transition::new(
                                "t1".to_string(),
                                "Launch".to_string(),
                                "Monitor".to_string(),
                                "EvtLaunch".to_string(),
                            )
                        )
                        .finish_substate()
                    .add_state("Monitor".to_string())
                        .final_state()
                        .finish_substate()
                    .finish_subregion()
                .add_subregion("sub_operation_region".to_string())
                    .add_state("SubLaunch".to_string())
                        .initial()
                        .add_activity(Activity::new(
                            "a1".to_string(),
                            "SubLaunch".to_string(),
                            "entry".to_string(),
                            vec!["send event:INVITE to state:Bob".to_string()],
                        ))
                        .add_activity(Activity::new(
                            "a2".to_string(),
                            "SubLaunch".to_string(),
                            "exit".to_string(),
                            vec!["trace exit action".to_string()],
                        ))
                        .finish_substate()
                    .finish_subregion()
                .finish_state()
            .finish_region()
        .build();

    assert_eq!(sm.id, "HierarchicalMachine");
    assert!(sm.depth() > 1); // Должна быть иерархическая структура
    
    let all_states = sm.states(true);
    assert!(all_states.len() >= 3); // Operation, Launch, Monitor, SubLaunch
}

#[test]
fn test_promela_generation_comprehensive() {
    let sm = StateMachineBuilder::new("TestMachine".to_string())
        .add_region("main_region".to_string())
            .add_state("State1".to_string())
                .initial()
                .add_transition(
                    Transition::new(
                        "t1".to_string(),
                        "State1".to_string(),
                        "State2".to_string(),
                        "Event1".to_string(),
                    )
                    .with_guard(vec!["x > 0".to_string()])
                    .with_effect(vec!["action1()".to_string(), "action2()".to_string()])
                )
                .finish_state()
            .add_state("State2".to_string())
                .add_transition(
                    Transition::new(
                        "t2".to_string(),
                        "State2".to_string(),
                        "State3".to_string(),
                        "Event2".to_string(),
                    )
                    .with_effect(vec!["send event:ACK to state:Remote".to_string()])
                )
                .finish_state()
            .add_state("State3".to_string())
                .final_state()
                .finish_state()
            .finish_region()
        .build();

    let mut output = Cursor::new(Vec::new());
    let result = generators::promela::generate_fsm(&mut output, &sm);
    assert!(result.is_ok(), "Failed to generate Promela code: {:?}", result.err());
    
    let generated_code = String::from_utf8(output.into_inner()).unwrap();
    
    // Проверяем основные элементы Promela кода
    assert!(generated_code.contains("Generated Promela FSM model"));
    assert!(generated_code.contains("mtype"));
    assert!(generated_code.contains("state_State1"));
    assert!(generated_code.contains("state_State2"));
    assert!(generated_code.contains("state_State3"));
    assert!(generated_code.contains("Event1"));
    assert!(generated_code.contains("Event2"));
    assert!(generated_code.contains("proctype StateMachine"));
    assert!(generated_code.contains("currentState = state_State1"));
    assert!(generated_code.contains("eventQueue"));
    assert!(generated_code.contains("inTransition"));
    
    // Проверяем переходы
    assert!(generated_code.contains("currentState == state_State1 && event == Event1"));
    assert!(generated_code.contains("currentState == state_State2 && event == Event2"));
    
    // Проверяем guards
    assert!(generated_code.contains("x > 0"));
    
    // Проверяем effects
    assert!(generated_code.contains("action1()"));
    assert!(generated_code.contains("action2()"));
    assert!(generated_code.contains("send event:ACK to state:Remote"));
    
    // Проверяем LTL свойства
    assert!(generated_code.contains("ltl"));
    assert!(generated_code.contains("safety"));
}

#[test]
fn test_tla_generation_comprehensive() {
    let sm = StateMachineBuilder::new("TLATestMachine".to_string())
        .add_region("main_region".to_string())
            .add_state("Initial".to_string())
                .initial()
                .add_transition(
                    Transition::new(
                        "t1".to_string(),
                        "Initial".to_string(),
                        "Working".to_string(),
                        "Start".to_string(),
                    )
                )
                .finish_state()
            .add_state("Working".to_string())
                .add_transition(
                    Transition::new(
                        "t2".to_string(),
                        "Working".to_string(),
                        "Final".to_string(),
                        "Finish".to_string(),
                    )
                )
                .finish_state()
            .add_state("Final".to_string())
                .final_state()
                .finish_state()
            .finish_region()
        .build();

    let mut output = Cursor::new(Vec::new());
    let result = generators::tla::generate_fsm(&mut output, &sm);
    assert!(result.is_ok(), "Failed to generate TLA+ code: {:?}", result.err());
    
    let generated_code = String::from_utf8(output.into_inner()).unwrap();
    
    // Проверяем основные элементы TLA+ кода
    assert!(generated_code.contains("MODULE TLATestMachine"));
    assert!(generated_code.contains("EXTENDS Naturals, Sequences, TLC"));
    assert!(generated_code.contains("CONSTANTS"));
    assert!(generated_code.contains("VARIABLES"));
    assert!(generated_code.contains("Initial"));
    assert!(generated_code.contains("Working"));
    assert!(generated_code.contains("Final"));
    assert!(generated_code.contains("Start"));
    assert!(generated_code.contains("Finish"));
    
    // Проверяем структуру TLA+
    assert!(generated_code.contains("States =="));
    assert!(generated_code.contains("Events =="));
    assert!(generated_code.contains("TypeInvariant =="));
    assert!(generated_code.contains("Init =="));
    assert!(generated_code.contains("Next =="));
    assert!(generated_code.contains("Spec =="));
    
    // Проверяем переходы
    assert!(generated_code.contains("Transition_Initial_Working"));
    assert!(generated_code.contains("Transition_Working_Final"));
    
    // Проверяем свойства
    assert!(generated_code.contains("THEOREM Spec => []TypeInvariant"));
    assert!(generated_code.contains("Liveness"));
    assert!(generated_code.contains("Safety"));
}

#[test]
fn test_hsm_generation() {
    let sm = StateMachineBuilder::new("HSMTestMachine".to_string())
        .add_region("main_region".to_string())
            .add_state("SuperState".to_string())
                .initial()
                .add_subregion("super_region".to_string())
                    .add_state("SubState1".to_string())
                        .initial()
                        .add_transition(
                            Transition::new(
                                "t1".to_string(),
                                "SubState1".to_string(),
                                "SubState2".to_string(),
                                "InternalEvent".to_string(),
                            )
                        )
                        .finish_substate()
                    .add_state("SubState2".to_string())
                        .finish_substate()
                    .finish_subregion()
                .finish_state()
            .finish_region()
        .build();

    let mut output = Cursor::new(Vec::new());
    let result = generators::promela::generate_hsm(&mut output, &sm);
    assert!(result.is_ok(), "Failed to generate HSM Promela code: {:?}", result.err());
    
    let generated_code = String::from_utf8(output.into_inner()).unwrap();
    
    // Проверяем HSM специфичные элементы
    assert!(generated_code.contains("Generated Promela HSM model"));
    assert!(generated_code.contains("stateStack"));
    assert!(generated_code.contains("stackDepth"));
    assert!(generated_code.contains("HierarchicalStateMachine"));
}

#[test]
fn test_state_machine_properties() {
    let sm = StateMachineBuilder::new("PropertiesTest".to_string())
        .add_region("region1".to_string())
            .add_state("A".to_string())
                .initial()
                .add_transition(
                    Transition::new("t1".to_string(), "A".to_string(), "B".to_string(), "e1".to_string())
                )
                .finish_state()
            .add_state("B".to_string())
                .add_transition(
                    Transition::new("t2".to_string(), "B".to_string(), "C".to_string(), "e2".to_string())
                )
                .finish_state()
            .add_state("C".to_string())
                .final_state()
                .finish_state()
            .finish_region()
        .build();

    // Тестируем различные свойства
    assert_eq!(sm.states(false).len(), 3);
    assert_eq!(sm.states(true).len(), 3);
    assert_eq!(sm.regions(false).len(), 1);
    assert_eq!(sm.regions(true).len(), 1);
    assert_eq!(sm.events().len(), 5); // e1, e2, NullEvent, EnterState, ExitState
    assert_eq!(sm.initial_states().len(), 1);
    assert!(sm.initial_states().contains("A"));
    
    // Тестируем поиск состояний
    assert!(sm.find_state("A").is_some());
    assert!(sm.find_state("B").is_some());
    assert!(sm.find_state("C").is_some());
    assert!(sm.find_state("NonExistent").is_none());
    
    // Тестируем поиск регионов
    assert!(sm.owner_region("A").is_some());
    assert!(sm.owner_region("B").is_some());
    assert!(sm.owner_region("C").is_some());
    assert!(sm.owner_region("NonExistent").is_none());
}

#[test]
fn test_transition_properties() {
    let transition = Transition::new(
        "test_transition".to_string(),
        "StateA".to_string(),
        "StateB".to_string(),
        "TestEvent".to_string(),
    )
    .with_guard(vec!["condition1".to_string(), "condition2".to_string()])
    .with_effect(vec!["action1()".to_string(), "action2()".to_string()])
    .with_location(Location {
        line: 42,
        column: 10,
        file: "test.plantuml".to_string(),
    });

    assert_eq!(transition.id, "test_transition");
    assert_eq!(transition.from_state, "StateA");
    assert_eq!(transition.to_state, "StateB");
    assert_eq!(transition.event, "TestEvent");
    assert_eq!(transition.guard.len(), 2);
    assert_eq!(transition.effect.len(), 2);
    assert_eq!(transition.location.line, 42);
    assert_eq!(transition.location.column, 10);
    assert_eq!(transition.location.file, "test.plantuml");
}

#[test]
fn test_activity_properties() {
    let activity = Activity::new(
        "test_activity".to_string(),
        "TestState".to_string(),
        "entry".to_string(),
        vec!["initialize()".to_string(), "setup()".to_string()],
    )
    .with_location(Location {
        line: 15,
        column: 5,
        file: "test.plantuml".to_string(),
    });

    assert_eq!(activity.id, "test_activity");
    assert_eq!(activity.state, "TestState");
    assert_eq!(activity.activity_type, "entry");
    assert_eq!(activity.args.len(), 2);
    assert_eq!(activity.location.line, 15);
}

// Тест для проверки корректности генерации при различных конфигурациях
#[test]
fn test_different_configurations() {
    use upml_rs::state_machine::StateMachineConfig;
    
    let config = StateMachineConfig {
        all_random: true,
    };
    
    let sm = StateMachineBuilder::new("ConfigTest".to_string())
        .with_config(config)
        .add_region("main".to_string())
            .add_state("State1".to_string())
                .initial()
                .finish_state()
            .finish_region()
        .build();
    
    assert!(sm.config.all_random);
    
    // Проверяем, что генерация работает с различными конфигурациями
    let mut promela_output = Cursor::new(Vec::new());
    let mut tla_output = Cursor::new(Vec::new());
    
    assert!(generators::promela::generate_fsm(&mut promela_output, &sm).is_ok());
    assert!(generators::tla::generate_fsm(&mut tla_output, &sm).is_ok());
}

// Стресс-тест с большим количеством состояний
#[test]
fn test_large_state_machine() {
    let builder = StateMachineBuilder::new("LargeStateMachine".to_string());
    let mut region_builder = builder.add_region("main_region".to_string());
    
    // Создаем 50 состояний с переходами
    for i in 0..50 {
        let state_name = format!("State{}", i);
        let next_state = format!("State{}", (i + 1) % 50);
        let event_name = format!("Event{}", i);
        let transition_id = format!("t{}", i);
        
        let mut state_builder = region_builder.add_state(state_name.clone());
        
        if i == 0 {
            state_builder = state_builder.initial();
        }
        
        state_builder = state_builder.add_transition(
            Transition::new(transition_id, state_name, next_state, event_name)
        );
        
        region_builder = state_builder.finish_state();
    }
    
    let sm = region_builder.finish_region().build();
    
    assert_eq!(sm.states(false).len(), 50);
    assert_eq!(sm.events().len(), 53); // 50 events + NullEvent, EnterState, ExitState
    
    // Проверяем, что генерация работает с большими машинами состояний
    let mut output = Cursor::new(Vec::new());
    let result = generators::promela::generate_fsm(&mut output, &sm);
    assert!(result.is_ok());
    
    let generated_code = String::from_utf8(output.into_inner()).unwrap();
    assert!(generated_code.len() > 1000); // Должен быть достаточно большой код
}