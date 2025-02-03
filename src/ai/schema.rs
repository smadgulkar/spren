use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, JsonSchema, Validate)]
pub struct AIResponseSchema {
    #[validate(length(min = 1, message = "Must have at least one step"))]
    pub steps: Vec<CommandStepSchema>,
    #[validate(length(min = 1, message = "Explanation cannot be empty"))]
    pub explanation: String,
    #[serde(default)]
    pub raw_response: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, Validate)]
pub struct CommandStepSchema {
    #[validate(length(min = 1, message = "Command cannot be empty"))]
    pub command: String,
    #[validate(length(min = 1, message = "Explanation cannot be empty"))]
    pub explanation: String,
    pub is_dangerous: bool,
    pub estimated_impact: ResourceImpactSchema,
    pub rollback_command: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, Validate)]
pub struct ResourceImpactSchema {
    #[validate(range(
        min = 0.0,
        max = 100.0,
        message = "CPU percentage must be between 0 and 100"
    ))]
    pub cpu_percentage: f32,
    #[validate(range(min = 0.0, message = "Memory usage cannot be negative"))]
    pub memory_mb: f32,
    #[validate(range(min = 0.0, message = "Disk usage cannot be negative"))]
    pub disk_mb: f32,
    #[validate(range(min = 0.0, message = "Network usage cannot be negative"))]
    pub network_mb: f32,
    #[validate(range(min = 0.0, message = "Duration cannot be negative"))]
    pub duration_seconds: f32,
}
