//! UPML-RS: Formal verification of UML state machines with Promela and TLA+/PlusCal
//! 
//! This is a Rust port of the original C++ upml tool.
//! It converts UML state machines (described in PlantUML) into formal verification models.

pub mod analysis;
pub mod ast;
pub mod parser;
pub mod state_machine;
pub mod generators;

pub use state_machine::StateMachine;
pub use parser::plantuml::parse_plantuml;
pub use generators::{promela, tla};

/// Main result type used throughout the library
pub type Result<T> = anyhow::Result<T>;

/// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");