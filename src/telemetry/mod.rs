use serde::Serialize;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::ai::AIError;

mod storage;
use storage::TelemetryStorage;

#[derive(Debug, Clone, Serialize)]
pub struct ErrorTelemetry {
    pub error_type: String,
    pub message: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub context: Option<String>,
}

#[derive(Debug)]
pub struct TelemetrySystem {
    errors: Arc<Mutex<Vec<ErrorTelemetry>>>,
    storage: TelemetryStorage,
}

impl TelemetrySystem {
    pub fn new(storage_path: std::path::PathBuf) -> Self {
        Self {
            errors: Arc::new(Mutex::new(Vec::new())),
            storage: TelemetryStorage::new(storage_path),
        }
    }

    pub async fn record_error(&self, error: impl Into<AIError>) {
        let error = error.into();
        let telemetry = ErrorTelemetry {
            error_type: format!("{:?}", error),
            message: error.to_string(),
            timestamp: chrono::Utc::now(),
            context: None,
        };
        
        // Store in memory
        self.errors.lock().await.push(telemetry.clone());
        
        // Persist to storage
        if let Err(e) = self.storage.store_error(&telemetry).await {
            eprintln!("Failed to store error telemetry: {}", e);
        }
    }

    pub async fn get_error_patterns(&self) -> Vec<ErrorTelemetry> {
        self.errors.lock().await.clone()
    }

    pub async fn get_recent_errors(&self, limit: usize) -> anyhow::Result<Vec<ErrorTelemetry>> {
        self.storage.get_recent_errors(limit).await
    }
} 