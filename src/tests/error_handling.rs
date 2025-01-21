use super::TestUtils;
use crate::ai::AIError;
use crate::executor::chain::ChainExecutor;
use crate::analysis::ProjectAnalyzer;
use crate::config::Config;
use anyhow::Result;
use std::path::PathBuf;

#[tokio::test]
async fn test_invalid_command_handling() {
    let mut chain = TestUtils::create_test_command_chain();
    chain.steps[0].command = "invalid_command_123".to_string();

    let mut executor = ChainExecutor::new(chain);
    let result = executor.execute_next().await;

    assert!(result.is_err(), "Invalid command should return an error");
}

#[tokio::test]
async fn test_rollback_mechanism() {
    let mut chain = TestUtils::create_test_command_chain();
    chain.steps[0].rollback_command = Some("echo 'rolling back'".to_string());

    let mut executor = ChainExecutor::new(chain);
    let result = executor.execute_next().await;
    assert!(result.is_ok(), "Initial execution should succeed");

    let rollback_result = executor.rollback().await;
    assert!(rollback_result.is_ok(), "Rollback should succeed");
}

#[tokio::test]
async fn test_invalid_project_path() -> Result<()> {
    let analyzer = ProjectAnalyzer::new(PathBuf::from("/nonexistent/path"));
    let result = analyzer.analyze().await;
    assert!(result.is_err());
    Ok(())
}

#[tokio::test]
async fn test_invalid_api_key() -> Result<()> {
    let utils = TestUtils::new()?;
    let project_dir = utils.create_test_project()?;
    
    let mut config = Config::load_test_config()?;
    config.ai.anthropic_api_key = Some("invalid_key".to_string());
    
    let analyzer = ProjectAnalyzer::new(&project_dir);
    let result = analyzer.analyze_with_llm(&config).await;
    
    match result {
        Err(e) => {
            let error_string = e.to_string();
            assert!(error_string.contains("Invalid API key") || error_string.contains("Unauthorized"));
            Ok(())
        }
        Ok(_) => panic!("Expected error for invalid API key"),
    }
}
