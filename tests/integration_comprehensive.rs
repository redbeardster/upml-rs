use upml_rs::generators;
use upml_rs::state_machine::{StateMachineBuilder, Transition, Activity};
use std::io::Cursor;


// Тест полного цикла: создание -> генерация -> проверка
#[test]
fn test_full_cycle_promela_fsm() {
    let sm = create_comprehensive_test_machine();
    
    let mut output = Cursor::new(Vec::new());
    let result = generators::promela::generate_fsm(&mut output, &sm);
    assert!(result.is_ok(), "Failed to generate Promela FSM: {:?}", result.err());
    
    let generated_code = String::from_utf8(output.into_inner()).unwrap();
    
    // Проверяем структуру сгенерированного Promela кода
    validate_promela_structure(&generated_code);
    
    // Проверяем специфичные элементы нашей машины состояний
    assert!(generated_code.contains("state_Idle"));
    assert!(generated_code.contains("state_Processing"));
    assert!(generated_code.contains("state_Error"));
    assert!(generated_code.contains("state_Completed"));
    
    // Проверяем события
    assert!(generated_code.contains("Start"));
    assert!(generated_code.contains("Process"));
    assert!(generated_code.contains("Complete"));
    assert!(generated_code.contains("Fail"));
    assert!(generated_code.contains("Recover"));
    
    // Проверяем переходы
    assert!(generated_code.contains("currentState == state_Idle && event == Start"));
    assert!(generated_code.contains("currentState == state_Processing && event == Complete"));
    assert!(generated_code.contains("currentState == state_Processing && event == Fail"));
    assert!(generated_code.contains("currentState == state_Error && event == Recover"));
    
    // Проверяем guards
    assert!(generated_code.contains("resource_available"));
    assert!(generated_code.contains("error_count < 3"));
    
    // Проверяем effects
    assert!(generated_code.contains("allocate_resource()"));
    assert!(generated_code.contains("increment_error_count()"));
    assert!(generated_code.contains("send event:NOTIFY"));
}

#[test]
fn test_full_cycle_promela_hsm() {
    let sm = create_hierarchical_test_machine();
    
    let mut output = Cursor::new(Vec::new());
    let result = generators::promela::generate_hsm(&mut output, &sm);
    assert!(result.is_ok(), "Failed to generate Promela HSM: {:?}", result.err());
    
    let generated_code = String::from_utf8(output.into_inner()).unwrap();
    
    // Проверяем HSM специфичные элементы
    assert!(generated_code.contains("Generated Promela HSM model"));
    assert!(generated_code.contains("stateStack"));
    assert!(generated_code.contains("stackDepth"));
    assert!(generated_code.contains("HierarchicalStateMachine"));
    
    // Проверяем иерархические состояния
    assert!(generated_code.contains("state_SuperState"));
    assert!(generated_code.contains("state_SubState1"));
    assert!(generated_code.contains("state_SubState2"));
}

#[test]
fn test_full_cycle_tla_fsm() {
    let sm = create_comprehensive_test_machine();
    
    let mut output = Cursor::new(Vec::new());
    let result = generators::tla::generate_fsm(&mut output, &sm);
    assert!(result.is_ok(), "Failed to generate TLA+ FSM: {:?}", result.err());
    
    let generated_code = String::from_utf8(output.into_inner()).unwrap();
    
    // Проверяем структуру TLA+ кода
    validate_tla_structure(&generated_code);
    
    // Проверяем специфичные элементы
    assert!(generated_code.contains("Idle"));
    assert!(generated_code.contains("Processing"));
    assert!(generated_code.contains("Error"));
    assert!(generated_code.contains("Completed"));
    
    // Проверяем переходы
    assert!(generated_code.contains("Transition_Idle_Processing"));
    assert!(generated_code.contains("Transition_Processing_Completed"));
    assert!(generated_code.contains("Transition_Processing_Error"));
    assert!(generated_code.contains("Transition_Error_Processing"));
    
    // Проверяем свойства
    assert!(generated_code.contains("THEOREM Spec => []TypeInvariant"));
    assert!(generated_code.contains("Liveness"));
    assert!(generated_code.contains("Safety"));
}

#[test]
fn test_error_handling_and_recovery() {
    // Тестируем обработку ошибок в различных компонентах
    
    // Пустая машина состояний
    let empty_sm = StateMachineBuilder::new("Empty".to_string()).build();
    
    let mut output = Cursor::new(Vec::new());
    assert!(generators::promela::generate_fsm(&mut output, &empty_sm).is_ok());
    
    let generated = String::from_utf8(output.into_inner()).unwrap();
    assert!(generated.contains("Generated Promela FSM model"));
    
    // Машина состояний только с одним состоянием
    let single_state_sm = StateMachineBuilder::new("SingleState".to_string())
        .add_region("main".to_string())
            .add_state("OnlyState".to_string())
                .initial()
                .final_state()
                .finish_state()
            .finish_region()
        .build();
    
    let mut output = Cursor::new(Vec::new());
    assert!(generators::promela::generate_fsm(&mut output, &single_state_sm).is_ok());
    assert!(generators::tla::generate_fsm(&mut output, &single_state_sm).is_ok());
}

#[test]
fn test_large_scale_generation() {
    // Тестируем генерацию для больших машин состояний
    let large_sm = create_large_test_machine(200);
    
    let mut promela_output = Cursor::new(Vec::new());
    let mut tla_output = Cursor::new(Vec::new());
    
    let promela_result = generators::promela::generate_fsm(&mut promela_output, &large_sm);
    let tla_result = generators::tla::generate_fsm(&mut tla_output, &large_sm);
    
    assert!(promela_result.is_ok(), "Failed to generate large Promela FSM");
    assert!(tla_result.is_ok(), "Failed to generate large TLA+ FSM");
    
    let promela_code = String::from_utf8(promela_output.into_inner()).unwrap();
    let tla_code = String::from_utf8(tla_output.into_inner()).unwrap();
    
    // Проверяем, что код достаточно большой
    assert!(promela_code.len() > 10000, "Generated Promela code too small for large machine");
    assert!(tla_code.len() > 10000, "Generated TLA+ code too small for large machine");
    
    // Проверяем, что все состояния присутствуют
    for i in 0..200 {
        assert!(promela_code.contains(&format!("state_State{}", i)));
        assert!(tla_code.contains(&format!("State{}", i)));
    }
}

#[test]
fn test_concurrent_regions_simulation() {
    // Создаем машину состояний с параллельными регионами
    let concurrent_sm = StateMachineBuilder::new("ConcurrentMachine".to_string())
        .add_region("region_a".to_string())
            .add_state("A1".to_string())
                .initial()
                .add_transition(
                    Transition::new("ta1".to_string(), "A1".to_string(), "A2".to_string(), "EventA".to_string())
                )
                .finish_state()
            .add_state("A2".to_string())
                .add_transition(
                    Transition::new("ta2".to_string(), "A2".to_string(), "A1".to_string(), "ResetA".to_string())
                )
                .finish_state()
            .finish_region()
        .add_region("region_b".to_string())
            .add_state("B1".to_string())
                .initial()
                .add_transition(
                    Transition::new("tb1".to_string(), "B1".to_string(), "B2".to_string(), "EventB".to_string())
                )
                .finish_state()
            .add_state("B2".to_string())
                .add_transition(
                    Transition::new("tb2".to_string(), "B2".to_string(), "B1".to_string(), "ResetB".to_string())
                )
                .finish_state()
            .finish_region()
        .build();
    
    // Проверяем структуру
    assert_eq!(concurrent_sm.regions(false).len(), 2);
    assert_eq!(concurrent_sm.states(false).len(), 4);
    
    // Генерируем код
    let mut output = Cursor::new(Vec::new());
    assert!(generators::promela::generate_fsm(&mut output, &concurrent_sm).is_ok());
    
    let generated = String::from_utf8(output.into_inner()).unwrap();
    assert!(generated.contains("state_A1"));
    assert!(generated.contains("state_A2"));
    assert!(generated.contains("state_B1"));
    assert!(generated.contains("state_B2"));
}

#[test]
fn test_complex_guards_and_effects() {
    let complex_sm = StateMachineBuilder::new("ComplexMachine".to_string())
        .add_region("main".to_string())
            .add_state("Start".to_string())
                .initial()
                .add_transition(
                    Transition::new(
                        "complex_transition".to_string(),
                        "Start".to_string(),
                        "End".to_string(),
                        "ComplexEvent".to_string(),
                    )
                    .with_guard(vec![
                        "(x > 0 && y < 100)".to_string(),
                        "resource_available".to_string(),
                        "!system_busy".to_string(),
                        "user_authenticated".to_string(),
                    ])
                    .with_effect(vec![
                        "allocate_resource(x, y)".to_string(),
                        "log_transition(\"Start\", \"End\")".to_string(),
                        "send event:NOTIFY to state:Monitor".to_string(),
                        "increment_counter()".to_string(),
                        "set_flag(true)".to_string(),
                        "trace complex transition executed".to_string(),
                    ])
                )
                .finish_state()
            .add_state("End".to_string())
                .final_state()
                .finish_state()
            .finish_region()
        .build();
    
    let mut promela_output = Cursor::new(Vec::new());
    let mut tla_output = Cursor::new(Vec::new());
    
    assert!(generators::promela::generate_fsm(&mut promela_output, &complex_sm).is_ok());
    assert!(generators::tla::generate_fsm(&mut tla_output, &complex_sm).is_ok());
    
    let promela_code = String::from_utf8(promela_output.into_inner()).unwrap();
    let tla_code = String::from_utf8(tla_output.into_inner()).unwrap();
    
    // Проверяем guards в Promela
    assert!(promela_code.contains("(x > 0 && y < 100)"));
    assert!(promela_code.contains("resource_available"));
    assert!(promela_code.contains("!system_busy"));
    assert!(promela_code.contains("user_authenticated"));
    
    // Проверяем effects в Promela
    assert!(promela_code.contains("allocate_resource(x, y)"));
    assert!(promela_code.contains("send event:NOTIFY to state:Monitor"));
    assert!(promela_code.contains("trace complex transition executed"));
    
    // Проверяем guards в TLA+
    assert!(tla_code.contains("(x > 0 && y < 100)"));
    assert!(tla_code.contains("resource_available"));
}

#[test]
fn test_activities_comprehensive() {
    let activity_sm = StateMachineBuilder::new("ActivityMachine".to_string())
        .add_region("main".to_string())
            .add_state("ActiveState".to_string())
                .initial()
                .add_activity(Activity::new(
                    "entry_activity".to_string(),
                    "ActiveState".to_string(),
                    "entry".to_string(),
                    vec![
                        "initialize_resources()".to_string(),
                        "start_timer(30)".to_string(),
                        "send event:READY to state:Monitor".to_string(),
                    ],
                ))
                .add_activity(Activity::new(
                    "exit_activity".to_string(),
                    "ActiveState".to_string(),
                    "exit".to_string(),
                    vec![
                        "cleanup_resources()".to_string(),
                        "stop_timer()".to_string(),
                        "log_state_exit()".to_string(),
                    ],
                ))
                .add_activity(Activity::new(
                    "timeout_activity".to_string(),
                    "ActiveState".to_string(),
                    "timeout".to_string(),
                    vec!["handle_timeout()".to_string()],
                ))
                .finish_state()
            .finish_region()
        .build();
    
    // Проверяем, что активности сохранены
    let region = activity_sm.regions.values().next().unwrap();
    let region_borrow = region.borrow();
    let state = region_borrow.substates.get("ActiveState").unwrap();
    let activities = &state.borrow().activities;
    
    assert_eq!(activities.len(), 3);
    assert!(activities.iter().any(|a| a.activity_type == "entry"));
    assert!(activities.iter().any(|a| a.activity_type == "exit"));
    assert!(activities.iter().any(|a| a.activity_type == "timeout"));
    
    // Проверяем генерацию
    let mut output = Cursor::new(Vec::new());
    assert!(generators::promela::generate_fsm(&mut output, &activity_sm).is_ok());
}

// Вспомогательные функции для создания тестовых машин состояний

fn create_comprehensive_test_machine() -> upml_rs::state_machine::StateMachine {
    StateMachineBuilder::new("ComprehensiveTestMachine".to_string())
        .add_region("main_region".to_string())
            .add_state("Idle".to_string())
                .initial()
                .add_transition(
                    Transition::new(
                        "start_processing".to_string(),
                        "Idle".to_string(),
                        "Processing".to_string(),
                        "Start".to_string(),
                    )
                    .with_guard(vec!["resource_available".to_string()])
                    .with_effect(vec!["allocate_resource()".to_string(), "set_busy_flag()".to_string()])
                )
                .add_activity(Activity::new(
                    "idle_entry".to_string(),
                    "Idle".to_string(),
                    "entry".to_string(),
                    vec!["initialize_system()".to_string()],
                ))
                .finish_state()
            .add_state("Processing".to_string())
                .add_transition(
                    Transition::new(
                        "complete_processing".to_string(),
                        "Processing".to_string(),
                        "Completed".to_string(),
                        "Complete".to_string(),
                    )
                    .with_effect(vec!["release_resource()".to_string(), "send event:NOTIFY to state:Monitor".to_string()])
                )
                .add_transition(
                    Transition::new(
                        "fail_processing".to_string(),
                        "Processing".to_string(),
                        "Error".to_string(),
                        "Fail".to_string(),
                    )
                    .with_guard(vec!["error_count < 3".to_string()])
                    .with_effect(vec!["increment_error_count()".to_string(), "log_error()".to_string()])
                )
                .finish_state()
            .add_state("Error".to_string())
                .add_transition(
                    Transition::new(
                        "recover_from_error".to_string(),
                        "Error".to_string(),
                        "Processing".to_string(),
                        "Recover".to_string(),
                    )
                    .with_effect(vec!["reset_error_state()".to_string()])
                )
                .finish_state()
            .add_state("Completed".to_string())
                .final_state()
                .finish_state()
            .finish_region()
        .build()
}

fn create_hierarchical_test_machine() -> upml_rs::state_machine::StateMachine {
    StateMachineBuilder::new("HierarchicalTestMachine".to_string())
        .add_region("main_region".to_string())
            .add_state("SuperState".to_string())
                .initial()
                .add_subregion("sub_region".to_string())
                    .add_state("SubState1".to_string())
                        .initial()
                        .add_transition(
                            Transition::new(
                                "internal_transition".to_string(),
                                "SubState1".to_string(),
                                "SubState2".to_string(),
                                "InternalEvent".to_string(),
                            )
                        )
                        .finish_substate()
                    .add_state("SubState2".to_string())
                        .final_state()
                        .finish_substate()
                    .finish_subregion()
                .finish_state()
            .finish_region()
        .build()
}

fn create_large_test_machine(num_states: usize) -> upml_rs::state_machine::StateMachine {
    let builder = StateMachineBuilder::new("LargeTestMachine".to_string());
    let mut region_builder = builder.add_region("main_region".to_string());
    
    for i in 0..num_states {
        let state_name = format!("State{}", i);
        let next_state = format!("State{}", (i + 1) % num_states);
        let event_name = format!("Event{}", i);
        let transition_id = format!("transition_{}", i);
        
        let mut state_builder = region_builder.add_state(state_name.clone());
        
        if i == 0 {
            state_builder = state_builder.initial();
        }
        if i == num_states - 1 {
            state_builder = state_builder.final_state();
        }
        
        state_builder = state_builder
            .add_transition(
                Transition::new(transition_id, state_name.clone(), next_state, event_name)
                    .with_guard(vec![format!("guard_condition_{}", i)])
                    .with_effect(vec![
                        format!("action_{}()", i),
                        format!("log(\"Executing transition {}\")", i),
                    ])
            )
            .add_activity(Activity::new(
                format!("entry_{}", i),
                state_name,
                "entry".to_string(),
                vec![format!("enter_state_{}()", i)],
            ));
        
        region_builder = state_builder.finish_state();
    }
    
    region_builder.finish_region().build()
}

// Функции валидации сгенерированного кода

fn validate_promela_structure(code: &str) {
    // Проверяем основные элементы Promela кода
    assert!(code.contains("/*"), "Missing comment header");
    assert!(code.contains("Generated Promela FSM model"), "Missing generation header");
    assert!(code.contains("mtype"), "Missing mtype declarations");
    assert!(code.contains("proctype"), "Missing process type");
    assert!(code.contains("currentState"), "Missing currentState variable");
    assert!(code.contains("eventQueue"), "Missing eventQueue");
    assert!(code.contains("inTransition"), "Missing inTransition flag");
    assert!(code.contains("do"), "Missing main loop");
    assert!(code.contains("od"), "Missing loop end");
    assert!(code.contains("ltl"), "Missing LTL properties");
    
    // Проверяем синтаксическую корректность
    assert!(!code.contains(";;"), "Double semicolons found");
    assert!(code.matches('{').count() == code.matches('}').count(), "Unbalanced braces");
    assert!(code.matches('(').count() == code.matches(')').count(), "Unbalanced parentheses");
}

fn validate_tla_structure(code: &str) {
    // Проверяем основные элементы TLA+ кода
    assert!(code.contains("(*"), "Missing comment header");
    assert!(code.contains("Generated TLA+/PlusCal FSM model"), "Missing generation header");
    assert!(code.contains("---- MODULE"), "Missing module declaration");
    assert!(code.contains("EXTENDS"), "Missing extends clause");
    assert!(code.contains("CONSTANTS"), "Missing constants section");
    assert!(code.contains("VARIABLES"), "Missing variables section");
    assert!(code.contains("TypeInvariant"), "Missing type invariant");
    assert!(code.contains("Init =="), "Missing init predicate");
    assert!(code.contains("Next =="), "Missing next relation");
    assert!(code.contains("Spec =="), "Missing specification");
    assert!(code.contains("THEOREM"), "Missing theorems");
    assert!(code.contains("===="), "Missing module end");
    
    // Проверяем TLA+ специфичные элементы
    assert!(code.contains("/\\"), "Missing conjunction operators");
    assert!(code.contains("\\/"), "Missing disjunction operators");
    assert!(code.contains("[]"), "Missing always operators");
    assert!(code.contains("<>"), "Missing eventually operators");
}