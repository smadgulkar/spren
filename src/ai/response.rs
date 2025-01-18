use serde::{Deserialize, Serialize};
use std::time::Duration;
use validator::Validate;
use super::error::AIError;
use super::schema::{AIResponseSchema, CommandStepSchema, ResourceImpactSchema};

#[derive(Debug, Clone)]
pub struct CommandChain {
    pub steps: Vec<CommandStep>,
    pub total_impact: ResourceImpact,
    pub explanation: String,
}

#[derive(Debug, Clone)]
pub struct CommandStep {
    pub command: String,
    pub explanation: String,
    pub is_dangerous: bool,
    pub impact: ResourceImpact,
    pub rollback_command: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ResourceImpact {
    pub cpu_usage: f32,
    pub memory_usage: f32,
    pub disk_usage: f32,
    pub network_usage: f32,
    pub estimated_duration: Duration,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VersionedResponse {
    version: String,
    #[serde(flatten)]
    response: AIResponseSchema,
}

impl VersionedResponse {
    pub fn validate(&self) -> Result<(), AIError> {
        // Version check
        if self.version != "1.0" {
            return Err(AIError::ValidationError(format!(
                "Unsupported response version: {}. Expected 1.0",
                self.version
            )));
        }

        // Schema validation
        self.response.validate().map_err(|e| AIError::ValidationError(
            format!("Response validation failed: {}", e)
        ))?;

        Ok(())
    }

    pub fn into_command_chain(self) -> Result<CommandChain, AIError> {
        self.validate()?;

        let steps = self.response.steps.into_iter()
            .map(CommandStep::from_schema)
            .collect::<Result<Vec<_>, _>>()?;

        let total_impact = ResourceImpact::calculate_total(&steps);

        Ok(CommandChain {
            steps,
            total_impact,
            explanation: self.response.explanation,
        })
    }
}

impl CommandStep {
    pub fn from_schema(schema: CommandStepSchema) -> Result<Self, AIError> {
        Ok(Self {
            command: schema.command,
            explanation: schema.explanation,
            is_dangerous: schema.is_dangerous,
            impact: ResourceImpact::from_schema(schema.estimated_impact)?,
            rollback_command: schema.rollback_command,
        })
    }
}

impl ResourceImpact {
    pub fn from_schema(schema: ResourceImpactSchema) -> Result<Self, AIError> {
        Ok(Self {
            cpu_usage: schema.cpu_percentage,
            memory_usage: schema.memory_mb,
            disk_usage: schema.disk_mb,
            network_usage: schema.network_mb,
            estimated_duration: Duration::from_secs_f32(schema.duration_seconds),
        })
    }

    pub fn calculate_total(steps: &[CommandStep]) -> Self {
        Self {
            cpu_usage: steps.iter().map(|s| s.impact.cpu_usage).sum(),
            memory_usage: steps.iter().map(|s| s.impact.memory_usage).sum(),
            disk_usage: steps.iter().map(|s| s.impact.disk_usage).sum(),
            network_usage: steps.iter().map(|s| s.impact.network_usage).sum(),
            estimated_duration: Duration::from_secs_f32(
                steps.iter()
                    .map(|s| s.impact.estimated_duration.as_secs_f32())
                    .sum()
            ),
        }
    }
} 