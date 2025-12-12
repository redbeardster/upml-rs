use std::process::{Command, Stdio};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use crate::Result;
use super::results::{VerificationResult, VerificationStatus, ToolResult};

/// Supported verification tools
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum VerificationTool {
    Spin,
    TlaPlus,
    NuSMV,
    Alloy,
}

impl VerificationTool {
    /// Get the executable name for this tool
    pub fn executable(&self) -> &'static str {
        match self {
            VerificationTool::Spin => "spin",
            VerificationTool::TlaPlus => "tlc",
            VerificationTool::NuSMV => "NuSMV",
            VerificationTool::Alloy => "alloy",
        }
    }

    /// Get the file extension for this tool's models
    pub fn file_extension(&self) -> &'static str {
        match self {
            VerificationTool::Spin => "pml",
            VerificationTool::TlaPlus => "tla",
            VerificationTool::NuSMV => "smv",
            VerificationTool::Alloy => "als",
        }
    }

    /// Get human-readable name
    pub fn display_name(&self) -> &'static str {
        match self {
            VerificationTool::Spin => "SPIN",
            VerificationTool::TlaPlus => "TLA+",
            VerificationTool::NuSMV => "NuSMV",
            VerificationTool::Alloy => "Alloy",
        }
    }
}

/// Configuration for running verification
#[derive(Debug, Clone)]
pub struct RunConfig {
    /// Timeout for each tool (in seconds)
    pub timeout: Option<u64>,
    /// Working directory for verification
    pub work_dir: PathBuf,
    /// Whether to run tools in parallel
    pub parallel: bool,
    /// Whether to keep intermediate files
    pub keep_files: bool,
    /// Additional arguments for each tool
    pub tool_args: std::collections::HashMap<VerificationTool, Vec<String>>,
}

impl Default for RunConfig {
    fn default() -> Self {
        Self {
            timeout: Some(300), // 5 minutes default
            work_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            parallel: true,
            keep_files: false,
            tool_args: std::collections::HashMap::new(),
        }
    }
}

/// Main verification runner
pub struct VerificationRunner {
    config: RunConfig,
    available_tools: Vec<VerificationTool>,
}

impl VerificationRunner {
    /// Create a new verification runner
    pub fn new(config: RunConfig) -> Self {
        let available_tools = Self::detect_available_tools();
        Self {
            config,
            available_tools,
        }
    }

    /// Create with default configuration
    pub fn default() -> Self {
        Self::new(RunConfig::default())
    }

    /// Detect which verification tools are available on the system
    pub fn detect_available_tools() -> Vec<VerificationTool> {
        let tools = [
            VerificationTool::Spin,
            VerificationTool::TlaPlus,
            VerificationTool::NuSMV,
            VerificationTool::Alloy,
        ];

        tools
            .iter()
            .filter(|tool| Self::is_tool_available(tool))
            .cloned()
            .collect()
    }

    /// Check if a specific tool is available
    pub fn is_tool_available(tool: &VerificationTool) -> bool {
        Command::new(tool.executable())
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|status| status.success())
            .unwrap_or_else(|_| {
                // Try alternative version checks
                match tool {
                    VerificationTool::Spin => {
                        Command::new("spin")
                            .arg("-V")
                            .stdout(Stdio::null())
                            .stderr(Stdio::null())
                            .status()
                            .map(|status| status.success())
                            .unwrap_or(false)
                    }
                    VerificationTool::NuSMV => {
                        Command::new("NuSMV")
                            .stdout(Stdio::null())
                            .stderr(Stdio::null())
                            .status()
                            .is_ok()
                    }
                    VerificationTool::Alloy => {
                        Command::new("alloy")
                            .arg("version")
                            .stdout(Stdio::null())
                            .stderr(Stdio::null())
                            .status()
                            .map(|status| status.success())
                            .unwrap_or(false)
                    }
                    _ => false,
                }
            })
    }

    /// Get list of available tools
    pub fn available_tools(&self) -> &[VerificationTool] {
        &self.available_tools
    }

    /// Run verification for all available tools
    pub fn run_all(&self, model_files: &std::collections::HashMap<VerificationTool, PathBuf>) -> Result<VerificationResult> {
        let start_time = Instant::now();
        let mut tool_results = std::collections::HashMap::new();

        if self.config.parallel {
            // Run tools in parallel using threads
            let handles: Vec<_> = self.available_tools
                .iter()
                .filter_map(|tool| {
                    model_files.get(tool).map(|file| {
                        let tool = tool.clone();
                        let file = file.clone();
                        let config = self.config.clone();
                        std::thread::spawn(move || {
                            Self::run_single_tool(&tool, &file, &config)
                        })
                    })
                })
                .collect();

            // Collect results
            for handle in handles {
                match handle.join() {
                    Ok(Ok((tool, result))) => {
                        tool_results.insert(tool, result);
                    }
                    Ok(Err(e)) => {
                        eprintln!("Tool execution error: {}", e);
                    }
                    Err(_) => {
                        eprintln!("Thread panic during verification");
                    }
                }
            }
        } else {
            // Run tools sequentially
            for tool in &self.available_tools {
                if let Some(file) = model_files.get(tool) {
                    match Self::run_single_tool(tool, file, &self.config) {
                        Ok((_, result)) => {
                            tool_results.insert(tool.clone(), result);
                        }
                        Err(e) => {
                            eprintln!("Error running {}: {}", tool.display_name(), e);
                        }
                    }
                }
            }
        }

        let total_time = start_time.elapsed();
        let overall_status = Self::determine_overall_status(&tool_results);
        let summary = Self::generate_summary(&tool_results);

        Ok(VerificationResult {
            overall_status,
            tool_results,
            total_time,
            summary,
        })
    }

    /// Run a single verification tool
    fn run_single_tool(tool: &VerificationTool, model_file: &Path, config: &RunConfig) -> Result<(VerificationTool, ToolResult)> {
        let start_time = Instant::now();
        
        println!("🔧 Running {} on {}...", tool.display_name(), model_file.display());

        let mut cmd = Command::new(tool.executable());
        
        // Add tool-specific arguments
        match tool {
            VerificationTool::Spin => {
                // For SPIN, we need to compile and run
                cmd.args(&["-a", model_file.to_str().unwrap()]);
            }
            VerificationTool::TlaPlus => {
                cmd.arg(model_file.to_str().unwrap());
            }
            VerificationTool::NuSMV => {
                cmd.arg(model_file.to_str().unwrap());
            }
            VerificationTool::Alloy => {
                cmd.args(&["exec", model_file.to_str().unwrap()]);
            }
        }

        // Add custom arguments if provided
        if let Some(args) = config.tool_args.get(tool) {
            cmd.args(args);
        }

        // Set working directory
        cmd.current_dir(&config.work_dir);

        // Execute with timeout
        let output = if let Some(timeout_secs) = config.timeout {
            Self::run_with_timeout(cmd, Duration::from_secs(timeout_secs))?
        } else {
            cmd.output()?
        };

        let execution_time = start_time.elapsed();
        let success = output.status.success();
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        // Parse tool-specific results
        let status = Self::parse_tool_output(tool, &stdout, &stderr, success);

        let result = ToolResult {
            tool: tool.clone(),
            status,
            execution_time,
            stdout,
            stderr,
            exit_code: output.status.code(),
        };

        println!("✅ {} completed in {:.2}s", tool.display_name(), execution_time.as_secs_f64());

        Ok((tool.clone(), result))
    }

    /// Run command with timeout
    fn run_with_timeout(mut cmd: Command, timeout: Duration) -> Result<std::process::Output> {
        use std::sync::mpsc;
        use std::thread;

        let (tx, rx) = mpsc::channel();
        
        let handle = thread::spawn(move || {
            let result = cmd.output();
            let _ = tx.send(result);
        });

        match rx.recv_timeout(timeout) {
            Ok(result) => result.map_err(|e| anyhow::anyhow!("Command execution failed: {}", e)),
            Err(_) => {
                // Timeout occurred - try to clean up the thread
                let _ = handle.join();
                Err(anyhow::anyhow!("Command timed out after {:?}", timeout))
            }
        }
    }

    /// Parse tool-specific output to determine verification status
    fn parse_tool_output(tool: &VerificationTool, stdout: &str, stderr: &str, success: bool) -> VerificationStatus {
        if !success {
            return VerificationStatus::Error;
        }

        match tool {
            VerificationTool::Spin => {
                if stdout.contains("errors: 0") || stdout.contains("0 errors") {
                    VerificationStatus::Success
                } else if stdout.contains("error") || stderr.contains("error") {
                    VerificationStatus::Failed
                } else {
                    VerificationStatus::Success
                }
            }
            VerificationTool::TlaPlus => {
                if stdout.contains("Finished computing initial states") && !stdout.contains("Error:") {
                    VerificationStatus::Success
                } else if stdout.contains("Error:") || stderr.contains("Error:") {
                    VerificationStatus::Failed
                } else {
                    VerificationStatus::Success
                }
            }
            VerificationTool::NuSMV => {
                if stdout.contains("is true") || (stdout.contains("specification") && !stdout.contains("is false")) {
                    VerificationStatus::Success
                } else if stdout.contains("is false") {
                    VerificationStatus::Failed
                } else {
                    VerificationStatus::Success
                }
            }
            VerificationTool::Alloy => {
                if stdout.contains("UNSAT") || (stdout.contains("SAT") && !stdout.contains("0       UNSAT")) {
                    VerificationStatus::Success
                } else if stdout.contains("0       UNSAT") {
                    VerificationStatus::Failed
                } else {
                    VerificationStatus::Success
                }
            }
        }
    }

    /// Determine overall verification status
    fn determine_overall_status(results: &std::collections::HashMap<VerificationTool, ToolResult>) -> VerificationStatus {
        if results.is_empty() {
            return VerificationStatus::Error;
        }

        let mut has_success = false;
        let mut has_failed = false;
        let mut has_error = false;

        for result in results.values() {
            match result.status {
                VerificationStatus::Success => has_success = true,
                VerificationStatus::Failed => has_failed = true,
                VerificationStatus::Error => has_error = true,
            }
        }

        if has_error {
            VerificationStatus::Error
        } else if has_failed {
            VerificationStatus::Failed
        } else if has_success {
            VerificationStatus::Success
        } else {
            VerificationStatus::Error
        }
    }

    /// Generate summary of verification results
    fn generate_summary(results: &std::collections::HashMap<VerificationTool, ToolResult>) -> String {
        let total = results.len();
        let success = results.values().filter(|r| r.status == VerificationStatus::Success).count();
        let failed = results.values().filter(|r| r.status == VerificationStatus::Failed).count();
        let errors = results.values().filter(|r| r.status == VerificationStatus::Error).count();

        format!(
            "Verification completed: {} tools run, {} successful, {} failed, {} errors",
            total, success, failed, errors
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_detection() {
        let tools = VerificationRunner::detect_available_tools();
        // At least one tool should be available in most environments
        println!("Available tools: {:?}", tools);
    }

    #[test]
    fn test_verification_tool_properties() {
        assert_eq!(VerificationTool::Spin.executable(), "spin");
        assert_eq!(VerificationTool::Spin.file_extension(), "pml");
        assert_eq!(VerificationTool::Spin.display_name(), "SPIN");

        assert_eq!(VerificationTool::NuSMV.executable(), "NuSMV");
        assert_eq!(VerificationTool::NuSMV.file_extension(), "smv");
        assert_eq!(VerificationTool::NuSMV.display_name(), "NuSMV");
    }

    #[test]
    fn test_run_config_default() {
        let config = RunConfig::default();
        assert_eq!(config.timeout, Some(300));
        assert!(config.parallel);
        assert!(!config.keep_files);
    }
}