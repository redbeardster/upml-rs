use upml_rs::state_machine::{StateMachineBuilder, Transition};
use upml_rs::generators;
use std::io;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Создаем простую state machine программно
    let sm = StateMachineBuilder::new("DemoMachine".to_string())
        .add_region("main_region".to_string())
            .add_state("Idle".to_string())
                .initial()
                .add_transition(Transition::new(
                    "t1".to_string(),
                    "Idle".to_string(),
                    "Active".to_string(),
                    "Start".to_string(),
                ))
                .finish_state()
            .add_state("Active".to_string())
                .add_transition(Transition::new(
                    "t2".to_string(),
                    "Active".to_string(),
                    "Idle".to_string(),
                    "Stop".to_string(),
                ))
                .finish_state()
            .finish_region()
        .build();

    println!("=== Generated Promela FSM ===");
    generators::promela::generate_fsm(io::stdout(), &sm)?;
    
    println!("\n=== Generated TLA+ FSM ===");
    generators::tla::generate_fsm(io::stdout(), &sm)?;

    Ok(())
}