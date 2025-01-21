use super::TestUtils;
use crate::ai::CommandChain;
use crate::executor::chain::ChainExecutor;

#[tokio::test]
async fn test_command_chain_validation() {
    let chain = TestUtils::create_test_command_chain();

    // Test step validation
    assert!(!chain.steps.is_empty(), "Chain should have steps");
    assert!(
        chain.steps.iter().all(|step| !step.command.is_empty()),
        "All steps should have commands"
    );

    // Test impact calculation
    assert!(
        chain.total_impact.cpu_usage > 0.0,
        "Total CPU impact should be calculated"
    );
    assert!(
        chain.total_impact.memory_usage > 0.0,
        "Total memory impact should be calculated"
    );
}

#[tokio::test]
async fn test_command_chain_execution_order() {
    let mut chain = TestUtils::create_test_command_chain();
    chain.steps.push(chain.steps[0].clone()); // Add a second step

    let mut executor = ChainExecutor::new(chain);

    // Execute first step
    let result1 = executor.execute_next().await;
    assert!(result1.is_ok(), "First step should execute successfully");

    // Execute second step
    let result2 = executor.execute_next().await;
    assert!(result2.is_ok(), "Second step should execute successfully");

    // Should be complete
    assert!(
        executor.is_complete(),
        "Executor should be complete after all steps"
    );
}
