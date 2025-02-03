use super::error::AIError;
use super::schema::{AIResponseSchema, CommandStepSchema, ResourceImpactSchema};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::time::Duration;
use validator::Validate;

#[derive(Clone, Serialize)]
pub struct CommandChain {
    pub steps: Vec<CommandStep>,
    pub total_impact: ResourceImpact,
    pub explanation: String,
}

#[derive(Clone, Serialize)]
pub struct CommandStep {
    pub command: String,
    pub explanation: String,
    pub is_dangerous: bool,
    pub impact: ResourceImpact,
    pub rollback_command: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResourceImpact {
    pub cpu_usage: f32,
    pub memory_usage: f32,
    pub disk_usage: f32,
    pub network_usage: f32,
    #[serde(skip)] // Skip duration during serialization
    pub estimated_duration: Duration,
}

impl Default for ResourceImpact {
    fn default() -> Self {
        Self {
            cpu_usage: 0.0,
            memory_usage: 0.0,
            disk_usage: 0.0,
            network_usage: 0.0,
            estimated_duration: Duration::from_secs(0),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VersionedResponse {
    version: String,
    #[serde(flatten)]
    response: AIResponseSchema,
}

impl VersionedResponse {
    pub fn validate(&self) -> Result<(), AIError> {
        if self.version != "1.0" {
            return Err(AIError::ValidationError(format!(
                "Unsupported response version: {}. Expected 1.0",
                self.version
            )));
        }

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
                steps
                    .iter()
                    .map(|s| s.impact.estimated_duration.as_secs_f32())
                    .sum(),
            ),
        }
    }
}

// Debug implementations
impl fmt::Debug for CommandChain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            writeln!(f, "CommandChain {{")?;
            writeln!(f, "  explanation: {:?},", self.explanation)?;
            writeln!(f, "  steps: [")?;
            for step in &self.steps {
                writeln!(f, "    {:?},", step)?;
            }
            writeln!(f, "  ],")?;
            writeln!(f, "  total_impact: {:?}", self.total_impact)?;
            write!(f, "}}")
        } else {
            f.debug_struct("CommandChain")
                .field("explanation", &self.explanation)
                .field("steps", &self.steps)
                .field("total_impact", &self.total_impact)
                .finish()
        }
    }
}

impl fmt::Debug for CommandStep {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            writeln!(f, "CommandStep {{")?;
            writeln!(f, "  command: {:?},", self.command)?;
            writeln!(f, "  explanation: {:?},", self.explanation)?;
            writeln!(f, "  is_dangerous: {},", self.is_dangerous)?;
            writeln!(f, "  impact: {:?},", self.impact)?;
            writeln!(f, "  rollback: {:?}", self.rollback_command)?;
            write!(f, "}}")
        } else {
            f.debug_struct("CommandStep")
                .field("command", &self.command)
                .field("explanation", &self.explanation)
                .field("is_dangerous", &self.is_dangerous)
                .field("impact", &self.impact)
                .field("rollback", &self.rollback_command)
                .finish()
        }
    }
}

impl fmt::Display for CommandChain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Command Chain:")?;
        writeln!(f, "=============")?;

        writeln!(f, "\n{}\n", self.explanation)?;

        writeln!(f, "Steps:")?;
        for (i, step) in self.steps.iter().enumerate() {
            writeln!(f, "\n{}. {}", i + 1, step)?;
        }

        writeln!(f, "\nTotal Impact:")?;
        write!(f, "{}", self.total_impact)
    }
}

impl fmt::Display for CommandStep {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}", self.explanation)?;
        writeln!(f, "Command: {}", self.command)?;

        if self.is_dangerous {
            writeln!(f, "⚠️  This command is potentially dangerous!")?;
        }

        if let Some(rollback) = &self.rollback_command {
            writeln!(f, "Rollback: {}", rollback)?;
        }

        write!(f, "Impact: {}", self.impact)
    }
}

impl fmt::Display for ResourceImpact {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "CPU: {:.1}%, Memory: {:.1}MB, Disk: {:.1}MB",
            self.cpu_usage, self.memory_usage, self.disk_usage
        )?;
        write!(
            f,
            "Network: {:.1}MB, Duration: {:.1}s",
            self.network_usage,
            self.estimated_duration.as_secs_f32()
        )
    }
}
