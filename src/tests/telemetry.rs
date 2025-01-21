use super::TestUtils;
use crate::telemetry::{ErrorContext, ErrorType, TelemetryManager};
use anyhow::{anyhow, Result};
use std::path::PathBuf;

#[tokio::test]
async fn test_error_recording() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let telemetry = TelemetryManager::new(temp_dir.path().to_path_buf());

    let context = ErrorContext {
        command: Some("test command".to_string()),
        working_directory: Some(PathBuf::from("/test")),
        shell_type: Some("bash".to_string()),
        os_info: None,
        additional_data: serde_json::json!({}),
    };

    telemetry
        .record_error(anyhow!("Test error"), context)
        .await?;

    let patterns = telemetry.get_error_patterns().await?;
    assert!(!patterns.is_empty());

    Ok(())
}

#[tokio::test]
async fn test_error_pattern_detection() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let telemetry = TelemetryManager::new(temp_dir.path().to_path_buf());

    let context = ErrorContext {
        command: Some("curl http://example.com".to_string()),
        working_directory: None,
        shell_type: None,
        os_info: None,
        additional_data: serde_json::json!({}),
    };

    // Record a network error
    telemetry
        .record_error(
            anyhow!("connection refused: failed to connect to host"),
            context,
        )
        .await?;

    let patterns = telemetry.get_error_patterns().await?;
    assert!(patterns
        .iter()
        .any(|p| matches!(p.error_type, ErrorType::NetworkError)));

    Ok(())
} 