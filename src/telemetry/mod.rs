use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::{error, info, warn};

mod storage;
pub use storage::TelemetryStorage;

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorTelemetry {
    pub error_type: ErrorType,
    pub message: String,
    pub timestamp: DateTime<Utc>,
    pub context: ErrorContext,
    pub stack_trace: Option<String>,
    pub recovery_action: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ErrorType {
    APIError,
    NetworkError,
    ValidationError,
    ExecutionError,
    FileSystemError,
    ConfigurationError,
    UnknownError,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorContext {
    pub command: Option<String>,
    pub working_directory: Option<PathBuf>,
    pub shell_type: Option<String>,
    pub os_info: Option<String>,
    pub additional_data: serde_json::Value,
}

pub struct TelemetryManager {
    storage: TelemetryStorage,
    error_patterns: ErrorPatternDB,
}

impl TelemetryManager {
    pub fn new(storage_path: PathBuf) -> Self {
        Self {
            storage: TelemetryStorage::new(storage_path),
            error_patterns: ErrorPatternDB::new(),
        }
    }

    pub async fn record_error(
        &self,
        error: impl std::error::Error,
        context: ErrorContext,
    ) -> Result<()> {
        let error_type = self.error_patterns.classify_error(&error);
        let recovery_action = self.error_patterns.get_recovery_suggestion(&error);

        let telemetry = ErrorTelemetry {
            error_type,
            message: error.to_string(),
            timestamp: Utc::now(),
            context,
            stack_trace: std::backtrace::Backtrace::capture().to_string().into(),
            recovery_action,
        };

        // Log the error
        error!(
            error_type = ?telemetry.error_type,
            message = %telemetry.message,
            timestamp = %telemetry.timestamp,
            "Error occurred"
        );

        // Store the error
        self.storage.store_error(&telemetry).await?;

        Ok(())
    }

    pub async fn get_error_patterns(&self) -> Result<Vec<ErrorPattern>> {
        self.error_patterns.get_frequent_patterns().await
    }
}

struct ErrorPatternDB {
    patterns: Vec<ErrorPattern>,
}

impl ErrorPatternDB {
    fn new() -> Self {
        Self {
            patterns: vec![
                ErrorPattern {
                    pattern: "connection refused".to_string(),
                    error_type: ErrorType::NetworkError,
                    recovery_suggestion: "Check network connectivity and try again".to_string(),
                    frequency: 0,
                },
                ErrorPattern {
                    pattern: "permission denied".to_string(),
                    error_type: ErrorType::ExecutionError,
                    recovery_suggestion: "Try running with elevated privileges".to_string(),
                    frequency: 0,
                },
                // Add more patterns
            ],
        }
    }

    fn classify_error(&self, error: &impl std::error::Error) -> ErrorType {
        let error_msg = error.to_string().to_lowercase();
        for pattern in &self.patterns {
            if error_msg.contains(&pattern.pattern) {
                return pattern.error_type.clone();
            }
        }
        ErrorType::UnknownError
    }

    fn get_recovery_suggestion(&self, error: &impl std::error::Error) -> Option<String> {
        let error_msg = error.to_string().to_lowercase();
        for pattern in &self.patterns {
            if error_msg.contains(&pattern.pattern) {
                return Some(pattern.recovery_suggestion.clone());
            }
        }
        None
    }

    async fn get_frequent_patterns(&self) -> Result<Vec<ErrorPattern>> {
        Ok(self.patterns.clone())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorPattern {
    pattern: String,
    error_type: ErrorType,
    recovery_suggestion: String,
    frequency: u32,
}
