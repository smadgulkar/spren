use super::ErrorTelemetry;
use anyhow::Result;
use serde_json::Value;
use std::fs::{self, File};
use std::path::PathBuf;

pub struct TelemetryStorage {
    storage_path: PathBuf,
}

impl TelemetryStorage {
    pub fn new(path: PathBuf) -> Self {
        Self { storage_path: path }
    }

    pub async fn store_error(&self, error: &ErrorTelemetry) -> Result<()> {
        let file_name = format!(
            "error_{}.json",
            error.timestamp.format("%Y%m%d_%H%M%S")
        );
        let path = self.storage_path.join(file_name);
        
        let json = serde_json::to_string_pretty(&error)?;
        fs::create_dir_all(&self.storage_path)?;
        fs::write(path, json)?;
        
        Ok(())
    }

    pub async fn get_recent_errors(&self, limit: usize) -> Result<Vec<ErrorTelemetry>> {
        let mut errors = Vec::new();
        
        if self.storage_path.exists() {
            for entry in fs::read_dir(&self.storage_path)? {
                let entry = entry?;
                if entry.path().extension().and_then(|s| s.to_str()) == Some("json") {
                    let content = fs::read_to_string(entry.path())?;
                    let error: ErrorTelemetry = serde_json::from_str(&content)?;
                    errors.push(error);
                }
            }
        }
        
        errors.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        Ok(errors.into_iter().take(limit).collect())
    }
} 