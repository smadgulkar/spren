use super::error::AIError;
use super::path_utils::{convert_mkdir_command, sanitize_windows_path};
use super::schema::{AIResponseSchema, CommandStepSchema, ResourceImpactSchema};
use crate::platform::PlatformCommand;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use validator::Validate;

#[derive(Debug, Clone)]
pub struct CommandChain {
    pub steps: Vec<CommandStep>,
    pub total_impact: ResourceImpact,
    pub explanation: String,
    pub raw_response: String,
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
    pub version: String,
    #[serde(flatten)]
    pub response: AIResponseSchema,
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
        self.response
            .validate()
            .map_err(|e| AIError::ValidationError(format!("Response validation failed: {}", e)))?;

        Ok(())
    }

    pub fn into_command_chain(self) -> Result<CommandChain, AIError> {
        self.validate()?;

        let steps = self
            .response
            .steps
            .into_iter()
            .map(CommandStep::from_schema)
            .collect::<Result<Vec<_>, _>>()?;

        let total_impact = ResourceImpact::calculate_total(&steps);

        Ok(CommandChain {
            steps,
            total_impact,
            explanation: self.response.explanation,
            raw_response: self.response.raw_response,
        })
    }
}

impl CommandStep {
    pub fn from_schema(schema: CommandStepSchema) -> Result<Self, AIError> {
        let command = if cfg!(windows) {
            let sanitized = sanitize_windows_path(&schema.command);
            convert_mkdir_command(&sanitized)
        } else {
            schema.command
        };

        Ok(Self {
            command,
            explanation: schema.explanation,
            is_dangerous: schema.is_dangerous,
            impact: ResourceImpact::from_schema(schema.estimated_impact)?,
            rollback_command: schema.rollback_command.map(|cmd| {
                if cfg!(windows) {
                    let sanitized = sanitize_windows_path(&cmd);
                    convert_mkdir_command(&sanitized)
                } else {
                    cmd
                }
            }),
        })
    }

    pub fn execute(&self) -> Result<String, anyhow::Error> {
        let platform_cmd = PlatformCommand::from_shell_command(&self.command)
            .ok_or_else(|| anyhow!("Invalid command: {}", self.command))?;
        platform_cmd.execute()
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
                steps
                    .iter()
                    .map(|s| s.impact.estimated_duration.as_secs_f32())
                    .sum(),
            ),
        }
    }
}

impl CommandChain {
    pub fn new(
        steps: Vec<CommandStep>,
        total_impact: ResourceImpact,
        explanation: String,
        raw_response: String,
    ) -> Self {
        Self {
            steps,
            total_impact,
            explanation,
            raw_response,
        }
    }
}
