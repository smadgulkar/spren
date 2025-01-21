use super::TestUtils;
use crate::ai::AIError;
use crate::executor::chain::ChainExecutor;

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
