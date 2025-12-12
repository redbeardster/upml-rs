use std::collections::HashMap;
use std::path::PathBuf;
use upml_rs::verification::{VerificationRunner, VerificationTool, RunConfig};
use upml_rs::{parse_plantuml, generators};

#[test]
fn test_verification_runner_creation() {
    let config = RunConfig::default();
    let runner = VerificationRunner::new(config);
    
    // Should have detected at least some tools or none
    let available = runner.available_tools();
    println!("Available tools: {:?}", available);
    
    // Test should pass regardless of what tools are available
    assert!(available.len() <= 4); // At most 4 tools supported
}

#[test]
fn test_tool_detection() {
    let tools = VerificationRunner::detect_available_tools();
    
    // Should return a valid list (may be empty if no tools installed)
    assert!(tools.len() <= 4);
    
    // Each detected tool should be available
    for tool in &tools {
        assert!(VerificationRunner::is_tool_available(tool));
    }
}

#[test]
fn test_verification_tool_properties() {
    // Test all tool properties
    let tools = [
        VerificationTool::Spin,
        VerificationTool::TlaPlus,
        VerificationTool::NuSMV,
        VerificationTool::Alloy,
    ];
    
    for tool in &tools {
        // Should have valid executable name
        assert!(!tool.executable().is_empty());
        
        // Should have valid file extension
        assert!(!tool.file_extension().is_empty());
        
        // Should have valid display name
        assert!(!tool.display_name().is_empty());
        
        // Extensions should be reasonable
        match tool {
            VerificationTool::Spin => {
                assert_eq!(tool.file_extension(), "pml");
                assert_eq!(tool.executable(), "spin");
            }
            VerificationTool::TlaPlus => {
                assert_eq!(tool.file_extension(), "tla");
                assert_eq!(tool.executable(), "tlc");
            }
            VerificationTool::NuSMV => {
                assert_eq!(tool.file_extension(), "smv");
                assert_eq!(tool.executable(), "NuSMV");
            }
            VerificationTool::Alloy => {
                assert_eq!(tool.file_extension(), "als");
                assert_eq!(tool.executable(), "alloy");
            }
        }
    }
}

#[test]
fn test_model_generation_for_verification() -> upml_rs::Result<()> {
    // Create a simple state machine
    let plantuml_content = r#"
        @startuml
        [*] --> Idle
        Idle --> Active : start
        Active --> Idle : stop
        Active --> [*]
        @enduml
    "#;
    
    let state_machine = parse_plantuml(plantuml_content)?;
    
    // Test that we can generate models for all tool types
    let tools = [
        VerificationTool::Spin,
        VerificationTool::TlaPlus,
        VerificationTool::NuSMV,
        VerificationTool::Alloy,
    ];
    
    for tool in &tools {
        let mut output = Vec::new();
        
        match tool {
            VerificationTool::Spin => {
                generators::promela::generate_fsm(&mut output, &state_machine)?;
            }
            VerificationTool::TlaPlus => {
                generators::tla::generate_fsm(&mut output, &state_machine)?;
            }
            VerificationTool::NuSMV => {
                generators::nusmv::generate_model(&mut output, &state_machine)?;
            }
            VerificationTool::Alloy => {
                generators::alloy::generate_model(&mut output, &state_machine)?;
            }
        }
        
        let generated = String::from_utf8(output).unwrap();
        
        // Should have generated some content
        assert!(!generated.is_empty());
        
        // Should contain state machine elements
        assert!(generated.contains("Idle") || generated.contains("idle"));
        assert!(generated.contains("Active") || generated.contains("active"));
    }
    
    Ok(())
}

#[test]
fn test_run_config_customization() {
    let mut config = RunConfig::default();
    
    // Test default values
    assert_eq!(config.timeout, Some(300));
    assert!(config.parallel);
    assert!(!config.keep_files);
    
    // Test customization
    config.timeout = Some(600);
    config.parallel = false;
    config.keep_files = true;
    
    // Add custom tool arguments
    config.tool_args.insert(
        VerificationTool::Spin,
        vec!["-a".to_string(), "-m10000".to_string()]
    );
    
    assert_eq!(config.timeout, Some(600));
    assert!(!config.parallel);
    assert!(config.keep_files);
    assert!(config.tool_args.contains_key(&VerificationTool::Spin));
}

#[test]
fn test_verification_with_mock_files() {
    // This test verifies the verification runner can handle file operations
    // without actually running external tools
    
    let config = RunConfig {
        timeout: Some(10), // Short timeout for testing
        work_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        parallel: false, // Sequential for predictable testing
        keep_files: false,
        tool_args: HashMap::new(),
    };
    
    let runner = VerificationRunner::new(config);
    
    // Create empty model files map (simulating no available tools)
    let model_files = HashMap::new();
    
    // Should handle empty input gracefully
    let result = runner.run_all(&model_files);
    assert!(result.is_ok());
    
    let verification_result = result.unwrap();
    assert_eq!(verification_result.tool_results.len(), 0);
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;
    
    #[test]
    fn test_end_to_end_verification_workflow() -> upml_rs::Result<()> {
        // Only run if we have at least one verification tool available
        let available_tools = VerificationRunner::detect_available_tools();
        if available_tools.is_empty() {
            println!("Skipping end-to-end test: no verification tools available");
            return Ok(());
        }
        
        // Create temporary directory for test files
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        
        // Create a simple state machine
        let plantuml_content = r#"
            @startuml
            [*] --> Start
            Start --> End : finish
            End --> [*]
            @enduml
        "#;
        
        let state_machine = parse_plantuml(plantuml_content)?;
        
        // Generate model files for available tools
        let mut model_files = HashMap::new();
        
        for tool in &available_tools {
            let filename = format!("test_model.{}", tool.file_extension());
            let file_path = temp_path.join(&filename);
            
            let mut file = fs::File::create(&file_path)?;
            
            match tool {
                VerificationTool::Spin => {
                    generators::promela::generate_fsm(&mut file, &state_machine)?;
                }
                VerificationTool::TlaPlus => {
                    generators::tla::generate_fsm(&mut file, &state_machine)?;
                }
                VerificationTool::NuSMV => {
                    generators::nusmv::generate_model(&mut file, &state_machine)?;
                }
                VerificationTool::Alloy => {
                    generators::alloy::generate_model(&mut file, &state_machine)?;
                }
            }
            
            model_files.insert(tool.clone(), file_path);
        }
        
        // Configure verification runner
        let config = RunConfig {
            timeout: Some(30), // 30 seconds should be enough for simple models
            work_dir: temp_path.to_path_buf(),
            parallel: false, // Sequential for predictable testing
            keep_files: true, // Keep files for inspection
            tool_args: HashMap::new(),
        };
        
        let runner = VerificationRunner::new(config);
        
        // Run verification
        let result = runner.run_all(&model_files)?;
        
        // Verify results
        assert_eq!(result.tool_results.len(), available_tools.len());
        
        // Each tool should have produced a result
        for tool in &available_tools {
            assert!(result.tool_results.contains_key(tool));
            let tool_result = &result.tool_results[tool];
            assert_eq!(tool_result.tool, *tool);
            
            // Should have some execution time
            assert!(tool_result.execution_time.as_millis() > 0);
        }
        
        // Should have a summary
        assert!(!result.summary.is_empty());
        
        println!("End-to-end verification test completed successfully!");
        println!("Tools tested: {:?}", available_tools);
        println!("Results: {}", result.summary);
        
        Ok(())
    }
}