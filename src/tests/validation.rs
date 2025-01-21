use super::TestUtils;
use crate::executor::validation::{CommandValidator, WarningSeverity};
use crate::shell::ShellType;
use anyhow::Result;

#[test]
fn test_dangerous_command_detection() -> Result<()> {
    let validator = CommandValidator::new(ShellType::Bash);
    let mut chain = TestUtils::create_test_command_chain();
    chain.steps[0].command = "rm -rf /".to_string();

    let report = validator.validate_chain(&chain)?;
    assert!(report.has_high_severity_warnings());
    Ok(())
}

#[test]
fn test_command_syntax_validation() -> Result<()> {
    let validator = CommandValidator::new(ShellType::Bash);
    let mut chain = TestUtils::create_test_command_chain();
    chain.steps[0].command = "ls ||| grep test".to_string();

    let report = validator.validate_chain(&chain)?;
    assert!(report.has_errors());
    Ok(())
}

#[test]
fn test_resource_impact_validation() -> Result<()> {
    let validator = CommandValidator::new(ShellType::Bash);
    let mut chain = TestUtils::create_test_command_chain();
    chain.steps[0].impact.cpu_usage = 0.9;

    let report = validator.validate_chain(&chain)?;
    assert!(report
        .warnings
        .iter()
        .any(|w| w.severity == WarningSeverity::Medium));
    Ok(())
} 