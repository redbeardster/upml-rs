use std::time::Duration;
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use super::runner::VerificationTool;

/// Overall verification status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VerificationStatus {
    /// Verification completed successfully (all properties hold)
    Success,
    /// Verification found property violations
    Failed,
    /// Verification could not complete due to errors
    Error,
}

impl std::fmt::Display for VerificationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VerificationStatus::Success => write!(f, "✅ Success"),
            VerificationStatus::Failed => write!(f, "❌ Failed"),
            VerificationStatus::Error => write!(f, "🔥 Error"),
        }
    }
}

/// Result from a single verification tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub tool: VerificationTool,
    pub status: VerificationStatus,
    pub execution_time: Duration,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
}

impl ToolResult {
    /// Get a summary of this tool's result
    pub fn summary(&self) -> String {
        format!(
            "{}: {} ({:.2}s)",
            self.tool.display_name(),
            self.status,
            self.execution_time.as_secs_f64()
        )
    }

    /// Check if this result indicates success
    pub fn is_success(&self) -> bool {
        self.status == VerificationStatus::Success
    }

    /// Check if this result indicates failure (property violation)
    pub fn is_failed(&self) -> bool {
        self.status == VerificationStatus::Failed
    }

    /// Check if this result indicates an error
    pub fn is_error(&self) -> bool {
        self.status == VerificationStatus::Error
    }

    /// Extract counterexamples or traces from the output
    pub fn extract_counterexamples(&self) -> Vec<String> {
        let mut counterexamples = Vec::new();
        
        match self.tool {
            VerificationTool::Spin => {
                // SPIN counterexamples are in the trail file or stdout
                if self.stdout.contains("trail") {
                    counterexamples.push("SPIN trail file generated".to_string());
                }
            }
            VerificationTool::TlaPlus => {
                // TLA+ shows error traces in stdout
                if self.stdout.contains("Error:") {
                    let lines: Vec<&str> = self.stdout.lines().collect();
                    for (i, line) in lines.iter().enumerate() {
                        if line.contains("Error:") {
                            // Collect the error and following context
                            let context: Vec<&str> = lines.iter().skip(i).take(10).cloned().collect();
                            counterexamples.push(context.join("\n"));
                            break;
                        }
                    }
                }
            }
            VerificationTool::NuSMV => {
                // NuSMV shows counterexamples with "as demonstrated by"
                if self.stdout.contains("as demonstrated by") {
                    let lines: Vec<&str> = self.stdout.lines().collect();
                    let mut in_counterexample = false;
                    let mut current_example = Vec::new();
                    
                    for line in lines {
                        if line.contains("as demonstrated by") {
                            in_counterexample = true;
                            current_example.clear();
                            current_example.push(line.to_string());
                        } else if in_counterexample {
                            if line.trim().is_empty() && !current_example.is_empty() {
                                counterexamples.push(current_example.join("\n"));
                                current_example.clear();
                                in_counterexample = false;
                            } else {
                                current_example.push(line.to_string());
                            }
                        }
                    }
                    
                    if !current_example.is_empty() {
                        counterexamples.push(current_example.join("\n"));
                    }
                }
            }
            VerificationTool::Alloy => {
                // Alloy shows instances when SAT
                if self.stdout.contains("SAT") && !self.stdout.contains("UNSAT") {
                    counterexamples.push("Alloy found satisfying instance".to_string());
                }
            }
        }
        
        counterexamples
    }

    /// Extract performance metrics from the output
    pub fn extract_metrics(&self) -> HashMap<String, String> {
        let mut metrics = HashMap::new();
        
        metrics.insert("execution_time".to_string(), format!("{:.2}s", self.execution_time.as_secs_f64()));
        
        match self.tool {
            VerificationTool::Spin => {
                // Extract SPIN-specific metrics
                if let Some(states) = self.extract_spin_states() {
                    metrics.insert("states_explored".to_string(), states);
                }
                if let Some(memory) = self.extract_spin_memory() {
                    metrics.insert("memory_used".to_string(), memory);
                }
            }
            VerificationTool::TlaPlus => {
                // Extract TLA+ metrics
                if let Some(states) = self.extract_tla_states() {
                    metrics.insert("states_generated".to_string(), states);
                }
            }
            VerificationTool::NuSMV => {
                // NuSMV metrics are usually not detailed in output
                metrics.insert("tool_version".to_string(), "NuSMV 2.x".to_string());
            }
            VerificationTool::Alloy => {
                // Extract Alloy metrics
                if let Some(vars) = self.extract_alloy_variables() {
                    metrics.insert("variables".to_string(), vars);
                }
            }
        }
        
        metrics
    }

    fn extract_spin_states(&self) -> Option<String> {
        for line in self.stdout.lines() {
            if line.contains("states") && (line.contains("reached") || line.contains("stored")) {
                return Some(line.trim().to_string());
            }
        }
        None
    }

    fn extract_spin_memory(&self) -> Option<String> {
        for line in self.stdout.lines() {
            if line.contains("memory") || line.contains("MB") || line.contains("KB") {
                return Some(line.trim().to_string());
            }
        }
        None
    }

    fn extract_tla_states(&self) -> Option<String> {
        for line in self.stdout.lines() {
            if line.contains("states generated") {
                return Some(line.trim().to_string());
            }
        }
        None
    }

    fn extract_alloy_variables(&self) -> Option<String> {
        for line in self.stdout.lines() {
            if line.contains("vars") || line.contains("clauses") {
                return Some(line.trim().to_string());
            }
        }
        None
    }
}

/// Complete verification result containing all tool results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    pub overall_status: VerificationStatus,
    pub tool_results: HashMap<VerificationTool, ToolResult>,
    pub total_time: Duration,
    pub summary: String,
}

impl VerificationResult {
    /// Print a formatted report of the verification results
    pub fn print_report(&self) {
        println!("\n🔍 Verification Results Report");
        println!("==============================");
        println!();
        
        println!("📊 Overall Status: {}", self.overall_status);
        println!("⏱️  Total Time: {:.2}s", self.total_time.as_secs_f64());
        println!("🔧 Tools Run: {}", self.tool_results.len());
        println!();

        // Individual tool results
        println!("📋 Individual Results:");
        for (tool, result) in &self.tool_results {
            println!("  • {}: {} ({:.2}s)", 
                    tool.display_name(), 
                    result.status, 
                    result.execution_time.as_secs_f64());
            
            // Show counterexamples if any
            let counterexamples = result.extract_counterexamples();
            if !counterexamples.is_empty() {
                println!("    Counterexamples found:");
                for (i, example) in counterexamples.iter().enumerate() {
                    println!("      {}. {}", i + 1, example.lines().next().unwrap_or(""));
                }
            }
            
            // Show key metrics
            let metrics = result.extract_metrics();
            if metrics.len() > 1 { // More than just execution_time
                println!("    Metrics:");
                for (key, value) in metrics {
                    if key != "execution_time" {
                        println!("      {}: {}", key, value);
                    }
                }
            }
        }
        
        println!();
        println!("📝 Summary: {}", self.summary);
        
        // Recommendations
        self.print_recommendations();
    }

    /// Print recommendations based on results
    fn print_recommendations(&self) {
        println!();
        println!("💡 Recommendations:");
        
        let success_count = self.tool_results.values().filter(|r| r.is_success()).count();
        let failed_count = self.tool_results.values().filter(|r| r.is_failed()).count();
        let error_count = self.tool_results.values().filter(|r| r.is_error()).count();
        
        if error_count > 0 {
            println!("  ⚠️  {} tools had execution errors - check tool installation and model syntax", error_count);
        }
        
        if failed_count > 0 {
            println!("  🔍 {} tools found property violations - review counterexamples", failed_count);
            println!("     Consider refining your model or adjusting properties");
        }
        
        if success_count == self.tool_results.len() {
            println!("  ✅ All tools verified successfully - your model appears correct!");
        } else if success_count > 0 {
            println!("  📊 Mixed results - some tools succeeded while others found issues");
            println!("     This may indicate tool-specific limitations or different property coverage");
        }
        
        // Tool-specific recommendations
        for (tool, result) in &self.tool_results {
            match (tool, &result.status) {
                (VerificationTool::Spin, VerificationStatus::Error) => {
                    println!("  🔧 SPIN error - check Promela syntax and compilation");
                }
                (VerificationTool::TlaPlus, VerificationStatus::Error) => {
                    println!("  🔧 TLA+ error - verify TLA+ syntax and module structure");
                }
                (VerificationTool::NuSMV, VerificationStatus::Failed) => {
                    println!("  🔧 NuSMV found counterexamples - examine temporal logic violations");
                }
                (VerificationTool::Alloy, VerificationStatus::Failed) => {
                    println!("  🔧 Alloy found structural issues - review constraints and assertions");
                }
                _ => {}
            }
        }
    }

    /// Export results to JSON
    pub fn to_json(&self) -> crate::Result<String> {
        serde_json::to_string_pretty(self).map_err(|e| anyhow::anyhow!("JSON serialization failed: {}", e))
    }

    /// Load results from JSON
    pub fn from_json(json: &str) -> crate::Result<Self> {
        serde_json::from_str(json).map_err(|e| anyhow::anyhow!("JSON deserialization failed: {}", e))
    }

    /// Get successful tools
    pub fn successful_tools(&self) -> Vec<&VerificationTool> {
        self.tool_results
            .iter()
            .filter(|(_, result)| result.is_success())
            .map(|(tool, _)| tool)
            .collect()
    }

    /// Get failed tools
    pub fn failed_tools(&self) -> Vec<&VerificationTool> {
        self.tool_results
            .iter()
            .filter(|(_, result)| result.is_failed())
            .map(|(tool, _)| tool)
            .collect()
    }

    /// Get tools with errors
    pub fn error_tools(&self) -> Vec<&VerificationTool> {
        self.tool_results
            .iter()
            .filter(|(_, result)| result.is_error())
            .map(|(tool, _)| tool)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verification_status_display() {
        assert_eq!(format!("{}", VerificationStatus::Success), "✅ Success");
        assert_eq!(format!("{}", VerificationStatus::Failed), "❌ Failed");
        assert_eq!(format!("{}", VerificationStatus::Error), "🔥 Error");
    }

    #[test]
    fn test_tool_result_summary() {
        let result = ToolResult {
            tool: VerificationTool::Spin,
            status: VerificationStatus::Success,
            execution_time: Duration::from_secs(5),
            stdout: "Test output".to_string(),
            stderr: "".to_string(),
            exit_code: Some(0),
        };

        assert_eq!(result.summary(), "SPIN: ✅ Success (5.00s)");
        assert!(result.is_success());
        assert!(!result.is_failed());
        assert!(!result.is_error());
    }

    #[test]
    fn test_counterexample_extraction() {
        let result = ToolResult {
            tool: VerificationTool::NuSMV,
            status: VerificationStatus::Failed,
            execution_time: Duration::from_secs(2),
            stdout: "-- specification EF state = Error is false\n-- as demonstrated by the following execution sequence\nTrace Description: CTL Counterexample".to_string(),
            stderr: "".to_string(),
            exit_code: Some(0),
        };

        let counterexamples = result.extract_counterexamples();
        assert!(!counterexamples.is_empty());
        assert!(counterexamples[0].contains("as demonstrated by"));
    }
}