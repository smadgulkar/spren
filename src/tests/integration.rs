use super::TestUtils;
use crate::ai::{self, AIError};
use crate::analysis::ProjectAnalyzer;
use crate::config::Config;
use crate::executor::chain::ChainExecutor;
use crate::shell::ShellType;
use anyhow::Result;

#[tokio::test]
async fn test_command_chain_execution() {
    let chain = TestUtils::create_test_command_chain();
    let mut executor = ChainExecutor::new(chain);

    let result = executor.execute_next().await;
    assert!(
        result.is_ok(),
        "Command execution failed: {:?}",
        result.err()
    );

    if let Ok(Some(output)) = result {
        assert!(output.success, "Command execution was not successful");
    }
}

#[tokio::test]
async fn test_api_integration() {
    let config = Config::default();
    let result = ai::get_command_chain("list files in current directory", &config).await;

    match result {
        Ok(chain) => {
            assert!(!chain.steps.is_empty(), "Command chain should not be empty");
            assert!(
                !chain.explanation.is_empty(),
                "Explanation should not be empty"
            );
        }
        Err(e) => panic!("API integration failed: {:?}", e),
    }
}

#[tokio::test]
async fn test_project_analysis() -> Result<()> {
    let utils = TestUtils::new()?;
    let project_dir = utils.create_test_project()?;

    let analyzer = ProjectAnalyzer::new(&project_dir);
    let analysis = analyzer.analyze().await?;

    // Test language detection
    assert!(analysis.languages.iter().any(|l| l.name == "Rust"));
    assert!(analysis.languages.iter().any(|l| l.name == "JavaScript"));
    assert!(analysis.languages.iter().any(|l| l.name == "Python"));

    // Test framework detection
    assert!(analysis.frameworks.iter().any(|f| f.name == "Rust/Cargo"));
    assert!(analysis.frameworks.iter().any(|f| f.name == "Node.js"));
    assert!(analysis.frameworks.iter().any(|f| f.name == "pip"));

    // Test dependency analysis
    assert!(analysis.dependencies.iter().any(|d| d.name == "serde"));
    assert!(analysis.dependencies.iter().any(|d| d.name == "express"));
    assert!(analysis.dependencies.iter().any(|d| d.name == "requests"));

    Ok(())
}

#[tokio::test]
async fn test_llm_analysis() -> Result<()> {
    let utils = TestUtils::new()?;
    let project_dir = utils.create_test_project()?;

    let config = Config::load_test_config()?;
    let analyzer = ProjectAnalyzer::new(&project_dir);
    let analysis = analyzer.analyze_with_llm(&config).await?;

    assert!(analysis.llm_insights.is_some());
    let insights = analysis.llm_insights.unwrap();
    assert!(insights.contains("Project Overview"));
    assert!(insights.contains("Architecture Analysis"));

    Ok(())
}
