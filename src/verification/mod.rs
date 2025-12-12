//! Verification tools integration and automation

pub mod runner;
pub mod results;
pub mod config;

pub use runner::{VerificationRunner, VerificationTool, RunConfig};
pub use results::{VerificationResult, VerificationStatus, ToolResult};
pub use config::{ToolConfig, generate_tool_config};