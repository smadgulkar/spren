use super::TestUtils;
use crate::ai::{self, AIError};
use crate::config::Config;
use crate::executor::chain::ChainExecutor;
use crate::shell::ShellType;

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
