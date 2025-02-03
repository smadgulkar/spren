mod visualization;
pub use visualization::HistoryVisualization;

use crate::ai::CommandChain;
use crate::executor::types::ExecutionStatus;
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;
use tokio::fs;

#[derive(Debug, Serialize, Deserialize)]
pub struct CommandHistory {
    pub entries: Vec<HistoryEntry>,
    pub max_entries: usize,
    storage_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub command_chain: CommandChain,
    pub execution_time: DateTime<Utc>,
    pub status: ExecutionStatus,
    pub output: Option<String>,
    pub error: Option<String>,
    pub execution_duration: Option<Duration>,
    pub working_directory: Option<PathBuf>,
    pub environment: Option<HistoryEnvironment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEnvironment {
    pub shell_type: String,
    pub os_info: String,
    pub user: Option<String>,
    pub variables: Vec<(String, String)>,
}

// Re-export these types
pub use self::visualization::*;

// ... rest of the history code ...
