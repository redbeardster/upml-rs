use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use upml_rs::state_machine::{StateMachineBuilder, Transition};
use upml_rs::generators;
use std::io::Cursor;

// Добавляем criterion в Cargo.toml как dev-dependency
fn benchmark_state_machine_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("state_machine_creation");
    
    for size in [10, 50, 100, 500].iter() {
        group.bench_with_input(BenchmarkId::new("linear_chain", size), size, |b, &size| {
            b.iter(|| {
                let mut builder = StateMachineBuilder::new("BenchmarkMachine".to_string());
                let mut region_builder = builder.add_region("main_region".to_string());
                
                for i in 0..size {
                    let state_name = format!("State{}", i);
                    let next_state = if i < size - 1 {
                        format!("State{}", i + 1)
                    } else {
                        "State0".to_string() // Замыкаем цикл
                    };
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
                
                black_box(region_builder.finish_region().build())
            });
        });
    }
    
    group.finish();
}

fn benchmark_promela_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("promela_generation");
    
    // Создаем машины состояний разного размера для бенчмарка
    let small_sm = create_test_state_machine(10);
    let medium_sm = create_test_state_machine(50);
    let large_sm = create_test_state_machine(100);
    
    group.bench_function("small_fsm_10_states", |b| {
        b.iter(|| {
            let mut output = Cursor::new(Vec::new());
            generators::promela::generate_fsm(&mut output, black_box(&small_sm)).unwrap();
            black_box(output)
        });
    });
    
    group.bench_function("medium_fsm_50_states", |b| {
        b.iter(|| {
            let mut output = Cursor::new(Vec::new());
            generators::promela::generate_fsm(&mut output, black_box(&medium_sm)).unwrap();
            black_box(output)
        });
    });
    
    group.bench_function("large_fsm_100_states", |b| {
        b.iter(|| {
            let mut output = Cursor::new(Vec::new());
            generators::promela::generate_fsm(&mut output, black_box(&large_sm)).unwrap();
            black_box(output)
        });
    });
    
    group.bench_function("small_hsm_10_states", |b| {
        b.iter(|| {
            let mut output = Cursor::new(Vec::new());
            generators::promela::generate_hsm(&mut output, black_box(&small_sm)).unwrap();
            black_box(output)
        });
    });
    
    group.finish();
}

fn benchmark_tla_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("tla_generation");
    
    let small_sm = create_test_state_machine(10);
    let medium_sm = create_test_state_machine(50);
    let large_sm = create_test_state_machine(100);
    
    group.bench_function("small_tla_10_states", |b| {
        b.iter(|| {
            let mut output = Cursor::new(Vec::new());
            generators::tla::generate_fsm(&mut output, black_box(&small_sm)).unwrap();
            black_box(output)
        });
    });
    
    group.bench_function("medium_tla_50_states", |b| {
        b.iter(|| {
            let mut output = Cursor::new(Vec::new());
            generators::tla::generate_fsm(&mut output, black_box(&medium_sm)).unwrap();
            black_box(output)
        });
    });
    
    group.bench_function("large_tla_100_states", |b| {
        b.iter(|| {
            let mut output = Cursor::new(Vec::new());
            generators::tla::generate_fsm(&mut output, black_box(&large_sm)).unwrap();
            black_box(output)
        });
    });
    
    group.finish();
}

fn benchmark_state_machine_queries(c: &mut Criterion) {
    let mut group = c.benchmark_group("state_machine_queries");
    
    let sm = create_test_state_machine(100);
    
    group.bench_function("get_all_states", |b| {
        b.iter(|| {
            black_box(sm.states(false))
        });
    });
    
    group.bench_function("get_all_states_recursive", |b| {
        b.iter(|| {
            black_box(sm.states(true))
        });
    });
    
    group.bench_function("get_all_events", |b| {
        b.iter(|| {
            black_box(sm.events())
        });
    });
    
    group.bench_function("find_state", |b| {
        b.iter(|| {
            black_box(sm.find_state("State50"))
        });
    });
    
    group.bench_function("get_initial_states", |b| {
        b.iter(|| {
            black_box(sm.initial_states())
        });
    });
    
    group.bench_function("calculate_depth", |b| {
        b.iter(|| {
            black_box(sm.depth())
        });
    });
    
    group.finish();
}

fn benchmark_complex_state_machine(c: &mut Criterion) {
    let mut group = c.benchmark_group("complex_state_machine");
    
    // Создаем сложную иерархическую машину состояний
    group.bench_function("create_hierarchical_machine", |b| {
        b.iter(|| {
            black_box(create_hierarchical_state_machine())
        });
    });
    
    let hierarchical_sm = create_hierarchical_state_machine();
    
    group.bench_function("generate_hierarchical_promela", |b| {
        b.iter(|| {
            let mut output = Cursor::new(Vec::new());
            generators::promela::generate_hsm(&mut output, black_box(&hierarchical_sm)).unwrap();
            black_box(output)
        });
    });
    
    group.finish();
}

// Вспомогательные функции для создания тестовых машин состояний
fn create_test_state_machine(num_states: usize) -> upml_rs::state_machine::StateMachine {
    let mut builder = StateMachineBuilder::new("TestMachine".to_string());
    let mut region_builder = builder.add_region("main_region".to_string());
    
    for i in 0..num_states {
        let state_name = format!("State{}", i);
        let next_state = format!("State{}", (i + 1) % num_states);
        let event_name = format!("Event{}", i);
        let transition_id = format!("t{}", i);
        
        let mut state_builder = region_builder.add_state(state_name.clone());
        
        if i == 0 {
            state_builder = state_builder.initial();
        }
        if i == num_states - 1 {
            state_builder = state_builder.final_state();
        }
        
        // Добавляем переход с guard и effect для большей сложности
        state_builder = state_builder.add_transition(
            Transition::new(transition_id, state_name, next_state, event_name)
                .with_guard(vec![format!("condition{}", i)])
                .with_effect(vec![format!("action{}()", i), format!("log(\"State{}\")", i)])
        );
        
        region_builder = state_builder.finish_state();
    }
    
    region_builder.finish_region().build()
}

fn create_hierarchical_state_machine() -> upml_rs::state_machine::StateMachine {
    StateMachineBuilder::new("HierarchicalMachine".to_string())
        .add_region("main_region".to_string())
            .add_state("SuperState1".to_string())
                .initial()
                .add_subregion("super1_region".to_string())
                    .add_state("SubState1_1".to_string())
                        .initial()
                        .add_transition(
                            Transition::new(
                                "t1".to_string(),
                                "SubState1_1".to_string(),
                                "SubState1_2".to_string(),
                                "InternalEvent1".to_string(),
                            )
                        )
                        .finish_substate()
                    .add_state("SubState1_2".to_string())
                        .add_transition(
                            Transition::new(
                                "t2".to_string(),
                                "SubState1_2".to_string(),
                                "SubState1_1".to_string(),
                                "InternalEvent2".to_string(),
                            )
                        )
                        .finish_substate()
                    .finish_subregion()
                .add_transition(
                    Transition::new(
                        "t3".to_string(),
                        "SuperState1".to_string(),
                        "SuperState2".to_string(),
                        "ExternalEvent1".to_string(),
                    )
                )
                .finish_state()
            .add_state("SuperState2".to_string())
                .add_subregion("super2_region".to_string())
                    .add_state("SubState2_1".to_string())
                        .initial()
                        .finish_substate()
                    .add_state("SubState2_2".to_string())
                        .final_state()
                        .finish_substate()
                    .finish_subregion()
                .finish_state()
            .finish_region()
        .build()
}

criterion_group!(
    benches,
    benchmark_state_machine_creation,
    benchmark_promela_generation,
    benchmark_tla_generation,
    benchmark_state_machine_queries,
    benchmark_complex_state_machine
);
criterion_main!(benches);