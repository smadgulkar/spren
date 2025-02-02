use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, JsonSchema, Validate)]
pub struct AIResponseSchema {
    #[validate(length(min = 1, message = "Must have at least one step"))]
    pub steps: Vec<CommandStepSchema>,
    #[validate(length(min = 1, message = "Explanation cannot be empty"))]
    pub explanation: String,
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

#[derive(Debug, Serialize, JsonSchema, Validate)]
pub struct ResourceImpactSchema {
    #[validate(range(min = 0.0, max = 100.0, message = "CPU percentage must be between 0 and 100"))]
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

// Keep the custom deserializer
impl<'de> Deserialize<'de> for ResourceImpactSchema {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]  // Add this to handle both string and number
        enum StringOrFloat {
            String(String),
            Float(f32),
        }

        #[derive(Deserialize)]
        struct Helper {
            cpu_percentage: f32,
            memory_mb: f32,
            disk_mb: f32,
            network_mb: f32,
            duration_seconds: StringOrFloat,
        }

        let helper = Helper::deserialize(deserializer)?;
        
        // Convert duration to f32
        let duration = match helper.duration_seconds {
            StringOrFloat::String(s) => {
                if s.starts_with('>') {
                    s.trim_start_matches('>').trim().parse().unwrap_or(60.0)
                } else {
                    s.parse().unwrap_or(60.0)
                }
            },
            StringOrFloat::Float(f) => f,
        };

        Ok(ResourceImpactSchema {
            cpu_percentage: helper.cpu_percentage,
            memory_mb: helper.memory_mb,
            disk_mb: helper.disk_mb,
            network_mb: helper.network_mb,
            duration_seconds: duration,
        })
    }
} 